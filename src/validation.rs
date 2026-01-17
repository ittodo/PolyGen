use crate::ast_model::{Definition, FieldDefinition, TableMember, TypeName};
use crate::error::ValidationError;
use std::collections::HashSet;

/// AST의 유효성을 검사하는 메인 함수입니다.
pub fn validate_ast(definitions: &[Definition]) -> Result<(), ValidationError> {
    let mut defined_types = HashSet::new();
    // 1. 스키마에 정의된 모든 타입의 전체 경로(FQN)를 수집합니다.
    collect_all_types(definitions, &mut Vec::new(), &mut defined_types)?;
    // 2. 사용된 타입들이 실제로 정의되었는지 확인합니다.
    validate_all_types(definitions, &mut Vec::new(), &defined_types)?;
    Ok(())
}

fn collect_from_members(
    members: &[TableMember],
    path: &mut Vec<String>,
    types: &mut HashSet<String>,
) -> Result<(), ValidationError> {
    for member in members {
        match member {
            TableMember::Embed(e) => {
                path.push(e.name.clone().expect("Embed name should be present"));
                let embed_fqn = path.join(".");
                if !types.insert(embed_fqn.clone()) {
                    return Err(ValidationError::DuplicateDefinition(embed_fqn));
                }
                // Recursively collect from the embed's members
                collect_from_members(&e.members, path, types)?;
                path.pop();
            }
            TableMember::Enum(e) => {
                if let Some(name) = &e.name {
                    path.push(name.clone());
                    let enum_fqn = path.join(".");
                    if !types.insert(enum_fqn.clone()) {
                        return Err(ValidationError::DuplicateDefinition(enum_fqn));
                    }
                    path.pop();
                }
            }
            TableMember::Field(FieldDefinition::InlineEmbed(ief)) => {
                path.push(ief.name.clone().expect("Inline embed name should be present"));
                let embed_fqn = path.join(".");
                if !types.insert(embed_fqn.clone()) {
                    return Err(ValidationError::DuplicateDefinition(embed_fqn));
                }
                // Recursively collect from the inline embed's members
                collect_from_members(&ief.members, path, types)?;
                path.pop();
            }
            _ => {} // Other members don't define new types in this context
        }
    }
    Ok(())
}

/// 재귀적으로 모든 타입 정의를 수집하여 `HashSet`에 추가합니다.
fn collect_all_types(
    definitions: &[Definition],
    path: &mut Vec<String>,
    types: &mut HashSet<String>,
) -> Result<(), ValidationError> {
    for def in definitions {
        match def {
            Definition::Namespace(ns) => {
                path.extend(ns.path.iter().cloned());
                collect_all_types(&ns.definitions, path, types)?;
                for _ in 0..ns.path.len() {
                    path.pop();
                }
            }
            Definition::Table(t) => {
                let fqn = path
                    .iter()
                    .chain(std::iter::once(t.name.as_ref().expect("Table name should be present")))
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(".");
                if !types.insert(fqn.clone()) {
                    return Err(ValidationError::DuplicateDefinition(fqn));
                }

                path.push(t.name.as_ref().expect("Table name should be present").clone());
                collect_from_members(&t.members, path, types)?;
                path.pop();
            }
            Definition::Enum(e) => {
                if let Some(name) = &e.name {
                    let fqn = path
                        .iter()
                        .chain(std::iter::once(name))
                        .cloned()
                        .collect::<Vec<_>>()
                        .join(".");
                    if !types.insert(fqn.clone()) {
                        return Err(ValidationError::DuplicateDefinition(fqn));
                    }
                }
            }
            Definition::Embed(e) => {
                let fqn = path
                    .iter()
                    .chain(std::iter::once(e.name.as_ref().expect("Embed name should be present")))
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(".");
                if !types.insert(fqn.clone()) {
                    return Err(ValidationError::DuplicateDefinition(fqn));
                }
                path.push(e.name.as_ref().expect("Embed name should be present").clone());
                collect_from_members(&e.members, path, types)?;
                path.pop();
            }
            Definition::Comment(_) => { /* Comments are not types, so we ignore them. */ }
            Definition::Annotation(_) => { /* Annotations are not types, so we ignore them. */ }
        }
    }
    Ok(())
}

