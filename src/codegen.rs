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
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::ir_model::SchemaContext;
use crate::lang_config::{LanguageConfig, StaticFileEntry};
use crate::rhai_generator;
use crate::template;

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
    /// Whether to enable preview mode (source marker injection).
    preview_mode: bool,
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
            preview_mode: false,
        }
    }

    /// Enables preview mode for source marker injection.
    pub fn with_preview_mode(mut self, enabled: bool) -> Self {
        self.preview_mode = enabled;
        self
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
    ///
    /// If the main template has a `.ptpl` extension, the new PolyTemplate engine
    /// is used instead of Rhai.
    pub fn generate(&self, ir_context: &SchemaContext) -> Result<()> {
        let main_template = self
            .config
            .as_ref()
            .map(|c| c.main_template(&self.language))
            .unwrap_or_else(|| format!("{}_file.rhai", self.language));

        if main_template.ends_with(".ptpl") {
            println!("Using PolyTemplate engine.");
            self.generate_with_polytemplate(ir_context, &main_template)?;
        } else {
            println!("Using Rhai template engine.");
            let template_path = self.template_dir().join(&main_template);
            let before = collect_output_files(&self.output_dir);
            rhai_generator::generate_code_with_rhai_opts(
                ir_context,
                &template_path,
                &self.output_dir,
                self.preview_mode,
            )?;
            let after = collect_output_files(&self.output_dir);
            record_manifest(&self.output_dir, &main_template, &before, &after);
        }

        Ok(())
    }

    /// Generates code using the PolyTemplate engine (`.ptpl` templates).
    fn generate_with_polytemplate(
        &self,
        ir_context: &SchemaContext,
        main_template: &str,
    ) -> Result<()> {
        // Build RenderConfig from language TOML
        let render_config = self.build_render_config();

        let engine_config = template::EngineConfig { render_config };

        // Load templates from the language template directory
        let engine = template::TemplateEngine::new(self.template_dir(), engine_config)
            .map_err(|e| anyhow::anyhow!("Failed to load .ptpl templates: {}", e))?;

        println!(
            "Loaded {} .ptpl templates from {}",
            engine.template_count(),
            self.template_dir().display()
        );

        // Determine file extension from config
        let extension = self
            .config
            .as_ref()
            .map(|c| c.extension.clone())
            .unwrap_or_default();

        // Load existing manifest for recording ptpl outputs
        let manifest_path = self.output_dir.join(MANIFEST_FILENAME);
        let mut manifest: HashMap<String, String> = if manifest_path.exists() {
            fs::read_to_string(&manifest_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            HashMap::new()
        };

        // Render each file in the schema
        for file in &ir_context.files {
            // Build output filename: "schemas/player.poly" → "player.go"
            let poly_path = Path::new(&file.path);
            let stem = poly_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("output");
            let output_filename = format!("{}{}", stem, extension);

            // Build context with file + extra variables
            let mut file_ctx = template::context::TemplateContext::with_file(file);
            file_ctx.set(
                "package_name",
                template::context::ContextValue::String(stem.to_lowercase()),
            );
            file_ctx.set(
                "source_path",
                template::context::ContextValue::String(file.path.clone()),
            );
            file_ctx.set(
                "schema",
                template::context::ContextValue::Schema(ir_context.clone()),
            );

            let result = engine
                .render(main_template, &file_ctx)
                .map_err(|e| anyhow::anyhow!("Template rendering error: {}", e))?;

            // Write output file
            let output_path = self.lang_output_dir().join(&output_filename);
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut content = result.lines.join("\n");
            if !content.ends_with('\n') {
                content.push('\n');
            }
            fs::write(&output_path, &content)?;
            println!("Generated: {}", output_path.display());

            // Record in manifest
            if let Ok(relative) = output_path.strip_prefix(&self.output_dir) {
                let key = relative.to_string_lossy().replace('\\', "/");
                manifest.insert(key, main_template.to_string());
            }

            // Write source map if in preview mode
            if self.preview_mode {
                // Append .ptpl.map to full filename (e.g. game_schema.go.ptpl.map)
                let map_filename = format!("{}.ptpl.map", output_path.display());
                let map_path = PathBuf::from(&map_filename);
                if let Ok(json) = result.source_map.to_json() {
                    fs::write(&map_path, json)?;
                }
            }
        }

        // Write manifest
        if let Ok(json) = serde_json::to_string_pretty(&manifest) {
            let _ = fs::write(&manifest_path, json);
        }

        Ok(())
    }

    /// Builds a `RenderConfig` from the language TOML configuration.
    fn build_render_config(&self) -> template::renderer::RenderConfig {
        let mut config = template::renderer::RenderConfig::default();

        if let Some(lang_config) = &self.config {
            config.type_map = lang_config.type_map.type_map();
            config.type_map_optional = lang_config.type_map.optional_format();
            config.type_map_list = lang_config.type_map.list_format();
            config.type_map_non_primitive = lang_config.type_map.non_primitive_format();

            config.binary_read = lang_config.binary_read.type_map();
            config.binary_read_option = lang_config.binary_read.sub_format("option");
            config.binary_read_list = lang_config.binary_read.sub_format("list");
            config.binary_read_enum = lang_config.binary_read.sub_format("enum");
            config.binary_read_struct = lang_config.binary_read.sub_format("struct");

            config.csv_read = lang_config.csv_read.type_map();
            config.csv_read_struct = lang_config.csv_read.sub_format("struct");

            // Load Rhai prelude scripts
            config.rhai_prelude = self.load_rhai_prelude(lang_config);
        }

        config
    }

    /// Loads Rhai prelude script contents from the language template directory.
    fn load_rhai_prelude(&self, lang_config: &LanguageConfig) -> Vec<String> {
        let mut scripts = Vec::new();
        for path in &lang_config.rhai.prelude {
            let full_path = self.template_dir().join(path);
            match fs::read_to_string(&full_path) {
                Ok(content) => scripts.push(content),
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to load Rhai prelude '{}': {}",
                        full_path.display(),
                        e
                    );
                }
            }
        }
        scripts
    }

    /// Generates code using a specific template file.
    ///
    /// Use this for additional templates beyond the main one (e.g., readers, writers).
    pub fn generate_with_template(
        &self,
        ir_context: &SchemaContext,
        template_name: &str,
    ) -> Result<()> {
        let template_path = self.template_dir().join(template_name);

        if template_path.exists() {
            let before = collect_output_files(&self.output_dir);
            rhai_generator::generate_code_with_rhai_opts(
                ir_context,
                &template_path,
                &self.output_dir,
                self.preview_mode,
            )?;
            let after = collect_output_files(&self.output_dir);
            record_manifest(&self.output_dir, template_name, &before, &after);
        }

        Ok(())
    }

    /// Generates code for all extra templates defined in the language config.
    ///
    /// Extra templates are processed after the main template.
    /// Supports both Rhai (`.rhai`) and PolyTemplate (`.ptpl`) extra templates.
    pub fn generate_extras(&self, ir_context: &SchemaContext) -> Result<()> {
        if let Some(config) = &self.config {
            for template_name in config.extra_templates() {
                if template_name.ends_with(".ptpl") {
                    println!("Processing extra ptpl template: {}", template_name);
                    self.generate_extra_with_polytemplate(ir_context, template_name)?;
                } else if self.has_template(template_name) {
                    println!("Processing extra template: {}", template_name);
                    self.generate_with_template(ir_context, template_name)?;
                }
            }
        }
        Ok(())
    }

    /// Generates code using an extra PolyTemplate (`.ptpl`) template.
    ///
    /// Derives the output filename suffix from the template name:
    /// e.g., `go_container_file.ptpl` → `_container` suffix.
    fn generate_extra_with_polytemplate(
        &self,
        ir_context: &SchemaContext,
        template_name: &str,
    ) -> Result<()> {
        let render_config = self.build_render_config();
        let engine_config = template::EngineConfig { render_config };

        let engine = template::TemplateEngine::new(self.template_dir(), engine_config)
            .map_err(|e| anyhow::anyhow!("Failed to load .ptpl templates: {}", e))?;

        let extension = self
            .config
            .as_ref()
            .map(|c| c.extension.clone())
            .unwrap_or_default();

        // Derive output suffix from template name
        let output_suffix = derive_output_suffix(template_name, &self.language);

        // Load existing manifest
        let manifest_path = self.output_dir.join(MANIFEST_FILENAME);
        let mut manifest: HashMap<String, String> = if manifest_path.exists() {
            fs::read_to_string(&manifest_path)
                .ok()
                .and_then(|s| serde_json::from_str(&s).ok())
                .unwrap_or_default()
        } else {
            HashMap::new()
        };

        for file in &ir_context.files {
            let poly_path = Path::new(&file.path);
            let stem = poly_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("output");
            let output_filename = format!("{}{}{}", stem, output_suffix, extension);

            // Build context
            let mut file_ctx = template::context::TemplateContext::with_file(file);
            file_ctx.set(
                "package_name",
                template::context::ContextValue::String(stem.to_lowercase()),
            );
            file_ctx.set(
                "source_path",
                template::context::ContextValue::String(file.path.clone()),
            );
            file_ctx.set(
                "schema",
                template::context::ContextValue::Schema(ir_context.clone()),
            );

            // Compute container_name: PascalCase(stem) + "Container"
            use heck::ToPascalCase;
            let container_name = format!("{}Container", stem.to_pascal_case());
            file_ctx.set(
                "container_name",
                template::context::ContextValue::String(container_name),
            );

            let result = engine
                .render(template_name, &file_ctx)
                .map_err(|e| anyhow::anyhow!("Template rendering error: {}", e))?;

            let output_path = self.lang_output_dir().join(&output_filename);
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut content = result.lines.join("\n");
            if !content.ends_with('\n') {
                content.push('\n');
            }
            fs::write(&output_path, &content)?;
            println!("Generated: {}", output_path.display());

            // Record in manifest
            if let Ok(relative) = output_path.strip_prefix(&self.output_dir) {
                let key = relative.to_string_lossy().replace('\\', "/");
                manifest.insert(key, template_name.to_string());
            }

            // Write source map if in preview mode
            if self.preview_mode {
                let map_filename = format!("{}.ptpl.map", output_path.display());
                let map_path = PathBuf::from(&map_filename);
                if let Ok(json) = result.source_map.to_json() {
                    fs::write(&map_path, json)?;
                }
            }
        }

        // Write manifest
        if let Ok(json) = serde_json::to_string_pretty(&manifest) {
            let _ = fs::write(&manifest_path, json);
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

/// Derives an output filename suffix from a template name.
///
/// Examples:
/// - `"go_container_file.ptpl"` with lang `"go"` → `"_container"`
/// - `"csharp_binary_readers_file.ptpl"` with lang `"csharp"` → `"_binary_readers"`
fn derive_output_suffix(template_name: &str, language: &str) -> String {
    let stem = Path::new(template_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(template_name);

    // Strip lang prefix: "go_container_file" → "container_file"
    let without_lang = stem.strip_prefix(&format!("{}_", language)).unwrap_or(stem);

    // Strip "_file" suffix: "container_file" → "container"
    let core = without_lang.strip_suffix("_file").unwrap_or(without_lang);

    // Use PascalCase with dot prefix to match Rhai naming:
    // "container" → ".Container", "binary_readers" → ".BinaryReaders"
    use heck::ToPascalCase;
    format!(".{}", core.to_pascal_case())
}

/// The manifest file name used to track which template generated each output file.
pub const MANIFEST_FILENAME: &str = ".polygen_manifest.json";

/// Recursively collect all file paths under a directory.
fn collect_output_files(dir: &Path) -> HashSet<PathBuf> {
    let mut files = HashSet::new();
    collect_files_recursive(dir, &mut files);
    files
}

fn collect_files_recursive(dir: &Path, files: &mut HashSet<PathBuf>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_files_recursive(&path, files);
            } else {
                files.insert(path);
            }
        }
    }
}

