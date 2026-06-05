use crate::ast_model::{
    AnnotationArg, BasicType, Cardinality, Constraint, Definition, FieldDefinition, Literal,
    Metadata, TableMember, TypeName, TypeWithCardinality,
};
use crate::error::ValidationError;
use crate::type_registry::{TypeKind, TypeRegistry};
use regex::Regex;
use std::collections::{HashMap, HashSet};

/// AST의 유효성을 검사하는 메인 함수입니다.
pub fn validate_ast(definitions: &[Definition]) -> Result<(), ValidationError> {
    let mut registry = TypeRegistry::new();
    // 1. 스키마에 정의된 모든 타입의 전체 경로(FQN)를 수집합니다.
    collect_all_types(definitions, &mut Vec::new(), &mut registry)?;
    let mut field_registry = FieldRegistry::default();
    collect_table_fields(definitions, &mut Vec::new(), &mut field_registry)?;
    // 2. 어노테이션 인자의 의미적 유효성을 확인합니다.
    validate_all_annotations(definitions, &mut Vec::new(), &registry)?;
    // 3. 사용된 타입들이 실제로 정의되었는지 확인합니다.
    validate_all_types(definitions, &mut Vec::new(), &registry, &field_registry)?;
    Ok(())
}

fn required_name<'a>(name: &'a Option<String>, kind: &str) -> Result<&'a str, ValidationError> {
    name.as_deref().ok_or_else(|| ValidationError::MissingName {
        kind: kind.to_string(),
    })
}

fn push_required_name(
    path: &mut Vec<String>,
    name: &Option<String>,
    kind: &str,
) -> Result<(), ValidationError> {
    path.push(required_name(name, kind)?.to_string());
    Ok(())
}

fn qualified_name(path: &[String], name: &str) -> String {
    let mut parts = path.to_vec();
    parts.push(name.to_string());
    parts.join(".")
}

fn collect_from_members(
    members: &[TableMember],
    path: &mut Vec<String>,
    registry: &mut TypeRegistry,
) -> Result<(), ValidationError> {
    for member in members {
        match member {
            TableMember::Embed(e) => {
                push_required_name(path, &e.name, "embed")?;
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
                push_required_name(path, &ief.name, "inline embed")?;
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
                let table_name = required_name(&t.name, "table")?;
                let fqn = qualified_name(path, table_name);
                if !registry.register(&fqn, TypeKind::Struct) {
                    return Err(ValidationError::DuplicateDefinition(fqn));
                }

                path.push(table_name.to_string());
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
                let embed_name = required_name(&e.name, "embed")?;
                let fqn = qualified_name(path, embed_name);
                if !registry.register(&fqn, TypeKind::Embed) {
                    return Err(ValidationError::DuplicateDefinition(fqn));
                }
                path.push(embed_name.to_string());
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
    if resolve_type_path(type_path, current_scope, registry).is_some() {
        return Ok(());
    }

    Err(ValidationError::TypeNotFound(used_type_str))
}

fn resolve_type_path(
    type_path: &[String],
    current_scope: &[String],
    registry: &TypeRegistry,
) -> Option<String> {
    let used_type_str = type_path.join(".");
    let current_namespace = current_scope.join(".");

    if let Some(fqn) = registry.resolve(&used_type_str, &current_namespace) {
        return Some(fqn.to_string());
    }

    let mut scope = current_scope.to_vec();
    loop {
        let mut potential_fqn_parts = scope.clone();
        potential_fqn_parts.extend_from_slice(type_path);
        let potential_fqn = potential_fqn_parts.join(".");
        if registry.contains(&potential_fqn) {
            return Some(potential_fqn);
        }
        if scope.is_empty() {
            break;
        }
        scope.pop();
    }

    None
}

#[derive(Default)]
struct FieldRegistry<'a> {
    table_fields: HashMap<String, HashMap<String, FieldInfo<'a>>>,
}

#[derive(Clone, Copy)]
struct FieldInfo<'a> {
    field_type: &'a TypeWithCardinality,
    is_primary_key: bool,
    is_unique: bool,
}

impl<'a> FieldRegistry<'a> {
    fn insert_table(&mut self, table_fqn: String, members: &'a [TableMember]) {
        let fields = members
            .iter()
            .filter_map(|member| {
                if let TableMember::Field(FieldDefinition::Regular(field)) = member {
                    let is_primary_key = field
                        .constraints
                        .iter()
                        .any(|constraint| matches!(constraint, Constraint::PrimaryKey));
                    let is_unique = field
                        .constraints
                        .iter()
                        .any(|constraint| matches!(constraint, Constraint::Unique));
                    return field.name.as_ref().map(|name| {
                        (
                            name.clone(),
                            FieldInfo {
                                field_type: &field.field_type,
                                is_primary_key,
                                is_unique,
                            },
                        )
                    });
                }
                None
            })
            .collect();

        self.table_fields.insert(table_fqn, fields);
    }

    fn field_info(&self, table_fqn: &str, field_name: &str) -> Option<FieldInfo<'a>> {
        self.table_fields
            .get(table_fqn)
            .and_then(|fields| fields.get(field_name).copied())
    }
}

fn collect_table_fields<'a>(
    definitions: &'a [Definition],
    path: &mut Vec<String>,
    field_registry: &mut FieldRegistry<'a>,
) -> Result<(), ValidationError> {
    for def in definitions {
        match def {
            Definition::Namespace(ns) => {
                path.extend(ns.path.iter().cloned());
                collect_table_fields(&ns.definitions, path, field_registry)?;
                for _ in 0..ns.path.len() {
                    path.pop();
                }
            }
            Definition::Table(t) => {
                let table_name = required_name(&t.name, "table")?;
                let table_fqn = qualified_name(path, table_name);
                field_registry.insert_table(table_fqn, &t.members);
            }
            Definition::Enum(_) | Definition::Embed(_) => {}
            Definition::Comment(_) | Definition::Annotation(_) => {}
        }
    }
    Ok(())
}

fn validate_table_members(
    members: &[TableMember],
    path: &mut Vec<String>,
    registry: &TypeRegistry,
    field_registry: &FieldRegistry<'_>,
) -> Result<(), ValidationError> {
    // Check for multiple auto_create fields (only one allowed per table)
    validate_single_auto_create(members, path)?;
    validate_single_primary_key(members, path)?;

    for member in members {
        match member {
            TableMember::Field(field) => match field {
                FieldDefinition::Regular(rf) => {
                    if let TypeName::Path(type_path) = &rf.field_type.base_type {
                        check_type_path(type_path, path, registry)?;
                    }
                    // Validate auto_create/auto_update constraints are only on timestamp fields
                    validate_timestamp_constraints(rf)?;
                    validate_field_constraints(rf, path, registry, field_registry)?;
                }
                FieldDefinition::InlineEmbed(ief) => {
                    push_required_name(path, &ief.name, "inline embed")?;
                    validate_table_members(&ief.members, path, registry, field_registry)?;
                    path.pop();
                }
                FieldDefinition::InlineEnum(_) => {}
            },
            TableMember::Embed(embed) => {
                push_required_name(path, &embed.name, "embed")?;
                validate_table_members(&embed.members, path, registry, field_registry)?;
                path.pop();
            }
            TableMember::Enum(_) => {}
            TableMember::Comment(_) => {}
        }
    }
    Ok(())
}

