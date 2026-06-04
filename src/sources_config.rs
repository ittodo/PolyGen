//! External source configuration for runtime data-loading concerns.
//!
//! `.poly` files describe schema shape. `*.sources.toml` files describe runtime
//! inputs such as CSV/JSON load paths.

use anyhow::{anyhow, Context, Result};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::ir_model::{LoadSourceDef, NamespaceDef, NamespaceItem, SchemaContext, StructDef};

/// External source configuration loaded from a TOML file.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct SourcesConfig {
    /// Table FQN -> load configuration.
    pub table_loads: BTreeMap<String, LoadSourceDef>,
}

/// Returns the conventional sources config path for a schema.
///
/// `schema.poly` resolves to `schema.sources.toml`.
pub fn default_sources_path(schema_path: &Path) -> PathBuf {
    let mut path = schema_path.to_path_buf();
    let stem = schema_path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "schema".to_string());
    path.set_file_name(format!("{stem}.sources.toml"));
    path
}

/// Loads a sources config from an explicit path or the conventional sidecar path.
pub fn load_sources_config(
    schema_path: &Path,
    explicit_path: Option<&Path>,
) -> Result<Option<(PathBuf, SourcesConfig)>> {
    let path = explicit_path
        .map(Path::to_path_buf)
        .unwrap_or_else(|| default_sources_path(schema_path));

    if !path.exists() {
        if explicit_path.is_some() {
            return Err(anyhow!("sources config not found: {}", path.display()));
        }
        return Ok(None);
    }

    let content = fs::read_to_string(&path)
        .with_context(|| format!("failed to read sources config: {}", path.display()))?;
    let config = parse_sources_config(&content)
        .with_context(|| format!("failed to parse sources config: {}", path.display()))?;
    Ok(Some((path, config)))
}

/// Parses sources TOML.
///
/// Supported shape:
///
/// ```toml
/// [tables."game.Item".load]
/// csv = "data/items.csv"
/// json = "data/items.json"
/// ```
pub fn parse_sources_config(content: &str) -> Result<SourcesConfig> {
    let value: toml::Value = toml::from_str(content)?;
    let root = value
        .as_table()
        .ok_or_else(|| anyhow!("sources config root must be a TOML table"))?;

    let mut config = SourcesConfig::default();
    let Some(tables_value) = root.get("tables") else {
        return Ok(config);
    };
    let tables = tables_value
        .as_table()
        .ok_or_else(|| anyhow!("'tables' must be a TOML table"))?;

    for (fqn, table_value) in tables {
        let table = table_value
            .as_table()
            .ok_or_else(|| anyhow!("tables.{fqn} must be a TOML table"))?;
        let Some(load_value) = table.get("load") else {
            continue;
        };
        let load = parse_load_source(fqn, load_value)?;
        if config.table_loads.insert(fqn.clone(), load).is_some() {
            return Err(anyhow!("duplicate load config for table '{fqn}'"));
        }
    }

    Ok(config)
}

fn parse_load_source(fqn: &str, value: &toml::Value) -> Result<LoadSourceDef> {
    let table = value
        .as_table()
        .ok_or_else(|| anyhow!("tables.{fqn}.load must be a TOML table"))?;

    let csv = optional_non_empty_string(table, "csv", fqn)?;
    let json = optional_non_empty_string(table, "json", fqn)?;

    if csv.is_none() && json.is_none() {
        return Err(anyhow!(
            "tables.{fqn}.load requires at least one of 'csv' or 'json'"
        ));
    }

    let allowed = ["csv", "json"];
    for key in table.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(anyhow!(
                "unsupported tables.{fqn}.load key '{key}'; expected 'csv' or 'json'"
            ));
        }
    }

    Ok(LoadSourceDef { csv, json })
}

fn optional_non_empty_string(
    table: &toml::map::Map<String, toml::Value>,
    key: &str,
    fqn: &str,
) -> Result<Option<String>> {
    let Some(value) = table.get(key) else {
        return Ok(None);
    };
    let value = value
        .as_str()
        .ok_or_else(|| anyhow!("tables.{fqn}.load.{key} must be a string"))?;
    if value.is_empty() {
        return Err(anyhow!("tables.{fqn}.load.{key} must not be empty"));
    }
    Ok(Some(value.to_string()))
}

