use pest::iterators::Pair;
use crate::ast_model::{Annotation, AnnotationParam, Metadata};
use crate::error::AstBuildError;
use crate::Rule;

use super::helpers::extract_comment_content;
use super::literals::parse_literal;

/// Helper function to parse doc comments and annotations from a stream of pairs.
/// It consumes `doc_comment` and `annotation` rules from the beginning of the stream,
/// allowing them to be interleaved.
pub fn parse_metadata(
    inner_pairs: &mut std::iter::Peekable<pest::iterators::Pairs<Rule>>,
) -> Result<Vec<Metadata>, AstBuildError> {
    let mut metadata = Vec::new();

    while let Some(p) = inner_pairs.peek() {
        match p.as_rule() {
            Rule::doc_comment => {
                let comment_pair = inner_pairs.next().unwrap();
                metadata.push(Metadata::DocComment(extract_comment_content(comment_pair)));
            }
            Rule::annotation => {
                let annotation_pair = inner_pairs.next().unwrap();
                metadata.push(Metadata::Annotation(parse_annotation(annotation_pair)?));
            }
            _ => {
                break;
            }
        }
    }

    Ok(metadata)
}

pub fn parse_annotation(pair: Pair<Rule>) -> Result<Annotation, AstBuildError> {
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
    Ok(Annotation {
        name: Some(name),
        params,
    })
}
