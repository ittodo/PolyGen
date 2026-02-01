use crate::ast_model::{
    Cardinality, Constraint, EnumVariant, FieldDefinition, InlineEmbedField, InlineEnumField,
    RegularField, TableMember, Timezone,
};
use crate::error::AstBuildError;
use crate::Rule;
use pest::iterators::Pair;

use super::definitions::{parse_embed, parse_enum};
use super::helpers::{extract_comment_content, parse_path};
use super::literals::parse_literal;
use super::metadata::parse_metadata;
use super::types::parse_type_with_cardinality;

pub fn parse_table_member(pair: Pair<Rule>) -> Result<TableMember, AstBuildError> {
    let (line, col) = pair.line_col();
    let mut member_inner = pair.into_inner().peekable();
    let metadata = parse_metadata(&mut member_inner)?;

    let member_pair = require_next!(
        member_inner,
        Rule::table_member,
        "field or embed definition",
        line,
        col
    )?;

    let (inner_line, inner_col) = member_pair.line_col();
    let mut member = match member_pair.as_rule() {
        Rule::field_definition => TableMember::Field(parse_field_definition(member_pair)?),
        Rule::embed_def => TableMember::Embed(parse_embed(member_pair)?),
        Rule::enum_def => TableMember::Enum(parse_enum(member_pair)?),
        found => unexpected_rule!(
            found,
            "field_definition, embed_def, or enum_def",
            inner_line,
            inner_col
        ),
    };

    // Attach metadata to the member
    match &mut member {
        TableMember::Field(FieldDefinition::Regular(f)) => f.metadata = metadata,
        TableMember::Field(FieldDefinition::InlineEmbed(f)) => f.metadata = metadata,
        TableMember::Field(FieldDefinition::InlineEnum(f)) => f.metadata = metadata,
        TableMember::Embed(e) => e.metadata = metadata,
        TableMember::Enum(e) => e.metadata = metadata,
        TableMember::Comment(_) => {}
    }

    Ok(member)
}

pub fn parse_field_definition(pair: Pair<Rule>) -> Result<FieldDefinition, AstBuildError> {
    let (line, col) = pair.line_col();
    let inner_pair = require_next!(
        pair.into_inner(),
        Rule::field_definition,
        "regular_field or inline_embed_field",
        line,
        col
    )?;

    let (inner_line, inner_col) = inner_pair.line_col();
    let def = match inner_pair.as_rule() {
        Rule::regular_field => FieldDefinition::Regular(parse_regular_field(inner_pair)?),
        Rule::inline_embed_field => {
            FieldDefinition::InlineEmbed(parse_inline_embed_field(inner_pair)?)
        }
        Rule::inline_enum_field => {
            FieldDefinition::InlineEnum(parse_inline_enum_field(inner_pair)?)
        }
        found => unexpected_rule!(
            found,
            "regular_field, inline_embed_field, or inline_enum_field",
            inner_line,
            inner_col
        ),
    };
    Ok(def)
}

pub fn parse_regular_field(pair: Pair<Rule>) -> Result<RegularField, AstBuildError> {
    let (line, col) = pair.line_col();
    let mut inner = pair.into_inner();

    let name = require_next!(inner, Rule::regular_field, "name", line, col)?
        .as_str()
        .to_string();
    let type_with_cardinality = parse_type_with_cardinality(require_next!(
        inner,
        Rule::regular_field,
        "type_with_cardinality",
        line,
        col
    )?)?;

    let mut constraints = Vec::new();
    let mut field_number = None;

    for p in inner {
        match p.as_rule() {
            Rule::constraint => constraints.push(parse_constraint(p)?),
            Rule::field_number => {
                let (p_line, p_col) = p.line_col();
                let text = require_next!(
                    p.into_inner(),
                    Rule::field_number,
                    "integer value",
                    p_line,
                    p_col
                )?
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
                unexpected_rule!(found, "constraint or field_number", p_line, p_col);
            }
        }
    }

    Ok(RegularField {
        metadata: Vec::new(),
        name: Some(name),
        field_type: type_with_cardinality,
        constraints,
        field_number,
    })
}

