//! Template renderer: walks the parsed template tree and produces output lines.
//!
//! The renderer resolves expressions against a [`TemplateContext`], applies filters,
//! handles `%for` loops, `%if` conditionals, and `%include` directives with recursive
//! template loading. Every output line is tracked in the [`SourceMap`].

use std::collections::HashMap;
use std::path::Path;

use crate::template::context::{ContextValue, TemplateContext};
use crate::template::expr::{CondExpr, Filter};
use crate::template::filters::apply_string_filter;
use crate::template::parser::{
    IncludeBinding, LineSegment, ParsedTemplate, TemplateNode,
};
use crate::template::source_map::{SourceMap, SourceMapEntry};

/// Maximum include depth to prevent infinite recursion.
const MAX_INCLUDE_DEPTH: usize = 16;

/// Rendering configuration including TOML-based type mappings.
#[derive(Debug, Clone, Default)]
pub struct RenderConfig {
    /// Type → language type mapping from `[type_map]`.
    pub type_map: HashMap<String, String>,
    /// Optional type format from `[type_map.optional]`.
    pub type_map_optional: Option<String>,
    /// List type format from `[type_map.list]`.
    pub type_map_list: Option<String>,
    /// Type → binary read expression from `[binary_read]`.
    pub binary_read: HashMap<String, String>,
    /// Optional binary read format from `[binary_read.option]`.
    pub binary_read_option: Option<String>,
    /// List binary read format from `[binary_read.list]`.
    pub binary_read_list: Option<String>,
    /// Enum binary read format from `[binary_read.enum]`.
    pub binary_read_enum: Option<String>,
    /// Struct binary read format from `[binary_read.struct]`.
    pub binary_read_struct: Option<String>,
    /// Type → CSV read expression from `[csv_read]`.
    pub csv_read: HashMap<String, String>,
    /// Struct CSV read format from `[csv_read.struct]`.
    pub csv_read_struct: Option<String>,
    /// Set of struct FQNs that are embedded somewhere in the schema.
    pub embedded_struct_fqns: std::collections::HashSet<String>,
    /// Set of all enum FQNs in the schema.
    pub all_enum_fqns: std::collections::HashSet<String>,
}

/// Result of rendering a template: output lines + source map.
#[derive(Debug)]
pub struct RenderResult {
    /// The generated output lines.
    pub lines: Vec<String>,
    /// Source map mapping each output line to its template origin.
    pub source_map: SourceMap,
}

/// The template renderer.
pub struct Renderer<'a> {
    /// Loaded templates cache (path → parsed template).
    templates: &'a HashMap<String, ParsedTemplate>,
    /// Rendering configuration (type maps, etc.).
    config: &'a RenderConfig,
    /// Current include stack for source mapping and recursion detection.
    include_stack: Vec<String>,
    /// Current IR path for source mapping.
    ir_path: Option<String>,
    /// Accumulated output lines.
    output_lines: Vec<String>,
    /// Source map entries (one per output line).
    source_map: SourceMap,
}

impl<'a> Renderer<'a> {
    /// Creates a new renderer.
    pub fn new(
        templates: &'a HashMap<String, ParsedTemplate>,
        config: &'a RenderConfig,
    ) -> Self {
        Self {
            templates,
            config,
            include_stack: Vec::new(),
            ir_path: None,
            output_lines: Vec::new(),
            source_map: SourceMap::new(),
        }
    }

    /// Renders a template with the given context.
    pub fn render(
        mut self,
        template_name: &str,
        context: &TemplateContext,
    ) -> Result<RenderResult, String> {
        let template = self
            .templates
            .get(template_name)
            .ok_or_else(|| format!("Template not found: '{}'", template_name))?;

        self.include_stack.push(template_name.to_string());
        let nodes = template.nodes.clone();
        let source_file = template.source_file.clone();

        self.render_nodes(&nodes, context, &source_file, 0)?;

        Ok(RenderResult {
            lines: self.output_lines,
            source_map: self.source_map,
        })
    }

