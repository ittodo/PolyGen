//! Rhai bridge for `%logic` blocks in PolyTemplate v2.
//!
//! Provides a sandboxed Rhai engine for computation within templates.
//! IR type registration, case-conversion functions, and optional I/O
//! operations (`write_file`) are available.
//!
//! The bridge converts between [`ContextValue`] and Rhai [`Dynamic`] values
//! at the `%endlogic` boundary.

use std::collections::HashMap;
use std::path::Path;

use rhai::{Dynamic, Engine, EvalAltResult, Scope, AST};

use crate::template::context::ContextValue;

/// Bridge between PolyTemplate and the Rhai scripting engine.
pub struct RhaiBridge {
    engine: Engine,
    scope: Scope<'static>,
    /// Compiled prelude AST containing function definitions.
    prelude_ast: Option<AST>,
}

impl Default for RhaiBridge {
    fn default() -> Self {
        Self::new()
    }
}

impl RhaiBridge {
    /// Creates a new Rhai bridge with IR types registered.
    pub fn new() -> Self {
        let mut engine = Engine::new();
        engine.set_max_expr_depths(2048, 2048);

        // Register IR types so Rhai can work with them
        crate::rhai::registry::register_types_and_getters(&mut engine);

        // Register basic case-conversion helpers
        use heck::{ToLowerCamelCase, ToPascalCase, ToSnakeCase};
        engine.register_fn("to_snake_case", |s: &str| s.to_snake_case());
        engine.register_fn("to_pascal_case", |s: &str| s.to_pascal_case());
        engine.register_fn("to_camel_case", |s: &str| s.to_lower_camel_case());

        Self {
            engine,
            scope: Scope::new(),
            prelude_ast: None,
        }
    }

    /// Executes a `%logic` block body and returns newly created/modified variables.
    ///
    /// 1. Syncs `context_bindings` into the Rhai Scope
    /// 2. Compiles the body, merging with prelude AST if present
    /// 3. Executes the combined AST
    /// 4. Extracts all Scope variables back as `ContextValue`
    /// 5. Returns the variables (caller merges into `file_bindings`)
    pub fn execute_logic(
        &mut self,
        body: &str,
        file_bindings: &HashMap<String, ContextValue>,
    ) -> Result<HashMap<String, ContextValue>, String> {
        // Sync file_bindings into Rhai Scope
        for (name, value) in file_bindings {
            let dynamic = context_value_to_dynamic(value);
            if self.scope.contains(name) {
                self.scope.set_value(name.to_string(), dynamic);
            } else {
                self.scope.push_dynamic(name.to_string(), dynamic);
            }
        }

        // Compile the logic body
        let body_ast = self
            .engine
            .compile(body)
            .map_err(|e| format!("Rhai compilation error: {}", e))?;

        // Merge with prelude AST if present (prelude functions become available)
        let combined_ast = if let Some(ref prelude) = self.prelude_ast {
            prelude.merge(&body_ast)
        } else {
            body_ast
        };

        // Execute the combined AST
        let _ = self
            .engine
            .eval_ast_with_scope::<Dynamic>(&mut self.scope, &combined_ast)
            .map_err(|e| format!("Rhai execution error: {}", e))?;

        // Extract all scope variables back as ContextValue
        let mut result = HashMap::new();
        for (name, _, value) in self.scope.iter() {
            match dynamic_to_context_value(value.clone()) {
                Ok(cv) => {
                    result.insert(name.to_string(), cv);
                }
                Err(e) => {
                    return Err(format!(
                        "Error converting Rhai variable '{}' to ContextValue: {}",
                        name, e
                    ));
                }
            }
        }

        Ok(result)
    }

    /// Loads prelude scripts by compiling them into a merged AST.
    ///
    /// Functions defined in prelude scripts become available
    /// in all subsequent `execute_logic()` calls.
    pub fn load_prelude(&mut self, scripts: &[String]) -> Result<(), String> {
        let mut combined: Option<AST> = None;
        for (i, script) in scripts.iter().enumerate() {
            let ast = self
                .engine
                .compile(script)
                .map_err(|e| format!("Rhai prelude script #{} error: {}", i + 1, e))?;
            combined = Some(match combined {
                Some(existing) => existing.merge(&ast),
                None => ast,
            });
        }
        self.prelude_ast = combined;
        Ok(())
    }

    /// Creates a child bridge for `%include` isolation.
    ///
    /// The child shares the same engine configuration but has a fresh scope.
    pub fn child(&self) -> Self {
        let mut engine = Engine::new();
        engine.set_max_expr_depths(2048, 2048);
        crate::rhai::registry::register_types_and_getters(&mut engine);

        use heck::{ToLowerCamelCase, ToPascalCase, ToSnakeCase};
        engine.register_fn("to_snake_case", |s: &str| s.to_snake_case());
        engine.register_fn("to_pascal_case", |s: &str| s.to_pascal_case());
        engine.register_fn("to_camel_case", |s: &str| s.to_lower_camel_case());

        Self {
            engine,
            scope: Scope::new(),
            prelude_ast: None,
        }
    }

