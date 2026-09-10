use polysheet_core::diff::DiffReport;
use polysheet_core::git::{list_references, snapshot_at_revision, stage_sheet_files};
use polysheet_core::merge::{merge_snapshots, MergeReport};
use polysheet_core::model::{
    CalculationDocument, FormulaDocument, RowComparison, RowDraft, RowRecord, SheetDocument,
    SheetKind,
};
use polysheet_core::validation::{table_fields, Diagnostic};
use polysheet_core::{OpenProjectOptions, PolySheetProject};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::State;

#[derive(Default)]
pub struct AppState {
    inner: Mutex<StateData>,
}

#[derive(Default)]
struct StateData {
    project: Option<PolySheetProject>,
    pending_merge: Option<MergeReport>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSummary {
    name: String,
    root: String,
    schema_path: String,
    is_saved: bool,
    tables: Vec<TableSummary>,
    sheets: Vec<SheetSummary>,
    diagnostics: Vec<Diagnostic>,
    normalization_required: Vec<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableSummary {
    fqn: String,
    name: String,
    is_readonly: bool,
    json_source: Option<String>,
    fields: Vec<FieldSummary>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldSummary {
    name: String,
    type_name: String,
    is_primary_key: bool,
    is_optional: bool,
    is_list: bool,
    is_enum: bool,
    is_struct: bool,
    enum_values: Vec<String>,
    foreign_key_target: Option<String>,
    foreign_key_field: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetSummary {
    id: String,
    name: String,
    kind: SheetKind,
    definition: Option<String>,
    row_count: usize,
    column_count: usize,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RowChunk {
    offset: usize,
    total: usize,
    rows: Vec<RowRecord>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NormalizationPreview {
    path: String,
    before: String,
    after: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CellEdit {
    sheet_id: String,
    row_id: String,
    field_path: String,
    value: Value,
    formula: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CalculationEdit {
    sheet_id: String,
    document: CalculationDocument,
}

#[tauri::command]
pub fn create_unsaved_project(
    state: State<'_, AppState>,
    name: String,
) -> Result<ProjectSummary, String> {
    command(|| {
        let project = PolySheetProject::create_unsaved(if name.trim().is_empty() {
            "제목 없는 PolySheet".to_string()
        } else {
            name.trim().to_string()
        });
        let summary = summarize(&project);
        state.inner.lock().unwrap().project = Some(project);
        Ok(summary)
    })
}

#[tauri::command]
pub fn create_project(
    state: State<'_, AppState>,
    root: String,
    name: String,
    schema: String,
    sources: Option<String>,
    data_root: Option<String>,
) -> Result<ProjectSummary, String> {
    command(|| {
        let project = PolySheetProject::create(
            root,
            name,
            schema,
            sources.as_deref().map(Path::new),
            data_root.as_deref().map(Path::new),
        )?;
        let summary = summarize(&project);
        state.inner.lock().unwrap().project = Some(project);
        Ok(summary)
    })
}

#[tauri::command]
pub fn open_project(state: State<'_, AppState>, root: String) -> Result<ProjectSummary, String> {
    command(|| {
        let root_path = resolve_project_root(Path::new(&root))?;
        let project = PolySheetProject::open(&root_path, OpenProjectOptions::default())?;
        let summary = summarize(&project);
        state.inner.lock().unwrap().project = Some(project);
        Ok(summary)
    })
}

fn resolve_project_root(selected: &Path) -> anyhow::Result<PathBuf> {
    if !selected.is_dir() {
        anyhow::bail!(
            "PolySheet 프로젝트 폴더를 선택해 주세요: {}",
            selected.display()
        );
    }
    if selected.join("workbook.toml").is_file() {
        return Ok(selected.to_path_buf());
    }

    let selected_name = selected
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let mut candidates = Vec::new();
    for search_root in selected.ancestors().take(4) {
        let Ok(entries) = std::fs::read_dir(search_root) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let is_project_dir = path.is_dir()
                && path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("polysheet"))
                && path.join("workbook.toml").is_file();
            if is_project_dir && !candidates.contains(&path) {
                candidates.push(path);
            }
        }
    }

    if let Some(exact_match) = candidates.iter().find(|candidate| {
        candidate
            .file_stem()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case(selected_name))
    }) {
        return Ok(exact_match.clone());
    }
    if candidates.len() == 1 {
        return Ok(candidates.remove(0));
    }

    if candidates.is_empty() {
        anyhow::bail!(
            "선택한 폴더에 workbook.toml이 없고, 가까운 위치에서도 .polysheet 프로젝트를 찾지 못했습니다: {}",
            selected.display()
        );
    }
    let candidate_list = candidates
        .iter()
        .map(|candidate| candidate.display().to_string())
        .collect::<Vec<_>>()
        .join(", ");
    anyhow::bail!(
        "가까운 위치에 PolySheet 프로젝트가 여러 개 있습니다. 열 프로젝트 폴더를 선택해 주세요: {candidate_list}"
    );
}

#[tauri::command]
pub fn attach_schema(
    state: State<'_, AppState>,
    schema: String,
    sources: Option<String>,
    data_root: Option<String>,
) -> Result<ProjectSummary, String> {
    with_project_mut(&state, |project| {
        project.attach_schema(
            schema,
            sources.as_deref().map(Path::new),
            data_root.as_deref().map(Path::new),
        )?;
        Ok(summarize(project))
    })
}

#[tauri::command]
pub fn save_project_as(
    state: State<'_, AppState>,
    root: String,
    approve_normalization: bool,
) -> Result<ProjectSummary, String> {
    with_project_mut(&state, |project| {
        save_project_as_atomically(project, Path::new(&root), approve_normalization)?;
        Ok(summarize(project))
    })
}

fn save_project_as_atomically(
    project: &mut PolySheetProject,
    root: &Path,
    approve_normalization: bool,
) -> anyhow::Result<()> {
    if !approve_normalization && !project.normalization_required.is_empty() {
        anyhow::bail!("JSON normalization approval is required before the first save");
    }
    let mut candidate = project.clone();
    candidate.save_as(root, approve_normalization)?;
    *project = candidate;
    Ok(())
}

#[tauri::command]
pub fn project_summary(state: State<'_, AppState>) -> Result<ProjectSummary, String> {
    with_project(&state, |project| Ok(summarize(project)))
}

#[tauri::command]
pub fn bind_data_sheet(
    state: State<'_, AppState>,
    name: String,
    definition: String,
) -> Result<ProjectSummary, String> {
    with_project_mut(&state, |project| {
        project.bind_data_sheet(name, definition)?;
        Ok(summarize(project))
    })
}

#[tauri::command]
pub fn add_calculation_sheet(
    state: State<'_, AppState>,
    name: String,
) -> Result<ProjectSummary, String> {
    with_project_mut(&state, |project| {
        project.add_calculation_sheet(name);
        Ok(summarize(project))
    })
}

#[tauri::command]
pub fn get_rows_chunk(
    state: State<'_, AppState>,
    sheet_id: String,
    offset: usize,
    limit: usize,
) -> Result<RowChunk, String> {
    with_project(&state, |project| {
        let sheet = match project.sheets.get(&sheet_id) {
            Some(SheetDocument::Data(sheet)) => sheet,
            _ => anyhow::bail!("unknown data sheet '{sheet_id}'"),
        };
        let end = offset
            .saturating_add(limit.min(5_000))
            .min(sheet.rows.len());
        let rows = if offset < end {
            sheet.rows[offset..end]
                .iter()
                .cloned()
                .map(frontend_safe_row)
                .collect()
        } else {
            Vec::new()
        };
        Ok(RowChunk {
            offset,
            total: sheet.rows.len(),
            rows,
        })
    })
}

#[tauri::command]
pub fn get_sheet_json(state: State<'_, AppState>, sheet_id: String) -> Result<String, String> {
    with_project(&state, |project| project.data_sheet_json(&sheet_id))
}

#[tauri::command]
pub fn get_row_json(
    state: State<'_, AppState>,
    sheet_id: String,
    row_id: String,
) -> Result<String, String> {
    with_project(&state, |project| project.get_row_json(&sheet_id, &row_id))
}

#[tauri::command]
pub fn get_row_draft(state: State<'_, AppState>, sheet_id: String) -> Result<RowDraft, String> {
    with_project(&state, |project| project.get_row_draft(&sheet_id))
}

#[tauri::command]
pub fn compare_rows(
    state: State<'_, AppState>,
    sheet_id: String,
    row_ids: Vec<String>,
) -> Result<RowComparison, String> {
    with_project(&state, |project| {
        let mut comparison = project.compare_rows(&sheet_id, &row_ids)?;
        for field in &mut comparison.fields {
            for value in &mut field.values {
                value.value = frontend_safe_value(std::mem::take(&mut value.value));
            }
        }
        Ok(comparison)
    })
}

#[tauri::command]
pub fn get_sheet_formulas(
    state: State<'_, AppState>,
    sheet_id: String,
) -> Result<FormulaDocument, String> {
    with_project(&state, |project| project.formula_document(&sheet_id))
}

#[tauri::command]
pub fn get_calculation_sheet(
    state: State<'_, AppState>,
    sheet_id: String,
) -> Result<CalculationDocument, String> {
    with_project(&state, |project| project.calculation_document(&sheet_id))
}

#[tauri::command]
pub fn apply_calculation_sheet(
    state: State<'_, AppState>,
    sheet_id: String,
    document: CalculationDocument,
) -> Result<ProjectSummary, String> {
    with_project_mut(&state, |project| {
        project.replace_calculation_document(&sheet_id, document)?;
        Ok(summarize(project))
    })
}

#[tauri::command]
pub fn insert_calculation_row(
    state: State<'_, AppState>,
    sheet_id: String,
    index: usize,
) -> Result<ProjectSummary, String> {
    with_project_mut(&state, |project| {
        project.insert_calculation_row(&sheet_id, index)?;
        Ok(summarize(project))
    })
}

#[tauri::command]
pub fn insert_calculation_column(
    state: State<'_, AppState>,
    sheet_id: String,
    index: usize,
) -> Result<ProjectSummary, String> {
    with_project_mut(&state, |project| {
        project.insert_calculation_column(&sheet_id, index)?;
        Ok(summarize(project))
    })
}

#[tauri::command]
pub fn apply_sheet_json(
    state: State<'_, AppState>,
    sheet_id: String,
    json: String,
) -> Result<ProjectSummary, String> {
    with_project_mut(&state, |project| {
        project.replace_data_sheet_json(&sheet_id, &json)?;
        Ok(summarize(project))
    })
}

#[tauri::command]
pub fn apply_row_json(
    state: State<'_, AppState>,
    sheet_id: String,
    row_id: String,
    json: String,
) -> Result<ProjectSummary, String> {
    with_project_mut(&state, |project| {
        project.apply_row_json(&sheet_id, &row_id, &json)?;
        Ok(summarize(project))
    })
}

#[tauri::command]
pub fn insert_row_json(
    state: State<'_, AppState>,
    sheet_id: String,
    json: String,
) -> Result<InsertedRow, String> {
    with_project_mut(&state, |project| {
        let row_id = project.insert_row_json(&sheet_id, &json)?;
        Ok(InsertedRow {
            row_id,
            project: summarize(project),
        })
    })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InsertedRow {
    row_id: String,
    project: ProjectSummary,
}

#[tauri::command]
pub fn apply_cell_edits(
    state: State<'_, AppState>,
    edits: Vec<CellEdit>,
    calculation_edits: Vec<CalculationEdit>,
) -> Result<ProjectSummary, String> {
    with_project_mut(&state, |project| {
        apply_cell_edits_atomically(project, edits, calculation_edits)?;
        Ok(summarize(project))
    })
}

fn apply_cell_edits_atomically(
    project: &mut PolySheetProject,
    edits: Vec<CellEdit>,
    calculation_edits: Vec<CalculationEdit>,
) -> anyhow::Result<()> {
    let baseline_errors = validation_error_keys(project);
    let mut candidate = project.clone();
    for edit in edits {
        candidate.set_formula(
            &edit.sheet_id,
            &edit.row_id,
            &edit.field_path,
            edit.formula,
            edit.value,
        )?;
    }
    for edit in calculation_edits {
        candidate.replace_calculation_document(&edit.sheet_id, edit.document)?;
    }
    candidate.refresh_diagnostics();

    let introduced_errors = validation_error_keys(&candidate)
        .difference(&baseline_errors)
        .cloned()
        .collect::<Vec<_>>();
    if !introduced_errors.is_empty() {
        anyhow::bail!(
            "cell edits failed validation: {}",
            introduced_errors.join("; ")
        );
    }

    *project = candidate;
    Ok(())
}

fn validation_error_keys(project: &PolySheetProject) -> std::collections::BTreeSet<String> {
    project
        .diagnostics
        .iter()
        .filter(|diagnostic| {
            diagnostic.severity == polysheet_core::validation::DiagnosticSeverity::Error
        })
        .map(|diagnostic| format!("{}: {}", diagnostic.path, diagnostic.message))
        .collect()
}

#[tauri::command]
pub fn apply_row_order(
    state: State<'_, AppState>,
    sheet_id: String,
    row_ids: Vec<String>,
) -> Result<ProjectSummary, String> {
    with_project_mut(&state, |project| {
        project.apply_row_order(&sheet_id, &row_ids)?;
        Ok(summarize(project))
    })
}

#[tauri::command]
pub fn confirm_row_identity(
    state: State<'_, AppState>,
    sheet_id: String,
    current_row_id: String,
    replacement_row_id: String,
) -> Result<ProjectSummary, String> {
    with_project_mut(&state, |project| {
        project.confirm_row_identity(&sheet_id, &current_row_id, &replacement_row_id)?;
        Ok(summarize(project))
    })
}

#[tauri::command]
pub fn get_normalization_preview(
    state: State<'_, AppState>,
) -> Result<Vec<NormalizationPreview>, String> {
    with_project(&state, |project| {
        Ok(project
            .normalization_preview()?
            .into_iter()
            .map(|(path, before, after)| NormalizationPreview {
                path,
                before,
                after,
            })
            .collect())
    })
}

#[tauri::command]
pub fn save_project(
    state: State<'_, AppState>,
    approve_normalization: bool,
) -> Result<ProjectSummary, String> {
    with_project_mut(&state, |project| {
        project.save(approve_normalization)?;
        Ok(summarize(project))
    })
}

#[tauri::command]
pub fn list_git_refs(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    with_project(&state, list_references)
}

#[tauri::command]
pub fn diff_project(
    state: State<'_, AppState>,
    base: String,
    target: String,
) -> Result<DiffReport, String> {
    with_project(&state, |project| {
        let base = snapshot_at_revision(project, &base)?;
        let target = snapshot_at_revision(project, &target)?;
        Ok(polysheet_core::diff_snapshots(&base, &target))
    })
}

#[tauri::command]
pub fn preview_merge(
    state: State<'_, AppState>,
    base: String,
    ours: String,
    theirs: String,
) -> Result<MergeReport, String> {
    command(|| {
        let mut state_data = state.inner.lock().unwrap();
        let project = state_data
            .project
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("no PolySheet project is open"))?;
        let base = snapshot_at_revision(project, &base)?;
        let ours = snapshot_at_revision(project, &ours)?;
        let theirs = snapshot_at_revision(project, &theirs)?;
        let report = merge_snapshots(&base, &ours, &theirs);
        state_data.pending_merge = Some(report.clone());
        Ok(report)
    })
}

#[tauri::command]
pub fn apply_pending_merge(state: State<'_, AppState>) -> Result<ProjectSummary, String> {
    command(|| {
        let mut state_data = state.inner.lock().unwrap();
        let report = state_data
            .pending_merge
            .as_ref()
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("there is no pending merge"))?;
        if !report.conflicts.is_empty() {
            anyhow::bail!("merge still has unresolved conflicts");
        }
        let project = state_data
            .project
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("no PolySheet project is open"))?;
        apply_merge_atomically(project, &report)?;
        state_data.pending_merge = None;
        Ok(summarize(state_data.project.as_ref().unwrap()))
    })
}

fn apply_merge_atomically(
    project: &mut PolySheetProject,
    report: &MergeReport,
) -> anyhow::Result<()> {
    if !project.normalization_required.is_empty() {
        anyhow::bail!(
            "JSON normalization approval is required before applying a merge; review and save the normalization first"
        );
    }
    let mut candidate = project.clone();
    candidate.apply_snapshot(&report.merged)?;
    if !report.recalculation_required {
        candidate.save(false)?;
    }
    *project = candidate;
    Ok(())
}

#[tauri::command]
pub fn stage_sheets(
    state: State<'_, AppState>,
    sheet_ids: Vec<String>,
) -> Result<Vec<String>, String> {
    with_project_mut(&state, |project| {
        project.save(false)?;
        stage_sheet_files(project, &sheet_ids)
    })
}

fn summarize(project: &PolySheetProject) -> ProjectSummary {
    let tables = project
        .schema_index
        .tables
        .values()
        .map(|table| TableSummary {
            fqn: table.fqn.clone(),
            name: table.name.clone(),
            is_readonly: table.is_readonly,
            json_source: table.load.as_ref().and_then(|load| load.json.clone()),
            fields: table_fields(table)
                .into_iter()
                .map(|field| {
                    let enum_values = project
                        .schema_index
                        .enums
                        .get(&unwrap_type(&field.field_type).fqn)
                        .map(|definition| {
                            definition
                                .items
                                .iter()
                                .filter_map(|item| match item {
                                    polygen::ir_model::EnumItem::Member(member) => {
                                        Some(member.name.clone())
                                    }
                                    _ => None,
                                })
                                .collect()
                        })
                        .unwrap_or_default();
                    FieldSummary {
                        name: field.name.clone(),
                        type_name: field.field_type.original.clone(),
                        is_primary_key: field.is_primary_key,
                        is_optional: field.field_type.is_option,
                        is_list: field.field_type.is_list,
                        is_enum: unwrap_type(&field.field_type).is_enum,
                        is_struct: unwrap_type(&field.field_type).is_struct,
                        enum_values,
                        foreign_key_target: field
                            .foreign_key
                            .as_ref()
                            .map(|foreign_key| foreign_key.target_table_fqn.clone()),
                        foreign_key_field: field
                            .foreign_key
                            .as_ref()
                            .map(|foreign_key| foreign_key.target_field.clone()),
                    }
                })
                .collect(),
        })
        .collect();
    let sheets = project
        .manifest
        .sheets
        .iter()
        .filter_map(|id| project.sheets.get(id))
        .map(|sheet| {
            let manifest = match sheet {
                SheetDocument::Data(sheet) => &sheet.manifest,
                SheetDocument::Calculation(sheet) => &sheet.manifest,
            };
            SheetSummary {
                id: manifest.id.clone(),
                name: manifest.name.clone(),
                kind: manifest.kind,
                definition: manifest.definition.clone(),
                row_count: manifest.row_count,
                column_count: manifest.column_count,
            }
        })
        .collect();
    ProjectSummary {
        name: project.manifest.name.clone(),
        root: project.root.to_string_lossy().to_string(),
        schema_path: project
            .loaded_schema
            .as_ref()
            .map(|loaded| loaded.schema_path.to_string_lossy().to_string())
            .unwrap_or_default(),
        is_saved: project.is_saved(),
        tables,
        sheets,
        diagnostics: project.diagnostics.clone(),
        normalization_required: project
            .normalization_required
            .iter()
            .map(|path| path.to_string_lossy().to_string())
            .collect(),
    }
}

fn unwrap_type(mut type_ref: &polygen::ir_model::TypeRef) -> &polygen::ir_model::TypeRef {
    while let Some(inner) = type_ref.inner_type.as_deref() {
        type_ref = inner;
    }
    type_ref
}

fn frontend_safe_row(mut row: RowRecord) -> RowRecord {
    row.value = frontend_safe_value(row.value);
    row
}

fn frontend_safe_value(value: Value) -> Value {
    const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
    match value {
        Value::Number(number) => {
            let unsafe_integer = number
                .as_u64()
                .is_some_and(|value| value > MAX_SAFE_INTEGER)
                || number
                    .as_i64()
                    .is_some_and(|value| value.unsigned_abs() > MAX_SAFE_INTEGER);
            if unsafe_integer {
                Value::String(number.to_string())
            } else {
                Value::Number(number)
            }
        }
        Value::Array(values) => Value::Array(values.into_iter().map(frontend_safe_value).collect()),
        Value::Object(object) => Value::Object(
            object
                .into_iter()
                .map(|(key, value)| (key, frontend_safe_value(value)))
                .collect(),
        ),
        other => other,
    }
}

fn command<T>(operation: impl FnOnce() -> anyhow::Result<T>) -> Result<T, String> {
    operation().map_err(|error| format!("{error:#}"))
}

fn with_project<T>(
    state: &State<'_, AppState>,
    operation: impl FnOnce(&PolySheetProject) -> anyhow::Result<T>,
) -> Result<T, String> {
    command(|| {
        let state = state.inner.lock().unwrap();
        let project = state
            .project
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("no PolySheet project is open"))?;
        operation(project)
    })
}

fn with_project_mut<T>(
    state: &State<'_, AppState>,
    operation: impl FnOnce(&mut PolySheetProject) -> anyhow::Result<T>,
) -> Result<T, String> {
    command(|| {
        let mut state = state.inner.lock().unwrap();
        let project = state
            .project
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("no PolySheet project is open"))?;
        operation(project)
    })
}

#[cfg(test)]
mod project_root_tests {
    use super::resolve_project_root;
    use std::fs;

