use crate::ast_model::Literal;
use crate::error::AstBuildError;
use crate::Rule;
use pest::iterators::Pair;

/// Helper function to parse a literal value
pub fn parse_literal(pair: Pair<Rule>) -> Result<Literal, AstBuildError> {
    // A literal can be passed as a wrapper pair (e.g., from a `default` value)
    // or as an annotation value, which additionally supports dotted paths and tuples.
    match pair.as_rule() {
        Rule::literal | Rule::annotation_value => {
            let (line, col) = pair.line_col();
            let inner = pair
                .into_inner()
                .next()
                .ok_or(AstBuildError::MissingElement {
                    rule: Rule::literal,
                    element: "literal value".to_string(),
                    line,
                    col,
                })?;
            return parse_literal(inner);
        }
        Rule::annotation_tuple => {
            let mut values = Vec::new();
            for value_pair in pair.into_inner() {
                values.push(parse_literal(value_pair)?);
            }
            return Ok(Literal::Tuple(values));
        }
        Rule::annotation_path => {
            return Ok(Literal::Path(
                pair.as_str().split('.').map(str::to_string).collect(),
            ));
        }
        _ => {}
    }

    let rule = pair.as_rule();
    let text = pair.as_str();

    let (line, col) = pair.line_col();

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