    /// Registers the `write_file(path, content)` function on the Rhai engine.
    ///
    /// This enables `%logic` blocks to write files directly (e.g., CSV schema files).
    /// When `preview_mode` is true and `entry_template` is set, wraps content
    /// with `/*@source:template_name*/` markers for GUI preview hinting.
    pub fn register_write_file(
        &mut self,
        preview_mode: bool,
        entry_template: Option<String>,
    ) {
        let preview_write = preview_mode;
        let entry_tmpl = entry_template;
        self.engine.register_fn(
            "write_file",
            move |path: &str, content: &str| -> Result<(), Box<EvalAltResult>> {
                // In preview mode, wrap unmarked content with the entry-point template name
                let final_content = if preview_write {
                    if let Some(ref tmpl_name) = entry_tmpl {
                        if !content.contains("/*@source:") {
                            format!("/*@source:{}*/\n{}/*@/source*/\n", tmpl_name, content)
                        } else {
                            content.to_string()
                        }
                    } else {
                        content.to_string()
                    }
                } else {
                    content.to_string()
                };

                if let Some(p) = Path::new(path).parent() {
                    if !p.exists() {
                        if let Err(e) = std::fs::create_dir_all(p) {
                            return Err(Box::new(EvalAltResult::ErrorSystem(
                                "Directory Creation Error".to_string(),
                                e.to_string().into(),
                            )));
                        }
                    }
                }
                match std::fs::write(path, &final_content) {
                    Ok(_) => Ok(()),
                    Err(e) => Err(Box::new(EvalAltResult::ErrorSystem(
                        "File Write Error".to_string(),
                        e.to_string().into(),
                    ))),
                }
            },
        );
    }
}

/// Converts a [`ContextValue`] to a Rhai [`Dynamic`] value.
pub fn context_value_to_dynamic(value: &ContextValue) -> Dynamic {
    match value {
        ContextValue::String(s) => Dynamic::from(s.clone()),
        ContextValue::Bool(b) => Dynamic::from(*b),
        ContextValue::Int(i) => Dynamic::from(*i),
        ContextValue::Float(f) => Dynamic::from(*f),
        ContextValue::List(items) => {
            let arr: Vec<Dynamic> = items.iter().map(context_value_to_dynamic).collect();
            Dynamic::from(arr)
        }
        ContextValue::Map(map) => {
            let mut rhai_map = rhai::Map::new();
            for (k, v) in map {
                rhai_map.insert(k.clone().into(), context_value_to_dynamic(v));
            }
            Dynamic::from(rhai_map)
        }
        ContextValue::Null => Dynamic::UNIT,
        // IR types: wrap as-is using Dynamic::from()
        ContextValue::Schema(s) => Dynamic::from(s.clone()),
        ContextValue::File(f) => Dynamic::from(f.clone()),
        ContextValue::Namespace(ns) => Dynamic::from(ns.clone()),
        ContextValue::Struct(s) => Dynamic::from(s.clone()),
        ContextValue::Field(f) => Dynamic::from(*f.clone()),
        ContextValue::TypeRef(t) => Dynamic::from(t.clone()),
        ContextValue::Enum(e) => Dynamic::from(e.clone()),
        ContextValue::EnumMember(m) => Dynamic::from(m.clone()),
        _ => Dynamic::from(value.to_display_string()),
    }
}