    /// Renders a list of template nodes.
    fn render_nodes(
        &mut self,
        nodes: &[TemplateNode],
        context: &TemplateContext,
        source_file: &str,
        indent: usize,
    ) -> Result<(), String> {
        for node in nodes {
            self.render_node(node, context, source_file, indent)?;
        }
        Ok(())
    }

    /// Renders a single template node.
    fn render_node(
        &mut self,
        node: &TemplateNode,
        context: &TemplateContext,
        source_file: &str,
        indent: usize,
    ) -> Result<(), String> {
        match node {
            TemplateNode::OutputLine { line, segments } => {
                let rendered = self.render_segments(segments, context)?;
                let indented = apply_indent(&rendered, indent);
                self.emit_line(indented, source_file, *line);
            }

            TemplateNode::BlankLine { line } => {
                self.emit_line(String::new(), source_file, *line);
            }

            TemplateNode::Conditional {
                condition,
                then_body,
                elif_branches,
                else_body,
                ..
            } => {
                if self.eval_condition(condition, context)? {
                    self.render_nodes(then_body, context, source_file, indent)?;
                } else {
                    let mut handled = false;
                    for (elif_cond, elif_body) in elif_branches {
                        if self.eval_condition(elif_cond, context)? {
                            self.render_nodes(elif_body, context, source_file, indent)?;
                            handled = true;
                            break;
                        }
                    }
                    if !handled {
                        if let Some(else_nodes) = else_body {
                            self.render_nodes(else_nodes, context, source_file, indent)?;
                        }
                    }
                }
            }

            TemplateNode::ForLoop {
                variable,
                collection,
                body,
                ..
            } => {
                let coll_value = self.resolve_collection_path(&collection.path, context)?;
                if let Some(items) = coll_value.as_list() {
                    for item in items {
                        let child_ctx = context.child_with(variable, item.clone());
                        self.render_nodes(body, &child_ctx, source_file, indent)?;
                    }
                }
            }

            TemplateNode::Include {
                line,
                template_path,
                context_bindings,
                indent: include_indent,
            } => {
                self.render_include(
                    template_path,
                    context_bindings,
                    *include_indent,
                    context,
                    source_file,
                    *line,
                    indent,
                )?;
            }
        }
        Ok(())
    }

    /// Renders an `%include` directive.
    #[allow(clippy::too_many_arguments)]
    fn render_include(
        &mut self,
        template_path: &str,
        bindings: &[IncludeBinding],
        include_indent: Option<usize>,
        context: &TemplateContext,
        _source_file: &str,
        _line: usize,
        parent_indent: usize,
    ) -> Result<(), String> {
        // Check recursion depth
        if self.include_stack.len() >= MAX_INCLUDE_DEPTH {
            return Err(format!(
                "Maximum include depth ({}) exceeded. Include stack: {:?}",
                MAX_INCLUDE_DEPTH, self.include_stack
            ));
        }

        // Resolve the template path (add .ptpl extension if needed)
        let full_path = if template_path.ends_with(".ptpl") {
            template_path.to_string()
        } else {
            format!("{}.ptpl", template_path)
        };

        let template = self
            .templates
            .get(&full_path)
            .ok_or_else(|| format!("Included template not found: '{}'", full_path))?;

        // Build child context from bindings
        let mut child_ctx = context.child();
        for binding in bindings {
            match binding {
                IncludeBinding::Focus(name) => {
                    // Resolve the focus path and set it as the primary context object
                    let path: Vec<String> = name.split('.').map(|s| s.to_string()).collect();
                    let value = self.resolve_path(&path, context)?;
                    // Infer a canonical binding name from the value type
                    let bind_name = infer_binding_name(&value, &path);
                    child_ctx.set(&bind_name, value.clone());
                    // For simple names (single segment), also bind the original name
                    // e.g., `with ns` binds both "namespace" (inferred) and "ns" (original)
                    if path.len() == 1 {
                        let original_name = &path[0];
                        if original_name != &bind_name {
                            child_ctx.set(original_name, value);
                        }
                    }
                }
                IncludeBinding::KeyValue(key, value) => {
                    child_ctx.set(key, ContextValue::String(value.clone()));
                }
            }
        }

        // Calculate total indent
        let total_indent = parent_indent + include_indent.unwrap_or(0);

        // Push onto include stack
        self.include_stack.push(full_path.clone());

        // Render the included template's nodes
        let nodes = template.nodes.clone();
        let included_source = template.source_file.clone();
        self.render_nodes(&nodes, &child_ctx, &included_source, total_indent)?;

        // Pop include stack
        self.include_stack.pop();

        Ok(())
    }

