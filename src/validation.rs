use crate::ast_model::{BasicType, Constraint, Definition, FieldDefinition, TableMember, TypeName};
use crate::error::ValidationError;
use crate::type_registry::{TypeKind, TypeRegistry};

/// AST의 유효성을 검사하는 메인 함수입니다.
pub fn validate_ast(definitions: &[Definition]) -> Result<(), ValidationError> {
    let mut registry = TypeRegistry::new();
    // 1. 스키마에 정의된 모든 타입의 전체 경로(FQN)를 수집합니다.
    collect_all_types(definitions, &mut Vec::new(), &mut registry)?;
    // 2. 사용된 타입들이 실제로 정의되었는지 확인합니다.
    validate_all_types(definitions, &mut Vec::new(), &registry)?;
    Ok(())
}

fn collect_from_members(
    members: &[TableMember],
    path: &mut Vec<String>,
    registry: &mut TypeRegistry,
) -> Result<(), ValidationError> {
    for member in members {
        match member {
            TableMember::Embed(e) => {
                path.push(e.name.clone().expect("Embed name should be present"));
                let embed_fqn = path.join(".");
                if !registry.register(&embed_fqn, TypeKind::Embed) {
                    return Err(ValidationError::DuplicateDefinition(embed_fqn));
                }
                // Recursively collect from the embed's members
                collect_from_members(&e.members, path, registry)?;
                path.pop();
            }
            TableMember::Enum(e) => {
                if let Some(name) = &e.name {
                    path.push(name.clone());
                    let enum_fqn = path.join(".");
                    if !registry.register(&enum_fqn, TypeKind::Enum) {
                        return Err(ValidationError::DuplicateDefinition(enum_fqn));
                    }
                    path.pop();
                }
            }
            TableMember::Field(FieldDefinition::InlineEmbed(ief)) => {
                path.push(
                    ief.name
                        .clone()
                        .expect("Inline embed name should be present"),
                );
                let embed_fqn = path.join(".");
                if !registry.register(&embed_fqn, TypeKind::Embed) {
                    return Err(ValidationError::DuplicateDefinition(embed_fqn));
                }
                // Recursively collect from the inline embed's members
                collect_from_members(&ief.members, path, registry)?;
                path.pop();
            }
            _ => {} // Other members don't define new types in this context
        }
    }
    Ok(())
}

/// 재귀적으로 모든 타입 정의를 수집하여 `TypeRegistry`에 등록합니다.
fn collect_all_types(
    definitions: &[Definition],
    path: &mut Vec<String>,
    registry: &mut TypeRegistry,
) -> Result<(), ValidationError> {
    for def in definitions {
        match def {
            Definition::Namespace(ns) => {
                path.extend(ns.path.iter().cloned());
                collect_all_types(&ns.definitions, path, registry)?;
                for _ in 0..ns.path.len() {
                    path.pop();
                }
            }
            Definition::Table(t) => {
                let fqn = path
                    .iter()
                    .chain(std::iter::once(
                        t.name.as_ref().expect("Table name should be present"),
                    ))
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(".");
                if !registry.register(&fqn, TypeKind::Struct) {
                    return Err(ValidationError::DuplicateDefinition(fqn));
                }

                path.push(
                    t.name
                        .as_ref()
                        .expect("Table name should be present")
                        .clone(),
                );
                collect_from_members(&t.members, path, registry)?;
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
                    if !registry.register(&fqn, TypeKind::Enum) {
                        return Err(ValidationError::DuplicateDefinition(fqn));
                    }
                }
            }
            Definition::Embed(e) => {
                let fqn = path
                    .iter()
                    .chain(std::iter::once(
                        e.name.as_ref().expect("Embed name should be present"),
                    ))
                    .cloned()
                    .collect::<Vec<_>>()
                    .join(".");
                if !registry.register(&fqn, TypeKind::Embed) {
                    return Err(ValidationError::DuplicateDefinition(fqn));
                }
                path.push(
                    e.name
                        .as_ref()
                        .expect("Embed name should be present")
                        .clone(),
                );
                collect_from_members(&e.members, path, registry)?;
                path.pop();
            }
            Definition::Comment(_) => { /* Comments are not types, so we ignore them. */ }
            Definition::Annotation(_) => { /* Annotations are not types, so we ignore them. */ }
        }
    }
    Ok(())
}

