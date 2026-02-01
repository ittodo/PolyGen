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
    IncludeBinding, LineSegment, MatchArm, ParsedTemplate, TemplateNode,
};
use crate::template::rhai_bridge::RhaiBridge;
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
    /// Non-primitive type format from `[type_map.non_primitive]`.
    /// Use `{{type}}` for the resolved type name.
    /// Example: `"global::{{type}}"` for C#.
    pub type_map_non_primitive: Option<String>,
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
    /// Rhai prelude script contents to execute before `%logic` blocks.
    ///
    /// Each entry is the full source code of a prelude script.
    /// Functions defined here are available in all `%logic` blocks.
    pub rhai_prelude: Vec<String>,
    /// When true, registers `write_file(path, content)` on the Rhai bridge,
    /// allowing `%logic` blocks to write files directly.
    pub enable_write_file: bool,
}

/// Result of rendering a template: output lines + source map.
#[derive(Debug)]
pub struct RenderResult {
    /// The generated output lines.
    pub lines: Vec<String>,
    /// Source map mapping each output line to its template origin.
    pub source_map: SourceMap,
}

/// Maximum while-loop iterations to prevent infinite loops.
const MAX_WHILE_ITERATIONS: usize = 10_000;

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
    /// File-scope variable bindings from `%let`/`%set` and `%logic`.
    file_bindings: HashMap<String, ContextValue>,
    /// Lazy-initialized Rhai bridge for `%logic` blocks.
    rhai_bridge: Option<RhaiBridge>,
    /// Defined blocks from `%block name(params)` for `%render`.
    blocks: HashMap<String, (Vec<String>, Vec<TemplateNode>)>,
}

