use crate::diff::{ProjectSnapshot, SnapshotSheet};
use crate::formula::{
    binding_has_broken_reference, capture_formula, render_formula, FormulaSheetLayout,
    FormulaWorkbookLayout,
};
use crate::model::{
    CalculationDocument, CalculationSheetDocument, DataSheetDocument, FormatDocument,
    FormulaDocument, InspectorField, RowComparison, RowComparisonField, RowComparisonValue,
    RowDraft, RowIdDocument, RowIdEntry, RowRecord, SheetDocument, SheetKind, SheetManifest,
    WorkbookManifest, FORMAT_VERSION,
};
use crate::transaction::{recover_transaction, write_transaction_with_deletions, FileUpdate};
use crate::validation::{
    canonical_scalar, primary_key, table_fields, validate_table_rows, Diagnostic,
    DiagnosticSeverity, SchemaIndex,
};
use anyhow::{bail, Context, Result};
use polygen::ir_model::{FieldDef, StructDef, TypeRef};
use polygen::LoadedProjectSchema;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Component, Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Default)]
pub struct OpenProjectOptions {
    pub allow_missing_data_files: bool,
}

#[derive(Debug, Clone)]
pub struct PolySheetProject {
    pub root: PathBuf,
    pub manifest: WorkbookManifest,
    pub loaded_schema: Option<LoadedProjectSchema>,
    pub schema_index: SchemaIndex,
    pub sheets: BTreeMap<String, SheetDocument>,
    pub diagnostics: Vec<Diagnostic>,
    pub normalization_required: Vec<PathBuf>,
    /// Sheet IDs removed by an applied structural merge and awaiting a
    /// transactionally persisted structural save.
    pub(crate) removed_sheet_ids: BTreeSet<String>,
}

impl PolySheetProject {
    pub fn create(
        root: impl AsRef<Path>,
        name: impl Into<String>,
        schema: impl AsRef<Path>,
        sources: Option<&Path>,
        data_root: Option<&Path>,
    ) -> Result<Self> {
        let root = absolute_path(root.as_ref())?;
        fs::create_dir_all(root.join("sheets"))?;
        let schema = validate_schema_path(schema.as_ref())?;
        let sources = sources.map(absolute_path).transpose()?;
        let data_root = data_root.map(absolute_path).transpose()?;
        let manifest = WorkbookManifest {
            format_version: FORMAT_VERSION,
            name: name.into(),
            schema: Some(relative_or_absolute(&root, &schema)),
            sources: sources
                .as_ref()
                .map(|path| relative_or_absolute(&root, path)),
            data_root: data_root
                .as_ref()
                .map(|path| relative_or_absolute(&root, path)),
            sheets: Vec::new(),
        };
        let loaded_schema = polygen::load_project_schema(&schema, sources.as_deref())?;
        let schema_index = SchemaIndex::from_schema(&loaded_schema.schema);
        Ok(Self {
            root,
            manifest,
            loaded_schema: Some(loaded_schema),
            schema_index,
            sheets: BTreeMap::new(),
            diagnostics: Vec::new(),
            normalization_required: Vec::new(),
            removed_sheet_ids: BTreeSet::new(),
        })
    }

    /// Create an in-memory calculation workbook. No files are written until
    /// `save_as` is called.
    pub fn create_unsaved(name: impl Into<String>) -> Self {
        let mut project = Self {
            root: PathBuf::new(),
            manifest: WorkbookManifest {
                format_version: FORMAT_VERSION,
                name: name.into(),
                schema: None,
                sources: None,
                data_root: None,
                sheets: Vec::new(),
            },
            loaded_schema: None,
            schema_index: SchemaIndex::default(),
            sheets: BTreeMap::new(),
            diagnostics: Vec::new(),
            normalization_required: Vec::new(),
            removed_sheet_ids: BTreeSet::new(),
        };
        project.add_calculation_sheet("Sheet 1");
        project
    }

    pub fn is_saved(&self) -> bool {
        !self.root.as_os_str().is_empty() && self.root.join("workbook.toml").exists()
    }

    pub fn attach_schema(
        &mut self,
        schema: impl AsRef<Path>,
        sources: Option<&Path>,
        data_root: Option<&Path>,
    ) -> Result<()> {
        if self
            .sheets
            .values()
            .any(|sheet| matches!(sheet, SheetDocument::Data(_)))
        {
            bail!("데이터 시트가 있는 프로젝트의 스키마는 교체할 수 없습니다");
        }
        let schema = validate_schema_path(schema.as_ref())?;
        let sources = sources.map(validate_sources_path).transpose()?;
        let data_root = data_root.map(absolute_path).transpose()?;
        let loaded_schema = polygen::load_project_schema(&schema, sources.as_deref())?;
        self.schema_index = SchemaIndex::from_schema(&loaded_schema.schema);
        self.manifest.schema = Some(manifest_path(&self.root, &schema));
        self.manifest.sources = sources.as_ref().map(|path| manifest_path(&self.root, path));
        self.manifest.data_root = data_root
            .as_ref()
            .map(|path| manifest_path(&self.root, path));
        self.loaded_schema = Some(loaded_schema);
        self.refresh_diagnostics();
        Ok(())
    }

    pub fn save_as(&mut self, root: impl AsRef<Path>, approve_normalization: bool) -> Result<()> {
        let root = absolute_path(root.as_ref())?;
        fs::create_dir_all(root.join("sheets"))
            .with_context(|| format!("프로젝트 폴더를 만들 수 없습니다: {}", root.display()))?;

        let old_root = self.root.clone();
        let schema = self
            .manifest
            .schema
            .as_deref()
            .map(|path| resolve_path(&old_root, path));
        let sources = self
            .manifest
            .sources
            .as_deref()
            .map(|path| resolve_path(&old_root, path));
        let data_root = self
            .manifest
            .data_root
            .as_deref()
            .map(|path| resolve_path(&old_root, path));

        self.root = root;
        if self.root != old_root {
            self.removed_sheet_ids.clear();
        }
        self.manifest.schema = schema
            .as_ref()
            .map(|path| relative_or_absolute(&self.root, path));
        self.manifest.sources = sources
            .as_ref()
            .map(|path| relative_or_absolute(&self.root, path));
        self.manifest.data_root = data_root
            .as_ref()
            .map(|path| relative_or_absolute(&self.root, path));
        self.save(approve_normalization)
    }

    pub fn open(root: impl AsRef<Path>, options: OpenProjectOptions) -> Result<Self> {
        let root = absolute_path(root.as_ref())?;
        recover_transaction(&root)?;
        let manifest_path = root.join("workbook.toml");
        let manifest: WorkbookManifest = read_toml(&manifest_path)?;
        if manifest.format_version != FORMAT_VERSION {
            bail!(
                "unsupported PolySheet format version {}; expected {}",
                manifest.format_version,
                FORMAT_VERSION
            );
        }
        let schema_path = manifest
            .schema
            .as_deref()
            .map(|path| resolve_path(&root, path));
        let sources_path = manifest
            .sources
            .as_deref()
            .map(|path| resolve_path(&root, path));
        let loaded_schema = schema_path
            .as_deref()
            .map(|path| polygen::load_project_schema(path, sources_path.as_deref()))
            .transpose()?;
        let schema_index = loaded_schema
            .as_ref()
            .map(|loaded| SchemaIndex::from_schema(&loaded.schema))
            .unwrap_or_default();
        let data_root = resolve_data_root(&root, &manifest, loaded_schema.as_ref());

        let mut project = Self {
            root,
            manifest,
            loaded_schema,
            schema_index,
            sheets: BTreeMap::new(),
            diagnostics: Vec::new(),
            normalization_required: Vec::new(),
            removed_sheet_ids: BTreeSet::new(),
        };

        let mut opened_sheet_ids = BTreeSet::new();
        let mut opened_definitions = BTreeSet::new();
        let mut opened_sources: Vec<(String, PathBuf)> = Vec::new();
        for sheet_id in project.manifest.sheets.clone() {
            validate_sheet_id(&sheet_id)?;
            if !opened_sheet_ids.insert(sheet_id.clone()) {
                bail!("workbook contains duplicate sheet id '{sheet_id}'");
            }
            let directory = project.sheet_directory(&sheet_id);
            let sheet_manifest: SheetManifest = read_toml(&directory.join("sheet.toml"))?;
            if sheet_manifest.id != sheet_id {
                bail!(
                    "sheet id mismatch: manifest references '{sheet_id}' but sheet.toml contains '{}'",
                    sheet_manifest.id
                );
            }
            let document = match sheet_manifest.kind {
                SheetKind::Data => project.open_data_sheet(
                    sheet_manifest,
                    &directory,
                    &data_root,
                    options.allow_missing_data_files,
                )?,
                SheetKind::Calculation => {
                    let mut document =
                        read_json_or_default::<CalculationDocument>(&directory.join("cells.json"))?;
                    let mut format =
                        read_json_or_default::<FormatDocument>(&directory.join("format.json"))?;
                    if document.version == 0 {
                        document.version = FORMAT_VERSION;
                    }
                    if format.version == 0 {
                        format.version = FORMAT_VERSION;
                    }
                    SheetDocument::Calculation(CalculationSheetDocument {
                        manifest: sheet_manifest,
                        document,
                        format,
                    })
                }
            };
            if let SheetDocument::Data(data) = &document {
                let definition = data
                    .manifest
                    .definition
                    .as_deref()
                    .context("data sheet has no definition")?;
                if !opened_definitions.insert(definition.to_string()) {
                    bail!("table definition '{definition}' is bound to more than one data sheet");
                }
                let source_path = PathBuf::from(&data.source_path);
                for (existing_sheet, existing) in &opened_sources {
                    if data_sources_equivalent(existing, &source_path)? {
                        bail!(
                            "data sheets '{existing_sheet}' and '{sheet_id}' share JSON source '{}'",
                            source_path.display()
                        );
                    }
                }
                opened_sources.push((sheet_id.clone(), source_path));
            }
            project.sheets.insert(sheet_id, document);
        }
        project.backfill_formula_bindings();
        project.refresh_normalization_required()?;
        project.refresh_diagnostics();
        Ok(project)
    }

    pub fn bind_data_sheet(
        &mut self,
        name: impl Into<String>,
        definition: impl Into<String>,
    ) -> Result<String> {
        let definition = definition.into();
        if self.loaded_schema.is_none() {
            bail!("먼저 .poly 스키마를 연결해 주세요");
        }
        if self.sheets.values().any(|sheet| {
            matches!(
                sheet,
                SheetDocument::Data(data)
                    if data.manifest.definition.as_deref() == Some(definition.as_str())
            )
        }) {
            bail!("table definition '{definition}' is already bound to a data sheet");
        }
        let table = self
            .schema_index
            .table(&definition)
            .with_context(|| format!("unknown table definition '{definition}'"))?
            .clone();
        require_readable_json_source(&table)?;
        let id = Uuid::now_v7().to_string();
        let manifest = SheetManifest {
            id: id.clone(),
            name: name.into(),
            kind: SheetKind::Data,
            definition: Some(definition.clone()),
            row_count: 0,
            column_count: table_fields(&table).len(),
        };
        let data_root = resolve_data_root(&self.root, &self.manifest, self.loaded_schema.as_ref());
        let document =
            self.open_data_sheet(manifest, &self.sheet_directory(&id), &data_root, true)?;
        if let SheetDocument::Data(data) = &document {
            let source_path = Path::new(&data.source_path);
            for sheet in self.sheets.values() {
                if let SheetDocument::Data(existing) = sheet {
                    if data_sources_equivalent(Path::new(&existing.source_path), source_path)? {
                        bail!(
                            "data sheet '{}' and table '{}' share JSON source '{}'",
                            existing.manifest.name,
                            definition,
                            source_path.display()
                        );
                    }
                }
            }
        }
        self.manifest.sheets.push(id.clone());
        self.sheets.insert(id.clone(), document);
        self.refresh_normalization_required()?;
        self.refresh_diagnostics();
        Ok(id)
    }

    pub fn add_calculation_sheet(&mut self, name: impl Into<String>) -> String {
        let id = Uuid::now_v7().to_string();
        let row_order = (0..100).map(|_| Uuid::now_v7().to_string()).collect();
        let column_order = (0..26).map(|_| Uuid::now_v7().to_string()).collect();
        let manifest = SheetManifest {
            id: id.clone(),
            name: name.into(),
            kind: SheetKind::Calculation,
            definition: None,
            row_count: 100,
            column_count: 26,
        };
        self.manifest.sheets.push(id.clone());
        self.sheets.insert(
            id.clone(),
            SheetDocument::Calculation(CalculationSheetDocument {
                manifest,
                document: CalculationDocument {
                    version: FORMAT_VERSION,
                    needs_recalculation: false,
                    row_order,
                    column_order,
                    cells: BTreeMap::new(),
                    formula_bindings: BTreeMap::new(),
                },
                format: FormatDocument {
                    version: FORMAT_VERSION,
                    ..FormatDocument::default()
                },
            }),
        );
        id
    }

    pub fn formula_document(&self, sheet_id: &str) -> Result<FormulaDocument> {
        match self.sheets.get(sheet_id) {
            Some(SheetDocument::Data(sheet)) => {
                let mut document = sheet.formulas.clone();
                let layout = self.formula_layout();
                for (row_id, fields) in &mut document.rows {
                    for (field_path, formula) in fields {
                        if let Some(binding) = document
                            .bindings
                            .get(row_id)
                            .and_then(|bindings| bindings.get(field_path))
                        {
                            *formula = render_formula(binding, sheet_id, &layout);
                        }
                    }
                }
                Ok(document)
            }
            _ => bail!("unknown data sheet '{sheet_id}'"),
        }
    }