/// Checks if a given type path can be resolved to a known type.
/// Uses TypeRegistry's resolve method which handles:
/// 1. Direct FQN match
/// 2. Qualified with current namespace
/// 3. Unique name resolution
fn check_type_path(
    type_path: &[String],
    current_scope: &[String],
    registry: &TypeRegistry,
) -> Result<(), ValidationError> {
    let used_type_str = type_path.join(".");
    let current_namespace = current_scope.join(".");

    // Use TypeRegistry's resolve method
    if registry
        .resolve(&used_type_str, &current_namespace)
        .is_some()
    {
        return Ok(());
    }

    // Also try walking up the scope for nested type resolution
    let mut scope = current_scope.to_vec();
    loop {
        let mut potential_fqn_parts = scope.clone();
        potential_fqn_parts.extend_from_slice(type_path);
        let potential_fqn = potential_fqn_parts.join(".");
        if registry.contains(&potential_fqn) {
            return Ok(());
        }
        if scope.is_empty() {
            break;
        }
        scope.pop();
    }

    Err(ValidationError::TypeNotFound(used_type_str))
}

fn validate_table_members(
    members: &[TableMember],
    path: &mut Vec<String>,
    registry: &TypeRegistry,
) -> Result<(), ValidationError> {
    // Check for multiple auto_create fields (only one allowed per table)
    validate_single_auto_create(members, path)?;

    for member in members {
        match member {
            TableMember::Field(field) => match field {
                FieldDefinition::Regular(rf) => {
                    if let TypeName::Path(type_path) = &rf.field_type.base_type {
                        check_type_path(type_path, path, registry)?;
                    }
                    // Validate auto_create/auto_update constraints are only on timestamp fields
                    validate_timestamp_constraints(rf)?;
                }
                FieldDefinition::InlineEmbed(ief) => {
                    path.push(ief.name.clone().expect("Inline embed must have a name"));
                    validate_table_members(&ief.members, path, registry)?;
                    path.pop();
                }
                FieldDefinition::InlineEnum(_) => {}
            },
            TableMember::Embed(embed) => {
                path.push(embed.name.clone().expect("Embed must have a name"));
                validate_table_members(&embed.members, path, registry)?;
                path.pop();
            }
            TableMember::Enum(_) => {}
            TableMember::Comment(_) => {}
        }
    }
    Ok(())
}

/// Validates that only one auto_create field exists per table.
fn validate_single_auto_create(
    members: &[TableMember],
    path: &[String],
) -> Result<(), ValidationError> {
    let auto_create_fields: Vec<String> = members
        .iter()
        .filter_map(|member| {
            if let TableMember::Field(FieldDefinition::Regular(rf)) = member {
                let has_auto_create = rf
                    .constraints
                    .iter()
                    .any(|c| matches!(c, Constraint::AutoCreate(_)));
                if has_auto_create {
                    return rf.name.clone();
                }
            }
            None
        })
        .collect();

    if auto_create_fields.len() > 1 {
        let table_name = path.last().map(|s| s.as_str()).unwrap_or("<unknown>");
        return Err(ValidationError::InvalidConstraint {
            field: auto_create_fields.join(", "),
            constraint: "auto_create".to_string(),
            message: format!(
                "only one auto_create field is allowed per table '{}', but found {} fields: [{}]",
                table_name,
                auto_create_fields.len(),
                auto_create_fields.join(", ")
            ),
        });
    }
    Ok(())
}

/// Validates that auto_create and auto_update constraints are only used with timestamp type.
fn validate_timestamp_constraints(
    rf: &crate::ast_model::RegularField,
) -> Result<(), ValidationError> {
    let is_timestamp = matches!(
        rf.field_type.base_type,
        TypeName::Basic(BasicType::Timestamp)
    );
    let field_name = rf.name.clone().unwrap_or_else(|| "<unknown>".to_string());

    for constraint in &rf.constraints {
        match constraint {
            Constraint::AutoCreate(_) => {
                if !is_timestamp {
                    return Err(ValidationError::InvalidConstraint {
                        field: field_name,
                        constraint: "auto_create".to_string(),
                        message: "auto_create can only be used with timestamp type".to_string(),
                    });
                }
            }
            Constraint::AutoUpdate(_) => {
                if !is_timestamp {
                    return Err(ValidationError::InvalidConstraint {
                        field: field_name,
                        constraint: "auto_update".to_string(),
                        message: "auto_update can only be used with timestamp type".to_string(),
                    });
                }
            }
            _ => {}
        }
    }
    Ok(())
}

