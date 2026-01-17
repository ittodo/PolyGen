use crate::error::CodeGenError;
use crate::ir_model::SchemaContext;
use crate::rhai::{register_core, register_csv};
use rhai::{Engine, Scope};
use std::path::Path;

pub fn generate_code_with_rhai(
    schema_context: &SchemaContext,
    template_path: &Path,
    output_dir: &Path,
) -> Result<String, CodeGenError> {
    let mut engine = Engine::new();
    // Increase recursion/complexity limits for larger templates
    engine.set_max_expr_depths(2048, 2048);
    register_core(&mut engine);
    register_csv(&mut engine);
    crate::rhai::register_csharp(&mut engine);

    let mut scope = Scope::new();

    // Register the schema context with the Rhai engine
    scope.push("schema", schema_context.clone());
    scope.push("output_dir", output_dir.to_string_lossy().to_string());

    let script = std::fs::read_to_string(template_path).map_err(|e| {
        CodeGenError::TemplateReadError {
            path: template_path.display().to_string(),
            source: e,
        }
    })?;

    engine
        .eval_with_scope::<String>(&mut scope, &script)
        .map_err(|e| CodeGenError::RhaiExecutionError {
            message: e.to_string(),
        })
}