    /// Renders output line segments into a single string.
    fn render_segments(
        &self,
        segments: &[LineSegment],
        context: &TemplateContext,
    ) -> Result<String, String> {
        let mut result = String::new();
        for segment in segments {
            match segment {
                LineSegment::Literal(text) => result.push_str(text),
                LineSegment::Expression(expr) => {
                    // Resolve the property path
                    let value = self.resolve_path(&expr.path, context)?;
                    let mut text = value.to_display_string();

                    // Apply filters in order
                    for filter in &expr.filters {
                        text = self.apply_filter(&text, filter, &value, context)?;
                    }

                    result.push_str(&text);
                }
            }
        }
        Ok(result)
    }

    /// Evaluates a condition expression.
    fn eval_condition(
        &self,
        cond: &CondExpr,
        context: &TemplateContext,
    ) -> Result<bool, String> {
        match cond {
            CondExpr::Property(path) => {
                let value = self.resolve_path(path, context)?;
                Ok(value.is_truthy())
            }
            CondExpr::PropertyWithFilter(path, filter) => {
                let value = self.resolve_path(path, context)?;
                let text = value.to_display_string();
                let filtered = self.apply_filter(&text, filter, &value, context)?;
                // For boolean-returning filters like is_embedded
                Ok(filtered == "true" || (!filtered.is_empty() && filtered != "false"))
            }
            CondExpr::Not(inner) => {
                let result = self.eval_condition(inner, context)?;
                Ok(!result)
            }
            CondExpr::And(left, right) => {
                let l = self.eval_condition(left, context)?;
                let r = self.eval_condition(right, context)?;
                Ok(l && r)
            }
            CondExpr::Or(left, right) => {
                let l = self.eval_condition(left, context)?;
                let r = self.eval_condition(right, context)?;
                Ok(l || r)
            }
        }
    }

    /// Resolves a property path against the context.
    fn resolve_path(
        &self,
        path: &[String],
        context: &TemplateContext,
    ) -> Result<ContextValue, String> {
        if path.is_empty() {
            return Err("Empty property path".to_string());
        }

        // First segment: look up in context bindings
        let root_name = &path[0];
        let root_value = context
            .get(root_name)
            .cloned()
            .unwrap_or(ContextValue::Null);

        // Resolve remaining segments
        if path.len() == 1 {
            Ok(root_value)
        } else {
            Ok(root_value.resolve_path(&path[1..]))
        }
    }

    /// Resolves a collection path for `%for` loops.
    fn resolve_collection_path(
        &self,
        path: &[String],
        context: &TemplateContext,
    ) -> Result<ContextValue, String> {
        self.resolve_path(path, context)
    }

    /// Applies a filter to a string value.
    ///
    /// Simple string filters are handled by `filters.rs`.
    /// Context-dependent filters (lang_type, binary_read, etc.) are handled here
    /// using the TOML configuration.
    fn apply_filter(
        &self,
        value: &str,
        filter: &Filter,
        original_value: &ContextValue,
        context: &TemplateContext,
    ) -> Result<String, String> {
        // Try simple string filters first
        if let Some(result) = apply_string_filter(value, filter) {
            return Ok(result);
        }

        // Context-dependent filters
        match filter {
            Filter::LangType => self.apply_lang_type_filter(original_value),
            Filter::Format => self.apply_format_filter(value, original_value, context),
            Filter::Count => self.apply_count_filter(original_value),
            Filter::Join(sep) => self.apply_join_filter(original_value, sep),
            Filter::BinaryRead => self.apply_binary_read_filter(original_value),
            Filter::BinaryReadOption => self.apply_binary_read_option_filter(original_value),
            Filter::BinaryReadList => self.apply_binary_read_list_filter(original_value),
            Filter::BinaryReadStruct => self.apply_binary_read_struct_filter(original_value),
            Filter::CsvRead => self.apply_csv_read_filter(original_value),
            Filter::IsEmbedded => self.apply_is_embedded_filter(value),
            _ => Ok(value.to_string()),
        }
    }

