//! Source map for line-level template-to-output mapping.
//!
//! Each generated output line gets a [`SourceMapEntry`] recording which template
//! file and line produced it, the full include stack, and the IR path being
//! processed at that point.

use serde::Serialize;

/// A complete source map for one generated output file.
#[derive(Debug, Clone, Default, Serialize)]
pub struct SourceMap {
    /// One entry per output line (0-indexed).
    pub entries: Vec<SourceMapEntry>,
}

/// Maps a single output line back to its template origin.
#[derive(Debug, Clone, Serialize)]
pub struct SourceMapEntry {
    /// Template file that produced this line (e.g. `"detail/field_declaration.ptpl"`).
    pub template_file: String,
    /// 1-based line number in the template file.
    pub template_line: usize,
    /// Full include chain leading to this line.
    /// Example: `["file/main_file.ptpl", "section/namespace_block.ptpl", "detail/field_declaration.ptpl"]`
    pub include_stack: Vec<String>,
    /// The IR node path being processed (e.g. `"game.character.Player.hp"`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ir_path: Option<String>,
}

impl SourceMap {
    /// Creates an empty source map.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an entry for the next output line.
    pub fn push(&mut self, entry: SourceMapEntry) {
        self.entries.push(entry);
    }

    /// Returns the number of mapped output lines.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns true if the source map has no entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Serializes the source map to pretty-printed JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_map_basic() {
        let mut map = SourceMap::new();
        assert!(map.is_empty());

        map.push(SourceMapEntry {
            template_file: "detail/field.ptpl".to_string(),
            template_line: 3,
            include_stack: vec![
                "file/main.ptpl".to_string(),
                "detail/field.ptpl".to_string(),
            ],
            ir_path: Some("game.Player.hp".to_string()),
        });

        assert_eq!(map.len(), 1);
        assert!(!map.is_empty());

        let json = map.to_json().unwrap();
        assert!(json.contains("detail/field.ptpl"));
        assert!(json.contains("game.Player.hp"));
    }

    #[test]
    fn test_source_map_no_ir_path() {
        let mut map = SourceMap::new();
        map.push(SourceMapEntry {
            template_file: "_header.ptpl".to_string(),
            template_line: 1,
            include_stack: vec!["_header.ptpl".to_string()],
            ir_path: None,
        });

        let json = map.to_json().unwrap();
        // ir_path should be omitted when None
        assert!(!json.contains("ir_path"));
    }
}
