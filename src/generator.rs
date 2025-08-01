use crate::ast::{Cardinality, Constraint, Definition, Embed, FieldDefinition, TableMember, TypeName, Literal, TypeWithCardinality};
use crate::ir_model::{SchemaContext, TypeDef};
use anyhow::{Context, Result};
use minijinja::{path_loader, Environment, Error};
use once_cell::sync::Lazy;
use regex::Regex;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use heck::ToUpperCamelCase;

/// A generic code generator using the minijinja template engine.
pub struct Generator<'a> {
    env: Environment<'a>,
}

impl<'a> Generator<'a> {
    /// Creates a new generator and sets up the template environment.
    pub fn new(template_dir: &Path) -> Result<Self> {
        let mut env = Environment::new();
        env.set_loader(path_loader(template_dir));
        env.add_filter("map_type", map_type_filter);
        env.add_filter("pascal_case", |s: &str| -> Result<String, Error> {
            Ok(heck::ToPascalCase::to_pascal_case(s))
        });
        env.add_filter("starts_with", |value: String, prefix: String| -> Result<bool, Error> {
            Ok(value.starts_with(&prefix))
        });
        Ok(Self { env })
    }

    /// Generates code for a given language based on the IR context.
    pub fn generate(&self, context: &SchemaContext, lang: &str, output_dir: &Path) -> Result<()> {
        fs::create_dir_all(output_dir)
            .with_context(|| format!("Failed to create output directory: {:?}", output_dir))?;

        // Generate one file per namespace.
        for (ns_name, ns_def) in &context.namespaces {
            // Prepare the context for the main template.
            let mut render_ctx = minijinja::context! {
                ns => ns_def,
                lang => lang,
            };

            let template_name = format!("{}/csharp_file.jinja", lang);
            let template = self.env.get_template(&template_name)?;

            let rendered_code = template.render(&mut render_ctx)?;

            let extension = match lang {
                "csharp" => "cs",
                "typescript" => "ts",
                _ => lang, // Fallback to using the language name as extension
            };

            let file_name = if ns_name.is_empty() {
                format!("GlobalTypes.{}", extension)
            } else {
                format!("{}.{}", ns_name.replace('.', "/"), extension)
            };

            let output_path = output_dir.join(file_name);
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&output_path, rendered_code)?;
            println!("Generated file: {}", output_path.display());

            // --- Generate Loader classes for C# if @load annotation is present ---
            if lang == "csharp" {
                for type_def in &ns_def.types {
                    if let TypeDef::Struct(struct_def) = type_def {
                        // Check for @load annotation
                        let has_load_annotation = struct_def.annotations.iter().any(|ann| ann.name == "load");

                        if has_load_annotation {
                            let loader_template_name = "csharp/csharp_loader.jinja";
                            let loader_template = self.env.get_template(loader_template_name)?;

                            let mut loader_render_ctx = minijinja::context! {
                                namespace => ns_name,
                                table => struct_def, // Pass the struct_def as 'table' to the template
                                lang => lang,
                            };

                            let rendered_loader_code = loader_template.render(&mut loader_render_ctx)?;

                            let loader_file_name = if ns_name.is_empty() {
                                format!("{}Loader.cs", struct_def.name)
                            } else {
                                format!("{}/{}Loader.cs", ns_name.replace('.', "/"), struct_def.name)
                            };

                            let loader_output_path = output_dir.join(loader_file_name);
                            if let Some(parent) = loader_output_path.parent() {
                                fs::create_dir_all(parent)?;
                            }
                            fs::write(&loader_output_path, rendered_loader_code)?;
                            println!("Generated loader file: {}", loader_output_path.display());
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Generates a Mermaid class diagram from the AST definitions.
    pub fn generate_mermaid_diagram(&self, ast_definitions: &[Definition]) -> Result<String> {
        let diagram = build_diagram_from_ast(ast_definitions);
        let template = self.env.get_template("mermaid/class_diagram.mmd.jinja")?;
        let rendered_code = template.render(minijinja::context! { diagram => diagram })?;
        Ok(rendered_code)
    }
}

// Use once_cell to compile regexes only once for performance and safety.
static RE_LIST: Lazy<Regex> = Lazy::new(|| Regex::new(r"List<([^>]+)>?").unwrap());
static RE_OPTION: Lazy<Regex> = Lazy::new(|| Regex::new(r"Option<([^>]+)>?").unwrap());

/// A custom minijinja filter to map language-agnostic types to language-specific types.
fn map_type_filter(poly_type: String, lang: String) -> Result<String, Error> {
    let base_type = if let Some(caps) = RE_LIST.captures(&poly_type) {
        let inner = map_type_filter(caps[1].to_string(), lang.clone())?;
        return Ok(match lang.as_str() {
            "csharp" => format!("System.Collections.Generic.List<{}>", inner),
            _ => format!("Array<{}>", inner),
        });
    } else if let Some(caps) = RE_OPTION.captures(&poly_type) {
        let inner = map_type_filter(caps[1].to_string(), lang.clone())?;
        return Ok(match lang.as_str() {
            "csharp" => format!("{}?", inner), // Only for value types, but simplified here.
            _ => format!("{} | null", inner),
        });
    } else {
        poly_type
    };

    let mapped_type = match lang.as_str() {
        "csharp" => match base_type.as_str() {
            "u32" => "uint".to_string(),
            "string" => "string".to_string(),
            "bool" => "bool".to_string(),
            // ... other basic types
            _ => base_type, // Custom types are assumed to be correct.
        },
        // Add other languages here
        // "typescript" => { ... }
        _ => base_type, // Default: return as-is
    };

    Ok(mapped_type)
}

/// Mermaid 클래스 다이어그램 생성을 위한 데이터 모델입니다.
#[derive(Debug, Default, serde::Serialize)]
pub struct ClassDiagram<'a> {
    pub classes: Vec<Class<'a>>,
    pub enums: Vec<Enum<'a>>,
    pub relationships: Vec<Relationship>, // Direct relationships (e.g., field references, FKs)
    #[serde(skip)] // Skip this field during serialization
    pub foreign_keys_for_reverse_lookup: Vec<(String, &'a Constraint)>, // (owner_fqn, ForeignKeyConstraint)
}

#[derive(Debug, serde::Serialize)]
pub struct Class<'a> {
    pub fqn: String, // Fully Qualified Name (e.g., "game.character.Player")
    pub name: &'a str,
    pub properties: Vec<Property<'a>>,
    pub annotations: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct Enum<'a> {
    pub fqn: String,
    pub name: &'a str,
    pub variants: Vec<&'a str>,
}

#[derive(Debug, serde::Serialize)]
pub struct Property<'a> {
    pub name: &'a str,
    pub type_name: String, // e.g., "string", "List<Position>"
}

#[derive(Debug, serde::Serialize)]
pub struct Relationship {
    pub from: String,
    pub from_cardinality: String, // e.g., "1", "0..1", "*"
    pub to: String,
    pub to_cardinality: String, // e.g., "1", "0..1", "*"
    pub link_type: String,      // e.g., "--", "-->", "<--"
    pub label: String,
}

/// AST를 기반으로 Mermaid 클래스 다이어그램을 생성합니다.
fn build_diagram_from_ast(ast_definitions: &[Definition]) -> ClassDiagram {
    let mut all_named_embed_fqns = HashSet::new(); // Renamed for clarity
    find_all_named_embed_fqns(
        // Renamed function
        ast_definitions,
        &mut Vec::new(),
        &mut all_named_embed_fqns,
    );

    let mut diagram = ClassDiagram::default();
    collect_diagram_parts(
        ast_definitions,
        &mut Vec::new(),
        &mut diagram,
        &all_named_embed_fqns, // Pass the set of all named embeds
    );

    // Process reverse relationships from collected foreign keys
    let mut reverse_relationships = Vec::new();
    for (owner_fqn, fk_constraint) in &diagram.foreign_keys_for_reverse_lookup {
        if let Constraint::ForeignKey(target_path, Some(alias)) = fk_constraint {
            // The target path is like ["game", "character", "Player", "id"]
            // We need the FQN of the table, which is all but the last element.
            let target_table_fqn = target_path[0..target_path.len() - 1].join(".");

            // This is the reverse relationship: from target table to owner table (junction table)
            // Example: Player "1" -- "*" PlayerSkill : skills
            reverse_relationships.push(Relationship {
                from: target_table_fqn,
                from_cardinality: "1".to_string(), // One instance of the target table
                to: owner_fqn.clone(),
                to_cardinality: "*".to_string(), // Many junction table entries can point to one target
                link_type: "--".to_string(),
                label: alias.clone(),
            });
        }
    }
    diagram.relationships.extend(reverse_relationships);

    diagram
}

/// Helper function to process a named embed definition and add it to the diagram as a class. (Namespace-level or Table-internal)
fn process_named_embed_to_class<'a>(
    embed: &'a Embed,
    path: &mut Vec<String>,
    diagram: &mut ClassDiagram<'a>,
    all_named_embed_fqns: &HashSet<String>, // This is the set of ALL named embeds
) {
    path.push(embed.name.clone());
    let fqn = path.join(".");
    let properties = embed
        .fields
        .iter()
        .map(|field| process_field(field, &fqn, all_named_embed_fqns).0)
        .collect();

    diagram.classes.push(Class {
        fqn,
        name: &embed.name,
        properties,
        annotations: vec![], // Embeds do not have annotations
    });
    path.pop();
}

/// AST 노드를 재귀적으로 탐색하여 다이어그램 구성 요소를 수집합니다.
fn collect_diagram_parts<'a>(
    // Renamed parameter for clarity
    definitions: &'a [Definition],
    path: &mut Vec<String>,
    diagram: &mut ClassDiagram<'a>,
    all_named_embed_fqns: &HashSet<String>, // This is the set of ALL named embeds
) {
    for def in definitions {
        match def {
            Definition::Namespace(ns) => {
                path.extend(ns.path.iter().cloned());
                collect_diagram_parts(&ns.definitions, path, diagram, all_named_embed_fqns);
                for _ in 0..ns.path.len() {
                    path.pop();
                }
            }
            Definition::Table(table) => {
                path.push(table.name.clone());
                let fqn = path.join(".");

                let annotations: Vec<String> = table
                    .annotations
                    .iter()
                    .map(|ann| {
                        let formatted_params = ann
                            .params
                            .iter()
                            .map(|p| {
                                let value_str = match &p.value {
                                    Literal::String(s) => s.clone(), // Get inner string without quotes
                                    _ => p.value.to_string(), // Use Display impl for others
                                };
                                format!("{}: {}", p.key, value_str)
                            })
                            .collect::<Vec<_>>()
                            .join(", ");

                        if ann.params.is_empty() {
                            ann.name.clone()
                        } else {
                            // Special handling for 'load' and 'save' to format as (Type: Path)
                            if ann.name == "load" || ann.name == "save" {
                                // Assuming 'type' and 'path' are always present for load/save
                                let type_param = ann
                                    .params
                                    .iter()
                                    .find(|p| p.key == "type")
                                    .map_or("unknown".to_string(), |p| match &p.value {
                                        Literal::String(s) => s.clone(),
                                        _ => p.value.to_string(),
                                    });
                                format!("{}({})", ann.name, type_param)
                            } else {
                                format!("{}({})", ann.name, formatted_params)
                            }
                        }
                    })
                    .collect();

                let mut properties = Vec::new();
                for member in &table.members {
                    match member {
                        TableMember::Field(field) => {
                            let (prop, rel) = process_field(field, &fqn, all_named_embed_fqns);
                            properties.push(prop);
                            if let Some(r) = rel {
                                diagram.relationships.push(r);
                            }

                            // Collect foreign keys with 'as' alias for reverse relationship generation
                            if let FieldDefinition::Regular(rf) = field {
                                for constraint in &rf.constraints {
                                    if let Constraint::ForeignKey(_, Some(_)) = constraint {
                                        diagram
                                            .foreign_keys_for_reverse_lookup
                                            .push((fqn.clone(), constraint));
                                    }
                                }
                            }
                        }
                        TableMember::Embed(embed) => {
                            // Named embeds inside tables are rendered as separate classes.
                            process_named_embed_to_class(
                                embed,
                                path,
                                diagram,
                                all_named_embed_fqns,
                            );
                        }
                    }
                }

                diagram.classes.push(Class {
                    fqn,
                    name: &table.name,
                    properties,
                    annotations,
                });
                path.pop();
            }
            Definition::Enum(e) => {
                // Enums are always rendered as separate classes
                path.push(e.name.clone());
                diagram.enums.push(Enum {
                    fqn: path.join("."),
                    name: &e.name,
                    variants: e.variants.iter().map(|s| s.name.as_str()).collect(),
                });
                path.pop();
            }
            Definition::Embed(embed) => {
                // Namespace-level embeds are rendered as separate classes
                process_named_embed_to_class(embed, path, diagram, all_named_embed_fqns);
            }
        }
    }
}

/// Recursively find all named embed definitions (namespace-level and table-internal)
/// for the purpose of identifying them as classes and preventing direct relationships to them.
fn find_all_named_embed_fqns(
    // Renamed function for clarity
    definitions: &[Definition],
    path: &mut Vec<String>,
    all_embed_fqns: &mut HashSet<String>, // This set will contain FQNs of ALL named embeds
) {
    for def in definitions {
        match def {
            Definition::Namespace(ns) => {
                path.extend(ns.path.iter().cloned());
                find_all_named_embed_fqns(&ns.definitions, path, all_embed_fqns);
                for _ in 0..ns.path.len() {
                    path.pop();
                }
            }
            Definition::Embed(embed) => {
                path.push(embed.name.clone());
                all_embed_fqns.insert(path.join("."));
                path.pop();
            }
            Definition::Table(table) => {
                path.push(table.name.clone());
                for member in &table.members {
                    if let TableMember::Embed(embed) = member {
                        path.push(embed.name.clone());
                        all_embed_fqns.insert(path.join("."));
                        path.pop();
                    }
                }
                path.pop();
            }
            _ => {}
        }
    }
}

/// AST 필드를 다이어그램 속성 및 관계로 변환합니다.
fn process_field<'a>(
    field: &'a FieldDefinition,
    owner_fqn: &str,
    _all_named_embed_fqns: &HashSet<String>, // Renamed parameter for clarity
) -> (Property<'a>, Option<Relationship>) {
    match field {
        FieldDefinition::Regular(rf) => {
            let type_name = format_type_name(&rf.field_type);
            let prop = Property {
                name: &rf.name,
                type_name,
            };

            // 1. Prioritize ForeignKey constraint for relationship generation
            for constraint in &rf.constraints {
                if let Constraint::ForeignKey(target_path, alias) = constraint {
                    let target_table_fqn = target_path[0..target_path.len() - 1].join(".");

                    let rel = Some(Relationship {
                        from: owner_fqn.to_string(),
                        from_cardinality: get_mermaid_cardinality_string(
                            &rf.field_type.cardinality,
                        ),
                        to: target_table_fqn,
                        to_cardinality: "1".to_string(), // The FK points to a single primary key
                        link_type: "--".to_string(),
                        label: alias.as_ref().unwrap_or(&rf.name).clone(),
                    });
                    return (prop, rel); // Return immediately if FK found
                }
            }

            // 2. If no ForeignKey, check if the field's type is a path (reference to another custom type)
            if let TypeName::Path(p) = &rf.field_type.base_type {
                let target_fqn = p.join(".");
                // Always create a relationship for named embed types (like StatBlock, Position)
                // Use composition (*--) for these relationships to indicate ownership/part-of.

                let rel = Some(Relationship {
                    from: owner_fqn.to_string(),
                    to: target_fqn,
                    from_cardinality: get_mermaid_cardinality_string(&rf.field_type.cardinality),
                    to_cardinality: "1".to_string(), // Assuming direct reference points to a single instance
                    link_type: "*--".to_string(),    // Use composition for embeds
                    label: rf.name.clone(),
                });
                return (prop, rel); // Return immediately if Path type found
            }

            // 3. If neither, return None for relationship
            (prop, None)
        }
        FieldDefinition::InlineEmbed(ief) => {
            // 인라인 embed는 별도의 클래스로 만들지 않고, 부모 클래스의 속성으로 직접 표시합니다.
            // Mermaid에서는 복합 타입을 직접 속성으로 표현하기 어렵기 때문에,
            // 타입 이름을 "EmbedName" 또는 "List<EmbedName>" 형태로 표시합니다.
            let class_name = ief.name.to_upper_camel_case(); // Use the field name as the base for the type name
            let type_name = format_cardinality_type(&class_name, &ief.cardinality);

            let prop = Property {
                name: &ief.name,
                type_name,
            };
            // 인라인 임베드는 별도의 관계를 생성하지 않습니다.
            (prop, None)
        }
    }
}

fn format_type_name(t: &TypeWithCardinality) -> String {
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

fn get_mermaid_cardinality_string(c: &Option<Cardinality>) -> String {
    match c {
        Some(Cardinality::Optional) => String::from("0..1"),
        Some(Cardinality::Array) => String::from("*"), // N
        None => String::from("1"),
    }
}
