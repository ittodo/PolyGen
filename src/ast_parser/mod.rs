#[macro_use]
mod macros;
mod definitions;
mod fields;
mod helpers;
mod literals;
mod metadata;
mod types;

use pest::iterators::Pair;
use std::path::PathBuf;

use crate::ast_model::{AstRoot, Definition, RenameRule};
use crate::error::AstBuildError;
use crate::Rule;

use definitions::parse_definition;
use helpers::extract_comment_content;

/// Parse a .renames file and return a list of rename rules.
///
/// Renames file format:
/// ```text
/// # Comment line
/// Player -> User;
/// User.user_name -> name;
/// ```
pub fn parse_renames_file(renames_pair: Pair<Rule>) -> Result<Vec<RenameRule>, AstBuildError> {
    let mut renames = Vec::new();

    for pair in renames_pair.into_inner() {
        match pair.as_rule() {
            Rule::rename_rule => {
                let rename = parse_rename_rule(pair)?;
                renames.push(rename);
            }
            Rule::rename_comment_line | Rule::EOI => (),
            _ => (),
        }
    }

    Ok(renames)
}

/// Parse a single rename rule: `path -> IDENT;`
fn parse_rename_rule(pair: Pair<Rule>) -> Result<RenameRule, AstBuildError> {
    let (line, col) = pair.line_col();
    let mut inner = pair.into_inner();

    // First is the path (from_path)
    let path_pair = inner.next().ok_or(AstBuildError::MissingElement {
        rule: Rule::rename_rule,
        element: "source path".to_string(),
        line,
        col,
    })?;
    let from_path: Vec<String> = path_pair
        .into_inner()
        .map(|p| p.as_str().to_string())
        .collect();

    // Second is the IDENT (to_name)
    let ident_pair = inner.next().ok_or(AstBuildError::MissingElement {
        rule: Rule::rename_rule,
        element: "target name".to_string(),
        line,
        col,
    })?;
    let to_name = ident_pair.as_str().to_string();

    Ok(RenameRule { from_path, to_name })
}

