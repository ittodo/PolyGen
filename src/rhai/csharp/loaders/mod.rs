//! C# Data Loader Code Generation
//!
//! This module provides code generation for C# data loaders that convert
//! raw data formats into class instances. Each loader type has its own submodule.
//!
//! ## Available Loaders
//!
//! - [`csv`]: CSV file loader generation
//!   - Header collection
//!   - Row reading (dictionary-based and indexed)
//!   - Row writing/appending
//!   - Dynamic methods for variable-length lists
//!
//! ## Planned Loaders
//!
//! - `json`: JSON file loader (TODO)
//! - `binary`: Binary reader/writer (TODO)
//!
//! ## Architecture
//!
//! Each loader generates C# code that follows a consistent pattern:
//!
//! ```text
//! [Struct]Csv class:
//!   - GetHeader() -> string[]
//!   - FromRow(row) -> Struct
//!   - FromRowWithPrefix(row, prefix) -> Struct
//!   - AppendRow(obj, cols)
//!   - ColumnCount() -> int
//! ```
//!
//! ---
//!
//! 이 모듈은 원시 데이터 형식을 클래스 인스턴스로 변환하는 C# 데이터 로더
//! 코드 생성을 제공합니다. 각 로더 유형은 자체 하위 모듈을 가집니다.

pub mod csv;

pub use csv::register_csv_loaders;
