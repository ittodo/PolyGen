use crate::ast_model::{BasicType, Cardinality, TypeName, TypeWithCardinality};
use crate::error::AstBuildError;
use crate::Rule;
use pest::iterators::Pair;

use super::definitions::parse_enum;
use super::helpers::parse_path;

pub fn parse_type_with_cardinality(pair: Pair<Rule>) -> Result<TypeWithCardinality, AstBuildError> {
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

pub fn parse_type_name(pair: Pair<Rule>) -> Result<TypeName, AstBuildError> {
    let (line, col) = pair.line_col();
    let inner_pair = pair
        .into_inner()
        .next()
        .ok_or(AstBuildError::MissingElement {
            rule: Rule::type_name,
            element: "path, basic_type or anonymous_enum_def".to_string(),
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
                "timestamp" => BasicType::Timestamp,
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
        Rule::anonymous_enum_def => TypeName::InlineEnum(parse_enum(inner_pair)?),
        found => {
            return Err(AstBuildError::UnexpectedRule {
                expected: "path, basic_type or anonymous_enum_def".to_string(),
                found,
                line: inner_line,
                col: inner_col,
            })
        }
    };
    Ok(tn)
}