    pub fn calculation_document(&self, sheet_id: &str) -> Result<CalculationDocument> {
        match self.sheets.get(sheet_id) {
            Some(SheetDocument::Calculation(sheet)) => {
                let mut document = sheet.document.clone();
                let layout = self.formula_layout();
                for (cell_id, cell) in &mut document.cells {
                    if let (Some(formula), Some(binding)) = (
                        cell.formula.as_mut(),
                        document.formula_bindings.get(cell_id),
                    ) {
                        *formula = render_formula(binding, sheet_id, &layout);
                    }
                }
                Ok(document)
            }
            _ => bail!("unknown calculation sheet '{sheet_id}'"),
        }
    }

    pub fn replace_calculation_document(
        &mut self,
        sheet_id: &str,
        mut document: CalculationDocument,
    ) -> Result<()> {
        if document.version != FORMAT_VERSION {
            bail!(
                "unsupported calculation document version {}; expected {}",
                document.version,
                FORMAT_VERSION
            );
        }
        let unique_rows = document
            .row_order
            .iter()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>();
        let unique_columns = document
            .column_order
            .iter()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>();
        if unique_rows.len() != document.row_order.len()
            || unique_columns.len() != document.column_order.len()
        {
            bail!("calculation sheet row and column IDs must be unique");
        }
        for key in document.cells.keys() {
            let Some((row_id, column_id)) = key.split_once(':') else {
                bail!("invalid calculation cell key '{key}'");
            };
            if !unique_rows.contains(row_id) || !unique_columns.contains(column_id) {
                bail!("calculation cell '{key}' references an unknown row or column ID");
            }
        }
        let layout = self.formula_layout_with_calculation(sheet_id, &document);
        document.formula_bindings.clear();
        for (cell_id, cell) in &document.cells {
            if let Some(formula) = cell
                .formula
                .as_deref()
                .filter(|formula| !formula.trim().is_empty())
            {
                document
                    .formula_bindings
                    .insert(cell_id.clone(), capture_formula(formula, sheet_id, &layout));
            }
        }
        let sheet = match self.sheets.get_mut(sheet_id) {
            Some(SheetDocument::Calculation(sheet)) => sheet,
            _ => bail!("unknown calculation sheet '{sheet_id}'"),
        };
        sheet.manifest.row_count = document.row_order.len();
        sheet.manifest.column_count = document.column_order.len();
        sheet.document = document;
        Ok(())
    }

    pub fn insert_calculation_row(&mut self, sheet_id: &str, index: usize) -> Result<String> {
        self.backfill_formula_bindings();
        let sheet = match self.sheets.get_mut(sheet_id) {
            Some(SheetDocument::Calculation(sheet)) => sheet,
            _ => bail!("unknown calculation sheet '{sheet_id}'"),
        };
        let id = Uuid::now_v7().to_string();
        sheet
            .document
            .row_order
            .insert(index.min(sheet.document.row_order.len()), id.clone());
        sheet.manifest.row_count = sheet.document.row_order.len();
        Ok(id)
    }

    pub fn insert_calculation_column(&mut self, sheet_id: &str, index: usize) -> Result<String> {
        self.backfill_formula_bindings();
        let sheet = match self.sheets.get_mut(sheet_id) {
            Some(SheetDocument::Calculation(sheet)) => sheet,
            _ => bail!("unknown calculation sheet '{sheet_id}'"),
        };
        let id = Uuid::now_v7().to_string();
        sheet
            .document
            .column_order
            .insert(index.min(sheet.document.column_order.len()), id.clone());
        sheet.manifest.column_count = sheet.document.column_order.len();
        Ok(id)
    }

    pub fn replace_data_sheet_json(&mut self, sheet_id: &str, json: &str) -> Result<()> {
        let parsed: Value = serde_json::from_str(json)?;
        let rows = parsed
            .as_array()
            .context("data sheet JSON must be an array")?
            .clone();
        let definition = match self.sheets.get(sheet_id) {
            Some(SheetDocument::Data(sheet)) => sheet
                .manifest
                .definition
                .as_deref()
                .context("data sheet is missing its definition")?
                .to_string(),
            _ => bail!("unknown data sheet '{sheet_id}'"),
        };
        let table = self
            .schema_index
            .table(&definition)
            .context("bound table no longer exists")?
            .clone();
        require_editable_json_source(&table)?;
        let primary_key = primary_key(&table).map(|field| field.name.clone());
        let sheet = match self.sheets.get_mut(sheet_id) {
            Some(SheetDocument::Data(sheet)) => sheet,
            _ => unreachable!(),
        };
        let (records, row_ids, issues) = build_row_records(
            &rows,
            &table,
            sheet.row_ids.clone(),
            sheet.row_ids.is_none(),
        );
        sheet.rows = records;
        sheet.row_ids = row_ids;
        sheet.identity_issues = issues;
        sheet.primary_key = primary_key;
        sheet.manifest.row_count = sheet.rows.len();
        self.refresh_diagnostics();
        Ok(())
    }

    pub fn get_row_json(&self, sheet_id: &str, row_id: &str) -> Result<String> {
        let (sheet, table) = self.data_sheet_and_table(sheet_id)?;
        let row = sheet
            .rows
            .iter()
            .find(|row| row.id == row_id)
            .with_context(|| format!("unknown row id '{row_id}'"))?;
        let canonical = canonicalize_struct(&row.value, table, &self.schema_index)?;
        let mut output = serde_json::to_string_pretty(&canonical)?;
        output.push('\n');
        Ok(output)
    }

    pub fn apply_row_json(&mut self, sheet_id: &str, row_id: &str, json: &str) -> Result<()> {
        let mut value: Value = serde_json::from_str(json)?;
        if !value.is_object() {
            bail!("row JSON must be an object");
        }
        let (definition, old_value, old_primary_key) = {
            let (sheet, table) = self.data_sheet_and_table(sheet_id)?;
            require_editable_json_source(table)?;
            let row = sheet
                .rows
                .iter()
                .find(|row| row.id == row_id)
                .with_context(|| format!("unknown row id '{row_id}'"))?;
            (
                table.fqn.clone(),
                row.value.clone(),
                primary_key(table).map(|field| field.name.clone()),
            )
        };
        if let Some(primary_key) = &old_primary_key {
            if old_value.get(primary_key) != value.get(primary_key) {
                bail!("primary key fields cannot be edited in place");
            }
        }
        let table = self
            .schema_index
            .table(&definition)
            .context("bound table no longer exists")?;
        value = coerce_struct_value(value, table, &self.schema_index)?;
        let mut rows = match self.sheets.get(sheet_id) {
            Some(SheetDocument::Data(sheet)) => sheet
                .rows
                .iter()
                .map(|row| {
                    if row.id == row_id {
                        value.clone()
                    } else {
                        row.value.clone()
                    }
                })
                .collect::<Vec<_>>(),
            _ => bail!("unknown data sheet '{sheet_id}'"),
        };
        ensure_valid_rows(table, &rows, &self.schema_index)?;
        let sheet = match self.sheets.get_mut(sheet_id) {
            Some(SheetDocument::Data(sheet)) => sheet,
            _ => unreachable!(),
        };
        let index = sheet
            .rows
            .iter()
            .position(|row| row.id == row_id)
            .context("row disappeared while applying JSON")?;
        sheet.rows[index].value = rows.swap_remove(index);
        if let Some(row_ids) = &mut sheet.row_ids {
            row_ids.rows[index].fingerprint = fingerprint(&sheet.rows[index].value);
        }
        self.refresh_diagnostics();
        Ok(())
    }

    pub fn get_row_draft(&self, sheet_id: &str) -> Result<RowDraft> {
        let (_sheet, table) = self.data_sheet_and_table(sheet_id)?;
        let (editable, readonly_reason) = match require_editable_json_source(table) {
            Ok(_) => (true, None),
            Err(error) => (false, Some(error.to_string())),
        };
        let mut object = Map::new();
        let mut missing_required = Vec::new();
        for field in table_fields(table) {
            if let Some(default) = &field.default_value {
                object.insert(
                    field.name.clone(),
                    parse_default_value(default, &field.field_type),
                );
            } else if field.field_type.is_option {
                object.insert(field.name.clone(), Value::Null);
            } else if field.field_type.is_list {
                object.insert(field.name.clone(), Value::Array(Vec::new()));
            } else {
                missing_required.push(field.name.clone());
            }
        }
        let canonical = canonicalize_struct(&Value::Object(object), table, &self.schema_index)?;
        let mut json = serde_json::to_string_pretty(&canonical)?;
        json.push('\n');
        Ok(RowDraft {
            json,
            missing_required,
            fields: inspector_fields(table, &self.schema_index),
            editable,
            readonly_reason,
        })
    }

    pub fn insert_row_json(&mut self, sheet_id: &str, json: &str) -> Result<String> {
        let mut value: Value = serde_json::from_str(json)?;
        if !value.is_object() {
            bail!("new row JSON must be an object");
        }
        let definition = {
            let (_sheet, table) = self.data_sheet_and_table(sheet_id)?;
            require_editable_json_source(table)?;
            table.fqn.clone()
        };
        let table = self
            .schema_index
            .table(&definition)
            .context("bound table no longer exists")?;
        value = coerce_struct_value(value, table, &self.schema_index)?;
        let mut values = match self.sheets.get(sheet_id) {
            Some(SheetDocument::Data(sheet)) => sheet
                .rows
                .iter()
                .map(|row| row.value.clone())
                .collect::<Vec<_>>(),
            _ => bail!("unknown data sheet '{sheet_id}'"),
        };
        values.push(value.clone());
        ensure_valid_rows(table, &values, &self.schema_index)?;
        let id = if let Some(primary) = primary_key(table) {
            format!(
                "pk:{}:{}",
                primary.field_type.type_name,
                canonical_scalar(value.get(&primary.name).unwrap_or(&Value::Null))
            )
        } else {
            Uuid::now_v7().to_string()
        };
        let sheet = match self.sheets.get_mut(sheet_id) {
            Some(SheetDocument::Data(sheet)) => sheet,
            _ => unreachable!(),
        };
        sheet.rows.push(RowRecord {
            id: id.clone(),
            value: value.clone(),
        });
        if let Some(row_ids) = &mut sheet.row_ids {
            row_ids.rows.push(RowIdEntry {
                id: id.clone(),
                fingerprint: fingerprint(&value),
            });
        }
        sheet.manifest.row_count = sheet.rows.len();
        self.refresh_diagnostics();
        Ok(id)
    }

    pub fn compare_rows(&self, sheet_id: &str, row_ids: &[String]) -> Result<RowComparison> {
        if row_ids.len() < 2 {
            bail!("row comparison requires at least two rows");
        }
        if row_ids.len() > 100 {
            bail!("interactive row comparison supports at most 100 rows");
        }
        let (sheet, table) = self.data_sheet_and_table(sheet_id)?;
        let rows = row_ids
            .iter()
            .map(|row_id| {
                sheet
                    .rows
                    .iter()
                    .find(|row| &row.id == row_id)
                    .with_context(|| format!("unknown row id '{row_id}'"))
            })
            .collect::<Result<Vec<_>>>()?;
        let mut fields = Vec::new();
        collect_comparison_fields(
            "",
            table,
            &self.schema_index,
            &rows,
            &sheet.formulas,
            &mut fields,
        );
        Ok(RowComparison {
            row_ids: row_ids.to_vec(),
            fields,
        })
    }

    fn data_sheet_and_table(&self, sheet_id: &str) -> Result<(&DataSheetDocument, &StructDef)> {
        let sheet = match self.sheets.get(sheet_id) {
            Some(SheetDocument::Data(sheet)) => sheet,
            _ => bail!("unknown data sheet '{sheet_id}'"),
        };
        let table = self
            .schema_index
            .table(
                sheet
                    .manifest
                    .definition
                    .as_deref()
                    .context("data sheet has no definition")?,
            )
            .context("bound table no longer exists")?;
        Ok((sheet, table))
    }

    pub fn patch_cell(
        &mut self,
        sheet_id: &str,
        row_id: &str,
        field_path: &str,
        value: Value,
    ) -> Result<()> {
        let definition = match self.sheets.get(sheet_id) {
            Some(SheetDocument::Data(sheet)) => sheet
                .manifest
                .definition
                .as_deref()
                .context("data sheet has no definition")?
                .to_string(),
            _ => bail!("unknown data sheet '{sheet_id}'"),
        };
        let table = self
            .schema_index
            .table(&definition)
            .context("bound table no longer exists")?;
        require_editable_json_source(table)?;
        let field = field_at_path(table, field_path, &self.schema_index)
            .with_context(|| format!("unknown field path '{field_path}'"))?;
        if field.is_primary_key {
            bail!("primary key fields cannot be edited in place");
        }
        let value = coerce_frontend_value(value, &field.field_type)?;
        let sheet = match self.sheets.get_mut(sheet_id) {
            Some(SheetDocument::Data(sheet)) => sheet,
            _ => unreachable!(),
        };
        let row = sheet
            .rows
            .iter_mut()
            .find(|row| row.id == row_id)
            .with_context(|| format!("unknown row id '{row_id}'"))?;
        set_value_at_path(&mut row.value, field_path, value)?;
        if let Some(row_ids) = &mut sheet.row_ids {
            if let Some(index) = sheet.rows.iter().position(|row| row.id == row_id) {
                row_ids.rows[index].fingerprint = fingerprint(&sheet.rows[index].value);
            }
        }
        self.refresh_diagnostics();
        Ok(())
    }

