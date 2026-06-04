//! User-facing schema diff reports.

use serde::Serialize;

use crate::migration::{MigrationDiff, SchemaChange};

/// Serializable schema diff summary for CLI text and JSON output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SchemaDiffReport {
    /// Number of detected changes.
    pub change_count: usize,
    /// True when no changes were detected.
    pub is_empty: bool,
    /// Warnings about destructive or manual migration work.
    pub warnings: Vec<String>,
    /// Detected changes in stable display order.
    pub changes: Vec<SchemaDiffChange>,
}

/// Serializable user-facing schema change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SchemaDiffChange {
    /// Change kind.
    pub kind: String,
    /// Namespace containing the changed table.
    pub namespace: String,
    /// Table name.
    pub table_name: String,
    /// Column name, when the change applies to a column.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column_name: Option<String>,
    /// Previous column type, when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub old_type: Option<String>,
    /// New column type, when applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub new_type: Option<String>,
    /// Whether a newly added column is nullable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_nullable: Option<bool>,
}

/// Convert an internal migration diff into a compact CLI report.
pub fn build_diff_report(diff: &MigrationDiff) -> SchemaDiffReport {
    let mut changes: Vec<_> = diff.changes.iter().map(change_to_report).collect();
    changes.sort_by(|a, b| {
        (
            &a.namespace,
            &a.table_name,
            &a.column_name,
            &a.kind,
            &a.old_type,
            &a.new_type,
        )
            .cmp(&(
                &b.namespace,
                &b.table_name,
                &b.column_name,
                &b.kind,
                &b.old_type,
                &b.new_type,
            ))
    });

    SchemaDiffReport {
        change_count: changes.len(),
        is_empty: changes.is_empty(),
        warnings: diff.warnings.clone(),
        changes,
    }
}

/// Render a diff report as stable, human-readable text.
pub fn render_diff_text(report: &SchemaDiffReport) -> String {
    let mut output = String::new();
    output.push_str("Schema Diff\n");
    output.push_str("===========\n\n");
    output.push_str(&format!("Changes: {}\n", report.change_count));
    output.push_str(&format!("Warnings: {}\n", report.warnings.len()));

    if report.changes.is_empty() {
        output.push_str("\nNo changes detected.\n");
    } else {
        output.push_str("\nChanges\n");
        output.push_str("-------\n");
        for change in &report.changes {
            output.push_str(&format!("- {}\n", format_change(change)));
        }
    }

    if !report.warnings.is_empty() {
        output.push_str("\nWarnings\n");
        output.push_str("--------\n");
        for warning in &report.warnings {
            output.push_str(&format!("- {}\n", warning));
        }
    }

    output
}

fn change_to_report(change: &SchemaChange) -> SchemaDiffChange {
    match change {
        SchemaChange::TableAdded {
            table_name,
            namespace,
            ..
        } => SchemaDiffChange {
            kind: "table_added".to_string(),
            namespace: namespace.clone(),
            table_name: table_name.clone(),
            column_name: None,
            old_type: None,
            new_type: None,
            is_nullable: None,
        },
        SchemaChange::TableRemoved {
            table_name,
            namespace,
        } => SchemaDiffChange {
            kind: "table_removed".to_string(),
            namespace: namespace.clone(),
            table_name: table_name.clone(),
            column_name: None,
            old_type: None,
            new_type: None,
            is_nullable: None,
        },
        SchemaChange::ColumnAdded {
            table_name,
            namespace,
            column_name,
            column_type,
            is_nullable,
        } => SchemaDiffChange {
            kind: "column_added".to_string(),
            namespace: namespace.clone(),
            table_name: table_name.clone(),
            column_name: Some(column_name.clone()),
            old_type: None,
            new_type: Some(column_type.clone()),
            is_nullable: Some(*is_nullable),
        },
        SchemaChange::ColumnRemoved {
            table_name,
            namespace,
            column_name,
        } => SchemaDiffChange {
            kind: "column_removed".to_string(),
            namespace: namespace.clone(),
            table_name: table_name.clone(),
            column_name: Some(column_name.clone()),
            old_type: None,
            new_type: None,
            is_nullable: None,
        },
        SchemaChange::ColumnTypeChanged {
            table_name,
            namespace,
            column_name,
            old_type,
            new_type,
        } => SchemaDiffChange {
            kind: "column_type_changed".to_string(),
            namespace: namespace.clone(),
            table_name: table_name.clone(),
            column_name: Some(column_name.clone()),
            old_type: Some(old_type.clone()),
            new_type: Some(new_type.clone()),
            is_nullable: None,
        },
    }
}

fn format_change(change: &SchemaDiffChange) -> String {
    let table = if change.namespace.is_empty() {
        change.table_name.clone()
    } else {
        format!("{}.{}", change.namespace, change.table_name)
    };

    match change.kind.as_str() {
        "table_added" => format!("table added: {}", table),
        "table_removed" => format!("table removed: {}", table),
        "column_added" => format!(
            "column added: {}.{} {}{}",
            table,
            change.column_name.as_deref().unwrap_or(""),
            change.new_type.as_deref().unwrap_or(""),
            if change.is_nullable == Some(true) {
                " nullable"
            } else {
                " not_null"
            }
        ),
        "column_removed" => format!(
            "column removed: {}.{}",
            table,
            change.column_name.as_deref().unwrap_or("")
        ),
        "column_type_changed" => format!(
            "column type changed: {}.{} {} -> {}",
            table,
            change.column_name.as_deref().unwrap_or(""),
            change.old_type.as_deref().unwrap_or(""),
            change.new_type.as_deref().unwrap_or("")
        ),
        _ => format!("{}: {}", change.kind, table),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migration::SchemaChange;

    #[test]
    fn test_build_diff_report_sorts_changes() {
        let diff = MigrationDiff {
            changes: vec![
                SchemaChange::ColumnAdded {
                    namespace: "game".to_string(),
                    table_name: "Player".to_string(),
                    column_name: "level".to_string(),
                    column_type: "INTEGER".to_string(),
                    is_nullable: false,
                },
                SchemaChange::TableAdded {
                    namespace: "game".to_string(),
                    table_name: "Account".to_string(),
                    struct_def: crate::ir_model::StructDef {
                        name: "Account".to_string(),
                        fqn: "game.Account".to_string(),
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
                    },
                },
            ],
            warnings: vec!["warning".to_string()],
        };

        let report = build_diff_report(&diff);

        assert_eq!(report.change_count, 2);
        assert_eq!(report.warnings.len(), 1);
        assert_eq!(report.changes[0].table_name, "Account");
        assert_eq!(report.changes[1].column_name.as_deref(), Some("level"));
    }

    #[test]
    fn test_render_diff_text_includes_changes_and_warnings() {
        let report = SchemaDiffReport {
            change_count: 1,
            is_empty: false,
            warnings: vec!["destructive".to_string()],
            changes: vec![SchemaDiffChange {
                kind: "column_type_changed".to_string(),
                namespace: "game".to_string(),
                table_name: "Player".to_string(),
                column_name: Some("level".to_string()),
                old_type: Some("INTEGER".to_string()),
                new_type: Some("TEXT".to_string()),
                is_nullable: None,
            }],
        };

        let text = render_diff_text(&report);

        assert!(text.contains("Schema Diff"));
        assert!(text.contains("column type changed"));
        assert!(text.contains("destructive"));
    }
}
