//! Common Utilities for Rhai Code Generation
//!
//! This module provides shared utility functions used across different code generators.
//! These utilities are language-agnostic and can be used by any target language generator.
//!
//! ## Submodules
//!
//! - [`ir_lookup`]: Functions for searching and resolving types within the IR structure
//!
//! ## Usage
//!
//! ```ignore
//! use crate::rhai::common::{resolve_struct, resolve_enum, unwrap_option};
//!
//! // Find a struct by type string
//! if let Some(s) = resolve_struct(files, "MyStruct", "MyNamespace") {
//!     // Use the struct definition
//! }
//! ```
//!
//! ---
//!
//! 이 모듈은 여러 코드 생성기에서 공통으로 사용하는 유틸리티 함수들을 제공합니다.
//! 이 유틸리티들은 언어에 구애받지 않으며 어떤 대상 언어 생성기에서도 사용할 수 있습니다.

pub mod ir_lookup;

pub use ir_lookup::*;
