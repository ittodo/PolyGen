use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// Current on-disk PolySheet document format.
pub const FORMAT_VERSION: u32 = 1;

/// Root manifest stored as `workbook.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkbookManifest {
    pub format_version: u32,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    pub sources: Option<String>,
    pub data_root: Option<String>,
    #[serde(default)]
    pub sheets: Vec<String>,
}

/// Per-sheet manifest stored as `sheets/<id>/sheet.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SheetManifest {
    pub id: String,
    pub name: String,
    pub kind: SheetKind,
    pub definition: Option<String>,
    #[serde(default)]
    pub row_count: usize,
    #[serde(default)]
    pub column_count: usize,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SheetKind {
    Data,
    Calculation,
}

/// Formula sidecar. Data-sheet keys are stable row IDs and field paths.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct FormulaDocument {
    pub version: u32,
    #[serde(default)]
    pub rows: BTreeMap<String, BTreeMap<String, String>>,
    #[serde(default)]
    pub cells: BTreeMap<String, String>,
    #[serde(default)]
    pub needs_recalculation: bool,
    /// Stable reference bindings keyed by row ID and field path. Formula text
    /// remains human-readable while these bindings preserve target identity
    /// across row and column insertion.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub bindings: BTreeMap<String, BTreeMap<String, FormulaBinding>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FormulaBinding {
    pub source: String,
    #[serde(default)]
    pub references: Vec<FormulaReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FormulaReference {
    pub start: usize,
    pub end: usize,
    pub sheet_id: String,
    pub row_id: String,
    pub column_id: String,
    #[serde(default)]
    pub absolute_row: bool,
    #[serde(default)]
    pub absolute_column: bool,
    #[serde(default)]
    pub explicit_sheet: bool,
}

/// View and sparse formatting sidecar.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct FormatDocument {
    pub version: u32,
    #[serde(default)]
    pub frozen_rows: usize,
    #[serde(default)]
    pub frozen_columns: usize,
    #[serde(default)]
    pub column_widths: BTreeMap<String, f64>,
    #[serde(default)]
    pub cells: BTreeMap<String, Value>,
}

/// Stable identity sidecar used when a table has no primary key.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RowIdDocument {
    pub version: u32,
    pub source_hash: String,
    #[serde(default)]
    pub rows: Vec<RowIdEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RowIdEntry {
    pub id: String,
    pub fingerprint: String,
}

/// Sparse calculation-sheet storage.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CalculationDocument {
    pub version: u32,
    #[serde(default)]
    pub needs_recalculation: bool,
    #[serde(default)]
    pub row_order: Vec<String>,
    #[serde(default)]
    pub column_order: Vec<String>,
    #[serde(default)]
    pub cells: BTreeMap<String, CalculationCell>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub formula_bindings: BTreeMap<String, FormulaBinding>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct CalculationCell {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formula: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RowRecord {
    pub id: String,
    pub value: Value,
}

/// Schema-derived description used by the contextual data inspector.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InspectorField {
    pub path: String,
    pub name: String,
    pub type_name: String,
    pub required: bool,
    pub is_primary_key: bool,
    pub is_optional: bool,
    pub is_list: bool,
    pub is_enum: bool,
    pub is_struct: bool,
    #[serde(default)]
    pub enum_values: Vec<String>,
    pub foreign_key_target: Option<String>,
    pub foreign_key_field: Option<String>,
    pub default_value: Option<String>,
    pub max_length: Option<u32>,
    pub range_min: Option<String>,
    pub range_max: Option<String>,
    pub regex_pattern: Option<String>,
    pub input_example: Value,
}

/// Initial JSON and metadata for adding a row.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RowDraft {
    pub json: String,
    pub missing_required: Vec<String>,
    pub fields: Vec<InspectorField>,
    pub editable: bool,
    pub readonly_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RowComparisonValue {
    pub row_id: String,
    pub present: bool,
    pub value: Value,
    pub formula: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RowComparisonField {
    pub path: String,
    pub type_name: String,
    pub is_list: bool,
    pub all_equal: bool,
    pub values: Vec<RowComparisonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RowComparison {
    pub row_ids: Vec<String>,
    pub fields: Vec<RowComparisonField>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DataSheetDocument {
    pub manifest: SheetManifest,
    pub source_path: String,
    pub primary_key: Option<String>,
    pub rows: Vec<RowRecord>,
    pub formulas: FormulaDocument,
    pub format: FormatDocument,
    pub row_ids: Option<RowIdDocument>,
    #[serde(default)]
    pub identity_issues: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CalculationSheetDocument {
    pub manifest: SheetManifest,
    pub document: CalculationDocument,
    pub format: FormatDocument,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SheetDocument {
    Data(DataSheetDocument),
    Calculation(CalculationSheetDocument),
}