/// Record newly created files into the manifest, mapping them to the template that created them.
fn record_manifest(
    output_dir: &Path,
    template_name: &str,
    before: &HashSet<PathBuf>,
    after: &HashSet<PathBuf>,
) {
    let new_files: Vec<&PathBuf> = after.difference(before).collect();
    if new_files.is_empty() {
        return;
    }

    let manifest_path = output_dir.join(MANIFEST_FILENAME);

    // Load existing manifest or create new
    let mut manifest: HashMap<String, String> = if manifest_path.exists() {
        fs::read_to_string(&manifest_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        HashMap::new()
    };

    // Add new entries
    for file_path in new_files {
        if let Ok(relative) = file_path.strip_prefix(output_dir) {
            let key = relative.to_string_lossy().replace('\\', "/");
            // Skip manifest file itself and debug files
            if key == MANIFEST_FILENAME || key.starts_with("debug/") {
                continue;
            }
            manifest.insert(key, template_name.to_string());
        }
    }

    // Write manifest
    if let Ok(json) = serde_json::to_string_pretty(&manifest) {
        let _ = fs::write(&manifest_path, json);
    }
}

/// Load the template manifest from an output directory.
///
/// Returns a map of `relative_file_path -> template_name`.
pub fn load_manifest(output_dir: &Path) -> HashMap<String, String> {
    let manifest_path = output_dir.join(MANIFEST_FILENAME);
    if manifest_path.exists() {
        fs::read_to_string(&manifest_path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    } else {
        HashMap::new()
    }
}