fn validate_single_primary_key(
    members: &[TableMember],
    path: &[String],
) -> Result<(), ValidationError> {
    let primary_key_fields: Vec<String> = members
        .iter()
        .filter_map(|member| {
            if let TableMember::Field(FieldDefinition::Regular(rf)) = member {
                let has_primary_key = rf
                    .constraints
                    .iter()
                    .any(|c| matches!(c, Constraint::PrimaryKey));
                if has_primary_key {
                    return rf.name.clone();
                }
            }
            None
        })
        .collect();

    if primary_key_fields.len() > 1 {
        let table_name = path.last().map(|s| s.as_str()).unwrap_or("<unknown>");
        return Err(ValidationError::InvalidConstraint {
            field: primary_key_fields.join(", "),
            constraint: "primary_key".to_string(),
            message: format!(
                "only one primary_key field is allowed per table '{}', but found {} fields: [{}]",
                table_name,
                primary_key_fields.len(),
                primary_key_fields.join(", ")
            ),
        });
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
            Constraint::AutoCreate(_) if !is_timestamp => {
                return Err(ValidationError::InvalidConstraint {
                    field: field_name,
                    constraint: "auto_create".to_string(),
                    message: "auto_create can only be used with timestamp type".to_string(),
                });
            }
            Constraint::AutoUpdate(_) if !is_timestamp => {
                return Err(ValidationError::InvalidConstraint {
                    field: field_name,
                    constraint: "auto_update".to_string(),
                    message: "auto_update can only be used with timestamp type".to_string(),
                });
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_field_constraints(
    rf: &crate::ast_model::RegularField,
    current_scope: &[String],
    registry: &TypeRegistry,
    field_registry: &FieldRegistry<'_>,
) -> Result<(), ValidationError> {
    let field_name = rf.name.clone().unwrap_or_else(|| "<unknown>".to_string());

    let mut default_value = None;
    let mut range_value = None;
    let mut has_default = false;
    let mut has_foreign_key = false;
    let mut has_index = false;
    let mut has_max_length = false;
    let mut has_primary_key = false;
    let mut has_range = false;
    let mut has_regex = false;
    let mut has_unique = false;

    for constraint in &rf.constraints {
        match constraint {
            Constraint::PrimaryKey => {
                if has_primary_key {
                    return invalid_field_constraint(
                        &field_name,
                        "primary_key",
                        "primary_key is specified more than once",
                    );
                }
                has_primary_key = true;
                validate_primary_key_constraint(
                    &field_name,
                    &rf.field_type,
                    current_scope,
                    registry,
                )?;
            }
            Constraint::Unique => {
                if has_unique {
                    return invalid_field_constraint(
                        &field_name,
                        "unique",
                        "unique is specified more than once",
                    );
                }
                has_unique = true;
                validate_unique_constraint(&field_name, &rf.field_type, current_scope, registry)?;
            }
            Constraint::Index => {
                if has_index {
                    return invalid_field_constraint(
                        &field_name,
                        "index",
                        "index is specified more than once",
                    );
                }
                has_index = true;
                validate_index_constraint(&field_name, &rf.field_type, current_scope, registry)?;
            }
            Constraint::Default(value) => {
                if has_default {
                    return invalid_field_constraint(
                        &field_name,
                        "default",
                        "default is specified more than once",
                    );
                }
                has_default = true;
                default_value = Some(value);
                validate_default_constraint(
                    &field_name,
                    value,
                    &rf.field_type,
                    current_scope,
                    registry,
                )?;
            }
            Constraint::ForeignKey(path, _) => {
                if has_foreign_key {
                    return invalid_field_constraint(
                        &field_name,
                        "foreign_key",
                        "foreign_key is specified more than once",
                    );
                }
                has_foreign_key = true;
                validate_foreign_key_constraint(
                    &field_name,
                    &rf.field_type,
                    path,
                    current_scope,
                    registry,
                    field_registry,
                )?;
            }
            Constraint::MaxLength(len) => {
                if has_max_length {
                    return invalid_field_constraint(
                        &field_name,
                        "max_length",
                        "max_length is specified more than once",
                    );
                }
                has_max_length = true;
                validate_max_length_constraint(&field_name, *len, &rf.field_type)?;
            }
            Constraint::Range(min, max) => {
                if has_range {
                    return invalid_field_constraint(
                        &field_name,
                        "range",
                        "range is specified more than once",
                    );
                }
                has_range = true;
                range_value = Some((min, max));
                validate_range_constraint(&field_name, min, max, &rf.field_type)?;
            }
            Constraint::Regex(pattern) => {
                if has_regex {
                    return invalid_field_constraint(
                        &field_name,
                        "regex",
                        "regex is specified more than once",
                    );
                }
                has_regex = true;
                validate_regex_constraint(&field_name, pattern, &rf.field_type)?;
            }
            _ => {}
        }
    }

    if let (Some(default), Some((min, max))) = (default_value, range_value) {
        validate_default_with_range(&field_name, default, min, max)?;
    }

    Ok(())
}

fn validate_primary_key_constraint(
    field_name: &str,
    field_type: &TypeWithCardinality,
    current_scope: &[String],
    registry: &TypeRegistry,
) -> Result<(), ValidationError> {
    if matches!(field_type.cardinality, Some(Cardinality::Optional)) {
        return invalid_field_constraint(
            field_name,
            "primary_key",
            "primary_key fields cannot be optional",
        );
    }

    validate_indexable_field_type(
        field_name,
        "primary_key",
        field_type,
        current_scope,
        registry,
    )
}

fn validate_unique_constraint(
    field_name: &str,
    field_type: &TypeWithCardinality,
    current_scope: &[String],
    registry: &TypeRegistry,
) -> Result<(), ValidationError> {
    validate_indexable_field_type(field_name, "unique", field_type, current_scope, registry)
}

fn validate_index_constraint(
    field_name: &str,
    field_type: &TypeWithCardinality,
    current_scope: &[String],
    registry: &TypeRegistry,
) -> Result<(), ValidationError> {
    validate_indexable_field_type(field_name, "index", field_type, current_scope, registry)
}

fn validate_indexable_field_type(
    field_name: &str,
    constraint: &str,
    field_type: &TypeWithCardinality,
    current_scope: &[String],
    registry: &TypeRegistry,
) -> Result<(), ValidationError> {
    if matches!(field_type.cardinality, Some(Cardinality::Array)) {
        return invalid_field_constraint(
            field_name,
            constraint,
            "constraint is not supported for array fields",
        );
    }

    match &field_type.base_type {
        TypeName::Basic(BasicType::Bytes) => invalid_field_constraint(
            field_name,
            constraint,
            "constraint is not supported for bytes fields",
        ),
        TypeName::Basic(_) | TypeName::InlineEnum(_) => Ok(()),
        TypeName::Path(path) => {
            let Some(fqn) = resolve_type_path(path, current_scope, registry) else {
                return invalid_field_constraint(
                    field_name,
                    constraint,
                    "constraint type reference could not be resolved",
                );
            };

            match registry.get_kind(&fqn) {
                Some(TypeKind::Enum) => Ok(()),
                Some(TypeKind::Struct | TypeKind::Embed) => invalid_field_constraint(
                    field_name,
                    constraint,
                    "constraint is not supported for struct or embed fields",
                ),
                None => invalid_field_constraint(
                    field_name,
                    constraint,
                    "constraint type reference could not be resolved",
                ),
            }
        }
    }
}

fn validate_foreign_key_constraint(
    field_name: &str,
    field_type: &TypeWithCardinality,
    target_path: &[String],
    current_scope: &[String],
    registry: &TypeRegistry,
    field_registry: &FieldRegistry<'_>,
) -> Result<(), ValidationError> {
    if target_path.len() < 2 {
        return invalid_field_constraint(
            field_name,
            "foreign_key",
            "foreign_key target must be a table field path",
        );
    }

    if matches!(field_type.cardinality, Some(Cardinality::Array)) {
        return invalid_field_constraint(
            field_name,
            "foreign_key",
            "foreign_key is not supported for array fields",
        );
    }

    let target_field_name = target_path.last().expect("checked target path length");
    let target_table_path = &target_path[..target_path.len() - 1];
    let Some(target_table_fqn) = resolve_type_path(target_table_path, current_scope, registry)
    else {
        return Err(ValidationError::TypeNotFound(target_table_path.join(".")));
    };

    if registry.get_kind(&target_table_fqn) != Some(TypeKind::Struct) {
        return invalid_field_constraint(
            field_name,
            "foreign_key",
            "foreign_key target must be a table",
        );
    }

    let Some(target_field_info) = field_registry.field_info(&target_table_fqn, target_field_name)
    else {
        return invalid_field_constraint(
            field_name,
            "foreign_key",
            "foreign_key target field does not exist",
        );
    };

    if !target_field_info.is_primary_key && !target_field_info.is_unique {
        return invalid_field_constraint(
            field_name,
            "foreign_key",
            "foreign_key target field must be primary_key or unique",
        );
    }

    if matches!(
        target_field_info.field_type.cardinality,
        Some(Cardinality::Array)
    ) {
        return invalid_field_constraint(
            field_name,
            "foreign_key",
            "foreign_key target field cannot be an array",
        );
    }

    let target_scope: Vec<String> = target_table_fqn.split('.').map(str::to_string).collect();
    if !field_types_are_compatible(
        field_type,
        current_scope,
        target_field_info.field_type,
        &target_scope,
        registry,
    ) {
        return invalid_field_constraint(
            field_name,
            "foreign_key",
            "foreign_key field type must match the target field type",
        );
    }

    Ok(())
}

fn field_types_are_compatible(
    source: &TypeWithCardinality,
    source_scope: &[String],
    target: &TypeWithCardinality,
    target_scope: &[String],
    registry: &TypeRegistry,
) -> bool {
    type_identity(&source.base_type, source_scope, registry)
        == type_identity(&target.base_type, target_scope, registry)
}

fn type_identity(
    type_name: &TypeName,
    current_scope: &[String],
    registry: &TypeRegistry,
) -> Option<String> {
    match type_name {
        TypeName::Basic(basic_type) => Some(format!("basic:{basic_type:?}")),
        TypeName::Path(path) => {
            resolve_type_path(path, current_scope, registry).map(|fqn| format!("path:{fqn}"))
        }
        TypeName::InlineEnum(_) => Some("inline_enum".to_string()),
    }
}

fn validate_default_constraint(
    field_name: &str,
    value: &Literal,
    field_type: &TypeWithCardinality,
    current_scope: &[String],
    registry: &TypeRegistry,
) -> Result<(), ValidationError> {
    if matches!(field_type.cardinality, Some(Cardinality::Array)) {
        return invalid_field_constraint(
            field_name,
            "default",
            "default is not supported for array fields",
        );
    }

    match &field_type.base_type {
        TypeName::Basic(basic_type) => {
            validate_basic_default_constraint(field_name, value, basic_type)
        }
        TypeName::InlineEnum(enum_def) => validate_inline_enum_default(field_name, value, enum_def),
        TypeName::Path(type_path) => {
            validate_path_default_constraint(field_name, value, type_path, current_scope, registry)
        }
    }
}

fn validate_basic_default_constraint(
    field_name: &str,
    value: &Literal,
    basic_type: &BasicType,
) -> Result<(), ValidationError> {
    match basic_type {
        BasicType::String => {
            if matches!(value, Literal::String(_)) {
                Ok(())
            } else {
                invalid_field_constraint(
                    field_name,
                    "default",
                    "string fields require a string default value",
                )
            }
        }
        BasicType::Bool => {
            if matches!(value, Literal::Boolean(_)) {
                Ok(())
            } else {
                invalid_field_constraint(
                    field_name,
                    "default",
                    "bool fields require a boolean default value",
                )
            }
        }
        BasicType::F32 | BasicType::F64 => {
            if matches!(value, Literal::Integer(_) | Literal::Float(_)) {
                Ok(())
            } else {
                invalid_field_constraint(
                    field_name,
                    "default",
                    "floating-point fields require a numeric default value",
                )
            }
        }
        BasicType::I8
        | BasicType::I16
        | BasicType::I32
        | BasicType::I64
        | BasicType::U8
        | BasicType::U16
        | BasicType::U32
        | BasicType::U64 => validate_integer_default_constraint(field_name, value, basic_type),
        BasicType::Bytes => invalid_field_constraint(
            field_name,
            "default",
            "default is not supported for bytes fields",
        ),
        BasicType::Timestamp => invalid_field_constraint(
            field_name,
            "default",
            "default is not supported for timestamp fields; use auto_create or auto_update",
        ),
    }
}

fn validate_integer_default_constraint(
    field_name: &str,
    value: &Literal,
    basic_type: &BasicType,
) -> Result<(), ValidationError> {
    let Literal::Integer(value) = value else {
        return invalid_field_constraint(
            field_name,
            "default",
            "integer fields require an integer default value",
        );
    };

    let (min, max) = integer_bounds(basic_type).expect("integer type must have bounds");
    let value = i128::from(*value);
    if value < min || value > max {
        return invalid_field_constraint(
            field_name,
            "default",
            "integer default value is outside the field type range",
        );
    }

    Ok(())
}

fn validate_inline_enum_default(
    field_name: &str,
    value: &Literal,
    enum_def: &crate::ast_model::Enum,
) -> Result<(), ValidationError> {
    match value {
        Literal::Identifier(name) => {
            if enum_def
                .variants
                .iter()
                .any(|variant| variant.name.as_deref() == Some(name.as_str()))
            {
                Ok(())
            } else {
                invalid_field_constraint(
                    field_name,
                    "default",
                    "inline enum default must reference an existing variant",
                )
            }
        }
        Literal::Integer(value) => {
            if enum_def
                .variants
                .iter()
                .any(|variant| variant.value == Some(*value))
            {
                Ok(())
            } else {
                invalid_field_constraint(
                    field_name,
                    "default",
                    "inline enum default integer must match an existing variant value",
                )
            }
        }
        _ => invalid_field_constraint(
            field_name,
            "default",
            "enum fields require an identifier or integer default value",
        ),
    }
}

fn validate_path_default_constraint(
    field_name: &str,
    value: &Literal,
    type_path: &[String],
    current_scope: &[String],
    registry: &TypeRegistry,
) -> Result<(), ValidationError> {
    let Some(fqn) = resolve_type_path(type_path, current_scope, registry) else {
        return invalid_field_constraint(
            field_name,
            "default",
            "default type reference could not be resolved",
        );
    };

    match registry.get_kind(&fqn) {
        Some(TypeKind::Enum) => {
            if matches!(value, Literal::Identifier(_) | Literal::Integer(_)) {
                Ok(())
            } else {
                invalid_field_constraint(
                    field_name,
                    "default",
                    "enum fields require an identifier or integer default value",
                )
            }
        }
        Some(TypeKind::Struct | TypeKind::Embed) => invalid_field_constraint(
            field_name,
            "default",
            "default is not supported for struct or embed fields",
        ),
        None => invalid_field_constraint(
            field_name,
            "default",
            "default type reference could not be resolved",
        ),
    }
}

fn validate_default_with_range(
    field_name: &str,
    default: &Literal,
    min: &Literal,
    max: &Literal,
) -> Result<(), ValidationError> {
    let default_value =
        literal_as_number(default).ok_or_else(|| ValidationError::InvalidConstraint {
            field: field_name.to_string(),
            constraint: "default".to_string(),
            message: "default value must be numeric when range is specified".to_string(),
        })?;
    let min_value = literal_as_number(min).expect("range validation guarantees numeric min");
    let max_value = literal_as_number(max).expect("range validation guarantees numeric max");

    if default_value < min_value || default_value > max_value {
        return invalid_field_constraint(
            field_name,
            "default",
            "default value must be within the field range",
        );
    }

    Ok(())
}

fn integer_bounds(basic_type: &BasicType) -> Option<(i128, i128)> {
    match basic_type {
        BasicType::I8 => Some((i128::from(i8::MIN), i128::from(i8::MAX))),
        BasicType::I16 => Some((i128::from(i16::MIN), i128::from(i16::MAX))),
        BasicType::I32 => Some((i128::from(i32::MIN), i128::from(i32::MAX))),
        BasicType::I64 => Some((i128::from(i64::MIN), i128::from(i64::MAX))),
        BasicType::U8 => Some((0, i128::from(u8::MAX))),
        BasicType::U16 => Some((0, i128::from(u16::MAX))),
        BasicType::U32 => Some((0, i128::from(u32::MAX))),
        BasicType::U64 => Some((0, i128::from(u64::MAX))),
        _ => None,
    }
}

fn validate_max_length_constraint(
    field_name: &str,
    len: u32,
    field_type: &crate::ast_model::TypeWithCardinality,
) -> Result<(), ValidationError> {
    if len == 0 {
        return invalid_field_constraint(
            field_name,
            "max_length",
            "max_length must be greater than 0",
        );
    }

    match field_type.base_type {
        TypeName::Basic(BasicType::String | BasicType::Bytes) => Ok(()),
        _ => invalid_field_constraint(
            field_name,
            "max_length",
            "max_length can only be used with string or bytes fields",
        ),
    }
}

fn validate_regex_constraint(
    field_name: &str,
    pattern: &str,
    field_type: &crate::ast_model::TypeWithCardinality,
) -> Result<(), ValidationError> {
    if !matches!(field_type.base_type, TypeName::Basic(BasicType::String)) {
        return invalid_field_constraint(
            field_name,
            "regex",
            "regex can only be used with string fields",
        );
    }

    Regex::new(pattern)
        .map(|_| ())
        .map_err(|_| ValidationError::InvalidConstraint {
            field: field_name.to_string(),
            constraint: "regex".to_string(),
            message: "regex pattern is invalid".to_string(),
        })
}

fn validate_range_constraint(
    field_name: &str,
    min: &Literal,
    max: &Literal,
    field_type: &crate::ast_model::TypeWithCardinality,
) -> Result<(), ValidationError> {
    let Some(numeric_kind) = numeric_type_kind(&field_type.base_type) else {
        return invalid_field_constraint(
            field_name,
            "range",
            "range can only be used with numeric fields",
        );
    };

    let min_value = literal_as_number(min).ok_or_else(|| ValidationError::InvalidConstraint {
        field: field_name.to_string(),
        constraint: "range".to_string(),
        message: "range bounds must be numeric literals".to_string(),
    })?;
    let max_value = literal_as_number(max).ok_or_else(|| ValidationError::InvalidConstraint {
        field: field_name.to_string(),
        constraint: "range".to_string(),
        message: "range bounds must be numeric literals".to_string(),
    })?;

    if min_value > max_value {
        return invalid_field_constraint(
            field_name,
            "range",
            "range minimum must be less than or equal to maximum",
        );
    }

    if numeric_kind == NumericKind::UnsignedInteger && min_value < 0.0 {
        return invalid_field_constraint(
            field_name,
            "range",
            "unsigned integer range minimum must be greater than or equal to 0",
        );
    }

    if numeric_kind != NumericKind::Float && (!is_integer_literal(min) || !is_integer_literal(max))
    {
        return invalid_field_constraint(
            field_name,
            "range",
            "integer field range bounds must be integer literals",
        );
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NumericKind {
    SignedInteger,
    UnsignedInteger,
    Float,
}

fn numeric_type_kind(type_name: &TypeName) -> Option<NumericKind> {
    match type_name {
        TypeName::Basic(BasicType::I8 | BasicType::I16 | BasicType::I32 | BasicType::I64) => {
            Some(NumericKind::SignedInteger)
        }
        TypeName::Basic(BasicType::U8 | BasicType::U16 | BasicType::U32 | BasicType::U64) => {
            Some(NumericKind::UnsignedInteger)
        }
        TypeName::Basic(BasicType::F32 | BasicType::F64) => Some(NumericKind::Float),
        _ => None,
    }
}

fn literal_as_number(literal: &Literal) -> Option<f64> {
    match literal {
        Literal::Integer(value) => Some(*value as f64),
        Literal::Float(value) => Some(*value),
        _ => None,
    }
}

fn is_integer_literal(literal: &Literal) -> bool {
    matches!(literal, Literal::Integer(_))
}

fn invalid_field_constraint<T>(
    field_name: &str,
    constraint: &str,
    message: &str,
) -> Result<T, ValidationError> {
    Err(ValidationError::InvalidConstraint {
        field: field_name.to_string(),
        constraint: constraint.to_string(),
        message: message.to_string(),
    })
}

fn validate_metadata_annotations(
    metadata: &[Metadata],
    target: &str,
    allow_pack: bool,
    allow_datasource: bool,
    allow_table_annotations: bool,
    allow_search: bool,
) -> Result<(), ValidationError> {
    for meta in metadata {
        if let Metadata::Annotation(annotation) = meta {
            if annotation.name.as_deref() == Some("cache") {
                validate_cache_annotation(&annotation.args, target)?;
            } else if annotation.name.as_deref() == Some("pack") {
                if !allow_pack {
                    return invalid_pack(
                        target,
                        "target",
                        "@pack can only be used on embed definitions",
                    );
                }
                validate_pack_annotation(&annotation.args, target)?;
            } else if annotation.name.as_deref() == Some("datasource") {
                if !allow_datasource {
                    return invalid_datasource(
                        target,
                        "target",
                        "@datasource can only be used on namespace or table definitions",
                    );
                }
                validate_datasource_annotation(&annotation.args, target)?;
            } else if annotation.name.as_deref() == Some("readonly") {
                if !allow_table_annotations {
                    return invalid_readonly(
                        target,
                        "target",
                        "@readonly can only be used on table definitions",
                    );
                }
                validate_readonly_annotation(&annotation.args, target)?;
            } else if annotation.name.as_deref() == Some("soft_delete") {
                if !allow_table_annotations {
                    return invalid_soft_delete(
                        target,
                        "target",
                        "@soft_delete can only be used on table definitions",
                    );
                }
                validate_soft_delete_annotation(&annotation.args, target)?;
            } else if annotation.name.as_deref() == Some("index") {
                if !allow_table_annotations {
                    return invalid_index(
                        target,
                        "target",
                        "@index can only be used on table definitions",
                    );
                }
                validate_index_annotation(&annotation.args, target)?;
            } else if annotation.name.as_deref() == Some("load") {
                if !allow_table_annotations {
                    return invalid_load(
                        target,
                        "target",
                        "@load can only be used on table definitions",
                    );
                }
                validate_load_annotation(&annotation.args, target)?;
            } else if annotation.name.as_deref() == Some("taggable") {
                if !allow_table_annotations {
                    return invalid_taggable(
                        target,
                        "target",
                        "@taggable can only be used on table definitions",
                    );
                }
                validate_taggable_annotation(&annotation.args, target)?;
            } else if annotation.name.as_deref() == Some("link_rows") {
                if !allow_table_annotations {
                    return invalid_link_rows(
                        target,
                        "target",
                        "@link_rows can only be used on table definitions",
                    );
                }
                validate_link_rows_annotation(&annotation.args, target)?;
            } else if annotation.name.as_deref() == Some("search") {
                if !allow_search {
                    return invalid_search(
                        target,
                        "target",
                        "@search can only be used on searchable fields",
                    );
                }
                validate_search_annotation(&annotation.args, target)?;
            }
        }
    }
    Ok(())
}

fn validate_cache_annotation(args: &[AnnotationArg], target: &str) -> Result<(), ValidationError> {
    let mut has_strategy = false;
    let mut has_ttl = false;

    for arg in args {
        match arg {
            AnnotationArg::Positional(literal) => {
                if has_strategy {
                    return invalid_cache(
                        target,
                        "strategy",
                        "strategy is specified more than once",
                    );
                }
                validate_cache_strategy_literal(literal, target)?;
                has_strategy = true;
            }
            AnnotationArg::Named(param) => match param.key.as_str() {
                "strategy" => {
                    if has_strategy {
                        return invalid_cache(
                            target,
                            "strategy",
                            "strategy is specified more than once",
                        );
                    }
                    validate_cache_strategy_literal(&param.value, target)?;
                    has_strategy = true;
                }
                "ttl" => {
                    if has_ttl {
                        return invalid_cache(target, "ttl", "ttl is specified more than once");
                    }
                    validate_cache_ttl_literal(&param.value, target)?;
                    has_ttl = true;
                }
                key => {
                    return invalid_cache(
                        target,
                        key,
                        "unsupported @cache parameter; expected 'strategy' or 'ttl'",
                    );
                }
            },
        }
    }

    Ok(())
}

fn validate_cache_strategy_literal(literal: &Literal, target: &str) -> Result<(), ValidationError> {
    let strategy = match literal {
        Literal::Identifier(value) | Literal::String(value) => value.as_str(),
        _ => {
            return invalid_cache(
                target,
                "strategy",
                "strategy must be an identifier or string literal",
            );
        }
    };

    match strategy {
        "full_load" | "on_demand" | "write_through" | "write_back" => Ok(()),
        _ => invalid_cache(
            target,
            "strategy",
            "unsupported strategy; expected one of full_load, on_demand, write_through, write_back",
        ),
    }
}

fn validate_cache_ttl_literal(literal: &Literal, target: &str) -> Result<(), ValidationError> {
    match literal {
        Literal::Integer(value) if *value >= 0 => Ok(()),
        Literal::Integer(_) => {
            invalid_cache(target, "ttl", "ttl must be greater than or equal to 0")
        }
        _ => invalid_cache(target, "ttl", "ttl must be an integer literal"),
    }
}

fn invalid_cache<T>(target: &str, part: &str, message: &str) -> Result<T, ValidationError> {
    Err(ValidationError::InvalidConstraint {
        field: target.to_string(),
        constraint: format!("@cache.{part}"),
        message: message.to_string(),
    })
}

fn validate_datasource_annotation(
    args: &[AnnotationArg],
    target: &str,
) -> Result<(), ValidationError> {
    if args.len() != 1 {
        return invalid_datasource(target, "value", "@datasource requires exactly one value");
    }

    let value = match &args[0] {
        AnnotationArg::Positional(literal) => validate_datasource_literal(literal, target)?,
        AnnotationArg::Named(param) if param.key == "value" => {
            validate_datasource_literal(&param.value, target)?
        }
        AnnotationArg::Named(param) => {
            return invalid_datasource(
                target,
                &param.key,
                "unsupported @datasource parameter; expected positional value or 'value'",
            );
        }
    };

    match value {
        "sqlite" | "mysql" | "mariadb" | "postgresql" | "postgres" | "redis" | "cache" => Ok(()),
        _ => invalid_datasource(
            target,
            "value",
            "unsupported datasource; expected one of sqlite, mysql, mariadb, postgresql, postgres, redis, cache",
        ),
    }
}

fn validate_datasource_literal<'a>(
    literal: &'a Literal,
    target: &str,
) -> Result<&'a str, ValidationError> {
    match literal {
        Literal::String(value) | Literal::Identifier(value) => Ok(value.as_str()),
        _ => invalid_datasource(
            target,
            "value",
            "datasource must be an identifier or string literal",
        ),
    }
}

fn invalid_datasource<T>(target: &str, part: &str, message: &str) -> Result<T, ValidationError> {
    Err(ValidationError::InvalidConstraint {
        field: target.to_string(),
        constraint: format!("@datasource.{part}"),
        message: message.to_string(),
    })
}

fn validate_readonly_annotation(
    args: &[AnnotationArg],
    target: &str,
) -> Result<(), ValidationError> {
    if args.is_empty() {
        Ok(())
    } else {
        invalid_readonly(target, "args", "@readonly does not accept arguments")
    }
}

fn invalid_readonly<T>(target: &str, part: &str, message: &str) -> Result<T, ValidationError> {
    Err(ValidationError::InvalidConstraint {
        field: target.to_string(),
        constraint: format!("@readonly.{part}"),
        message: message.to_string(),
    })
}

fn validate_soft_delete_annotation(
    args: &[AnnotationArg],
    target: &str,
) -> Result<(), ValidationError> {
    soft_delete_field_name(args, target).map(|_| ())
}

fn soft_delete_field_name<'a>(
    args: &'a [AnnotationArg],
    target: &str,
) -> Result<&'a str, ValidationError> {
    if args.len() != 1 {
        return invalid_soft_delete(
            target,
            "field",
            "@soft_delete requires exactly one field name",
        );
    }

    let literal = match &args[0] {
        AnnotationArg::Positional(literal) => literal,
        AnnotationArg::Named(param) if param.key == "field" => &param.value,
        AnnotationArg::Named(param) => {
            return invalid_soft_delete(
                target,
                &param.key,
                "unsupported @soft_delete parameter; expected positional field name or 'field'",
            );
        }
    };

    match literal {
        Literal::String(value) | Literal::Identifier(value) => Ok(value.as_str()),
        _ => invalid_soft_delete(
            target,
            "field",
            "soft delete field must be an identifier or string literal",
        ),
    }
}

fn validate_soft_delete_field(
    args: &[AnnotationArg],
    table: &crate::ast_model::Table,
    target: &str,
) -> Result<(), ValidationError> {
    let field_name = soft_delete_field_name(args, target)?;

    let Some(field) = table.members.iter().find_map(|member| {
        if let TableMember::Field(FieldDefinition::Regular(rf)) = member {
            if rf.name.as_deref() == Some(field_name) {
                return Some(rf);
            }
        }
        None
    }) else {
        return invalid_soft_delete(
            target,
            "field",
            "soft delete field must reference a regular field on the same table",
        );
    };

    let is_optional_timestamp = matches!(
        field.field_type.base_type,
        TypeName::Basic(BasicType::Timestamp)
    ) && matches!(
        field.field_type.cardinality,
        Some(crate::ast_model::Cardinality::Optional)
    );

    if !is_optional_timestamp {
        return invalid_soft_delete(
            target,
            "field",
            "soft delete field must have type timestamp?",
        );
    }

    Ok(())
}

fn validate_soft_delete_fields(
    metadata: &[Metadata],
    table: &crate::ast_model::Table,
    target: &str,
) -> Result<(), ValidationError> {
    for meta in metadata {
        if let Metadata::Annotation(annotation) = meta {
            if annotation.name.as_deref() == Some("soft_delete") {
                validate_soft_delete_field(&annotation.args, table, target)?;
            }
        }
    }
    Ok(())
}

fn invalid_soft_delete<T>(target: &str, part: &str, message: &str) -> Result<T, ValidationError> {
    Err(ValidationError::InvalidConstraint {
        field: target.to_string(),
        constraint: format!("@soft_delete.{part}"),
        message: message.to_string(),
    })
}

fn validate_index_annotation(args: &[AnnotationArg], target: &str) -> Result<(), ValidationError> {
    let mut field_count = 0;
    let mut has_unique = false;

    for arg in args {
        match arg {
            AnnotationArg::Positional(literal) => {
                index_field_name(literal, target)?;
                field_count += 1;
            }
            AnnotationArg::Named(param) => {
                if param.key != "unique" {
                    return invalid_index(
                        target,
                        &param.key,
                        "unsupported @index parameter; expected field names and optional 'unique'",
                    );
                }
                if has_unique {
                    return invalid_index(target, "unique", "unique is specified more than once");
                }
                validate_index_unique_literal(&param.value, target)?;
                has_unique = true;
            }
        }
    }

    if field_count == 0 {
        return invalid_index(target, "fields", "@index requires at least one field name");
    }

    Ok(())
}

fn validate_index_unique_literal(literal: &Literal, target: &str) -> Result<(), ValidationError> {
    match literal {
        Literal::Boolean(_) => Ok(()),
        Literal::Integer(0 | 1) => Ok(()),
        Literal::String(value) | Literal::Identifier(value)
            if matches!(value.as_str(), "true" | "false" | "1" | "0") =>
        {
            Ok(())
        }
        _ => invalid_index(
            target,
            "unique",
            "unique must be a boolean literal or one of true, false, 1, 0",
        ),
    }
}

fn index_field_name<'a>(literal: &'a Literal, target: &str) -> Result<&'a str, ValidationError> {
    match literal {
        Literal::Identifier(value) | Literal::String(value) => Ok(value.as_str()),
        _ => invalid_index(
            target,
            "fields",
            "index field names must be identifiers or string literals",
        ),
    }
}

