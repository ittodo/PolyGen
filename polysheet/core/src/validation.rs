use base64::Engine;
use chrono::DateTime;
use polygen::ir_model::{
    EnumDef, EnumItem, FieldDef, IndexDef, NamespaceDef, NamespaceItem, SchemaContext, StructDef,
    StructItem, TypeRef,
};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: DiagnosticSeverity,
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Default)]
pub struct SchemaIndex {
    pub tables: BTreeMap<String, StructDef>,
    pub structs: BTreeMap<String, StructDef>,
    pub enums: BTreeMap<String, EnumDef>,
}

impl SchemaIndex {
    pub fn from_schema(schema: &SchemaContext) -> Self {
        let mut index = Self {
            tables: BTreeMap::new(),
            structs: BTreeMap::new(),
            enums: BTreeMap::new(),
        };
        for file in &schema.files {
            for namespace in &file.namespaces {
                collect_namespace(namespace, &mut index);
            }
        }
        index
    }

    pub fn table(&self, fqn: &str) -> Option<&StructDef> {
        self.tables.get(fqn)
    }
}

fn collect_namespace(namespace: &NamespaceDef, index: &mut SchemaIndex) {
    for item in &namespace.items {
        match item {
            NamespaceItem::Struct(definition) => collect_struct(definition, index),
            NamespaceItem::Enum(definition) => {
                index
                    .enums
                    .insert(definition.fqn.clone(), definition.clone());
            }
            NamespaceItem::Namespace(child) => collect_namespace(child, index),
            NamespaceItem::Comment(_) => {}
        }
    }
}

fn collect_struct(definition: &StructDef, index: &mut SchemaIndex) {
    index
        .structs
        .insert(definition.fqn.clone(), definition.clone());
    if !definition.is_embed {
        index
            .tables
            .insert(definition.fqn.clone(), definition.clone());
    }
    for item in &definition.items {
        match item {
            StructItem::EmbeddedStruct(child) => collect_struct(child, index),
            StructItem::InlineEnum(definition) => {
                index
                    .enums
                    .insert(definition.fqn.clone(), definition.clone());
            }
            _ => {}
        }
    }
}

pub fn table_fields(table: &StructDef) -> Vec<&FieldDef> {
    table
        .items
        .iter()
        .filter_map(|item| match item {
            StructItem::Field(field) => Some(field.as_ref()),
            _ => None,
        })
        .collect()
}

pub fn primary_key(table: &StructDef) -> Option<&FieldDef> {
    table_fields(table)
        .into_iter()
        .find(|field| field.is_primary_key)
}

pub fn validate_table_rows(
    table: &StructDef,
    rows: &[Value],
    index: &SchemaIndex,
) -> Vec<Diagnostic> {
    let mut diagnostics = Vec::new();
    let mut primary_values = BTreeSet::new();
    for (row_index, row) in rows.iter().enumerate() {
        let path = format!("{}[{}]", table.fqn, row_index);
        validate_struct_value(table, row, &path, index, &mut diagnostics);
        if let Some(primary_key) = primary_key(table) {
            if let Some(value) = row.get(&primary_key.name) {
                let canonical = canonical_scalar(value);
                if !primary_values.insert(canonical) {
                    diagnostics.push(Diagnostic {
                        severity: DiagnosticSeverity::Error,
                        path: format!("{path}.{}", primary_key.name),
                        message: "duplicate primary key".to_string(),
                    });
                }
            }
        }
    }
    validate_unique_indexes(table, rows, index, &mut diagnostics);
    diagnostics
}

