use crate::ast_model::Cardinality;
use crate::error::AstBuildError;
use crate::Rule;
use pest::iterators::Pair;

/// Helper function to parse a path (e.g., "game.common")
pub fn parse_path(pair: Pair<Rule>) -> Vec<String> {
    pair.into_inner()
        .filter(|p| p.as_rule() == Rule::IDENT)
        .map(|p| p.as_str().to_string())
        .collect()
}

/// Extract the content from a comment pair, removing comment markers
pub fn extract_comment_content(comment_pair: Pair<Rule>) -> String {
    let s = comment_pair.as_str();
    if let Some(content) = s.strip_prefix("///") {
        content.trim().to_string()
    } else if let Some(content) = s.strip_prefix("//") {
        content.trim().to_string()
    } else if let Some(content) = s.strip_prefix("/*") {
        content.trim_end_matches("*/").trim().to_string()
    } else {
        s.trim().to_string()
    }
}

/// Parse a field cardinality marker (`?` or `[]`).
pub fn parse_cardinality(pair: Pair<Rule>) -> Result<Cardinality, AstBuildError> {
    let (line, col) = pair.line_col();
    match pair.as_str() {
        "?" => Ok(Cardinality::Optional),
        "[]" => Ok(Cardinality::Array),
        s => Err(AstBuildError::InvalidValue {
            element: "cardinality".to_string(),
            value: s.to_string(),
            line,
            col,
        }),
    }
}

/// Parse a field number suffix (`= 1`).
pub fn parse_field_number(pair: Pair<Rule>) -> Result<u32, AstBuildError> {
    let (line, col) = pair.line_col();
    let number_pair = pair
        .into_inner()
        .next()
        .ok_or(AstBuildError::MissingElement {
            rule: Rule::field_number,
            element: "integer value".to_string(),
            line,
            col,
        })?;
    let text = number_pair.as_str();
    text.parse().map_err(|_| AstBuildError::InvalidValue {
        element: "field_number".to_string(),
        value: text.to_string(),
        line,
        col,
    })
}
