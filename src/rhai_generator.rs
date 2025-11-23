use crate::ir_model::{
    EnumDef, EnumItem, EnumMember, FieldDef, FileDef, NamespaceDef, NamespaceItem, SchemaContext,
    StructDef, StructItem,
};
use crate::rhai::{register_core, register_csv};
use rhai::{Array, Dynamic, Engine, Scope};
use std::path::Path;

pub fn generate_code_with_rhai(
    schema_context: &SchemaContext,
    template_path: &Path,
    output_dir: &Path,
) -> Result<String, String> {
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

    let script = match std::fs::read_to_string(template_path) {
        Ok(s) => s,
        Err(e) => return Err(e.to_string()),
    };

    match engine.eval_with_scope::<String>(&mut scope, &script) {
        Ok(result) => Ok(result),
        Err(e) => Err(e.to_string()),
    }
}
