use pest::iterators::Pair;
use crate::ast_model::Literal;
use crate::error::AstBuildError;
use crate::Rule;

/// Helper function to parse a literal value
pub fn parse_literal(pair: Pair<Rule>) -> Result<Literal, AstBuildError> {
    // A literal can be passed as a wrapper pair (e.g., from a `default` value)
    // or as a direct token pair (e.g., from an annotation parameter).
    // This handles both cases by consuming the pair to get an inner token,
    // or falling back to a clone of the original pair if it has no inner token.
    let cloned_pair = pair.clone();
    let token_pair = pair.into_inner().next().unwrap_or(cloned_pair);
    let rule = token_pair.as_rule();
    let text = token_pair.as_str();

    let (line, col) = token_pair.line_col();

    let literal = match rule {
        Rule::STRING_LITERAL => {
            // Remove quotes and handle escaped characters (basic for now)
            Literal::String(text[1..text.len() - 1].to_string())
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
