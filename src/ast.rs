use pest::iterators::Pair;
use std::fmt;
use std::path::PathBuf;

use crate::error::AstBuildError;
use crate::Rule;

/// Represents the entire content of a single schema file.
#[derive(Debug, PartialEq, Clone, Default)]
pub struct AstRoot {
    pub path: PathBuf,
    pub file_imports: Vec<String>,
    pub definitions: Vec<Definition>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Definition {
    Namespace(Namespace),
    Table(Table),
    Enum(Enum),
    Embed(Embed),
    Comment(String),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Namespace {
    pub path: Vec<String>,
    pub imports: Vec<NamespaceImport>,
    pub definitions: Vec<Definition>,
}

/// Represents a `import game.common.*;` or `import game.common.Type;` statement.
#[derive(Debug, PartialEq, Clone)]
pub struct NamespaceImport {
    pub path: Vec<String>,
    pub all: bool, // true for `.*`
}

#[derive(Debug, PartialEq, Clone)]
pub struct Table {
    pub doc_comment: Option<String>,
    pub annotations: Vec<Annotation>,
    pub name: String,
    pub members: Vec<TableMember>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum TableMember {
    Field(FieldDefinition),
    Embed(Embed), // Named embed definition within a table
    Comment(String),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Annotation {
    pub name: String,
    pub params: Vec<AnnotationParam>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct AnnotationParam {
    pub key: String,
    pub value: Literal,
}

#[derive(Debug, PartialEq, Clone)]
pub enum FieldDefinition {
    Regular(RegularField),
    InlineEmbed(InlineEmbedField),
}

#[derive(Debug, PartialEq, Clone)]
pub struct RegularField {
    pub doc_comment: Option<String>,
    pub annotations: Vec<Annotation>,
    pub name: String,
    pub field_type: TypeWithCardinality,
    pub constraints: Vec<Constraint>,
    pub field_number: Option<u32>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct TypeWithCardinality {
    pub base_type: TypeName,
    pub cardinality: Option<Cardinality>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum TypeName {
    Path(Vec<String>),
    Basic(BasicType),
}

#[derive(Debug, PartialEq, Clone)]
pub enum Cardinality {
    Optional, // ?
    Array,    // []
}

#[derive(Debug, PartialEq, Clone)]
pub enum BasicType {
    String,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    Bool,
    Bytes,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Constraint {
    PrimaryKey,
    Unique,
    MaxLength(u32),
    Default(Literal),
    Range(Literal, Literal),
    Regex(String),
    ForeignKey(Vec<String>, Option<String>), // path, optional 'as' identifier
}

#[derive(Debug, PartialEq, Clone)]
pub struct InlineEmbedField {
    pub doc_comment: Option<String>,
    pub annotations: Vec<Annotation>,
    pub name: String,
    pub fields: Vec<FieldDefinition>,
    pub cardinality: Option<Cardinality>,
    pub field_number: Option<u32>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct EnumVariant {
    pub doc_comment: Option<String>,
    pub annotations: Vec<Annotation>,
    pub name: String,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Enum {
    pub doc_comment: Option<String>,
    pub annotations: Vec<Annotation>,
    pub name: String,
    pub variants: Vec<EnumVariant>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Embed {
    pub doc_comment: Option<String>,
    pub annotations: Vec<Annotation>,
    pub name: String,
    pub fields: Vec<FieldDefinition>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Literal {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Identifier(String), // For annotation_param values that are identifiers
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Literal::String(s) => write!(f, "{}", s),
            Literal::Integer(i) => write!(f, "{}", i),
            Literal::Float(fl) => write!(f, "{}", fl),
            Literal::Boolean(b) => write!(f, "{}", b),
            Literal::Identifier(id) => write!(f, "{}", id),
        }
    }
}

// Helper function to parse a path (e.g., "game.common")
fn parse_path(pair: Pair<Rule>) -> Vec<String> {
    pair.into_inner()
        .filter(|p| p.as_rule() == Rule::IDENT)
        .map(|p| p.as_str().to_string())
        .collect()
}

// Helper function to parse a literal value
fn parse_literal(pair: Pair<Rule>) -> Result<Literal, AstBuildError> {
    // A literal can be passed as a wrapper pair (e.g., from a `default` value)
    // or as a direct token pair (e.g., from an annotation parameter).
    // This handles both cases by consuming the pair to get an inner token,
    // or falling back to a clone of the original pair if it has no inner token.
    let cloned_pair = pair.clone();
    let token_pair = pair.into_inner().next().unwrap_or(cloned_pair); // This is safe
    let rule = token_pair.as_rule();
    let text = token_pair.as_str();

    let (line, col) = token_pair.line_col();

    let literal = match rule {
        Rule::STRING_LITERAL => {
            // Remove quotes and handle escaped characters (basic for now)
            Literal::String(
                text[1..text.len() - 1]
                    .replace("\\\"", "\"")
                    .replace("\\\\", "\\"),
            )
        }
        Rule::INTEGER => {
            Literal::Integer(text.parse().map_err(|_| AstBuildError::InvalidValue {
                element: "integer".to_string(),
                value: text.to_string(),
                line,
                col,
            })?)
        }
        Rule::FLOAT => Literal::Float(text.parse().map_err(|_| AstBuildError::InvalidValue {
            element: "float".to_string(),
            value: text.to_string(),
            line,
            col,
        })?),
        Rule::BOOLEAN => {
            Literal::Boolean(text.parse().map_err(|_| AstBuildError::InvalidValue {
                element: "boolean".to_string(),
                value: text.to_string(),
                line,
                col,
            })?)
        }
        Rule::IDENT => Literal::Identifier(text.to_string()),
        _ => {
            return Err(AstBuildError::UnexpectedRule {
                expected: "a literal value".to_string(),
                found: rule,
                line,
                col,
            })
        }
    };
    Ok(literal)
}

fn extract_comment_content(comment_pair: Pair<Rule>) -> String {
    comment_pair
        .as_str()
        .trim_start_matches("///")
        .trim()
        .to_string()
}

// Helper function to parse doc comments from a stream of pairs.
// It consumes `doc_comment` rules from the beginning of the stream.
fn parse_doc_comments(
    inner_pairs: &mut std::iter::Peekable<pest::iterators::Pairs<Rule>>,
) -> Option<String> {
    let mut doc_comments = Vec::new();
    while let Some(p) = inner_pairs.peek() {
        if p.as_rule() == Rule::doc_comment {
            // Consume the pair
            doc_comments.push(extract_comment_content(inner_pairs.next().unwrap()));
        } else {
            break;
        }
    }

    if !doc_comments.is_empty() {
        Some(doc_comments.join("\n"))
    } else {
        None
    }
}

// Helper function to parse annotations from a stream of pairs.
fn parse_annotations(
    inner_pairs: &mut std::iter::Peekable<pest::iterators::Pairs<Rule>>,
) -> Result<Vec<Annotation>, AstBuildError> {
    let mut annotations = Vec::new();
    while let Some(p) = inner_pairs.peek() {
        if p.as_rule() == Rule::annotation {
            // Consume the pair and parse it
            annotations.push(parse_annotation(inner_pairs.next().unwrap())?);
        } else {
            break;
        }
    }
    Ok(annotations)
}

// Main function to build the AST
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
                        let path_literal = item_pair.into_inner().next().unwrap(); // STRING_LITERAL
                        let path_str = path_literal.as_str();
                        // Remove quotes
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
            Rule::EOI => (), // Skip End of Input
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

fn parse_definition(pair: Pair<Rule>) -> Result<Definition, AstBuildError> {
    let (_line, _col) = pair.line_col();
    let mut inner_pairs = pair.into_inner().peekable();
    let doc_comment = parse_doc_comments(&mut inner_pairs);
    let annotations = parse_annotations(&mut inner_pairs)?;

    let def_pair = inner_pairs.next().unwrap(); // Safe due to grammar
    let (inner_line, inner_col) = def_pair.line_col();

    let mut definition = match def_pair.as_rule() {
        Rule::namespace => Definition::Namespace(parse_namespace(def_pair)?),
        Rule::table => Definition::Table(parse_table(def_pair)?),
        Rule::enum_def => Definition::Enum(parse_enum(def_pair)?),
        Rule::embed_def => Definition::Embed(parse_embed(def_pair)?),
        found => {
            return Err(AstBuildError::UnexpectedRule {
                expected: "namespace, table, enum, or embed".to_string(),
                found,
                line: inner_line,
                col: inner_col,
            })
        }
    };

    // Attach the doc comment to the definition
    match &mut definition {
        Definition::Table(t) => {
            t.doc_comment = doc_comment;
            t.annotations = annotations;
        }
        Definition::Enum(e) => {
            e.doc_comment = doc_comment;
            e.annotations = annotations;
        }
        Definition::Embed(e) => {
            e.doc_comment = doc_comment;
            e.annotations = annotations;
        }
        Definition::Namespace(_) => {} // Comments on namespaces can be added later if needed
        Definition::Comment(_) => {}   // Comments don't have annotations or other comments
    }
    Ok(definition)
}

fn parse_namespace(pair: Pair<Rule>) -> Result<Namespace, AstBuildError> {
    let (line, col) = pair.line_col();
    let mut inner = pair.into_inner();
    let path_pair = inner.next().ok_or(AstBuildError::MissingElement {
        rule: Rule::namespace,
        element: "path".to_string(),
        line,
        col,
    })?;
    let path = parse_path(path_pair);
    let mut imports = Vec::new();
    let mut definitions = Vec::new();

    for p in inner {
        if p.as_rule() == Rule::namespace_body_item {
            let item_pair = p.into_inner().next().unwrap(); // Safe due to grammar
            match item_pair.as_rule() {
                Rule::namespace_import => {
                    imports.push(parse_namespace_import(item_pair)?);
                }
                Rule::definition => {
                    definitions.push(parse_definition(item_pair)?);
                }
                Rule::doc_comment => {
                    definitions.push(Definition::Comment(extract_comment_content(item_pair)));
                }
                _ => {} // Should not happen based on grammar
            }
        }
    }
    Ok(Namespace {
        path,
        imports,
        definitions,
    })
}

fn parse_namespace_import(pair: Pair<Rule>) -> Result<NamespaceImport, AstBuildError> {
    let mut inner = pair.into_inner();
    let path = parse_path(inner.next().unwrap()); // path is mandatory
    let all = inner.next().is_some(); // `.*` is the only other possible token
    Ok(NamespaceImport { path, all })
}

fn parse_table(pair: Pair<Rule>) -> Result<Table, AstBuildError> {
    let mut name = String::new();
    let mut members = Vec::new();

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::IDENT => name = p.as_str().to_string(),
            Rule::table_member => {
                members.push(parse_table_member(p)?);
            }
            Rule::doc_comment => {
                members.push(TableMember::Comment(extract_comment_content(p)));
            }
            found => {
                let (line, col) = p.line_col();
                return Err(AstBuildError::UnexpectedRule {
                    expected: "IDENT or table_member".to_string(),
                    found,
                    line,
                    col,
                });
            }
        }
    }
    Ok(Table {
        doc_comment: None,       // Will be set by the parent parser (build_ast_from_pairs)
        annotations: Vec::new(), // Will be set by the parent parser
        name,
        members,
    })
}

fn parse_table_member(pair: Pair<Rule>) -> Result<TableMember, AstBuildError> {
    let (p_line, p_col) = pair.line_col();
    let mut member_inner = pair.into_inner().peekable();
    let doc_comment = parse_doc_comments(&mut member_inner);
    let annotations = parse_annotations(&mut member_inner)?;

    let member_pair = member_inner.next().ok_or(AstBuildError::MissingElement {
        rule: Rule::table_member,
        element: "field or embed definition".to_string(),
        line: p_line,
        col: p_col,
    })?;

    let (inner_line, inner_col) = member_pair.line_col();
    let mut member = match member_pair.as_rule() {
        Rule::field_definition => TableMember::Field(parse_field_definition(member_pair)?),
        Rule::embed_def => TableMember::Embed(parse_embed(member_pair)?),
        found => {
            return Err(AstBuildError::UnexpectedRule {
                expected: "field_definition or embed_def".to_string(),
                found,
                line: inner_line,
                col: inner_col,
            })
        }
    };

    // Attach comment to the member
    match &mut member {
        TableMember::Field(FieldDefinition::Regular(f)) => {
            f.doc_comment = doc_comment;
            f.annotations = annotations;
        }
        TableMember::Field(FieldDefinition::InlineEmbed(f)) => {
            f.doc_comment = doc_comment;
            f.annotations = annotations;
        }
        TableMember::Embed(e) => {
            e.doc_comment = doc_comment;
            e.annotations = annotations;
        }
        TableMember::Comment(_) => {}
    }

    Ok(member)
}

fn parse_annotation(pair: Pair<Rule>) -> Result<Annotation, AstBuildError> {
    let (line, col) = pair.line_col();
    let mut inner = pair.into_inner();
    let name = inner
        .next()
        .ok_or(AstBuildError::MissingElement {
            rule: Rule::annotation,
            element: "name".to_string(),
            line,
            col,
        })?
        .as_str()
        .to_string();
    let mut params = Vec::new();
    if let Some(params_list_pair) = inner.next() {
        for p in params_list_pair.into_inner() {
            if p.as_rule() == Rule::annotation_param {
                let (p_line, p_col) = p.line_col();
                let mut param_inner = p.into_inner();
                let key = param_inner
                    .next()
                    .ok_or(AstBuildError::MissingElement {
                        rule: Rule::annotation_param,
                        element: "key".to_string(),
                        line: p_line,
                        col: p_col,
                    })?
                    .as_str()
                    .to_string();
                let value_pair = param_inner.next().ok_or(AstBuildError::MissingElement {
                    rule: Rule::annotation_param,
                    element: "value".to_string(),
                    line: p_line,
                    col: p_col,
                })?;
                params.push(AnnotationParam {
                    key,
                    value: parse_literal(value_pair)?,
                });
            }
        }
    }
    Ok(Annotation { name, params })
}

fn parse_field_definition(pair: Pair<Rule>) -> Result<FieldDefinition, AstBuildError> {
    let (line, col) = pair.line_col();
    let inner_pair = pair
        .into_inner()
        .next()
        .ok_or(AstBuildError::MissingElement {
            rule: Rule::field_definition,
            element: "regular_field or inline_embed_field".to_string(),
            line,
            col,
        })?;

    let (inner_line, inner_col) = inner_pair.line_col();
    let def = match inner_pair.as_rule() {
        Rule::regular_field => FieldDefinition::Regular(parse_regular_field(inner_pair)?),
        Rule::inline_embed_field => {
            FieldDefinition::InlineEmbed(parse_inline_embed_field(inner_pair)?)
        }
        found => {
            return Err(AstBuildError::UnexpectedRule {
                expected: "regular_field or inline_embed_field".to_string(),
                found,
                line: inner_line,
                col: inner_col,
            })
        }
    };
    Ok(def)
}

fn parse_regular_field(pair: Pair<Rule>) -> Result<RegularField, AstBuildError> {
    let (line, col) = pair.line_col();

    let mut inner = pair.into_inner();
    let name = inner
        .next()
        .ok_or(AstBuildError::MissingElement {
            rule: Rule::regular_field,
            element: "name".to_string(),
            line,
            col,
        })?
        .as_str()
        .to_string();
    let type_with_cardinality =
        parse_type_with_cardinality(inner.next().ok_or(AstBuildError::MissingElement {
            rule: Rule::regular_field,
            element: "type_with_cardinality".to_string(),
            line,
            col,
        })?)?;
    let mut constraints = Vec::new();
    let mut field_number = None;

    for p in inner {
        match p.as_rule() {
            Rule::constraint => constraints.push(parse_constraint(p)?),
            Rule::field_number => {
                let (p_line, p_col) = p.line_col();
                let text = p
                    .into_inner()
                    .next()
                    .ok_or(AstBuildError::MissingElement {
                        rule: Rule::field_number,
                        element: "integer value".to_string(),
                        line: p_line,
                        col: p_col,
                    })?
                    .as_str();
                field_number = Some(text.parse().map_err(|_| AstBuildError::InvalidValue {
                    element: "field_number".to_string(),
                    value: text.to_string(),
                    line: p_line,
                    col: p_col,
                })?);
            }
            found => {
                let (p_line, p_col) = p.line_col();
                return Err(AstBuildError::UnexpectedRule {
                    expected: "constraint or field_number".to_string(),
                    found,
                    line: p_line,
                    col: p_col,
                });
            }
        }
    }
    Ok(RegularField {
        doc_comment: None,       // Will be set by the parent parser (parse_table)
        annotations: Vec::new(), // Will be set by the parent parser
        name,
        field_type: type_with_cardinality,
        constraints,
        field_number,
    })
}

fn parse_type_with_cardinality(pair: Pair<Rule>) -> Result<TypeWithCardinality, AstBuildError> {
    let (line, col) = pair.line_col();
    let mut inner = pair.into_inner();
    let type_name = parse_type_name(inner.next().ok_or(AstBuildError::MissingElement {
        rule: Rule::type_with_cardinality,
        element: "type_name".to_string(),
        line,
        col,
    })?)?;
    let cardinality = match inner.next() {
        Some(p) => match p.as_rule() {
            Rule::cardinality => {
                let (p_line, p_col) = p.line_col();
                match p.as_str() {
                    "?" => Some(Cardinality::Optional),
                    "[]" => Some(Cardinality::Array),
                    s => {
                        return Err(AstBuildError::InvalidValue {
                            element: "cardinality".to_string(),
                            value: s.to_string(),
                            line: p_line,
                            col: p_col,
                        })
                    }
                }
            }
            found => {
                let (p_line, p_col) = p.line_col();
                return Err(AstBuildError::UnexpectedRule {
                    expected: "cardinality".to_string(),
                    found,
                    line: p_line,
                    col: p_col,
                });
            }
        },
        None => None,
    };
    Ok(TypeWithCardinality {
        base_type: type_name,
        cardinality,
    })
}

fn parse_type_name(pair: Pair<Rule>) -> Result<TypeName, AstBuildError> {
    let (line, col) = pair.line_col();
    let inner_pair = pair
        .into_inner()
        .next()
        .ok_or(AstBuildError::MissingElement {
            rule: Rule::type_name,
            element: "path or basic_type".to_string(),
            line,
            col,
        })?;

    let (inner_line, inner_col) = inner_pair.line_col();
    let tn = match inner_pair.as_rule() {
        Rule::path => TypeName::Path(parse_path(inner_pair)),
        Rule::basic_type => {
            let text = inner_pair.as_str();
            TypeName::Basic(match text {
                "string" => BasicType::String,
                "i8" => BasicType::I8,
                "i16" => BasicType::I16,
                "i32" => BasicType::I32,
                "i64" => BasicType::I64,
                "u8" => BasicType::U8,
                "u16" => BasicType::U16,
                "u32" => BasicType::U32,
                "u64" => BasicType::U64,
                "f32" => BasicType::F32,
                "f64" => BasicType::F64,
                "bool" => BasicType::Bool,
                "bytes" => BasicType::Bytes,
                _ => {
                    return Err(AstBuildError::InvalidValue {
                        element: "basic_type".to_string(),
                        value: text.to_string(),
                        line: inner_line,
                        col: inner_col,
                    })
                }
            })
        }
        found => {
            return Err(AstBuildError::UnexpectedRule {
                expected: "path or basic_type".to_string(),
                found,
                line: inner_line,
                col: inner_col,
            })
        }
    };
    Ok(tn)
}

fn parse_constraint(pair: Pair<Rule>) -> Result<Constraint, AstBuildError> {
    let (line, col) = pair.line_col();
    let inner_pair = pair
        .into_inner()
        .next()
        .ok_or(AstBuildError::MissingElement {
            rule: Rule::constraint,
            element: "constraint type".to_string(),
            line,
            col,
        })?;

    let (inner_line, inner_col) = inner_pair.line_col();
    let constraint = match inner_pair.as_rule() {
        Rule::primary_key => Constraint::PrimaryKey,
        Rule::unique => Constraint::Unique,
        Rule::max_length => {
            let text = inner_pair
                .into_inner()
                .next()
                .ok_or(AstBuildError::MissingElement {
                    rule: Rule::max_length,
                    element: "integer value".to_string(),
                    line: inner_line,
                    col: inner_col,
                })?
                .as_str();
            let val = text.parse().map_err(|_| AstBuildError::InvalidValue {
                element: "max_length".to_string(),
                value: text.to_string(),
                line: inner_line,
                col: inner_col,
            })?;
            Constraint::MaxLength(val)
        }
        Rule::default_val => {
            let val = parse_literal(inner_pair.into_inner().next().ok_or(
                AstBuildError::MissingElement {
                    rule: Rule::default_val,
                    element: "literal value".to_string(),
                    line: inner_line,
                    col: inner_col,
                },
            )?)?;
            Constraint::Default(val)
        }
        Rule::range_val => {
            let mut values = inner_pair.into_inner();
            let val1 = parse_literal(values.next().ok_or(AstBuildError::MissingElement {
                rule: Rule::range_val,
                element: "first value".to_string(),
                line: inner_line,
                col: inner_col,
            })?)?;
            let val2 = parse_literal(values.next().ok_or(AstBuildError::MissingElement {
                rule: Rule::range_val,
                element: "second value".to_string(),
                line: inner_line,
                col: inner_col,
            })?)?;
            Constraint::Range(val1, val2)
        }
        Rule::regex_val => {
            let s = inner_pair
                .into_inner()
                .next()
                .ok_or(AstBuildError::MissingElement {
                    rule: Rule::regex_val,
                    element: "string literal".to_string(),
                    line: inner_line,
                    col: inner_col,
                })?
                .as_str();
            Constraint::Regex(s[1..s.len() - 1].to_string()) // Remove quotes
        }
        Rule::foreign_key_val => {
            let mut inner = inner_pair.into_inner();
            let path = parse_path(inner.next().ok_or(AstBuildError::MissingElement {
                rule: Rule::foreign_key_val,
                element: "path".to_string(),
                line: inner_line,
                col: inner_col,
            })?);
            // The optional alias part `as <name>` is likely wrapped in its own grammar rule.
            // After the path, the next (and last) optional pair should be the alias identifier itself.
            // The `as` keyword is likely consumed by the parser without creating a token.
            let alias = if let Some(ident_pair) = inner.next() {
                Some(ident_pair.as_str().to_string())
            } else {
                None
            };
            Constraint::ForeignKey(path, alias)
        }
        found => {
            return Err(AstBuildError::UnexpectedRule {
                expected: "a constraint type".to_string(),
                found,
                line: inner_line,
                col: inner_col,
            })
        }
    };
    Ok(constraint)
}

fn parse_inline_embed_field(pair: Pair<Rule>) -> Result<InlineEmbedField, AstBuildError> {
    let mut name = String::new();
    let mut fields = Vec::new();
    let mut cardinality = None;
    let mut field_number = None;

    let (line, col) = pair.line_col();
    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::IDENT => name = p.as_str().to_string(),
            Rule::table_member => {
                let (p_line, p_col) = p.line_col();
                let mut member_inner = p.into_inner().peekable();
                let doc_comment = parse_doc_comments(&mut member_inner);
                let annotations = parse_annotations(&mut member_inner)?;

                let member_pair = member_inner.next().ok_or(AstBuildError::MissingElement {
                    rule: Rule::table_member,
                    element: "field definition inside inline embed".to_string(),
                    line: p_line,
                    col: p_col,
                })?;

                // Inline embeds can only contain fields, not other named embeds.
                if member_pair.as_rule() == Rule::field_definition {
                    let mut field_def = parse_field_definition(member_pair)?;
                    match &mut field_def {
                        FieldDefinition::Regular(f) => {
                            f.doc_comment = doc_comment;
                            f.annotations = annotations;
                        }
                        FieldDefinition::InlineEmbed(f) => {
                            f.doc_comment = doc_comment;
                            f.annotations = annotations;
                        }
                    }
                    fields.push(field_def);
                } else {
                    return Err(AstBuildError::UnexpectedRule {
                        expected: "field_definition".to_string(),
                        found: member_pair.as_rule(),
                        line: member_pair.line_col().0,
                        col: member_pair.line_col().1,
                    });
                }
            }
            Rule::cardinality => {
                let (p_line, p_col) = p.line_col();
                cardinality = Some(match p.as_str() {
                    "?" => Cardinality::Optional,
                    "[]" => Cardinality::Array,
                    s => {
                        return Err(AstBuildError::InvalidValue {
                            element: "cardinality".to_string(),
                            value: s.to_string(),
                            line: p_line,
                            col: p_col,
                        })
                    }
                })
            }
            Rule::field_number => {
                let (p_line, p_col) = p.line_col();
                let text = p
                    .into_inner()
                    .next()
                    .ok_or(AstBuildError::MissingElement {
                        rule: Rule::field_number,
                        element: "integer value".to_string(),
                        line: p_line,
                        col: p_col,
                    })?
                    .as_str();
                field_number = Some(text.parse().map_err(|_| AstBuildError::InvalidValue {
                    element: "field_number".to_string(),
                    value: text.to_string(),
                    line: p_line,
                    col: p_col,
                })?);
            }
            Rule::doc_comment => {}
            found => {
                let (p_line, p_col) = p.line_col();
                return Err(AstBuildError::UnexpectedRule {
                    expected: "IDENT, table_member, cardinality, or field_number".to_string(),
                    found,
                    line: p_line,
                    col: p_col,
                });
            }
        }
    }

    if name.is_empty() {
        return Err(AstBuildError::MissingElement {
            rule: Rule::inline_embed_field,
            element: "name".to_string(),
            line,
            col,
        });
    }
    Ok(InlineEmbedField {
        doc_comment: None,       // Will be set by the parent parser (parse_table)
        annotations: Vec::new(), // Will be set by the parent parser
        name,
        fields,
        cardinality,
        field_number,
    })
}

fn parse_enum(pair: Pair<Rule>) -> Result<Enum, AstBuildError> {
    let (line, col) = pair.line_col();
    let mut inner = pair.into_inner();
    let name = inner
        .next()
        .ok_or(AstBuildError::MissingElement {
            rule: Rule::enum_def,
            element: "name".to_string(),
            line,
            col,
        })?
        .as_str()
        .to_string();

    let mut variants = Vec::new();
    for p in inner {
        if p.as_rule() == Rule::enum_variant {
            let (p_line, p_col) = p.line_col();
            let mut variant_inner = p.into_inner().peekable();
            let doc_comment = parse_doc_comments(&mut variant_inner);
            let annotations = parse_annotations(&mut variant_inner)?;
            let variant_name = variant_inner
                .next()
                .ok_or(AstBuildError::MissingElement {
                    rule: Rule::enum_variant,
                    element: "name".to_string(),
                    line: p_line,
                    col: p_col,
                })?
                .as_str()
                .to_string();

            variants.push(EnumVariant {
                doc_comment,
                annotations,
                name: variant_name,
            });
        }
    }
    Ok(Enum {
        doc_comment: None,       // Will be set by the parent parser (build_ast_from_pairs)
        annotations: Vec::new(), // Will be set by the parent parser
        name,
        variants,
    })
}

fn parse_embed(pair: Pair<Rule>) -> Result<Embed, AstBuildError> {
    let (line, col) = pair.line_col();
    let mut inner = pair.into_inner();
    let name = inner
        .next()
        .ok_or(AstBuildError::MissingElement {
            rule: Rule::embed_def,
            element: "name".to_string(),
            line,
            col,
        })?
        .as_str()
        .to_string();
    let mut fields = Vec::new();
    for p in inner {
        if p.as_rule() == Rule::table_member {
            let (p_line, p_col) = p.line_col();
            let mut member_inner = p.into_inner().peekable();
            let doc_comment = parse_doc_comments(&mut member_inner);
            let annotations = parse_annotations(&mut member_inner)?;

            let member_pair = member_inner.next().ok_or(AstBuildError::MissingElement {
                rule: Rule::table_member,
                element: "field definition inside embed".to_string(),
                line: p_line,
                col: p_col,
            })?;

            // Named embeds can only contain fields, not other named embeds.
            if member_pair.as_rule() == Rule::field_definition {
                let mut field_def = parse_field_definition(member_pair)?;
                match &mut field_def {
                    FieldDefinition::Regular(f) => {
                        f.doc_comment = doc_comment;
                        f.annotations = annotations;
                    }
                    FieldDefinition::InlineEmbed(f) => {
                        f.doc_comment = doc_comment;
                        f.annotations = annotations;
                    }
                }
                fields.push(field_def);
            } else {
                return Err(AstBuildError::UnexpectedRule {
                    expected: "field_definition".to_string(),
                    found: member_pair.as_rule(),
                    line: member_pair.line_col().0,
                    col: member_pair.line_col().1,
                });
            }
        } else {
            let (p_line, p_col) = p.line_col();
            return Err(AstBuildError::UnexpectedRule {
                expected: "table_member".to_string(),
                found: p.as_rule(),
                line: p_line,
                col: p_col,
            });
        }
    }
    Ok(Embed {
        doc_comment: None,       // Will be set by the parent parser
        annotations: Vec::new(), // Will be set by the parent parser
        name,
        fields,
    })
}
