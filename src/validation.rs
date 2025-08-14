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

                // Also collect named embeds and enums defined inside the table.
                path.push(t.name.as_ref().expect("Table name should be present").clone());
                for member in &t.members {
                    if let TableMember::Embed(e) = member {
                        path.push(e.name.clone().expect("Embed name should be present"));
                        let embed_fqn = path.join(".");
                        if !types.insert(embed_fqn.clone()) {
                            return Err(ValidationError::DuplicateDefinition(embed_fqn));
                        }
                        path.pop();
                    } else if let TableMember::Enum(e) = member {
                        // Named enums embedded within a table should still have a name.
                        if let Some(name) = &e.name {
                            path.push(name.clone());
                            let enum_fqn = path.join(".");
                            if !types.insert(enum_fqn.clone()) {
                                return Err(ValidationError::DuplicateDefinition(enum_fqn));
                            }
                            path.pop();
                        } else {
                            // This case should ideally not happen if grammar ensures named enums in table members have names.
                            // If it does, it's an error in AST construction or grammar.
                            // For now, we'll just ignore it as inline enums are not globally addressable.
                        }
                    }
                }
                path.pop();
            }
            Definition::Enum(e) => {
                // Only named enums are collected as globally defined types.
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
                for member in &t.members {
                    match member {
                        TableMember::Field(FieldDefinition::Regular(field)) => {
                            if let TypeName::Path(type_path) = &field.field_type.base_type {
                                check_type_path(type_path, path, all_types)?;
                            } else if let TypeName::InlineEnum(_) = &field.field_type.base_type {
                                // Inline enums do not reference other types, so no validation needed here.
                            }
                        }
                        TableMember::Field(FieldDefinition::InlineEmbed(inline_embed)) => {
                            for f in &inline_embed.fields {
                                if let FieldDefinition::Regular(rf) = f {
                                    if let TypeName::Path(type_path) = &rf.field_type.base_type {
                                        check_type_path(type_path, path, all_types)?;
                                    } else if let TypeName::InlineEnum(_) = &rf.field_type.base_type {
                                        // Inline enums do not reference other types, so no validation needed here.
                                    }
                                }
                            }
                        }
                        TableMember::Field(FieldDefinition::InlineEnum(_)) => { /* Inline enums do not reference other types */ }
                        TableMember::Embed(_) => { /* Embeds are checked as top-level types */ }
                        TableMember::Enum(_) => { /* Inline enums do not reference other types */ }
                        TableMember::Comment(_) => { /* Comments do not reference types */ }
                    }
                }
                path.pop();
            }
            Definition::Enum(_) => { /* Enums do not reference other types */ }
            Definition::Embed(e) => {
                // Validate types used within the embed's fields.
                path.push(e.name.clone().expect("Embed name should be present"));
                 for field in &e.fields {
                    if let FieldDefinition::Regular(rf) = field {
                        if let TypeName::Path(type_path) = &rf.field_type.base_type {
                            check_type_path(type_path, path, all_types)?;
                        }
                    }
                }
                path.pop();
            }
            Definition::Comment(_) => { /* Comments do not reference types */ }
            Definition::Annotation(_) => { /* Annotations do not reference types */ }
        }
    }
    Ok(())
}