pub fn parse_constraint(pair: Pair<Rule>) -> Result<Constraint, AstBuildError> {
    let (line, col) = pair.line_col();
    let inner_pair = require_next!(
        pair.into_inner(),
        Rule::constraint,
        "constraint type",
        line,
        col
    )?;

    let (inner_line, inner_col) = inner_pair.line_col();
    let constraint = match inner_pair.as_rule() {
        Rule::primary_key => Constraint::PrimaryKey,
        Rule::unique => Constraint::Unique,
        Rule::index => Constraint::Index,
        Rule::max_length => {
            let text = require_next!(
                inner_pair.into_inner(),
                Rule::max_length,
                "integer value",
                inner_line,
                inner_col
            )?
            .as_str();
            Constraint::MaxLength(text.parse().map_err(|_| AstBuildError::InvalidValue {
                element: "max_length".to_string(),
                value: text.to_string(),
                line: inner_line,
                col: inner_col,
            })?)
        }
        Rule::default_val => {
            let val = parse_literal(require_next!(
                inner_pair.into_inner(),
                Rule::default_val,
                "literal value",
                inner_line,
                inner_col
            )?)?;
            Constraint::Default(val)
        }
        Rule::range_val => {
            let mut values = inner_pair.into_inner();
            let val1 = parse_literal(require_next!(
                values,
                Rule::range_val,
                "first value",
                inner_line,
                inner_col
            )?)?;
            let val2 = parse_literal(require_next!(
                values,
                Rule::range_val,
                "second value",
                inner_line,
                inner_col
            )?)?;
            Constraint::Range(val1, val2)
        }
        Rule::regex_val => {
            let s = require_next!(
                inner_pair.into_inner(),
                Rule::regex_val,
                "string literal",
                inner_line,
                inner_col
            )?
            .as_str();
            Constraint::Regex(s[1..s.len() - 1].to_string())
        }
        Rule::foreign_key_val => {
            let mut inner = inner_pair.into_inner();
            let path = parse_path(require_next!(
                inner,
                Rule::foreign_key_val,
                "path",
                inner_line,
                inner_col
            )?);
            let alias = inner
                .next()
                .map(|ident_pair| ident_pair.as_str().to_string());
            Constraint::ForeignKey(path, alias)
        }
        Rule::auto_create => {
            let tz = inner_pair
                .into_inner()
                .next()
                .map(parse_timezone)
                .transpose()?;
            Constraint::AutoCreate(tz)
        }
        Rule::auto_update => {
            let tz = inner_pair
                .into_inner()
                .next()
                .map(parse_timezone)
                .transpose()?;
            Constraint::AutoUpdate(tz)
        }
        found => unexpected_rule!(found, "a constraint type", inner_line, inner_col),
    };
    Ok(constraint)
}

/// Parse a timezone specification.
///
/// Supports:
/// - `utc`: UTC timezone
/// - `local`: System local timezone
/// - `+9`, `-5`, `+5:30`: UTC offset
/// - `"Korea Standard Time"`: Windows TimeZone ID
fn parse_timezone(pair: Pair<Rule>) -> Result<Timezone, AstBuildError> {
    let (line, col) = pair.line_col();
    let inner_pair = require_next!(
        pair.into_inner(),
        Rule::timezone,
        "timezone specification",
        line,
        col
    )?;

    let (inner_line, inner_col) = inner_pair.line_col();
    let tz = match inner_pair.as_rule() {
        Rule::tz_utc => Timezone::Utc,
        Rule::tz_local => Timezone::Local,
        Rule::tz_offset => {
            let text = inner_pair.as_str();
            parse_tz_offset(text).map_err(|_| AstBuildError::InvalidValue {
                element: "timezone offset".to_string(),
                value: text.to_string(),
                line: inner_line,
                col: inner_col,
            })?
        }
        Rule::STRING_LITERAL => {
            let s = inner_pair.as_str();
            // Remove quotes
            Timezone::Named(s[1..s.len() - 1].to_string())
        }
        found => unexpected_rule!(
            found,
            "timezone (utc, local, offset, or string)",
            inner_line,
            inner_col
        ),
    };
    Ok(tz)
}

