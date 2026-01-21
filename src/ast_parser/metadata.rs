use pest::iterators::Pair;
use crate::ast_model::{Annotation, AnnotationArg, AnnotationParam, Metadata};
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

/// Parse an annotation with support for both positional and named arguments.
///
/// Examples:
/// - `@taggable` - no arguments
/// - `@load(csv: "data.csv")` - named only
/// - `@index(name, level)` - positional only
/// - `@index(name, level, unique: true)` - mixed
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

    let mut args = Vec::new();

    if let Some(args_list_pair) = inner.next() {
        for arg_pair in args_list_pair.into_inner() {
            match arg_pair.as_rule() {
                Rule::annotation_arg => {
                    // annotation_arg = { annotation_param | literal }
                    let inner_pair = arg_pair.into_inner().next().unwrap();
                    match inner_pair.as_rule() {
                        Rule::annotation_param => {
                            let (p_line, p_col) = inner_pair.line_col();
                            let mut param_inner = inner_pair.into_inner();
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
                            args.push(AnnotationArg::Named(AnnotationParam {
                                key,
                                value: parse_literal(value_pair)?,
                            }));
                        }
                        _ => {
                            // Positional argument (literal, which includes IDENT)
                            let value = parse_literal(inner_pair)?;
                            args.push(AnnotationArg::Positional(value));
                        }
                    }
                }
                Rule::annotation_param => {
                    // Direct annotation_param (for backward compatibility if needed)
                    let (p_line, p_col) = arg_pair.line_col();
                    let mut param_inner = arg_pair.into_inner();
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
                    args.push(AnnotationArg::Named(AnnotationParam {
                        key,
                        value: parse_literal(value_pair)?,
                    }));
                }
                _ => {
                    // Positional argument
                    let value = parse_literal(arg_pair)?;
                    args.push(AnnotationArg::Positional(value));
                }
            }
        }
    }

    Ok(Annotation {
        name: Some(name),
        args,
    })
}