    /// `lang_type` filter: maps a poly type to the target language type.
    fn apply_lang_type_filter(&self, value: &ContextValue) -> Result<String, String> {
        match value {
            ContextValue::TypeRef(type_ref) => {
                // Always use TOML type_map for ptpl rendering.
                // The IR's lang_type field is set for Rhai templates and may not
                // match the target language when using TOML-based type mapping.
                let base = self
                    .config
                    .type_map
                    .get(&type_ref.type_name)
                    .cloned()
                    .unwrap_or_else(|| type_ref.type_name.clone());

                if type_ref.is_option {
                    if let Some(ref fmt) = self.config.type_map_optional {
                        Ok(fmt.replace("{{type}}", &base))
                    } else {
                        Ok(format!("{}?", base))
                    }
                } else if type_ref.is_list {
                    if let Some(ref fmt) = self.config.type_map_list {
                        Ok(fmt.replace("{{type}}", &base))
                    } else {
                        Ok(format!("List<{}>", base))
                    }
                } else {
                    Ok(base)
                }
            }
            ContextValue::Field(field) => {
                // Delegate to the field's type
                self.apply_lang_type_filter(&ContextValue::TypeRef(field.field_type.clone()))
            }
            ContextValue::String(s) => {
                // Direct string lookup
                Ok(self
                    .config
                    .type_map
                    .get(s)
                    .cloned()
                    .unwrap_or_else(|| s.clone()))
            }
            _ => Ok(value.to_display_string()),
        }
    }

    /// `format` filter: formats a default value for the target language.
    fn apply_format_filter(
        &self,
        value: &str,
        _original: &ContextValue,
        _context: &TemplateContext,
    ) -> Result<String, String> {
        // For now, pass through. Language-specific formatting can be added later.
        Ok(value.to_string())
    }

    /// `count` filter: returns the count of items in a collection.
    fn apply_count_filter(&self, value: &ContextValue) -> Result<String, String> {
        match value {
            ContextValue::List(items) => Ok(items.len().to_string()),
            _ => Ok("0".to_string()),
        }
    }

    /// `join` filter: joins a list with a separator.
    fn apply_join_filter(&self, value: &ContextValue, sep: &str) -> Result<String, String> {
        match value {
            ContextValue::List(items) => {
                let strings: Vec<String> = items.iter().map(|v| v.to_display_string()).collect();
                Ok(strings.join(sep))
            }
            _ => Ok(value.to_display_string()),
        }
    }

    /// `binary_read` filter: generates binary read expression from TOML mapping.
    fn apply_binary_read_filter(&self, value: &ContextValue) -> Result<String, String> {
        let type_name = self.extract_type_name(value);
        Ok(self
            .config
            .binary_read
            .get(&type_name)
            .cloned()
            .unwrap_or_else(|| format!("/* unknown binary_read for {} */", type_name)))
    }

    /// `binary_read_option` filter: generates Option<T> binary read expression.
    fn apply_binary_read_option_filter(&self, value: &ContextValue) -> Result<String, String> {
        if let Some(ref fmt) = self.config.binary_read_option {
            let type_ref = self.extract_type_ref(value);
            let inner = type_ref.as_ref().and_then(|t| t.inner_type.as_ref());
            let inner_type_name = inner.map(|t| t.type_name.as_str()).unwrap_or("unknown");
            let lang_type = self.apply_lang_type_filter(value)?;
            let read_expr = self
                .config
                .binary_read
                .get(inner_type_name)
                .cloned()
                .unwrap_or_default();
            Ok(fmt
                .replace("{{lang_type}}", &lang_type)
                .replace("{{read_expr_lambda}}", &read_expr))
        } else {
            Ok("/* binary_read_option not configured */".to_string())
        }
    }