/// Checks if a given type path can be resolved to a known type.
/// It first checks if the path is already a fully-qualified name (FQN).
/// If not, it tries to resolve it relative to the current scope (e.g., `current.namespace.MyType`).
fn check_type_path(
    type_path: &[String],
    current_scope: &[String],
    all_types: &HashSet<String>,
) -> Result<(), ValidationError> {
    let used_type_str = type_path.join(".");
    // 1. Check if the path is already a valid fully-qualified name (FQN).
    if all_types.contains(&used_type_str) {
        return Ok(());
    }

    // 2. If not, try to resolve it by walking up the current scope.
    // For a scope `a.b.c` and a type `T`, it checks:
    // - a.b.c.T
    // - a.b.T
    // - a.T
    // - T
    let mut scope = current_scope.to_vec();
    loop {
        let mut potential_fqn_parts = scope.clone();
        potential_fqn_parts.extend_from_slice(type_path);
        let potential_fqn = potential_fqn_parts.join(".");
        if all_types.contains(&potential_fqn) {
            return Ok(());
        }
        if scope.is_empty() {
            break;
        }
        scope.pop();
    }
    // 3. If no resolution worked, the type is not found.
    Err(ValidationError::TypeNotFound(used_type_str))
}

fn validate_table_members(
    members: &[TableMember],
    path: &mut Vec<String>,
    all_types: &HashSet<String>,
) -> Result<(), ValidationError> {
    for member in members {
        match member {
            TableMember::Field(field) => match field {
                FieldDefinition::Regular(rf) => {
                    if let TypeName::Path(type_path) = &rf.field_type.base_type {
                        check_type_path(type_path, path, all_types)?;
                    }
                }
                FieldDefinition::InlineEmbed(ief) => {
                    path.push(ief.name.clone().expect("Inline embed must have a name"));
                    validate_table_members(&ief.members, path, all_types)?;
                    path.pop();
                }
                FieldDefinition::InlineEnum(_) => {}
            },
            TableMember::Embed(embed) => {
                path.push(embed.name.clone().expect("Embed must have a name"));
                validate_table_members(&embed.members, path, all_types)?;
                path.pop();
            }
            TableMember::Enum(_) => {}
            TableMember::Comment(_) => {}
        }
    }
    Ok(())
}