/// Applies a sources config to the IR, overriding legacy `@load` values.
pub fn apply_sources_config(context: &mut SchemaContext, config: &SourcesConfig) -> Result<()> {
    if config.table_loads.is_empty() {
        return Ok(());
    }

    let mut remaining: BTreeSet<String> = config.table_loads.keys().cloned().collect();
    for file in &mut context.files {
        for namespace in &mut file.namespaces {
            apply_to_namespace(namespace, config, &mut remaining)?;
        }
    }

    if !remaining.is_empty() {
        return Err(anyhow!(
            "sources config references unknown table(s): {}",
            remaining.into_iter().collect::<Vec<_>>().join(", ")
        ));
    }

    Ok(())
}

fn apply_to_namespace(
    namespace: &mut NamespaceDef,
    config: &SourcesConfig,
    remaining: &mut BTreeSet<String>,
) -> Result<()> {
    for item in &mut namespace.items {
        match item {
            NamespaceItem::Struct(struct_def) => apply_to_struct(struct_def, config, remaining)?,
            NamespaceItem::Namespace(child) => apply_to_namespace(child, config, remaining)?,
            _ => {}
        }
    }
    Ok(())
}

fn apply_to_struct(
    struct_def: &mut StructDef,
    config: &SourcesConfig,
    remaining: &mut BTreeSet<String>,
) -> Result<()> {
    if let Some(load) = config.table_loads.get(&struct_def.fqn) {
        if struct_def.is_embed {
            return Err(anyhow!(
                "sources config cannot assign load settings to embed '{}'",
                struct_def.fqn
            ));
        }
        struct_def.load = Some(load.clone());
        remaining.remove(&struct_def.fqn);
    }

    for item in &mut struct_def.items {
        if let crate::ir_model::StructItem::EmbeddedStruct(child) = item {
            apply_to_struct(child, config, remaining)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir_model::{FileDef, NamespaceDef, NamespaceItem};

    #[test]
    fn parses_table_loads() {
        let config = parse_sources_config(
            r#"
            [tables."game.Item".load]
            csv = "data/items.csv"
            json = "data/items.json"
            "#,
        )
        .expect("sources config should parse");

        assert_eq!(
            config.table_loads.get("game.Item"),
            Some(&LoadSourceDef {
                csv: Some("data/items.csv".to_string()),
                json: Some("data/items.json".to_string()),
            })
        );
    }

    #[test]
    fn rejects_empty_load() {
        let err = parse_sources_config(
            r#"
            [tables."game.Item".load]
            "#,
        )
        .expect_err("empty load should fail");

        assert!(err.to_string().contains("requires at least one"));
    }

    #[test]
    fn applies_load_to_matching_table() {
        let mut context = SchemaContext {
            files: vec![FileDef {
                path: "schema.poly".to_string(),
                namespaces: vec![NamespaceDef {
                    name: "game".to_string(),
                    datasource: None,
                    items: vec![NamespaceItem::Struct(StructDef {
                        name: "Item".to_string(),
                        fqn: "game.Item".to_string(),
                        is_embed: false,
                        datasource: None,
                        cache_strategy: None,
                        load: None,
                        is_readonly: false,
                        soft_delete_field: None,
                        pack_separator: None,
                        header: Vec::new(),
                        items: Vec::new(),
                        indexes: Vec::new(),
                        relations: Vec::new(),
                    })],
                }],
                renames: Vec::new(),
            }],
        };
        let config = parse_sources_config(
            r#"
            [tables."game.Item".load]
            csv = "data/items.csv"
            "#,
        )
        .unwrap();

        apply_sources_config(&mut context, &config).unwrap();

        let NamespaceItem::Struct(item) = &context.files[0].namespaces[0].items[0] else {
            panic!("expected struct");
        };
        assert_eq!(
            item.load,
            Some(LoadSourceDef {
                csv: Some("data/items.csv".to_string()),
                json: None,
            })
        );
    }
}
