use crate::ast_model::{self, Metadata};
use crate::ir_model::{AnnotationDef, AnnotationParam};

/// Extracts the @datasource value from metadata.
pub(super) fn extract_datasource(metadata: &[Metadata]) -> Option<String> {
    extract_annotation_string(metadata, "datasource")
}

/// Extracts the @cache strategy from metadata.
pub(super) fn extract_cache_strategy(metadata: &[Metadata]) -> Option<String> {
    extract_annotation_string(metadata, "cache")
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