/// 재귀적으로 모든 필드를 순회하며 사용된 타입의 유효성을 검사합니다.
fn validate_all_types(
    definitions: &[Definition],
    path: &mut Vec<String>,
    all_types: &HashSet<String>,
) -> Result<(), ValidationError> {
    for def in definitions {
        match def {
            Definition::Namespace(ns) => {
                path.extend(ns.path.iter().cloned());
                validate_all_types(&ns.definitions, path, all_types)?;
                for _ in 0..ns.path.len() {
                    path.pop();
                }
            }
            Definition::Table(t) => {
                path.push(t.name.clone().expect("Table name should be present"));
                validate_table_members(&t.members, path, all_types)?;
                path.pop();
            }
            Definition::Enum(_) => { /* Enums do not reference other types */ }
            Definition::Embed(e) => {
                // Validate types used within the embed's fields.
                path.push(e.name.clone().expect("Embed name should be present"));
                validate_table_members(&e.members, path, all_types)?;
                path.pop();
            }
            Definition::Comment(_) => { /* Comments do not reference other types */ }
            Definition::Annotation(_) => { /* Annotations do not reference types */ }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast_model::*;

    /// Helper to create a simple table with a name
    fn make_table(name: &str, members: Vec<TableMember>) -> Definition {
        Definition::Table(Table {
            metadata: vec![],
            name: Some(name.to_string()),
            members,
        })
    }

    /// Helper to create a simple enum with a name
    fn make_enum(name: &str) -> Definition {
        Definition::Enum(Enum {
            metadata: vec![],
            name: Some(name.to_string()),
            variants: vec![],
        })
    }

    /// Helper to create a simple embed with a name
    fn make_embed(name: &str, members: Vec<TableMember>) -> Definition {
        Definition::Embed(Embed {
            metadata: vec![],
            name: Some(name.to_string()),
            members,
        })
    }

    /// Helper to create a namespace
    fn make_namespace(path: Vec<&str>, definitions: Vec<Definition>) -> Definition {
        Definition::Namespace(Namespace {
            path: path.into_iter().map(String::from).collect(),
            imports: vec![],
            definitions,
        })
    }

    /// Helper to create a regular field with a path type
    fn make_field_with_type(name: &str, type_path: Vec<&str>) -> TableMember {
        TableMember::Field(FieldDefinition::Regular(RegularField {
            metadata: vec![],
            name: Some(name.to_string()),
            field_type: TypeWithCardinality {
                base_type: TypeName::Path(type_path.into_iter().map(String::from).collect()),
                cardinality: None,
            },
            constraints: vec![],
            field_number: None,
        }))
    }

    /// Helper to create a regular field with a basic type
    fn make_field_basic(name: &str, basic_type: BasicType) -> TableMember {
        TableMember::Field(FieldDefinition::Regular(RegularField {
            metadata: vec![],
            name: Some(name.to_string()),
            field_type: TypeWithCardinality {
                base_type: TypeName::Basic(basic_type),
                cardinality: None,
            },
            constraints: vec![],
            field_number: None,
        }))
    }

    /// Helper to create an inline embed field
    fn make_inline_embed(name: &str, members: Vec<TableMember>) -> TableMember {
        TableMember::Field(FieldDefinition::InlineEmbed(InlineEmbedField {
            metadata: vec![],
            name: Some(name.to_string()),
            members,
            cardinality: None,
            field_number: None,
        }))
    }

    // ========== Basic Validation Tests ==========

    #[test]
    fn test_empty_definitions_is_valid() {
        let definitions: Vec<Definition> = vec![];
        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_single_table_is_valid() {
        let definitions = vec![make_table("User", vec![])];
        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_single_enum_is_valid() {
        let definitions = vec![make_enum("Status")];
        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_single_embed_is_valid() {
        let definitions = vec![make_embed("Address", vec![])];
        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_table_with_basic_type_field_is_valid() {
        let definitions = vec![make_table(
            "User",
            vec![make_field_basic("name", BasicType::String)],
        )];
        assert!(validate_ast(&definitions).is_ok());
    }

    // ========== Duplicate Definition Tests ==========

    #[test]
    fn test_duplicate_table_names_fails() {
        let definitions = vec![make_table("User", vec![]), make_table("User", vec![])];
        let result = validate_ast(&definitions);
        assert_eq!(
            result,
            Err(ValidationError::DuplicateDefinition("User".to_string()))
        );
    }

    #[test]
    fn test_duplicate_enum_names_fails() {
        let definitions = vec![make_enum("Status"), make_enum("Status")];
        let result = validate_ast(&definitions);
        assert_eq!(
            result,
            Err(ValidationError::DuplicateDefinition("Status".to_string()))
        );
    }

    #[test]
    fn test_duplicate_embed_names_fails() {
        let definitions = vec![
            make_embed("Address", vec![]),
            make_embed("Address", vec![]),
        ];
        let result = validate_ast(&definitions);
        assert_eq!(
            result,
            Err(ValidationError::DuplicateDefinition("Address".to_string()))
        );
    }

    #[test]
    fn test_table_and_enum_same_name_fails() {
        let definitions = vec![make_table("Item", vec![]), make_enum("Item")];
        let result = validate_ast(&definitions);
        assert_eq!(
            result,
            Err(ValidationError::DuplicateDefinition("Item".to_string()))
        );
    }

    // ========== Type Reference Tests ==========

    #[test]
    fn test_field_referencing_defined_type_is_valid() {
        let definitions = vec![
            make_enum("Status"),
            make_table("User", vec![make_field_with_type("status", vec!["Status"])]),
        ];
        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_field_referencing_undefined_type_fails() {
        let definitions = vec![make_table(
            "User",
            vec![make_field_with_type("status", vec!["UndefinedType"])],
        )];
        let result = validate_ast(&definitions);
        assert_eq!(
            result,
            Err(ValidationError::TypeNotFound("UndefinedType".to_string()))
        );
    }

    #[test]
    fn test_field_referencing_table_type_is_valid() {
        let definitions = vec![
            make_embed("Address", vec![]),
            make_table(
                "User",
                vec![make_field_with_type("address", vec!["Address"])],
            ),
        ];
        assert!(validate_ast(&definitions).is_ok());
    }

    // ========== Namespace Tests ==========

    #[test]
    fn test_namespace_type_resolution() {
        let definitions = vec![make_namespace(
            vec!["game", "common"],
            vec![
                make_enum("Status"),
                make_table("User", vec![make_field_with_type("status", vec!["Status"])]),
            ],
        )];
        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_cross_namespace_reference_with_fqn() {
        let definitions = vec![
            make_namespace(vec!["game", "common"], vec![make_enum("Status")]),
            make_namespace(
                vec!["game", "user"],
                vec![make_table(
                    "User",
                    vec![make_field_with_type("status", vec!["game", "common", "Status"])],
                )],
            ),
        ];
        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_nested_namespace_type_resolution() {
        // Type defined in parent namespace should be accessible from child
        let definitions = vec![make_namespace(
            vec!["game"],
            vec![
                make_enum("GlobalStatus"),
                make_namespace(
                    vec!["user"],
                    vec![make_table(
                        "Player",
                        vec![make_field_with_type("status", vec!["GlobalStatus"])],
                    )],
                ),
            ],
        )];
        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_duplicate_type_in_namespace_fails() {
        let definitions = vec![make_namespace(
            vec!["game"],
            vec![make_enum("Status"), make_enum("Status")],
        )];
        let result = validate_ast(&definitions);
        assert_eq!(
            result,
            Err(ValidationError::DuplicateDefinition(
                "game.Status".to_string()
            ))
        );
    }

    #[test]
    fn test_same_name_different_namespaces_is_valid() {
        let definitions = vec![
            make_namespace(vec!["game", "user"], vec![make_enum("Status")]),
            make_namespace(vec!["game", "item"], vec![make_enum("Status")]),
        ];
        assert!(validate_ast(&definitions).is_ok());
    }

    // ========== Inline Embed Tests ==========

    #[test]
    fn test_inline_embed_type_collection() {
        let definitions = vec![make_table(
            "User",
            vec![make_inline_embed(
                "Profile",
                vec![make_field_basic("bio", BasicType::String)],
            )],
        )];
        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_inline_embed_can_reference_external_type() {
        let definitions = vec![
            make_enum("Country"),
            make_table(
                "User",
                vec![make_inline_embed(
                    "Address",
                    vec![make_field_with_type("country", vec!["Country"])],
                )],
            ),
        ];
        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_duplicate_inline_embed_in_same_table_fails() {
        let definitions = vec![make_table(
            "User",
            vec![
                make_inline_embed("Profile", vec![]),
                make_inline_embed("Profile", vec![]),
            ],
        )];
        let result = validate_ast(&definitions);
        assert_eq!(
            result,
            Err(ValidationError::DuplicateDefinition(
                "User.Profile".to_string()
            ))
        );
    }

    // ========== Nested Embed Tests ==========

    #[test]
    fn test_nested_embed_in_table() {
        let definitions = vec![make_table(
            "User",
            vec![TableMember::Embed(Embed {
                metadata: vec![],
                name: Some("Settings".to_string()),
                members: vec![make_field_basic("darkMode", BasicType::Bool)],
            })],
        )];
        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_nested_enum_in_table() {
        let definitions = vec![make_table(
            "User",
            vec![TableMember::Enum(Enum {
                metadata: vec![],
                name: Some("Role".to_string()),
                variants: vec![],
            })],
        )];
        assert!(validate_ast(&definitions).is_ok());
    }

    // ========== Comments and Annotations ==========

    #[test]
    fn test_comments_are_ignored() {
        let definitions = vec![
            Definition::Comment("This is a comment".to_string()),
            make_table("User", vec![]),
        ];
        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_annotations_are_ignored() {
        let definitions = vec![
            Definition::Annotation(Annotation {
                name: Some("deprecated".to_string()),
                params: vec![],
            }),
            make_table("User", vec![]),
        ];
        assert!(validate_ast(&definitions).is_ok());
    }
}