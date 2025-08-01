use crate::ir_model::SchemaContext;
use anyhow::{Context, Result};
use minijinja::{path_loader, Environment, Error};
use once_cell::sync::Lazy;
use regex::Regex;
use std::fs;
use std::path::Path;

/// A generic code generator using the minijinja template engine.
pub struct Generator<'a> {
    env: Environment<'a>,
}

impl<'a> Generator<'a> {
    /// Creates a new generator and sets up the template environment.
    pub fn new(template_dir: &Path) -> Result<Self> {
        let mut env = Environment::new();
        env.set_loader(path_loader(template_dir));
        env.add_filter("map_type", map_type_filter);
        env.add_filter("pascal_case", |s: &str| -> Result<String, Error> {
            Ok(heck::ToPascalCase::to_pascal_case(s))
        });
        Ok(Self { env })
    }

    /// Generates code for a given language based on the IR context.
    pub fn generate(&self, context: &SchemaContext, lang: &str, output_dir: &Path) -> Result<()> {
        fs::create_dir_all(output_dir)
            .with_context(|| format!("Failed to create output directory: {:?}", output_dir))?;

        // Generate one file per namespace.
        for (ns_name, ns_def) in &context.namespaces {
            // Prepare the context for the template.
            // We pass the language so the template can use it in filters.
            let mut render_ctx = minijinja::context! {
                ns => ns_def,
                lang => lang,
            };

            let template_name = format!("{}_file.jinja", lang);
            let template = self.env.get_template(&template_name)?;

            let rendered_code = template.render(&mut render_ctx)?;

            let extension = match lang {
                "csharp" => "cs",
                "typescript" => "ts",
                _ => lang, // Fallback to using the language name as extension
            };

            let file_name = if ns_name.is_empty() {
                format!("GlobalTypes.{}", extension)
            } else {
                format!("{}.{}", ns_name.replace('.', "/"), extension)
            };

            let output_path = output_dir.join(file_name);
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&output_path, rendered_code)?;
            println!("Generated file: {}", output_path.display());
        }

        Ok(())
    }
}

// Use once_cell to compile regexes only once for performance and safety.
static RE_LIST: Lazy<Regex> = Lazy::new(|| Regex::new(r"List<(.+)>").unwrap());
static RE_OPTION: Lazy<Regex> = Lazy::new(|| Regex::new(r"Option<(.+)>").unwrap());

/// A custom minijinja filter to map language-agnostic types to language-specific types.
fn map_type_filter(poly_type: String, lang: String) -> Result<String, Error> {
    let base_type = if let Some(caps) = RE_LIST.captures(&poly_type) {
        let inner = map_type_filter(caps[1].to_string(), lang.clone())?;
        return Ok(match lang.as_str() {
            "csharp" => format!("System.Collections.Generic.List<{}>", inner),
            _ => format!("Array<{}>", inner),
        });
    } else if let Some(caps) = RE_OPTION.captures(&poly_type) {
        let inner = map_type_filter(caps[1].to_string(), lang.clone())?;
        return Ok(match lang.as_str() {
            "csharp" => format!("{}?", inner), // Only for value types, but simplified here.
            _ => format!("{} | null", inner),
        });
    } else {
        poly_type
    };

    let mapped_type = match lang.as_str() {
        "csharp" => match base_type.as_str() {
            "u32" => "uint".to_string(),
            "string" => "string".to_string(),
            "bool" => "bool".to_string(),
            // ... other basic types
            _ => base_type, // Custom types are assumed to be correct.
        },
        // Add other languages here
        // "typescript" => { ... }
        _ => base_type, // Default: return as-is
    };

    Ok(mapped_type)
}
