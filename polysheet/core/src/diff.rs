use crate::model::{FormulaDocument, RowRecord, SheetDocument, SheetKind};
use crate::project::PolySheetProject;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectSnapshot {
    pub sheets: BTreeMap<String, SnapshotSheet>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SnapshotSheet {
    pub id: String,
    pub name: String,
    pub kind: SheetKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub definition: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_path: Option<String>,
    pub rows: Vec<RowRecord>,
    pub formulas: FormulaDocument,
    pub format: Value,
}

impl ProjectSnapshot {
    pub fn from_project(project: &PolySheetProject) -> Self {
        let sheets = project
            .sheets
            .iter()
            .map(|(id, sheet)| {
                let snapshot = match sheet {
                    SheetDocument::Data(sheet) => SnapshotSheet {
                        id: id.clone(),
                        name: sheet.manifest.name.clone(),
                        kind: SheetKind::Data,
                        definition: sheet.manifest.definition.clone(),
                        source_path: Some(sheet.source_path.clone()),
                        rows: sheet.rows.clone(),
                        formulas: sheet.formulas.clone(),
                        format: serde_json::to_value(&sheet.format).unwrap_or(Value::Null),
                    },
                    SheetDocument::Calculation(sheet) => SnapshotSheet {
                        id: id.clone(),
                        name: sheet.manifest.name.clone(),
                        kind: SheetKind::Calculation,
                        definition: None,
                        source_path: None,
                        rows: vec![RowRecord {
                            id: "calculation".to_string(),
                            value: serde_json::to_value(&sheet.document).unwrap_or(Value::Null),
                        }],
                        formulas: FormulaDocument::default(),
                        format: serde_json::to_value(&sheet.format).unwrap_or(Value::Null),
                    },
                };
                (id.clone(), snapshot)
            })
            .collect();
        Self { sheets }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    SheetAdded,
    SheetDeleted,
    RowAdded,
    RowDeleted,
    RowMoved,
    ValueChanged,
    FormulaChanged,
    FormatChanged,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DiffEntry {
    pub kind: ChangeKind,
    pub sheet_id: String,
    pub sheet_name: String,
    pub row_id: Option<String>,
    pub path: String,
    pub before: Option<Value>,
    pub after: Option<Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct DiffReport {
    pub changes: Vec<DiffEntry>,
}

pub fn diff_snapshots(base: &ProjectSnapshot, target: &ProjectSnapshot) -> DiffReport {
    let mut changes = Vec::new();
    let sheet_ids = base
        .sheets
        .keys()
        .chain(target.sheets.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    for sheet_id in sheet_ids {
        match (base.sheets.get(&sheet_id), target.sheets.get(&sheet_id)) {
            (None, Some(sheet)) => changes.push(DiffEntry {
                kind: ChangeKind::SheetAdded,
                sheet_id: sheet_id.clone(),
                sheet_name: sheet.name.clone(),
                row_id: None,
                path: "$".to_string(),
                before: None,
                after: Some(Value::String(sheet.name.clone())),
            }),
            (Some(sheet), None) => changes.push(DiffEntry {
                kind: ChangeKind::SheetDeleted,
                sheet_id: sheet_id.clone(),
                sheet_name: sheet.name.clone(),
                row_id: None,
                path: "$".to_string(),
                before: Some(Value::String(sheet.name.clone())),
                after: None,
            }),
            (Some(base_sheet), Some(target_sheet)) => {
                diff_sheet(base_sheet, target_sheet, &mut changes);
            }
            (None, None) => unreachable!(),
        }
    }
    DiffReport { changes }
}

fn diff_sheet(base: &SnapshotSheet, target: &SnapshotSheet, changes: &mut Vec<DiffEntry>) {
    if base.name != target.name {
        push_change(
            changes,
            ChangeKind::ValueChanged,
            target,
            None,
            "$name",
            Some(Value::String(base.name.clone())),
            Some(Value::String(target.name.clone())),
        );
    }
    if base.kind != target.kind {
        push_change(
            changes,
            ChangeKind::ValueChanged,
            target,
            None,
            "$kind",
            serde_json::to_value(base.kind).ok(),
            serde_json::to_value(target.kind).ok(),
        );
    }
    if base.definition != target.definition {
        push_change(
            changes,
            ChangeKind::ValueChanged,
            target,
            None,
            "$definition",
            base.definition.clone().map(Value::String),
            target.definition.clone().map(Value::String),
        );
    }
    if base.source_path != target.source_path {
        push_change(
            changes,
            ChangeKind::ValueChanged,
            target,
            None,
            "$source_path",
            base.source_path.clone().map(Value::String),
            target.source_path.clone().map(Value::String),
        );
    }

    let base_rows = base
        .rows
        .iter()
        .map(|row| (row.id.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    let target_rows = target
        .rows
        .iter()
        .map(|row| (row.id.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    let row_ids = base_rows
        .keys()
        .chain(target_rows.keys())
        .copied()
        .collect::<BTreeSet<_>>();
    for row_id in row_ids {
        match (base_rows.get(row_id), target_rows.get(row_id)) {
            (None, Some(row)) => push_change(
                changes,
                ChangeKind::RowAdded,
                target,
                Some(row_id),
                "$",
                None,
                Some(row.value.clone()),
            ),
            (Some(row), None) => push_change(
                changes,
                ChangeKind::RowDeleted,
                target,
                Some(row_id),
                "$",
                Some(row.value.clone()),
                None,
            ),
            (Some(before), Some(after)) => {
                diff_value(&before.value, &after.value, "", target, row_id, changes)
            }
            (None, None) => unreachable!(),
        }
    }
    let base_order = base.rows.iter().map(|row| &row.id).collect::<Vec<_>>();
    let target_order = target.rows.iter().map(|row| &row.id).collect::<Vec<_>>();
    if base_order != target_order {
        for (index, row_id) in target_order.iter().enumerate() {
            if let Some(before_index) = base_order.iter().position(|candidate| candidate == row_id)
            {
                if before_index != index {
                    push_change(
                        changes,
                        ChangeKind::RowMoved,
                        target,
                        Some(row_id),
                        "$order",
                        Some(Value::from(before_index)),
                        Some(Value::from(index)),
                    );
                }
            }
        }
    }

    let base_formulas = serde_json::to_value(&base.formulas).unwrap_or(Value::Null);
    let target_formulas = serde_json::to_value(&target.formulas).unwrap_or(Value::Null);
    diff_sidecar(
        &base_formulas,
        &target_formulas,
        "$formula",
        ChangeKind::FormulaChanged,
        target,
        changes,
    );
    diff_sidecar(
        &base.format,
        &target.format,
        "$format",
        ChangeKind::FormatChanged,
        target,
        changes,
    );
}

fn diff_value(
    before: &Value,
    after: &Value,
    path: &str,
    sheet: &SnapshotSheet,
    row_id: &str,
    changes: &mut Vec<DiffEntry>,
) {
    if before == after {
        return;
    }
    match (before, after) {
        (Value::Object(before), Value::Object(after)) => {
            let keys = before
                .keys()
                .chain(after.keys())
                .cloned()
                .collect::<BTreeSet<_>>();
            for key in keys {
                let next_path = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{path}.{key}")
                };
                match (before.get(&key), after.get(&key)) {
                    (Some(left), Some(right)) => {
                        diff_value(left, right, &next_path, sheet, row_id, changes)
                    }
                    (left, right) => push_change(
                        changes,
                        value_change_kind(sheet, &next_path),
                        sheet,
                        Some(row_id),
                        &next_path,
                        left.cloned(),
                        right.cloned(),
                    ),
                }
            }
        }
        // Lists are atomic in format v1.
        _ => push_change(
            changes,
            value_change_kind(sheet, path),
            sheet,
            Some(row_id),
            if path.is_empty() { "$" } else { path },
            Some(before.clone()),
            Some(after.clone()),
        ),
    }
}

fn value_change_kind(sheet: &SnapshotSheet, path: &str) -> ChangeKind {
    if sheet.kind == SheetKind::Calculation
        && (path.ends_with(".formula") || path == "formula" || path.contains("formula_bindings"))
    {
        ChangeKind::FormulaChanged
    } else {
        ChangeKind::ValueChanged
    }
}

fn diff_sidecar(
    before: &Value,
    after: &Value,
    path: &str,
    kind: ChangeKind,
    sheet: &SnapshotSheet,
    changes: &mut Vec<DiffEntry>,
) {
    if before != after {
        push_change(
            changes,
            kind,
            sheet,
            None,
            path,
            Some(before.clone()),
            Some(after.clone()),
        );
    }
}

fn push_change(
    changes: &mut Vec<DiffEntry>,
    kind: ChangeKind,
    sheet: &SnapshotSheet,
    row_id: Option<&str>,
    path: &str,
    before: Option<Value>,
    after: Option<Value>,
) {
    changes.push(DiffEntry {
        kind,
        sheet_id: sheet.id.clone(),
        sheet_name: sheet.name.clone(),
        row_id: row_id.map(str::to_string),
        path: path.to_string(),
        before,
        after,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(name: &str, rows: Vec<RowRecord>) -> ProjectSnapshot {
        ProjectSnapshot {
            sheets: BTreeMap::from([(
                "sheet".into(),
                SnapshotSheet {
                    id: "sheet".into(),
                    name: name.into(),
                    kind: SheetKind::Data,
                    definition: Some("game.Item".into()),
                    source_path: Some("data/items.json".into()),
                    rows,
                    formulas: FormulaDocument::default(),
                    format: Value::Null,
                },
            )]),
        }
    }

    #[test]
    fn reports_field_change_without_replacing_row() {
        let base = snapshot(
            "Items",
            vec![RowRecord {
                id: "1".into(),
                value: serde_json::json!({"id": 1, "name": "A"}),
            }],
        );
        let target = snapshot(
            "Items",
            vec![RowRecord {
                id: "1".into(),
                value: serde_json::json!({"id": 1, "name": "B"}),
            }],
        );
        let report = diff_snapshots(&base, &target);
        assert_eq!(report.changes.len(), 1);
        assert_eq!(report.changes[0].path, "name");
    }

    #[test]
    fn reports_calculation_formula_separately() {
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
        let report = diff_snapshots(&calculation("=1+1"), &calculation("=1+2"));
        assert!(report
            .changes
            .iter()
            .any(|change| change.kind == ChangeKind::FormulaChanged));
    }

    #[test]
    fn reports_sheet_metadata_changes() {
        let base = snapshot(
            "Items",
            vec![RowRecord {
                id: "1".into(),
                value: serde_json::json!({"id": 1}),
            }],
        );
        let mut target = base.clone();
        let target_sheet = target.sheets.get_mut("sheet").unwrap();
        target_sheet.name = "Products".into();
        target_sheet.definition = Some("game.Product".into());
        target_sheet.source_path = Some("data/products.json".into());

        let report = diff_snapshots(&base, &target);

        assert_eq!(report.changes.len(), 3);
        assert_eq!(
            report
                .changes
                .iter()
                .map(|change| change.path.as_str())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["$definition", "$name", "$source_path"])
        );
    }
}