    pub fn apply_row_order(&mut self, sheet_id: &str, row_ids: &[String]) -> Result<()> {
        let (_sheet, table) = self.data_sheet_and_table(sheet_id)?;
        require_editable_json_source(table)?;
        let sheet = match self.sheets.get_mut(sheet_id) {
            Some(SheetDocument::Data(sheet)) => sheet,
            _ => bail!("unknown data sheet '{sheet_id}'"),
        };
        let current = sheet
            .rows
            .iter()
            .map(|row| row.id.clone())
            .collect::<BTreeSet<_>>();
        let requested = row_ids.iter().cloned().collect::<BTreeSet<_>>();
        if current != requested || requested.len() != row_ids.len() {
            bail!("row order must contain every stable row ID exactly once");
        }
        let mut rows = std::mem::take(&mut sheet.rows)
            .into_iter()
            .map(|row| (row.id.clone(), row))
            .collect::<BTreeMap<_, _>>();
        let mut ordered_rows = Vec::with_capacity(row_ids.len());
        for id in row_ids {
            ordered_rows.push(
                rows.remove(id)
                    .with_context(|| format!("row '{id}' disappeared while reordering"))?,
            );
        }
        sheet.rows = ordered_rows;
        if let Some(row_ids_document) = &mut sheet.row_ids {
            let by_id = std::mem::take(&mut row_ids_document.rows)
                .into_iter()
                .map(|entry| (entry.id.clone(), entry))
                .collect::<BTreeMap<_, _>>();
            row_ids_document.rows = row_ids
                .iter()
                .filter_map(|id| by_id.get(id).cloned())
                .collect();
        }
        Ok(())
    }

    pub fn confirm_row_identity(
        &mut self,
        sheet_id: &str,
        current_row_id: &str,
        replacement_row_id: &str,
    ) -> Result<()> {
        Uuid::parse_str(replacement_row_id)
            .with_context(|| format!("'{replacement_row_id}' is not a valid UUID row ID"))?;
        let sheet = match self.sheets.get_mut(sheet_id) {
            Some(SheetDocument::Data(sheet)) if sheet.row_ids.is_some() => sheet,
            Some(SheetDocument::Data(_)) => {
                bail!("primary-key data sheets do not use row identity confirmation")
            }
            _ => bail!("unknown data sheet '{sheet_id}'"),
        };
        if current_row_id != replacement_row_id
            && sheet.rows.iter().any(|row| row.id == replacement_row_id)
        {
            bail!("row ID '{replacement_row_id}' is already assigned");
        }
        let row = sheet
            .rows
            .iter_mut()
            .find(|row| row.id == current_row_id)
            .with_context(|| format!("unknown row ID '{current_row_id}'"))?;
        row.id = replacement_row_id.to_string();
        if let Some(row_ids) = &mut sheet.row_ids {
            let entry = row_ids
                .rows
                .iter_mut()
                .find(|entry| entry.id == current_row_id)
                .context("row ID sidecar is out of sync")?;
            entry.id = replacement_row_id.to_string();
        }
        if current_row_id != replacement_row_id {
            if let Some(current_formulas) = sheet.formulas.rows.remove(current_row_id) {
                sheet
                    .formulas
                    .rows
                    .entry(replacement_row_id.to_string())
                    .or_default()
                    .extend(current_formulas);
            }
            if let Some(current_bindings) = sheet.formulas.bindings.remove(current_row_id) {
                sheet
                    .formulas
                    .bindings
                    .entry(replacement_row_id.to_string())
                    .or_default()
                    .extend(current_bindings);
            }
        }
        sheet
            .identity_issues
            .retain(|issue| !issue.contains(current_row_id));
        self.refresh_diagnostics();
        Ok(())
    }

    pub fn set_formula(
        &mut self,
        sheet_id: &str,
        row_id: &str,
        field_path: &str,
        formula: Option<String>,
        calculated_value: Value,
    ) -> Result<()> {
        let formula_binding = formula
            .as_deref()
            .filter(|formula| !formula.trim().is_empty())
            .map(|formula| capture_formula(formula, sheet_id, &self.formula_layout()));
        if formula
            .as_deref()
            .is_some_and(|value| !value.trim().is_empty())
        {
            let definition = match self.sheets.get(sheet_id) {
                Some(SheetDocument::Data(sheet)) => sheet
                    .manifest
                    .definition
                    .as_deref()
                    .context("data sheet has no definition")?,
                _ => bail!("unknown data sheet '{sheet_id}'"),
            };
            let table = self
                .schema_index
                .table(definition)
                .context("bound table no longer exists")?;
            let field = field_at_path(table, field_path, &self.schema_index)
                .with_context(|| format!("unknown field path '{field_path}'"))?;
            if is_unsafe_formula_integer(&calculated_value, &field.field_type) {
                bail!(
                    "formula result for '{field_path}' exceeds the JavaScript safe integer range (#NUM!)"
                );
            }
        }
        self.patch_cell(sheet_id, row_id, field_path, calculated_value)?;
        let sheet = match self.sheets.get_mut(sheet_id) {
            Some(SheetDocument::Data(sheet)) => sheet,
            _ => bail!("unknown data sheet '{sheet_id}'"),
        };
        let fields = sheet.formulas.rows.entry(row_id.to_string()).or_default();
        if let Some(formula) = formula.filter(|formula| !formula.trim().is_empty()) {
            fields.insert(field_path.to_string(), formula);
            if let Some(binding) = formula_binding {
                sheet
                    .formulas
                    .bindings
                    .entry(row_id.to_string())
                    .or_default()
                    .insert(field_path.to_string(), binding);
            }
        } else {
            fields.remove(field_path);
            if let Some(bindings) = sheet.formulas.bindings.get_mut(row_id) {
                bindings.remove(field_path);
                if bindings.is_empty() {
                    sheet.formulas.bindings.remove(row_id);
                }
            }
        }
        if fields.is_empty() {
            sheet.formulas.rows.remove(row_id);
        }
        sheet.formulas.needs_recalculation = false;
        Ok(())
    }

    pub fn data_sheet_json(&self, sheet_id: &str) -> Result<String> {
        let sheet = match self.sheets.get(sheet_id) {
            Some(SheetDocument::Data(sheet)) => sheet,
            _ => bail!("unknown data sheet '{sheet_id}'"),
        };
        let table = self
            .schema_index
            .table(
                sheet
                    .manifest
                    .definition
                    .as_deref()
                    .context("data sheet has no definition")?,
            )
            .context("bound table no longer exists")?;
        canonical_rows_json(
            &sheet
                .rows
                .iter()
                .map(|row| row.value.clone())
                .collect::<Vec<_>>(),
            table,
            &self.schema_index,
        )
    }

    pub fn refresh_diagnostics(&mut self) {
        let mut diagnostics = Vec::new();
        let formula_layout = self.formula_layout();
        for sheet in self.sheets.values() {
            if let SheetDocument::Data(sheet) = sheet {
                if sheet.formulas.needs_recalculation {
                    diagnostics.push(Diagnostic {
                        severity: DiagnosticSeverity::Error,
                        path: sheet.manifest.name.clone(),
                        message: "merged formulas must be recalculated before saving".to_string(),
                    });
                }
                for (row_id, fields) in &sheet.formulas.bindings {
                    for (field_path, binding) in fields {
                        if binding_has_broken_reference(binding, &formula_layout) {
                            diagnostics.push(Diagnostic {
                                severity: DiagnosticSeverity::Error,
                                path: format!("{}.{}.{}", sheet.manifest.name, row_id, field_path),
                                message: "수식이 삭제된 행 또는 열을 참조합니다 (#REF!)"
                                    .to_string(),
                            });
                        }
                    }
                }
                for issue in &sheet.identity_issues {
                    diagnostics.push(Diagnostic {
                        severity: DiagnosticSeverity::Error,
                        path: sheet.manifest.name.clone(),
                        message: issue.clone(),
                    });
                }
                if let Some(definition) = sheet.manifest.definition.as_deref() {
                    if let Some(table) = self.schema_index.table(definition) {
                        let rows = sheet
                            .rows
                            .iter()
                            .map(|row| row.value.clone())
                            .collect::<Vec<_>>();
                        diagnostics.extend(validate_table_rows(table, &rows, &self.schema_index));
                    }
                }
            } else if let SheetDocument::Calculation(sheet) = sheet {
                if sheet.document.needs_recalculation {
                    diagnostics.push(Diagnostic {
                        severity: DiagnosticSeverity::Error,
                        path: sheet.manifest.name.clone(),
                        message: "merged formulas must be recalculated before saving".to_string(),
                    });
                }
                for (cell_id, binding) in &sheet.document.formula_bindings {
                    if binding_has_broken_reference(binding, &formula_layout) {
                        diagnostics.push(Diagnostic {
                            severity: DiagnosticSeverity::Error,
                            path: format!("{}.{}", sheet.manifest.name, cell_id),
                            message: "수식이 삭제된 행 또는 열을 참조합니다 (#REF!)".to_string(),
                        });
                    }
                }
            }
        }
        for sheet in self.sheets.values() {
            let SheetDocument::Data(source) = sheet else {
                continue;
            };
            let Some(definition) = source.manifest.definition.as_deref() else {
                continue;
            };
            let Some(table) = self.schema_index.table(definition) else {
                continue;
            };
            for field in table_fields(table) {
                let Some(foreign_key) = field.foreign_key.as_ref() else {
                    continue;
                };
                let target = self.sheets.values().find_map(|candidate| match candidate {
                    SheetDocument::Data(target)
                        if target.manifest.definition.as_deref()
                            == Some(foreign_key.target_table_fqn.as_str()) =>
                    {
                        Some(target)
                    }
                    _ => None,
                });
                let Some(target) = target else {
                    diagnostics.push(Diagnostic {
                        severity: DiagnosticSeverity::Warning,
                        path: format!("{}.{}", source.manifest.name, field.name),
                        message: format!(
                            "foreign-key target '{}' is not open as a data sheet",
                            foreign_key.target_table_fqn
                        ),
                    });
                    continue;
                };
                let target_values = target
                    .rows
                    .iter()
                    .filter_map(|row| row.value.get(&foreign_key.target_field))
                    .map(canonical_scalar)
                    .collect::<BTreeSet<_>>();
                for (row_index, row) in source.rows.iter().enumerate() {
                    let Some(value) = row.value.get(&field.name) else {
                        continue;
                    };
                    if value.is_null() {
                        continue;
                    }
                    if !target_values.contains(&canonical_scalar(value)) {
                        diagnostics.push(Diagnostic {
                            severity: DiagnosticSeverity::Error,
                            path: format!("{}[{}].{}", source.manifest.name, row_index, field.name),
                            message: format!(
                                "foreign key does not exist in {}.{}",
                                foreign_key.target_table_fqn, foreign_key.target_field
                            ),
                        });
                    }
                }
            }
        }
        self.diagnostics = diagnostics;
    }

    fn formula_layout(&self) -> FormulaWorkbookLayout {
        self.formula_layout_with_override(None)
    }

    fn formula_layout_with_calculation(
        &self,
        sheet_id: &str,
        document: &CalculationDocument,
    ) -> FormulaWorkbookLayout {
        self.formula_layout_with_override(Some((sheet_id, document)))
    }

    fn formula_layout_with_override(
        &self,
        calculation_override: Option<(&str, &CalculationDocument)>,
    ) -> FormulaWorkbookLayout {
        let layouts = self
            .manifest
            .sheets
            .iter()
            .filter_map(|sheet_id| {
                let sheet = self.sheets.get(sheet_id)?;
                match sheet {
                    SheetDocument::Data(sheet) => {
                        let definition = sheet.manifest.definition.as_deref()?;
                        let table = self.schema_index.table(definition)?;
                        Some(FormulaSheetLayout {
                            id: sheet_id.clone(),
                            name: sheet.manifest.name.clone(),
                            row_ids: sheet.rows.iter().map(|row| row.id.clone()).collect(),
                            column_ids: table_fields(table)
                                .into_iter()
                                .map(|field| field.name.clone())
                                .collect(),
                            row_offset: 1,
                            column_offset: 1,
                        })
                    }
                    SheetDocument::Calculation(sheet) => {
                        let document = calculation_override
                            .filter(|(override_id, _)| *override_id == sheet_id)
                            .map(|(_, document)| document)
                            .unwrap_or(&sheet.document);
                        Some(FormulaSheetLayout {
                            id: sheet_id.clone(),
                            name: sheet.manifest.name.clone(),
                            row_ids: document.row_order.clone(),
                            column_ids: document.column_order.clone(),
                            row_offset: 0,
                            column_offset: 0,
                        })
                    }
                }
            })
            .collect::<Vec<_>>();
        FormulaWorkbookLayout::new(layouts)
    }

    fn backfill_formula_bindings(&mut self) {
        let layout = self.formula_layout();
        for (sheet_id, sheet) in &mut self.sheets {
            match sheet {
                SheetDocument::Data(sheet) => {
                    for (row_id, fields) in &sheet.formulas.rows {
                        for (field_path, formula) in fields {
                            let current = sheet
                                .formulas
                                .bindings
                                .get(row_id)
                                .and_then(|bindings| bindings.get(field_path));
                            if current.is_none() {
                                sheet
                                    .formulas
                                    .bindings
                                    .entry(row_id.clone())
                                    .or_default()
                                    .insert(
                                        field_path.clone(),
                                        capture_formula(formula, sheet_id, &layout),
                                    );
                            }
                        }
                    }
                }
                SheetDocument::Calculation(sheet) => {
                    for (cell_id, cell) in &sheet.document.cells {
                        let Some(formula) = cell.formula.as_deref() else {
                            continue;
                        };
                        sheet
                            .document
                            .formula_bindings
                            .entry(cell_id.clone())
                            .or_insert_with(|| capture_formula(formula, sheet_id, &layout));
                    }
                }
            }
        }
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
    }

