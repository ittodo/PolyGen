//! Rhai Template Engine Integration
//!
//! This module provides the bridge between PolyGen's IR (Intermediate Representation)
//! and the Rhai scripting engine used for code generation templates.
//!
//! ## Overview
//!
//! The Rhai generator executes `.rhai` template scripts with access to:
//! - `schema`: The complete [`SchemaContext`] containing all parsed type definitions
//! - `output_dir`: The target directory for generated files
//!
//! Templates use Rhai's scripting capabilities combined with registered helper
//! functions to generate language-specific code (C#, C++, Rust, TypeScript, etc.).
//!
//! ## Template Structure
//!
//! Templates are located in `templates/<language>/` and typically include:
//! - `<lang>_file.rhai`: Main entry point for code generation
//! - `<lang>_*_file.rhai`: Additional generators (CSV loaders, JSON mappers, etc.)
//! - `rhai_utils/`: Shared utility functions

use crate::error::CodeGenError;
use crate::ir_model::SchemaContext;
use crate::rhai::{register_core_with_entry, register_csharp};
use rhai::{Engine, Scope};
use std::path::Path;

/// Executes a Rhai template script to generate code from the schema context.
///
/// This function sets up the Rhai engine with all registered helper functions,
/// injects the schema context and output directory, then evaluates the template.
///
/// # Arguments
///
/// * `schema_context` - The IR containing all parsed type definitions
/// * `template_path` - Path to the `.rhai` template file
/// * `output_dir` - Target directory where generated files will be written
///
/// # Returns
///
/// Returns the template's output string on success, or a [`CodeGenError`] on failure.
///
/// # Errors
///
/// * [`CodeGenError::TemplateReadError`] - Failed to read the template file
/// * [`CodeGenError::RhaiExecutionError`] - Template execution failed
pub fn generate_code_with_rhai(
    schema_context: &SchemaContext,
    template_path: &Path,
    output_dir: &Path,
) -> Result<String, CodeGenError> {
    generate_code_with_rhai_opts(schema_context, template_path, output_dir, false)
}

/// Executes a Rhai template script with optional preview mode.
///
/// When `preview_mode` is true, a `_preview_mode` boolean variable is injected
/// into the Rhai scope, allowing templates to conditionally emit source markers
/// using `source_mark(template_name, content)`.
pub fn generate_code_with_rhai_opts(
    schema_context: &SchemaContext,
    template_path: &Path,
    output_dir: &Path,
    preview_mode: bool,
) -> Result<String, CodeGenError> {
    let mut engine = Engine::new();
    // Increase recursion/complexity limits for larger templates
    engine.set_max_expr_depths(2048, 2048);
    let entry_name = template_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    register_core_with_entry(&mut engine, preview_mode, entry_name);
    register_csharp(&mut engine);

    let mut scope = Scope::new();

    // Register the schema context with the Rhai engine
    scope.push("schema", schema_context.clone());
    scope.push("output_dir", output_dir.to_string_lossy().to_string());
    scope.push("_preview_mode", preview_mode);

    let script =
        std::fs::read_to_string(template_path).map_err(|e| CodeGenError::TemplateReadError {
            path: template_path.display().to_string(),
            source: e,
        })?;

    engine
        .eval_with_scope::<String>(&mut scope, &script)
        .map_err(|e| CodeGenError::RhaiExecutionError {
            message: e.to_string(),
        })
}