/// Main function to build the AST from parsed pairs
pub fn build_ast_from_pairs(
    main_pair: Pair<Rule>,
    path: PathBuf,
) -> Result<AstRoot, AstBuildError> {
    let mut ast_root = AstRoot {
        path,
        ..Default::default()
    };

    for pair in main_pair.into_inner() {
        match pair.as_rule() {
            Rule::toplevel_item => {
                let (line, col) = pair.line_col();
                let item_pair = pair
                    .into_inner()
                    .next()
                    .ok_or(AstBuildError::MissingElement {
                        rule: Rule::toplevel_item,
                        element: "file_import or definition".to_string(),
                        line,
                        col,
                    })?;

                match item_pair.as_rule() {
                    Rule::file_import => {
                        let path_literal = item_pair.into_inner().next().unwrap();
                        let path_str = path_literal.as_str();
                        let path = path_str[1..path_str.len() - 1].to_string();
                        ast_root.file_imports.push(path);
                    }
                    Rule::definition => {
                        ast_root.definitions.push(parse_definition(item_pair)?);
                    }
                    Rule::doc_comment => {
                        ast_root
                            .definitions
                            .push(Definition::Comment(extract_comment_content(item_pair)));
                    }
                    found => {
                        return Err(AstBuildError::UnexpectedRule {
                            expected: "file_import or definition".to_string(),
                            found,
                            line,
                            col,
                        })
                    }
                }
            }
            Rule::EOI => (),
            found => {
                let (line, col) = pair.line_col();
                return Err(AstBuildError::UnexpectedRule {
                    expected: "definition".to_string(),
                    found,
                    line,
                    col,
                });
            }
        }
    }
    Ok(ast_root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast_model::*;
    use crate::Polygen;
    use pest::Parser;

    /// Helper to parse a schema string and build AST
    fn parse_schema(input: &str) -> Result<AstRoot, AstBuildError> {
        let input = input.replace("\r\n", "\n");
        let main_pair = Polygen::parse(Rule::main, &input)
            .expect("Failed to parse")
            .next()
            .expect("No main pair");
        build_ast_from_pairs(main_pair, PathBuf::from("test.schema"))
    }

    // ========== File Import Tests ==========

    #[test]
    fn test_parse_file_import() {
        let ast = parse_schema(r#"import "common/types.poly";"#).unwrap();
        assert_eq!(ast.file_imports.len(), 1);
        assert_eq!(ast.file_imports[0], "common/types.poly");
    }

    #[test]
    fn test_parse_multiple_file_imports() {
        let ast = parse_schema(
            r#"
            import "common/types.poly";
            import "game/items.poly";
            "#,
        )
        .unwrap();
        assert_eq!(ast.file_imports.len(), 2);
        assert_eq!(ast.file_imports[0], "common/types.poly");
        assert_eq!(ast.file_imports[1], "game/items.poly");
    }

    // ========== Table Tests ==========

    #[test]
    fn test_parse_empty_table() {
        let ast = parse_schema("table User {}").unwrap();
        assert_eq!(ast.definitions.len(), 1);
        if let Definition::Table(table) = &ast.definitions[0] {
            assert_eq!(table.name, Some("User".to_string()));
            assert!(table.members.is_empty());
        } else {
            panic!("Expected Table definition");
        }
    }

    #[test]
    fn test_parse_table_with_basic_fields() {
        let ast = parse_schema(
            r#"
            table User {
                id: i32;
                name: string;
                active: bool;
            }
            "#,
        )
        .unwrap();

        if let Definition::Table(table) = &ast.definitions[0] {
            assert_eq!(table.name, Some("User".to_string()));
            assert_eq!(table.members.len(), 3);

            // Check first field
            if let TableMember::Field(FieldDefinition::Regular(rf)) = &table.members[0] {
                assert_eq!(rf.name, Some("id".to_string()));
                assert_eq!(rf.field_type.base_type, TypeName::Basic(BasicType::I32));
            } else {
                panic!("Expected Regular field");
            }
        } else {
            panic!("Expected Table definition");
        }
    }

    #[test]
    fn test_parse_table_with_optional_field() {
        let ast = parse_schema(
            r#"
            table User {
                nickname: string?;
            }
            "#,
        )
        .unwrap();

        if let Definition::Table(table) = &ast.definitions[0] {
            if let TableMember::Field(FieldDefinition::Regular(rf)) = &table.members[0] {
                assert_eq!(rf.field_type.cardinality, Some(Cardinality::Optional));
            } else {
                panic!("Expected Regular field");
            }
        } else {
            panic!("Expected Table definition");
        }
    }

    #[test]
    fn test_parse_table_with_array_field() {
        let ast = parse_schema(
            r#"
            table User {
                tags: string[];
            }
            "#,
        )
        .unwrap();

        if let Definition::Table(table) = &ast.definitions[0] {
            if let TableMember::Field(FieldDefinition::Regular(rf)) = &table.members[0] {
                assert_eq!(rf.field_type.cardinality, Some(Cardinality::Array));
            } else {
                panic!("Expected Regular field");
            }
        } else {
            panic!("Expected Table definition");
        }
    }

    #[test]
    fn test_parse_table_with_custom_type_field() {
        let ast = parse_schema(
            r#"
            table User {
                address: game.common.Address;
            }
            "#,
        )
        .unwrap();

        if let Definition::Table(table) = &ast.definitions[0] {
            if let TableMember::Field(FieldDefinition::Regular(rf)) = &table.members[0] {
                if let TypeName::Path(path) = &rf.field_type.base_type {
                    assert_eq!(path, &vec!["game", "common", "Address"]);
                } else {
                    panic!("Expected Path type");
                }
            } else {
                panic!("Expected Regular field");
            }
        } else {
            panic!("Expected Table definition");
        }
    }

    // ========== Constraint Tests ==========

    #[test]
    fn test_parse_primary_key_constraint() {
        let ast = parse_schema(
            r#"
            table User {
                id: i32 primary_key;
            }
            "#,
        )
        .unwrap();

        if let Definition::Table(table) = &ast.definitions[0] {
            if let TableMember::Field(FieldDefinition::Regular(rf)) = &table.members[0] {
                assert!(rf.constraints.contains(&Constraint::PrimaryKey));
            } else {
                panic!("Expected Regular field");
            }
        } else {
            panic!("Expected Table definition");
        }
    }

    #[test]
    fn test_parse_max_length_constraint() {
        let ast = parse_schema(
            r#"
            table User {
                name: string max_length(100);
            }
            "#,
        )
        .unwrap();

        if let Definition::Table(table) = &ast.definitions[0] {
            if let TableMember::Field(FieldDefinition::Regular(rf)) = &table.members[0] {
                assert!(rf.constraints.contains(&Constraint::MaxLength(100)));
            } else {
                panic!("Expected Regular field");
            }
        } else {
            panic!("Expected Table definition");
        }
    }

    #[test]
    fn test_parse_multiple_constraints() {
        let ast = parse_schema(
            r#"
            table User {
                email: string unique max_length(255);
            }
            "#,
        )
        .unwrap();

        if let Definition::Table(table) = &ast.definitions[0] {
            if let TableMember::Field(FieldDefinition::Regular(rf)) = &table.members[0] {
                assert!(rf.constraints.contains(&Constraint::Unique));
                assert!(rf.constraints.contains(&Constraint::MaxLength(255)));
            } else {
                panic!("Expected Regular field");
            }
        } else {
            panic!("Expected Table definition");
        }
    }

    // ========== Enum Tests ==========

    #[test]
    fn test_parse_simple_enum() {
        let ast = parse_schema(
            r#"
            enum Status {
                Active
                Inactive
            }
            "#,
        )
        .unwrap();

        if let Definition::Enum(e) = &ast.definitions[0] {
            assert_eq!(e.name, Some("Status".to_string()));
            assert_eq!(e.variants.len(), 2);
            assert_eq!(e.variants[0].name, Some("Active".to_string()));
            assert_eq!(e.variants[1].name, Some("Inactive".to_string()));
        } else {
            panic!("Expected Enum definition");
        }
    }

    #[test]
    fn test_parse_enum_with_explicit_values() {
        let ast = parse_schema(
            r#"
            enum Priority {
                Low = 0
                Medium = 5
                High = 10
            }
            "#,
        )
        .unwrap();

        if let Definition::Enum(e) = &ast.definitions[0] {
            assert_eq!(e.variants[0].value, Some(0));
            assert_eq!(e.variants[1].value, Some(5));
            assert_eq!(e.variants[2].value, Some(10));
        } else {
            panic!("Expected Enum definition");
        }
    }

    // ========== Namespace Tests ==========

    #[test]
    fn test_parse_namespace() {
        let ast = parse_schema(
            r#"
            namespace game.common {
                table Item {}
            }
            "#,
        )
        .unwrap();

        if let Definition::Namespace(ns) = &ast.definitions[0] {
            assert_eq!(ns.path, vec!["game", "common"]);
            assert_eq!(ns.definitions.len(), 1);
        } else {
            panic!("Expected Namespace definition");
        }
    }

    #[test]
    fn test_parse_nested_namespaces() {
        let ast = parse_schema(
            r#"
            namespace game {
                namespace common {
                    table Item {}
                }
            }
            "#,
        )
        .unwrap();

        if let Definition::Namespace(outer_ns) = &ast.definitions[0] {
            assert_eq!(outer_ns.path, vec!["game"]);
            if let Definition::Namespace(inner_ns) = &outer_ns.definitions[0] {
                assert_eq!(inner_ns.path, vec!["common"]);
            } else {
                panic!("Expected inner Namespace");
            }
        } else {
            panic!("Expected Namespace definition");
        }
    }

    // ========== Annotation Tests ==========

    #[test]
    fn test_parse_simple_annotation() {
        let ast = parse_schema(
            r#"
            @deprecated
            table OldUser {}
            "#,
        )
        .unwrap();

        if let Definition::Table(table) = &ast.definitions[0] {
            assert_eq!(table.metadata.len(), 1);
            if let Metadata::Annotation(ann) = &table.metadata[0] {
                assert_eq!(ann.name, Some("deprecated".to_string()));
                assert!(ann.args.is_empty());
            } else {
                panic!("Expected Annotation metadata");
            }
        } else {
            panic!("Expected Table definition");
        }
    }

    #[test]
    fn test_parse_annotation_with_params() {
        let ast = parse_schema(
            r#"
            @csv(name: "users", delimiter: ",")
            table User {}
            "#,
        )
        .unwrap();

        if let Definition::Table(table) = &ast.definitions[0] {
            if let Metadata::Annotation(ann) = &table.metadata[0] {
                assert_eq!(ann.name, Some("csv".to_string()));
                assert_eq!(ann.args.len(), 2);
                // Both should be Named args
                if let AnnotationArg::Named(p) = &ann.args[0] {
                    assert_eq!(p.key, "name");
                } else {
                    panic!("Expected Named argument");
                }
                if let AnnotationArg::Named(p) = &ann.args[1] {
                    assert_eq!(p.key, "delimiter");
                } else {
                    panic!("Expected Named argument");
                }
            } else {
                panic!("Expected Annotation metadata");
            }
        } else {
            panic!("Expected Table definition");
        }
    }

    #[test]
    fn test_parse_annotation_with_positional_args() {
        let ast = parse_schema(
            r#"
            @index(name, level)
            table User {}
            "#,
        )
        .unwrap();

        if let Definition::Table(table) = &ast.definitions[0] {
            if let Metadata::Annotation(ann) = &table.metadata[0] {
                assert_eq!(ann.name, Some("index".to_string()));
                assert_eq!(ann.args.len(), 2);
                // Both should be Positional args
                if let AnnotationArg::Positional(lit) = &ann.args[0] {
                    assert_eq!(lit.to_string(), "name");
                } else {
                    panic!("Expected Positional argument");
                }
                if let AnnotationArg::Positional(lit) = &ann.args[1] {
                    assert_eq!(lit.to_string(), "level");
                } else {
                    panic!("Expected Positional argument");
                }
            } else {
                panic!("Expected Annotation metadata");
            }
        } else {
            panic!("Expected Table definition");
        }
    }

    #[test]
    fn test_parse_annotation_with_mixed_args() {
        let ast = parse_schema(
            r#"
            @index(name, level, unique: true)
            table User {}
            "#,
        )
        .unwrap();

        if let Definition::Table(table) = &ast.definitions[0] {
            if let Metadata::Annotation(ann) = &table.metadata[0] {
                assert_eq!(ann.name, Some("index".to_string()));
                assert_eq!(ann.args.len(), 3);
                // First two are Positional
                assert!(matches!(&ann.args[0], AnnotationArg::Positional(_)));
                assert!(matches!(&ann.args[1], AnnotationArg::Positional(_)));
                // Third is Named
                if let AnnotationArg::Named(p) = &ann.args[2] {
                    assert_eq!(p.key, "unique");
                } else {
                    panic!("Expected Named argument for third arg");
                }
            } else {
                panic!("Expected Annotation metadata");
            }
        } else {
            panic!("Expected Table definition");
        }
    }

    // ========== Embed Tests ==========

    #[test]
    fn test_parse_embed_definition() {
        let ast = parse_schema(
            r#"
            embed Address {
                street: string;
                city: string;
            }
            "#,
        )
        .unwrap();

        if let Definition::Embed(embed) = &ast.definitions[0] {
            assert_eq!(embed.name, Some("Address".to_string()));
            assert_eq!(embed.members.len(), 2);
        } else {
            panic!("Expected Embed definition");
        }
    }

    #[test]
    fn test_parse_inline_embed_field() {
        let ast = parse_schema(
            r#"
            table User {
                profile: embed {
                    bio: string;
                    avatar: string;
                };
            }
            "#,
        )
        .unwrap();

        if let Definition::Table(table) = &ast.definitions[0] {
            if let TableMember::Field(FieldDefinition::InlineEmbed(ief)) = &table.members[0] {
                assert_eq!(ief.name, Some("profile".to_string()));
                assert_eq!(ief.members.len(), 2);
            } else {
                panic!("Expected InlineEmbed field");
            }
        } else {
            panic!("Expected Table definition");
        }
    }

    // ========== Inline Enum Tests ==========

    #[test]
    fn test_parse_inline_enum_field() {
        let ast = parse_schema(
            r#"
            table User {
                role: enum {
                    Admin
                    User
                    Guest
                };
            }
            "#,
        )
        .unwrap();

        if let Definition::Table(table) = &ast.definitions[0] {
            if let TableMember::Field(FieldDefinition::InlineEnum(ief)) = &table.members[0] {
                assert_eq!(ief.name, Some("role".to_string()));
                assert_eq!(ief.variants.len(), 3);
            } else {
                panic!("Expected InlineEnum field");
            }
        } else {
            panic!("Expected Table definition");
        }
    }

    // ========== Doc Comment Tests ==========

    #[test]
    fn test_parse_doc_comment_on_table() {
        let ast = parse_schema(
            r#"
            // This is a user table
            table User {}
            "#,
        )
        .unwrap();

        if let Definition::Table(table) = &ast.definitions[0] {
            assert_eq!(table.metadata.len(), 1);
            if let Metadata::DocComment(comment) = &table.metadata[0] {
                assert!(comment.contains("This is a user table"));
            } else {
                panic!("Expected DocComment metadata");
            }
        } else {
            panic!("Expected Table definition");
        }
    }

    #[test]
    fn test_parse_block_comment() {
        let ast = parse_schema(
            r#"
            /* This is a
               multi-line comment */
            table User {}
            "#,
        )
        .unwrap();

        if let Definition::Table(table) = &ast.definitions[0] {
            assert_eq!(table.metadata.len(), 1);
            if let Metadata::DocComment(comment) = &table.metadata[0] {
                assert!(comment.contains("multi-line"));
            } else {
                panic!("Expected DocComment metadata");
            }
        } else {
            panic!("Expected Table definition");
        }
    }

    // ========== Field Number Tests ==========

    #[test]
    fn test_parse_field_with_number() {
        let ast = parse_schema(
            r#"
            table User {
                id: i32 = 1;
                name: string = 2;
            }
            "#,
        )
        .unwrap();

        if let Definition::Table(table) = &ast.definitions[0] {
            if let TableMember::Field(FieldDefinition::Regular(rf)) = &table.members[0] {
                assert_eq!(rf.field_number, Some(1));
            } else {
                panic!("Expected Regular field");
            }
            if let TableMember::Field(FieldDefinition::Regular(rf)) = &table.members[1] {
                assert_eq!(rf.field_number, Some(2));
            } else {
                panic!("Expected Regular field");
            }
        } else {
            panic!("Expected Table definition");
        }
    }

    // ========== All Basic Types Tests ==========

    #[test]
    fn test_parse_all_basic_types() {
        let ast = parse_schema(
            r#"
            table AllTypes {
                a: string;
                b: i8;
                c: i16;
                d: i32;
                e: i64;
                f: u8;
                g: u16;
                h: u32;
                i: u64;
                j: f32;
                k: f64;
                l: bool;
                m: bytes;
            }
            "#,
        )
        .unwrap();

        if let Definition::Table(table) = &ast.definitions[0] {
            assert_eq!(table.members.len(), 13);

            let expected_types = [
                BasicType::String,
                BasicType::I8,
                BasicType::I16,
                BasicType::I32,
                BasicType::I64,
                BasicType::U8,
                BasicType::U16,
                BasicType::U32,
                BasicType::U64,
                BasicType::F32,
                BasicType::F64,
                BasicType::Bool,
                BasicType::Bytes,
            ];

            for (i, expected) in expected_types.iter().enumerate() {
                if let TableMember::Field(FieldDefinition::Regular(rf)) = &table.members[i] {
                    assert_eq!(rf.field_type.base_type, TypeName::Basic(expected.clone()));
                } else {
                    panic!("Expected Regular field at index {}", i);
                }
            }
        } else {
            panic!("Expected Table definition");
        }
    }

    // ========== Complex Schema Test ==========

    #[test]
    fn test_parse_complex_schema() {
        let ast = parse_schema(
            r#"
            import "common.poly";

            namespace game.character {
                // Status enum for characters
                enum Status {
                    Active = 1
                    Inactive = 2
                }

                @csv(name: "characters")
                table Character {
                    id: i32 primary_key;
                    name: string max_length(100);
                    status: Status;
                    stats: embed {
                        hp: i32;
                        mp: i32;
                    };
                    items: Item[];
                }
            }
            "#,
        )
        .unwrap();

        assert_eq!(ast.file_imports.len(), 1);
        assert_eq!(ast.definitions.len(), 1);

        if let Definition::Namespace(ns) = &ast.definitions[0] {
            assert_eq!(ns.path, vec!["game", "character"]);
            // Should have: comment, enum, table
            assert!(ns.definitions.len() >= 2);
        } else {
            panic!("Expected Namespace definition");
        }
    }

    // ========== Renames File Tests ==========

    #[test]
    fn test_parse_renames_file() {
        let input = r#"
# Table rename
Player -> User;

# Field renames
User.user_name -> name;
game.Player.hp -> health;
"#;
        let input = input.replace("\r\n", "\n");
        let renames_pair = Polygen::parse(Rule::renames_file, &input)
            .expect("Failed to parse renames file")
            .next()
            .expect("No renames_file pair");

        let renames = super::parse_renames_file(renames_pair).expect("Failed to build renames");

        assert_eq!(renames.len(), 3);

        // First rename: Player -> User
        assert_eq!(renames[0].from_path, vec!["Player"]);
        assert_eq!(renames[0].to_name, "User");

        // Second rename: User.user_name -> name
        assert_eq!(renames[1].from_path, vec!["User", "user_name"]);
        assert_eq!(renames[1].to_name, "name");

        // Third rename: game.Player.hp -> health
        assert_eq!(renames[2].from_path, vec!["game", "Player", "hp"]);
        assert_eq!(renames[2].to_name, "health");
    }

    // ========== Timestamp and Auto-Timestamp Tests ==========

    #[test]
    fn test_parse_timestamp_type() {
        let ast = parse_schema(
            r#"
            table Event {
                created_at: timestamp;
            }
            "#,
        )
        .unwrap();

        if let Definition::Table(table) = &ast.definitions[0] {
            if let TableMember::Field(FieldDefinition::Regular(rf)) = &table.members[0] {
                assert_eq!(
                    rf.field_type.base_type,
                    TypeName::Basic(BasicType::Timestamp)
                );
            } else {
                panic!("Expected Regular field");
            }
        } else {
            panic!("Expected Table definition");
        }
    }

    #[test]
    fn test_parse_auto_create_default() {
        let ast = parse_schema(
            r#"
            table Event {
                created_at: timestamp auto_create;
            }
            "#,
        )
        .unwrap();

        if let Definition::Table(table) = &ast.definitions[0] {
            if let TableMember::Field(FieldDefinition::Regular(rf)) = &table.members[0] {
                assert_eq!(rf.constraints.len(), 1);
                assert!(matches!(rf.constraints[0], Constraint::AutoCreate(None)));
            } else {
                panic!("Expected Regular field");
            }
        } else {
            panic!("Expected Table definition");
        }
    }

    #[test]
    fn test_parse_auto_update_utc() {
        let ast = parse_schema(
            r#"
            table Event {
                updated_at: timestamp auto_update(utc);
            }
            "#,
        )
        .unwrap();

        if let Definition::Table(table) = &ast.definitions[0] {
            if let TableMember::Field(FieldDefinition::Regular(rf)) = &table.members[0] {
                assert_eq!(rf.constraints.len(), 1);
                if let Constraint::AutoUpdate(Some(tz)) = &rf.constraints[0] {
                    assert_eq!(*tz, Timezone::Utc);
                } else {
                    panic!("Expected AutoUpdate with UTC timezone");
                }
            } else {
                panic!("Expected Regular field");
            }
        } else {
            panic!("Expected Table definition");
        }
    }

    #[test]
    fn test_parse_auto_create_offset() {
        let ast = parse_schema(
            r#"
            table Event {
                korea_time: timestamp auto_create(+9);
                india_time: timestamp auto_create(+5:30);
                ny_time: timestamp auto_create(-5);
            }
            "#,
        )
        .unwrap();

        if let Definition::Table(table) = &ast.definitions[0] {
            // Korea time: +9
            if let TableMember::Field(FieldDefinition::Regular(rf)) = &table.members[0] {
                if let Constraint::AutoCreate(Some(Timezone::Offset { hours, minutes })) =
                    &rf.constraints[0]
                {
                    assert_eq!(*hours, 9);
                    assert_eq!(*minutes, 0);
                } else {
                    panic!("Expected AutoCreate with +9 offset");
                }
            }

            // India time: +5:30
            if let TableMember::Field(FieldDefinition::Regular(rf)) = &table.members[1] {
                if let Constraint::AutoCreate(Some(Timezone::Offset { hours, minutes })) =
                    &rf.constraints[0]
                {
                    assert_eq!(*hours, 5);
                    assert_eq!(*minutes, 30);
                } else {
                    panic!("Expected AutoCreate with +5:30 offset");
                }
            }

            // NY time: -5
            if let TableMember::Field(FieldDefinition::Regular(rf)) = &table.members[2] {
                if let Constraint::AutoCreate(Some(Timezone::Offset { hours, minutes })) =
                    &rf.constraints[0]
                {
                    assert_eq!(*hours, -5);
                    assert_eq!(*minutes, 0);
                } else {
                    panic!("Expected AutoCreate with -5 offset");
                }
            }
        } else {
            panic!("Expected Table definition");
        }
    }

    #[test]
    fn test_parse_auto_create_named_timezone() {
        let ast = parse_schema(
            r#"
            table Event {
                korea_time: timestamp auto_create("Korea Standard Time");
            }
            "#,
        )
        .unwrap();

        if let Definition::Table(table) = &ast.definitions[0] {
            if let TableMember::Field(FieldDefinition::Regular(rf)) = &table.members[0] {
                if let Constraint::AutoCreate(Some(Timezone::Named(name))) = &rf.constraints[0] {
                    assert_eq!(name, "Korea Standard Time");
                } else {
                    panic!("Expected AutoCreate with named timezone");
                }
            } else {
                panic!("Expected Regular field");
            }
        } else {
            panic!("Expected Table definition");
        }
    }

    #[test]
    fn test_parse_auto_update_local() {
        let ast = parse_schema(
            r#"
            table Event {
                local_time: timestamp auto_update(local);
            }
            "#,
        )
        .unwrap();

        if let Definition::Table(table) = &ast.definitions[0] {
            if let TableMember::Field(FieldDefinition::Regular(rf)) = &table.members[0] {
                if let Constraint::AutoUpdate(Some(tz)) = &rf.constraints[0] {
                    assert_eq!(*tz, Timezone::Local);
                } else {
                    panic!("Expected AutoUpdate with local timezone");
                }
            } else {
                panic!("Expected Regular field");
            }
        } else {
            panic!("Expected Table definition");
        }
    }
}
