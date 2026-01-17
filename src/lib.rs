//! # PolyGen - Polyglot Code Generator
//!
//! PolyGen is a code generation tool that transforms custom schema definitions into
//! code for multiple target languages. It uses a pipeline approach:
//!
//! 1. **Parsing**: Schema files (`.poly`) are parsed into an Abstract Syntax Tree (AST)
//! 2. **Validation**: The AST is validated for correctness (duplicate types, undefined references)
//! 3. **IR Building**: The AST is transformed into an Intermediate Representation (IR)
//! 4. **Code Generation**: The IR is passed to Rhai templates to generate target language code
//!
//! ## Quick Start
//!
//! ```ignore
//! use polygen::{CompilationPipeline, PipelineConfig};
//!
//! let config = PipelineConfig::new(
//!     "schemas/main.poly",
//!     "templates",
//!     "output",
//! );
//! let pipeline = CompilationPipeline::new(config);
//! pipeline.run()?;
//! ```
//!
//! ## Modules
//!
//! - [`ast_model`]: AST types representing parsed schema definitions
//! - [`ast_parser`]: Parser for `.poly` schema files
//! - [`ir_model`]: Intermediate Representation types for template consumption
//! - [`ir_builder`]: Transforms AST to IR
//! - [`type_registry`]: Centralized type management for type resolution
//! - [`validation`]: AST validation (duplicate detection, type reference checking)
//! - [`pipeline`]: High-level compilation pipeline orchestration
//! - [`codegen`]: Code generation utilities
//! - [`rhai`] / [`rhai_generator`]: Rhai template engine integration

use anyhow::Result;
use clap::Parser as ClapParser;
use pest_derive::Parser;
use std::path::PathBuf;

// All modules are now part of the library
pub mod ast_model;
pub mod ast_parser;
pub mod codegen;
pub mod error;
pub mod ir_builder;
pub mod ir_model;
pub mod lang_config;
pub mod pipeline;
pub mod rhai;
pub mod rhai_generator;
pub mod type_registry;
pub mod validation;

// Re-exports for convenience
pub use crate::ast_model::AstRoot;
pub use crate::ir_model::SchemaContext;
pub use crate::pipeline::{parse_and_merge_schemas, CompilationPipeline, PipelineConfig};

#[derive(Parser)]
#[grammar = "polygen.pest"]
pub struct Polygen;

/// Polyglot Code Generator from a custom schema language
#[derive(ClapParser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Path to the root schema file
    #[arg(short, long)]
    pub schema_path: PathBuf,

    /// Path to the directory containing templates
    #[arg(short, long, default_value = "templates")]
    pub templates_dir: PathBuf,

    /// Path to the output directory for generated code
    #[arg(short, long, default_value = "output")]
    pub output_dir: PathBuf,

    /// Target language for code generation (e.g., csharp). If omitted, runs for all templates under --templates-dir.
    #[arg(short, long)]
    pub lang: Option<String>,
}

/// Run the code generation pipeline with the given CLI arguments
pub fn run(cli: Cli) -> Result<()> {
    let mut config = PipelineConfig::new(
        cli.schema_path,
        cli.templates_dir,
        cli.output_dir,
    );

    if let Some(lang) = cli.lang {
        config = config.with_language(lang);
    }

    let pipeline = CompilationPipeline::new(config);
    pipeline.run()
}

/// Build IR from ASTs (convenience function for backward compatibility)
pub fn build_ir_from_asts(asts: &[AstRoot]) -> SchemaContext {
    ir_builder::build_ir(asts)
}