    /// `binary_read_list` filter: generates List<T> binary read expression.
    fn apply_binary_read_list_filter(&self, value: &ContextValue) -> Result<String, String> {
        if let Some(ref fmt) = self.config.binary_read_list {
            let type_ref = self.extract_type_ref(value);
            let inner = type_ref.as_ref().and_then(|t| t.inner_type.as_ref());
            let inner_type_name = inner.map(|t| t.type_name.as_str()).unwrap_or("unknown");
            let lang_type = self.apply_lang_type_filter(value)?;
            let read_expr = self
                .config
                .binary_read
                .get(inner_type_name)
                .cloned()
                .unwrap_or_default();
            Ok(fmt
                .replace("{{lang_type}}", &lang_type)
                .replace("{{read_expr_lambda}}", &read_expr))
        } else {
            Ok("/* binary_read_list not configured */".to_string())
        }
    }

    /// `binary_read_struct` filter: generates struct binary read expression.
    fn apply_binary_read_struct_filter(&self, value: &ContextValue) -> Result<String, String> {
        if let Some(ref fmt) = self.config.binary_read_struct {
            let type_ref = self.extract_type_ref(value);
            let type_name = type_ref.as_ref().map(|t| t.type_name.as_str()).unwrap_or("Unknown");
            let ns_fqn = type_ref.as_ref().map(|t| t.namespace_fqn.as_str()).unwrap_or("");
            Ok(fmt
                .replace("{{type_name}}", type_name)
                .replace("{{reader_ns}}", ns_fqn))
        } else {
            Ok("/* binary_read_struct not configured */".to_string())
        }
    }

    /// `csv_read` filter: generates CSV read expression from TOML mapping.
    fn apply_csv_read_filter(&self, value: &ContextValue) -> Result<String, String> {
        let type_name = self.extract_type_name(value);
        // Check for type-specific mapping first, then _default
        let expr = self
            .config
            .csv_read
            .get(&type_name)
            .or_else(|| self.config.csv_read.get("_default"))
            .cloned()
            .unwrap_or_else(|| format!("/* unknown csv_read for {} */", type_name));

        let lang_type = self.apply_lang_type_filter(value)?;
        Ok(expr.replace("{{lang_type}}", &lang_type))
    }

    /// `is_embedded` filter: checks if a struct FQN is embedded anywhere.
    fn apply_is_embedded_filter(&self, fqn: &str) -> Result<String, String> {
        Ok(self.config.embedded_struct_fqns.contains(fqn).to_string())
    }

    /// Extracts the base type name from a context value.
    fn extract_type_name(&self, value: &ContextValue) -> String {
        match value {
            ContextValue::TypeRef(t) => t.type_name.clone(),
            ContextValue::Field(f) => f.field_type.type_name.clone(),
            ContextValue::String(s) => s.clone(),
            _ => "unknown".to_string(),
        }
    }

    /// Extracts a TypeRef from a context value.
    fn extract_type_ref(&self, value: &ContextValue) -> Option<crate::ir_model::TypeRef> {
        match value {
            ContextValue::TypeRef(t) => Some(t.clone()),
            ContextValue::Field(f) => Some(f.field_type.clone()),
            _ => None,
        }
    }

    /// Emits an output line and records source map entry.
    fn emit_line(&mut self, line: String, source_file: &str, template_line: usize) {
        self.output_lines.push(line);
        self.source_map.push(SourceMapEntry {
            template_file: source_file.to_string(),
            template_line,
            include_stack: self.include_stack.clone(),
            ir_path: self.ir_path.clone(),
        });
    }

    /// Sets the current IR path for source mapping.
    pub fn set_ir_path(&mut self, path: Option<String>) {
        self.ir_path = path;
    }
}

/// Applies indentation to a line.
fn apply_indent(line: &str, indent_level: usize) -> String {
    if indent_level == 0 || line.is_empty() {
        line.to_string()
    } else {
        let indent = "    ".repeat(indent_level);
        format!("{}{}", indent, line)
    }
}