fn validate_index_fields(
    metadata: &[Metadata],
    table: &crate::ast_model::Table,
    target: &str,
    current_scope: &[String],
    registry: &TypeRegistry,
) -> Result<(), ValidationError> {
    let table_fields: HashMap<&str, &TypeWithCardinality> = table
        .members
        .iter()
        .filter_map(|member| {
            if let TableMember::Field(FieldDefinition::Regular(rf)) = member {
                return rf.name.as_deref().map(|name| (name, &rf.field_type));
            }
            None
        })
        .collect();

    for meta in metadata {
        if let Metadata::Annotation(annotation) = meta {
            if annotation.name.as_deref() == Some("index") {
                let mut seen = HashSet::new();
                for arg in &annotation.args {
                    if let AnnotationArg::Positional(literal) = arg {
                        let field_name = index_field_name(literal, target)?;
                        if !seen.insert(field_name) {
                            return invalid_index(
                                target,
                                "fields",
                                "index field names must be unique within one @index annotation",
                            );
                        }
                        let Some(field_type) = table_fields.get(field_name) else {
                            return invalid_index(
                                target,
                                "fields",
                                "index field must reference a regular field on the same table",
                            );
                        };
                        validate_index_field_type(
                            target,
                            field_name,
                            field_type,
                            current_scope,
                            registry,
                        )?;
                    }
                }
            }
        }
    }

    Ok(())
}

fn validate_index_field_type(
    target: &str,
    _field_name: &str,
    field_type: &TypeWithCardinality,
    current_scope: &[String],
    registry: &TypeRegistry,
) -> Result<(), ValidationError> {
    if matches!(field_type.cardinality, Some(Cardinality::Array)) {
        return invalid_index(target, "fields", "index field cannot be an array field");
    }

    match &field_type.base_type {
        TypeName::Basic(BasicType::Bytes) => {
            invalid_index(target, "fields", "index field cannot be a bytes field")
        }
        TypeName::Basic(_) | TypeName::InlineEnum(_) => Ok(()),
        TypeName::Path(path) => {
            let Some(fqn) = resolve_type_path(path, current_scope, registry) else {
                return invalid_index(target, "fields", "index field type could not be resolved");
            };

            match registry.get_kind(&fqn) {
                Some(TypeKind::Enum) => Ok(()),
                Some(TypeKind::Struct | TypeKind::Embed) => invalid_index(
                    target,
                    "fields",
                    "index field cannot be a struct or embed field",
                ),
                None => invalid_index(target, "fields", "index field type could not be resolved"),
            }
        }
    }
}

fn invalid_index<T>(target: &str, part: &str, message: &str) -> Result<T, ValidationError> {
    Err(ValidationError::InvalidConstraint {
        field: target.to_string(),
        constraint: format!("@index.{part}"),
        message: message.to_string(),
    })
}

fn validate_load_annotation(args: &[AnnotationArg], target: &str) -> Result<(), ValidationError> {
    let mut has_path = false;
    let mut seen = HashSet::new();

    for arg in args {
        match arg {
            AnnotationArg::Positional(_) => {
                return invalid_load(target, "args", "@load only accepts named parameters");
            }
            AnnotationArg::Named(param) => {
                if !matches!(param.key.as_str(), "csv" | "json") {
                    return invalid_load(
                        target,
                        &param.key,
                        "unsupported @load parameter; expected 'csv' or 'json'",
                    );
                }
                if !seen.insert(param.key.as_str()) {
                    return invalid_load(
                        target,
                        &param.key,
                        "@load parameter is specified more than once",
                    );
                }
                validate_non_empty_string_literal(&param.value, target, "@load", &param.key)?;
                has_path = true;
            }
        }
    }

    if has_path {
        Ok(())
    } else {
        invalid_load(
            target,
            "args",
            "@load requires at least one of 'csv' or 'json'",
        )
    }
}

