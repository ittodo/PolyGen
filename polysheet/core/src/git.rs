use crate::diff::ProjectSnapshot;
use crate::model::{
    CalculationDocument, FormatDocument, FormulaDocument, RowIdDocument, RowRecord, SheetDocument,
    SheetKind, SheetManifest, WorkbookManifest, FORMAT_VERSION,
};
use crate::project::{build_row_records, PolySheetProject};
use crate::validation::SchemaIndex;
use anyhow::{anyhow, bail, Context, Result};
use git2::{ErrorCode, ObjectType, Oid, Repository, Tree};
use pest::Parser;
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::collections::{BTreeSet, HashMap, VecDeque};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

const MAX_MANIFEST_BYTES: usize = 1024 * 1024;
const MAX_SCHEMA_FILE_BYTES: usize = 8 * 1024 * 1024;
const MAX_SCHEMA_DEPENDENCY_FILES: usize = 256;
const MAX_SCHEMA_DEPENDENCY_BYTES: usize = 64 * 1024 * 1024;
const MAX_DATA_SOURCE_BYTES: usize = 256 * 1024 * 1024;
const MAX_SIDECAR_BYTES: usize = 64 * 1024 * 1024;
const MAX_SHEETS: usize = 1024;
const MAX_REVISION_SNAPSHOT_BYTES: usize = 512 * 1024 * 1024;
const MAX_REVISION_SNAPSHOT_ROWS: usize = 2_000_000;

struct RevisionReadContext {
    cache: HashMap<Oid, Arc<[u8]>>,
    consumed_bytes: usize,
    consumed_rows: usize,
    maximum_bytes: usize,
    maximum_rows: usize,
}

impl Default for RevisionReadContext {
    fn default() -> Self {
        Self {
            cache: HashMap::new(),
            consumed_bytes: 0,
            consumed_rows: 0,
            maximum_bytes: MAX_REVISION_SNAPSHOT_BYTES,
            maximum_rows: MAX_REVISION_SNAPSHOT_ROWS,
        }
    }
}

impl RevisionReadContext {
    fn charge_bytes(&mut self, bytes: usize, path: &Path) -> Result<()> {
        let consumed_bytes = self
            .consumed_bytes
            .checked_add(bytes)
            .context("revision snapshot byte count overflow")?;
        if consumed_bytes > self.maximum_bytes {
            bail!(
                "revision snapshot exceeds aggregate byte limit of {} while reading {}",
                self.maximum_bytes,
                path.display()
            );
        }
        self.consumed_bytes = consumed_bytes;
        Ok(())
    }

    fn charge_rows(&mut self, rows: usize, sheet_id: &str) -> Result<()> {
        let consumed_rows = self
            .consumed_rows
            .checked_add(rows)
            .context("revision snapshot row count overflow")?;
        if consumed_rows > self.maximum_rows {
            bail!(
                "revision snapshot exceeds aggregate row limit of {} at sheet '{}'",
                self.maximum_rows,
                sheet_id
            );
        }
        self.consumed_rows = consumed_rows;
        Ok(())
    }

    #[cfg(test)]
    fn with_limits(maximum_bytes: usize, maximum_rows: usize) -> Self {
        Self {
            maximum_bytes,
            maximum_rows,
            ..Self::default()
        }
    }
}

#[derive(Default)]
struct MaterializedPathRegistry {
    #[cfg(windows)]
    claims: HashMap<String, PathBuf>,
}

impl MaterializedPathRegistry {
    fn claim(&mut self, relative: &Path) -> Result<()> {
        #[cfg(not(windows))]
        {
            let _ = relative;
            Ok(())
        }

        #[cfg(windows)]
        {
            let mut exact_prefix = PathBuf::new();
            let mut key = String::new();
            for component in relative.components() {
                let Component::Normal(component) = component else {
                    bail!(
                        "revision materialization path is not repository-relative: {}",
                        relative.display()
                    );
                };
                let component = component.to_str().with_context(|| {
                    format!(
                        "revision materialization path is not valid UTF-8: {}",
                        relative.display()
                    )
                })?;
                validate_windows_materialized_component(component, relative)?;
                exact_prefix.push(component);
                if !key.is_empty() {
                    key.push('/');
                }
                // Windows performs case-insensitive path lookup using an
                // uppercase-style comparison. Uppercasing also catches aliases
                // such as Greek sigma/final-sigma that lowercase keys miss.
                key.push_str(&component.to_uppercase());
                if let Some(existing) = self.claims.get(&key) {
                    if existing != &exact_prefix {
                        bail!(
                            "revision materialization paths alias on Windows: '{}' and '{}'",
                            existing.display(),
                            exact_prefix.display()
                        );
                    }
                } else {
                    self.claims.insert(key.clone(), exact_prefix.clone());
                }
            }
            Ok(())
        }
    }
}