/// 재귀적으로 모든 필드를 순회하며 사용된 타입의 유효성을 검사합니다.
fn validate_all_types(
    definitions: &[Definition],
    path: &mut Vec<String>,
    registry: &TypeRegistry,
) -> Result<(), ValidationError> {
    for def in definitions {
        match def {
            Definition::Namespace(ns) => {
                path.extend(ns.path.iter().cloned());
                validate_all_types(&ns.definitions, path, registry)?;
                for _ in 0..ns.path.len() {
                    path.pop();
                }
            }
            Definition::Table(t) => {
                path.push(t.name.clone().expect("Table name should be present"));
                validate_table_members(&t.members, path, registry)?;
                path.pop();
            }
            Definition::Enum(_) => { /* Enums do not reference other types */ }
            Definition::Embed(e) => {
                // Validate types used within the embed's fields.
                path.push(e.name.clone().expect("Embed name should be present"));
                validate_table_members(&e.members, path, registry)?;
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
            metadata: vec![],
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

    /// Helper to create a regular field with a basic type and constraints
    fn make_field_with_constraints(
        name: &str,
        basic_type: BasicType,
        constraints: Vec<Constraint>,
    ) -> TableMember {
        TableMember::Field(FieldDefinition::Regular(RegularField {
            metadata: vec![],
            name: Some(name.to_string()),
            field_type: TypeWithCardinality {
                base_type: TypeName::Basic(basic_type),
                cardinality: None,
            },
            constraints,
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
        let definitions = vec![make_embed("Address", vec![]), make_embed("Address", vec![])];
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
                    vec![make_field_with_type(
                        "status",
                        vec!["game", "common", "Status"],
                    )],
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
                args: vec![],
            }),
            make_table("User", vec![]),
        ];
        assert!(validate_ast(&definitions).is_ok());
    }

    // ========== Timestamp Constraint Validation Tests ==========

    #[test]
    fn test_auto_create_on_timestamp_is_valid() {
        let definitions = vec![make_table(
            "Event",
            vec![make_field_with_constraints(
                "created_at",
                BasicType::Timestamp,
                vec![Constraint::AutoCreate(None)],
            )],
        )];
        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_auto_update_on_timestamp_is_valid() {
        let definitions = vec![make_table(
            "Event",
            vec![make_field_with_constraints(
                "updated_at",
                BasicType::Timestamp,
                vec![Constraint::AutoUpdate(Some(Timezone::Utc))],
            )],
        )];
        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_auto_create_on_non_timestamp_fails() {
        let definitions = vec![make_table(
            "Event",
            vec![make_field_with_constraints(
                "name",
                BasicType::String,
                vec![Constraint::AutoCreate(None)],
            )],
        )];
        let result = validate_ast(&definitions);
        assert_eq!(
            result,
            Err(ValidationError::InvalidConstraint {
                field: "name".to_string(),
                constraint: "auto_create".to_string(),
                message: "auto_create can only be used with timestamp type".to_string(),
            })
        );
    }

    #[test]
    fn test_auto_update_on_non_timestamp_fails() {
        let definitions = vec![make_table(
            "Event",
            vec![make_field_with_constraints(
                "count",
                BasicType::I32,
                vec![Constraint::AutoUpdate(Some(Timezone::Local))],
            )],
        )];
        let result = validate_ast(&definitions);
        assert_eq!(
            result,
            Err(ValidationError::InvalidConstraint {
                field: "count".to_string(),
                constraint: "auto_update".to_string(),
                message: "auto_update can only be used with timestamp type".to_string(),
            })
        );
    }

    #[test]
    fn test_multiple_auto_create_fails() {
        let definitions = vec![make_table(
            "Event",
            vec![
                make_field_with_constraints(
                    "created_at",
                    BasicType::Timestamp,
                    vec![Constraint::AutoCreate(None)],
                ),
                make_field_with_constraints(
                    "inserted_at",
                    BasicType::Timestamp,
                    vec![Constraint::AutoCreate(Some(Timezone::Utc))],
                ),
            ],
        )];
        let result = validate_ast(&definitions);
        assert!(result.is_err());
        if let Err(ValidationError::InvalidConstraint {
            constraint,
            message,
            ..
        }) = result
        {
            assert_eq!(constraint, "auto_create");
            assert!(message.contains("only one auto_create field is allowed"));
        } else {
            panic!("Expected InvalidConstraint error");
        }
    }

    #[test]
    fn test_multiple_auto_update_is_valid() {
        // Multiple auto_update fields should be allowed
        let definitions = vec![make_table(
            "Event",
            vec![
                make_field_with_constraints(
                    "updated_at",
                    BasicType::Timestamp,
                    vec![Constraint::AutoUpdate(None)],
                ),
                make_field_with_constraints(
                    "modified_at",
                    BasicType::Timestamp,
                    vec![Constraint::AutoUpdate(Some(Timezone::Local))],
                ),
            ],
        )];
        assert!(validate_ast(&definitions).is_ok());
    }
}
