//! PolyTemplate engine for `.ptpl` template files.
//!
//! This module provides a new template system that coexists with the existing Rhai
//! engine. Templates use `.ptpl` extension and a simplified directive-based syntax
//! designed for readability and line-level source mapping.
//!
//! # Template Syntax
//!
//! - Lines starting with `%` are directives (not emitted to output)
//! - All other lines are output lines with `{{expr | filter}}` interpolation
//! - Directives: `%if`, `%elif`, `%else`, `%endif`, `%for`, `%endfor`,
//!   `%include`, `%blank`, `%--` (comment)
//!
//! # Example
//!
//! ```text
//! %-- This is a comment
//! using System;
//! public class {{struct.name}}
//! {
//! %for field in struct.fields
//!     public {{field.field_type | lang_type}} {{field.name}};
//! %endfor
//! }
//! ```
//!
//! # Usage
//!
//! ```rust,ignore
//! use polygen::template::{TemplateEngine, EngineConfig};
//!
//! let engine = TemplateEngine::new("templates/csharp", EngineConfig::default())?;
//! let result = engine.render("file/main_file", &context)?;
//! println!("{}", result.to_string());
//! ```

pub mod context;
pub mod expr;
pub mod filters;
pub mod parser;
pub mod renderer;
pub mod source_map;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use context::TemplateContext;
use parser::ParsedTemplate;
use renderer::{RenderConfig, RenderResult, Renderer};

/// Configuration for the template engine.
#[derive(Debug, Clone, Default)]
pub struct EngineConfig {
    /// Rendering configuration (type maps, binary read maps, etc.).
    pub render_config: RenderConfig,
}

/// The PolyTemplate engine.
///
/// Loads `.ptpl` templates from a directory and renders them with a given context.
pub struct TemplateEngine {
    /// Loaded and parsed templates, keyed by relative path.
    templates: HashMap<String, ParsedTemplate>,
    /// Base directory for templates.
    base_dir: PathBuf,
    /// Engine configuration.
    config: EngineConfig,
}

impl TemplateEngine {
    /// Creates a new template engine, loading all `.ptpl` files from `template_dir`.
    pub fn new(template_dir: impl AsRef<Path>, config: EngineConfig) -> Result<Self, String> {
        let base_dir = template_dir.as_ref().to_path_buf();
        let templates = renderer::load_templates(&base_dir)?;

        Ok(Self {
            templates,
            base_dir,
            config,
        })
    }

    /// Returns the base directory for templates.
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Returns the number of loaded templates.
    pub fn template_count(&self) -> usize {
        self.templates.len()
    }

    /// Returns the names of all loaded templates.
    pub fn template_names(&self) -> Vec<&str> {
        self.templates.keys().map(|s| s.as_str()).collect()
    }

    /// Checks if a template exists.
    pub fn has_template(&self, name: &str) -> bool {
        let key = normalize_template_name(name);
        self.templates.contains_key(&key)
    }

    /// Renders a template by name with the given context.
    ///
    /// The template name can omit the `.ptpl` extension.
    pub fn render(
        &self,
        template_name: &str,
        context: &TemplateContext,
    ) -> Result<RenderResult, String> {
        let key = normalize_template_name(template_name);
        let renderer = Renderer::new(&self.templates, &self.config.render_config);
        renderer.render(&key, context)
    }

    /// Renders a template and returns the output as a single string.
    pub fn render_to_string(
        &self,
        template_name: &str,
        context: &TemplateContext,
    ) -> Result<String, String> {
        let result = self.render(template_name, context)?;
        Ok(result.lines.join("\n"))
    }

    /// Returns a reference to the loaded templates map.
    pub fn templates(&self) -> &HashMap<String, ParsedTemplate> {
        &self.templates
    }

    /// Updates the render configuration.
    pub fn set_render_config(&mut self, config: RenderConfig) {
        self.config.render_config = config;
    }
}

/// Normalizes a template name by adding `.ptpl` extension if missing.
fn normalize_template_name(name: &str) -> String {
    if name.ends_with(".ptpl") {
        name.to_string()
    } else {
        format!("{}.ptpl", name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use context::ContextValue;

    #[test]
    fn test_normalize_template_name() {
        assert_eq!(normalize_template_name("test"), "test.ptpl");
        assert_eq!(normalize_template_name("test.ptpl"), "test.ptpl");
        assert_eq!(
            normalize_template_name("section/class_body"),
            "section/class_body.ptpl"
        );
    }

    #[test]
    fn test_engine_render_inline() {
        // Create a temporary directory with a test template
        let temp_dir = tempfile::tempdir().unwrap();
        let template_path = temp_dir.path().join("test.ptpl");
        std::fs::write(&template_path, "Hello, {{name}}!").unwrap();

        let engine = TemplateEngine::new(temp_dir.path(), EngineConfig::default()).unwrap();
        assert_eq!(engine.template_count(), 1);
        assert!(engine.has_template("test"));
        assert!(engine.has_template("test.ptpl"));

        let mut ctx = TemplateContext::new();
        ctx.set("name", ContextValue::String("World".to_string()));

        let output = engine.render_to_string("test", &ctx).unwrap();
        assert_eq!(output, "Hello, World!");
    }

    #[test]
    fn test_engine_with_subdirectories() {
        let temp_dir = tempfile::tempdir().unwrap();

        // Create subdirectory structure
        let section_dir = temp_dir.path().join("section");
        std::fs::create_dir(&section_dir).unwrap();

        std::fs::write(
            temp_dir.path().join("main.ptpl"),
            "%include \"section/body.ptpl\"",
        )
        .unwrap();
        std::fs::write(section_dir.join("body.ptpl"), "Body content here").unwrap();

        let engine = TemplateEngine::new(temp_dir.path(), EngineConfig::default()).unwrap();
        assert_eq!(engine.template_count(), 2);

        let ctx = TemplateContext::new();
        let output = engine.render_to_string("main", &ctx).unwrap();
        assert_eq!(output, "Body content here");
    }
}