    fn create_project(root: &std::path::Path) {
        fs::create_dir_all(root).unwrap();
        fs::write(
            root.join("workbook.toml"),
            "format_version = 1\nname = \"Test\"\n",
        )
        .unwrap();
    }

    #[test]
    fn opens_a_selected_project_directory_directly() {
        let temp = tempfile::tempdir().unwrap();
        let project = temp.path().join("game.polysheet");
        create_project(&project);

        assert_eq!(resolve_project_root(&project).unwrap(), project);
    }

    #[test]
    fn finds_the_matching_project_from_its_data_directory() {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("examples/data/game");
        fs::create_dir_all(&data).unwrap();
        let project = temp.path().join("examples/game.polysheet");
        create_project(&project);
        create_project(&temp.path().join("examples/other.polysheet"));

        assert_eq!(resolve_project_root(&data).unwrap(), project);
    }

    #[test]
    fn reports_ambiguous_nearby_projects() {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("examples/data");
        fs::create_dir_all(&data).unwrap();
        create_project(&temp.path().join("examples/first.polysheet"));
        create_project(&temp.path().join("examples/second.polysheet"));

        let error = resolve_project_root(&data).unwrap_err().to_string();
        assert!(error.contains("여러 개"));
    }
}

#[cfg(test)]
mod cell_edit_tests {
    use super::{
        apply_cell_edits_atomically, apply_merge_atomically, save_project_as_atomically,
        CalculationEdit, CellEdit,
    };
    use polysheet_core::diff::ProjectSnapshot;
    use polysheet_core::merge::MergeReport;
    use polysheet_core::PolySheetProject;
    use serde_json::json;
    use std::fs;