/// Infers a binding name from a context value and its path.
///
/// For example, if the path is `["item", "as_struct"]` and the value is a Struct,
/// the binding name would be "struct".
fn infer_binding_name(value: &ContextValue, path: &[String]) -> String {
    // First try to infer from the value type
    match value {
        ContextValue::Struct(_) => return "struct".to_string(),
        ContextValue::Field(_) => return "field".to_string(),
        ContextValue::Namespace(_) => return "namespace".to_string(),
        ContextValue::Enum(_) => return "enum".to_string(),
        ContextValue::EnumMember(_) => return "member".to_string(),
        ContextValue::File(_) => return "file".to_string(),
        ContextValue::Schema(_) => return "schema".to_string(),
        ContextValue::TypeRef(_) => return "type".to_string(),
        _ => {}
    }

    // Fallback: use the last path segment
    path.last()
        .map(|s| s.to_string())
        .unwrap_or_else(|| "item".to_string())
}

/// Loads all `.ptpl` template files from a directory tree into a HashMap.
///
/// The keys are relative paths from `base_dir` (e.g., `"section/namespace_block.ptpl"`).
pub fn load_templates(base_dir: &Path) -> Result<HashMap<String, ParsedTemplate>, String> {
    let mut templates = HashMap::new();
    load_templates_recursive(base_dir, base_dir, &mut templates)?;
    Ok(templates)
}

