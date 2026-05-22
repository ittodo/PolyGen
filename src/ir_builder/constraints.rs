use crate::ast_model;
use crate::ir_model::{self, ForeignKeyDef};

use super::type_names::qualify;

/// Information extracted from field constraints.
#[derive(Debug, Default)]
pub(super) struct ConstraintInfo {
    pub(super) is_primary_key: bool,
    pub(super) is_unique: bool,
    pub(super) is_index: bool,
    pub(super) foreign_key: Option<ForeignKeyDef>,
    pub(super) max_length: Option<u32>,
    pub(super) default_value: Option<String>,
    pub(super) range: Option<ir_model::RangeDef>,
    pub(super) regex_pattern: Option<String>,
    pub(super) auto_create: Option<ir_model::TimezoneRef>,
    pub(super) auto_update: Option<ir_model::TimezoneRef>,
}

/// Extracts structured constraint information from AST constraints.
/// `current_ns` is used to fully qualify FK target table names that lack a namespace prefix.
pub(super) fn extract_constraint_info(
    constraints: &[ast_model::Constraint],
    current_ns: &str,
) -> ConstraintInfo {
    let mut info = ConstraintInfo::default();

    for constraint in constraints {
        match constraint {
            ast_model::Constraint::PrimaryKey => info.is_primary_key = true,
            ast_model::Constraint::Unique => info.is_unique = true,
            ast_model::Constraint::ForeignKey(path, alias) => {
                // Parse the path: e.g., ["game", "character", "Player", "id"]
                // The last element is the field, everything before is the table FQN
                if let (Some(target_field), true) = (path.last().cloned(), path.len() >= 2) {
                    let raw_table_fqn = path[..path.len() - 1].join(".");
                    let target_table_fqn = qualify(&raw_table_fqn, current_ns);
                    info.foreign_key = Some(ForeignKeyDef {
                        target_table_fqn,
                        target_field,
                        alias: alias.clone(),
                    });
                    // FK fields get a GroupIndex automatically
                    info.is_index = true;
                }
            }
            ast_model::Constraint::Index => info.is_index = true,
            ast_model::Constraint::MaxLength(len) => info.max_length = Some(*len),
            ast_model::Constraint::Default(lit) => {
                info.default_value = Some(literal_to_string(lit));
            }
            ast_model::Constraint::Range(min, max) => {
                let literal_type = match min {
                    ast_model::Literal::Integer(_) => "integer",
                    ast_model::Literal::Float(_) => "float",
                    _ => "integer", // Default to integer for other types
                };
                info.range = Some(ir_model::RangeDef {
                    min: literal_to_string(min),
                    max: literal_to_string(max),
                    literal_type: literal_type.to_string(),
                });
            }
            ast_model::Constraint::Regex(pattern) => {
                info.regex_pattern = Some(pattern.clone());
            }
            ast_model::Constraint::AutoCreate(tz) => {
                info.auto_create = Some(timezone_to_ref(tz.as_ref()));
            }
            ast_model::Constraint::AutoUpdate(tz) => {
                info.auto_update = Some(timezone_to_ref(tz.as_ref()));
            }
        }
    }

    info
}

/// Converts field constraints from the AST into a vector of strings
/// suitable for C# attributes.
pub(super) fn convert_constraints_to_attributes(
    constraints: &[ast_model::Constraint],
) -> Vec<String> {
    constraints
        .iter()
        .filter_map(|c| match c {
            // `primary_key` is mapped to the `[Key]` attribute, common in ORMs like EF Core.
            ast_model::Constraint::PrimaryKey => Some("Key".to_string()),
            // `unique` can be mapped to an index attribute.
            ast_model::Constraint::Unique => Some("Index(IsUnique = true)".to_string()),
            ast_model::Constraint::MaxLength(len) => Some(format!("MaxLength({})", len)),
            // ForeignKey is a relationship, not a simple attribute, so we ignore it here.
            ast_model::Constraint::ForeignKey(_, _) => None,
            // Other constraints are not (yet) represented as attributes.
            _ => None,
        })
        .collect()
}

/// Converts an AST Timezone to an IR TimezoneRef.
fn timezone_to_ref(tz: Option<&ast_model::Timezone>) -> ir_model::TimezoneRef {
    match tz {
        None => ir_model::TimezoneRef {
            kind: "utc".to_string(), // Default to UTC when not specified
            offset_hours: None,
            offset_minutes: None,
            name: None,
        },
        Some(ast_model::Timezone::Utc) => ir_model::TimezoneRef {
            kind: "utc".to_string(),
            offset_hours: None,
            offset_minutes: None,
            name: None,
        },
        Some(ast_model::Timezone::Local) => ir_model::TimezoneRef {
            kind: "local".to_string(),
            offset_hours: None,
            offset_minutes: None,
            name: None,
        },
        Some(ast_model::Timezone::Offset { hours, minutes }) => ir_model::TimezoneRef {
            kind: "offset".to_string(),
            offset_hours: Some(*hours),
            offset_minutes: Some(*minutes),
            name: None,
        },
        Some(ast_model::Timezone::Named(name)) => ir_model::TimezoneRef {
            kind: "named".to_string(),
            offset_hours: None,
            offset_minutes: None,
            name: Some(name.clone()),
        },
    }
}

/// Converts an AST Literal to a string representation.
fn literal_to_string(lit: &ast_model::Literal) -> String {
    match lit {
        ast_model::Literal::String(s) => format!("\"{}\"", s),
        ast_model::Literal::Integer(i) => i.to_string(),
        ast_model::Literal::Float(f) => f.to_string(),
        ast_model::Literal::Boolean(b) => b.to_string(),
        ast_model::Literal::Identifier(id) => id.clone(),
    }
}