    fn fixture() -> anyhow::Result<(tempfile::TempDir, PolySheetProject, String, String)> {
        let temp = tempfile::tempdir()?;
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
            r#"[{"id":1,"name":"before"}]"#,
        )?;
        let root = temp.path().join("game.polysheet");
        let mut project = PolySheetProject::create(&root, "Game", &schema, Some(&sources), None)?;
        let sheet_id = project.bind_data_sheet("Items", "game.Item")?;
        let row_id = match project.sheets.get(&sheet_id) {
            Some(polysheet_core::model::SheetDocument::Data(sheet)) => sheet.rows[0].id.clone(),
            _ => unreachable!(),
        };
        Ok((temp, project, sheet_id, row_id))
    }

    #[test]
    fn failed_cell_edit_batch_preserves_the_original_session() -> anyhow::Result<()> {
        let (_temp, mut project, sheet_id, row_id) = fixture()?;
        let before = project.get_row_json(&sheet_id, &row_id)?;

        let result = apply_cell_edits_atomically(
            &mut project,
            vec![
                CellEdit {
                    sheet_id: sheet_id.clone(),
                    row_id: row_id.clone(),
                    field_path: "name".to_string(),
                    value: json!("after"),
                    formula: None,
                },
                CellEdit {
                    sheet_id: sheet_id.clone(),
                    row_id: row_id.clone(),
                    field_path: "missing".to_string(),
                    value: json!("invalid"),
                    formula: None,
                },
            ],
            Vec::new(),
        );

        assert!(result.is_err());
        assert_eq!(project.get_row_json(&sheet_id, &row_id)?, before);
        Ok(())
    }