    pub fn save(&mut self, approve_normalization: bool) -> Result<()> {
        if self.root.as_os_str().is_empty() {
            bail!("처음 저장할 프로젝트 폴더를 선택해 주세요");
        }
        self.refresh_diagnostics();
        if self.has_errors() {
            bail!("project contains validation or row identity errors");
        }
        if !approve_normalization && !self.normalization_required.is_empty() {
            bail!("JSON normalization approval is required before the first save");
        }

        let mut updates = vec![FileUpdate {
            path: self.root.join("workbook.toml"),
            contents: canonical_toml(&self.manifest)?.into_bytes(),
        }];
        let sheet_ids = self.manifest.sheets.clone();
        let root = self.root.clone();
        let schema_index = &self.schema_index;
        for sheet_id in sheet_ids {
            let directory = root.join("sheets").join(&sheet_id);
            let document = self
                .sheets
                .get_mut(&sheet_id)
                .with_context(|| format!("missing sheet document '{sheet_id}'"))?;
            match document {
                SheetDocument::Data(sheet) => {
                    sheet.manifest.row_count = sheet.rows.len();
                    let definition = sheet
                        .manifest
                        .definition
                        .as_deref()
                        .context("data sheet has no definition")?;
                    let table = schema_index
                        .table(definition)
                        .context("bound table no longer exists")?;
                    let json = canonical_rows_json(
                        &sheet
                            .rows
                            .iter()
                            .map(|row| row.value.clone())
                            .collect::<Vec<_>>(),
                        table,
                        schema_index,
                    )?;
                    updates.push(toml_update(directory.join("sheet.toml"), &sheet.manifest)?);
                    updates.push(json_update(
                        directory.join("formulas.json"),
                        &sheet.formulas,
                    )?);
                    updates.push(json_update(directory.join("format.json"), &sheet.format)?);
                    if let Some(row_ids) = &mut sheet.row_ids {
                        row_ids.source_hash = fingerprint(&Value::Array(
                            sheet.rows.iter().map(|row| row.value.clone()).collect(),
                        ));
                        updates.push(json_update(directory.join("rowids.json"), row_ids)?);
                    }
                    updates.push(FileUpdate {
                        path: PathBuf::from(&sheet.source_path),
                        contents: json.into_bytes(),
                    });
                }
                SheetDocument::Calculation(sheet) => {
                    updates.push(toml_update(directory.join("sheet.toml"), &sheet.manifest)?);
                    updates.push(json_update(directory.join("cells.json"), &sheet.document)?);
                    updates.push(json_update(directory.join("format.json"), &sheet.format)?);
                }
            }
        }
        let mut deletions = Vec::new();
        for sheet_id in &self.removed_sheet_ids {
            deletions.extend(sheet_sidecar_deletion_paths(&self.root, sheet_id)?);
        }
        write_transaction_with_deletions(&self.root, &updates, &deletions)?;
        for sheet_id in &self.removed_sheet_ids {
            remove_empty_sheet_sidecar_directory(&self.root, sheet_id)?;
        }
        self.removed_sheet_ids.clear();
        self.normalization_required.clear();
        Ok(())
    }

    pub fn normalization_preview(&self) -> Result<Vec<(String, String, String)>> {
        let mut preview = Vec::new();
        for path in &self.normalization_required {
            let before = fs::read_to_string(path)
                .with_context(|| format!("failed to read {}", path.display()))?;
            let Some(SheetDocument::Data(sheet)) = self.sheets.values().find(|sheet| {
                matches!(sheet, SheetDocument::Data(data) if Path::new(&data.source_path) == path)
            }) else {
                continue;
            };
            let definition = sheet
                .manifest
                .definition
                .as_deref()
                .context("data sheet has no definition")?;
            let table = self
                .schema_index
                .table(definition)
                .context("bound table no longer exists")?;
            let after = canonical_rows_json(
                &sheet
                    .rows
                    .iter()
                    .map(|row| row.value.clone())
                    .collect::<Vec<_>>(),
                table,
                &self.schema_index,
            )?;
            preview.push((path.to_string_lossy().to_string(), before, after));
        }
        Ok(preview)
    }

    pub fn apply_snapshot(&mut self, snapshot: &ProjectSnapshot) -> Result<()> {
        let data_root = resolve_data_root(&self.root, &self.manifest, self.loaded_schema.as_ref());
        let mut next_sheets = BTreeMap::new();
        let mut definitions = BTreeSet::new();
        let mut data_sources: Vec<(String, PathBuf)> = Vec::new();

        for (sheet_id, snapshot_sheet) in &snapshot.sheets {
            validate_sheet_id(sheet_id)?;
            if snapshot_sheet.id != *sheet_id {
                bail!(
                    "snapshot sheet key '{sheet_id}' does not match embedded id '{}'",
                    snapshot_sheet.id
                );
            }
            let document = sheet_document_from_snapshot(
                snapshot_sheet,
                self.sheets.get(sheet_id),
                &self.schema_index,
                &data_root,
            )?;
            if let SheetDocument::Data(data) = &document {
                let definition = data
                    .manifest
                    .definition
                    .as_deref()
                    .context("data sheet has no definition")?;
                if !definitions.insert(definition.to_string()) {
                    bail!("snapshot binds table '{definition}' to more than one data sheet");
                }
                let source_path = PathBuf::from(&data.source_path);
                for (existing_sheet, existing_path) in &data_sources {
                    if data_sources_equivalent(existing_path, &source_path)? {
                        bail!(
                            "snapshot data sheets '{existing_sheet}' and '{sheet_id}' share JSON source '{}'",
                            source_path.display()
                        );
                    }
                }
                data_sources.push((sheet_id.clone(), source_path));
            }
            next_sheets.insert(sheet_id.clone(), document);
        }

        let removed = self
            .sheets
            .keys()
            .filter(|sheet_id| !snapshot.sheets.contains_key(*sheet_id))
            .cloned()
            .collect::<Vec<_>>();
        for sheet_id in removed {
            self.removed_sheet_ids.insert(sheet_id);
        }
        for sheet_id in snapshot.sheets.keys() {
            self.removed_sheet_ids.remove(sheet_id);
        }

        let mut manifest_order = self
            .manifest
            .sheets
            .iter()
            .filter(|sheet_id| snapshot.sheets.contains_key(*sheet_id))
            .cloned()
            .collect::<Vec<_>>();
        let added_sheet_ids = snapshot
            .sheets
            .keys()
            .filter(|sheet_id| !manifest_order.contains(sheet_id))
            .cloned()
            .collect::<Vec<_>>();
        manifest_order.extend(added_sheet_ids);
        self.manifest.sheets = manifest_order;
        self.sheets = next_sheets;
        self.refresh_diagnostics();
        Ok(())
    }

    fn open_data_sheet(
        &self,
        manifest: SheetManifest,
        directory: &Path,
        data_root: &Path,
        allow_missing: bool,
    ) -> Result<SheetDocument> {
        let definition = manifest
            .definition
            .as_deref()
            .context("data sheet requires a table definition")?;
        let table = self
            .schema_index
            .table(definition)
            .with_context(|| format!("unknown table definition '{definition}'"))?;
        let json_source = require_readable_json_source(table)?;
        let source_path = if Path::new(&json_source).is_absolute() {
            PathBuf::from(&json_source)
        } else {
            data_root.join(&json_source)
        };
        let rows = if source_path.exists() {
            let value: Value = serde_json::from_str(&fs::read_to_string(&source_path)?)
                .with_context(|| format!("invalid JSON source: {}", source_path.display()))?;
            value
                .as_array()
                .with_context(|| {
                    format!("JSON source must be an array: {}", source_path.display())
                })?
                .clone()
        } else if allow_missing {
            Vec::new()
        } else {
            bail!("JSON source not found: {}", source_path.display());
        };
        let mut formulas =
            read_json_or_default::<FormulaDocument>(&directory.join("formulas.json"))?;
        let mut format = read_json_or_default::<FormatDocument>(&directory.join("format.json"))?;
        if formulas.version == 0 {
            formulas.version = FORMAT_VERSION;
        }
        if format.version == 0 {
            format.version = FORMAT_VERSION;
        }
        let row_ids_path = directory.join("rowids.json");
        let prior_row_ids = if row_ids_path.exists() {
            Some(read_json(&row_ids_path)?)
        } else {
            None
        };
        let is_initial_identity = prior_row_ids.is_none();
        let (records, row_ids, identity_issues) =
            build_row_records(&rows, table, prior_row_ids, is_initial_identity);
        Ok(SheetDocument::Data(DataSheetDocument {
            manifest,
            source_path: source_path.to_string_lossy().to_string(),
            primary_key: primary_key(table).map(|field| field.name.clone()),
            rows: records,
            formulas,
            format,
            row_ids,
            identity_issues,
        }))
    }

    fn refresh_normalization_required(&mut self) -> Result<()> {
        let mut paths = Vec::new();
        for sheet in self.sheets.values() {
            let SheetDocument::Data(sheet) = sheet else {
                continue;
            };
            let source_path = Path::new(&sheet.source_path);
            if !source_path.exists() {
                continue;
            }
            let definition = sheet
                .manifest
                .definition
                .as_deref()
                .context("data sheet has no definition")?;
            let table = self
                .schema_index
                .table(definition)
                .context("bound table no longer exists")?;
            let canonical = canonical_rows_json(
                &sheet
                    .rows
                    .iter()
                    .map(|row| row.value.clone())
                    .collect::<Vec<_>>(),
                table,
                &self.schema_index,
            )?;
            if fs::read(source_path)? != canonical.as_bytes() {
                paths.push(source_path.to_path_buf());
            }
        }
        paths.sort();
        paths.dedup();
        self.normalization_required = paths;
        Ok(())
    }

    fn sheet_directory(&self, sheet_id: &str) -> PathBuf {
        self.root.join("sheets").join(sheet_id)
    }
}

fn sheet_document_from_snapshot(
    snapshot: &SnapshotSheet,
    existing: Option<&SheetDocument>,
    schema_index: &SchemaIndex,
    data_root: &Path,
) -> Result<SheetDocument> {
    if let Some(existing) = existing {
        let existing_kind = match existing {
            SheetDocument::Data(_) => SheetKind::Data,
            SheetDocument::Calculation(_) => SheetKind::Calculation,
        };
        if existing_kind != snapshot.kind {
            bail!(
                "snapshot changes sheet '{}' from {:?} to {:?}; delete and add it explicitly",
                snapshot.id,
                existing_kind,
                snapshot.kind
            );
        }
    }
    ensure_unique_snapshot_row_ids(snapshot)?;

    match snapshot.kind {
        SheetKind::Data => {
            let existing_data = match existing {
                Some(SheetDocument::Data(data)) => Some(data),
                Some(SheetDocument::Calculation(_)) => unreachable!(),
                None => None,
            };
            let definition = snapshot
                .definition
                .as_deref()
                .filter(|value| !value.is_empty())
                .or_else(|| {
                    existing_data
                        .and_then(|data| data.manifest.definition.as_deref())
                        .filter(|value| !value.is_empty())
                })
                .with_context(|| {
                    format!(
                        "cannot add data sheet '{}': snapshot is missing its definition",
                        snapshot.id
                    )
                })?;
            if existing_data.is_none() && snapshot.definition.as_deref().is_none_or(str::is_empty) {
                bail!(
                    "cannot add data sheet '{}': snapshot is missing its definition",
                    snapshot.id
                );
            }
            let table = schema_index.table(definition).with_context(|| {
                format!(
                    "cannot apply data sheet '{}': unknown table definition '{definition}'",
                    snapshot.id
                )
            })?;
            let expected_source_path = data_source_path(table, data_root)?;
            let source_path = snapshot
                .source_path
                .as_deref()
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
                .or_else(|| existing_data.map(|data| PathBuf::from(&data.source_path)))
                .with_context(|| {
                    format!(
                        "cannot add data sheet '{}': snapshot is missing its source path",
                        snapshot.id
                    )
                })?;
            if existing_data.is_none() && snapshot.source_path.as_deref().is_none_or(str::is_empty)
            {
                bail!(
                    "cannot add data sheet '{}': snapshot is missing its source path",
                    snapshot.id
                );
            }
            if !paths_equivalent(&source_path, &expected_source_path)? {
                bail!(
                    "cannot apply data sheet '{}': source path '{}' does not match schema source '{}'",
                    snapshot.id,
                    source_path.display(),
                    expected_source_path.display()
                );
            }

            let mut formulas = snapshot.formulas.clone();
            if formulas.version == 0 {
                formulas.version = FORMAT_VERSION;
            }
            let mut format: FormatDocument = serde_json::from_value(snapshot.format.clone())
                .with_context(|| {
                    format!("data sheet '{}' has invalid format metadata", snapshot.id)
                })?;
            if format.version == 0 {
                format.version = FORMAT_VERSION;
            }
            let values = snapshot
                .rows
                .iter()
                .map(|row| row.value.clone())
                .collect::<Vec<_>>();
            let primary = primary_key(table);
            if primary.is_some() {
                let (expected_rows, _, _) = build_row_records(&values, table, None, true);
                if expected_rows
                    .iter()
                    .map(|row| &row.id)
                    .ne(snapshot.rows.iter().map(|row| &row.id))
                {
                    bail!(
                        "data sheet '{}' contains row IDs that do not match its primary key",
                        snapshot.id
                    );
                }
            }
            let row_ids = primary.is_none().then(|| RowIdDocument {
                version: FORMAT_VERSION,
                source_hash: fingerprint(&Value::Array(values)),
                rows: snapshot
                    .rows
                    .iter()
                    .map(|row| RowIdEntry {
                        id: row.id.clone(),
                        fingerprint: fingerprint(&row.value),
                    })
                    .collect(),
            });
            Ok(SheetDocument::Data(DataSheetDocument {
                manifest: SheetManifest {
                    id: snapshot.id.clone(),
                    name: snapshot.name.clone(),
                    kind: SheetKind::Data,
                    definition: Some(definition.to_string()),
                    row_count: snapshot.rows.len(),
                    column_count: table_fields(table).len(),
                },
                source_path: expected_source_path.to_string_lossy().to_string(),
                primary_key: primary.map(|field| field.name.clone()),
                rows: snapshot.rows.clone(),
                formulas,
                format,
                row_ids,
                identity_issues: Vec::new(),
            }))
        }
        SheetKind::Calculation => {
            if snapshot.definition.is_some() || snapshot.source_path.is_some() {
                bail!(
                    "calculation sheet '{}' must not contain data source metadata",
                    snapshot.id
                );
            }
            let [row] = snapshot.rows.as_slice() else {
                bail!(
                    "calculation sheet '{}' snapshot must contain exactly one document row",
                    snapshot.id
                );
            };
            if row.id != "calculation" {
                bail!(
                    "calculation sheet '{}' snapshot has invalid document row id '{}'",
                    snapshot.id,
                    row.id
                );
            }
            let mut document: CalculationDocument = serde_json::from_value(row.value.clone())
                .with_context(|| {
                    format!(
                        "calculation sheet '{}' has an invalid calculation document",
                        snapshot.id
                    )
                })?;
            if document.version == 0 {
                document.version = FORMAT_VERSION;
            }
            let mut format: FormatDocument = serde_json::from_value(snapshot.format.clone())
                .with_context(|| {
                    format!(
                        "calculation sheet '{}' has invalid format metadata",
                        snapshot.id
                    )
                })?;
            if format.version == 0 {
                format.version = FORMAT_VERSION;
            }
            Ok(SheetDocument::Calculation(CalculationSheetDocument {
                manifest: SheetManifest {
                    id: snapshot.id.clone(),
                    name: snapshot.name.clone(),
                    kind: SheetKind::Calculation,
                    definition: None,
                    row_count: document.row_order.len(),
                    column_count: document.column_order.len(),
                },
                document,
                format,
            }))
        }
    }
}

