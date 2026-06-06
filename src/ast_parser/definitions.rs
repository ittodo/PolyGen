use crate::ast_model::{
    Definition, Embed, Enum, EnumVariant, Namespace, NamespaceImport, Table, TableMember,
};
use crate::error::AstBuildError;
use crate::Rule;
use pest::iterators::Pair;

use super::fields::parse_table_body_item;
use super::helpers::{extract_comment_content, parse_path};
use super::metadata::parse_metadata;

pub fn parse_definition(pair: Pair<Rule>) -> Result<Definition, AstBuildError> {
    let (line, col) = pair.line_col();
    let mut inner_pairs = pair.into_inner().peekable();
    let metadata = parse_metadata(&mut inner_pairs)?;

    let def_pair = inner_pairs.next().ok_or(AstBuildError::MissingElement {
        rule: Rule::definition,
        element: "definition body".to_string(),
        line,
        col,
    })?;
    let (inner_line, inner_col) = def_pair.line_col();

    let definition = match def_pair.as_rule() {
        Rule::namespace => {
            let mut ns = parse_namespace(def_pair)?;
            ns.metadata = metadata;
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
            let (p_line, p_col) = p.line_col();
            let item_pair = p.into_inner().next().ok_or(AstBuildError::MissingElement {
                rule: Rule::namespace_body_item,
                element: "item".to_string(),
                line: p_line,
                col: p_col,
            })?;
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
        metadata: Vec::new(), // Will be set by caller (parse_definition)
        path,
        imports,
        definitions,
    })
}

pub fn parse_namespace_import(pair: Pair<Rule>) -> Result<NamespaceImport, AstBuildError> {
    let (line, col) = pair.line_col();
    let mut inner = pair.into_inner();
    let path = parse_path(inner.next().ok_or(AstBuildError::MissingElement {
        rule: Rule::namespace_import,
        element: "path".to_string(),
        line,
        col,
    })?);
    let all = inner.next().is_some();
    Ok(NamespaceImport { path, all })
}

pub fn parse_table(pair: Pair<Rule>) -> Result<Table, AstBuildError> {
    let (name, members) = parse_named_member_block(pair, Rule::table, "table name")?;
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
            enum_name = Some(p.as_str().to_string());
            inner.next();
        }
    }

    let mut variants = Vec::new();
    for p in inner {
        if p.as_rule() == Rule::enum_variant {
            variants.push(parse_enum_variant(p)?);
        }
    }
    Ok(Enum {
        metadata: Vec::new(),
        name: enum_name,
        variants,
    })
}

pub fn parse_enum_variant(pair: Pair<Rule>) -> Result<EnumVariant, AstBuildError> {
    let (line, col) = pair.line_col();
    let mut inner = pair.into_inner().peekable();
    let metadata = parse_metadata(&mut inner)?;
    let name = inner
        .next()
        .ok_or(AstBuildError::MissingElement {
            rule: Rule::enum_variant,
            element: "name".to_string(),
            line,
            col,
        })?
        .as_str()
        .to_string();

    let value = if matches!(inner.peek().map(Pair::as_rule), Some(Rule::INTEGER)) {
        let value_pair = inner.next().ok_or(AstBuildError::MissingElement {
            rule: Rule::enum_variant,
            element: "value".to_string(),
            line,
            col,
        })?;
        Some(
            value_pair
                .as_str()
                .parse()
                .map_err(|_| AstBuildError::InvalidValue {
                    element: "enum variant value".to_string(),
                    value: value_pair.as_str().to_string(),
                    line,
                    col,
                })?,
        )
    } else {
        None
    };

    let inline_comment = match inner.peek() {
        Some(end_pair) if end_pair.as_rule() == Rule::enum_variant_end => {
            let end_pair = inner.next().ok_or(AstBuildError::MissingElement {
                rule: Rule::enum_variant,
                element: "end".to_string(),
                line,
                col,
            })?;
            parse_enum_variant_inline_comment(end_pair.as_str())
        }
        _ => None,
    };

    Ok(EnumVariant {
        metadata,
        name: Some(name),
        value,
        inline_comment,
    })
}

fn parse_enum_variant_inline_comment(end_text: &str) -> Option<String> {
    let comment_text = end_text.get(end_text.find("//")? + 2..)?;
    let cleaned = comment_text.trim();
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned.to_string())
    }
}

pub fn parse_embed(pair: Pair<Rule>) -> Result<Embed, AstBuildError> {
    let (name, members) = parse_named_member_block(pair, Rule::embed_def, "embed name")?;
    Ok(Embed {
        metadata: Vec::new(),
        name: Some(name),
        members,
    })
}

fn parse_named_member_block(
    pair: Pair<Rule>,
    owner_rule: Rule,
    name_element: &str,
) -> Result<(String, Vec<TableMember>), AstBuildError> {
    let (line, col) = pair.line_col();
    let mut inner = pair.into_inner();
    let name = inner.next().ok_or(AstBuildError::MissingElement {
        rule: owner_rule,
        element: name_element.to_string(),
        line,
        col,
    })?;
    if name.as_rule() != Rule::IDENT {
        let (name_line, name_col) = name.line_col();
        return Err(AstBuildError::UnexpectedRule {
            expected: "IDENT".to_string(),
            found: name.as_rule(),
            line: name_line,
            col: name_col,
        });
    }

    let name = name.as_str().to_string();
    let mut members = Vec::new();
    for p in inner {
        members.push(parse_table_body_item(p)?);
    }
    Ok((name, members))
}
