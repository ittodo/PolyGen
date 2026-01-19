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

use crate::ast_model::{AstRoot, Definition};
use crate::error::AstBuildError;
use crate::Rule;

use definitions::parse_definition;
use helpers::extract_comment_content;

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
                assert!(ann.params.is_empty());
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
                assert_eq!(ann.params.len(), 2);
                assert_eq!(ann.params[0].key, "name");
                assert_eq!(ann.params[1].key, "delimiter");
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
            /// This is a user table
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
}