fn ensure_unique_snapshot_row_ids(snapshot: &SnapshotSheet) -> Result<()> {
    let mut row_ids = BTreeSet::new();
    for row in &snapshot.rows {
        if row.id.is_empty() {
            bail!("sheet '{}' contains an empty row ID", snapshot.id);
        }
        if !row_ids.insert(&row.id) {
            bail!(
                "sheet '{}' contains duplicate row ID '{}'",
                snapshot.id,
                row.id
            );
        }
    }
    Ok(())
}

fn data_source_path(table: &StructDef, data_root: &Path) -> Result<PathBuf> {
    let json_source = require_readable_json_source(table)?;
    Ok(if Path::new(&json_source).is_absolute() {
        PathBuf::from(json_source)
    } else {
        data_root.join(json_source)
    })
}

fn paths_equivalent(left: &Path, right: &Path) -> Result<bool> {
    let left = normalized_path_key(left)?;
    let right = normalized_path_key(right)?;
    #[cfg(windows)]
    {
        Ok(left.eq_ignore_ascii_case(&right))
    }
    #[cfg(not(windows))]
    {
        Ok(left == right)
    }
}

fn data_sources_equivalent(left: &Path, right: &Path) -> Result<bool> {
    if paths_equivalent(left, right)? {
        return Ok(true);
    }
    if !left.exists() || !right.exists() {
        return Ok(false);
    }
    paths_equivalent(&dunce::canonicalize(left)?, &dunce::canonicalize(right)?)
}

fn normalized_path_key(path: &Path) -> Result<String> {
    let absolute = absolute_path(path)?;
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    Ok(normalized.to_string_lossy().replace('\\', "/"))
}

fn validate_sheet_id(sheet_id: &str) -> Result<()> {
    if sheet_id.contains('/') || sheet_id.contains('\\') {
        bail!("invalid sheet id '{sheet_id}'");
    }
    let mut components = Path::new(sheet_id).components();
    if !matches!(components.next(), Some(Component::Normal(_))) || components.next().is_some() {
        bail!("invalid sheet id '{sheet_id}'");
    }
    Ok(())
}

const SHEET_SIDECAR_FILE_NAMES: [&str; 5] = [
    "sheet.toml",
    "formulas.json",
    "format.json",
    "rowids.json",
    "cells.json",
];

fn validated_sheet_sidecar_directory(root: &Path, sheet_id: &str) -> Result<Option<PathBuf>> {
    validate_sheet_id(sheet_id)?;
    let sheets_root = root.join("sheets");
    let directory = sheets_root.join(sheet_id);
    let metadata = match fs::symlink_metadata(&directory) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        bail!(
            "refusing to use non-directory sheet sidecar path '{}'",
            directory.display()
        );
    }
    let canonical_root = sheets_root.canonicalize()?;
    let canonical_directory = directory.canonicalize()?;
    if !canonical_directory.starts_with(&canonical_root) {
        bail!(
            "refusing to remove sheet sidecar outside project: '{}'",
            directory.display()
        );
    }
    Ok(Some(directory))
}

fn sheet_sidecar_deletion_paths(root: &Path, sheet_id: &str) -> Result<Vec<PathBuf>> {
    let Some(directory) = validated_sheet_sidecar_directory(root, sheet_id)? else {
        return Ok(Vec::new());
    };
    Ok(SHEET_SIDECAR_FILE_NAMES
        .iter()
        .map(|file_name| directory.join(file_name))
        .collect())
}

fn remove_empty_sheet_sidecar_directory(root: &Path, sheet_id: &str) -> Result<()> {
    let Some(directory) = validated_sheet_sidecar_directory(root, sheet_id)? else {
        return Ok(());
    };
    if fs::read_dir(&directory)?.next().transpose()?.is_some() {
        return Ok(());
    }
    match fs::remove_dir(&directory) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            // Preserve a file that appeared after the emptiness check instead of
            // turning a completed save into a recursive cleanup attempt.
            if fs::read_dir(&directory)
                .ok()
                .and_then(|mut entries| entries.next())
                .is_some()
            {
                return Ok(());
            }
            return Err(error).with_context(|| {
                format!(
                    "failed to remove empty deleted sheet '{}': {}",
                    sheet_id,
                    directory.display()
                )
            });
        }
    }
    Ok(())
}

pub(crate) fn build_row_records(
    rows: &[Value],
    table: &StructDef,
    prior: Option<RowIdDocument>,
    initial: bool,
) -> (Vec<RowRecord>, Option<RowIdDocument>, Vec<String>) {
    if let Some(primary) = primary_key(table) {
        let records = rows
            .iter()
            .map(|row| {
                let value = row.get(&primary.name).unwrap_or(&Value::Null);
                RowRecord {
                    id: format!(
                        "pk:{}:{}",
                        primary.field_type.type_name,
                        canonical_scalar(value)
                    ),
                    value: row.clone(),
                }
            })
            .collect();
        return (records, None, Vec::new());
    }

    let mut issues = Vec::new();
    let current_hash = fingerprint(&Value::Array(rows.to_vec()));
    if let Some(prior) = &prior {
        if prior.source_hash == current_hash
            && prior.rows.len() == rows.len()
            && prior
                .rows
                .iter()
                .zip(rows)
                .all(|(entry, row)| entry.fingerprint == fingerprint(row))
        {
            let records = rows
                .iter()
                .zip(&prior.rows)
                .map(|(row, entry)| RowRecord {
                    id: entry.id.clone(),
                    value: row.clone(),
                })
                .collect();
            return (
                records,
                Some(RowIdDocument {
                    version: FORMAT_VERSION,
                    source_hash: current_hash,
                    rows: prior.rows.clone(),
                }),
                issues,
            );
        }
    }

    let prior_counts = prior
        .as_ref()
        .map(|document| fingerprint_counts(document.rows.iter().map(|entry| &entry.fingerprint)))
        .unwrap_or_default();
    let current_fingerprints = rows.iter().map(fingerprint).collect::<Vec<_>>();
    let current_counts = fingerprint_counts(current_fingerprints.iter());
    let mut unique_prior = prior
        .as_ref()
        .into_iter()
        .flat_map(|document| &document.rows)
        .filter(|entry| prior_counts.get(&entry.fingerprint) == Some(&1))
        .map(|entry| (entry.fingerprint.clone(), entry.id.clone()))
        .collect::<BTreeMap<_, _>>();
    let matched_prior = current_fingerprints
        .iter()
        .filter(|fingerprint| {
            current_counts.get(*fingerprint) == Some(&1) && unique_prior.contains_key(*fingerprint)
        })
        .cloned()
        .collect::<BTreeSet<_>>();
    let unmatched_prior_exists = prior.as_ref().is_some_and(|document| {
        document
            .rows
            .iter()
            .any(|entry| !matched_prior.contains(&entry.fingerprint))
    });
    let mut id_entries = Vec::with_capacity(rows.len());
    let mut records = Vec::with_capacity(rows.len());
    for (index, (row, row_fingerprint)) in rows
        .iter()
        .zip(current_fingerprints.into_iter())
        .enumerate()
    {
        let reused = current_counts.get(&row_fingerprint) == Some(&1)
            && prior_counts.get(&row_fingerprint) == Some(&1);
        let id = if reused {
            unique_prior
                .remove(&row_fingerprint)
                .unwrap_or_else(|| Uuid::now_v7().to_string())
        } else {
            Uuid::now_v7().to_string()
        };
        let duplicate_ambiguity = current_counts.get(&row_fingerprint).copied().unwrap_or(0) > 1
            || prior_counts.get(&row_fingerprint).copied().unwrap_or(0) > 1;
        if !initial && prior.is_some() && !reused && (duplicate_ambiguity || unmatched_prior_exists)
        {
            issues.push(format!(
                "row {index} (row id {id}) is modified or fingerprint-ambiguous and needs row identity confirmation"
            ));
        }
        id_entries.push(RowIdEntry {
            id: id.clone(),
            fingerprint: row_fingerprint,
        });
        records.push(RowRecord {
            id,
            value: row.clone(),
        });
    }
    (
        records,
        Some(RowIdDocument {
            version: FORMAT_VERSION,
            source_hash: current_hash,
            rows: id_entries,
        }),
        issues,
    )
}

fn fingerprint_counts<'a>(
    fingerprints: impl Iterator<Item = &'a String>,
) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for fingerprint in fingerprints {
        *counts.entry(fingerprint.clone()).or_insert(0) += 1;
    }
    counts
}

pub fn canonical_rows_json(
    rows: &[Value],
    table: &StructDef,
    index: &SchemaIndex,
) -> Result<String> {
    let canonical = Value::Array(
        rows.iter()
            .map(|row| canonicalize_struct(row, table, index))
            .collect::<Result<Vec<_>>>()?,
    );
    let mut output = serde_json::to_string_pretty(&canonical)?;
    output.push('\n');
    Ok(output)
}

fn canonicalize_struct(value: &Value, table: &StructDef, index: &SchemaIndex) -> Result<Value> {
    let object = value.as_object().context("expected object")?;
    let mut canonical = Map::new();
    for field in table_fields(table) {
        if let Some(field_value) = object.get(&field.name) {
            canonical.insert(
                field.name.clone(),
                canonicalize_type(field_value, &field.field_type, index)?,
            );
        }
    }
    for (key, field_value) in object {
        if !canonical.contains_key(key) {
            canonical.insert(key.clone(), field_value.clone());
        }
    }
    Ok(Value::Object(canonical))
}

fn canonicalize_type(value: &Value, type_ref: &TypeRef, index: &SchemaIndex) -> Result<Value> {
    if value.is_null() {
        return Ok(Value::Null);
    }
    if type_ref.is_option {
        return type_ref.inner_type.as_deref().map_or_else(
            || Ok(value.clone()),
            |inner| canonicalize_type(value, inner, index),
        );
    }
    if type_ref.is_list {
        if let (Some(items), Some(inner)) = (value.as_array(), type_ref.inner_type.as_deref()) {
            return Ok(Value::Array(
                items
                    .iter()
                    .map(|item| canonicalize_type(item, inner, index))
                    .collect::<Result<Vec<_>>>()?,
            ));
        }
    }
    if type_ref.is_struct {
        if let Some(definition) = index.structs.get(&type_ref.fqn) {
            return canonicalize_struct(value, definition, index);
        }
    }
    Ok(value.clone())
}

fn ensure_valid_rows(table: &StructDef, rows: &[Value], index: &SchemaIndex) -> Result<()> {
    let errors = validate_table_rows(table, rows, index)
        .into_iter()
        .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
        .collect::<Vec<_>>();
    if errors.is_empty() {
        return Ok(());
    }
    let message = errors
        .iter()
        .take(8)
        .map(|diagnostic| format!("{}: {}", diagnostic.path, diagnostic.message))
        .collect::<Vec<_>>()
        .join("\n");
    bail!("row validation failed:\n{message}")
}

fn parse_default_value(default: &str, type_ref: &TypeRef) -> Value {
    let target = unwrap_cardinality(type_ref);
    match target.type_name.as_str() {
        "string" | "timestamp" | "bytes" => {
            serde_json::from_str(default).unwrap_or_else(|_| Value::String(default.to_string()))
        }
        "bool" => default
            .parse::<bool>()
            .map(Value::Bool)
            .unwrap_or_else(|_| Value::String(default.to_string())),
        "i8" | "i16" | "i32" | "i64" => default
            .parse::<i64>()
            .map(|number| Value::Number(number.into()))
            .unwrap_or_else(|_| Value::String(default.to_string())),
        "u8" | "u16" | "u32" | "u64" => default
            .parse::<u64>()
            .map(|number| Value::Number(number.into()))
            .unwrap_or_else(|_| Value::String(default.to_string())),
        "f32" | "f64" => default
            .parse::<f64>()
            .ok()
            .and_then(serde_json::Number::from_f64)
            .map(Value::Number)
            .unwrap_or_else(|| Value::String(default.to_string())),
        _ => Value::String(default.to_string()),
    }
}

fn inspector_fields(table: &StructDef, index: &SchemaIndex) -> Vec<InspectorField> {
    let mut fields = Vec::new();
    collect_inspector_fields("", table, index, &mut fields);
    fields
}