#[cfg(windows)]
fn validate_windows_materialized_component(component: &str, path: &Path) -> Result<()> {
    if component.ends_with(' ')
        || component.ends_with('.')
        || component.split('.').any(|segment| segment.ends_with(' '))
        || component
            .chars()
            .any(|character| character <= '\u{1f}' || r#"<>:\"/\\|?*"#.contains(character))
    {
        bail!(
            "revision path is not safely representable on Windows: {}",
            path.display()
        );
    }
    let stem = component
        .split('.')
        .next()
        .unwrap_or(component)
        .to_ascii_uppercase();
    let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || stem
            .strip_prefix("COM")
            .or_else(|| stem.strip_prefix("LPT"))
            .is_some_and(|suffix| suffix.len() == 1 && matches!(suffix.as_bytes()[0], b'1'..=b'9'));
    if reserved {
        bail!(
            "revision path uses a reserved Windows device name: {}",
            path.display()
        );
    }
    Ok(())
}

pub fn list_references(project: &PolySheetProject) -> Result<Vec<String>> {
    let repository = Repository::discover(&project.root)?;
    let mut references = repository
        .references()?
        .filter_map(|reference| reference.ok())
        .filter_map(|reference| reference.shorthand().map(str::to_string))
        .collect::<Vec<_>>();
    references.sort();
    references.dedup();
    Ok(references)
}

pub fn snapshot_at_revision(project: &PolySheetProject, revision: &str) -> Result<ProjectSnapshot> {
    if revision.eq_ignore_ascii_case("worktree") {
        return Ok(ProjectSnapshot::from_project(project));
    }
    let repository = Repository::discover(&project.root)?;
    let workdir = repository
        .workdir()
        .context("bare Git repositories are not supported")?;
    let object = repository
        .revparse_single(revision)
        .with_context(|| format!("cannot resolve Git revision '{revision}'"))?;
    let commit = object
        .peel_to_commit()
        .with_context(|| format!("Git revision '{revision}' does not resolve to a commit"))?;
    let tree = commit.tree()?;
    let mut read_context = RevisionReadContext::default();
    let project_root = repository_path(workdir, &project.root)?;
    let workbook_path = project_root.join("workbook.toml");
    let workbook_bytes = read_revision_path(
        &mut read_context,
        &repository,
        &tree,
        revision,
        &workbook_path,
        MAX_MANIFEST_BYTES,
    )?;
    let workbook: WorkbookManifest = toml::from_str(
        std::str::from_utf8(&workbook_bytes).context("revision workbook.toml is not UTF-8")?,
    )?;
    if workbook.format_version != FORMAT_VERSION {
        bail!(
            "unsupported PolySheet format version {} in revision '{revision}'; expected {}",
            workbook.format_version,
            FORMAT_VERSION
        );
    }
    if workbook.sheets.len() > MAX_SHEETS {
        bail!("revision workbook contains more than the supported {MAX_SHEETS} sheets");
    }
    ensure_unique_revision_sheet_ids(&workbook.sheets)?;
    let revision_schema = load_revision_schema(
        &mut read_context,
        &repository,
        &tree,
        revision,
        &project_root,
        &workbook,
    )?;
    let data_root = revision_data_root(
        &project_root,
        &workbook,
        revision_schema.schema_path.as_deref(),
        revision_schema.sources_path.as_deref(),
    )?;
    let mut sheets = std::collections::BTreeMap::new();
    let mut bound_definitions = BTreeSet::new();
    let mut bound_sources = Vec::<(String, PathBuf)>::new();

    for sheet_id in workbook.sheets {
        let directory = revision_sheet_directory(&project_root, &sheet_id)?;
        let manifest_path = directory.join("sheet.toml");
        let manifest_bytes = read_revision_path(
            &mut read_context,
            &repository,
            &tree,
            revision,
            &manifest_path,
            MAX_MANIFEST_BYTES,
        )?;
        let manifest: SheetManifest = toml::from_str(
            std::str::from_utf8(&manifest_bytes).context("revision sheet.toml is not UTF-8")?,
        )?;
        if manifest.id != sheet_id {
            bail!(
                "revision sheet id mismatch: workbook references '{sheet_id}' but sheet.toml contains '{}'",
                manifest.id
            );
        }
        let snapshot_definition = (manifest.kind == SheetKind::Data)
            .then(|| manifest.definition.clone())
            .flatten();
        let mut snapshot_source_path = None;
        let (rows, formulas) = match manifest.kind {
            SheetKind::Data => {
                let definition = manifest
                    .definition
                    .as_deref()
                    .context("revision data sheet is missing definition")?;
                if !bound_definitions.insert(definition.to_string()) {
                    bail!(
                        "revision binds table definition '{definition}' to more than one data sheet"
                    );
                }
                let table = revision_schema
                    .index
                    .table(definition)
                    .with_context(|| format!("unknown revision table definition '{definition}'"))?;
                let json_source = table
                    .load
                    .as_ref()
                    .and_then(|load| load.json.as_deref())
                    .with_context(|| format!("table '{definition}' has no JSON source"))?;
                if json_source.contains('*') || json_source.contains('?') {
                    bail!("wildcard JSON sources cannot be diffed as editable sheets");
                }
                let source_path = resolve_revision_path(
                    &data_root,
                    json_source,
                    &format!("JSON source for table '{definition}'"),
                )?;
                if let Some((existing_sheet, _)) = bound_sources
                    .iter()
                    .find(|(_, existing)| revision_source_paths_alias(existing, &source_path))
                {
                    bail!(
                        "revision data sheets '{existing_sheet}' and '{sheet_id}' share JSON source '{}'",
                        source_path.display()
                    );
                }
                bound_sources.push((sheet_id.clone(), source_path.clone()));
                let source_bytes = read_revision_path(
                    &mut read_context,
                    &repository,
                    &tree,
                    revision,
                    &source_path,
                    MAX_DATA_SOURCE_BYTES,
                )?;
                let source: Value = serde_json::from_slice(&source_bytes).with_context(|| {
                    format!("invalid revision JSON source at {}", source_path.display())
                })?;
                let source_rows = source
                    .as_array()
                    .context("revision JSON source must be an array")?;
                // Charge before cloning rows into RowRecord values. Otherwise a
                // small JSON array with millions of scalar entries can allocate
                // the complete snapshot before the aggregate row limit fires.
                read_context.charge_rows(source_rows.len(), &sheet_id)?;
                let requires_row_ids = crate::validation::primary_key(table).is_none();
                let prior = if requires_row_ids {
                    read_revision_json_optional::<RowIdDocument>(
                        &mut read_context,
                        &repository,
                        &tree,
                        revision,
                        &directory.join("rowids.json"),
                        MAX_SIDECAR_BYTES,
                    )?
                } else {
                    None
                };
                if requires_row_ids && prior.is_none() && !source_rows.is_empty() {
                    bail!(
                        "revision data sheet '{}' has no stable row identity sidecar",
                        manifest.name
                    );
                }
                let (records, _, issues) = build_row_records(source_rows, table, prior, false);
                if !issues.is_empty() {
                    bail!(
                        "revision '{revision}' has unresolved row identities for sheet '{}'",
                        manifest.name
                    );
                }
                let mut formulas = read_revision_json_optional::<FormulaDocument>(
                    &mut read_context,
                    &repository,
                    &tree,
                    revision,
                    &directory.join("formulas.json"),
                    MAX_SIDECAR_BYTES,
                )?
                .unwrap_or_default();
                if formulas.version == 0 {
                    formulas.version = FORMAT_VERSION;
                }
                snapshot_source_path = Some(snapshot_source_path_string(
                    project,
                    &sheet_id,
                    &workdir.join(&source_path),
                ));
                (records, formulas)
            }
            SheetKind::Calculation => {
                read_context.charge_rows(1, &sheet_id)?;
                let mut document = read_revision_json_optional::<CalculationDocument>(
                    &mut read_context,
                    &repository,
                    &tree,
                    revision,
                    &directory.join("cells.json"),
                    MAX_SIDECAR_BYTES,
                )?
                .unwrap_or_default();
                if document.version == 0 {
                    document.version = FORMAT_VERSION;
                }
                (
                    vec![RowRecord {
                        id: "calculation".into(),
                        value: serde_json::to_value(document)?,
                    }],
                    FormulaDocument::default(),
                )
            }
        };
        ensure_unique_revision_row_ids(&rows, &sheet_id)?;
        let mut format = read_revision_json_optional::<FormatDocument>(
            &mut read_context,
            &repository,
            &tree,
            revision,
            &directory.join("format.json"),
            MAX_SIDECAR_BYTES,
        )?
        .unwrap_or_default();
        if format.version == 0 {
            format.version = FORMAT_VERSION;
        }
        let format = serde_json::to_value(format)?;
        sheets.insert(
            sheet_id.clone(),
            crate::diff::SnapshotSheet {
                id: sheet_id,
                name: manifest.name,
                kind: manifest.kind,
                definition: snapshot_definition,
                source_path: snapshot_source_path,
                rows,
                formulas,
                format,
            },
        );
    }
    Ok(ProjectSnapshot { sheets })
}

fn revision_source_paths_alias(left: &Path, right: &Path) -> bool {
    #[cfg(windows)]
    {
        left.to_string_lossy().replace('\\', "/").to_uppercase()
            == right.to_string_lossy().replace('\\', "/").to_uppercase()
    }
    #[cfg(not(windows))]
    {
        left == right
    }
}

fn ensure_unique_revision_row_ids(rows: &[RowRecord], sheet_id: &str) -> Result<()> {
    let mut row_ids = BTreeSet::new();
    for row in rows {
        if row.id.is_empty() {
            bail!("revision sheet '{sheet_id}' contains an empty row id");
        }
        if !row_ids.insert(row.id.as_str()) {
            bail!(
                "revision sheet '{sheet_id}' contains duplicate row id '{}'",
                row.id
            );
        }
    }
    Ok(())
}

fn ensure_unique_revision_sheet_ids(sheet_ids: &[String]) -> Result<()> {
    #[cfg(windows)]
    let unique = sheet_ids
        .iter()
        .map(|sheet_id| sheet_id.to_uppercase())
        .collect::<BTreeSet<_>>();
    #[cfg(not(windows))]
    let unique = sheet_ids.iter().collect::<BTreeSet<_>>();
    if unique.len() != sheet_ids.len() {
        bail!("revision workbook contains duplicate sheet ids or path aliases");
    }
    Ok(())
}

fn snapshot_source_path_string(
    project: &PolySheetProject,
    sheet_id: &str,
    revision_path: &Path,
) -> String {
    if let Some(SheetDocument::Data(current)) = project.sheets.get(sheet_id) {
        let current_path = Path::new(&current.source_path);
        if paths_refer_to_same_source(revision_path, current_path) {
            return current.source_path.clone();
        }
    }
    revision_path.to_string_lossy().into_owned()
}

fn paths_refer_to_same_source(left: &Path, right: &Path) -> bool {
    let left = dunce::canonicalize(left).unwrap_or_else(|_| dunce::simplified(left).to_path_buf());
    let right =
        dunce::canonicalize(right).unwrap_or_else(|_| dunce::simplified(right).to_path_buf());
    #[cfg(windows)]
    {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    }
    #[cfg(not(windows))]
    {
        left == right
    }
}

struct RevisionSchema {
    index: SchemaIndex,
    schema_path: Option<PathBuf>,
    sources_path: Option<PathBuf>,
}

fn load_revision_schema(
    read_context: &mut RevisionReadContext,
    repository: &Repository,
    tree: &Tree<'_>,
    revision: &str,
    project_root: &Path,
    workbook: &WorkbookManifest,
) -> Result<RevisionSchema> {
    let Some(schema_config) = workbook.schema.as_deref() else {
        if workbook.sources.is_some() {
            bail!("revision workbook links a sources file without a schema");
        }
        return Ok(RevisionSchema {
            index: SchemaIndex::default(),
            schema_path: None,
            sources_path: None,
        });
    };

    let schema_path = resolve_revision_path(project_root, schema_config, "workbook schema")?;
    let materialized = tempfile::tempdir().context("cannot create revision schema workspace")?;
    let mut path_registry = MaterializedPathRegistry::default();
    materialize_schema_dependencies(
        read_context,
        &mut path_registry,
        repository,
        tree,
        revision,
        materialized.path(),
        &schema_path,
    )?;

    let sources_path = if let Some(sources_config) = workbook.sources.as_deref() {
        let sources_path = resolve_revision_path(project_root, sources_config, "workbook sources")?;
        let bytes = read_revision_path(
            read_context,
            repository,
            tree,
            revision,
            &sources_path,
            MAX_SCHEMA_FILE_BYTES,
        )?;
        write_materialized_file(
            materialized.path(),
            &sources_path,
            &bytes,
            &mut path_registry,
        )?;
        Some(sources_path)
    } else {
        let default_sources = schema_path.with_extension("sources.toml");
        match read_revision_path_optional(
            read_context,
            repository,
            tree,
            revision,
            &default_sources,
            MAX_SCHEMA_FILE_BYTES,
        )? {
            Some(bytes) => {
                write_materialized_file(
                    materialized.path(),
                    &default_sources,
                    &bytes,
                    &mut path_registry,
                )?;
                Some(default_sources)
            }
            None => None,
        }
    };

    let materialized_schema = materialized.path().join(&schema_path);
    let materialized_sources = sources_path
        .as_ref()
        .map(|path| materialized.path().join(path));
    let loaded =
        polygen::load_project_schema(&materialized_schema, materialized_sources.as_deref())
            .with_context(|| {
                format!(
                    "cannot load schema '{}' from revision '{revision}'",
                    schema_path.display()
                )
            })?;
    Ok(RevisionSchema {
        index: SchemaIndex::from_schema(&loaded.schema),
        schema_path: Some(schema_path),
        sources_path,
    })
}

fn materialize_schema_dependencies(
    read_context: &mut RevisionReadContext,
    path_registry: &mut MaterializedPathRegistry,
    repository: &Repository,
    tree: &Tree<'_>,
    revision: &str,
    destination: &Path,
    root_schema: &Path,
) -> Result<()> {
    let mut queue = VecDeque::from([root_schema.to_path_buf()]);
    let mut visited = BTreeSet::new();
    let mut total_bytes = 0usize;

    while let Some(schema_path) = queue.pop_front() {
        if !visited.insert(schema_path.clone()) {
            continue;
        }
        if visited.len() > MAX_SCHEMA_DEPENDENCY_FILES {
            bail!(
                "revision schema dependency count exceeds limit of {MAX_SCHEMA_DEPENDENCY_FILES}"
            );
        }
        let bytes = read_revision_path(
            read_context,
            repository,
            tree,
            revision,
            &schema_path,
            MAX_SCHEMA_FILE_BYTES,
        )?;
        total_bytes = total_bytes
            .checked_add(bytes.len())
            .context("revision schema dependency size overflow")?;
        if total_bytes > MAX_SCHEMA_DEPENDENCY_BYTES {
            bail!(
                "revision schema dependencies exceed {} bytes",
                MAX_SCHEMA_DEPENDENCY_BYTES
            );
        }
        let materialized_path =
            write_materialized_file(destination, &schema_path, &bytes, path_registry)?;
        if schema_path
            .extension()
            .and_then(|extension| extension.to_str())
            == Some("renames")
        {
            continue;
        }

        let source = std::str::from_utf8(&bytes)
            .with_context(|| format!("revision schema is not UTF-8: {}", schema_path.display()))?;
        let main_pair = polygen::Polygen::parse(polygen::Rule::main, source)
            .with_context(|| format!("invalid revision schema: {}", schema_path.display()))?
            .next()
            .with_context(|| {
                format!(
                    "revision schema has no main rule: {}",
                    schema_path.display()
                )
            })?;
        let ast = polygen::ast_parser::build_ast_from_pairs(main_pair, materialized_path)
            .with_context(|| format!("invalid revision schema: {}", schema_path.display()))?;
        let base = schema_path.parent().unwrap_or_else(|| Path::new(""));
        for import in ast.file_imports {
            let import_path = resolve_revision_path(base, &import, "schema import")?;
            queue.push_back(import_path);
        }
    }
    Ok(())
}

fn write_materialized_file(
    root: &Path,
    relative: &Path,
    bytes: &[u8],
    path_registry: &mut MaterializedPathRegistry,
) -> Result<PathBuf> {
    path_registry.claim(relative)?;
    let path = root.join(relative);
    let parent = path
        .parent()
        .with_context(|| format!("revision path has no parent: {}", relative.display()))?;
    fs::create_dir_all(parent)?;
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&path)
        .with_context(|| {
            format!(
                "revision materialization path already exists or aliases another path: {}",
                relative.display()
            )
        })?;
    file.write_all(bytes)?;
    Ok(path)
}

