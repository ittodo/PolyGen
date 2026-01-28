//! Code generation module.
//!
//! This module provides the infrastructure for generating code in various target languages.
//! It uses Rhai templates to transform the IR into language-specific code.
//!
//! # Usage Example
//!
//! ```rust,ignore
//! use polygen::codegen::CodeGenerator;
//! use polygen::ir_model::SchemaContext;
//! use std::path::PathBuf;
//!
//! // Create a code generator for C#
//! let generator = CodeGenerator::new(
//!     "csharp",
//!     PathBuf::from("templates"),
//!     PathBuf::from("output"),
//! );
//!
//! // Generate code from the schema context
//! let schema_context: SchemaContext = /* ... */;
//! generator.generate(&schema_context)?;
//!
//! // Generate extra templates (CSV loaders, JSON mappers, etc.)
//! generator.generate_extras(&schema_context)?;
//!
//! // Copy static support files
//! generator.copy_configured_static_files(&PathBuf::from("."))?;
//! ```
//!
//! # Template Organization
//!
//! Templates are organized in directories by language:
//! ```text
//! templates/
//! ├── csharp/
//! │   ├── csharp.toml                # Language configuration
//! │   ├── csharp_file.rhai           # Main template
//! │   ├── csharp_binary_readers_file.rhai
//! │   └── ...
//! └── mysql/
//!     └── mysql_file.rhai
//! ```
//!
//! # Static Files
//!
//! Some languages require static support files (e.g., utility classes).
//! These are configured in the language's TOML file and copied from `static/<lang>/`.
//!
//! # Language Configuration
//!
//! Each language can have a `{lang}.toml` file that configures:
//! - File extension for generated files
//! - Static files to copy
//! - Main and extra templates

use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

use crate::ir_model::SchemaContext;
use crate::lang_config::{LanguageConfig, StaticFileEntry};
use crate::rhai_generator;

/// Configuration for a static file to be copied during code generation.
///
/// Static files are language-specific support files that are copied
/// verbatim to the output directory.
pub struct StaticFileConfig {
    /// Path to the source file (relative to project root).
    pub source: PathBuf,
    /// Subdirectory within the language output directory.
    pub dest_subdir: PathBuf,
    /// The filename in the destination.
    pub filename: String,
}

/// Code generator for a specific target language.
///
/// Handles template lookup, code generation, and static file copying
/// for a single target language.
pub struct CodeGenerator {
    /// Target language identifier (e.g., "csharp", "mysql").
    pub language: String,
    /// Root directory containing all language template directories.
    pub templates_dir: PathBuf,
    /// Base output directory for generated code.
    pub output_dir: PathBuf,
    /// Language configuration loaded from TOML file.
    config: Option<LanguageConfig>,
}

impl CodeGenerator {
    /// Creates a new CodeGenerator for the specified language.
    ///
    /// # Arguments
    ///
    /// * `language` - The target language identifier
    /// * `templates_dir` - Root directory containing template subdirectories
    /// * `output_dir` - Base directory for generated output
    pub fn new(language: impl Into<String>, templates_dir: PathBuf, output_dir: PathBuf) -> Self {
        let language = language.into();
        let config = LanguageConfig::load_for_language(&templates_dir, &language).ok();

        Self {
            language,
            templates_dir,
            output_dir,
            config,
        }
    }

    /// Returns the language configuration, if available.
    pub fn config(&self) -> Option<&LanguageConfig> {
        self.config.as_ref()
    }

    /// Returns the path to this language's template directory.
    pub fn template_dir(&self) -> PathBuf {
        self.templates_dir.join(&self.language)
    }

    /// Returns the path to this language's output directory.
    pub fn lang_output_dir(&self) -> PathBuf {
        self.output_dir.join(&self.language)
    }

    /// Checks if a template file exists for this language.
    pub fn has_template(&self, template_name: &str) -> bool {
        self.template_dir().join(template_name).exists()
    }

    /// Generates code using the main template for this language.
    ///
    /// The main template is determined by:
    /// 1. The `templates.main` setting in the language config
    /// 2. Fallback to `{language}_file.rhai`
    pub fn generate(&self, ir_context: &SchemaContext) -> Result<()> {
        let main_template = self
            .config
            .as_ref()
            .map(|c| c.main_template(&self.language))
            .unwrap_or_else(|| format!("{}_file.rhai", self.language));

        let template_path = self.template_dir().join(&main_template);

        println!("Using Rhai template engine.");
        rhai_generator::generate_code_with_rhai(ir_context, &template_path, &self.output_dir)?;

        Ok(())
    }

    /// Generates code using a specific template file.
    ///
    /// Use this for additional templates beyond the main one (e.g., readers, writers).
    pub fn generate_with_template(&self, ir_context: &SchemaContext, template_name: &str) -> Result<()> {
        let template_path = self.template_dir().join(template_name);

        if template_path.exists() {
            rhai_generator::generate_code_with_rhai(ir_context, &template_path, &self.output_dir)?;
        }

        Ok(())
    }

    /// Generates code for all extra templates defined in the language config.
    ///
    /// Extra templates are processed after the main template.
    pub fn generate_extras(&self, ir_context: &SchemaContext) -> Result<()> {
        if let Some(config) = &self.config {
            for template_name in config.extra_templates() {
                if self.has_template(template_name) {
                    println!("Processing extra template: {}", template_name);
                    self.generate_with_template(ir_context, template_name)?;
                }
            }
        }
        Ok(())
    }

    /// Copies static support files to the output directory.
    ///
    /// Static files are copied if they exist at the source path.
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

    /// Copies static files defined in the language configuration.
    ///
    /// # Arguments
    ///
    /// * `base_dir` - Base directory for resolving relative source paths
    pub fn copy_configured_static_files(&self, base_dir: &Path) -> Result<()> {
        if let Some(config) = &self.config {
            let entries = config.static_file_configs(base_dir);
            for entry in entries {
                self.copy_static_file_entry(&entry)?;
            }
        }
        Ok(())
    }

    /// Copies a single static file entry.
    fn copy_static_file_entry(&self, entry: &StaticFileEntry) -> Result<()> {
        let dest_dir = self.lang_output_dir().join(&entry.dest_subdir);
        fs::create_dir_all(&dest_dir)?;

        let dest_path = dest_dir.join(&entry.filename);
        if entry.source.exists() {
            fs::copy(&entry.source, &dest_path)?;
            println!("Copied static file to {}", dest_path.display());
        }
        Ok(())
    }
}

/// Returns the list of static files required for C# code generation.
///
/// These files provide utility classes for binary serialization, CSV parsing, etc.
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
            dest_subdir: common.clone(),
            filename: "PolygenAttributes.cs".to_string(),
        },
        StaticFileConfig {
            source: PathBuf::from("static/csharp/DataContainer.cs"),
            dest_subdir: common,
            filename: "DataContainer.cs".to_string(),
        },
    ]
}

/// Discovers available target languages from the templates directory.
///
/// A language is considered available if its template directory exists
/// and contains a main template file (`{lang}_file.rhai`).
///
/// # Arguments
///
/// * `templates_dir` - Root directory containing language template subdirectories
///
/// # Returns
///
/// A list of language identifiers (directory names) that have valid templates.
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