fn invalid_load<T>(target: &str, part: &str, message: &str) -> Result<T, ValidationError> {
    Err(ValidationError::InvalidConstraint {
        field: target.to_string(),
        constraint: format!("@load.{part}"),
        message: message.to_string(),
    })
}

fn validate_taggable_annotation(
    args: &[AnnotationArg],
    target: &str,
) -> Result<(), ValidationError> {
    if args.is_empty() {
        Ok(())
    } else {
        invalid_taggable(target, "args", "@taggable does not accept arguments")
    }
}

fn invalid_taggable<T>(target: &str, part: &str, message: &str) -> Result<T, ValidationError> {
    Err(ValidationError::InvalidConstraint {
        field: target.to_string(),
        constraint: format!("@taggable.{part}"),
        message: message.to_string(),
    })
}

fn validate_link_rows_annotation(
    args: &[AnnotationArg],
    target: &str,
) -> Result<(), ValidationError> {
    if args.len() != 1 {
        return invalid_link_rows(
            target,
            "target",
            "@link_rows requires exactly one target type",
        );
    }

    match &args[0] {
        AnnotationArg::Positional(literal) => {
            validate_non_empty_name_literal(literal, target, "@link_rows", "target")
        }
        AnnotationArg::Named(param) => invalid_link_rows(
            target,
            &param.key,
            "unsupported @link_rows parameter; expected positional target type",
        ),
    }
}

fn invalid_link_rows<T>(target: &str, part: &str, message: &str) -> Result<T, ValidationError> {
    Err(ValidationError::InvalidConstraint {
        field: target.to_string(),
        constraint: format!("@link_rows.{part}"),
        message: message.to_string(),
    })
}

fn validate_non_empty_string_literal(
    literal: &Literal,
    target: &str,
    annotation: &str,
    part: &str,
) -> Result<(), ValidationError> {
    match literal {
        Literal::String(value) if !value.trim().is_empty() => Ok(()),
        Literal::String(_) => Err(ValidationError::InvalidConstraint {
            field: target.to_string(),
            constraint: format!("{annotation}.{part}"),
            message: "value must not be empty".to_string(),
        }),
        _ => Err(ValidationError::InvalidConstraint {
            field: target.to_string(),
            constraint: format!("{annotation}.{part}"),
            message: "value must be a string literal".to_string(),
        }),
    }
}

fn validate_non_empty_name_literal(
    literal: &Literal,
    target: &str,
    annotation: &str,
    part: &str,
) -> Result<(), ValidationError> {
    match literal {
        Literal::Identifier(value) | Literal::String(value) if !value.trim().is_empty() => Ok(()),
        Literal::Identifier(_) | Literal::String(_) => Err(ValidationError::InvalidConstraint {
            field: target.to_string(),
            constraint: format!("{annotation}.{part}"),
            message: "value must not be empty".to_string(),
        }),
        _ => Err(ValidationError::InvalidConstraint {
            field: target.to_string(),
            constraint: format!("{annotation}.{part}"),
            message: "value must be an identifier or string literal".to_string(),
        }),
    }
}

fn validate_pack_annotation(args: &[AnnotationArg], target: &str) -> Result<(), ValidationError> {
    let mut has_separator = false;

    for arg in args {
        match arg {
            AnnotationArg::Positional(_) => {
                return invalid_pack(
                    target,
                    "separator",
                    "@pack only accepts named parameter 'separator'",
                );
            }
            AnnotationArg::Named(param) => {
                if param.key != "separator" {
                    return invalid_pack(
                        target,
                        &param.key,
                        "unsupported @pack parameter; expected 'separator'",
                    );
                }
                if has_separator {
                    return invalid_pack(
                        target,
                        "separator",
                        "separator is specified more than once",
                    );
                }
                validate_pack_separator_literal(&param.value, target)?;
                has_separator = true;
            }
        }
    }

    Ok(())
}

fn validate_pack_separator_literal(literal: &Literal, target: &str) -> Result<(), ValidationError> {
    let separator = match literal {
        Literal::String(value) => value,
        _ => {
            return invalid_pack(target, "separator", "separator must be a string literal");
        }
    };

    if separator.chars().count() != 1 {
        return invalid_pack(
            target,
            "separator",
            "separator must contain exactly one character",
        );
    }

    if matches!(separator.chars().next(), Some('\'' | '\\')) {
        return invalid_pack(
            target,
            "separator",
            "separator cannot be a single quote or backslash",
        );
    }

    Ok(())
}

fn invalid_pack<T>(target: &str, part: &str, message: &str) -> Result<T, ValidationError> {
    Err(ValidationError::InvalidConstraint {
        field: target.to_string(),
        constraint: format!("@pack.{part}"),
        message: message.to_string(),
    })
}

#[derive(Clone, Copy)]
enum SearchFieldKind {
    String,
    Exact,
    Unsupported,
}

fn validate_search_annotation(args: &[AnnotationArg], target: &str) -> Result<(), ValidationError> {
    let mut has_mode = false;
    let mut seen = HashSet::new();

    for arg in args {
        match arg {
            AnnotationArg::Positional(literal) => {
                if has_mode {
                    return invalid_search(target, "mode", "mode is specified more than once");
                }
                validate_search_identifier(literal, target, "mode")?;
                has_mode = true;
            }
            AnnotationArg::Named(param) => {
                if !seen.insert(param.key.as_str()) {
                    return invalid_search(
                        target,
                        &param.key,
                        "search parameter is specified more than once",
                    );
                }

                match param.key.as_str() {
                    "mode" => {
                        if has_mode {
                            return invalid_search(
                                target,
                                "mode",
                                "mode is specified more than once",
                            );
                        }
                        validate_search_identifier(&param.value, target, "mode")?;
                        has_mode = true;
                    }
                    "n" | "min" => validate_positive_integer(&param.value, target, &param.key)?,
                    "normalize" => validate_search_identifier(&param.value, target, "normalize")?,
                    "name" => validate_search_identifier(&param.value, target, "name")?,
                    "target" => validate_search_identifier(&param.value, target, "target")?,
                    key => {
                        return invalid_search(
                            target,
                            key,
                            "unsupported @search parameter; expected mode, n, min, normalize, name, or target",
                        );
                    }
                }
            }
        }
    }

    Ok(())
}

fn validate_search_identifier(
    literal: &Literal,
    target: &str,
    part: &str,
) -> Result<(), ValidationError> {
    match literal {
        Literal::Identifier(_) | Literal::String(_) => Ok(()),
        _ => invalid_search(
            target,
            part,
            "value must be an identifier or string literal",
        ),
    }
}

fn validate_positive_integer(
    literal: &Literal,
    target: &str,
    part: &str,
) -> Result<(), ValidationError> {
    match literal {
        Literal::Integer(value) if *value > 0 => Ok(()),
        Literal::Integer(_) => invalid_search(target, part, "value must be greater than 0"),
        _ => invalid_search(target, part, "value must be an integer literal"),
    }
}

fn search_literal_value<'a>(
    literal: &'a Literal,
    target: &str,
    part: &str,
) -> Result<&'a str, ValidationError> {
    match literal {
        Literal::Identifier(value) | Literal::String(value) => Ok(value.as_str()),
        _ => invalid_search(
            target,
            part,
            "value must be an identifier or string literal",
        ),
    }
}

fn validate_regular_field_search_annotations(
    metadata: &[Metadata],
    target: &str,
    field_type: &TypeWithCardinality,
    current_scope: &[String],
    registry: &TypeRegistry,
) -> Result<(), ValidationError> {
    let kind = search_field_kind(field_type, current_scope, registry);
    validate_search_annotations_for_kind(metadata, target, kind)
}

fn search_field_kind(
    field_type: &TypeWithCardinality,
    current_scope: &[String],
    registry: &TypeRegistry,
) -> SearchFieldKind {
    if matches!(field_type.cardinality, Some(Cardinality::Array)) {
        return SearchFieldKind::Unsupported;
    }

    match &field_type.base_type {
        TypeName::Basic(BasicType::String) => SearchFieldKind::String,
        TypeName::Basic(
            BasicType::U8
            | BasicType::U16
            | BasicType::U32
            | BasicType::U64
            | BasicType::I8
            | BasicType::I16
            | BasicType::I32
            | BasicType::I64
            | BasicType::Bool
            | BasicType::Timestamp,
        ) => SearchFieldKind::Exact,
        TypeName::Basic(BasicType::F32 | BasicType::F64 | BasicType::Bytes) => {
            SearchFieldKind::Unsupported
        }
        TypeName::InlineEnum(_) => SearchFieldKind::Exact,
        TypeName::Path(type_path) => resolve_type_path(type_path, current_scope, registry)
            .and_then(|fqn| registry.get_kind(&fqn))
            .map(|kind| {
                if kind == TypeKind::Enum {
                    SearchFieldKind::Exact
                } else {
                    SearchFieldKind::Unsupported
                }
            })
            .unwrap_or(SearchFieldKind::Unsupported),
    }
}

fn validate_search_annotations_for_kind(
    metadata: &[Metadata],
    target: &str,
    kind: SearchFieldKind,
) -> Result<(), ValidationError> {
    let mut count = 0;

    for meta in metadata {
        if let Metadata::Annotation(annotation) = meta {
            if annotation.name.as_deref() != Some("search") {
                continue;
            }

            count += 1;
            if count > 1 {
                return invalid_search(target, "args", "@search is specified more than once");
            }

            if matches!(kind, SearchFieldKind::Unsupported) {
                return invalid_search(
                    target,
                    "target",
                    "@search is not supported for this field type",
                );
            }

            let mut mode = match kind {
                SearchFieldKind::String => "ngram",
                SearchFieldKind::Exact => "exact",
                SearchFieldKind::Unsupported => unreachable!(),
            };
            let mut has_n = false;
            let mut has_min = false;
            let mut has_normalize = false;
            let mut target_value = "csharp";

            for arg in &annotation.args {
                match arg {
                    AnnotationArg::Positional(literal) => {
                        mode = search_literal_value(literal, target, "mode")?;
                    }
                    AnnotationArg::Named(param) => match param.key.as_str() {
                        "mode" => mode = search_literal_value(&param.value, target, "mode")?,
                        "n" => has_n = true,
                        "min" => has_min = true,
                        "normalize" => has_normalize = true,
                        "target" => {
                            target_value = search_literal_value(&param.value, target, "target")?
                        }
                        "name" => {}
                        _ => {}
                    },
                }
            }

            match mode {
                "exact" => {}
                "ngram" | "word" if matches!(kind, SearchFieldKind::String) => {}
                "ngram" | "word" => {
                    return invalid_search(
                        target,
                        "mode",
                        "ngram and word search modes are only supported for string fields",
                    );
                }
                _ => {
                    return invalid_search(
                        target,
                        "mode",
                        "unsupported search mode; expected exact, ngram, or word",
                    );
                }
            }

            if has_n && mode != "ngram" {
                return invalid_search(target, "n", "n can only be used with ngram mode");
            }
            if has_min && matches!(kind, SearchFieldKind::Exact) {
                return invalid_search(
                    target,
                    "min",
                    "min can only be used with string search modes",
                );
            }
            if has_normalize && matches!(kind, SearchFieldKind::Exact) {
                return invalid_search(
                    target,
                    "normalize",
                    "normalize can only be used with string search modes",
                );
            }
            if !matches!(
                target_value,
                "csharp"
                    | "csharp_binary_ref"
                    | "csharp_container"
                    | "rust"
                    | "rust_container"
                    | "cpp"
                    | "cpp_container"
                    | "cpp_binary_ref"
                    | "typescript"
                    | "typescript_container"
                    | "typescript_binary_ref"
                    | "go"
                    | "go_container"
                    | "go_binary_ref"
                    | "python"
                    | "python_container"
                    | "kotlin"
                    | "kotlin_container"
                    | "swift"
                    | "swift_container"
                    | "unreal"
                    | "unreal_registry"
            ) {
                return invalid_search(
                    target,
                    "target",
                    "unsupported search target; expected csharp, csharp_binary_ref, csharp_container, rust, rust_container, cpp, cpp_container, cpp_binary_ref, typescript, typescript_container, typescript_binary_ref, go, go_container, go_binary_ref, python, python_container, kotlin, kotlin_container, swift, swift_container, unreal, or unreal_registry",
                );
            }
        }
    }

    Ok(())
}

fn invalid_search<T>(target: &str, part: &str, message: &str) -> Result<T, ValidationError> {
    Err(ValidationError::InvalidConstraint {
        field: target.to_string(),
        constraint: format!("@search.{part}"),
        message: message.to_string(),
    })
}

fn validate_member_annotations(
    members: &[TableMember],
    path: &mut Vec<String>,
    registry: &TypeRegistry,
) -> Result<(), ValidationError> {
    for member in members {
        match member {
            TableMember::Field(field) => match field {
                FieldDefinition::Regular(rf) => {
                    let target = rf
                        .name
                        .as_deref()
                        .map(|name| qualified_name(path, name))
                        .unwrap_or_else(|| "<anonymous field>".to_string());
                    validate_metadata_annotations(
                        &rf.metadata,
                        &target,
                        false,
                        false,
                        false,
                        true,
                    )?;
                    validate_regular_field_search_annotations(
                        &rf.metadata,
                        &target,
                        &rf.field_type,
                        path,
                        registry,
                    )?;
                }
                FieldDefinition::InlineEmbed(ief) => {
                    let inline_name = required_name(&ief.name, "inline embed")?;
                    let target = qualified_name(path, inline_name);
                    validate_metadata_annotations(
                        &ief.metadata,
                        &target,
                        false,
                        false,
                        false,
                        false,
                    )?;
                    path.push(inline_name.to_string());
                    validate_member_annotations(&ief.members, path, registry)?;
                    path.pop();
                }
                FieldDefinition::InlineEnum(e) => {
                    let target = e
                        .name
                        .as_deref()
                        .map(|name| qualified_name(path, name))
                        .unwrap_or_else(|| "<anonymous enum field>".to_string());
                    validate_metadata_annotations(&e.metadata, &target, false, false, false, true)?;
                    validate_search_annotations_for_kind(
                        &e.metadata,
                        &target,
                        SearchFieldKind::Exact,
                    )?;
                }
            },
            TableMember::Embed(e) => {
                let embed_name = required_name(&e.name, "embed")?;
                let target = qualified_name(path, embed_name);
                validate_metadata_annotations(&e.metadata, &target, true, false, false, false)?;
                path.push(embed_name.to_string());
                validate_member_annotations(&e.members, path, registry)?;
                path.pop();
            }
            TableMember::Enum(e) => {
                let target = e
                    .name
                    .as_deref()
                    .map(|name| qualified_name(path, name))
                    .unwrap_or_else(|| "<anonymous enum>".to_string());
                validate_metadata_annotations(&e.metadata, &target, false, false, false, false)?;
            }
            TableMember::Comment(_) => {}
        }
    }
    Ok(())
}