fn validate_unique_indexes(
    table: &StructDef,
    rows: &[Value],
    schema_index: &SchemaIndex,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for unique_index in table.indexes.iter().filter(|index| index.is_unique) {
        if unique_index.fields.is_empty() || is_primary_key_index(table, unique_index) {
            continue;
        }
        let mut seen = BTreeSet::new();
        for (row_index, row) in rows.iter().enumerate() {
            let Some(object) = row.as_object() else {
                continue;
            };
            let mut key = Vec::with_capacity(unique_index.fields.len());
            let mut has_null_or_missing_component = false;
            for field in &unique_index.fields {
                let Some(value) = object.get(&field.name) else {
                    has_null_or_missing_component = true;
                    break;
                };
                if value.is_null() {
                    has_null_or_missing_component = true;
                    break;
                }
                key.push(canonical_index_value(
                    value,
                    &field.field_type,
                    schema_index,
                ));
            }
            // PolySheet follows the generated Kotlin/Swift and SQL container policy:
            // an optional/null component does not participate in a unique index.
            if has_null_or_missing_component || key.len() != unique_index.fields.len() {
                continue;
            }
            if !seen.insert(key) {
                let row_path = format!("{}[{}]", table.fqn, row_index);
                let path = if unique_index.fields.len() == 1 {
                    format!("{row_path}.{}", unique_index.fields[0].name)
                } else {
                    row_path
                };
                let fields = unique_index
                    .fields
                    .iter()
                    .map(|field| field.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ");
                diagnostics.push(error(
                    &path,
                    &format!("duplicate unique index '{}' ({fields})", unique_index.name),
                ));
            }
        }
    }
}

fn is_primary_key_index(table: &StructDef, index: &IndexDef) -> bool {
    index.fields.len() == 1
        && table_fields(table)
            .into_iter()
            .any(|field| field.is_primary_key && field.name == index.fields[0].name)
}

fn canonical_index_value(value: &Value, type_ref: &TypeRef, schema_index: &SchemaIndex) -> String {
    let type_ref = if type_ref.is_option {
        type_ref.inner_type.as_deref().unwrap_or(type_ref)
    } else {
        type_ref
    };
    if type_ref.is_enum {
        if let Some(definition) = schema_index.enums.get(&type_ref.fqn) {
            if let Some(member) = definition.items.iter().find_map(|item| match item {
                EnumItem::Member(member)
                    if value.as_str() == Some(member.name.as_str())
                        || value
                            .as_i64()
                            .is_some_and(|number| member.value == Some(number)) =>
                {
                    Some(member)
                }
                _ => None,
            }) {
                return format!("enum:{}:{}", definition.fqn, member.name);
            }
        }
    }
    match type_ref.type_name.as_str() {
        "i8" | "i16" | "i32" | "i64" => value
            .as_i64()
            .map(i128::from)
            .or_else(|| value.as_str().and_then(|text| text.parse::<i128>().ok()))
            .map(|number| format!("integer:{number}"))
            .unwrap_or_else(|| canonical_scalar(value)),
        "u8" | "u16" | "u32" | "u64" => value
            .as_u64()
            .map(u128::from)
            .or_else(|| value.as_str().and_then(|text| text.parse::<u128>().ok()))
            .map(|number| format!("integer:{number}"))
            .unwrap_or_else(|| canonical_scalar(value)),
        "f32" => value
            .as_f64()
            .map(|number| number as f32)
            .map(|number| {
                let normalized = if number == 0.0 { 0.0 } else { number };
                format!("f32:{:08x}", normalized.to_bits())
            })
            .unwrap_or_else(|| canonical_scalar(value)),
        "f64" => value
            .as_f64()
            .map(|number| {
                let normalized = if number == 0.0 { 0.0 } else { number };
                format!("f64:{:016x}", normalized.to_bits())
            })
            .unwrap_or_else(|| canonical_scalar(value)),
        "timestamp" => value
            .as_str()
            .and_then(|text| DateTime::parse_from_rfc3339(text).ok())
            .map(|timestamp| {
                format!(
                    "timestamp:{}:{}",
                    timestamp.timestamp(),
                    timestamp.timestamp_subsec_nanos()
                )
            })
            .unwrap_or_else(|| canonical_scalar(value)),
        _ => canonical_scalar(value),
    }
}

fn validate_struct_value(
    definition: &StructDef,
    value: &Value,
    path: &str,
    index: &SchemaIndex,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(object) = value.as_object() else {
        diagnostics.push(error(path, "expected JSON object"));
        return;
    };
    let fields = table_fields(definition);
    let known: BTreeSet<&str> = fields.iter().map(|field| field.name.as_str()).collect();
    for key in object.keys() {
        if !known.contains(key.as_str()) {
            diagnostics.push(error(&format!("{path}.{key}"), "unknown field"));
        }
    }
    for field in fields {
        let field_path = format!("{path}.{}", field.name);
        match object.get(&field.name) {
            Some(field_value) => {
                validate_type(
                    &field.field_type,
                    field_value,
                    &field_path,
                    index,
                    diagnostics,
                );
                validate_constraints(field, field_value, &field_path, diagnostics);
            }
            None if !field.field_type.is_option => {
                diagnostics.push(error(&field_path, "missing required field"));
            }
            None => {}
        }
    }
}

fn validate_type(
    type_ref: &TypeRef,
    value: &Value,
    path: &str,
    index: &SchemaIndex,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if type_ref.is_option {
        if value.is_null() {
            return;
        }
        if let Some(inner) = type_ref.inner_type.as_deref() {
            validate_type(inner, value, path, index, diagnostics);
        }
        return;
    }
    if value.is_null() {
        diagnostics.push(error(path, "null is not allowed"));
        return;
    }
    if type_ref.is_list {
        let Some(items) = value.as_array() else {
            diagnostics.push(error(path, "expected JSON array"));
            return;
        };
        if let Some(inner) = type_ref.inner_type.as_deref() {
            for (item_index, item) in items.iter().enumerate() {
                validate_type(
                    inner,
                    item,
                    &format!("{path}[{item_index}]"),
                    index,
                    diagnostics,
                );
            }
        }
        return;
    }
    if type_ref.is_enum {
        validate_enum(type_ref, value, path, index, diagnostics);
        return;
    }
    if type_ref.is_struct {
        match index.structs.get(&type_ref.fqn) {
            Some(definition) => validate_struct_value(definition, value, path, index, diagnostics),
            None => diagnostics.push(error(path, "unresolved struct type")),
        }
        return;
    }
    if type_ref.is_primitive {
        validate_primitive(&type_ref.type_name, value, path, diagnostics);
    }
}

fn validate_enum(
    type_ref: &TypeRef,
    value: &Value,
    path: &str,
    index: &SchemaIndex,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let Some(definition) = index.enums.get(&type_ref.fqn) else {
        diagnostics.push(error(path, "unresolved enum type"));
        return;
    };
    let valid = definition.items.iter().any(|item| match item {
        EnumItem::Member(member) => {
            value.as_str() == Some(member.name.as_str())
                || value
                    .as_i64()
                    .is_some_and(|number| member.value == Some(number))
        }
        EnumItem::Comment(_) => false,
    });
    if !valid {
        diagnostics.push(error(path, "unknown enum value"));
    }
}

fn validate_primitive(
    primitive: &str,
    value: &Value,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let valid = match primitive {
        "string" => value.is_string(),
        "bytes" => value.as_str().is_some_and(|text| {
            base64::engine::general_purpose::STANDARD
                .decode(text)
                .is_ok()
        }),
        "timestamp" => value
            .as_str()
            .is_some_and(|text| DateTime::parse_from_rfc3339(text).is_ok()),
        "bool" => value.is_boolean(),
        "f32" | "f64" => value.is_number(),
        "i8" => integer_in_range(value, i8::MIN as i128, i8::MAX as i128),
        "i16" => integer_in_range(value, i16::MIN as i128, i16::MAX as i128),
        "i32" => integer_in_range(value, i32::MIN as i128, i32::MAX as i128),
        "i64" => integer_in_range(value, i64::MIN as i128, i64::MAX as i128),
        "u8" => integer_in_range(value, 0, u8::MAX as i128),
        "u16" => integer_in_range(value, 0, u16::MAX as i128),
        "u32" => integer_in_range(value, 0, u32::MAX as i128),
        "u64" => {
            value.as_u64().is_some()
                || value
                    .as_str()
                    .and_then(|text| text.parse::<u64>().ok())
                    .is_some()
        }
        _ => false,
    };
    if !valid {
        diagnostics.push(error(path, &format!("invalid {primitive} value")));
    }
}

fn integer_in_range(value: &Value, minimum: i128, maximum: i128) -> bool {
    let parsed = value
        .as_i64()
        .map(i128::from)
        .or_else(|| value.as_u64().map(i128::from))
        .or_else(|| value.as_str().and_then(|text| text.parse::<i128>().ok()));
    parsed.is_some_and(|number| number >= minimum && number <= maximum)
}

fn validate_constraints(
    field: &FieldDef,
    value: &Value,
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if value.is_null() {
        return;
    }
    if let Some(maximum) = field.max_length {
        if let Some(text) = value.as_str() {
            if text.chars().count() > maximum as usize {
                diagnostics.push(error(path, &format!("exceeds max_length({maximum})")));
            }
        }
    }
    if let Some(range) = &field.range {
        let number = value
            .as_f64()
            .or_else(|| value.as_str().and_then(|text| text.parse::<f64>().ok()));
        let minimum = range.min.parse::<f64>().ok();
        let maximum = range.max.parse::<f64>().ok();
        if let (Some(number), Some(minimum), Some(maximum)) = (number, minimum, maximum) {
            if number < minimum || number > maximum {
                diagnostics.push(error(path, "value is outside the allowed range"));
            }
        }
    }
    if let (Some(pattern), Some(text)) = (&field.regex_pattern, value.as_str()) {
        if Regex::new(pattern)
            .ok()
            .is_some_and(|regex| !regex.is_match(text))
        {
            diagnostics.push(error(path, "value does not match the required pattern"));
        }
    }
}

pub fn canonical_scalar(value: &Value) -> String {
    match value {
        Value::String(text) => format!("string:{text}"),
        Value::Number(number) => format!("number:{number}"),
        Value::Bool(boolean) => format!("bool:{boolean}"),
        Value::Null => "null".to_string(),
        other => format!("json:{other}"),
    }
}

fn error(path: &str, message: &str) -> Diagnostic {
    Diagnostic {
        severity: DiagnosticSeverity::Error,
        path: path.to_string(),
        message: message.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use std::fs;
    use tempfile::tempdir;

    fn validation_fixture() -> Result<(tempfile::TempDir, SchemaIndex)> {
        let temp = tempdir()?;
        let schema_path = temp.path().join("types.poly");
        fs::write(
            &schema_path,
            r#"
namespace game {
    enum Rarity { Common = 1; Rare = 2; }
    embed Metadata {
        note: string?;
        created_at: timestamp;
        payload: bytes;
    }
    table Item {
        id: u32 primary_key;
        rarity: Rarity;
        tags: string[];
        metadata: Metadata;
        signed: i64;
        unsigned: u64;
    }
}
"#,
        )?;
        let loaded = polygen::load_project_schema(&schema_path, None)?;
        Ok((temp, SchemaIndex::from_schema(&loaded.schema)))
    }

    #[test]
    fn validates_enum_optional_embed_list_timestamp_bytes_and_64_bit_integers() -> Result<()> {
        let (_temp, index) = validation_fixture()?;
        let table = index.table("game.Item").unwrap();
        let valid = serde_json::json!([{
            "id": 1,
            "rarity": "Rare",
            "tags": ["weapon", "rare"],
            "metadata": {
                "note": null,
                "created_at": "2026-07-25T10:20:30+09:00",
                "payload": "AQID"
            },
            "signed": "-9223372036854775808",
            "unsigned": "18446744073709551615"
        }]);
        assert!(validate_table_rows(table, valid.as_array().unwrap(), &index).is_empty());

        let invalid = serde_json::json!([{
            "id": 1,
            "rarity": "Legendary",
            "tags": "weapon",
            "metadata": {
                "created_at": "not-a-time",
                "payload": "%%%"
            },
            "signed": "9223372036854775808",
            "unsigned": -1
        }]);
        let diagnostics = validate_table_rows(table, invalid.as_array().unwrap(), &index);
        for expected in [
            "rarity",
            "tags",
            "created_at",
            "payload",
            "signed",
            "unsigned",
        ] {
            assert!(
                diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.path.contains(expected)),
                "missing diagnostic for {expected}: {diagnostics:?}"
            );
        }
        Ok(())
    }

    #[test]
    fn validates_field_and_composite_unique_indexes_with_nulls_excluded() -> Result<()> {
        let temp = tempdir()?;
        let schema_path = temp.path().join("unique.poly");
        fs::write(
            &schema_path,
            r#"
namespace game {
    @index(tenant, external_id, unique: true)
    table Account {
        id: u32 primary_key;
        email: string? unique;
        numeric_code: u64 unique;
        tenant: string?;
        external_id: string;
    }
}
"#,
        )?;
        let loaded = polygen::load_project_schema(&schema_path, None)?;
        let index = SchemaIndex::from_schema(&loaded.schema);
        let table = index.table("game.Account").unwrap();

        let nulls = serde_json::json!([
            {"id": 1, "email": null, "numeric_code": 1, "tenant": null, "external_id": "same"},
            {"id": 2, "numeric_code": 2, "tenant": null, "external_id": "same"},
            {"id": 3, "email": null, "numeric_code": 3, "external_id": "same"}
        ]);
        assert!(validate_table_rows(table, nulls.as_array().unwrap(), &index).is_empty());

        let duplicates = serde_json::json!([
            {"id": 1, "email": "same@example.com", "numeric_code": 7, "tenant": "apac", "external_id": "42"},
            {"id": 2, "email": "same@example.com", "numeric_code": "7", "tenant": "apac", "external_id": "42"}
        ]);
        let diagnostics = validate_table_rows(table, duplicates.as_array().unwrap(), &index);
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.message.starts_with("duplicate unique index"))
                .count(),
            3,
            "unexpected diagnostics: {diagnostics:?}"
        );
        for expected_index in ["ByEmail", "ByNumericCode", "ByTenantExternalId"] {
            assert!(
                diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.message.contains(expected_index)),
                "missing unique diagnostic for {expected_index}: {diagnostics:?}"
            );
        }
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.path == "game.Account[1].email" && diagnostic.message.contains("ByEmail")
        }));
        assert!(diagnostics.iter().any(|diagnostic| {
            diagnostic.path == "game.Account[1]"
                && diagnostic.message.contains("ByTenantExternalId")
        }));
        Ok(())
    }

    #[test]
    fn keeps_primary_key_uniqueness_strict_without_duplicate_index_diagnostics() -> Result<()> {
        let (_temp, index) = validation_fixture()?;
        let table = index.table("game.Item").unwrap();
        let rows = serde_json::json!([
            {
                "id": null,
                "rarity": "Rare",
                "tags": [],
                "metadata": {"created_at": "2026-07-25T10:20:30+09:00", "payload": "AQID"},
                "signed": 1,
                "unsigned": 1
            },
            {
                "id": null,
                "rarity": "Rare",
                "tags": [],
                "metadata": {"created_at": "2026-07-25T10:20:30+09:00", "payload": "AQID"},
                "signed": 2,
                "unsigned": 2
            }
        ]);
        let diagnostics = validate_table_rows(table, rows.as_array().unwrap(), &index);
        assert_eq!(
            diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.message == "duplicate primary key")
                .count(),
            1
        );
        assert!(!diagnostics
            .iter()
            .any(|diagnostic| diagnostic.message.contains("ById")));
        Ok(())
    }
}
