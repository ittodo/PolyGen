//! Language configuration module.
//!
//! This module provides configuration loading for target languages from TOML files.
//! It replaces hardcoded language settings with configurable definitions.
//!
//! # Configuration Format
//!
//! Each language has its own `{lang}.toml` file in the templates directory:
//!
//! ```toml
//! # templates/csharp/csharp.toml
//! extension = ".cs"
//!
//! [static_files]
//! "Common/DataSource.cs" = "static/csharp/DataSource.cs"
//! "Common/BinaryUtils.cs" = "static/csharp/BinaryUtils.cs"
//!
//! [templates]
//! main = "csharp_file.rhai"
//! extra = [
//!     "csharp_binary_readers_file.rhai",
//!     "csharp_binary_writers_file.rhai",
//! ]
//! ```

use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Configuration for a target language.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct LanguageConfig {
    /// File extension for generated files (e.g., ".cs", ".sql").
    #[serde(default)]
    pub extension: String,

    /// Static files to copy: destination path -> source path.
    #[serde(default)]
    pub static_files: HashMap<String, String>,

    /// Template configuration.
    #[serde(default)]
    pub templates: TemplateConfig,

    /// Type mapping: poly type name → language type name (e.g., "u32" → "uint").
    #[serde(default)]
    pub type_map: TypeMapConfig,

    /// Binary read expressions: poly type name → read expression.
    #[serde(default)]
    pub binary_read: RoleMappingConfig,

    /// Binary write expressions: poly type name → write expression.
    #[serde(default)]
    pub binary_write: RoleMappingConfig,

    /// CSV read expressions: poly type name → read expression.
    #[serde(default)]
    pub csv_read: RoleMappingConfig,

    /// CSV write expressions: poly type name → write expression.
    #[serde(default)]
    pub csv_write: RoleMappingConfig,
}

/// Type mapping configuration with optional/list format patterns.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct TypeMapConfig {
    /// Primitive type mappings: poly type → lang type.
    #[serde(flatten)]
    pub types: HashMap<String, toml::Value>,
}