fn collect_inspector_fields(
    prefix: &str,
    definition: &StructDef,
    index: &SchemaIndex,
    target: &mut Vec<InspectorField>,
) {
    for field in table_fields(definition) {
        let path = if prefix.is_empty() {
            field.name.clone()
        } else {
            format!("{prefix}.{}", field.name)
        };
        let unwrapped = unwrap_cardinality(&field.field_type);
        let enum_values: Vec<String> = index
            .enums
            .get(&unwrapped.fqn)
            .map(|definition| {
                definition
                    .items
                    .iter()
                    .filter_map(|item| match item {
                        polygen::ir_model::EnumItem::Member(member) => Some(member.name.clone()),
                        _ => None,
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let input_example = example_value(field, &enum_values);
        target.push(InspectorField {
            path: path.clone(),
            name: field.name.clone(),
            type_name: field.field_type.original.clone(),
            required: !field.field_type.is_option,
            is_primary_key: field.is_primary_key,
            is_optional: field.field_type.is_option,
            is_list: field.field_type.is_list,
            is_enum: unwrapped.is_enum,
            is_struct: unwrapped.is_struct,
            enum_values,
            foreign_key_target: field
                .foreign_key
                .as_ref()
                .map(|foreign_key| foreign_key.target_table_fqn.clone()),
            foreign_key_field: field
                .foreign_key
                .as_ref()
                .map(|foreign_key| foreign_key.target_field.clone()),
            default_value: field.default_value.clone(),
            max_length: field.max_length,
            range_min: field.range.as_ref().map(|range| range.min.clone()),
            range_max: field.range.as_ref().map(|range| range.max.clone()),
            regex_pattern: field.regex_pattern.clone(),
            input_example,
        });
        if unwrapped.is_struct && !field.field_type.is_list {
            if let Some(child) = index.structs.get(&unwrapped.fqn) {
                collect_inspector_fields(&path, child, index, target);
            }
        }
    }
}

fn example_value(field: &FieldDef, enum_values: &[String]) -> Value {
    if let Some(default) = &field.default_value {
        return parse_default_value(default, &field.field_type);
    }
    if field.field_type.is_option {
        return Value::Null;
    }
    if field.field_type.is_list {
        return Value::Array(Vec::new());
    }
    if let Some(first) = enum_values.first() {
        return Value::String(first.clone());
    }
    match unwrap_cardinality(&field.field_type).type_name.as_str() {
        "string" => Value::String("text".to_string()),
        "timestamp" => Value::String("2026-01-01T00:00:00Z".to_string()),
        "bytes" => Value::String("AQID".to_string()),
        "bool" => Value::Bool(false),
        "f32" | "f64" => serde_json::json!(0.0),
        "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" => {
            serde_json::json!(0)
        }
        _ => Value::Object(Map::new()),
    }
}

fn value_at_path<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.')
        .try_fold(value, |current, segment| current.get(segment))
}

fn collect_comparison_fields(
    prefix: &str,
    definition: &StructDef,
    index: &SchemaIndex,
    rows: &[&RowRecord],
    formulas: &FormulaDocument,
    target: &mut Vec<RowComparisonField>,
) {
    for field in table_fields(definition) {
        let path = if prefix.is_empty() {
            field.name.clone()
        } else {
            format!("{prefix}.{}", field.name)
        };
        let unwrapped = unwrap_cardinality(&field.field_type);
        if unwrapped.is_struct && !field.field_type.is_list {
            if let Some(child) = index.structs.get(&unwrapped.fqn) {
                collect_comparison_fields(&path, child, index, rows, formulas, target);
                continue;
            }
        }
        let values = rows
            .iter()
            .map(|row| {
                let value = value_at_path(&row.value, &path);
                RowComparisonValue {
                    row_id: row.id.clone(),
                    present: value.is_some(),
                    value: value.cloned().unwrap_or(Value::Null),
                    formula: formulas
                        .rows
                        .get(&row.id)
                        .and_then(|fields| fields.get(&path))
                        .cloned(),
                }
            })
            .collect::<Vec<_>>();
        let all_equal = values.windows(2).all(|pair| {
            pair[0].present == pair[1].present
                && pair[0].value == pair[1].value
                && pair[0].formula == pair[1].formula
        });
        target.push(RowComparisonField {
            path,
            type_name: field.field_type.original.clone(),
            is_list: field.field_type.is_list,
            all_equal,
            values,
        });
    }
}

fn require_editable_json_source(table: &StructDef) -> Result<String> {
    if table.is_readonly {
        bail!("table '{}' is @readonly", table.fqn);
    }
    require_readable_json_source(table)
}

fn require_readable_json_source(table: &StructDef) -> Result<String> {
    let json = table
        .load
        .as_ref()
        .and_then(|load| load.json.clone())
        .with_context(|| format!("table '{}' has no JSON source", table.fqn))?;
    if json.contains('*') || json.contains('?') {
        bail!("wildcard JSON sources are read-only in PolySheet v1");
    }
    Ok(json)
}

fn resolve_data_root(
    root: &Path,
    manifest: &WorkbookManifest,
    loaded: Option<&LoadedProjectSchema>,
) -> PathBuf {
    manifest
        .data_root
        .as_deref()
        .map(|path| resolve_path(root, path))
        .or_else(|| {
            loaded
                .and_then(|loaded| loaded.sources_path.as_deref())
                .and_then(Path::parent)
                .map(Path::to_path_buf)
        })
        .or_else(|| loaded.and_then(|loaded| loaded.schema_path.parent().map(Path::to_path_buf)))
        .unwrap_or_else(|| root.to_path_buf())
}

fn validate_schema_path(path: &Path) -> Result<PathBuf> {
    if path.as_os_str().is_empty() {
        bail!(".poly 스키마 파일을 선택해 주세요");
    }
    if path.is_dir() {
        bail!(
            "'.poly 스키마 경로'에는 폴더가 아니라 .poly 파일을 선택해 주세요: {}",
            path.display()
        );
    }
    if path.extension().and_then(|extension| extension.to_str()) != Some("poly") {
        bail!(
            ".poly 확장자의 스키마 파일을 선택해 주세요: {}",
            path.display()
        );
    }
    absolute_path(path).with_context(|| format!("스키마 파일을 열 수 없습니다: {}", path.display()))
}

fn validate_sources_path(path: &Path) -> Result<PathBuf> {
    if path.is_dir() {
        bail!(
            "'.sources.toml 경로'에는 폴더가 아니라 TOML 파일을 선택해 주세요: {}",
            path.display()
        );
    }
    absolute_path(path)
        .with_context(|| format!("sources 설정 파일을 열 수 없습니다: {}", path.display()))
}

fn set_value_at_path(target: &mut Value, path: &str, value: Value) -> Result<()> {
    let mut current = target;
    let segments = path.split('.').collect::<Vec<_>>();
    for segment in &segments[..segments.len().saturating_sub(1)] {
        current = current
            .as_object_mut()
            .context("field path traverses a non-object value")?
            .get_mut(*segment)
            .with_context(|| format!("field path segment '{segment}' does not exist"))?;
    }
    let final_segment = segments.last().context("field path must not be empty")?;
    current
        .as_object_mut()
        .context("field target is not an object")?
        .insert((*final_segment).to_string(), value);
    Ok(())
}

fn field_at_path<'a>(
    table: &'a StructDef,
    path: &str,
    index: &'a SchemaIndex,
) -> Option<&'a FieldDef> {
    let mut definition = table;
    let segments = path.split('.').collect::<Vec<_>>();
    for (position, segment) in segments.iter().enumerate() {
        let field = table_fields(definition)
            .into_iter()
            .find(|field| field.name == *segment)?;
        if position == segments.len() - 1 {
            return Some(field);
        }
        let type_ref = unwrap_cardinality(&field.field_type);
        definition = index.structs.get(&type_ref.fqn)?;
    }
    None
}

fn unwrap_cardinality(mut type_ref: &TypeRef) -> &TypeRef {
    while let Some(inner) = type_ref.inner_type.as_deref() {
        type_ref = inner;
    }
    type_ref
}

fn coerce_frontend_value(value: Value, type_ref: &TypeRef) -> Result<Value> {
    if value.is_null() {
        return Ok(value);
    }
    if type_ref.is_option {
        return match type_ref.inner_type.as_deref() {
            Some(inner) => coerce_frontend_value(value, inner),
            None => Ok(value),
        };
    }
    if type_ref.is_list || type_ref.is_struct {
        return Ok(value);
    }
    let primitive = type_ref.type_name.as_str();
    if matches!(primitive, "i64" | "u64") {
        if let Some(text) = value.as_str() {
            let number = if primitive == "u64" {
                serde_json::Number::from(text.parse::<u64>()?)
            } else {
                serde_json::Number::from(text.parse::<i64>()?)
            };
            return Ok(Value::Number(number));
        }
    }
    Ok(value)
}

fn coerce_struct_value(value: Value, definition: &StructDef, index: &SchemaIndex) -> Result<Value> {
    let mut object = value.as_object().context("expected JSON object")?.clone();
    for field in table_fields(definition) {
        let Some(field_value) = object.remove(&field.name) else {
            continue;
        };
        object.insert(
            field.name.clone(),
            coerce_typed_value(field_value, &field.field_type, index)?,
        );
    }
    Ok(Value::Object(object))
}

fn coerce_typed_value(value: Value, type_ref: &TypeRef, index: &SchemaIndex) -> Result<Value> {
    if value.is_null() {
        return Ok(value);
    }
    if type_ref.is_option {
        return match type_ref.inner_type.as_deref() {
            Some(inner) => coerce_typed_value(value, inner, index),
            None => Ok(value),
        };
    }
    if type_ref.is_list {
        if let (Some(values), Some(inner)) = (value.as_array(), type_ref.inner_type.as_deref()) {
            return values
                .iter()
                .cloned()
                .map(|item| coerce_typed_value(item, inner, index))
                .collect::<Result<Vec<_>>>()
                .map(Value::Array);
        }
    }
    if type_ref.is_struct {
        if let Some(definition) = index.structs.get(&type_ref.fqn) {
            return coerce_struct_value(value, definition, index);
        }
    }
    coerce_frontend_value(value, type_ref)
}

fn is_unsafe_formula_integer(value: &Value, type_ref: &TypeRef) -> bool {
    let type_ref = unwrap_cardinality(type_ref);
    if !matches!(type_ref.type_name.as_str(), "i64" | "u64") {
        return false;
    }
    const MAX_SAFE_INTEGER: i128 = 9_007_199_254_740_991;
    let integer = value
        .as_i64()
        .map(i128::from)
        .or_else(|| value.as_u64().map(i128::from))
        .or_else(|| value.as_str()?.parse::<i128>().ok());
    integer.is_some_and(|integer| integer.unsigned_abs() > MAX_SAFE_INTEGER as u128)
}

pub fn fingerprint(value: &Value) -> String {
    hash_bytes(serde_json::to_vec(value).unwrap_or_default().as_slice())
}

pub fn hash_bytes(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn canonical_toml<T: Serialize>(value: &T) -> Result<String> {
    let mut output = toml::to_string_pretty(value)?;
    if !output.ends_with('\n') {
        output.push('\n');
    }
    Ok(output.replace("\r\n", "\n"))
}

fn canonical_json<T: Serialize>(value: &T) -> Result<Vec<u8>> {
    let mut output = serde_json::to_string_pretty(value)?;
    output.push('\n');
    Ok(output.into_bytes())
}

fn toml_update<T: Serialize>(path: PathBuf, value: &T) -> Result<FileUpdate> {
    Ok(FileUpdate {
        path,
        contents: canonical_toml(value)?.into_bytes(),
    })
}

fn json_update<T: Serialize>(path: PathBuf, value: &T) -> Result<FileUpdate> {
    Ok(FileUpdate {
        path,
        contents: canonical_json(value)?,
    })
}

fn read_toml<T: DeserializeOwned>(path: &Path) -> Result<T> {
    toml::from_str(&fs::read_to_string(path)?)
        .with_context(|| format!("failed to parse {}", path.display()))
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    serde_json::from_str(&fs::read_to_string(path)?)
        .with_context(|| format!("failed to parse {}", path.display()))
}

fn read_json_or_default<T: DeserializeOwned + Default>(path: &Path) -> Result<T> {
    if path.exists() {
        read_json(path)
    } else {
        Ok(T::default())
    }
}

fn absolute_path(path: &Path) -> Result<PathBuf> {
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

fn resolve_path(base: &Path, path: &str) -> PathBuf {
    let path = Path::new(path);
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

fn relative_or_absolute(base: &Path, path: &Path) -> String {
    pathdiff::diff_paths(path, base)
        .unwrap_or_else(|| path.to_path_buf())
        .to_string_lossy()
        .replace('\\', "/")
}

fn manifest_path(base: &Path, path: &Path) -> String {
    if base.as_os_str().is_empty() {
        path.to_string_lossy().replace('\\', "/")
    } else {
        relative_or_absolute(base, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn fixture() -> Result<(tempfile::TempDir, PolySheetProject)> {
        let temp = tempdir()?;
        let schema = temp.path().join("game.poly");
        let sources = temp.path().join("game.sources.toml");
        fs::create_dir_all(temp.path().join("data"))?;
        fs::write(
            &schema,
            "namespace game { table Item { id: u32 primary_key; name: string; count: u64; } }",
        )?;
        fs::write(
            &sources,
            r#"[tables."game.Item".load]
json = "data/items.json"
"#,
        )?;
        fs::write(
            temp.path().join("data/items.json"),
            r#"[{"name":"A","id":1,"count":18446744073709551615}]"#,
        )?;
        let root = temp.path().join("game.polysheet");
        let mut project = PolySheetProject::create(&root, "Game", &schema, Some(&sources), None)?;
        project.bind_data_sheet("Items", "game.Item")?;
        Ok((temp, project))
    }

    fn persist_data_sheet_copy(
        project: &mut PolySheetProject,
        source_id: &str,
        copied_id: &str,
        definition: &str,
    ) -> Result<()> {
        let SheetDocument::Data(mut copied) = project
            .sheets
            .get(source_id)
            .context("source data sheet is missing")?
            .clone()
        else {
            bail!("source sheet is not a data sheet");
        };
        copied.manifest.id = copied_id.to_string();
        copied.manifest.name = format!("Copy of {source_id}");
        copied.manifest.definition = Some(definition.to_string());
        let directory = project.root.join("sheets").join(copied_id);
        fs::create_dir_all(&directory)?;
        fs::write(
            directory.join("sheet.toml"),
            canonical_toml(&copied.manifest)?,
        )?;
        fs::write(
            directory.join("formulas.json"),
            canonical_json(&copied.formulas)?,
        )?;
        fs::write(
            directory.join("format.json"),
            canonical_json(&copied.format)?,
        )?;
        if let Some(row_ids) = &copied.row_ids {
            fs::write(directory.join("rowids.json"), canonical_json(row_ids)?)?;
        }
        project.manifest.sheets.push(copied_id.to_string());
        fs::write(
            project.root.join("workbook.toml"),
            canonical_toml(&project.manifest)?,
        )?;
        Ok(())
    }

    #[test]
    fn canonical_save_is_byte_stable_and_schema_ordered() -> Result<()> {
        let (temp, mut project) = fixture()?;
        assert_eq!(project.normalization_required.len(), 1);
        assert!(project.save(false).is_err());
        assert_eq!(project.normalization_preview()?.len(), 1);
        project.save(true)?;
        let data_path = temp.path().join("data/items.json");
        let first = fs::read(&data_path)?;
        assert!(
            String::from_utf8_lossy(&first).find("\"id\"").unwrap()
                < String::from_utf8_lossy(&first).find("\"name\"").unwrap()
        );
        project.save(true)?;
        assert_eq!(first, fs::read(data_path)?);
        Ok(())
    }

    #[test]
    fn primary_key_identity_survives_field_edit() -> Result<()> {
        let (_temp, mut project) = fixture()?;
        let sheet_id = project.manifest.sheets[0].clone();
        let row_id = match project.sheets.get(&sheet_id).unwrap() {
            SheetDocument::Data(sheet) => sheet.rows[0].id.clone(),
            _ => unreachable!(),
        };
        project.patch_cell(&sheet_id, &row_id, "name", Value::String("B".into()))?;
        let row_after = match project.sheets.get(&sheet_id).unwrap() {
            SheetDocument::Data(sheet) => &sheet.rows[0],
            _ => unreachable!(),
        };
        assert_eq!(row_after.id, row_id);
        assert_eq!(row_after.value["name"], "B");
        Ok(())
    }

    #[test]
    fn row_inspector_reads_edits_inserts_and_compares_by_stable_id() -> Result<()> {
        let (_temp, mut project) = fixture()?;
        let sheet_id = project.manifest.sheets[0].clone();
        let first_id = match project.sheets.get(&sheet_id).unwrap() {
            SheetDocument::Data(sheet) => sheet.rows[0].id.clone(),
            _ => unreachable!(),
        };

        let draft = project.get_row_draft(&sheet_id)?;
        assert!(draft.editable);
        assert_eq!(draft.missing_required, vec!["id", "name", "count"]);
        assert!(draft.fields.iter().any(|field| field.path == "name"));

        let second_id = project.insert_row_json(
            &sheet_id,
            r#"{"id":2,"name":"Second","count":"9007199254740992"}"#,
        )?;
        assert_eq!(
            project.get_row_json(&sheet_id, &second_id)?,
            "{\n  \"id\": 2,\n  \"name\": \"Second\",\n  \"count\": 9007199254740992\n}\n"
        );

        project.apply_row_json(
            &sheet_id,
            &second_id,
            r#"{"id":2,"name":"Changed","count":"9007199254740992"}"#,
        )?;
        let comparison = project.compare_rows(&sheet_id, &[first_id.clone(), second_id.clone()])?;
        assert_eq!(comparison.row_ids, vec![first_id, second_id]);
        let name = comparison
            .fields
            .iter()
            .find(|field| field.path == "name")
            .unwrap();
        assert!(!name.all_equal);
        assert_eq!(name.values[1].value, "Changed");
        Ok(())
    }

    #[test]
    fn row_inspector_rejects_invalid_rows_primary_key_changes_and_large_comparisons() -> Result<()>
    {
        let (_temp, mut project) = fixture()?;
        let sheet_id = project.manifest.sheets[0].clone();
        let row_id = match project.sheets.get(&sheet_id).unwrap() {
            SheetDocument::Data(sheet) => sheet.rows[0].id.clone(),
            _ => unreachable!(),
        };
        assert!(project
            .apply_row_json(&sheet_id, &row_id, r#"{"id":2,"name":"A","count":1}"#)
            .unwrap_err()
            .to_string()
            .contains("primary key"));
        assert!(project
            .insert_row_json(&sheet_id, r#"{"id":2,"name":"missing count"}"#)
            .unwrap_err()
            .to_string()
            .contains("missing required field"));
        assert!(project
            .compare_rows(&sheet_id, &vec![row_id; 101])
            .unwrap_err()
            .to_string()
            .contains("at most 100"));
        Ok(())
    }

    #[test]
    fn keyless_rows_reconnect_unique_fingerprints_and_block_ambiguous_edits() -> Result<()> {
        let temp = tempdir()?;
        let schema_path = temp.path().join("keyless.poly");
        fs::write(
            &schema_path,
            "namespace game { table Note { text: string; rank: u32; } }",
        )?;
        let loaded = polygen::load_project_schema(&schema_path, None)?;
        let index = SchemaIndex::from_schema(&loaded.schema);
        let table = index.table("game.Note").unwrap();
        let original = vec![
            serde_json::json!({"text":"A","rank":1}),
            serde_json::json!({"text":"B","rank":2}),
        ];
        let (initial_rows, sidecar, issues) = build_row_records(&original, table, None, true);
        assert!(issues.is_empty());

        let reordered = vec![original[1].clone(), original[0].clone()];
        let (reconnected, _, issues) = build_row_records(&reordered, table, sidecar.clone(), false);
        assert!(issues.is_empty());
        assert_eq!(reconnected[0].id, initial_rows[1].id);
        assert_eq!(reconnected[1].id, initial_rows[0].id);

        let modified = vec![
            serde_json::json!({"text":"A changed","rank":1}),
            original[1].clone(),
        ];
        let (_, _, issues) = build_row_records(&modified, table, sidecar.clone(), false);
        assert!(!issues.is_empty());

        let duplicated = vec![original[0].clone(), original[0].clone()];
        let (_, _, issues) = build_row_records(&duplicated, table, sidecar, false);
        assert!(!issues.is_empty());
        Ok(())
    }

    #[test]
    fn keyless_external_edit_can_be_relinked_to_previous_uuid() -> Result<()> {
        let temp = tempdir()?;
        let schema = temp.path().join("notes.poly");
        let sources = temp.path().join("notes.sources.toml");
        fs::create_dir_all(temp.path().join("data"))?;
        fs::write(
            &schema,
            "namespace game { table Note { text: string; rank: u32; } }",
        )?;
        fs::write(
            &sources,
            r#"[tables."game.Note".load]
json = "data/notes.json"
"#,
        )?;
        let data_path = temp.path().join("data/notes.json");
        fs::write(&data_path, r#"[{"text":"A","rank":1}]"#)?;
        let root = temp.path().join("notes.polysheet");
        let mut project = PolySheetProject::create(&root, "Notes", &schema, Some(&sources), None)?;
        let sheet_id = project.bind_data_sheet("Notes", "game.Note")?;
        let previous_id = match project.sheets.get(&sheet_id).unwrap() {
            SheetDocument::Data(sheet) => sheet.rows[0].id.clone(),
            _ => unreachable!(),
        };
        project.save(true)?;

        fs::write(&data_path, r#"[{"text":"A edited","rank":1}]"#)?;
        let mut reopened = PolySheetProject::open(&root, OpenProjectOptions::default())?;
        assert!(reopened.has_errors());
        let current_id = match reopened.sheets.get(&sheet_id).unwrap() {
            SheetDocument::Data(sheet) => sheet.rows[0].id.clone(),
            _ => unreachable!(),
        };
        reopened.confirm_row_identity(&sheet_id, &current_id, &previous_id)?;
        assert!(!reopened.has_errors());
        let relinked = match reopened.sheets.get(&sheet_id).unwrap() {
            SheetDocument::Data(sheet) => &sheet.rows[0].id,
            _ => unreachable!(),
        };
        assert_eq!(relinked, &previous_id);
        Ok(())
    }

    #[test]
    fn unsafe_64_bit_formula_is_rejected() -> Result<()> {
        let (_temp, mut project) = fixture()?;
        let sheet_id = project.manifest.sheets[0].clone();
        let row_id = match project.sheets.get(&sheet_id).unwrap() {
            SheetDocument::Data(sheet) => sheet.rows[0].id.clone(),
            _ => unreachable!(),
        };
        let error = project
            .set_formula(
                &sheet_id,
                &row_id,
                "count",
                Some("=1+1".into()),
                Value::String("18446744073709551615".into()),
            )
            .unwrap_err();
        assert!(error.to_string().contains("#NUM!"));
        Ok(())
    }

    #[test]
    fn formulas_materialize_and_calculation_cells_roundtrip() -> Result<()> {
        let (_temp, mut project) = fixture()?;
        let data_id = project.manifest.sheets[0].clone();
        let row_id = match project.sheets.get(&data_id).unwrap() {
            SheetDocument::Data(sheet) => sheet.rows[0].id.clone(),
            _ => unreachable!(),
        };
        project.set_formula(
            &data_id,
            &row_id,
            "count",
            Some("=1+1".into()),
            Value::String("2".into()),
        )?;

        let calculation_id = project.add_calculation_sheet("Scratch");
        let mut calculation = project.calculation_document(&calculation_id)?;
        let key = format!(
            "{}:{}",
            calculation.row_order[0], calculation.column_order[0]
        );
        calculation.cells.insert(
            key.clone(),
            crate::model::CalculationCell {
                value: Some(Value::Number(serde_json::Number::from(2))),
                formula: Some("=1+1".into()),
            },
        );
        project.replace_calculation_document(&calculation_id, calculation)?;
        project.save(true)?;

        let reopened = PolySheetProject::open(&project.root, OpenProjectOptions::default())?;
        assert_eq!(
            reopened.formula_document(&data_id)?.rows[&row_id]["count"],
            "=1+1"
        );
        assert_eq!(
            reopened.calculation_document(&calculation_id)?.cells[&key]
                .formula
                .as_deref(),
            Some("=1+1")
        );
        assert!(reopened.data_sheet_json(&data_id)?.contains("\"count\": 2"));
        Ok(())
    }

    #[test]
    fn unsaved_calculation_workbook_can_be_saved_and_reopened_without_schema() -> Result<()> {
        let temp = tempdir()?;
        let root = temp.path().join("draft.polysheet");
        let mut project = PolySheetProject::create_unsaved("Draft");
        assert!(!project.is_saved());
        assert!(project.loaded_schema.is_none());
        assert_eq!(project.manifest.sheets.len(), 1);

        project.save_as(&root, false)?;
        assert!(project.is_saved());
        assert!(root.join("workbook.toml").is_file());

        let reopened = PolySheetProject::open(&root, OpenProjectOptions::default())?;
        assert!(reopened.loaded_schema.is_none());
        assert_eq!(reopened.manifest.name, "Draft");
        assert_eq!(reopened.manifest.sheets.len(), 1);
        Ok(())
    }

    #[test]
    fn calculation_formula_targets_survive_row_and_column_insertion() -> Result<()> {
        let mut project = PolySheetProject::create_unsaved("Stable formulas");
        let sheet_id = project.manifest.sheets[0].clone();
        let mut document = project.calculation_document(&sheet_id)?;
        let target_row = document.row_order[0].clone();
        let formula_row = document.row_order[1].clone();
        let target_column = document.column_order[0].clone();
        let formula_column = document.column_order[1].clone();
        let formula_cell = format!("{formula_row}:{formula_column}");
        document.cells.insert(
            formula_cell.clone(),
            crate::model::CalculationCell {
                value: Some(Value::Number(serde_json::Number::from(1))),
                formula: Some("=$A$1".into()),
            },
        );
        project.replace_calculation_document(&sheet_id, document)?;

        let stored = match project.sheets.get(&sheet_id).unwrap() {
            SheetDocument::Calculation(sheet) => {
                sheet.document.formula_bindings[&formula_cell].clone()
            }
            _ => unreachable!(),
        };
        assert_eq!(stored.references[0].row_id, target_row);
        assert_eq!(stored.references[0].column_id, target_column);

        project.insert_calculation_row(&sheet_id, 0)?;
        project.insert_calculation_column(&sheet_id, 0)?;
        let rendered = project.calculation_document(&sheet_id)?;
        assert_eq!(
            rendered.cells[&formula_cell].formula.as_deref(),
            Some("=$B$2")
        );
        Ok(())
    }

    #[test]
    fn data_formula_targets_follow_schema_field_identity_not_column_position() -> Result<()> {
        let (_temp, mut project) = fixture()?;
        let sheet_id = project.manifest.sheets[0].clone();
        let row_id = match project.sheets.get(&sheet_id).unwrap() {
            SheetDocument::Data(sheet) => sheet.rows[0].id.clone(),
            _ => unreachable!(),
        };
        project.set_formula(
            &sheet_id,
            &row_id,
            "count",
            Some("=C2".into()),
            Value::String("2".into()),
        )?;

        let table = project.schema_index.tables.get_mut("game.Item").unwrap();
        let name_index = table
            .items
            .iter()
            .position(|item| {
                matches!(
                    item,
                    polygen::ir_model::StructItem::Field(field) if field.name == "name"
                )
            })
            .unwrap();
        let count_index = table
            .items
            .iter()
            .position(|item| {
                matches!(
                    item,
                    polygen::ir_model::StructItem::Field(field) if field.name == "count"
                )
            })
            .unwrap();
        table.items.swap(name_index, count_index);

        assert_eq!(
            project.formula_document(&sheet_id)?.rows[&row_id]["count"],
            "=D2"
        );
        Ok(())
    }

    #[test]
    fn one_sided_structural_insert_merges_without_formula_conflict() -> Result<()> {
        let mut project = PolySheetProject::create_unsaved("Merge");
        let sheet_id = project.manifest.sheets[0].clone();
        let mut document = project.calculation_document(&sheet_id)?;
        let formula_cell = format!("{}:{}", document.row_order[1], document.column_order[1]);
        document.cells.insert(
            formula_cell,
            crate::model::CalculationCell {
                value: Some(Value::Number(serde_json::Number::from(1))),
                formula: Some("=A1".into()),
            },
        );
        project.replace_calculation_document(&sheet_id, document)?;
        let base = ProjectSnapshot::from_project(&project);
        project.insert_calculation_column(&sheet_id, 0)?;
        let ours = ProjectSnapshot::from_project(&project);

        let report = crate::merge::merge_snapshots(&base, &ours, &base);
        assert!(report.conflicts.is_empty());
        assert!(!report.recalculation_required);
        assert_eq!(report.merged, ours);
        Ok(())
    }

    #[test]
    fn apply_snapshot_adds_calculation_sheet_and_persists_its_files() -> Result<()> {
        let temp = tempdir()?;
        let root = temp.path().join("calculation.polysheet");
        let mut project = PolySheetProject::create_unsaved("Calculation merge");
        project.save_as(&root, true)?;
        let mut snapshot = ProjectSnapshot::from_project(&project);
        let added_id = "018f0000-0000-7000-8000-000000000099";
        let calculation = CalculationDocument {
            version: FORMAT_VERSION,
            row_order: vec!["row".into()],
            column_order: vec!["column".into()],
            ..CalculationDocument::default()
        };
        snapshot.sheets.insert(
            added_id.into(),
            SnapshotSheet {
                id: added_id.into(),
                name: "Added calculation".into(),
                kind: SheetKind::Calculation,
                definition: None,
                source_path: None,
                rows: vec![RowRecord {
                    id: "calculation".into(),
                    value: serde_json::to_value(calculation)?,
                }],
                formulas: FormulaDocument::default(),
                format: serde_json::to_value(FormatDocument {
                    version: FORMAT_VERSION,
                    ..FormatDocument::default()
                })?,
            },
        );

        project.apply_snapshot(&snapshot)?;
        project.save(true)?;

        assert!(project.manifest.sheets.contains(&added_id.to_string()));
        assert!(root
            .join("sheets")
            .join(added_id)
            .join("sheet.toml")
            .is_file());
        assert!(root
            .join("sheets")
            .join(added_id)
            .join("cells.json")
            .is_file());
        let reopened = PolySheetProject::open(&root, OpenProjectOptions::default())?;
        assert!(matches!(
            reopened.sheets.get(added_id),
            Some(SheetDocument::Calculation(_))
        ));
        Ok(())
    }

    #[test]
    fn apply_snapshot_deletes_calculation_sheet_and_its_sidecars() -> Result<()> {
        let temp = tempdir()?;
        let root = temp.path().join("deleted-calculation.polysheet");
        let mut project = PolySheetProject::create_unsaved("Calculation delete");
        project.save_as(&root, true)?;
        let sheet_id = project.manifest.sheets[0].clone();
        let sheet_directory = root.join("sheets").join(&sheet_id);
        assert!(sheet_directory.is_dir());

        project.apply_snapshot(&ProjectSnapshot {
            sheets: BTreeMap::new(),
        })?;
        project.save(true)?;

        assert!(project.sheets.is_empty());
        assert!(project.manifest.sheets.is_empty());
        assert!(!sheet_directory.exists());
        let reopened = PolySheetProject::open(&root, OpenProjectOptions::default())?;
        assert!(reopened.sheets.is_empty());
        assert!(reopened.manifest.sheets.is_empty());
        Ok(())
    }

    #[test]
    fn structural_save_deletes_only_known_sidecars_and_preserves_source_data() -> Result<()> {
        let (_temp, mut project) = fixture()?;
        project.save(true)?;
        let sheet_id = project.manifest.sheets[0].clone();
        let directory = project.root.join("sheets").join(&sheet_id);
        let unknown = directory.join("notes.keep");
        fs::write(&unknown, "user-owned")?;
        let source_path = match project.sheets.get(&sheet_id).unwrap() {
            SheetDocument::Data(sheet) => PathBuf::from(&sheet.source_path),
            _ => unreachable!(),
        };
        let source_before = fs::read(&source_path)?;

        project.apply_snapshot(&ProjectSnapshot {
            sheets: BTreeMap::new(),
        })?;
        project.save(true)?;

        assert!(project.removed_sheet_ids.is_empty());
        assert_eq!(fs::read(&source_path)?, source_before);
        assert_eq!(fs::read_to_string(&unknown)?, "user-owned");
        for file_name in SHEET_SIDECAR_FILE_NAMES {
            assert!(!directory.join(file_name).exists(), "{file_name} remains");
        }
        assert!(directory.is_dir(), "unknown file must keep the directory");

        project.save(true)?;
        assert_eq!(fs::read(&source_path)?, source_before);
        assert_eq!(fs::read_to_string(&unknown)?, "user-owned");
        let reopened = PolySheetProject::open(&project.root, OpenProjectOptions::default())?;
        assert!(reopened.sheets.is_empty());
        Ok(())
    }

    #[test]
    fn apply_snapshot_replaces_data_sheet_and_removes_deleted_sidecars() -> Result<()> {
        let (temp, mut project) = fixture()?;
        project.save(true)?;
        let old_id = project.manifest.sheets[0].clone();
        let old_directory = project.root.join("sheets").join(&old_id);
        assert!(old_directory.is_dir());

        let mut snapshot = ProjectSnapshot::from_project(&project);
        let mut replacement = snapshot.sheets.remove(&old_id).unwrap();
        let replacement_id = "018f0000-0000-7000-8000-000000000098";
        replacement.id = replacement_id.into();
        replacement.name = "Replacement items".into();
        snapshot.sheets.insert(replacement_id.into(), replacement);

        project.apply_snapshot(&snapshot)?;
        assert!(!project.sheets.contains_key(&old_id));
        assert!(project.removed_sheet_ids.contains(&old_id));
        assert_eq!(project.manifest.sheets, vec![replacement_id]);
        project.save(true)?;

        assert!(!old_directory.exists());
        assert!(project
            .root
            .join("sheets")
            .join(replacement_id)
            .join("formulas.json")
            .is_file());
        assert!(temp.path().join("data/items.json").is_file());
        let reopened = PolySheetProject::open(&project.root, OpenProjectOptions::default())?;
        assert_eq!(reopened.manifest.sheets, vec![replacement_id]);
        assert!(matches!(
            reopened.sheets.get(replacement_id),
            Some(SheetDocument::Data(_))
        ));
        Ok(())
    }

    #[test]
    fn apply_snapshot_rejects_new_data_sheet_without_metadata_without_mutating_project(
    ) -> Result<()> {
        let (_temp, mut project) = fixture()?;
        let before_manifest = project.manifest.clone();
        let before_sheets = project.sheets.clone();
        let old_id = project.manifest.sheets[0].clone();
        let mut snapshot = ProjectSnapshot::from_project(&project);
        let mut added = snapshot.sheets.remove(&old_id).unwrap();
        added.id = "missing-metadata".into();
        added.definition = None;
        added.source_path = None;
        snapshot.sheets.insert(added.id.clone(), added);

        let error = project.apply_snapshot(&snapshot).unwrap_err();

        assert!(error.to_string().contains("missing its definition"));
        assert_eq!(project.manifest, before_manifest);
        assert_eq!(project.sheets, before_sheets);
        assert!(project.removed_sheet_ids.is_empty());
        Ok(())
    }

    #[test]
    fn apply_snapshot_rejects_data_source_path_mismatch() -> Result<()> {
        let (temp, mut project) = fixture()?;
        let mut snapshot = ProjectSnapshot::from_project(&project);
        let sheet = snapshot.sheets.values_mut().next().unwrap();
        sheet.source_path = Some(
            temp.path()
                .join("data/other.json")
                .to_string_lossy()
                .to_string(),
        );

        let error = project.apply_snapshot(&snapshot).unwrap_err();

        assert!(error.to_string().contains("does not match schema source"));
        Ok(())
    }

    #[test]
    fn open_rejects_sheet_id_path_traversal() -> Result<()> {
        let temp = tempdir()?;
        let root = temp.path().join("bad.polysheet");
        fs::create_dir_all(&root)?;
        fs::write(
            root.join("workbook.toml"),
            r#"format_version = 1
name = "Bad"
sheets = ["../outside"]
"#,
        )?;

        let error = PolySheetProject::open(&root, OpenProjectOptions::default()).unwrap_err();

        assert!(error.to_string().contains("invalid sheet id"));
        Ok(())
    }

    #[test]
    fn sheet_ids_reject_both_path_separator_styles() {
        assert!(validate_sheet_id("nested/sheet").is_err());
        assert!(validate_sheet_id("nested\\sheet").is_err());
    }

    #[test]
    fn open_rejects_duplicate_sheet_ids() -> Result<()> {
        let temp = tempdir()?;
        let root = temp.path().join("duplicate.polysheet");
        let mut project = PolySheetProject::create_unsaved("Duplicate");
        project.save_as(&root, true)?;
        let sheet_id = project.manifest.sheets[0].clone();
        project.manifest.sheets.push(sheet_id.clone());
        fs::write(
            root.join("workbook.toml"),
            canonical_toml(&project.manifest)?,
        )?;

        let error = PolySheetProject::open(&root, OpenProjectOptions::default()).unwrap_err();

        assert!(error
            .to_string()
            .contains(&format!("duplicate sheet id '{sheet_id}'")));
        Ok(())
    }

    #[test]
    fn open_rejects_duplicate_data_table_bindings() -> Result<()> {
        let (_temp, mut project) = fixture()?;
        project.save(true)?;
        let source_id = project.manifest.sheets[0].clone();
        let copied_id = "018f0000-0000-7000-8000-000000000077";
        persist_data_sheet_copy(&mut project, &source_id, copied_id, "game.Item")?;

        let error = PolySheetProject::open(&project.root, OpenProjectOptions::default())
            .unwrap_err()
            .to_string();

        assert!(error.contains("game.Item"));
        assert!(error.contains("more than one data sheet"));
        Ok(())
    }

    #[test]
    fn bind_and_open_reject_data_tables_that_share_a_json_source() -> Result<()> {
        let temp = tempdir()?;
        let schema = temp.path().join("shared.poly");
        let sources = temp.path().join("shared.sources.toml");
        let data = temp.path().join("data/shared.json");
        fs::create_dir_all(data.parent().unwrap())?;
        fs::write(
            &schema,
            "namespace game { table First { id: u32 primary_key; } table Second { id: u32 primary_key; } }",
        )?;
        fs::write(
            &sources,
            r#"[tables."game.First".load]
json = "data/shared.json"

[tables."game.Second".load]
json = "data/./shared.json"
"#,
        )?;
        fs::write(&data, "[]\n")?;
        let root = temp.path().join("shared.polysheet");
        let mut project = PolySheetProject::create(&root, "Shared", &schema, Some(&sources), None)?;
        let first_id = project.bind_data_sheet("First", "game.First")?;

        let bind_error = project
            .bind_data_sheet("Second", "game.Second")
            .unwrap_err()
            .to_string();
        assert!(bind_error.contains("share JSON source"));
        project.save(true)?;

        let copied_id = "018f0000-0000-7000-8000-000000000076";
        persist_data_sheet_copy(&mut project, &first_id, copied_id, "game.Second")?;
        let open_error = PolySheetProject::open(&root, OpenProjectOptions::default())
            .unwrap_err()
            .to_string();
        assert!(open_error.contains("share JSON source"));
        Ok(())
    }

    #[test]
    fn schema_directory_reports_a_specific_user_error() -> Result<()> {
        let temp = tempdir()?;
        let mut project = PolySheetProject::create_unsaved("Draft");
        let error = project.attach_schema(temp.path(), None, None).unwrap_err();
        assert!(error
            .to_string()
            .contains("폴더가 아니라 .poly 파일을 선택해 주세요"));
        Ok(())
    }

    #[test]
    #[ignore = "large 100k x 20 acceptance fixture"]
    fn canonicalizes_100k_rows_by_20_scalar_fields() -> Result<()> {
        let temp = tempdir()?;
        let schema_path = temp.path().join("large.poly");
        let fields = (0..20)
            .map(|index| format!("f{index}: u32;"))
            .collect::<Vec<_>>()
            .join(" ");
        fs::write(
            &schema_path,
            format!("namespace perf {{ table Row {{ {fields} }} }}"),
        )?;
        let loaded = polygen::load_project_schema(&schema_path, None)?;
        let index = SchemaIndex::from_schema(&loaded.schema);
        let table = index.table("perf.Row").unwrap();
        let rows = (0..100_000u32)
            .map(|row| {
                Value::Object(
                    (0..20)
                        .map(|column| {
                            (
                                format!("f{column}"),
                                Value::Number(serde_json::Number::from(row + column)),
                            )
                        })
                        .collect(),
                )
            })
            .collect::<Vec<_>>();
        assert!(validate_table_rows(table, &rows, &index).is_empty());
        let canonical = canonical_rows_json(&rows, table, &index)?;
        assert!(canonical.starts_with("[\n  {\n    \"f0\": 0,"));
        assert!(canonical.ends_with("  }\n]\n"));
        Ok(())
    }
}
