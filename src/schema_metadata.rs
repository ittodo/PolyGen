//! Schema metadata helpers for generated database artifacts.
//!
//! The metadata is intentionally derived from the serialized IR, not source text,
//! so formatting-only changes in `.poly` files do not change the hash.

use crate::ir_model::SchemaContext;

/// Metadata stored in `_polygen_schema`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaMetadata {
    /// Stable non-cryptographic hash of the canonical schema JSON.
    pub hash: String,
    /// Canonical compact JSON representation of the schema IR.
    pub json: String,
}

/// Build schema metadata from the IR.
pub fn build_schema_metadata(schema: &SchemaContext) -> Result<SchemaMetadata, serde_json::Error> {
    let json = serde_json::to_string(schema)?;
    let hash = stable_fnv1a64_hex(json.as_bytes());
    Ok(SchemaMetadata { hash, json })
}

/// Convert a string into a single-quoted SQL literal.
pub fn sql_string_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn stable_fnv1a64_hex(bytes: &[u8]) -> String {
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x00000100000001b3;

    let mut hash = FNV_OFFSET_BASIS;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    format!("{:016x}", hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sql_string_literal_escapes_quotes() {
        assert_eq!(sql_string_literal("a'b"), "'a''b'");
    }

    #[test]
    fn test_schema_metadata_is_stable_for_same_schema() {
        let schema = SchemaContext::default();

        let first = build_schema_metadata(&schema).expect("metadata should serialize");
        let second = build_schema_metadata(&schema).expect("metadata should serialize");

        assert_eq!(first, second);
        assert_eq!(first.json, r#"{"files":[]}"#);
        assert_eq!(first.hash.len(), 16);
    }
}
