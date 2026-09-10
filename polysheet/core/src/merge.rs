use crate::diff::{ProjectSnapshot, SnapshotSheet};
use crate::model::{RowRecord, SheetKind};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MergeConflict {
    pub sheet_id: String,
    pub row_id: Option<String>,
    pub path: String,
    pub base: Option<Value>,
    pub ours: Option<Value>,
    pub theirs: Option<Value>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MergeReport {
    pub merged: ProjectSnapshot,
    pub conflicts: Vec<MergeConflict>,
    pub recalculation_required: bool,
}

pub fn merge_snapshots(
    base: &ProjectSnapshot,
    ours: &ProjectSnapshot,
    theirs: &ProjectSnapshot,
) -> MergeReport {
    let mut conflicts = Vec::new();
    let mut recalculation_required = false;
    let mut sheets = BTreeMap::new();
    let sheet_ids = base
        .sheets
        .keys()
        .chain(ours.sheets.keys())
        .chain(theirs.sheets.keys())
        .cloned()
        .collect::<BTreeSet<_>>();

    for sheet_id in sheet_ids {
        let base_sheet = base.sheets.get(&sheet_id);
        let ours_sheet = ours.sheets.get(&sheet_id);
        let theirs_sheet = theirs.sheets.get(&sheet_id);
        let merged = match (base_sheet, ours_sheet, theirs_sheet) {
            (None, Some(ours), None) => Some(ours.clone()),
            (None, None, Some(theirs)) => Some(theirs.clone()),
            (None, Some(ours), Some(theirs)) if ours == theirs => Some(ours.clone()),
            (Some(base), Some(ours), None) if ours == base => None,
            (Some(base), None, Some(theirs)) if theirs == base => None,
            (Some(_), None, None) => None,
            (Some(base), Some(ours), Some(theirs)) => Some(merge_sheet(
                base,
                ours,
                theirs,
                &mut conflicts,
                &mut recalculation_required,
            )),
            (_, Some(ours), Some(theirs)) => {
                conflicts.push(MergeConflict {
                    sheet_id: sheet_id.clone(),
                    row_id: None,
                    path: "$".into(),
                    base: base_sheet.map(|sheet| Value::String(sheet.name.clone())),
                    ours: Some(Value::String(ours.name.clone())),
                    theirs: Some(Value::String(theirs.name.clone())),
                    reason: "sheet added differently on both sides".into(),
                });
                Some(ours.clone())
            }
            (Some(base), Some(ours), None) => {
                conflicts.push(sheet_delete_conflict(base, Some(ours), None));
                Some(ours.clone())
            }
            (Some(base), None, Some(theirs)) => {
                conflicts.push(sheet_delete_conflict(base, None, Some(theirs)));
                Some(theirs.clone())
            }
            (None, None, None) => None,
        };
        if let Some(sheet) = merged {
            if base_sheet.is_none() {
                validate_added_sheet_metadata(&sheet, ours_sheet, theirs_sheet, &mut conflicts);
            }
            sheets.insert(sheet_id, sheet);
        }
    }

    MergeReport {
        merged: ProjectSnapshot { sheets },
        conflicts,
        recalculation_required,
    }
}

fn merge_sheet(
    base: &SnapshotSheet,
    ours: &SnapshotSheet,
    theirs: &SnapshotSheet,
    conflicts: &mut Vec<MergeConflict>,
    recalculation_required: &mut bool,
) -> SnapshotSheet {
    if base.kind != ours.kind || base.kind != theirs.kind {
        conflicts.push(MergeConflict {
            sheet_id: base.id.clone(),
            row_id: None,
            path: "$kind".into(),
            base: Some(Value::String(format!("{:?}", base.kind))),
            ours: Some(Value::String(format!("{:?}", ours.kind))),
            theirs: Some(Value::String(format!("{:?}", theirs.kind))),
            reason: "sheet kind changes require an explicit delete and add".into(),
        });
        return ours.clone();
    }

    let name = merge_required_metadata(
        &base.name,
        &ours.name,
        &theirs.name,
        "$name",
        &base.id,
        conflicts,
    );
    let definition = merge_optional_metadata(
        base.definition.as_deref(),
        ours.definition.as_deref(),
        theirs.definition.as_deref(),
        "$definition",
        &base.id,
        conflicts,
    );
    let source_path = merge_optional_metadata(
        base.source_path.as_deref(),
        ours.source_path.as_deref(),
        theirs.source_path.as_deref(),
        "$source_path",
        &base.id,
        conflicts,
    );

    let base_rows = row_map(&base.rows);
    let ours_rows = row_map(&ours.rows);
    let theirs_rows = row_map(&theirs.rows);
    let ids = base_rows
        .keys()
        .chain(ours_rows.keys())
        .chain(theirs_rows.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut merged_by_id = BTreeMap::new();

    for id in ids {
        match (base_rows.get(&id), ours_rows.get(&id), theirs_rows.get(&id)) {
            (None, Some(ours), None) => {
                merged_by_id.insert(id, (*ours).clone());
            }
            (None, None, Some(theirs)) => {
                merged_by_id.insert(id, (*theirs).clone());
            }
            (None, Some(ours), Some(theirs)) if ours == theirs => {
                merged_by_id.insert(id, (*ours).clone());
            }
            (None, Some(ours), Some(theirs)) => {
                conflicts.push(row_conflict(
                    base,
                    &id,
                    None,
                    Some(&ours.value),
                    Some(&theirs.value),
                    "same row ID added differently",
                ));
                merged_by_id.insert(id, (*ours).clone());
            }
            (Some(_base), None, None) => {}
            (Some(base_row), Some(ours), None) if ours == base_row => {}
            (Some(base_row), None, Some(theirs)) if theirs == base_row => {}
            (Some(base_row), Some(ours), None) => {
                conflicts.push(row_conflict(
                    base,
                    &id,
                    Some(&base_row.value),
                    Some(&ours.value),
                    None,
                    "row deleted on theirs and edited on ours",
                ));
                merged_by_id.insert(id, (*ours).clone());
            }
            (Some(base_row), None, Some(theirs)) => {
                conflicts.push(row_conflict(
                    base,
                    &id,
                    Some(&base_row.value),
                    None,
                    Some(&theirs.value),
                    "row deleted on ours and edited on theirs",
                ));
                merged_by_id.insert(id, (*theirs).clone());
            }
            (Some(base_row), Some(ours), Some(theirs)) => {
                let merged_value = merge_value(
                    Some(&base_row.value),
                    Some(&ours.value),
                    Some(&theirs.value),
                    "",
                    &base.id,
                    Some(&id),
                    conflicts,
                )
                .unwrap_or_else(|| ours.value.clone());
                merged_by_id.insert(
                    id.clone(),
                    RowRecord {
                        id,
                        value: merged_value,
                    },
                );
            }
            (None, None, None) => unreachable!(),
        }
    }

    let base_order = row_order(base);
    let ours_order = row_order(ours);
    let theirs_order = row_order(theirs);
    let order = if ours_order == base_order {
        theirs_order
    } else if theirs_order == base_order || ours_order == theirs_order {
        ours_order
    } else {
        conflicts.push(MergeConflict {
            sheet_id: base.id.clone(),
            row_id: None,
            path: "$order".into(),
            base: Some(serde_json::json!(base_order)),
            ours: Some(serde_json::json!(ours_order)),
            theirs: Some(serde_json::json!(theirs_order)),
            reason: "row order changed differently on both sides".into(),
        });
        ours_order
    };
    let mut rows = Vec::new();
    for id in order {
        if let Some(row) = merged_by_id.remove(&id) {
            rows.push(row);
        }
    }
    rows.extend(merged_by_id.into_values());

    if base.kind == crate::model::SheetKind::Calculation {
        let base_formulas = calculation_formulas(base);
        let ours_formulas = calculation_formulas(ours);
        let theirs_formulas = calculation_formulas(theirs);
        let calculation_formula_changed =
            ours_formulas != base_formulas || theirs_formulas != base_formulas;
        if calculation_formula_changed {
            *recalculation_required = true;
            if let Some(Value::Object(document)) = rows.first_mut().map(|row| &mut row.value) {
                document.insert("needs_recalculation".into(), Value::Bool(true));
            }
        }
    }

    let base_formulas = serde_json::to_value(&base.formulas).unwrap_or(Value::Null);
    let ours_formulas = serde_json::to_value(&ours.formulas).unwrap_or(Value::Null);
    let theirs_formulas = serde_json::to_value(&theirs.formulas).unwrap_or(Value::Null);
    let merged_formulas = merge_value(
        Some(&base_formulas),
        Some(&ours_formulas),
        Some(&theirs_formulas),
        "$formula",
        &base.id,
        None,
        conflicts,
    )
    .unwrap_or(ours_formulas.clone());
    let formula_changed = merged_formulas != base_formulas;
    if formula_changed {
        *recalculation_required = true;
    }
    let mut formulas =
        serde_json::from_value(merged_formulas).unwrap_or_else(|_| ours.formulas.clone());
    formulas.needs_recalculation = formula_changed;

    let format = merge_value(
        Some(&base.format),
        Some(&ours.format),
        Some(&theirs.format),
        "$format",
        &base.id,
        None,
        conflicts,
    )
    .unwrap_or_else(|| ours.format.clone());

    SnapshotSheet {
        id: base.id.clone(),
        name,
        kind: ours.kind,
        definition,
        source_path,
        rows,
        formulas,
        format,
    }
}

fn calculation_formulas(sheet: &SnapshotSheet) -> BTreeMap<String, Value> {
    let Some(document) = sheet.rows.first() else {
        return BTreeMap::new();
    };
    let Ok(document) =
        serde_json::from_value::<crate::model::CalculationDocument>(document.value.clone())
    else {
        return BTreeMap::new();
    };
    document
        .cells
        .into_iter()
        .filter_map(|(cell_id, cell)| {
            cell.formula.map(|formula| {
                let binding = document.formula_bindings.get(&cell_id);
                (
                    cell_id,
                    serde_json::json!({
                        "formula": formula,
                        "binding": binding,
                    }),
                )
            })
        })
        .collect()
}

fn merge_required_metadata(
    base: &str,
    ours: &str,
    theirs: &str,
    path: &str,
    sheet_id: &str,
    conflicts: &mut Vec<MergeConflict>,
) -> String {
    merge_optional_metadata(
        Some(base),
        Some(ours),
        Some(theirs),
        path,
        sheet_id,
        conflicts,
    )
    .unwrap_or_else(|| ours.to_string())
}

fn merge_optional_metadata(
    base: Option<&str>,
    ours: Option<&str>,
    theirs: Option<&str>,
    path: &str,
    sheet_id: &str,
    conflicts: &mut Vec<MergeConflict>,
) -> Option<String> {
    if ours == theirs {
        return ours.map(str::to_string);
    }
    if ours == base {
        return theirs.map(str::to_string);
    }
    if theirs == base {
        return ours.map(str::to_string);
    }
    conflicts.push(MergeConflict {
        sheet_id: sheet_id.to_string(),
        row_id: None,
        path: path.to_string(),
        base: base.map(|value| Value::String(value.to_string())),
        ours: ours.map(|value| Value::String(value.to_string())),
        theirs: theirs.map(|value| Value::String(value.to_string())),
        reason: "sheet metadata changed differently on both sides".into(),
    });
    ours.map(str::to_string)
}

fn validate_added_sheet_metadata(
    merged: &SnapshotSheet,
    ours: Option<&SnapshotSheet>,
    theirs: Option<&SnapshotSheet>,
    conflicts: &mut Vec<MergeConflict>,
) {
    let missing_data_metadata = merged.kind == SheetKind::Data
        && (merged.definition.as_deref().is_none_or(str::is_empty)
            || merged.source_path.as_deref().is_none_or(str::is_empty));
    let unexpected_calculation_metadata = merged.kind == SheetKind::Calculation
        && (merged.definition.is_some() || merged.source_path.is_some());
    if !missing_data_metadata && !unexpected_calculation_metadata {
        return;
    }

    conflicts.push(MergeConflict {
        sheet_id: merged.id.clone(),
        row_id: None,
        path: "$metadata".into(),
        base: None,
        ours: ours.map(sheet_metadata_value),
        theirs: theirs.map(sheet_metadata_value),
        reason: if missing_data_metadata {
            "added data sheet is missing its definition or source path"
        } else {
            "added calculation sheet contains data-sheet metadata"
        }
        .into(),
    });
}

fn sheet_metadata_value(sheet: &SnapshotSheet) -> Value {
    serde_json::json!({
        "name": sheet.name,
        "kind": sheet.kind,
        "definition": sheet.definition,
        "source_path": sheet.source_path,
    })
}

fn merge_value(
    base: Option<&Value>,
    ours: Option<&Value>,
    theirs: Option<&Value>,
    path: &str,
    sheet_id: &str,
    row_id: Option<&str>,
    conflicts: &mut Vec<MergeConflict>,
) -> Option<Value> {
    if ours == theirs {
        return ours.cloned();
    }
    if ours == base {
        return theirs.cloned();
    }
    if theirs == base {
        return ours.cloned();
    }
    if let (Some(Value::Object(base)), Some(Value::Object(ours)), Some(Value::Object(theirs))) =
        (base, ours, theirs)
    {
        let keys = base
            .keys()
            .chain(ours.keys())
            .chain(theirs.keys())
            .cloned()
            .collect::<BTreeSet<_>>();
        let mut merged = Map::new();
        for key in keys {
            let child_path = if path.is_empty() {
                key.clone()
            } else {
                format!("{path}.{key}")
            };
            if let Some(value) = merge_value(
                base.get(&key),
                ours.get(&key),
                theirs.get(&key),
                &child_path,
                sheet_id,
                row_id,
                conflicts,
            ) {
                merged.insert(key, value);
            }
        }
        return Some(Value::Object(merged));
    }
    conflicts.push(MergeConflict {
        sheet_id: sheet_id.to_string(),
        row_id: row_id.map(str::to_string),
        path: if path.is_empty() {
            "$".into()
        } else {
            path.into()
        },
        base: base.cloned(),
        ours: ours.cloned(),
        theirs: theirs.cloned(),
        reason: if matches!(ours, Some(Value::Array(_))) || matches!(theirs, Some(Value::Array(_)))
        {
            "array changed differently on both sides".into()
        } else {
            "field changed differently on both sides".into()
        },
    });
    ours.cloned()
}

fn row_map(rows: &[RowRecord]) -> BTreeMap<String, &RowRecord> {
    rows.iter().map(|row| (row.id.clone(), row)).collect()
}

fn row_order(sheet: &SnapshotSheet) -> Vec<String> {
    sheet.rows.iter().map(|row| row.id.clone()).collect()
}

fn row_conflict(
    sheet: &SnapshotSheet,
    row_id: &str,
    base: Option<&Value>,
    ours: Option<&Value>,
    theirs: Option<&Value>,
    reason: &str,
) -> MergeConflict {
    MergeConflict {
        sheet_id: sheet.id.clone(),
        row_id: Some(row_id.to_string()),
        path: "$".into(),
        base: base.cloned(),
        ours: ours.cloned(),
        theirs: theirs.cloned(),
        reason: reason.into(),
    }
}

fn sheet_delete_conflict(
    base: &SnapshotSheet,
    ours: Option<&SnapshotSheet>,
    theirs: Option<&SnapshotSheet>,
) -> MergeConflict {
    MergeConflict {
        sheet_id: base.id.clone(),
        row_id: None,
        path: "$".into(),
        base: Some(Value::String(base.name.clone())),
        ours: ours.map(|sheet| Value::String(sheet.name.clone())),
        theirs: theirs.map(|sheet| Value::String(sheet.name.clone())),
        reason: "sheet deleted on one side and edited on the other".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::SnapshotSheet;
    use crate::model::{FormulaDocument, SheetKind};

    fn snapshot(value: Value) -> ProjectSnapshot {
        ProjectSnapshot {
            sheets: BTreeMap::from([(
                "sheet".into(),
                SnapshotSheet {
                    id: "sheet".into(),
                    name: "Items".into(),
                    kind: SheetKind::Data,
                    definition: Some("game.Item".into()),
                    source_path: Some("data/items.json".into()),
                    rows: vec![RowRecord {
                        id: "1".into(),
                        value,
                    }],
                    formulas: FormulaDocument::default(),
                    format: Value::Null,
                },
            )]),
        }
    }

    fn calculation_sheet(id: &str, name: &str) -> SnapshotSheet {
        SnapshotSheet {
            id: id.into(),
            name: name.into(),
            kind: SheetKind::Calculation,
            definition: None,
            source_path: None,
            rows: vec![RowRecord {
                id: "calculation".into(),
                value: serde_json::json!({
                    "version": 1,
                    "row_order": ["r"],
                    "column_order": ["c"],
                    "cells": {}
                }),
            }],
            formulas: FormulaDocument::default(),
            format: serde_json::json!({"version": 1}),
        }
    }

    #[test]
    fn merges_independent_field_edits() {
        let base = snapshot(serde_json::json!({"name":"A","count":1}));
        let ours = snapshot(serde_json::json!({"name":"B","count":1}));
        let theirs = snapshot(serde_json::json!({"name":"A","count":2}));
        let report = merge_snapshots(&base, &ours, &theirs);
        assert!(report.conflicts.is_empty());
        assert_eq!(
            report.merged.sheets["sheet"].rows[0].value,
            serde_json::json!({"name":"B","count":2})
        );
    }

    #[test]
    fn reports_same_field_conflict() {
        let base = snapshot(serde_json::json!({"name":"A"}));
        let ours = snapshot(serde_json::json!({"name":"B"}));
        let theirs = snapshot(serde_json::json!({"name":"C"}));
        let report = merge_snapshots(&base, &ours, &theirs);
        assert_eq!(report.conflicts.len(), 1);
        assert_eq!(report.conflicts[0].path, "name");
    }

    #[test]
    fn treats_concurrent_array_changes_as_atomic_conflict() {
        let base = snapshot(serde_json::json!({"tags":["a"]}));
        let ours = snapshot(serde_json::json!({"tags":["a","b"]}));
        let theirs = snapshot(serde_json::json!({"tags":["a","c"]}));
        let report = merge_snapshots(&base, &ours, &theirs);
        assert_eq!(report.conflicts.len(), 1);
        assert_eq!(report.conflicts[0].path, "tags");
        assert!(report.conflicts[0].reason.contains("array"));
    }

    #[test]
    fn reports_delete_versus_edit() {
        let base = snapshot(serde_json::json!({"name":"A"}));
        let mut ours = base.clone();
        ours.sheets.get_mut("sheet").unwrap().rows.clear();
        let theirs = snapshot(serde_json::json!({"name":"B"}));
        let report = merge_snapshots(&base, &ours, &theirs);
        assert_eq!(report.conflicts.len(), 1);
        assert!(report.conflicts[0].reason.contains("deleted"));
    }

    #[test]
    fn merges_theirs_only_added_calculation_sheet() {
        let base = ProjectSnapshot {
            sheets: BTreeMap::new(),
        };
        let ours = base.clone();
        let added = calculation_sheet("calc", "Scratch");
        let theirs = ProjectSnapshot {
            sheets: BTreeMap::from([("calc".into(), added.clone())]),
        };

        let report = merge_snapshots(&base, &ours, &theirs);

        assert!(report.conflicts.is_empty());
        assert_eq!(report.merged.sheets.get("calc"), Some(&added));
    }

    #[test]
    fn merges_theirs_only_added_data_sheet_with_metadata() {
        let base = ProjectSnapshot {
            sheets: BTreeMap::new(),
        };
        let ours = base.clone();
        let mut added = snapshot(serde_json::json!({"id": 1, "name": "A"}))
            .sheets
            .remove("sheet")
            .unwrap();
        added.id = "items".into();
        let theirs = ProjectSnapshot {
            sheets: BTreeMap::from([("items".into(), added.clone())]),
        };

        let report = merge_snapshots(&base, &ours, &theirs);

        assert!(report.conflicts.is_empty());
        assert_eq!(report.merged.sheets.get("items"), Some(&added));
    }

    #[test]
    fn reports_added_data_sheet_without_metadata() {
        let base = ProjectSnapshot {
            sheets: BTreeMap::new(),
        };
        let ours = base.clone();
        let mut added = snapshot(serde_json::json!({"id": 1, "name": "A"}))
            .sheets
            .remove("sheet")
            .unwrap();
        added.definition = None;
        added.source_path = None;
        let theirs = ProjectSnapshot {
            sheets: BTreeMap::from([("sheet".into(), added)]),
        };

        let report = merge_snapshots(&base, &ours, &theirs);

        assert_eq!(report.conflicts.len(), 1);
        assert_eq!(report.conflicts[0].path, "$metadata");
        assert!(report.conflicts[0].reason.contains("definition or source"));
    }

    #[test]
    fn applies_one_sided_sheet_deletion_to_result() {
        let base = snapshot(serde_json::json!({"id": 1, "name": "A"}));
        let ours = base.clone();
        let theirs = ProjectSnapshot {
            sheets: BTreeMap::new(),
        };

        let report = merge_snapshots(&base, &ours, &theirs);

        assert!(report.conflicts.is_empty());
        assert!(report.merged.sheets.is_empty());
    }

    #[test]
    fn reports_sheet_delete_versus_other_side_edit() {
        let base = snapshot(serde_json::json!({"id": 1, "name": "A"}));
        let ours = snapshot(serde_json::json!({"id": 1, "name": "B"}));
        let theirs = ProjectSnapshot {
            sheets: BTreeMap::new(),
        };

        let report = merge_snapshots(&base, &ours, &theirs);

        assert_eq!(report.conflicts.len(), 1);
        assert!(report.conflicts[0].reason.contains("sheet deleted"));
        assert_eq!(report.merged, ours);
    }

    #[test]
    fn reports_differently_added_sheet_on_both_sides() {
        let base = ProjectSnapshot {
            sheets: BTreeMap::new(),
        };
        let ours = ProjectSnapshot {
            sheets: BTreeMap::from([("calc".into(), calculation_sheet("calc", "Ours"))]),
        };
        let theirs = ProjectSnapshot {
            sheets: BTreeMap::from([("calc".into(), calculation_sheet("calc", "Theirs"))]),
        };

        let report = merge_snapshots(&base, &ours, &theirs);

        assert_eq!(report.conflicts.len(), 1);
        assert!(report.conflicts[0].reason.contains("added differently"));
    }

    #[test]
    fn merges_one_sided_sheet_metadata_change() {
        let base = snapshot(serde_json::json!({"id": 1, "name": "A"}));
        let ours = base.clone();
        let mut theirs = base.clone();
        theirs.sheets.get_mut("sheet").unwrap().name = "Products".into();

        let report = merge_snapshots(&base, &ours, &theirs);

        assert!(report.conflicts.is_empty());
        assert_eq!(report.merged.sheets["sheet"].name, "Products");
    }

    #[test]
    fn reports_two_sided_sheet_metadata_change() {
        let base = snapshot(serde_json::json!({"id": 1, "name": "A"}));
        let mut ours = base.clone();
        ours.sheets.get_mut("sheet").unwrap().name = "Ours".into();
        let mut theirs = base.clone();
        theirs.sheets.get_mut("sheet").unwrap().name = "Theirs".into();

        let report = merge_snapshots(&base, &ours, &theirs);

        assert_eq!(report.conflicts.len(), 1);
        assert_eq!(report.conflicts[0].path, "$name");
        assert!(report.conflicts[0].reason.contains("metadata"));
    }

    #[test]
    fn formula_change_requires_recalculation() {
        let base = snapshot(serde_json::json!({"count":1}));
        let mut ours = base.clone();
        ours.sheets.get_mut("sheet").unwrap().formulas = FormulaDocument {
            version: 1,
            rows: BTreeMap::from([(
                "1".into(),
                BTreeMap::from([("count".into(), "=1+1".into())]),
            )]),
            ..FormulaDocument::default()
        };
        let report = merge_snapshots(&base, &ours, &base);
        assert!(report.conflicts.is_empty());
        assert!(report.recalculation_required);
        assert!(report.merged.sheets["sheet"].formulas.needs_recalculation);
    }

    #[test]
    fn calculation_formula_change_requires_recalculation() {
        let calculation = |formula: &str| ProjectSnapshot {
            sheets: BTreeMap::from([(
                "calc".into(),
                SnapshotSheet {
                    id: "calc".into(),
                    name: "Scratch".into(),
                    kind: SheetKind::Calculation,
                    definition: None,
                    source_path: None,
                    rows: vec![RowRecord {
                        id: "calculation".into(),
                        value: serde_json::json!({
                            "version": 1,
                            "row_order": ["r"],
                            "column_order": ["c"],
                            "cells": {"r:c": {"value": 2, "formula": formula}}
                        }),
                    }],
                    formulas: FormulaDocument::default(),
                    format: Value::Null,
                },
            )]),
        };
        let base = calculation("=1+1");
        let ours = calculation("=1+2");
        let report = merge_snapshots(&base, &ours, &base);
        assert!(report.recalculation_required);
        assert_eq!(
            report.merged.sheets["calc"].rows[0].value["needs_recalculation"],
            true
        );
    }
}