fn validate_all_annotations(
    definitions: &[Definition],
    path: &mut Vec<String>,
    registry: &TypeRegistry,
) -> Result<(), ValidationError> {
    for def in definitions {
        match def {
            Definition::Namespace(ns) => {
                path.extend(ns.path.iter().cloned());
                let target = path.join(".");
                validate_metadata_annotations(&ns.metadata, &target, false, true, false, false)?;
                validate_all_annotations(&ns.definitions, path, registry)?;
                for _ in 0..ns.path.len() {
                    path.pop();
                }
            }
            Definition::Table(t) => {
                let table_name = required_name(&t.name, "table")?;
                let target = qualified_name(path, table_name);
                validate_metadata_annotations(&t.metadata, &target, false, true, true, false)?;
                validate_soft_delete_fields(&t.metadata, t, &target)?;
                path.push(table_name.to_string());
                validate_index_fields(&t.metadata, t, &target, path, registry)?;
                validate_member_annotations(&t.members, path, registry)?;
                path.pop();
            }
            Definition::Enum(e) => {
                let target = e
                    .name
                    .as_deref()
                    .map(|name| qualified_name(path, name))
                    .unwrap_or_else(|| "<anonymous enum>".to_string());
                validate_metadata_annotations(&e.metadata, &target, false, false, false, false)?;
            }
            Definition::Embed(e) => {
                let embed_name = required_name(&e.name, "embed")?;
                let target = qualified_name(path, embed_name);
                validate_metadata_annotations(&e.metadata, &target, true, false, false, false)?;
                path.push(embed_name.to_string());
                validate_member_annotations(&e.members, path, registry)?;
                path.pop();
            }
            Definition::Comment(_) | Definition::Annotation(_) => {}
        }
    }
    Ok(())
}

