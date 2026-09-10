//! Public project-schema loading API for editor and tooling integrations.
//!
//! This module exposes the same parse, validation, IR, lint, and external
//! sources configuration pipeline used by code generation without producing
//! generated files.

use anyhow::{Context, Result};
use serde::Serialize;
use std::path::{Path, PathBuf};

use crate::ast_model::Definition;
use crate::ir_model::SchemaContext;
use crate::schema_lint::SchemaLintReport;
use crate::{ir_builder, pipeline, schema_lint, sources_config, validation};

/// A fully loaded PolyGen project schema suitable for editor integrations.
#[derive(Debug, Clone, Serialize)]
pub struct LoadedProjectSchema {
    /// Canonical entry schema path.
    pub schema_path: PathBuf,
    /// Canonical sources configuration path, when one was loaded.
    pub sources_path: Option<PathBuf>,
    /// Validated schema IR with external source paths applied.
    pub schema: SchemaContext,
    /// Non-fatal schema lint diagnostics.
    pub lint: SchemaLintReport,
}

/// Parse, validate, lint, and build a schema graph, then apply sources config.
///
/// `sources_path` overrides the conventional `<schema>.sources.toml` sidecar.
/// No output files are created.
pub fn load_project_schema(
    schema_path: impl AsRef<Path>,
    sources_path: Option<&Path>,
) -> Result<LoadedProjectSchema> {
    let requested_schema_path = schema_path.as_ref();
    let canonical_schema_path = requested_schema_path.canonicalize().with_context(|| {
        format!(
            "failed to resolve schema path: {}",
            requested_schema_path.display()
        )
    })?;
    let asts = pipeline::parse_and_merge_schemas_quiet(requested_schema_path, None)?;
    let definitions: Vec<Definition> = asts
        .iter()
        .flat_map(|ast| ast.definitions.clone())
        .collect();
    validation::validate_ast(&definitions)?;

    let lint = schema_lint::lint_asts(&asts);
    let mut schema = ir_builder::build_ir(&asts);
    let loaded_sources = sources_config::load_sources_config(requested_schema_path, sources_path)?;
    let resolved_sources_path = if let Some((path, config)) = loaded_sources {
        sources_config::apply_sources_config(&mut schema, &config)?;
        Some(path.canonicalize().with_context(|| {
            format!("failed to resolve sources config path: {}", path.display())
        })?)
    } else {
        None
    };

    Ok(LoadedProjectSchema {
        schema_path: canonical_schema_path,
        sources_path: resolved_sources_path,
        schema,
        lint,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema_lint::SchemaLintWarning;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn loads_validated_schema_with_default_sources() -> Result<()> {
        let temp = tempdir()?;
        let schema_path = temp.path().join("game.poly");
        let sources_path = temp.path().join("game.sources.toml");
        fs::write(
            &schema_path,
            "namespace game { table Item { id: u32 primary_key; name: string; } }",
        )?;
        fs::write(
            &sources_path,
            r#"[tables."game.Item".load]
json = "data/items.json"
"#,
        )?;

        let loaded = load_project_schema(&schema_path, None)?;
        assert_eq!(loaded.schema_path, schema_path.canonicalize()?);
        assert_eq!(
            loaded.sources_path.as_deref(),
            Some(sources_path.canonicalize()?.as_path())
        );

        let serialized = serde_json::to_value(&loaded.schema)?;
        assert!(serialized.to_string().contains("data/items.json"));
        Ok(())
    }

    #[test]
    fn explicit_sources_override_the_default_sidecar() -> Result<()> {
        let temp = tempdir()?;
        let schema_path = temp.path().join("game.poly");
        let default_sources_path = temp.path().join("game.sources.toml");
        let explicit_sources_path = temp.path().join("editor.sources.toml");
        fs::write(
            &schema_path,
            "namespace game { table Item { id: u32 primary_key; name: string; } }",
        )?;
        fs::write(
            &default_sources_path,
            r#"[tables."game.Item".load]
json = "data/default-items.json"
"#,
        )?;
        fs::write(
            &explicit_sources_path,
            r#"[tables."game.Item".load]
json = "data/editor-items.json"
"#,
        )?;

        let loaded = load_project_schema(&schema_path, Some(&explicit_sources_path))?;
        assert_eq!(
            loaded.sources_path.as_deref(),
            Some(explicit_sources_path.canonicalize()?.as_path())
        );
        let serialized = serde_json::to_string(&loaded.schema)?;
        assert!(serialized.contains("data/editor-items.json"));
        assert!(!serialized.contains("data/default-items.json"));
        Ok(())
    }

    #[test]
    fn loads_imports_and_renames_and_returns_lint_warnings() -> Result<()> {
        let temp = tempdir()?;
        let schema_path = temp.path().join("game.poly");
        fs::write(
            &schema_path,
            r#"
import "used.poly";
import "unused.poly";
import "game.renames";

namespace game {
    table Item {
        id: u32 primary_key;
        stats: game.common.Stats;
    }
}
"#,
        )?;
        fs::write(
            temp.path().join("used.poly"),
            "namespace game.common { embed Stats { value: u32; } }",
        )?;
        fs::write(
            temp.path().join("unused.poly"),
            "namespace game.common { embed Unused { value: u32; } }",
        )?;
        fs::write(temp.path().join("game.renames"), "game.OldItem -> Item;\n")?;

        let loaded = load_project_schema(&schema_path, None)?;
        assert_eq!(loaded.schema.files.len(), 3);
        assert_eq!(
            loaded
                .schema
                .files
                .iter()
                .map(|file| file.renames.len())
                .sum::<usize>(),
            1
        );
        assert!(loaded
            .lint
            .warnings
            .iter()
            .any(|warning| matches!(warning, SchemaLintWarning::UnusedFileImport { .. })));
        Ok(())
    }

    #[test]
    fn rejects_sources_for_an_unknown_table() -> Result<()> {
        let temp = tempdir()?;
        let schema_path = temp.path().join("game.poly");
        let sources_path = temp.path().join("game.sources.toml");
        fs::write(
            &schema_path,
            "namespace game { table Item { id: u32 primary_key; } }",
        )?;
        fs::write(
            &sources_path,
            r#"[tables."game.Missing".load]
json = "data/missing.json"
"#,
        )?;

        let error = load_project_schema(&schema_path, None).unwrap_err();
        assert!(error
            .to_string()
            .contains("sources config references unknown table"));
        Ok(())
    }

    #[test]
    fn missing_schema_error_includes_the_requested_path() {
        let missing = PathBuf::from("missing-project-schema.poly");
        let error = load_project_schema(&missing, None).unwrap_err();
        assert!(error.to_string().contains(&missing.display().to_string()));
    }

    #[cfg(unix)]
    #[test]
    fn default_sources_are_resolved_next_to_the_requested_symlink() -> Result<()> {
        use std::os::unix::fs::symlink;

        let temp = tempdir()?;
        let real_dir = temp.path().join("real");
        let link_dir = temp.path().join("linked");
        fs::create_dir_all(&real_dir)?;
        fs::create_dir_all(&link_dir)?;
        let real_schema = real_dir.join("game.poly");
        let linked_schema = link_dir.join("game.poly");
        let linked_sources = link_dir.join("game.sources.toml");
        fs::write(
            &real_schema,
            "namespace game { table Item { id: u32 primary_key; } }",
        )?;
        symlink(&real_schema, &linked_schema)?;
        fs::write(
            &linked_sources,
            r#"[tables."game.Item".load]
json = "data/items.json"
"#,
        )?;

        let loaded = load_project_schema(&linked_schema, None)?;
        assert_eq!(loaded.schema_path, real_schema.canonicalize()?);
        assert_eq!(
            loaded.sources_path.as_deref(),
            Some(linked_sources.canonicalize()?.as_path())
        );
        Ok(())
    }

    #[test]
    fn rejects_invalid_schema_before_building_ir() -> Result<()> {
        let temp = tempdir()?;
        let schema_path = temp.path().join("invalid.poly");
        fs::write(
            &schema_path,
            "namespace game { table Item { missing: UnknownType; } }",
        )?;

        let error = load_project_schema(&schema_path, None).unwrap_err();
        assert!(error.to_string().contains("UnknownType"));
        Ok(())
    }
}