/// Parse UTC offset string like "+9", "-5", "+5:30" into Timezone::Offset
fn parse_tz_offset(s: &str) -> Result<Timezone, ()> {
    let (sign, rest) = if let Some(r) = s.strip_prefix('+') {
        (1i8, r)
    } else if let Some(r) = s.strip_prefix('-') {
        (-1i8, r)
    } else {
        return Err(());
    };

    let (hours, minutes) = if let Some(colon_pos) = rest.find(':') {
        let hours: i8 = rest[..colon_pos].parse().map_err(|_| ())?;
        let minutes: u8 = rest[colon_pos + 1..].parse().map_err(|_| ())?;
        (hours, minutes)
    } else {
        let hours: i8 = rest.parse().map_err(|_| ())?;
        (hours, 0u8)
    };

    Ok(Timezone::Offset {
        hours: sign * hours,
        minutes,
    })
}

pub fn parse_inline_embed_field(pair: Pair<Rule>) -> Result<InlineEmbedField, AstBuildError> {
    let (line, col) = pair.line_col();
    let mut name = String::new();
    let mut members = Vec::new();
    let mut cardinality = None;
    let mut field_number = None;

    for p in pair.into_inner() {
        match p.as_rule() {
            Rule::IDENT => name = p.as_str().to_string(),
            Rule::table_member => members.push(parse_table_member(p)?),
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
                });
            }
            Rule::field_number => {
                let (p_line, p_col) = p.line_col();
                let text = require_next!(
                    p.into_inner(),
                    Rule::field_number,
                    "integer value",
                    p_line,
                    p_col
                )?
                .as_str();
                field_number = Some(text.parse().map_err(|_| AstBuildError::InvalidValue {
                    element: "field_number".to_string(),
                    value: text.to_string(),
                    line: p_line,
                    col: p_col,
                })?);
            }
            Rule::doc_comment => members.push(TableMember::Comment(extract_comment_content(p))),
            found => {
                let (p_line, p_col) = p.line_col();
                unexpected_rule!(
                    found,
                    "IDENT, table_member, cardinality, or field_number",
                    p_line,
                    p_col
                );
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
        metadata: Vec::new(),
        name: Some(name),
        members,
        cardinality,
        field_number,
    })
}

pub fn parse_inline_enum_field(pair: Pair<Rule>) -> Result<InlineEnumField, AstBuildError> {
    let (line, col) = pair.line_col();
    let mut inner = pair.into_inner();

    let name = require_next!(inner, Rule::inline_enum_field, "name (IDENT)", line, col)?
        .as_str()
        .to_string();

    let mut variants = Vec::new();
    let mut cardinality = None;
    let mut field_number = None;

    for p in inner {
        match p.as_rule() {
            Rule::enum_variant => {
                let (p_line, p_col) = p.line_col();
                let mut variant_inner = p.into_inner().peekable();
                let metadata = parse_metadata(&mut variant_inner)?;
                let variant_name =
                    require_next!(variant_inner, Rule::enum_variant, "name", p_line, p_col)?
                        .as_str()
                        .to_string();

                let mut variant_value: Option<i64> = None;
                if let Some(value_pair) = variant_inner.peek() {
                    if value_pair.as_rule() == Rule::INTEGER {
                        let consumed_value_pair = variant_inner.next().unwrap();
                        variant_value =
                            Some(consumed_value_pair.as_str().parse().map_err(|_| {
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
                });
            }
            Rule::field_number => {
                let (p_line, p_col) = p.line_col();
                let text = require_next!(
                    p.into_inner(),
                    Rule::field_number,
                    "integer value",
                    p_line,
                    p_col
                )?
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
                unexpected_rule!(
                    found,
                    "enum_variant, cardinality, or field_number",
                    p_line,
                    p_col
                );
            }
        }
    }

    Ok(InlineEnumField {
        metadata: Vec::new(),
        name: Some(name),
        variants,
        cardinality,
        field_number,
    })
}