/// Converts a Rhai [`Dynamic`] value to a [`ContextValue`].
///
/// Enforces the single-line string constraint: strings containing `\n` are rejected.
pub fn dynamic_to_context_value(value: Dynamic) -> Result<ContextValue, String> {
    if value.is_unit() {
        return Ok(ContextValue::Null);
    }
    if value.is_bool() {
        return Ok(ContextValue::Bool(value.as_bool().unwrap()));
    }
    if value.is_int() {
        return Ok(ContextValue::Int(value.as_int().unwrap()));
    }
    if value.is_float() {
        return Ok(ContextValue::Float(value.as_float().unwrap()));
    }
    if value.is_string() {
        let s = value.into_string().unwrap();
        return Ok(ContextValue::String(s));
    }
    if value.is_array() {
        let arr = value.into_array().unwrap();
        let mut items = Vec::new();
        for item in arr {
            items.push(dynamic_to_context_value(item)?);
        }
        return Ok(ContextValue::List(items));
    }
    if value.is_map() {
        let rhai_map = value.cast::<rhai::Map>();
        let mut map = HashMap::new();
        for (k, v) in rhai_map {
            let key = k.to_string();
            let val = dynamic_to_context_value(v)?;
            map.insert(key, val);
        }
        return Ok(ContextValue::Map(map));
    }

    // IR types: try to downcast
    if let Some(s) = value.clone().try_cast::<crate::ir_model::SchemaContext>() {
        return Ok(ContextValue::Schema(s));
    }
    if let Some(f) = value.clone().try_cast::<crate::ir_model::FileDef>() {
        return Ok(ContextValue::File(f));
    }
    if let Some(ns) = value.clone().try_cast::<crate::ir_model::NamespaceDef>() {
        return Ok(ContextValue::Namespace(ns));
    }
    if let Some(s) = value.clone().try_cast::<crate::ir_model::StructDef>() {
        return Ok(ContextValue::Struct(s));
    }
    if let Some(f) = value.clone().try_cast::<crate::ir_model::FieldDef>() {
        return Ok(ContextValue::Field(Box::new(f)));
    }
    if let Some(t) = value.clone().try_cast::<crate::ir_model::TypeRef>() {
        return Ok(ContextValue::TypeRef(t));
    }
    if let Some(e) = value.clone().try_cast::<crate::ir_model::EnumDef>() {
        return Ok(ContextValue::Enum(e));
    }
    if let Some(m) = value.clone().try_cast::<crate::ir_model::EnumMember>() {
        return Ok(ContextValue::EnumMember(m));
    }

    // Fallback: convert to string
    Ok(ContextValue::String(value.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execute_logic_simple_variable() {
        let mut bridge = RhaiBridge::new();
        let bindings = HashMap::new();
        let result = bridge.execute_logic("let x = 42;", &bindings).unwrap();
        assert_eq!(result.get("x").unwrap().to_display_string(), "42");
    }

    #[test]
    fn test_execute_logic_string_variable() {
        let mut bridge = RhaiBridge::new();
        let bindings = HashMap::new();
        let result = bridge
            .execute_logic("let name = \"hello\";", &bindings)
            .unwrap();
        assert_eq!(result.get("name").unwrap().to_display_string(), "hello");
    }

    #[test]
    fn test_execute_logic_reads_file_bindings() {
        let mut bridge = RhaiBridge::new();
        let mut bindings = HashMap::new();
        bindings.insert("base".to_string(), ContextValue::Int(10));

        let result = bridge
            .execute_logic("let doubled = base * 2;", &bindings)
            .unwrap();
        assert_eq!(result.get("doubled").unwrap().to_display_string(), "20");
    }

    #[test]
    fn test_execute_logic_multiline_string() {
        let mut bridge = RhaiBridge::new();
        let bindings = HashMap::new();
        let result = bridge
            .execute_logic("let multiline = \"line1\\nline2\";", &bindings)
            .unwrap();
        assert_eq!(
            result.get("multiline").unwrap().to_display_string(),
            "line1\nline2"
        );
    }

    #[test]
    fn test_execute_logic_case_functions() {
        let mut bridge = RhaiBridge::new();
        let bindings = HashMap::new();
        let result = bridge
            .execute_logic("let s = to_pascal_case(\"hello_world\");", &bindings)
            .unwrap();
        assert_eq!(result.get("s").unwrap().to_display_string(), "HelloWorld");
    }

    #[test]
    fn test_execute_logic_function_definition() {
        let mut bridge = RhaiBridge::new();
        let bindings = HashMap::new();
        let result = bridge
            .execute_logic("fn add(a, b) { a + b }\nlet result = add(3, 4);", &bindings)
            .unwrap();
        assert_eq!(result.get("result").unwrap().to_display_string(), "7");
    }

    #[test]
    fn test_context_value_roundtrip() {
        // String
        let cv = ContextValue::String("hello".to_string());
        let dyn_val = context_value_to_dynamic(&cv);
        let back = dynamic_to_context_value(dyn_val).unwrap();
        assert_eq!(back.to_display_string(), "hello");

        // Int
        let cv = ContextValue::Int(42);
        let dyn_val = context_value_to_dynamic(&cv);
        let back = dynamic_to_context_value(dyn_val).unwrap();
        assert_eq!(back.to_display_string(), "42");

        // Bool
        let cv = ContextValue::Bool(true);
        let dyn_val = context_value_to_dynamic(&cv);
        let back = dynamic_to_context_value(dyn_val).unwrap();
        assert!(back.is_truthy());
    }

    #[test]
    fn test_child_isolation() {
        let mut bridge = RhaiBridge::new();
        let bindings = HashMap::new();
        bridge
            .execute_logic("let parent_var = 1;", &bindings)
            .unwrap();

        let mut child = bridge.child();
        let child_result = child
            .execute_logic("let child_var = 2;", &HashMap::new())
            .unwrap();

        // Child should not have parent's variables
        assert!(child_result.get("parent_var").is_none());
        assert!(child_result.get("child_var").is_some());
    }

    #[test]
    fn test_load_prelude() {
        let mut bridge = RhaiBridge::new();
        bridge
            .load_prelude(&["fn triple(x) { x * 3 }".to_string()])
            .unwrap();

        let bindings = HashMap::new();
        let result = bridge
            .execute_logic("let val = triple(7);", &bindings)
            .unwrap();
        assert_eq!(result.get("val").unwrap().to_display_string(), "21");
    }

    #[test]
    fn test_load_prelude_error() {
        let mut bridge = RhaiBridge::new();
        let result = bridge.load_prelude(&["invalid rhai {{{{".to_string()]);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("prelude script #1"));
    }
}