    #[test]
    fn successful_cell_edit_batch_swaps_the_validated_candidate() -> anyhow::Result<()> {
        let (_temp, mut project, sheet_id, row_id) = fixture()?;

        apply_cell_edits_atomically(
            &mut project,
            vec![CellEdit {
                sheet_id: sheet_id.clone(),
                row_id: row_id.clone(),
                field_path: "name".to_string(),
                value: json!("after"),
                formula: None,
            }],
            Vec::new(),
        )?;

        assert!(project
            .get_row_json(&sheet_id, &row_id)?
            .contains("\"after\""));
        Ok(())
    }

    #[test]
    fn failed_combined_grid_batch_preserves_data_and_calculation_sheets() -> anyhow::Result<()> {
        let (_temp, mut project, sheet_id, row_id) = fixture()?;
        let calculation_id = project.add_calculation_sheet("Calculation");
        let before_data = project.get_row_json(&sheet_id, &row_id)?;
        let before_calculation = project.calculation_document(&calculation_id)?;
        let mut invalid_calculation = before_calculation.clone();
        invalid_calculation.version = u32::MAX;

        let result = apply_cell_edits_atomically(
            &mut project,
            vec![CellEdit {
                sheet_id: sheet_id.clone(),
                row_id: row_id.clone(),
                field_path: "name".to_string(),
                value: json!("after"),
                formula: None,
            }],
            vec![CalculationEdit {
                sheet_id: calculation_id.clone(),
                document: invalid_calculation,
            }],
        );

        assert!(result.is_err());
        assert_eq!(project.get_row_json(&sheet_id, &row_id)?, before_data);
        assert_eq!(
            project.calculation_document(&calculation_id)?,
            before_calculation
        );
        Ok(())
    }

    #[test]
    fn failed_save_as_preserves_the_session_and_does_not_create_the_target() -> anyhow::Result<()> {
        let (temp, mut project, _sheet_id, _row_id) = fixture()?;
        let original_root = project.root.clone();
        let target = temp.path().join("new.polysheet");

        let result = save_project_as_atomically(&mut project, &target, false);

        assert!(result.is_err());
        assert_eq!(project.root, original_root);
        assert!(!target.exists());
        Ok(())
    }

    #[test]
    fn merge_cannot_implicitly_approve_pending_normalization() -> anyhow::Result<()> {
        let (_temp, mut project, _sheet_id, _row_id) = fixture()?;
        let before = ProjectSnapshot::from_project(&project);
        let report = MergeReport {
            merged: before.clone(),
            conflicts: Vec::new(),
            recalculation_required: false,
        };

        let result = apply_merge_atomically(&mut project, &report);

        assert!(result.is_err());
        assert_eq!(ProjectSnapshot::from_project(&project), before);
        Ok(())
    }
}
