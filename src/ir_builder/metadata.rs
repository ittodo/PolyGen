use crate::ast_model::{self, Metadata};
use crate::ir_model::{AnnotationDef, AnnotationParam};

/// Extracts the @datasource value from metadata.
pub(super) fn extract_datasource(metadata: &[Metadata]) -> Option<String> {
    extract_annotation_string(metadata, "datasource")
}

/// Extracts the @cache strategy from metadata.
pub(super) fn extract_cache_strategy(metadata: &[Metadata]) -> Option<String> {
    for meta in metadata {
        if let Metadata::Annotation(ann) = meta {
            if ann.name.as_deref() == Some("cache") {
                for arg in &ann.args {
                    if let ast_model::AnnotationArg::Positional(lit) = arg {
                        return Some(lit.to_string().trim_matches('"').to_string());
                    }
                }

                for arg in &ann.args {
                    if let ast_model::AnnotationArg::Named(param) = arg {
                        if param.key == "strategy" {
                            return Some(param.value.to_string().trim_matches('"').to_string());
                        }
                    }
                }
            }
        }
    }

    None
}

/// Extracts the @soft_delete field name from metadata.
pub(super) fn extract_soft_delete_field(metadata: &[Metadata]) -> Option<String> {
    extract_annotation_string(metadata, "soft_delete")
}

/// Checks if @readonly annotation is present.
pub(super) fn is_readonly(metadata: &[Metadata]) -> bool {
    extract_annotation_string(metadata, "readonly").is_some()
}

/// Extracts the @pack separator from metadata.
/// Returns Some(separator) if @pack is present:
/// - `@pack` -> Some(";") (default separator)
/// - `@pack(separator: ",")` -> Some(",")
///
/// Returns None if @pack is not present.
pub(super) fn extract_pack_separator(metadata: &[Metadata]) -> Option<String> {
    for meta in metadata {
        if let Metadata::Annotation(ann) = meta {
            if ann.name.as_deref() == Some("pack") {
                // Check for separator argument
                for arg in &ann.args {
                    if let ast_model::AnnotationArg::Named(param) = arg {
                        if param.key == "separator" {
                            return Some(param.value.to_string().trim_matches('"').to_string());
                        }
                    }
                }
                // Default separator is ";"
                return Some(";".to_string());
            }
        }
    }
    None
}

/// Converts a single AST annotation to IR annotation definition.
pub(super) fn convert_annotation_to_ir(ast_ann: &ast_model::Annotation) -> AnnotationDef {
    let mut positional_args = Vec::new();
    let mut params = Vec::new();

    for arg in &ast_ann.args {
        match arg {
            ast_model::AnnotationArg::Positional(lit) => {
                positional_args.push(lit.to_string());
            }
            ast_model::AnnotationArg::Named(p) => {
                params.push(AnnotationParam {
                    key: p.key.clone(),
                    value: p.value.to_string(),
                });
            }
        }
    }

    AnnotationDef {
        name: ast_ann
            .name
            .clone()
            .unwrap_or_else(|| "unnamed".to_string()),
        positional_args,
        params,
    }
}

/// Helper to extract a string value from an annotation.
fn extract_annotation_string(metadata: &[Metadata], annotation_name: &str) -> Option<String> {
    for meta in metadata {
        if let Metadata::Annotation(ann) = meta {
            if ann.name.as_deref() == Some(annotation_name) {
                // Get the first positional or the value parameter
                if let Some(arg) = ann.args.first() {
                    match arg {
                        ast_model::AnnotationArg::Positional(lit) => {
                            return Some(lit.to_string().trim_matches('"').to_string());
                        }
                        ast_model::AnnotationArg::Named(param) => {
                            return Some(param.value.to_string().trim_matches('"').to_string());
                        }
                    }
                }
                // For annotations without arguments (like @readonly), return empty string
                return Some(String::new());
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast_model::{Annotation, AnnotationArg, AnnotationParam, Literal, Metadata};

    fn annotation(args: Vec<AnnotationArg>) -> Vec<Metadata> {
        vec![Metadata::Annotation(Annotation {
            name: Some("cache".to_string()),
            args,
        })]
    }

    #[test]
    fn cache_strategy_reads_positional_value() {
        let metadata = annotation(vec![AnnotationArg::Positional(Literal::String(
            "full_load".to_string(),
        ))]);

        assert_eq!(
            extract_cache_strategy(&metadata),
            Some("full_load".to_string())
        );
    }

    #[test]
    fn cache_strategy_reads_named_strategy_even_after_ttl() {
        let metadata = annotation(vec![
            AnnotationArg::Named(AnnotationParam {
                key: "ttl".to_string(),
                value: Literal::Integer(300),
            }),
            AnnotationArg::Named(AnnotationParam {
                key: "strategy".to_string(),
                value: Literal::Identifier("on_demand".to_string()),
            }),
        ]);

        assert_eq!(
            extract_cache_strategy(&metadata),
            Some("on_demand".to_string())
        );
    }

    #[test]
    fn cache_strategy_ignores_ttl_without_strategy() {
        let metadata = annotation(vec![AnnotationArg::Named(AnnotationParam {
            key: "ttl".to_string(),
            value: Literal::Integer(300),
        })]);

        assert_eq!(extract_cache_strategy(&metadata), None);
    }
}
