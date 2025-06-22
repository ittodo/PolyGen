use crate::ast::{
    self, BasicType, Cardinality, Definition, Embed, FieldDefinition, TableMember, TypeName,
};
use crate::mermaid_model::{Class, ClassDiagram, Enum, Property, Relationship};
use askama::Template;
use heck::ToUpperCamelCase;

#[derive(Template)]
#[template(path = "mermaid/class_diagram.mmd.txt", escape = "none")]
struct MermaidTemplate<'a> {
    diagram: ClassDiagram<'a>,
}

/// AST를 기반으로 Mermaid 클래스 다이어그램을 생성합니다.
pub fn generate_mermaid_diagram(ast_definitions: &[Definition]) -> String {
    let diagram = build_diagram_from_ast(ast_definitions);
    let template = MermaidTemplate { diagram };
    template.render().unwrap()
}

/// AST를 순회하며 `ClassDiagram` 데이터 모델을 구축합니다.
fn build_diagram_from_ast(ast_definitions: &[Definition]) -> ClassDiagram {
    let mut diagram = ClassDiagram::default();
    collect_diagram_parts(ast_definitions, &mut Vec::new(), &mut diagram);
    diagram
}

/// Helper function to process a named embed definition and add it to the diagram as a class.
fn process_embed_definition<'a>(
    embed: &'a Embed,
    path: &mut Vec<String>,
    diagram: &mut ClassDiagram<'a>,
) {
    path.push(embed.name.clone());
    let fqn = path.join(".");
    let properties = embed
        .fields
        .iter()
        .map(|field| process_field(field, &fqn).0)
        .collect();

    diagram.classes.push(Class {
        fqn,
        name: &embed.name,
        properties,
    });
    path.pop();
}

/// AST 노드를 재귀적으로 탐색하여 다이어그램 구성 요소를 수집합니다.
fn collect_diagram_parts<'a>(
    definitions: &'a [Definition],
    path: &mut Vec<String>,
    diagram: &mut ClassDiagram<'a>,
) {
    for def in definitions {
        match def {
            Definition::Namespace(ns) => {
                path.extend(ns.path.iter().cloned());
                collect_diagram_parts(&ns.definitions, path, diagram);
                for _ in 0..ns.path.len() {
                    path.pop();
                }
            }
            Definition::Table(table) => {
                path.push(table.name.clone());
                let fqn = path.join(".");

                let mut properties = Vec::new();
                for member in &table.members {
                    match member {
                        TableMember::Field(field) => {
                            let (prop, rel) = process_field(field, &fqn);
                            properties.push(prop);
                            if let Some(r) = rel {
                                diagram.relationships.push(r);
                            }
                        }
                        TableMember::Embed(embed) => {
                            // 명명된 embed 정의는 클래스로 취급합니다.
                            process_embed_definition(embed, path, diagram);
                        }
                    }
                }

                diagram.classes.push(Class {
                    fqn,
                    name: &table.name,
                    properties,
                });
                path.pop();
            }
            Definition::Enum(e) => {
                path.push(e.name.clone());
                diagram.enums.push(Enum {
                    fqn: path.join("."),
                    name: &e.name,
                    variants: e.variants.iter().map(|s| s.as_str()).collect(),
                });
                path.pop();
            }
            Definition::Embed(embed) => {
                process_embed_definition(embed, path, diagram);
            }
        }
    }
}

/// AST 필드를 다이어그램 속성 및 관계로 변환합니다.
fn process_field<'a>(
    field: &'a FieldDefinition,
    owner_fqn: &str,
) -> (Property<'a>, Option<Relationship>) {
    match field {
        FieldDefinition::Regular(rf) => {
            let type_name = format_type_name(&rf.field_type);
            let prop = Property {
                name: &rf.name,
                type_name,
            };

            // 타입이 다른 클래스/임베드/열거형을 참조하는 경우 관계를 생성합니다.
            let rel = if let TypeName::Path(p) = &rf.field_type.base_type {
                Some(Relationship {
                    from: owner_fqn.to_string(),
                    to: p.join("."),
                    link: format_cardinality_link(&rf.field_type.cardinality),
                    label: rf.name.clone(),
                })
            } else {
                None
            };

            (prop, rel)
        }
        FieldDefinition::InlineEmbed(ief) => {
            // 인라인 embed는 중첩 클래스로 취급합니다.
            let class_name = ief.name.to_upper_camel_case();
            let fqn = format!("{}.{}", owner_fqn, class_name);
            let type_name = format_cardinality_type(&class_name, &ief.cardinality);

            let prop = Property {
                name: &ief.name,
                type_name,
            };
            let rel = Some(Relationship {
                from: owner_fqn.to_string(),
                to: fqn,
                link: format_cardinality_link(&ief.cardinality),
                label: ief.name.clone(),
            });

            (prop, rel)
        }
    }
}

fn format_type_name(t: &ast::TypeWithCardinality) -> String {
    let base = match &t.base_type {
        TypeName::Path(p) => p.last().unwrap().clone(),
        TypeName::Basic(b) => format!("{:?}", b).to_lowercase(),
    };
    format_cardinality_type(&base, &t.cardinality)
}

fn format_cardinality_type(base: &str, c: &Option<Cardinality>) -> String {
    match c {
        Some(Cardinality::Optional) => format!("{}?", base),
        Some(Cardinality::Array) => format!("List<{}>", base),
        None => base.to_string(),
    }
}

fn format_cardinality_link(c: &Option<Cardinality>) -> String {
    match c {
        Some(Cardinality::Optional) => String::from("-- \"0..1\""),
        Some(Cardinality::Array) => String::from("-- \"*\""),
        None => String::from("-- \"1\""),
    }
}