fn revision_data_root(
    project_root: &Path,
    workbook: &WorkbookManifest,
    schema_path: Option<&Path>,
    sources_path: Option<&Path>,
) -> Result<PathBuf> {
    if let Some(data_root) = workbook.data_root.as_deref() {
        return resolve_revision_path(project_root, data_root, "workbook data_root");
    }
    Ok(sources_path
        .and_then(Path::parent)
        .or_else(|| schema_path.and_then(Path::parent))
        .unwrap_or(project_root)
        .to_path_buf())
}

pub fn stage_sheet_files(project: &PolySheetProject, sheet_ids: &[String]) -> Result<Vec<String>> {
    let repository = Repository::discover(&project.root)?;
    let workdir = repository
        .workdir()
        .context("bare Git repositories are not supported")?;
    let mut index = repository.index()?;
    let mut staged = Vec::new();
    stage_removed_sheet_files(project, workdir, &mut index, &mut staged)?;
    for sheet_id in sheet_ids {
        let Some(sheet) = project.sheets.get(sheet_id) else {
            bail!("unknown sheet '{sheet_id}'");
        };
        let directory = project.root.join("sheets").join(sheet_id);
        let mut paths = vec![directory.join("sheet.toml"), directory.join("format.json")];
        match sheet {
            SheetDocument::Data(data) => {
                paths.push(PathBuf::from(&data.source_path));
                paths.push(directory.join("formulas.json"));
                if data.row_ids.is_some() {
                    paths.push(directory.join("rowids.json"));
                }
            }
            SheetDocument::Calculation(_) => paths.push(directory.join("cells.json")),
        }
        for path in paths {
            let relative = repository_path(workdir, &path)?;
            index.add_path(&relative)?;
            staged.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
    let workbook = repository_path(workdir, &project.root.join("workbook.toml"))?;
    index.add_path(&workbook)?;
    staged.push(workbook.to_string_lossy().replace('\\', "/"));
    index.write()?;
    staged.sort();
    staged.dedup();
    Ok(staged)
}

fn stage_removed_sheet_files(
    project: &PolySheetProject,
    workdir: &Path,
    index: &mut git2::Index,
    staged: &mut Vec<String>,
) -> Result<()> {
    let sheets_root = repository_path(workdir, &project.root.join("sheets"))?;
    let active_ids = project
        .manifest
        .sheets
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let removed_paths = index
        .iter()
        .filter_map(|entry| {
            let path = PathBuf::from(String::from_utf8_lossy(&entry.path).into_owned());
            let relative = path.strip_prefix(&sheets_root).ok()?;
            let sheet_id = match relative.components().next()? {
                Component::Normal(component) => component.to_str()?,
                _ => return None,
            };
            (!active_ids.contains(sheet_id) && !workdir.join(&path).exists()).then_some(path)
        })
        .collect::<Vec<_>>();
    for path in removed_paths {
        index.remove_path(&path)?;
        staged.push(path.to_string_lossy().replace('\\', "/"));
    }
    Ok(())
}

pub fn git_attributes_lines(project: &PolySheetProject) -> Result<Vec<String>> {
    let repository = Repository::discover(&project.root)?;
    let workdir = repository
        .workdir()
        .context("bare Git repositories are not supported")?;
    let mut lines = Vec::new();
    for sheet in project.sheets.values() {
        if let SheetDocument::Data(data) = sheet {
            let path = repository_path(workdir, Path::new(&data.source_path))?;
            lines.push(format!("{} diff=polysheet", git_attributes_pattern(&path)?));
        }
    }
    lines.sort();
    lines.dedup();
    Ok(lines)
}

/// Encode one exact repository path as a `.gitattributes` pattern.
///
/// Attribute patterns are whitespace-delimited and use gitignore-style glob
/// syntax. Escape delimiters, comment/negation prefixes, quotes, and every glob
/// metacharacter so the rule can match only this exact repository path.
fn git_attributes_pattern(path: &Path) -> Result<String> {
    let path = path
        .to_str()
        .with_context(|| format!("Git attributes path is not valid UTF-8: {}", path.display()))?;
    let portable = path.replace('\\', "/");
    if portable.chars().any(char::is_control) {
        bail!("Git attributes path contains a control character");
    }

    // These entries are written to the repository-root `.gitattributes`.
    // A leading slash is required for root-level files; without it, Git also
    // applies a slash-free pattern to files with that name in subdirectories.
    let mut pattern = String::with_capacity(portable.len() + 1);
    pattern.push('/');
    for character in portable.chars() {
        if matches!(
            character,
            ' ' | '\\' | '#' | '!' | '"' | '*' | '?' | '[' | ']'
        ) {
            pattern.push('\\');
        }
        pattern.push(character);
    }
    Ok(pattern)
}

fn read_revision_json_optional<T: DeserializeOwned>(
    read_context: &mut RevisionReadContext,
    repository: &Repository,
    tree: &Tree<'_>,
    revision: &str,
    path: &Path,
    maximum_bytes: usize,
) -> Result<Option<T>> {
    read_revision_path_optional(
        read_context,
        repository,
        tree,
        revision,
        path,
        maximum_bytes,
    )?
    .map(|bytes| {
        serde_json::from_slice(&bytes)
            .with_context(|| format!("invalid revision JSON at {}", path.display()))
    })
    .transpose()
}

fn read_revision_path(
    read_context: &mut RevisionReadContext,
    repository: &Repository,
    tree: &Tree<'_>,
    revision: &str,
    path: &Path,
    maximum_bytes: usize,
) -> Result<Arc<[u8]>> {
    read_revision_path_optional(
        read_context,
        repository,
        tree,
        revision,
        path,
        maximum_bytes,
    )?
    .ok_or_else(|| anyhow!("path not found in revision {revision}: {}", path.display()))
}

fn read_revision_path_optional(
    read_context: &mut RevisionReadContext,
    repository: &Repository,
    tree: &Tree<'_>,
    revision: &str,
    path: &Path,
    maximum_bytes: usize,
) -> Result<Option<Arc<[u8]>>> {
    let entry = match tree.get_path(path) {
        Ok(entry) => entry,
        Err(error) if error.code() == ErrorCode::NotFound => return Ok(None),
        Err(error) => {
            return Err(error).with_context(|| {
                format!(
                    "cannot read path from revision {revision}: {}",
                    path.display()
                )
            });
        }
    };
    if entry.kind() != Some(ObjectType::Blob) {
        bail!("revision path is not a file: {}", path.display());
    }
    if entry.filemode() == 0o120000 {
        bail!(
            "revision symbolic links are not supported: {}",
            path.display()
        );
    }
    let object_id = entry.id();
    if let Some(bytes) = read_context.cache.get(&object_id).cloned() {
        if bytes.len() > maximum_bytes {
            bail!(
                "revision file exceeds {} byte limit: {}",
                maximum_bytes,
                path.display()
            );
        }
        read_context.charge_bytes(bytes.len(), path)?;
        return Ok(Some(bytes));
    }

    let blob = repository.find_blob(object_id)?;
    let blob_size = blob.size();
    if blob_size > maximum_bytes {
        bail!(
            "revision file exceeds {} byte limit: {}",
            maximum_bytes,
            path.display()
        );
    }
    read_context.charge_bytes(blob_size, path)?;
    let bytes = Arc::<[u8]>::from(blob.content());
    read_context.cache.insert(object_id, bytes.clone());
    Ok(Some(bytes))
}

fn revision_sheet_directory(project_root: &Path, sheet_id: &str) -> Result<PathBuf> {
    if sheet_id.contains('/') || sheet_id.contains('\\') {
        bail!("revision sheet id is not a safe path component: '{sheet_id}'");
    }
    let mut components = Path::new(sheet_id).components();
    let component = match (components.next(), components.next()) {
        (Some(Component::Normal(component)), None) => component,
        _ => bail!("revision sheet id is not a safe path component: '{sheet_id}'"),
    };
    validate_revision_path_components(Path::new(sheet_id))?;
    Ok(project_root.join("sheets").join(component))
}

fn resolve_revision_path(base: &Path, configured: &str, label: &str) -> Result<PathBuf> {
    let portable_configured = configured.replace('\\', "/");
    let configured_path = Path::new(&portable_configured);
    if configured_path.as_os_str().is_empty() {
        bail!("revision {label} path is empty");
    }
    let bytes = portable_configured.as_bytes();
    let has_windows_prefix = bytes.len() >= 2 && bytes[0].is_ascii_alphabetic() && bytes[1] == b':';
    if has_windows_prefix
        || portable_configured.starts_with("//")
        || configured_path
            .components()
            .any(|component| matches!(component, Component::Prefix(_) | Component::RootDir))
    {
        bail!("revision {label} path must be relative to the Git repository: {configured}");
    }

    let unresolved = base.join(configured_path);
    let mut resolved = PathBuf::new();
    for component in unresolved.components() {
        match component {
            Component::Normal(component) => resolved.push(component),
            Component::CurDir => {}
            Component::ParentDir => {
                if !resolved.pop() {
                    bail!("revision {label} path escapes the Git repository: {configured}");
                }
            }
            Component::Prefix(_) | Component::RootDir => {
                bail!("revision {label} base is not repository-relative")
            }
        }
    }
    validate_revision_path_components(&resolved)?;
    Ok(resolved)
}

fn validate_revision_path_components(path: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        for component in path.components() {
            let Component::Normal(component) = component else {
                bail!(
                    "revision path is not repository-relative: {}",
                    path.display()
                );
            };
            let component = component
                .to_str()
                .with_context(|| format!("revision path is not valid UTF-8: {}", path.display()))?;
            validate_windows_materialized_component(component, path)?;
        }
    }
    #[cfg(not(windows))]
    {
        let _ = path;
    }
    Ok(())
}

fn repository_path(workdir: &Path, path: &Path) -> Result<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let canonical_workdir = dunce::canonicalize(workdir)
        .with_context(|| format!("cannot resolve Git workdir: {}", workdir.display()))?;
    let canonical_path = dunce::canonicalize(&absolute)
        .with_context(|| format!("cannot resolve repository path: {}", absolute.display()))?;
    canonical_path
        .strip_prefix(&canonical_workdir)
        .map(Path::to_path_buf)
        .with_context(|| {
            format!(
                "path is outside the Git repository: {}",
                canonical_path.display()
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::diff_snapshots;
    use crate::model::{SheetDocument, WorkbookManifest};
    use crate::project::OpenProjectOptions;
    use git2::{AttrCheckFlags, IndexAddOption, Oid, Signature};
    use std::fs;
    use tempfile::tempdir;

    fn commit_all(repository: &Repository, message: &str) -> Result<Oid> {
        let mut index = repository.index()?;
        index.add_all(["*"], IndexAddOption::DEFAULT, None)?;
        index.write()?;
        let tree_id = index.write_tree()?;
        let tree = repository.find_tree(tree_id)?;
        let signature = Signature::now("PolySheet Test", "polysheet@example.invalid")?;
        let parent = repository
            .head()
            .ok()
            .and_then(|head| head.peel_to_commit().ok());
        let parents = parent.iter().collect::<Vec<_>>();
        Ok(repository.commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &parents,
        )?)
    }

    #[test]
    fn revision_reads_reuse_blobs_and_enforce_the_logical_aggregate_budget() -> Result<()> {
        let temp = tempdir()?;
        let repository = Repository::init(temp.path())?;
        fs::write(temp.path().join("blob.bin"), b"12345678")?;
        let revision = commit_all(&repository, "blob")?;
        let commit = repository.find_commit(revision)?;
        let tree = commit.tree()?;
        let mut read_context = RevisionReadContext::with_limits(16, 10);

        let first = read_revision_path(
            &mut read_context,
            &repository,
            &tree,
            "HEAD",
            Path::new("blob.bin"),
            16,
        )?;
        let second = read_revision_path(
            &mut read_context,
            &repository,
            &tree,
            "HEAD",
            Path::new("blob.bin"),
            16,
        )?;

        assert!(Arc::ptr_eq(&first, &second));
        assert_eq!(read_context.cache.len(), 1);
        assert_eq!(read_context.consumed_bytes, 16);
        let error = read_revision_path(
            &mut read_context,
            &repository,
            &tree,
            "HEAD",
            Path::new("blob.bin"),
            16,
        )
        .unwrap_err();
        assert!(error.to_string().contains("aggregate byte limit"));

        read_context.charge_rows(10, "sheet")?;
        assert!(read_context.charge_rows(1, "sheet").is_err());
        Ok(())
    }

    #[cfg(windows)]
    #[test]
    fn rejects_windows_materialization_aliases_and_unsafe_components() -> Result<()> {
        let mut registry = MaterializedPathRegistry::default();
        registry.claim(Path::new("Schemas/Types.poly"))?;
        let error = registry.claim(Path::new("schemas/Other.poly")).unwrap_err();
        assert!(error.to_string().contains("alias on Windows"));

        let mut unicode_registry = MaterializedPathRegistry::default();
        unicode_registry.claim(Path::new("Σchemas/Types.poly"))?;
        assert!(unicode_registry
            .claim(Path::new("ςchemas/Other.poly"))
            .is_err());

        for unsafe_path in ["schema/CON.poly", "schema/name. ", "schema/file.poly:ads"] {
            let mut registry = MaterializedPathRegistry::default();
            assert!(registry.claim(Path::new(unsafe_path)).is_err());
            assert!(resolve_revision_path(Path::new("project"), unsafe_path, "source").is_err());
        }
        for unsafe_sheet_id in ["CON", "name. ", "file:ads"] {
            assert!(revision_sheet_directory(Path::new("project"), unsafe_sheet_id).is_err());
        }
        assert!(ensure_unique_revision_sheet_ids(&["Sheet".into(), "sheet".into()]).is_err());
        Ok(())
    }

    #[test]
    fn git_attributes_patterns_match_only_the_literal_source_path() -> Result<()> {
        let temp = tempdir()?;
        let repository = Repository::init(temp.path())?;
        let paths = [
            "data/item # one.json",
            "data/literal[1].json",
            "data/star*.json",
            "#root file.json",
            "!important.json",
            "data/say\"hi\".json",
            "data/한글.json",
        ];
        fs::create_dir_all(temp.path().join("data"))?;
        for path in [
            "data/item # one.json",
            "data/literal[1].json",
            "#root file.json",
            "!important.json",
            "data/한글.json",
        ] {
            fs::write(temp.path().join(path), "[]")?;
        }
        let contents = paths
            .iter()
            .map(|path| {
                Ok(format!(
                    "{} diff=polysheet",
                    git_attributes_pattern(Path::new(path))?
                ))
            })
            .collect::<Result<Vec<_>>>()?
            .join("\n");
        fs::write(temp.path().join(".gitattributes"), contents)?;
        let mut index = repository.index()?;
        index.add_path(Path::new(".gitattributes"))?;
        index.write()?;
        drop(index);
        drop(repository);
        let repository = Repository::open(temp.path())?;
        let flags = AttrCheckFlags::FILE_THEN_INDEX | AttrCheckFlags::NO_SYSTEM;

        for path in paths {
            assert_eq!(
                repository.get_attr(Path::new(path), "diff", flags)?,
                Some("polysheet"),
                "literal path did not match: {path}"
            );
        }
        for wildcard_near_miss in [
            "data/literal1.json",
            "data/star-any.json",
            "nested/#root file.json",
        ] {
            assert_eq!(
                repository.get_attr(Path::new(wildcard_near_miss), "diff", flags)?,
                None,
                "escaped pattern matched a different path: {wildcard_near_miss}"
            );
        }
        assert!(git_attributes_pattern(Path::new("data/line\nbreak.json")).is_err());
        Ok(())
    }

    #[test]
    fn historical_snapshot_matches_live_defaults_for_missing_sidecars() -> Result<()> {
        let temp = tempdir()?;
        let repository = Repository::init(temp.path())?;
        let schema = temp.path().join("game.poly");
        let sources = temp.path().join("game.sources.toml");
        fs::create_dir_all(temp.path().join("data"))?;
        fs::write(
            &schema,
            "namespace game { table Item { id: u32 primary_key; name: string; } }",
        )?;
        fs::write(
            &sources,
            r#"[tables."game.Item".load]
json = "data/items.json"
"#,
        )?;
        fs::write(
            temp.path().join("data/items.json"),
            r#"[{"id":1,"name":"A"}]"#,
        )?;
        let root = temp.path().join("game.polysheet");
        let mut project = PolySheetProject::create(&root, "Game", &schema, Some(&sources), None)?;
        let data_sheet_id = project.bind_data_sheet("Items", "game.Item")?;
        let calculation_sheet_id = project.add_calculation_sheet("Calculations");
        project.save(true)?;

        let data_directory = root.join("sheets").join(&data_sheet_id);
        fs::remove_file(data_directory.join("formulas.json"))?;
        fs::remove_file(data_directory.join("format.json"))?;
        let calculation_directory = root.join("sheets").join(&calculation_sheet_id);
        fs::remove_file(calculation_directory.join("cells.json"))?;
        fs::remove_file(calculation_directory.join("format.json"))?;
        commit_all(&repository, "missing optional sidecars")?;

        let reopened = PolySheetProject::open(&root, OpenProjectOptions::default())?;
        let historical = snapshot_at_revision(&reopened, "HEAD")?;
        assert_eq!(historical, ProjectSnapshot::from_project(&reopened));
        let mut applied = reopened.clone();
        applied.apply_snapshot(&historical)?;
        Ok(())
    }

    #[test]
    fn historical_snapshot_rejects_duplicate_row_ids() -> Result<()> {
        let temp = tempdir()?;
        let repository = Repository::init(temp.path())?;
        let schema = temp.path().join("game.poly");
        let sources = temp.path().join("game.sources.toml");
        fs::create_dir_all(temp.path().join("data"))?;
        fs::write(
            &schema,
            "namespace game { table Item { id: u32 primary_key; name: string; } }",
        )?;
        fs::write(
            &sources,
            r#"[tables."game.Item".load]
json = "data/items.json"
"#,
        )?;
        let source = temp.path().join("data/items.json");
        fs::write(&source, r#"[{"id":1,"name":"A"}]"#)?;
        let root = temp.path().join("game.polysheet");
        let mut project = PolySheetProject::create(&root, "Game", &schema, Some(&sources), None)?;
        project.bind_data_sheet("Items", "game.Item")?;
        project.save(true)?;
        fs::write(&source, r#"[{"id":1,"name":"A"},{"id":1,"name":"B"}]"#)?;
        commit_all(&repository, "duplicate ids")?;

        let error = snapshot_at_revision(&project, "HEAD").unwrap_err();
        assert!(error.to_string().contains("duplicate row id"));
        Ok(())
    }

    #[test]
    fn historical_snapshot_rejects_missing_stable_row_id_sidecar() -> Result<()> {
        let temp = tempdir()?;
        let repository = Repository::init(temp.path())?;
        let schema = temp.path().join("game.poly");
        let sources = temp.path().join("game.sources.toml");
        fs::create_dir_all(temp.path().join("data"))?;
        fs::write(&schema, "namespace game { table Item { name: string; } }")?;
        fs::write(
            &sources,
            r#"[tables."game.Item".load]
json = "data/items.json"
"#,
        )?;
        fs::write(temp.path().join("data/items.json"), r#"[{"name":"A"}]"#)?;
        let root = temp.path().join("game.polysheet");
        let mut project = PolySheetProject::create(&root, "Game", &schema, Some(&sources), None)?;
        let sheet_id = project.bind_data_sheet("Items", "game.Item")?;
        project.save(true)?;
        fs::remove_file(root.join("sheets").join(sheet_id).join("rowids.json"))?;
        commit_all(&repository, "missing stable identities")?;

        let error = snapshot_at_revision(&project, "HEAD").unwrap_err();
        assert!(error.to_string().contains("no stable row identity sidecar"));
        Ok(())
    }

    #[test]
    fn historical_snapshot_rejects_duplicate_table_bindings() -> Result<()> {
        let temp = tempdir()?;
        let repository = Repository::init(temp.path())?;
        let schema = temp.path().join("game.poly");
        let sources = temp.path().join("game.sources.toml");
        fs::create_dir_all(temp.path().join("data"))?;
        fs::write(
            &schema,
            "namespace game { table Item { id: u32 primary_key; } }",
        )?;
        fs::write(
            &sources,
            r#"[tables."game.Item".load]
json = "data/items.json"
"#,
        )?;
        fs::write(temp.path().join("data/items.json"), r#"[{"id":1}]"#)?;
        let root = temp.path().join("game.polysheet");
        let mut project = PolySheetProject::create(&root, "Game", &schema, Some(&sources), None)?;
        let first_id = project.bind_data_sheet("Items", "game.Item")?;
        project.save(true)?;

        let second_id = "018f0000-0000-7000-8000-000000000077";
        let first_directory = root.join("sheets").join(&first_id);
        let second_directory = root.join("sheets").join(second_id);
        fs::create_dir_all(&second_directory)?;
        let mut sheet_manifest: SheetManifest =
            toml::from_str(&fs::read_to_string(first_directory.join("sheet.toml"))?)?;
        sheet_manifest.id = second_id.into();
        sheet_manifest.name = "Duplicate items".into();
        fs::write(
            second_directory.join("sheet.toml"),
            toml::to_string_pretty(&sheet_manifest)?,
        )?;
        for sidecar in ["formulas.json", "format.json"] {
            fs::copy(
                first_directory.join(sidecar),
                second_directory.join(sidecar),
            )?;
        }
        let workbook_path = root.join("workbook.toml");
        let mut workbook: WorkbookManifest = toml::from_str(&fs::read_to_string(&workbook_path)?)?;
        workbook.sheets.push(second_id.into());
        fs::write(&workbook_path, toml::to_string_pretty(&workbook)?)?;
        commit_all(&repository, "duplicate table binding")?;

        let error = snapshot_at_revision(&project, "HEAD").unwrap_err();
        assert!(error.to_string().contains("more than one data sheet"));
        Ok(())
    }

    #[test]
    fn historical_snapshot_rejects_normalized_duplicate_json_sources() -> Result<()> {
        let temp = tempdir()?;
        let repository = Repository::init(temp.path())?;
        let schema = temp.path().join("game.poly");
        let sources = temp.path().join("game.sources.toml");
        fs::create_dir_all(temp.path().join("data"))?;
        fs::write(
            &schema,
            "namespace game { table ItemA { id: u32 primary_key; } table ItemB { id: u32 primary_key; } }",
        )?;
        fs::write(
            &sources,
            r#"[tables."game.ItemA".load]
json = "data/items.json"

[tables."game.ItemB".load]
json = "data/nested/../items.json"
"#,
        )?;
        fs::write(temp.path().join("data/items.json"), r#"[{"id":1}]"#)?;
        let root = temp.path().join("game.polysheet");
        let mut project = PolySheetProject::create(&root, "Game", &schema, Some(&sources), None)?;
        let first_id = project.bind_data_sheet("Items A", "game.ItemA")?;
        project.save(true)?;

        let second_id = "018f0000-0000-7000-8000-000000000078";
        let first_directory = root.join("sheets").join(&first_id);
        let second_directory = root.join("sheets").join(second_id);
        fs::create_dir_all(&second_directory)?;
        let mut sheet_manifest: SheetManifest =
            toml::from_str(&fs::read_to_string(first_directory.join("sheet.toml"))?)?;
        sheet_manifest.id = second_id.into();
        sheet_manifest.name = "Items B".into();
        sheet_manifest.definition = Some("game.ItemB".into());
        fs::write(
            second_directory.join("sheet.toml"),
            toml::to_string_pretty(&sheet_manifest)?,
        )?;
        let workbook_path = root.join("workbook.toml");
        let mut workbook: WorkbookManifest = toml::from_str(&fs::read_to_string(&workbook_path)?)?;
        workbook.sheets.push(second_id.into());
        fs::write(&workbook_path, toml::to_string_pretty(&workbook)?)?;
        commit_all(&repository, "duplicate normalized source")?;

        let error = snapshot_at_revision(&project, "HEAD").unwrap_err();
        assert!(error.to_string().contains("share JSON source"));
        Ok(())
    }

    #[cfg(windows)]
    #[test]
    fn historical_source_alias_check_is_windows_case_insensitive() {
        assert!(revision_source_paths_alias(
            Path::new("Data/Σource.json"),
            Path::new("data/ςource.json")
        ));
    }

    #[test]
    fn compares_head_to_worktree_and_stages_the_atomic_sheet_set() -> Result<()> {
        let temp = tempdir()?;
        let repository = Repository::init(temp.path())?;
        let schema = temp.path().join("game.poly");
        let sources = temp.path().join("game.sources.toml");
        fs::create_dir_all(temp.path().join("data"))?;
        fs::write(
            &schema,
            "namespace game { table Item { id: u32 primary_key; name: string; } }",
        )?;
        fs::write(
            &sources,
            r#"[tables."game.Item".load]
json = "data/items.json"
"#,
        )?;
        fs::write(
            temp.path().join("data/items.json"),
            "[{\"id\":1,\"name\":\"A\"}]",
        )?;
        let root = temp.path().join("game.polysheet");
        let mut project = PolySheetProject::create(&root, "Game", &schema, Some(&sources), None)?;
        let sheet_id = project.bind_data_sheet("Items", "game.Item")?;
        project.save(true)?;

        let mut index = repository.index()?;
        index.add_all(["*"], IndexAddOption::DEFAULT, None)?;
        index.write()?;
        let tree_id = index.write_tree()?;
        let tree = repository.find_tree(tree_id)?;
        let signature = Signature::now("PolySheet Test", "polysheet@example.invalid")?;
        repository.commit(Some("HEAD"), &signature, &signature, "initial", &tree, &[])?;

        let row_id = match project.sheets.get(&sheet_id).unwrap() {
            SheetDocument::Data(sheet) => sheet.rows[0].id.clone(),
            _ => unreachable!(),
        };
        project.patch_cell(&sheet_id, &row_id, "name", Value::String("B".into()))?;
        let head = snapshot_at_revision(&project, "HEAD")?;
        let worktree = snapshot_at_revision(&project, "WORKTREE")?;
        let report = diff_snapshots(&head, &worktree);
        assert!(report.changes.iter().any(|change| change.path == "name"));

        project.save(false)?;
        let staged = stage_sheet_files(&project, std::slice::from_ref(&sheet_id))?;
        assert!(staged.iter().any(|path| path.ends_with("data/items.json")));
        assert!(staged.iter().any(|path| path.ends_with("formulas.json")));
        assert!(staged.iter().any(|path| path.ends_with("workbook.toml")));
        Ok(())
    }

    #[test]
    fn historical_snapshot_uses_revision_schema_imports_sources_and_data() -> Result<()> {
        let temp = tempdir()?;
        let repository = Repository::init(temp.path())?;
        let project_root = temp.path().join("editor/game.polysheet");
        fs::create_dir_all(temp.path().join("schemas/v1/shared"))?;
        fs::create_dir_all(temp.path().join("config"))?;
        fs::create_dir_all(project_root.join("data/v1"))?;
        fs::write(
            temp.path().join("schemas/v1/root.poly"),
            r#"
import "shared/types.poly";
import "migration.renames";
namespace game {
    table Item {
        id: u32 primary_key;
        legacy_name: string;
        stats: shared.Stats;
    }
}
"#,
        )?;
        fs::write(
            temp.path().join("schemas/v1/shared/types.poly"),
            "namespace shared { embed Stats { power: u32; } }",
        )?;
        fs::write(
            temp.path().join("schemas/v1/migration.renames"),
            "game.OldItem -> Item;\n",
        )?;
        fs::write(
            temp.path().join("config/v1.sources.toml"),
            r#"[tables."game.Item".load]
json = "v1/items.json"
"#,
        )?;
        fs::write(
            project_root.join("data/v1/items.json"),
            r#"[{"id":1,"legacy_name":"old","stats":{"power":10}}]"#,
        )?;
        let mut project = PolySheetProject::create(
            &project_root,
            "Game",
            temp.path().join("schemas/v1/root.poly"),
            Some(&temp.path().join("config/v1.sources.toml")),
            Some(&project_root.join("data")),
        )?;
        let sheet_id = project.bind_data_sheet("Items", "game.Item")?;
        project.save(true)?;
        let revision_v1 = commit_all(&repository, "v1")?;

        fs::create_dir_all(temp.path().join("schemas/v2/shared"))?;
        fs::create_dir_all(project_root.join("data/v2"))?;
        fs::write(
            temp.path().join("schemas/v2/root.poly"),
            r#"
import "shared/types.poly";
namespace game {
    table Item {
        id: u32 primary_key;
        current_name: string;
        stats: shared.Stats;
    }
}
"#,
        )?;
        fs::write(
            temp.path().join("schemas/v2/shared/types.poly"),
            "namespace shared { embed Stats { power: u32; rank: u32?; } }",
        )?;
        fs::write(
            temp.path().join("config/v2.sources.toml"),
            r#"[tables."game.Item".load]
json = "v2/items.json"
"#,
        )?;
        fs::write(
            project_root.join("data/v2/items.json"),
            r#"[{"id":1,"current_name":"new","stats":{"power":20,"rank":2}}]"#,
        )?;
        let workbook_path = project_root.join("workbook.toml");
        let mut workbook: WorkbookManifest = toml::from_str(&fs::read_to_string(&workbook_path)?)?;
        workbook.schema = Some("../../schemas/v2/root.poly".to_string());
        workbook.sources = Some("../../config/v2.sources.toml".to_string());
        workbook.data_root = Some("data".to_string());
        fs::write(&workbook_path, toml::to_string_pretty(&workbook)?)?;
        fs::remove_dir_all(temp.path().join("schemas/v1"))?;
        fs::remove_file(temp.path().join("config/v1.sources.toml"))?;
        fs::remove_dir_all(project_root.join("data/v1"))?;
        commit_all(&repository, "v2")?;

        let current = PolySheetProject::open(&project_root, OpenProjectOptions::default())?;
        let historical = snapshot_at_revision(&current, &revision_v1.to_string())?;
        let old_sheet = historical.sheets.get(&sheet_id).unwrap();
        assert_eq!(old_sheet.definition.as_deref(), Some("game.Item"));
        assert_eq!(old_sheet.rows[0].value["legacy_name"], "old");
        assert!(old_sheet.rows[0].value.get("current_name").is_none());
        assert!(old_sheet
            .source_path
            .as_deref()
            .unwrap()
            .replace('\\', "/")
            .ends_with("/data/v1/items.json"));

        let head = snapshot_at_revision(&current, "HEAD")?;
        let new_sheet = head.sheets.get(&sheet_id).unwrap();
        assert_eq!(new_sheet.rows[0].value["current_name"], "new");
        assert!(new_sheet.rows[0].value.get("legacy_name").is_none());
        assert!(new_sheet
            .source_path
            .as_deref()
            .unwrap()
            .replace('\\', "/")
            .ends_with("/data/v2/items.json"));
        Ok(())
    }

    #[test]
    fn rejects_absolute_and_repository_escaping_revision_paths() {
        let base = Path::new("editor/game.polysheet");
        let absolute = resolve_revision_path(base, "/outside.poly", "workbook schema")
            .unwrap_err()
            .to_string();
        assert!(absolute.contains("must be relative to the Git repository"));
        assert!(resolve_revision_path(base, r"C:\outside.poly", "workbook schema").is_err());

        let escaping = resolve_revision_path(base, "../../../outside.poly", "workbook schema")
            .unwrap_err()
            .to_string();
        assert!(escaping.contains("escapes the Git repository"));
        assert!(resolve_revision_path(base, r"..\..\..\outside.poly", "workbook schema").is_err());
        assert!(revision_sheet_directory(base, "../other").is_err());
    }

    #[test]
    fn historical_snapshot_rejects_duplicate_workbook_sheet_ids() -> Result<()> {
        let temp = tempdir()?;
        let repository = Repository::init(temp.path())?;
        let root = temp.path().join("book.polysheet");
        let mut project = PolySheetProject::create_unsaved("Book");
        project.save_as(&root, true)?;

        let workbook_path = root.join("workbook.toml");
        let mut workbook: WorkbookManifest = toml::from_str(&fs::read_to_string(&workbook_path)?)?;
        workbook.sheets.push(workbook.sheets[0].clone());
        fs::write(&workbook_path, toml::to_string_pretty(&workbook)?)?;
        commit_all(&repository, "duplicate sheet id")?;

        let error = snapshot_at_revision(&project, "HEAD")
            .unwrap_err()
            .to_string();
        assert!(error.contains("duplicate sheet ids"));
        Ok(())
    }

    #[test]
    fn repository_path_resolves_parent_segments_before_containment_check() -> Result<()> {
        let temp = tempdir()?;
        let repository_root = temp.path().join("repo");
        let outside_root = temp.path().join("outside");
        fs::create_dir_all(repository_root.join("sub"))?;
        fs::create_dir_all(&outside_root)?;
        let outside_file = outside_root.join("data.json");
        fs::write(&outside_file, "[]")?;
        let repository = Repository::init(&repository_root)?;
        let workdir = repository.workdir().unwrap();
        let disguised_outside = repository_root.join("sub/../../outside/data.json");

        let error = repository_path(workdir, &disguised_outside)
            .unwrap_err()
            .to_string();
        assert!(error.contains("outside the Git repository"));
        Ok(())
    }

    #[test]
    fn stages_missing_tracked_sidecars_for_removed_sheets() -> Result<()> {
        let temp = tempdir()?;
        let repository = Repository::init(temp.path())?;
        let schema = temp.path().join("game.poly");
        let sources = temp.path().join("game.sources.toml");
        fs::create_dir_all(temp.path().join("data"))?;
        fs::write(
            &schema,
            "namespace game { table Item { id: u32 primary_key; } }",
        )?;
        fs::write(
            &sources,
            r#"[tables."game.Item".load]
json = "data/items.json"
"#,
        )?;
        fs::write(temp.path().join("data/items.json"), "[]")?;
        let root = temp.path().join("game.polysheet");
        let mut project = PolySheetProject::create(&root, "Game", &schema, Some(&sources), None)?;
        let removed_id = project.add_calculation_sheet("Removed");
        project.save(true)?;
        commit_all(&repository, "initial")?;

        project.manifest.sheets.retain(|id| id != &removed_id);
        project.sheets.remove(&removed_id);
        project.save(false)?;
        fs::remove_dir_all(root.join("sheets").join(&removed_id))?;

        let staged = stage_sheet_files(&project, &[])?;
        let removed_prefix = format!("game.polysheet/sheets/{removed_id}/");
        assert!(staged
            .iter()
            .any(|path| { path.starts_with(&removed_prefix) && path.ends_with("sheet.toml") }));
        assert!(staged
            .iter()
            .any(|path| path.starts_with(&removed_prefix) && path.ends_with("cells.json")));
        assert!(staged.iter().any(|path| path.ends_with("workbook.toml")));
        let index = Repository::open(temp.path())?.index()?;
        let remaining = index
            .iter()
            .map(|entry| String::from_utf8_lossy(&entry.path).into_owned())
            .filter(|path| path.starts_with(&removed_prefix))
            .collect::<Vec<_>>();
        assert!(
            remaining.is_empty(),
            "remaining index entries: {remaining:?}"
        );
        Ok(())
    }
}
