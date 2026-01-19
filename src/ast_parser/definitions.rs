use pest::iterators::Pair;
use crate::ast_model::{
    Definition, Embed, Enum, EnumVariant, Metadata, Namespace, NamespaceImport, Table, TableMember,
};
use crate::error::AstBuildError;
use crate::Rule;

use super::fields::parse_table_member;
use super::helpers::{extract_comment_content, parse_path};
use super::metadata::parse_metadata;

pub fn parse_definition(pair: Pair<Rule>) -> Result<Definition, AstBuildError> {
    let (_line, _col) = pair.line_col();
    let mut inner_pairs = pair.into_inner().peekable();
    let metadata = parse_metadata(&mut inner_pairs)?;

    let def_pair = inner_pairs.next().unwrap();
    let (inner_line, inner_col) = def_pair.line_col();

    let definition = match def_pair.as_rule() {
        Rule::namespace => {
            let mut ns = parse_namespace(def_pair)?;
            let mut new_defs: Vec<Definition> = metadata
                .into_iter()
                .map(|m| match m {
                    Metadata::DocComment(c) => Definition::Comment(c),
                    Metadata::Annotation(a) => Definition::Annotation(a),
                })
                .collect();
            new_defs.append(&mut ns.definitions);
            ns.definitions = new_defs;
            Definition::Namespace(ns)
        }
        Rule::table => {
            let mut table = parse_table(def_pair)?;
            table.metadata = metadata;
            Definition::Table(table)
        }
        Rule::enum_def => {
            let mut enum_def = parse_enum(def_pair)?;
            enum_def.metadata = metadata;
            Definition::Enum(enum_def)
        }
        Rule::embed_def => {
            let mut embed = parse_embed(def_pair)?;
            embed.metadata = metadata;
            Definition::Embed(embed)
        }
        found => {
            return Err(AstBuildError::UnexpectedRule {
                expected: "namespace, table, enum, or embed".to_string(),
                found,
                line: inner_line,
                col: inner_col,
            })
        }
    };

    Ok(definition)
}

pub fn parse_namespace(pair: Pair<Rule>) -> Result<Namespace, AstBuildError> {
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
            let item_pair = p.into_inner().next().unwrap();
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
                _ => {}
            }
        }
    }
    Ok(Namespace {
        path,
        imports,
        definitions,
    })
}

pub fn parse_namespace_import(pair: Pair<Rule>) -> Result<NamespaceImport, AstBuildError> {
    let mut inner = pair.into_inner();
    let path = parse_path(inner.next().unwrap());
    let all = inner.next().is_some();
    Ok(NamespaceImport { path, all })
}

pub fn parse_table(pair: Pair<Rule>) -> Result<Table, AstBuildError> {
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
        metadata: Vec::new(),
        name: Some(name),
        members,
    })
}

pub fn parse_enum(pair: Pair<Rule>) -> Result<Enum, AstBuildError> {
    let (_line, _col) = pair.line_col();
    let mut inner = pair.into_inner();

    let mut enum_name: Option<String> = None;
    if let Some(p) = inner.peek() {
        if p.as_rule() == Rule::IDENT {
            enum_name = Some(inner.next().unwrap().as_str().to_string());
        }
    }

    let mut variants = Vec::new();
    for p in inner {
        if p.as_rule() == Rule::enum_variant {
            let (p_line, p_col) = p.line_col();
            let mut variant_inner = p.into_inner().peekable();
            let metadata = parse_metadata(&mut variant_inner)?;
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

            let mut variant_value: Option<i64> = None;
            if let Some(value_pair) = variant_inner.peek() {
                if value_pair.as_rule() == Rule::INTEGER {
                    let consumed_value_pair = variant_inner.next().unwrap();
                    variant_value = Some(consumed_value_pair.as_str().parse().map_err(|_| {
                        AstBuildError::InvalidValue {
                            element: "enum variant value".to_string(),
                            value: consumed_value_pair.as_str().to_string(),
                            line: p_line,
                            col: p_col,
                        }
                    })?);
                }
            }

            // Parse optional inline comment from enum_variant_end
            let mut inline_comment: Option<String> = None;
            if let Some(end_pair) = variant_inner.peek() {
                if end_pair.as_rule() == Rule::enum_variant_end {
                    let end_text = variant_inner.next().unwrap().as_str();
                    // enum_variant_end contains: (";" | ",") ~ spaces ~ inline_comment?
                    // Extract inline comment if present (starts with //)
                    if let Some(comment_start) = end_text.find("//") {
                        let comment_text = &end_text[comment_start..];
                        let cleaned = comment_text.trim_start_matches("//").trim();
                        if !cleaned.is_empty() {
                            inline_comment = Some(cleaned.to_string());
                        }
                    }
                }
            }

            variants.push(EnumVariant {
                metadata,
                name: Some(variant_name),
                value: variant_value,
                inline_comment,
            });
        }
    }
    Ok(Enum {
        metadata: Vec::new(),
        name: enum_name,
        variants,
    })
}

pub fn parse_embed(pair: Pair<Rule>) -> Result<Embed, AstBuildError> {
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
    let mut members = Vec::new();
    for p in inner {
        match p.as_rule() {
            Rule::table_member => {
                members.push(parse_table_member(p)?);
            }
            Rule::doc_comment => {
                members.push(TableMember::Comment(extract_comment_content(p)));
            }
            found => {
                let (p_line, p_col) = p.line_col();
                return Err(AstBuildError::UnexpectedRule {
                    expected: "table_member or doc_comment".to_string(),
                    found,
                    line: p_line,
                    col: p_col,
                });
            }
        }
    }
    Ok(Embed {
        metadata: Vec::new(),
        name: Some(name),
        members,
    })
}