impl<'a> Renderer<'a> {
    /// Creates a new renderer.
    pub fn new(templates: &'a HashMap<String, ParsedTemplate>, config: &'a RenderConfig) -> Self {
        Self {
            templates,
            config,
            include_stack: Vec::new(),
            ir_path: None,
            output_lines: Vec::new(),
            source_map: SourceMap::new(),
            file_bindings: HashMap::new(),
            rhai_bridge: None,
            blocks: HashMap::new(),
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
                // Handle multi-line strings from %logic blocks
                if rendered.contains('\n') {
                    for sub_line in rendered.split('\n') {
                        let indented = apply_indent(sub_line, indent);
                        self.emit_line(indented, source_file, *line);
                    }
                } else {
                    let indented = apply_indent(&rendered, indent);
                    self.emit_line(indented, source_file, *line);
                }
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
                        // Apply where filter if present
                        if let Some(ref where_cond) = collection.where_filter {
                            if !self.eval_condition(where_cond, &child_ctx)? {
                                continue;
                            }
                        }
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

            TemplateNode::LetSet { name, expr, .. } => {
                let value = self.eval_let_expr(expr, context)?;
                self.file_bindings.insert(name.clone(), value);
            }

            TemplateNode::While {
                condition, body, ..
            } => {
                let mut iterations = 0;
                while self.eval_condition(condition, context)? {
                    self.render_nodes(body, context, source_file, indent)?;
                    iterations += 1;
                    if iterations >= MAX_WHILE_ITERATIONS {
                        return Err(format!(
                            "{}:{}: %while loop exceeded maximum iterations ({})",
                            source_file,
                            match node {
                                TemplateNode::While { line, .. } => *line,
                                _ => 0,
                            },
                            MAX_WHILE_ITERATIONS
                        ));
                    }
                }
            }

            TemplateNode::LogicBlock { line, body } => {
                // Lazy-init the Rhai bridge with prelude scripts
                if self.rhai_bridge.is_none() {
                    let mut bridge = RhaiBridge::new();
                    if self.config.enable_write_file {
                        bridge.register_write_file();
                    }
                    if !self.config.rhai_prelude.is_empty() {
                        bridge
                            .load_prelude(&self.config.rhai_prelude)
                            .map_err(|e| format!("{}:{}: {}", source_file, line, e))?;
                    }
                    self.rhai_bridge = Some(bridge);
                }
                let bridge = self.rhai_bridge.as_mut().unwrap();
                // Merge context bindings + file_bindings (file_bindings take priority)
                let mut combined = context.bindings().clone();
                for (k, v) in &self.file_bindings {
                    combined.insert(k.clone(), v.clone());
                }
                let new_bindings = bridge
                    .execute_logic(body, &combined)
                    .map_err(|e| format!("{}:{}: {}", source_file, line, e))?;
                // Only merge NEW variables back (not context originals)
                for (name, value) in new_bindings {
                    if context.get(&name).is_none() {
                        self.file_bindings.insert(name, value);
                    }
                }
            }
            TemplateNode::Match {
                line,
                subject,
                arms,
                else_body,
            } => {
                self.render_match(
                    subject,
                    arms,
                    else_body.as_deref(),
                    context,
                    source_file,
                    *line,
                    indent,
                )?;
            }

            TemplateNode::BlockDef {
                name, params, body, ..
            } => {
                // Register block for later %render calls (no output)
                self.blocks
                    .insert(name.clone(), (params.clone(), body.clone()));
            }

            TemplateNode::Render {
                line,
                target,
                binding,
            } => {
                self.render_block_call(
                    target,
                    binding.as_deref(),
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

        // Push onto include stack, save and clear file_bindings, rhai_bridge, blocks for isolation
        self.include_stack.push(full_path.clone());
        let saved_bindings = std::mem::take(&mut self.file_bindings);
        let saved_bridge = self.rhai_bridge.take();
        let saved_blocks = std::mem::take(&mut self.blocks);

        // Render the included template's nodes
        let nodes = template.nodes.clone();
        let included_source = template.source_file.clone();
        let result = self.render_nodes(&nodes, &child_ctx, &included_source, total_indent);

        // Restore file_bindings, rhai_bridge, blocks and pop include stack
        self.file_bindings = saved_bindings;
        self.rhai_bridge = saved_bridge;
        self.blocks = saved_blocks;
        self.include_stack.pop();

        result?;

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
    fn eval_condition(&self, cond: &CondExpr, context: &TemplateContext) -> Result<bool, String> {
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
            CondExpr::Equals(path, literal) => {
                let value = self.resolve_path(path, context)?;
                Ok(value.to_display_string() == *literal)
            }
            CondExpr::NotEquals(path, literal) => {
                let value = self.resolve_path(path, context)?;
                Ok(value.to_display_string() != *literal)
            }
        }
    }

    /// Resolves a property path against file_bindings first, then context.
    fn resolve_path(
        &self,
        path: &[String],
        context: &TemplateContext,
    ) -> Result<ContextValue, String> {
        if path.is_empty() {
            return Err("Empty property path".to_string());
        }

        // First segment: check file_bindings, then context
        let root_name = &path[0];
        let root_value = self
            .file_bindings
            .get(root_name)
            .cloned()
            .or_else(|| context.get(root_name).cloned())
            .unwrap_or(ContextValue::Null);

        // Resolve remaining segments
        if path.len() == 1 {
            Ok(root_value)
        } else {
            Ok(root_value.resolve_path(&path[1..]))
        }
    }

    /// Renders a `%match` construct.
    #[allow(clippy::too_many_arguments)]
    fn render_match(
        &mut self,
        subject: &str,
        arms: &[MatchArm],
        else_body: Option<&[TemplateNode]>,
        context: &TemplateContext,
        source_file: &str,
        _line: usize,
        indent: usize,
    ) -> Result<(), String> {
        // Evaluate subject to a string value
        let subject_value = self.eval_match_subject(subject, context)?;

        // Try each arm
        for arm in arms {
            if self.match_pattern(&subject_value, &arm.pattern)? {
                // Check guard if present
                if let Some(ref guard) = arm.guard {
                    let guard_cond = crate::template::expr::parse_condition(guard)
                        .map_err(|e| format!("{}:{}: guard error: {}", source_file, arm.line, e))?;
                    if !self.eval_condition(&guard_cond, context)? {
                        continue;
                    }
                }
                self.render_nodes(&arm.body, context, source_file, indent)?;
                return Ok(());
            }
        }

        // No arm matched — try else
        if let Some(else_nodes) = else_body {
            self.render_nodes(else_nodes, context, source_file, indent)?;
        }

        Ok(())
    }

    /// Evaluates a match subject expression to a string.
    fn eval_match_subject(
        &self,
        subject: &str,
        context: &TemplateContext,
    ) -> Result<String, String> {
        let path: Vec<String> = subject.split('.').map(|s| s.trim().to_string()).collect();
        let value = self.resolve_path(&path, context)?;
        Ok(value.to_display_string())
    }

    /// Checks if a subject value matches a pattern.
    ///
    /// Patterns:
    /// - `_` — wildcard, always matches
    /// - `"literal"` — string literal comparison
    /// - bare value — compared as string
    fn match_pattern(&self, subject: &str, pattern: &str) -> Result<bool, String> {
        let pattern = pattern.trim();

        // Wildcard
        if pattern == "_" {
            return Ok(true);
        }

        // String literal: `"value"`
        if pattern.starts_with('"') && pattern.ends_with('"') && pattern.len() >= 2 {
            let literal = &pattern[1..pattern.len() - 1];
            return Ok(subject == literal);
        }

        // Bare value comparison
        Ok(subject == pattern)
    }

    /// Renders a `%render` block call.
    #[allow(clippy::too_many_arguments)]
    fn render_block_call(
        &mut self,
        target: &str,
        binding: Option<&str>,
        context: &TemplateContext,
        source_file: &str,
        line: usize,
        indent: usize,
    ) -> Result<(), String> {
        // Dynamic dispatch: $var_name -> resolve from file_bindings
        let block_name = if let Some(var_name) = target.strip_prefix('$') {
            self.file_bindings
                .get(var_name)
                .map(|v| v.to_display_string())
                .ok_or_else(|| {
                    format!(
                        "{}:{}: Dynamic block name '{}' not found in bindings",
                        source_file, line, var_name
                    )
                })?
        } else {
            target.to_string()
        };

        // Look up block
        let (params, body) = self.blocks.get(&block_name).cloned().ok_or_else(|| {
            format!(
                "{}:{}: Block '{}' not defined",
                source_file, line, block_name
            )
        })?;

        // Build context with binding
        let child_ctx = if let Some(binding_expr) = binding {
            let path: Vec<String> = binding_expr
                .split('.')
                .map(|s| s.trim().to_string())
                .collect();
            let value = self.resolve_path(&path, context)?;

            // Bind value to first param name (or the binding expression name)
            let bind_name = params.first().map(|s| s.as_str()).unwrap_or(binding_expr);
            context.child_with(bind_name, value)
        } else {
            context.child()
        };

        self.render_nodes(&body, &child_ctx, source_file, indent)
    }

    /// Evaluates a `%let`/`%set` expression.
    ///
    /// Supports:
    /// - String literals: `"hello"` → String
    /// - Integer literals: `42` → Int
    /// - Float literals: `3.14` → Float
    /// - Boolean literals: `true`/`false` → Bool
    /// - Property paths: `struct.name` → resolved value
    fn eval_let_expr(&self, expr: &str, context: &TemplateContext) -> Result<ContextValue, String> {
        let expr = expr.trim();

        // String literal
        if expr.starts_with('"') && expr.ends_with('"') && expr.len() >= 2 {
            return Ok(ContextValue::String(expr[1..expr.len() - 1].to_string()));
        }

        // Boolean literals
        if expr == "true" {
            return Ok(ContextValue::Bool(true));
        }
        if expr == "false" {
            return Ok(ContextValue::Bool(false));
        }

        // Integer literal
        if let Ok(i) = expr.parse::<i64>() {
            return Ok(ContextValue::Int(i));
        }

        // Float literal
        if let Ok(f) = expr.parse::<f64>() {
            return Ok(ContextValue::Float(f));
        }

        // Property path
        let path: Vec<String> = expr.split('.').map(|s| s.trim().to_string()).collect();
        self.resolve_path(&path, context)
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

    /// Maps a non-wrapped TypeRef to its target language base type.
    /// Uses TOML type_map for primitives, and applies non_primitive format for others.
    fn map_base_type(&self, type_ref: &crate::ir_model::TypeRef) -> String {
        self.config
            .type_map
            .get(&type_ref.type_name)
            .cloned()
            .unwrap_or_else(|| {
                if !type_ref.is_primitive {
                    // For non-primitive types, use the `original` field from IR
                    // which mirrors how Rhai templates resolve types:
                    // - Same-namespace types: "ItemType" (no dot, no prefix)
                    // - Cross-namespace types: "game.common.StatBlock" (has dot, gets prefix)
                    let raw_name = &type_ref.original;
                    if raw_name.contains('.') {
                        if let Some(ref fmt) = self.config.type_map_non_primitive {
                            fmt.replace("{{type}}", raw_name)
                        } else {
                            raw_name.clone()
                        }
                    } else {
                        raw_name.clone()
                    }
                } else {
                    type_ref.type_name.clone()
                }
            })
    }

    /// `lang_type` filter: maps a poly type to the target language type.
    fn apply_lang_type_filter(&self, value: &ContextValue) -> Result<String, String> {
        match value {
            ContextValue::TypeRef(type_ref) => {
                // For Option/List types, delegate to inner_type for the base mapping,
                // then wrap with the appropriate format. This avoids double-wrapping
                // since `original` already includes the wrapper (e.g., "List<Position>").
                if type_ref.is_option {
                    let inner_base = if let Some(inner) = &type_ref.inner_type {
                        self.apply_lang_type_filter(&ContextValue::TypeRef(*inner.clone()))?
                    } else {
                        self.map_base_type(type_ref)
                    };
                    if let Some(ref fmt) = self.config.type_map_optional {
                        Ok(fmt.replace("{{type}}", &inner_base))
                    } else {
                        Ok(format!("{}?", inner_base))
                    }
                } else if type_ref.is_list {
                    let inner_base = if let Some(inner) = &type_ref.inner_type {
                        self.apply_lang_type_filter(&ContextValue::TypeRef(*inner.clone()))?
                    } else {
                        self.map_base_type(type_ref)
                    };
                    if let Some(ref fmt) = self.config.type_map_list {
                        Ok(fmt.replace("{{type}}", &inner_base))
                    } else {
                        Ok(format!("List<{}>", inner_base))
                    }
                } else {
                    Ok(self.map_base_type(type_ref))
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
            let type_name = type_ref
                .as_ref()
                .map(|t| t.type_name.as_str())
                .unwrap_or("Unknown");
            let ns_fqn = type_ref
                .as_ref()
                .map(|t| t.namespace_fqn.as_str())
                .unwrap_or("");
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
        ContextValue::Relation(_) => return "relation".to_string(),
        ContextValue::Index(_) => return "index".to_string(),
        ContextValue::IndexField(_) => return "index_field".to_string(),
        ContextValue::Annotation(_) => return "annotation".to_string(),
        ContextValue::ForeignKey(_) => return "foreign_key".to_string(),
        ContextValue::AnnotationParam(_) => return "param".to_string(),
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
        let source = "%if a\nA\n%elif b\nB\n%elif c\nC\n%else\nD\n%endif";
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
    fn test_render_let_string() {
        let source = "%let greeting = \"hello\"\n{{greeting}}";
        let templates = make_templates(&[("test.ptpl", source)]);
        let config = RenderConfig::default();
        let ctx = TemplateContext::new();

        let renderer = Renderer::new(&templates, &config);
        let result = renderer.render("test.ptpl", &ctx).unwrap();
        assert_eq!(result.lines, vec!["hello"]);
    }

    #[test]
    fn test_render_let_int() {
        let source = "%let count = 42\n{{count}}";
        let templates = make_templates(&[("test.ptpl", source)]);
        let config = RenderConfig::default();
        let ctx = TemplateContext::new();

        let renderer = Renderer::new(&templates, &config);
        let result = renderer.render("test.ptpl", &ctx).unwrap();
        assert_eq!(result.lines, vec!["42"]);
    }

    #[test]
    fn test_render_let_path() {
        let source = "%let n = name\n{{n}}";
        let templates = make_templates(&[("test.ptpl", source)]);
        let config = RenderConfig::default();
        let mut ctx = TemplateContext::new();
        ctx.set("name", ContextValue::String("World".to_string()));

        let renderer = Renderer::new(&templates, &config);
        let result = renderer.render("test.ptpl", &ctx).unwrap();
        assert_eq!(result.lines, vec!["World"]);
    }

    #[test]
    fn test_render_set_overwrite() {
        let source = "%let x = \"first\"\n{{x}}\n%set x = \"second\"\n{{x}}";
        let templates = make_templates(&[("test.ptpl", source)]);
        let config = RenderConfig::default();
        let ctx = TemplateContext::new();

        let renderer = Renderer::new(&templates, &config);
        let result = renderer.render("test.ptpl", &ctx).unwrap();
        assert_eq!(result.lines, vec!["first", "second"]);
    }

    #[test]
    fn test_render_let_in_condition() {
        let source = "%let flag = true\n%if flag\nyes\n%endif";
        let templates = make_templates(&[("test.ptpl", source)]);
        let config = RenderConfig::default();
        let ctx = TemplateContext::new();

        let renderer = Renderer::new(&templates, &config);
        let result = renderer.render("test.ptpl", &ctx).unwrap();
        assert_eq!(result.lines, vec!["yes"]);
    }

    #[test]
    fn test_render_for_where() {
        let source = "%for item in items | where item.is_active\n{{item.name}}\n%endfor";
        let templates = make_templates(&[("test.ptpl", source)]);
        let config = RenderConfig::default();
        let mut ctx = TemplateContext::new();

        let mut active = HashMap::new();
        active.insert("name".to_string(), ContextValue::String("A".to_string()));
        active.insert("is_active".to_string(), ContextValue::Bool(true));

        let mut inactive = HashMap::new();
        inactive.insert("name".to_string(), ContextValue::String("B".to_string()));
        inactive.insert("is_active".to_string(), ContextValue::Bool(false));

        let mut active2 = HashMap::new();
        active2.insert("name".to_string(), ContextValue::String("C".to_string()));
        active2.insert("is_active".to_string(), ContextValue::Bool(true));

        ctx.set(
            "items",
            ContextValue::List(vec![
                ContextValue::Map(active),
                ContextValue::Map(inactive),
                ContextValue::Map(active2),
            ]),
        );

        let renderer = Renderer::new(&templates, &config);
        let result = renderer.render("test.ptpl", &ctx).unwrap();
        assert_eq!(result.lines, vec!["A", "C"]);
    }

    #[test]
    fn test_render_include_isolates_file_bindings() {
        let main = "%let x = \"main\"\n%include \"child.ptpl\"\n{{x}}";
        let child = "%let x = \"child\"\n{{x}}";
        let templates = make_templates(&[("main.ptpl", main), ("child.ptpl", child)]);
        let config = RenderConfig::default();
        let ctx = TemplateContext::new();

        let renderer = Renderer::new(&templates, &config);
        let result = renderer.render("main.ptpl", &ctx).unwrap();
        // child sets x="child" but main's x should still be "main" after include
        assert_eq!(result.lines, vec!["child", "main"]);
    }

    #[test]
    fn test_render_logic_block() {
        let source = "%logic\nlet x = 42;\n%endlogic\n{{x}}";
        let templates = make_templates(&[("test.ptpl", source)]);
        let config = RenderConfig::default();
        let ctx = TemplateContext::new();

        let renderer = Renderer::new(&templates, &config);
        let result = renderer.render("test.ptpl", &ctx).unwrap();
        assert_eq!(result.lines, vec!["42"]);
    }

    #[test]
    fn test_render_logic_with_let() {
        let source = "%let base = 10\n%logic\nlet doubled = base * 2;\n%endlogic\n{{doubled}}";
        let templates = make_templates(&[("test.ptpl", source)]);
        let config = RenderConfig::default();
        let ctx = TemplateContext::new();

        let renderer = Renderer::new(&templates, &config);
        let result = renderer.render("test.ptpl", &ctx).unwrap();
        assert_eq!(result.lines, vec!["20"]);
    }

    #[test]
    fn test_render_logic_function() {
        let source =
            "%logic\nfn greet(name) { \"Hello \" + name }\nlet msg = greet(\"World\");\n%endlogic\n{{msg}}";
        let templates = make_templates(&[("test.ptpl", source)]);
        let config = RenderConfig::default();
        let ctx = TemplateContext::new();

        let renderer = Renderer::new(&templates, &config);
        let result = renderer.render("test.ptpl", &ctx).unwrap();
        assert_eq!(result.lines, vec!["Hello World"]);
    }

    #[test]
    fn test_render_logic_include_isolation() {
        let main = "%logic\nlet x = 1;\n%endlogic\n%include \"child.ptpl\"\n{{x}}";
        let child = "%logic\nlet x = 99;\n%endlogic\n{{x}}";
        let templates = make_templates(&[("main.ptpl", main), ("child.ptpl", child)]);
        let config = RenderConfig::default();
        let ctx = TemplateContext::new();

        let renderer = Renderer::new(&templates, &config);
        let result = renderer.render("main.ptpl", &ctx).unwrap();
        // child outputs 99, then main's x is still 1
        assert_eq!(result.lines, vec!["99", "1"]);
    }

    #[test]
    fn test_render_match_literal() {
        let source = "%let t = \"u32\"\n%match t\n%when \"u32\"\nuint\n%when \"string\"\nstr\n%else\nobject\n%endmatch";
        let templates = make_templates(&[("test.ptpl", source)]);
        let config = RenderConfig::default();
        let ctx = TemplateContext::new();

        let renderer = Renderer::new(&templates, &config);
        let result = renderer.render("test.ptpl", &ctx).unwrap();
        assert_eq!(result.lines, vec!["uint"]);
    }

    #[test]
    fn test_render_match_else() {
        let source = "%let t = \"bool\"\n%match t\n%when \"u32\"\nuint\n%else\nother\n%endmatch";
        let templates = make_templates(&[("test.ptpl", source)]);
        let config = RenderConfig::default();
        let ctx = TemplateContext::new();

        let renderer = Renderer::new(&templates, &config);
        let result = renderer.render("test.ptpl", &ctx).unwrap();
        assert_eq!(result.lines, vec!["other"]);
    }

    #[test]
    fn test_render_match_wildcard() {
        let source = "%let t = \"anything\"\n%match t\n%when _\ncaught\n%endmatch";
        let templates = make_templates(&[("test.ptpl", source)]);
        let config = RenderConfig::default();
        let ctx = TemplateContext::new();

        let renderer = Renderer::new(&templates, &config);
        let result = renderer.render("test.ptpl", &ctx).unwrap();
        assert_eq!(result.lines, vec!["caught"]);
    }

    #[test]
    fn test_render_block_and_render() {
        let source = "%block greet(name)\nHello {{name}}!\n%endblock\n%render greet with greeting";
        let templates = make_templates(&[("test.ptpl", source)]);
        let config = RenderConfig::default();
        let mut ctx = TemplateContext::new();
        ctx.set("greeting", ContextValue::String("World".to_string()));

        let renderer = Renderer::new(&templates, &config);
        let result = renderer.render("test.ptpl", &ctx).unwrap();
        assert_eq!(result.lines, vec!["Hello World!"]);
    }

    #[test]
    fn test_render_block_no_params() {
        let source = "%block header\n// Generated code\n%endblock\n%render header";
        let templates = make_templates(&[("test.ptpl", source)]);
        let config = RenderConfig::default();
        let ctx = TemplateContext::new();

        let renderer = Renderer::new(&templates, &config);
        let result = renderer.render("test.ptpl", &ctx).unwrap();
        assert_eq!(result.lines, vec!["// Generated code"]);
    }

    #[test]
    fn test_render_block_dynamic() {
        let source =
            "%block a\nA\n%endblock\n%block b\nB\n%endblock\n%let which = \"a\"\n%render $which";
        let templates = make_templates(&[("test.ptpl", source)]);
        let config = RenderConfig::default();
        let ctx = TemplateContext::new();

        let renderer = Renderer::new(&templates, &config);
        let result = renderer.render("test.ptpl", &ctx).unwrap();
        assert_eq!(result.lines, vec!["A"]);
    }

    #[test]
    fn test_render_logic_with_prelude() {
        let source = "%logic\nlet result = double(21);\n%endlogic\n{{result}}";
        let templates = make_templates(&[("test.ptpl", source)]);
        let mut config = RenderConfig::default();
        config.rhai_prelude = vec!["fn double(x) { x * 2 }".to_string()];
        let ctx = TemplateContext::new();

        let renderer = Renderer::new(&templates, &config);
        let result = renderer.render("test.ptpl", &ctx).unwrap();
        assert_eq!(result.lines, vec!["42"]);
    }

    #[test]
    fn test_render_logic_accesses_context_variables() {
        // %logic blocks should be able to access context variables (schema, file, etc.)
        let source = "%logic\nlet ctx_name = name;\n%endlogic\n{{ctx_name}}";
        let templates = make_templates(&[("test.ptpl", source)]);
        let config = RenderConfig::default();
        let mut ctx = TemplateContext::new();
        ctx.set("name", ContextValue::String("FromContext".to_string()));

        let renderer = Renderer::new(&templates, &config);
        let result = renderer.render("test.ptpl", &ctx).unwrap();
        assert_eq!(result.lines, vec!["FromContext"]);
    }

    #[test]
    fn test_render_logic_does_not_overwrite_context_variables() {
        // Variables from context should not be overwritten back to file_bindings
        let source = "%logic\nlet name = name + \"_modified\";\nlet new_var = 42;\n%endlogic\n{{name}} {{new_var}}";
        let templates = make_templates(&[("test.ptpl", source)]);
        let config = RenderConfig::default();
        let mut ctx = TemplateContext::new();
        ctx.set("name", ContextValue::String("Original".to_string()));

        let renderer = Renderer::new(&templates, &config);
        let result = renderer.render("test.ptpl", &ctx).unwrap();
        // name should remain "Original" (from context, not overwritten)
        // new_var should be accessible (new variable created in logic)
        assert_eq!(result.lines, vec!["Original 42"]);
    }

    #[test]
    fn test_render_source_map_tracking() {
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