impl TypeMapConfig {
    /// Returns the flat type → lang_type mapping (excluding sub-tables).
    pub fn type_map(&self) -> HashMap<String, String> {
        self.types
            .iter()
            .filter_map(|(k, v)| {
                if let toml::Value::String(s) = v {
                    Some((k.clone(), s.clone()))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Returns the optional type format (e.g., `"*{{type}}"`).
    pub fn optional_format(&self) -> Option<String> {
        self.types.get("optional").and_then(|v| {
            v.as_table()
                .and_then(|t| t.get("format"))
                .and_then(|f| f.as_str())
                .map(|s| s.to_string())
        })
    }

    /// Returns the list type format (e.g., `"[]{{type}}"`).
    pub fn list_format(&self) -> Option<String> {
        self.types.get("list").and_then(|v| {
            v.as_table()
                .and_then(|t| t.get("format"))
                .and_then(|f| f.as_str())
                .map(|s| s.to_string())
        })
    }
}

/// Role-based mapping configuration for binary/csv operations.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct RoleMappingConfig {
    /// Type → expression mappings and sub-table overrides.
    #[serde(flatten)]
    pub entries: HashMap<String, toml::Value>,
}

impl RoleMappingConfig {
    /// Returns the flat type → expression mapping (excluding sub-tables).
    pub fn type_map(&self) -> HashMap<String, String> {
        self.entries
            .iter()
            .filter_map(|(k, v)| {
                if let toml::Value::String(s) = v {
                    Some((k.clone(), s.clone()))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Returns the format string for a named sub-table (e.g., "option", "list", "enum", "struct").
    pub fn sub_format(&self, name: &str) -> Option<String> {
        self.entries.get(name).and_then(|v| {
            v.as_table()
                .and_then(|t| t.get("format"))
                .and_then(|f| f.as_str())
                .map(|s| s.to_string())
        })
    }
}

/// Template configuration for a language.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct TemplateConfig {
    /// Main template file name.
    #[serde(default)]
    pub main: Option<String>,

    /// Extra template files to run after the main template.
    #[serde(default)]
    pub extra: Vec<String>,
}

impl LanguageConfig {
    /// Loads language configuration from a TOML file.
    ///
    /// # Arguments
    ///
    /// * `config_path` - Path to the TOML configuration file
    ///
    /// # Returns
    ///
    /// The parsed configuration, or an error if loading/parsing fails.
    pub fn load(config_path: &Path) -> Result<Self, Box<ConfigError>> {
        let content = fs::read_to_string(config_path)
            .map_err(|e| Box::new(ConfigError::IoError(config_path.to_path_buf(), e)))?;

        toml::from_str(&content)
            .map_err(|e| Box::new(ConfigError::ParseError(config_path.to_path_buf(), e)))
    }

    /// Tries to load language configuration from the templates directory.
    ///
    /// Looks for `{lang}.toml` in the language's template directory.
    /// Returns default config if file doesn't exist.
    ///
    /// # Arguments
    ///
    /// * `templates_dir` - Root templates directory
    /// * `language` - Language identifier
    pub fn load_for_language(templates_dir: &Path, language: &str) -> Result<Self, Box<ConfigError>> {
        let config_path = templates_dir.join(language).join(format!("{}.toml", language));

        if config_path.exists() {
            Self::load(&config_path)
        } else {
            // Return default config with conventional main template name
            Ok(Self {
                extension: String::new(),
                static_files: HashMap::new(),
                templates: TemplateConfig {
                    main: Some(format!("{}_file.rhai", language)),
                    extra: Vec::new(),
                },
                type_map: TypeMapConfig::default(),
                binary_read: RoleMappingConfig::default(),
                binary_write: RoleMappingConfig::default(),
                csv_read: RoleMappingConfig::default(),
                csv_write: RoleMappingConfig::default(),
            })
        }
    }

    /// Returns the main template name, defaulting to `{lang}_file.rhai`.
    pub fn main_template(&self, language: &str) -> String {
        self.templates
            .main
            .clone()
            .unwrap_or_else(|| format!("{}_file.rhai", language))
    }

    /// Returns the list of extra templates to process.
    pub fn extra_templates(&self) -> &[String] {
        &self.templates.extra
    }

    /// Converts static_files config to StaticFileConfig list.
    ///
    /// # Arguments
    ///
    /// * `base_dir` - Base directory for resolving relative source paths
    pub fn static_file_configs(&self, base_dir: &Path) -> Vec<StaticFileEntry> {
        self.static_files
            .iter()
            .map(|(dest, src)| {
                let dest_path = PathBuf::from(dest);
                let (dest_subdir, filename) = if let Some(parent) = dest_path.parent() {
                    (
                        parent.to_path_buf(),
                        dest_path.file_name().unwrap().to_string_lossy().to_string(),
                    )
                } else {
                    (PathBuf::new(), dest.clone())
                };

                StaticFileEntry {
                    source: base_dir.join(src),
                    dest_subdir,
                    filename,
                }
            })
            .collect()
    }
}

/// Entry for a static file to be copied.
#[derive(Debug, Clone)]
pub struct StaticFileEntry {
    /// Source file path.
    pub source: PathBuf,
    /// Destination subdirectory within language output.
    pub dest_subdir: PathBuf,
    /// Destination filename.
    pub filename: String,
}

/// Errors that can occur during configuration loading.
#[derive(Debug)]
pub enum ConfigError {
    /// IO error reading the config file.
    IoError(PathBuf, std::io::Error),
    /// TOML parsing error.
    ParseError(PathBuf, toml::de::Error),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::IoError(path, e) => {
                write!(f, "Failed to read config file '{}': {}", path.display(), e)
            }
            ConfigError::ParseError(path, e) => {
                write!(f, "Failed to parse config file '{}': {}", path.display(), e)
            }
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigError::IoError(_, e) => Some(e),
            ConfigError::ParseError(_, e) => Some(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_config_file(dir: &Path, lang: &str, content: &str) -> PathBuf {
        let lang_dir = dir.join(lang);
        fs::create_dir_all(&lang_dir).unwrap();
        let config_path = lang_dir.join(format!("{}.toml", lang));
        let mut file = fs::File::create(&config_path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        config_path
    }

    #[test]
    fn test_load_basic_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_content = r#"
extension = ".cs"

[static_files]
"Common/Utils.cs" = "static/csharp/Utils.cs"

[templates]
main = "csharp_file.rhai"
extra = ["csharp_readers.rhai"]
"#;

        create_config_file(temp_dir.path(), "csharp", config_content);

        let config = LanguageConfig::load_for_language(temp_dir.path(), "csharp").unwrap();

        assert_eq!(config.extension, ".cs");
        assert_eq!(config.static_files.len(), 1);
        assert_eq!(
            config.static_files.get("Common/Utils.cs"),
            Some(&"static/csharp/Utils.cs".to_string())
        );
        assert_eq!(config.templates.main, Some("csharp_file.rhai".to_string()));
        assert_eq!(config.templates.extra, vec!["csharp_readers.rhai"]);
    }

    #[test]
    fn test_load_missing_config_returns_default() {
        let temp_dir = TempDir::new().unwrap();
        let lang_dir = temp_dir.path().join("rust");
        fs::create_dir_all(&lang_dir).unwrap();

        let config = LanguageConfig::load_for_language(temp_dir.path(), "rust").unwrap();

        assert_eq!(config.main_template("rust"), "rust_file.rhai");
        assert!(config.static_files.is_empty());
        assert!(config.templates.extra.is_empty());
    }

    #[test]
    fn test_main_template_default() {
        let config = LanguageConfig::default();
        assert_eq!(config.main_template("python"), "python_file.rhai");
    }

    #[test]
    fn test_static_file_configs() {
        let temp_dir = TempDir::new().unwrap();
        let config_content = r#"
[static_files]
"Common/Utils.cs" = "static/csharp/Utils.cs"
"Helpers/Helper.cs" = "static/csharp/Helper.cs"
"#;

        create_config_file(temp_dir.path(), "csharp", config_content);

        let config = LanguageConfig::load_for_language(temp_dir.path(), "csharp").unwrap();
        let entries = config.static_file_configs(temp_dir.path());

        assert_eq!(entries.len(), 2);

        // Find the Utils.cs entry
        let utils_entry = entries.iter().find(|e| e.filename == "Utils.cs").unwrap();
        assert_eq!(utils_entry.dest_subdir, PathBuf::from("Common"));
    }

    #[test]
    fn test_type_map_config() {
        let temp_dir = TempDir::new().unwrap();
        let config_content = r#"
extension = ".go"

[type_map]
u8 = "uint8"
u32 = "uint32"
string = "string"
bool = "bool"

[type_map.optional]
format = "*{{type}}"

[type_map.list]
format = "[]{{type}}"

[templates]
main = "go_file.ptpl"
"#;

        create_config_file(temp_dir.path(), "go", config_content);
        let config = LanguageConfig::load_for_language(temp_dir.path(), "go").unwrap();

        let tm = config.type_map.type_map();
        assert_eq!(tm.get("u32"), Some(&"uint32".to_string()));
        assert_eq!(tm.get("string"), Some(&"string".to_string()));
        assert_eq!(
            config.type_map.optional_format(),
            Some("*{{type}}".to_string())
        );
        assert_eq!(
            config.type_map.list_format(),
            Some("[]{{type}}".to_string())
        );
    }

    #[test]
    fn test_partial_config() {
        let temp_dir = TempDir::new().unwrap();
        // Only extension specified, rest should use defaults
        let config_content = r#"
extension = ".py"
"#;

        create_config_file(temp_dir.path(), "python", config_content);

        let config = LanguageConfig::load_for_language(temp_dir.path(), "python").unwrap();

        assert_eq!(config.extension, ".py");
        assert!(config.static_files.is_empty());
        assert_eq!(config.main_template("python"), "python_file.rhai");
    }
}