/// Recursively loads `.ptpl` files.
fn load_templates_recursive(
    dir: &Path,
    base_dir: &Path,
    templates: &mut HashMap<String, ParsedTemplate>,
) -> Result<(), String> {
    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("Failed to read directory '{}': {}", dir.display(), e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Directory entry error: {}", e))?;
        let path = entry.path();

        if path.is_dir() {
            load_templates_recursive(&path, base_dir, templates)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("ptpl") {
            let relative = path
                .strip_prefix(base_dir)
                .map_err(|e| format!("Path strip error: {}", e))?;
            let key = relative.to_string_lossy().replace('\\', "/");

            let source = std::fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read '{}': {}", path.display(), e))?;

            let parsed = crate::template::parser::parse_template(&source, &key)?;
            templates.insert(key, parsed);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::template::parser::parse_template;

    fn make_templates(entries: &[(&str, &str)]) -> HashMap<String, ParsedTemplate> {
        let mut map = HashMap::new();
        for (name, source) in entries {
            let parsed = parse_template(source, name).unwrap();
            map.insert(name.to_string(), parsed);
        }
        map
    }

    #[test]
    fn test_render_simple_output() {
        let templates = make_templates(&[("test.ptpl", "Hello, World!")]);
        let config = RenderConfig::default();
        let ctx = TemplateContext::new();

        let renderer = Renderer::new(&templates, &config);
        let result = renderer.render("test.ptpl", &ctx).unwrap();

        assert_eq!(result.lines, vec!["Hello, World!"]);
        assert_eq!(result.source_map.len(), 1);
        assert_eq!(result.source_map.entries[0].template_line, 1);
    }

    #[test]
    fn test_render_interpolation() {
        let templates = make_templates(&[("test.ptpl", "Hello, {{name}}!")]);
        let config = RenderConfig::default();
        let mut ctx = TemplateContext::new();
        ctx.set("name", ContextValue::String("PolyGen".to_string()));

        let renderer = Renderer::new(&templates, &config);
        let result = renderer.render("test.ptpl", &ctx).unwrap();

        assert_eq!(result.lines, vec!["Hello, PolyGen!"]);
    }

    #[test]
    fn test_render_for_loop() {
        let source = "%for item in items\n  - {{item}}\n%endfor";
        let templates = make_templates(&[("test.ptpl", source)]);
        let config = RenderConfig::default();
        let mut ctx = TemplateContext::new();
        ctx.set(
            "items",
            ContextValue::List(vec![
                ContextValue::String("alpha".to_string()),
                ContextValue::String("beta".to_string()),
            ]),
        );

        let renderer = Renderer::new(&templates, &config);
        let result = renderer.render("test.ptpl", &ctx).unwrap();

        assert_eq!(result.lines, vec!["  - alpha", "  - beta"]);
    }

    #[test]
    fn test_render_conditional() {
        let source = "%if show\nvisible\n%else\nhidden\n%endif";
        let templates = make_templates(&[("test.ptpl", source)]);
        let config = RenderConfig::default();

        // Test true branch
        let mut ctx = TemplateContext::new();
        ctx.set("show", ContextValue::Bool(true));
        let renderer = Renderer::new(&templates, &config);
        let result = renderer.render("test.ptpl", &ctx).unwrap();
        assert_eq!(result.lines, vec!["visible"]);

        // Test false branch
        let mut ctx = TemplateContext::new();
        ctx.set("show", ContextValue::Bool(false));
        let renderer = Renderer::new(&templates, &config);
        let result = renderer.render("test.ptpl", &ctx).unwrap();
        assert_eq!(result.lines, vec!["hidden"]);
    }

    #[test]
    fn test_render_blank_line() {
        let source = "line1\n%blank\nline2";
        let templates = make_templates(&[("test.ptpl", source)]);
        let config = RenderConfig::default();
        let ctx = TemplateContext::new();

        let renderer = Renderer::new(&templates, &config);
        let result = renderer.render("test.ptpl", &ctx).unwrap();

        assert_eq!(result.lines, vec!["line1", "", "line2"]);
    }

    #[test]
    fn test_render_include() {
        let main = "%include \"child.ptpl\" with name";
        let child = "Hello from child: {{name}}";
        let templates = make_templates(&[("main.ptpl", main), ("child.ptpl", child)]);
        let config = RenderConfig::default();
        let mut ctx = TemplateContext::new();
        ctx.set("name", ContextValue::String("World".to_string()));

        let renderer = Renderer::new(&templates, &config);
        let result = renderer.render("main.ptpl", &ctx).unwrap();

        assert_eq!(result.lines, vec!["Hello from child: World"]);
        // Check include stack in source map
        assert_eq!(result.source_map.entries[0].include_stack.len(), 2);
    }

    #[test]
    fn test_render_include_with_indent() {
        let main = "root\n%include \"child.ptpl\" indent=1";
        let child = "indented line";
        let templates = make_templates(&[("main.ptpl", main), ("child.ptpl", child)]);
        let config = RenderConfig::default();
        let ctx = TemplateContext::new();

        let renderer = Renderer::new(&templates, &config);
        let result = renderer.render("main.ptpl", &ctx).unwrap();

        assert_eq!(result.lines, vec!["root", "    indented line"]);
    }

    #[test]
    fn test_render_filter() {
        let source = "{{name | upper}}";
        let templates = make_templates(&[("test.ptpl", source)]);
        let config = RenderConfig::default();
        let mut ctx = TemplateContext::new();
        ctx.set("name", ContextValue::String("hello".to_string()));

        let renderer = Renderer::new(&templates, &config);
        let result = renderer.render("test.ptpl", &ctx).unwrap();

        assert_eq!(result.lines, vec!["HELLO"]);
    }

    #[test]
    fn test_render_elif() {
        let source =
            "%if a\nA\n%elif b\nB\n%elif c\nC\n%else\nD\n%endif";
        let templates = make_templates(&[("test.ptpl", source)]);
        let config = RenderConfig::default();

        // Test second elif
        let mut ctx = TemplateContext::new();
        ctx.set("a", ContextValue::Bool(false));
        ctx.set("b", ContextValue::Bool(false));
        ctx.set("c", ContextValue::Bool(true));
        let renderer = Renderer::new(&templates, &config);
        let result = renderer.render("test.ptpl", &ctx).unwrap();
        assert_eq!(result.lines, vec!["C"]);
    }

    #[test]
    fn test_source_map_tracking() {
        let source = "line1\nline2\nline3";
        let templates = make_templates(&[("test.ptpl", source)]);
        let config = RenderConfig::default();
        let ctx = TemplateContext::new();

        let renderer = Renderer::new(&templates, &config);
        let result = renderer.render("test.ptpl", &ctx).unwrap();

        assert_eq!(result.source_map.len(), 3);
        assert_eq!(result.source_map.entries[0].template_line, 1);
        assert_eq!(result.source_map.entries[1].template_line, 2);
        assert_eq!(result.source_map.entries[2].template_line, 3);
    }
}
