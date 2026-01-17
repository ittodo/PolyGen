//! Code generation module
//!
//! Handles code generation for different target languages using Rhai templates.

use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

use crate::ir_model::SchemaContext;
use crate::rhai_generator;

/// Configuration for static files to copy for a specific language
pub struct StaticFileConfig {
    pub source: PathBuf,
    pub dest_subdir: PathBuf,
    pub filename: String,
}

/// Code generator for a specific target language
pub struct CodeGenerator {
    /// Target language (e.g., "csharp")
    pub language: String,
    /// Directory containing templates
    pub templates_dir: PathBuf,
    /// Base output directory
    pub output_dir: PathBuf,
}

impl CodeGenerator {
    /// Create a new CodeGenerator for the specified language
    pub fn new(language: impl Into<String>, templates_dir: PathBuf, output_dir: PathBuf) -> Self {
        Self {
            language: language.into(),
            templates_dir,
            output_dir,
        }
    }

    /// Get the language-specific template directory
    pub fn template_dir(&self) -> PathBuf {
        self.templates_dir.join(&self.language)
    }

    /// Get the language-specific output directory
    pub fn lang_output_dir(&self) -> PathBuf {
        self.output_dir.join(&self.language)
    }

    /// Check if a template exists
    pub fn has_template(&self, template_name: &str) -> bool {
        self.template_dir().join(template_name).exists()
    }

    /// Generate code using the main template
    pub fn generate(&self, ir_context: &SchemaContext) -> Result<()> {
        let main_template = format!("{}_file.rhai", self.language);
        let template_path = self.template_dir().join(&main_template);

        println!("Using Rhai template engine.");
        rhai_generator::generate_code_with_rhai(ir_context, &template_path, &self.output_dir)?;

        Ok(())
    }

    /// Generate code using a specific template
    pub fn generate_with_template(&self, ir_context: &SchemaContext, template_name: &str) -> Result<()> {
        let template_path = self.template_dir().join(template_name);

        if template_path.exists() {
            rhai_generator::generate_code_with_rhai(ir_context, &template_path, &self.output_dir)?;
        }

        Ok(())
    }

    /// Copy static files for the language
    pub fn copy_static_files(&self, static_files: &[StaticFileConfig]) -> Result<()> {
        for config in static_files {
            let dest_dir = self.lang_output_dir().join(&config.dest_subdir);
            fs::create_dir_all(&dest_dir)?;

            let dest_path = dest_dir.join(&config.filename);
            if config.source.exists() {
                fs::copy(&config.source, &dest_path)?;
                println!("Copied static file to {}", dest_path.display());
            }
        }
        Ok(())
    }
}

/// Get the list of static files for C#
pub fn csharp_static_files() -> Vec<StaticFileConfig> {
    let common = PathBuf::from("Common");
    vec![
        StaticFileConfig {
            source: PathBuf::from("static/csharp/DataSource.cs"),
            dest_subdir: common.clone(),
            filename: "DataSource.cs".to_string(),
        },
        StaticFileConfig {
            source: PathBuf::from("static/csharp/BinaryUtils.cs"),
            dest_subdir: common.clone(),
            filename: "BinaryUtils.cs".to_string(),
        },
        StaticFileConfig {
            source: PathBuf::from("static/csharp/CsvUtils.cs"),
            dest_subdir: common.clone(),
            filename: "CsvUtils.cs".to_string(),
        },
        StaticFileConfig {
            source: PathBuf::from("static/csharp/JsonCsvConverter.cs"),
            dest_subdir: common.clone(),
            filename: "JsonCsvConverter.cs".to_string(),
        },
        StaticFileConfig {
            source: PathBuf::from("static/csharp/JsonUtils.cs"),
            dest_subdir: common.clone(),
            filename: "JsonUtils.cs".to_string(),
        },
        StaticFileConfig {
            source: PathBuf::from("static/csharp/PolygenAttributes.cs"),
            dest_subdir: common,
            filename: "PolygenAttributes.cs".to_string(),
        },
    ]
}

/// Discover available languages from templates directory
pub fn discover_languages(templates_dir: &Path) -> Vec<String> {
    let mut languages = Vec::new();

    if let Ok(entries) = fs::read_dir(templates_dir) {
        for entry in entries.flatten() {
            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                let name = entry.file_name().to_string_lossy().to_string();
                let main_template = entry.path().join(format!("{}_file.rhai", name));
                if main_template.exists() {
                    languages.push(name);
                }
            }
        }
    }

    languages
}