/// 재귀적으로 모든 필드를 순회하며 사용된 타입의 유효성을 검사합니다.
fn validate_all_types(
    definitions: &[Definition],
    path: &mut Vec<String>,
    registry: &TypeRegistry,
    field_registry: &FieldRegistry<'_>,
) -> Result<(), ValidationError> {
    for def in definitions {
        match def {
            Definition::Namespace(ns) => {
                path.extend(ns.path.iter().cloned());
                validate_all_types(&ns.definitions, path, registry, field_registry)?;
                for _ in 0..ns.path.len() {
                    path.pop();
                }
            }
            Definition::Table(t) => {
                push_required_name(path, &t.name, "table")?;
                validate_table_members(&t.members, path, registry, field_registry)?;
                path.pop();
            }
            Definition::Enum(_) => { /* Enums do not reference other types */ }
            Definition::Embed(e) => {
                // Validate types used within the embed's fields.
                push_required_name(path, &e.name, "embed")?;
                validate_table_members(&e.members, path, registry, field_registry)?;
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

    /// Helper to create a table with metadata.
    fn make_table_with_metadata(
        name: &str,
        metadata: Vec<Metadata>,
        members: Vec<TableMember>,
    ) -> Definition {
        Definition::Table(Table {
            metadata,
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

    /// Helper to create an embed with metadata.
    fn make_embed_with_metadata(
        name: &str,
        metadata: Vec<Metadata>,
        members: Vec<TableMember>,
    ) -> Definition {
        Definition::Embed(Embed {
            metadata,
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

    /// Helper to create a namespace with metadata.
    fn make_namespace_with_metadata(
        path: Vec<&str>,
        metadata: Vec<Metadata>,
        definitions: Vec<Definition>,
    ) -> Definition {
        Definition::Namespace(Namespace {
            metadata,
            path: path.into_iter().map(String::from).collect(),
            imports: vec![],
            definitions,
        })
    }

    fn cache_metadata(args: Vec<AnnotationArg>) -> Vec<Metadata> {
        vec![Metadata::Annotation(Annotation {
            name: Some("cache".to_string()),
            args,
        })]
    }

    fn pack_metadata(args: Vec<AnnotationArg>) -> Vec<Metadata> {
        vec![Metadata::Annotation(Annotation {
            name: Some("pack".to_string()),
            args,
        })]
    }

    fn datasource_metadata(args: Vec<AnnotationArg>) -> Vec<Metadata> {
        vec![Metadata::Annotation(Annotation {
            name: Some("datasource".to_string()),
            args,
        })]
    }

    fn readonly_metadata(args: Vec<AnnotationArg>) -> Vec<Metadata> {
        vec![Metadata::Annotation(Annotation {
            name: Some("readonly".to_string()),
            args,
        })]
    }

    fn soft_delete_metadata(args: Vec<AnnotationArg>) -> Vec<Metadata> {
        vec![Metadata::Annotation(Annotation {
            name: Some("soft_delete".to_string()),
            args,
        })]
    }

    fn index_metadata(args: Vec<AnnotationArg>) -> Vec<Metadata> {
        vec![Metadata::Annotation(Annotation {
            name: Some("index".to_string()),
            args,
        })]
    }

    fn load_metadata(args: Vec<AnnotationArg>) -> Vec<Metadata> {
        vec![Metadata::Annotation(Annotation {
            name: Some("load".to_string()),
            args,
        })]
    }

    fn taggable_metadata(args: Vec<AnnotationArg>) -> Vec<Metadata> {
        vec![Metadata::Annotation(Annotation {
            name: Some("taggable".to_string()),
            args,
        })]
    }

    fn link_rows_metadata(args: Vec<AnnotationArg>) -> Vec<Metadata> {
        vec![Metadata::Annotation(Annotation {
            name: Some("link_rows".to_string()),
            args,
        })]
    }

    fn named_arg(key: &str, value: Literal) -> AnnotationArg {
        AnnotationArg::Named(AnnotationParam {
            key: key.to_string(),
            value,
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

    /// Helper to create an optional regular field with a basic type
    fn make_optional_field_basic(name: &str, basic_type: BasicType) -> TableMember {
        TableMember::Field(FieldDefinition::Regular(RegularField {
            metadata: vec![],
            name: Some(name.to_string()),
            field_type: TypeWithCardinality {
                base_type: TypeName::Basic(basic_type),
                cardinality: Some(Cardinality::Optional),
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

    fn make_primary_key_field(name: &str, basic_type: BasicType) -> TableMember {
        make_field_with_constraints(name, basic_type, vec![Constraint::PrimaryKey])
    }

    /// Helper to create a regular field with an arbitrary type and constraints
    fn make_field_with_type_and_constraints(
        name: &str,
        base_type: TypeName,
        cardinality: Option<Cardinality>,
        constraints: Vec<Constraint>,
    ) -> TableMember {
        TableMember::Field(FieldDefinition::Regular(RegularField {
            metadata: vec![],
            name: Some(name.to_string()),
            field_type: TypeWithCardinality {
                base_type,
                cardinality,
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

    #[test]
    fn test_missing_table_name_fails() {
        let definitions = vec![Definition::Table(Table {
            metadata: vec![],
            name: None,
            members: vec![],
        })];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::MissingName {
                kind: "table".to_string(),
            })
        );
    }

    #[test]
    fn test_missing_inline_embed_name_fails() {
        let definitions = vec![make_table(
            "User",
            vec![TableMember::Field(FieldDefinition::InlineEmbed(
                InlineEmbedField {
                    metadata: vec![],
                    name: None,
                    members: vec![],
                    cardinality: None,
                    field_number: None,
                },
            ))],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::MissingName {
                kind: "inline embed".to_string(),
            })
        );
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

    #[test]
    fn test_cache_annotation_named_strategy_and_ttl_is_valid() {
        let definitions = vec![make_table_with_metadata(
            "Session",
            cache_metadata(vec![
                named_arg("strategy", Literal::Identifier("write_through".to_string())),
                named_arg("ttl", Literal::Integer(300)),
            ]),
            vec![],
        )];

        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_cache_annotation_positional_strategy_is_valid() {
        let definitions = vec![make_table_with_metadata(
            "Session",
            cache_metadata(vec![AnnotationArg::Positional(Literal::String(
                "full_load".to_string(),
            ))]),
            vec![],
        )];

        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_cache_annotation_on_namespace_is_valid() {
        let definitions = vec![make_namespace_with_metadata(
            vec!["game", "cache"],
            cache_metadata(vec![named_arg(
                "strategy",
                Literal::Identifier("on_demand".to_string()),
            )]),
            vec![make_table("Session", vec![])],
        )];

        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_cache_annotation_rejects_unknown_strategy() {
        let definitions = vec![make_table_with_metadata(
            "Session",
            cache_metadata(vec![named_arg(
                "strategy",
                Literal::Identifier("never".to_string()),
            )]),
            vec![],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Session".to_string(),
                constraint: "@cache.strategy".to_string(),
                message: "unsupported strategy; expected one of full_load, on_demand, write_through, write_back".to_string(),
            })
        );
    }

    #[test]
    fn test_cache_annotation_rejects_non_integer_ttl() {
        let definitions = vec![make_table_with_metadata(
            "Session",
            cache_metadata(vec![named_arg("ttl", Literal::String("300".to_string()))]),
            vec![],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Session".to_string(),
                constraint: "@cache.ttl".to_string(),
                message: "ttl must be an integer literal".to_string(),
            })
        );
    }

    #[test]
    fn test_cache_annotation_rejects_negative_ttl() {
        let definitions = vec![make_table_with_metadata(
            "Session",
            cache_metadata(vec![named_arg("ttl", Literal::Integer(-1))]),
            vec![],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Session".to_string(),
                constraint: "@cache.ttl".to_string(),
                message: "ttl must be greater than or equal to 0".to_string(),
            })
        );
    }

    #[test]
    fn test_cache_annotation_rejects_unknown_parameter() {
        let definitions = vec![make_table_with_metadata(
            "Session",
            cache_metadata(vec![named_arg("region", Literal::String("us".to_string()))]),
            vec![],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Session".to_string(),
                constraint: "@cache.region".to_string(),
                message: "unsupported @cache parameter; expected 'strategy' or 'ttl'".to_string(),
            })
        );
    }

    #[test]
    fn test_cache_annotation_rejects_duplicate_strategy() {
        let definitions = vec![make_table_with_metadata(
            "Session",
            cache_metadata(vec![
                AnnotationArg::Positional(Literal::Identifier("full_load".to_string())),
                named_arg("strategy", Literal::Identifier("write_back".to_string())),
            ]),
            vec![],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Session".to_string(),
                constraint: "@cache.strategy".to_string(),
                message: "strategy is specified more than once".to_string(),
            })
        );
    }

    #[test]
    fn test_datasource_annotation_positional_value_is_valid_on_table() {
        let definitions = vec![make_table_with_metadata(
            "Player",
            datasource_metadata(vec![AnnotationArg::Positional(Literal::String(
                "sqlite".to_string(),
            ))]),
            vec![],
        )];

        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_datasource_annotation_identifier_value_is_valid_on_namespace() {
        let definitions = vec![make_namespace_with_metadata(
            vec!["game", "cache"],
            datasource_metadata(vec![AnnotationArg::Positional(Literal::Identifier(
                "cache".to_string(),
            ))]),
            vec![make_table("Session", vec![])],
        )];

        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_datasource_annotation_named_value_is_valid() {
        let definitions = vec![make_table_with_metadata(
            "Player",
            datasource_metadata(vec![named_arg(
                "value",
                Literal::Identifier("postgres".to_string()),
            )]),
            vec![],
        )];

        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_datasource_annotation_rejects_embed_target() {
        let definitions = vec![make_embed_with_metadata(
            "Position",
            datasource_metadata(vec![AnnotationArg::Positional(Literal::String(
                "sqlite".to_string(),
            ))]),
            vec![],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Position".to_string(),
                constraint: "@datasource.target".to_string(),
                message: "@datasource can only be used on namespace or table definitions"
                    .to_string(),
            })
        );
    }

    #[test]
    fn test_datasource_annotation_rejects_missing_value() {
        let definitions = vec![make_table_with_metadata(
            "Player",
            datasource_metadata(vec![]),
            vec![],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Player".to_string(),
                constraint: "@datasource.value".to_string(),
                message: "@datasource requires exactly one value".to_string(),
            })
        );
    }

    #[test]
    fn test_datasource_annotation_rejects_multiple_values() {
        let definitions = vec![make_table_with_metadata(
            "Player",
            datasource_metadata(vec![
                AnnotationArg::Positional(Literal::String("sqlite".to_string())),
                AnnotationArg::Positional(Literal::String("redis".to_string())),
            ]),
            vec![],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Player".to_string(),
                constraint: "@datasource.value".to_string(),
                message: "@datasource requires exactly one value".to_string(),
            })
        );
    }

    #[test]
    fn test_datasource_annotation_rejects_unknown_parameter() {
        let definitions = vec![make_table_with_metadata(
            "Player",
            datasource_metadata(vec![named_arg(
                "name",
                Literal::String("sqlite".to_string()),
            )]),
            vec![],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Player".to_string(),
                constraint: "@datasource.name".to_string(),
                message: "unsupported @datasource parameter; expected positional value or 'value'"
                    .to_string(),
            })
        );
    }

    #[test]
    fn test_datasource_annotation_rejects_non_string_value() {
        let definitions = vec![make_table_with_metadata(
            "Player",
            datasource_metadata(vec![AnnotationArg::Positional(Literal::Integer(1))]),
            vec![],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Player".to_string(),
                constraint: "@datasource.value".to_string(),
                message: "datasource must be an identifier or string literal".to_string(),
            })
        );
    }

    #[test]
    fn test_datasource_annotation_rejects_unknown_datasource() {
        let definitions = vec![make_table_with_metadata(
            "Player",
            datasource_metadata(vec![AnnotationArg::Positional(Literal::String(
                "oracle".to_string(),
            ))]),
            vec![],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Player".to_string(),
                constraint: "@datasource.value".to_string(),
                message: "unsupported datasource; expected one of sqlite, mysql, mariadb, postgresql, postgres, redis, cache".to_string(),
            })
        );
    }

    #[test]
    fn test_readonly_annotation_is_valid_on_table() {
        let definitions = vec![make_table_with_metadata(
            "Item",
            readonly_metadata(vec![]),
            vec![],
        )];

        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_readonly_annotation_rejects_arguments() {
        let definitions = vec![make_table_with_metadata(
            "Item",
            readonly_metadata(vec![AnnotationArg::Positional(Literal::Boolean(true))]),
            vec![],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Item".to_string(),
                constraint: "@readonly.args".to_string(),
                message: "@readonly does not accept arguments".to_string(),
            })
        );
    }

    #[test]
    fn test_readonly_annotation_rejects_embed_target() {
        let definitions = vec![make_embed_with_metadata(
            "Stats",
            readonly_metadata(vec![]),
            vec![],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Stats".to_string(),
                constraint: "@readonly.target".to_string(),
                message: "@readonly can only be used on table definitions".to_string(),
            })
        );
    }

    #[test]
    fn test_soft_delete_annotation_is_valid_on_table_with_timestamp_optional_field() {
        let definitions = vec![make_table_with_metadata(
            "Player",
            soft_delete_metadata(vec![AnnotationArg::Positional(Literal::String(
                "deleted_at".to_string(),
            ))]),
            vec![make_optional_field_basic(
                "deleted_at",
                BasicType::Timestamp,
            )],
        )];

        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_soft_delete_annotation_named_field_is_valid() {
        let definitions = vec![make_table_with_metadata(
            "Player",
            soft_delete_metadata(vec![named_arg(
                "field",
                Literal::Identifier("deleted_at".to_string()),
            )]),
            vec![make_optional_field_basic(
                "deleted_at",
                BasicType::Timestamp,
            )],
        )];

        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_soft_delete_annotation_rejects_embed_target() {
        let definitions = vec![make_embed_with_metadata(
            "Stats",
            soft_delete_metadata(vec![AnnotationArg::Positional(Literal::String(
                "deleted_at".to_string(),
            ))]),
            vec![],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Stats".to_string(),
                constraint: "@soft_delete.target".to_string(),
                message: "@soft_delete can only be used on table definitions".to_string(),
            })
        );
    }

    #[test]
    fn test_soft_delete_annotation_rejects_missing_field_argument() {
        let definitions = vec![make_table_with_metadata(
            "Player",
            soft_delete_metadata(vec![]),
            vec![],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Player".to_string(),
                constraint: "@soft_delete.field".to_string(),
                message: "@soft_delete requires exactly one field name".to_string(),
            })
        );
    }

    #[test]
    fn test_soft_delete_annotation_rejects_unknown_parameter() {
        let definitions = vec![make_table_with_metadata(
            "Player",
            soft_delete_metadata(vec![named_arg(
                "column",
                Literal::String("deleted_at".to_string()),
            )]),
            vec![],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Player".to_string(),
                constraint: "@soft_delete.column".to_string(),
                message:
                    "unsupported @soft_delete parameter; expected positional field name or 'field'"
                        .to_string(),
            })
        );
    }

    #[test]
    fn test_soft_delete_annotation_rejects_non_string_field() {
        let definitions = vec![make_table_with_metadata(
            "Player",
            soft_delete_metadata(vec![AnnotationArg::Positional(Literal::Integer(1))]),
            vec![],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Player".to_string(),
                constraint: "@soft_delete.field".to_string(),
                message: "soft delete field must be an identifier or string literal".to_string(),
            })
        );
    }

    #[test]
    fn test_soft_delete_annotation_rejects_missing_table_field() {
        let definitions = vec![make_table_with_metadata(
            "Player",
            soft_delete_metadata(vec![AnnotationArg::Positional(Literal::String(
                "deleted_at".to_string(),
            ))]),
            vec![make_optional_field_basic(
                "removed_at",
                BasicType::Timestamp,
            )],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Player".to_string(),
                constraint: "@soft_delete.field".to_string(),
                message: "soft delete field must reference a regular field on the same table"
                    .to_string(),
            })
        );
    }

    #[test]
    fn test_soft_delete_annotation_rejects_non_timestamp_field() {
        let definitions = vec![make_table_with_metadata(
            "Player",
            soft_delete_metadata(vec![AnnotationArg::Positional(Literal::String(
                "deleted_at".to_string(),
            ))]),
            vec![make_optional_field_basic("deleted_at", BasicType::String)],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Player".to_string(),
                constraint: "@soft_delete.field".to_string(),
                message: "soft delete field must have type timestamp?".to_string(),
            })
        );
    }

    #[test]
    fn test_soft_delete_annotation_rejects_non_optional_timestamp_field() {
        let definitions = vec![make_table_with_metadata(
            "Player",
            soft_delete_metadata(vec![AnnotationArg::Positional(Literal::String(
                "deleted_at".to_string(),
            ))]),
            vec![make_field_basic("deleted_at", BasicType::Timestamp)],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Player".to_string(),
                constraint: "@soft_delete.field".to_string(),
                message: "soft delete field must have type timestamp?".to_string(),
            })
        );
    }

    #[test]
    fn test_index_annotation_single_field_is_valid() {
        let definitions = vec![make_table_with_metadata(
            "Player",
            index_metadata(vec![AnnotationArg::Positional(Literal::Identifier(
                "name".to_string(),
            ))]),
            vec![make_field_basic("name", BasicType::String)],
        )];

        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_index_annotation_composite_unique_is_valid() {
        let definitions = vec![make_table_with_metadata(
            "Player",
            index_metadata(vec![
                AnnotationArg::Positional(Literal::Identifier("guild_id".to_string())),
                AnnotationArg::Positional(Literal::Identifier("level".to_string())),
                named_arg("unique", Literal::Boolean(true)),
            ]),
            vec![
                make_field_basic("guild_id", BasicType::U32),
                make_field_basic("level", BasicType::U16),
            ],
        )];

        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_index_annotation_rejects_embed_target() {
        let definitions = vec![make_embed_with_metadata(
            "Stats",
            index_metadata(vec![AnnotationArg::Positional(Literal::Identifier(
                "hp".to_string(),
            ))]),
            vec![make_field_basic("hp", BasicType::U32)],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Stats".to_string(),
                constraint: "@index.target".to_string(),
                message: "@index can only be used on table definitions".to_string(),
            })
        );
    }

    #[test]
    fn test_index_annotation_rejects_missing_fields() {
        let definitions = vec![make_table_with_metadata(
            "Player",
            index_metadata(vec![named_arg("unique", Literal::Boolean(true))]),
            vec![make_field_basic("name", BasicType::String)],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Player".to_string(),
                constraint: "@index.fields".to_string(),
                message: "@index requires at least one field name".to_string(),
            })
        );
    }

    #[test]
    fn test_index_annotation_rejects_non_string_field_name() {
        let definitions = vec![make_table_with_metadata(
            "Player",
            index_metadata(vec![AnnotationArg::Positional(Literal::Integer(1))]),
            vec![make_field_basic("name", BasicType::String)],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Player".to_string(),
                constraint: "@index.fields".to_string(),
                message: "index field names must be identifiers or string literals".to_string(),
            })
        );
    }

    #[test]
    fn test_index_annotation_rejects_unknown_parameter() {
        let definitions = vec![make_table_with_metadata(
            "Player",
            index_metadata(vec![
                AnnotationArg::Positional(Literal::Identifier("name".to_string())),
                named_arg("order", Literal::String("asc".to_string())),
            ]),
            vec![make_field_basic("name", BasicType::String)],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Player".to_string(),
                constraint: "@index.order".to_string(),
                message: "unsupported @index parameter; expected field names and optional 'unique'"
                    .to_string(),
            })
        );
    }

    #[test]
    fn test_index_annotation_rejects_invalid_unique_value() {
        let definitions = vec![make_table_with_metadata(
            "Player",
            index_metadata(vec![
                AnnotationArg::Positional(Literal::Identifier("name".to_string())),
                named_arg("unique", Literal::String("yes".to_string())),
            ]),
            vec![make_field_basic("name", BasicType::String)],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Player".to_string(),
                constraint: "@index.unique".to_string(),
                message: "unique must be a boolean literal or one of true, false, 1, 0".to_string(),
            })
        );
    }

    #[test]
    fn test_index_annotation_rejects_duplicate_unique() {
        let definitions = vec![make_table_with_metadata(
            "Player",
            index_metadata(vec![
                AnnotationArg::Positional(Literal::Identifier("name".to_string())),
                named_arg("unique", Literal::Boolean(true)),
                named_arg("unique", Literal::Boolean(false)),
            ]),
            vec![make_field_basic("name", BasicType::String)],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Player".to_string(),
                constraint: "@index.unique".to_string(),
                message: "unique is specified more than once".to_string(),
            })
        );
    }

    #[test]
    fn test_index_annotation_rejects_missing_table_field() {
        let definitions = vec![make_table_with_metadata(
            "Player",
            index_metadata(vec![AnnotationArg::Positional(Literal::Identifier(
                "missing".to_string(),
            ))]),
            vec![make_field_basic("name", BasicType::String)],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Player".to_string(),
                constraint: "@index.fields".to_string(),
                message: "index field must reference a regular field on the same table".to_string(),
            })
        );
    }

    #[test]
    fn test_index_annotation_rejects_duplicate_field() {
        let definitions = vec![make_table_with_metadata(
            "Player",
            index_metadata(vec![
                AnnotationArg::Positional(Literal::Identifier("name".to_string())),
                AnnotationArg::Positional(Literal::Identifier("name".to_string())),
            ]),
            vec![make_field_basic("name", BasicType::String)],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Player".to_string(),
                constraint: "@index.fields".to_string(),
                message: "index field names must be unique within one @index annotation"
                    .to_string(),
            })
        );
    }

    #[test]
    fn test_index_annotation_allows_enum_field() {
        let definitions = vec![
            make_enum("Status"),
            make_table_with_metadata(
                "Player",
                index_metadata(vec![AnnotationArg::Positional(Literal::Identifier(
                    "status".to_string(),
                ))]),
                vec![make_field_with_type("status", vec!["Status"])],
            ),
        ];

        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_index_annotation_rejects_array_field() {
        let definitions = vec![make_table_with_metadata(
            "Player",
            index_metadata(vec![AnnotationArg::Positional(Literal::Identifier(
                "tags".to_string(),
            ))]),
            vec![make_field_with_type_and_constraints(
                "tags",
                TypeName::Basic(BasicType::String),
                Some(Cardinality::Array),
                vec![],
            )],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Player".to_string(),
                constraint: "@index.fields".to_string(),
                message: "index field cannot be an array field".to_string(),
            })
        );
    }

    #[test]
    fn test_index_annotation_rejects_bytes_field() {
        let definitions = vec![make_table_with_metadata(
            "Player",
            index_metadata(vec![AnnotationArg::Positional(Literal::Identifier(
                "payload".to_string(),
            ))]),
            vec![make_field_basic("payload", BasicType::Bytes)],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Player".to_string(),
                constraint: "@index.fields".to_string(),
                message: "index field cannot be a bytes field".to_string(),
            })
        );
    }

    #[test]
    fn test_index_annotation_rejects_embed_field() {
        let definitions = vec![
            make_embed("Stats", vec![make_field_basic("hp", BasicType::U32)]),
            make_table_with_metadata(
                "Player",
                index_metadata(vec![AnnotationArg::Positional(Literal::Identifier(
                    "stats".to_string(),
                ))]),
                vec![make_field_with_type("stats", vec!["Stats"])],
            ),
        ];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Player".to_string(),
                constraint: "@index.fields".to_string(),
                message: "index field cannot be a struct or embed field".to_string(),
            })
        );
    }

    #[test]
    fn test_load_annotation_csv_and_json_are_valid() {
        let definitions = vec![make_table_with_metadata(
            "Player",
            load_metadata(vec![
                named_arg("csv", Literal::String("data/players.csv".to_string())),
                named_arg("json", Literal::String("data/players.json".to_string())),
            ]),
            vec![],
        )];

        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_load_annotation_rejects_embed_target() {
        let definitions = vec![make_embed_with_metadata(
            "Stats",
            load_metadata(vec![named_arg(
                "csv",
                Literal::String("data/stats.csv".to_string()),
            )]),
            vec![],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Stats".to_string(),
                constraint: "@load.target".to_string(),
                message: "@load can only be used on table definitions".to_string(),
            })
        );
    }

    #[test]
    fn test_load_annotation_rejects_missing_path() {
        let definitions = vec![make_table_with_metadata(
            "Player",
            load_metadata(vec![]),
            vec![],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Player".to_string(),
                constraint: "@load.args".to_string(),
                message: "@load requires at least one of 'csv' or 'json'".to_string(),
            })
        );
    }

    #[test]
    fn test_load_annotation_rejects_positional_arg() {
        let definitions = vec![make_table_with_metadata(
            "Player",
            load_metadata(vec![AnnotationArg::Positional(Literal::String(
                "data/players.csv".to_string(),
            ))]),
            vec![],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Player".to_string(),
                constraint: "@load.args".to_string(),
                message: "@load only accepts named parameters".to_string(),
            })
        );
    }

    #[test]
    fn test_load_annotation_rejects_unknown_parameter() {
        let definitions = vec![make_table_with_metadata(
            "Player",
            load_metadata(vec![named_arg(
                "yaml",
                Literal::String("data/players.yaml".to_string()),
            )]),
            vec![],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Player".to_string(),
                constraint: "@load.yaml".to_string(),
                message: "unsupported @load parameter; expected 'csv' or 'json'".to_string(),
            })
        );
    }

    #[test]
    fn test_load_annotation_rejects_non_string_path() {
        let definitions = vec![make_table_with_metadata(
            "Player",
            load_metadata(vec![named_arg("csv", Literal::Integer(1))]),
            vec![],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Player".to_string(),
                constraint: "@load.csv".to_string(),
                message: "value must be a string literal".to_string(),
            })
        );
    }

    #[test]
    fn test_load_annotation_rejects_duplicate_parameter() {
        let definitions = vec![make_table_with_metadata(
            "Player",
            load_metadata(vec![
                named_arg("csv", Literal::String("data/players.csv".to_string())),
                named_arg("csv", Literal::String("data/players2.csv".to_string())),
            ]),
            vec![],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Player".to_string(),
                constraint: "@load.csv".to_string(),
                message: "@load parameter is specified more than once".to_string(),
            })
        );
    }

    #[test]
    fn test_taggable_annotation_is_valid_on_table() {
        let definitions = vec![make_table_with_metadata(
            "Player",
            taggable_metadata(vec![]),
            vec![],
        )];

        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_taggable_annotation_rejects_arguments() {
        let definitions = vec![make_table_with_metadata(
            "Player",
            taggable_metadata(vec![AnnotationArg::Positional(Literal::Identifier(
                "row".to_string(),
            ))]),
            vec![],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Player".to_string(),
                constraint: "@taggable.args".to_string(),
                message: "@taggable does not accept arguments".to_string(),
            })
        );
    }

    #[test]
    fn test_taggable_annotation_rejects_embed_target() {
        let definitions = vec![make_embed_with_metadata(
            "Stats",
            taggable_metadata(vec![]),
            vec![],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Stats".to_string(),
                constraint: "@taggable.target".to_string(),
                message: "@taggable can only be used on table definitions".to_string(),
            })
        );
    }

    #[test]
    fn test_link_rows_annotation_is_valid_on_table() {
        let definitions = vec![make_table_with_metadata(
            "Player",
            link_rows_metadata(vec![AnnotationArg::Positional(Literal::Identifier(
                "Character".to_string(),
            ))]),
            vec![],
        )];

        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_link_rows_annotation_rejects_embed_target() {
        let definitions = vec![make_embed_with_metadata(
            "Stats",
            link_rows_metadata(vec![AnnotationArg::Positional(Literal::Identifier(
                "Character".to_string(),
            ))]),
            vec![],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Stats".to_string(),
                constraint: "@link_rows.target".to_string(),
                message: "@link_rows can only be used on table definitions".to_string(),
            })
        );
    }

    #[test]
    fn test_link_rows_annotation_rejects_missing_target() {
        let definitions = vec![make_table_with_metadata(
            "Player",
            link_rows_metadata(vec![]),
            vec![],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Player".to_string(),
                constraint: "@link_rows.target".to_string(),
                message: "@link_rows requires exactly one target type".to_string(),
            })
        );
    }

    #[test]
    fn test_link_rows_annotation_rejects_named_parameter() {
        let definitions = vec![make_table_with_metadata(
            "Player",
            link_rows_metadata(vec![named_arg(
                "target",
                Literal::Identifier("Character".to_string()),
            )]),
            vec![],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Player".to_string(),
                constraint: "@link_rows.target".to_string(),
                message: "unsupported @link_rows parameter; expected positional target type"
                    .to_string(),
            })
        );
    }

    #[test]
    fn test_link_rows_annotation_rejects_non_name_target() {
        let definitions = vec![make_table_with_metadata(
            "Player",
            link_rows_metadata(vec![AnnotationArg::Positional(Literal::Integer(1))]),
            vec![],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Player".to_string(),
                constraint: "@link_rows.target".to_string(),
                message: "value must be an identifier or string literal".to_string(),
            })
        );
    }

    #[test]
    fn test_max_length_constraint_is_valid_on_string() {
        let definitions = vec![make_table(
            "Player",
            vec![make_field_with_constraints(
                "name",
                BasicType::String,
                vec![Constraint::MaxLength(100)],
            )],
        )];

        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_max_length_constraint_rejects_non_string_or_bytes() {
        let definitions = vec![make_table(
            "Player",
            vec![make_field_with_constraints(
                "level",
                BasicType::U32,
                vec![Constraint::MaxLength(100)],
            )],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "level".to_string(),
                constraint: "max_length".to_string(),
                message: "max_length can only be used with string or bytes fields".to_string(),
            })
        );
    }

    #[test]
    fn test_max_length_constraint_rejects_zero() {
        let definitions = vec![make_table(
            "Player",
            vec![make_field_with_constraints(
                "name",
                BasicType::String,
                vec![Constraint::MaxLength(0)],
            )],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "name".to_string(),
                constraint: "max_length".to_string(),
                message: "max_length must be greater than 0".to_string(),
            })
        );
    }

    #[test]
    fn test_regex_constraint_is_valid_on_string() {
        let definitions = vec![make_table(
            "Player",
            vec![make_field_with_constraints(
                "email",
                BasicType::String,
                vec![Constraint::Regex(".*@.*".to_string())],
            )],
        )];

        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_regex_constraint_rejects_non_string() {
        let definitions = vec![make_table(
            "Player",
            vec![make_field_with_constraints(
                "level",
                BasicType::U32,
                vec![Constraint::Regex("[0-9]+".to_string())],
            )],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "level".to_string(),
                constraint: "regex".to_string(),
                message: "regex can only be used with string fields".to_string(),
            })
        );
    }

    #[test]
    fn test_regex_constraint_rejects_invalid_pattern() {
        let definitions = vec![make_table(
            "Player",
            vec![make_field_with_constraints(
                "email",
                BasicType::String,
                vec![Constraint::Regex("[".to_string())],
            )],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "email".to_string(),
                constraint: "regex".to_string(),
                message: "regex pattern is invalid".to_string(),
            })
        );
    }

    #[test]
    fn test_range_constraint_is_valid_on_integer() {
        let definitions = vec![make_table(
            "Player",
            vec![make_field_with_constraints(
                "level",
                BasicType::U32,
                vec![Constraint::Range(
                    Literal::Integer(1),
                    Literal::Integer(100),
                )],
            )],
        )];

        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_range_constraint_is_valid_on_float() {
        let definitions = vec![make_table(
            "Player",
            vec![make_field_with_constraints(
                "speed",
                BasicType::F32,
                vec![Constraint::Range(Literal::Float(0.5), Literal::Integer(10))],
            )],
        )];

        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_range_constraint_rejects_non_numeric_field() {
        let definitions = vec![make_table(
            "Player",
            vec![make_field_with_constraints(
                "name",
                BasicType::String,
                vec![Constraint::Range(Literal::Integer(1), Literal::Integer(10))],
            )],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "name".to_string(),
                constraint: "range".to_string(),
                message: "range can only be used with numeric fields".to_string(),
            })
        );
    }

    #[test]
    fn test_range_constraint_rejects_non_numeric_bound() {
        let definitions = vec![make_table(
            "Player",
            vec![make_field_with_constraints(
                "level",
                BasicType::U32,
                vec![Constraint::Range(
                    Literal::String("low".to_string()),
                    Literal::Integer(10),
                )],
            )],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "level".to_string(),
                constraint: "range".to_string(),
                message: "range bounds must be numeric literals".to_string(),
            })
        );
    }

    #[test]
    fn test_range_constraint_rejects_min_greater_than_max() {
        let definitions = vec![make_table(
            "Player",
            vec![make_field_with_constraints(
                "level",
                BasicType::U32,
                vec![Constraint::Range(Literal::Integer(10), Literal::Integer(1))],
            )],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "level".to_string(),
                constraint: "range".to_string(),
                message: "range minimum must be less than or equal to maximum".to_string(),
            })
        );
    }

    #[test]
    fn test_range_constraint_rejects_negative_unsigned_minimum() {
        let definitions = vec![make_table(
            "Player",
            vec![make_field_with_constraints(
                "level",
                BasicType::U32,
                vec![Constraint::Range(
                    Literal::Integer(-1),
                    Literal::Integer(10),
                )],
            )],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "level".to_string(),
                constraint: "range".to_string(),
                message: "unsigned integer range minimum must be greater than or equal to 0"
                    .to_string(),
            })
        );
    }

    #[test]
    fn test_range_constraint_rejects_float_bound_on_integer_field() {
        let definitions = vec![make_table(
            "Player",
            vec![make_field_with_constraints(
                "level",
                BasicType::I32,
                vec![Constraint::Range(Literal::Float(1.5), Literal::Integer(10))],
            )],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "level".to_string(),
                constraint: "range".to_string(),
                message: "integer field range bounds must be integer literals".to_string(),
            })
        );
    }

    #[test]
    fn test_primary_key_constraint_is_valid_on_scalar_field() {
        let definitions = vec![make_table(
            "Player",
            vec![make_primary_key_field("id", BasicType::U32)],
        )];

        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_primary_key_constraint_rejects_duplicate_primary_key_on_field() {
        let definitions = vec![make_table(
            "Player",
            vec![make_field_with_constraints(
                "id",
                BasicType::U32,
                vec![Constraint::PrimaryKey, Constraint::PrimaryKey],
            )],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "id".to_string(),
                constraint: "primary_key".to_string(),
                message: "primary_key is specified more than once".to_string(),
            })
        );
    }

    #[test]
    fn test_primary_key_constraint_rejects_multiple_primary_keys_per_table() {
        let definitions = vec![make_table(
            "Player",
            vec![
                make_primary_key_field("id", BasicType::U32),
                make_primary_key_field("external_id", BasicType::U64),
            ],
        )];

        let result = validate_ast(&definitions);
        assert!(matches!(
            result,
            Err(ValidationError::InvalidConstraint {
                constraint,
                ..
            }) if constraint == "primary_key"
        ));
    }

    #[test]
    fn test_primary_key_constraint_rejects_optional_field() {
        let definitions = vec![make_table(
            "Player",
            vec![make_field_with_type_and_constraints(
                "id",
                TypeName::Basic(BasicType::U32),
                Some(Cardinality::Optional),
                vec![Constraint::PrimaryKey],
            )],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "id".to_string(),
                constraint: "primary_key".to_string(),
                message: "primary_key fields cannot be optional".to_string(),
            })
        );
    }

    #[test]
    fn test_primary_key_constraint_rejects_array_field() {
        let definitions = vec![make_table(
            "Player",
            vec![make_field_with_type_and_constraints(
                "ids",
                TypeName::Basic(BasicType::U32),
                Some(Cardinality::Array),
                vec![Constraint::PrimaryKey],
            )],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "ids".to_string(),
                constraint: "primary_key".to_string(),
                message: "constraint is not supported for array fields".to_string(),
            })
        );
    }

    #[test]
    fn test_unique_constraint_is_valid_on_optional_scalar_field() {
        let definitions = vec![make_table(
            "Player",
            vec![make_field_with_type_and_constraints(
                "email",
                TypeName::Basic(BasicType::String),
                Some(Cardinality::Optional),
                vec![Constraint::Unique],
            )],
        )];

        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_unique_constraint_rejects_duplicate_unique_on_field() {
        let definitions = vec![make_table(
            "Player",
            vec![make_field_with_constraints(
                "email",
                BasicType::String,
                vec![Constraint::Unique, Constraint::Unique],
            )],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "email".to_string(),
                constraint: "unique".to_string(),
                message: "unique is specified more than once".to_string(),
            })
        );
    }

    #[test]
    fn test_unique_constraint_rejects_array_field() {
        let definitions = vec![make_table(
            "Player",
            vec![make_field_with_type_and_constraints(
                "tags",
                TypeName::Basic(BasicType::String),
                Some(Cardinality::Array),
                vec![Constraint::Unique],
            )],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "tags".to_string(),
                constraint: "unique".to_string(),
                message: "constraint is not supported for array fields".to_string(),
            })
        );
    }

    #[test]
    fn test_unique_constraint_rejects_embed_field() {
        let definitions = vec![
            make_embed("Stats", vec![make_field_basic("hp", BasicType::U32)]),
            make_table(
                "Player",
                vec![make_field_with_type_and_constraints(
                    "stats",
                    TypeName::Path(vec!["Stats".to_string()]),
                    None,
                    vec![Constraint::Unique],
                )],
            ),
        ];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "stats".to_string(),
                constraint: "unique".to_string(),
                message: "constraint is not supported for struct or embed fields".to_string(),
            })
        );
    }

    #[test]
    fn test_index_constraint_is_valid_on_scalar_field() {
        let definitions = vec![make_table(
            "Player",
            vec![make_field_with_constraints(
                "guild_id",
                BasicType::U32,
                vec![Constraint::Index],
            )],
        )];

        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_index_constraint_allows_enum_field() {
        let definitions = vec![
            make_enum("Status"),
            make_table(
                "Player",
                vec![make_field_with_type_and_constraints(
                    "status",
                    TypeName::Path(vec!["Status".to_string()]),
                    None,
                    vec![Constraint::Index],
                )],
            ),
        ];

        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_index_constraint_rejects_duplicate_index_on_field() {
        let definitions = vec![make_table(
            "Player",
            vec![make_field_with_constraints(
                "guild_id",
                BasicType::U32,
                vec![Constraint::Index, Constraint::Index],
            )],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "guild_id".to_string(),
                constraint: "index".to_string(),
                message: "index is specified more than once".to_string(),
            })
        );
    }

    #[test]
    fn test_index_constraint_rejects_array_field() {
        let definitions = vec![make_table(
            "Player",
            vec![make_field_with_type_and_constraints(
                "tags",
                TypeName::Basic(BasicType::String),
                Some(Cardinality::Array),
                vec![Constraint::Index],
            )],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "tags".to_string(),
                constraint: "index".to_string(),
                message: "constraint is not supported for array fields".to_string(),
            })
        );
    }

    #[test]
    fn test_index_constraint_rejects_embed_field() {
        let definitions = vec![
            make_embed("Stats", vec![make_field_basic("hp", BasicType::U32)]),
            make_table(
                "Player",
                vec![make_field_with_type_and_constraints(
                    "stats",
                    TypeName::Path(vec!["Stats".to_string()]),
                    None,
                    vec![Constraint::Index],
                )],
            ),
        ];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "stats".to_string(),
                constraint: "index".to_string(),
                message: "constraint is not supported for struct or embed fields".to_string(),
            })
        );
    }

    #[test]
    fn test_foreign_key_constraint_is_valid_for_matching_table_field() {
        let definitions = vec![
            make_table("User", vec![make_primary_key_field("id", BasicType::U32)]),
            make_table(
                "Post",
                vec![make_field_with_constraints(
                    "user_id",
                    BasicType::U32,
                    vec![Constraint::ForeignKey(
                        vec!["User".to_string(), "id".to_string()],
                        None,
                    )],
                )],
            ),
        ];

        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_foreign_key_constraint_resolves_same_namespace_before_unique_name() {
        let definitions = vec![
            make_namespace(
                vec!["admin"],
                vec![make_table(
                    "User",
                    vec![make_primary_key_field("id", BasicType::U64)],
                )],
            ),
            make_namespace(
                vec!["game"],
                vec![
                    make_table("User", vec![make_primary_key_field("id", BasicType::U32)]),
                    make_table(
                        "Post",
                        vec![make_field_with_constraints(
                            "user_id",
                            BasicType::U32,
                            vec![Constraint::ForeignKey(
                                vec!["User".to_string(), "id".to_string()],
                                None,
                            )],
                        )],
                    ),
                ],
            ),
        ];

        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_foreign_key_constraint_resolves_target_type_in_target_scope() {
        let definitions = vec![
            make_namespace(
                vec!["a"],
                vec![
                    make_enum("Status"),
                    make_table(
                        "Parent",
                        vec![make_field_with_type_and_constraints(
                            "id",
                            TypeName::Path(vec!["Status".to_string()]),
                            None,
                            vec![Constraint::PrimaryKey],
                        )],
                    ),
                ],
            ),
            make_namespace(
                vec!["b"],
                vec![
                    make_enum("Status"),
                    make_table(
                        "Child",
                        vec![make_field_with_type_and_constraints(
                            "parent_id",
                            TypeName::Path(vec!["a".to_string(), "Status".to_string()]),
                            None,
                            vec![Constraint::ForeignKey(
                                vec!["a".to_string(), "Parent".to_string(), "id".to_string()],
                                None,
                            )],
                        )],
                    ),
                ],
            ),
        ];

        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_foreign_key_constraint_rejects_duplicate_foreign_key() {
        let definitions = vec![
            make_table("User", vec![make_primary_key_field("id", BasicType::U32)]),
            make_table(
                "Post",
                vec![make_field_with_constraints(
                    "user_id",
                    BasicType::U32,
                    vec![
                        Constraint::ForeignKey(vec!["User".to_string(), "id".to_string()], None),
                        Constraint::ForeignKey(vec!["User".to_string(), "id".to_string()], None),
                    ],
                )],
            ),
        ];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "user_id".to_string(),
                constraint: "foreign_key".to_string(),
                message: "foreign_key is specified more than once".to_string(),
            })
        );
    }

    #[test]
    fn test_foreign_key_constraint_rejects_short_target_path() {
        let definitions = vec![make_table(
            "Post",
            vec![make_field_with_constraints(
                "user_id",
                BasicType::U32,
                vec![Constraint::ForeignKey(vec!["User".to_string()], None)],
            )],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "user_id".to_string(),
                constraint: "foreign_key".to_string(),
                message: "foreign_key target must be a table field path".to_string(),
            })
        );
    }

    #[test]
    fn test_foreign_key_constraint_reports_missing_target_table_as_type_not_found() {
        let definitions = vec![make_table(
            "Post",
            vec![make_field_with_constraints(
                "user_id",
                BasicType::U32,
                vec![Constraint::ForeignKey(
                    vec!["User".to_string(), "id".to_string()],
                    None,
                )],
            )],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::TypeNotFound("User".to_string()))
        );
    }

    #[test]
    fn test_foreign_key_constraint_rejects_non_table_target() {
        let definitions = vec![
            make_enum("Status"),
            make_table(
                "Post",
                vec![make_field_with_constraints(
                    "status_id",
                    BasicType::U32,
                    vec![Constraint::ForeignKey(
                        vec!["Status".to_string(), "id".to_string()],
                        None,
                    )],
                )],
            ),
        ];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "status_id".to_string(),
                constraint: "foreign_key".to_string(),
                message: "foreign_key target must be a table".to_string(),
            })
        );
    }

    #[test]
    fn test_foreign_key_constraint_rejects_missing_target_field() {
        let definitions = vec![
            make_table("User", vec![make_primary_key_field("id", BasicType::U32)]),
            make_table(
                "Post",
                vec![make_field_with_constraints(
                    "user_id",
                    BasicType::U32,
                    vec![Constraint::ForeignKey(
                        vec!["User".to_string(), "missing".to_string()],
                        None,
                    )],
                )],
            ),
        ];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "user_id".to_string(),
                constraint: "foreign_key".to_string(),
                message: "foreign_key target field does not exist".to_string(),
            })
        );
    }

    #[test]
    fn test_foreign_key_constraint_rejects_type_mismatch() {
        let definitions = vec![
            make_table("User", vec![make_primary_key_field("id", BasicType::U32)]),
            make_table(
                "Post",
                vec![make_field_with_constraints(
                    "user_id",
                    BasicType::U64,
                    vec![Constraint::ForeignKey(
                        vec!["User".to_string(), "id".to_string()],
                        None,
                    )],
                )],
            ),
        ];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "user_id".to_string(),
                constraint: "foreign_key".to_string(),
                message: "foreign_key field type must match the target field type".to_string(),
            })
        );
    }

    #[test]
    fn test_foreign_key_constraint_rejects_array_field() {
        let definitions = vec![
            make_table("User", vec![make_primary_key_field("id", BasicType::U32)]),
            make_table(
                "Post",
                vec![make_field_with_type_and_constraints(
                    "user_ids",
                    TypeName::Basic(BasicType::U32),
                    Some(Cardinality::Array),
                    vec![Constraint::ForeignKey(
                        vec!["User".to_string(), "id".to_string()],
                        None,
                    )],
                )],
            ),
        ];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "user_ids".to_string(),
                constraint: "foreign_key".to_string(),
                message: "foreign_key is not supported for array fields".to_string(),
            })
        );
    }

    #[test]
    fn test_foreign_key_constraint_rejects_non_unique_target_field() {
        let definitions = vec![
            make_table("User", vec![make_field_basic("code", BasicType::String)]),
            make_table(
                "Post",
                vec![make_field_with_constraints(
                    "user_code",
                    BasicType::String,
                    vec![Constraint::ForeignKey(
                        vec!["User".to_string(), "code".to_string()],
                        None,
                    )],
                )],
            ),
        ];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "user_code".to_string(),
                constraint: "foreign_key".to_string(),
                message: "foreign_key target field must be primary_key or unique".to_string(),
            })
        );
    }

    #[test]
    fn test_default_constraint_is_valid_on_scalar_fields() {
        let definitions = vec![make_table(
            "Player",
            vec![
                make_field_with_constraints(
                    "name",
                    BasicType::String,
                    vec![Constraint::Default(Literal::String("guest".to_string()))],
                ),
                make_field_with_constraints(
                    "level",
                    BasicType::U16,
                    vec![Constraint::Default(Literal::Integer(1))],
                ),
                make_field_with_constraints(
                    "speed",
                    BasicType::F32,
                    vec![Constraint::Default(Literal::Float(1.5))],
                ),
                make_field_with_constraints(
                    "enabled",
                    BasicType::Bool,
                    vec![Constraint::Default(Literal::Boolean(true))],
                ),
            ],
        )];

        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_default_constraint_rejects_duplicate_default() {
        let definitions = vec![make_table(
            "Player",
            vec![make_field_with_constraints(
                "level",
                BasicType::U32,
                vec![
                    Constraint::Default(Literal::Integer(1)),
                    Constraint::Default(Literal::Integer(2)),
                ],
            )],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "level".to_string(),
                constraint: "default".to_string(),
                message: "default is specified more than once".to_string(),
            })
        );
    }

    #[test]
    fn test_default_constraint_rejects_array_field() {
        let definitions = vec![make_table(
            "Player",
            vec![make_field_with_type_and_constraints(
                "tags",
                TypeName::Basic(BasicType::String),
                Some(Cardinality::Array),
                vec![Constraint::Default(Literal::String("new".to_string()))],
            )],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "tags".to_string(),
                constraint: "default".to_string(),
                message: "default is not supported for array fields".to_string(),
            })
        );
    }

    #[test]
    fn test_default_constraint_rejects_wrong_literal_type() {
        let definitions = vec![make_table(
            "Player",
            vec![make_field_with_constraints(
                "level",
                BasicType::U32,
                vec![Constraint::Default(Literal::String("1".to_string()))],
            )],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "level".to_string(),
                constraint: "default".to_string(),
                message: "integer fields require an integer default value".to_string(),
            })
        );
    }

    #[test]
    fn test_default_constraint_rejects_integer_outside_type_range() {
        let definitions = vec![make_table(
            "Player",
            vec![make_field_with_constraints(
                "level",
                BasicType::U8,
                vec![Constraint::Default(Literal::Integer(300))],
            )],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "level".to_string(),
                constraint: "default".to_string(),
                message: "integer default value is outside the field type range".to_string(),
            })
        );
    }

    #[test]
    fn test_default_constraint_rejects_default_outside_range() {
        let definitions = vec![make_table(
            "Player",
            vec![make_field_with_constraints(
                "level",
                BasicType::U32,
                vec![
                    Constraint::Default(Literal::Integer(0)),
                    Constraint::Range(Literal::Integer(1), Literal::Integer(100)),
                ],
            )],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "level".to_string(),
                constraint: "default".to_string(),
                message: "default value must be within the field range".to_string(),
            })
        );
    }

    #[test]
    fn test_default_constraint_allows_enum_identifier() {
        let definitions = vec![
            make_enum("Status"),
            make_table(
                "Player",
                vec![make_field_with_type_and_constraints(
                    "status",
                    TypeName::Path(vec!["Status".to_string()]),
                    None,
                    vec![Constraint::Default(Literal::Identifier(
                        "Online".to_string(),
                    ))],
                )],
            ),
        ];

        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_default_constraint_resolves_enum_with_scoped_resolver() {
        let definitions = vec![
            make_namespace(
                vec!["a"],
                vec![
                    make_enum("Status"),
                    make_table(
                        "Player",
                        vec![make_field_with_type_and_constraints(
                            "status",
                            TypeName::Path(vec!["Status".to_string()]),
                            None,
                            vec![Constraint::Default(Literal::Identifier(
                                "Active".to_string(),
                            ))],
                        )],
                    ),
                ],
            ),
            make_namespace(vec!["b"], vec![make_enum("Status")]),
        ];

        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_default_constraint_rejects_struct_field() {
        let definitions = vec![
            make_embed("Stats", vec![make_field_basic("hp", BasicType::U32)]),
            make_table(
                "Player",
                vec![make_field_with_type_and_constraints(
                    "stats",
                    TypeName::Path(vec!["Stats".to_string()]),
                    None,
                    vec![Constraint::Default(Literal::Identifier(
                        "Default".to_string(),
                    ))],
                )],
            ),
        ];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "stats".to_string(),
                constraint: "default".to_string(),
                message: "default is not supported for struct or embed fields".to_string(),
            })
        );
    }

    #[test]
    fn test_pack_annotation_without_args_is_valid_on_embed() {
        let definitions = vec![make_embed_with_metadata(
            "Position",
            pack_metadata(vec![]),
            vec![make_field_basic("x", BasicType::F32)],
        )];

        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_pack_annotation_separator_is_valid_on_nested_embed() {
        let definitions = vec![make_table(
            "Sprite",
            vec![TableMember::Embed(Embed {
                metadata: pack_metadata(vec![named_arg(
                    "separator",
                    Literal::String("|".to_string()),
                )]),
                name: Some("Color".to_string()),
                members: vec![make_field_basic("r", BasicType::U8)],
            })],
        )];

        assert!(validate_ast(&definitions).is_ok());
    }

    #[test]
    fn test_pack_annotation_rejects_table_target() {
        let definitions = vec![make_table_with_metadata(
            "Sprite",
            pack_metadata(vec![]),
            vec![make_field_basic("id", BasicType::U32)],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Sprite".to_string(),
                constraint: "@pack.target".to_string(),
                message: "@pack can only be used on embed definitions".to_string(),
            })
        );
    }

    #[test]
    fn test_pack_annotation_rejects_positional_separator() {
        let definitions = vec![make_embed_with_metadata(
            "Position",
            pack_metadata(vec![AnnotationArg::Positional(Literal::String(
                ",".to_string(),
            ))]),
            vec![],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Position".to_string(),
                constraint: "@pack.separator".to_string(),
                message: "@pack only accepts named parameter 'separator'".to_string(),
            })
        );
    }

    #[test]
    fn test_pack_annotation_rejects_unknown_parameter() {
        let definitions = vec![make_embed_with_metadata(
            "Position",
            pack_metadata(vec![named_arg(
                "delimiter",
                Literal::String(",".to_string()),
            )]),
            vec![],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Position".to_string(),
                constraint: "@pack.delimiter".to_string(),
                message: "unsupported @pack parameter; expected 'separator'".to_string(),
            })
        );
    }

    #[test]
    fn test_pack_annotation_rejects_non_string_separator() {
        let definitions = vec![make_embed_with_metadata(
            "Position",
            pack_metadata(vec![named_arg("separator", Literal::Integer(1))]),
            vec![],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Position".to_string(),
                constraint: "@pack.separator".to_string(),
                message: "separator must be a string literal".to_string(),
            })
        );
    }

    #[test]
    fn test_pack_annotation_rejects_multi_character_separator() {
        let definitions = vec![make_embed_with_metadata(
            "Position",
            pack_metadata(vec![named_arg(
                "separator",
                Literal::String("::".to_string()),
            )]),
            vec![],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Position".to_string(),
                constraint: "@pack.separator".to_string(),
                message: "separator must contain exactly one character".to_string(),
            })
        );
    }

    #[test]
    fn test_pack_annotation_rejects_duplicate_separator() {
        let definitions = vec![make_embed_with_metadata(
            "Position",
            pack_metadata(vec![
                named_arg("separator", Literal::String(",".to_string())),
                named_arg("separator", Literal::String("|".to_string())),
            ]),
            vec![],
        )];

        assert_eq!(
            validate_ast(&definitions),
            Err(ValidationError::InvalidConstraint {
                field: "Position".to_string(),
                constraint: "@pack.separator".to_string(),
                message: "separator is specified more than once".to_string(),
            })
        );
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
