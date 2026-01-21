//! PolyGen Runtime Support Library for Rust
//!
//! This module provides common utilities for data loading and container management.

use std::collections::HashMap;
use std::hash::Hash;
use std::io::{Read, Write, Result as IoResult};

// ============================================================================
// Data Container Traits
// ============================================================================

/// Trait for data rows that can be stored in a container.
pub trait DataRow: Clone {
    /// The primary key type for this row.
    type PrimaryKey: Eq + Hash + Clone;

    /// Returns the primary key of this row.
    fn primary_key(&self) -> Self::PrimaryKey;
}

/// A unique index that maps a key to a single row.
/// Used for primary_key and unique constraints.
#[derive(Debug, Clone)]
pub struct UniqueIndex<K, V>
where
    K: Eq + Hash,
{
    index: HashMap<K, V>,
}

impl<K, V> UniqueIndex<K, V>
where
    K: Eq + Hash,
{
    pub fn new() -> Self {
        Self {
            index: HashMap::new(),
        }
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        self.index.get(key)
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.index.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.index.len()
    }

    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.index.keys()
    }

    pub fn values(&self) -> impl Iterator<Item = &V> {
        self.index.values()
    }

    pub fn insert(&mut self, key: K, value: V) {
        self.index.insert(key, value);
    }

    pub fn clear(&mut self) {
        self.index.clear();
    }
}

impl<K, V> Default for UniqueIndex<K, V>
where
    K: Eq + Hash,
{
    fn default() -> Self {
        Self::new()
    }
}

/// A group index that maps a key to multiple row indices.
/// Used for foreign_key relationships and index constraints.
#[derive(Debug, Clone)]
pub struct GroupIndex<K>
where
    K: Eq + Hash,
{
    index: HashMap<K, Vec<usize>>,
}

impl<K> GroupIndex<K>
where
    K: Eq + Hash,
{
    pub fn new() -> Self {
        Self {
            index: HashMap::new(),
        }
    }

    pub fn get(&self, key: &K) -> &[usize] {
        self.index.get(key).map(|v| v.as_slice()).unwrap_or(&[])
    }

    pub fn contains_key(&self, key: &K) -> bool {
        self.index.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.index.len()
    }

    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    pub fn keys(&self) -> impl Iterator<Item = &K> {
        self.index.keys()
    }

    pub fn add(&mut self, key: K, row_index: usize) {
        self.index.entry(key).or_default().push(row_index);
    }

    pub fn clear(&mut self) {
        self.index.clear();
    }
}

impl<K> Default for GroupIndex<K>
where
    K: Eq + Hash,
{
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Binary I/O Utilities
// ============================================================================

/// Extension trait for reading PolyGen binary format.
pub trait BinaryReadExt: Read {
    fn read_u8(&mut self) -> IoResult<u8> {
        let mut buf = [0u8; 1];
        self.read_exact(&mut buf)?;
        Ok(buf[0])
    }

    fn read_u16(&mut self) -> IoResult<u16> {
        let mut buf = [0u8; 2];
        self.read_exact(&mut buf)?;
        Ok(u16::from_le_bytes(buf))
    }

    fn read_u32(&mut self) -> IoResult<u32> {
        let mut buf = [0u8; 4];
        self.read_exact(&mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }

    fn read_u64(&mut self) -> IoResult<u64> {
        let mut buf = [0u8; 8];
        self.read_exact(&mut buf)?;
        Ok(u64::from_le_bytes(buf))
    }

    fn read_i8(&mut self) -> IoResult<i8> {
        let mut buf = [0u8; 1];
        self.read_exact(&mut buf)?;
        Ok(buf[0] as i8)
    }

    fn read_i16(&mut self) -> IoResult<i16> {
        let mut buf = [0u8; 2];
        self.read_exact(&mut buf)?;
        Ok(i16::from_le_bytes(buf))
    }

    fn read_i32(&mut self) -> IoResult<i32> {
        let mut buf = [0u8; 4];
        self.read_exact(&mut buf)?;
        Ok(i32::from_le_bytes(buf))
    }

    fn read_i64(&mut self) -> IoResult<i64> {
        let mut buf = [0u8; 8];
        self.read_exact(&mut buf)?;
        Ok(i64::from_le_bytes(buf))
    }

    fn read_f32(&mut self) -> IoResult<f32> {
        let mut buf = [0u8; 4];
        self.read_exact(&mut buf)?;
        Ok(f32::from_le_bytes(buf))
    }

    fn read_f64(&mut self) -> IoResult<f64> {
        let mut buf = [0u8; 8];
        self.read_exact(&mut buf)?;
        Ok(f64::from_le_bytes(buf))
    }

    fn read_bool(&mut self) -> IoResult<bool> {
        Ok(self.read_u8()? != 0)
    }

    fn read_string(&mut self) -> IoResult<String> {
        let len = self.read_u32()? as usize;
        let mut buf = vec![0u8; len];
        self.read_exact(&mut buf)?;
        String::from_utf8(buf).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    fn read_bytes(&mut self) -> IoResult<Vec<u8>> {
        let len = self.read_u32()? as usize;
        let mut buf = vec![0u8; len];
        self.read_exact(&mut buf)?;
        Ok(buf)
    }

    fn read_optional<T, F>(&mut self, read_fn: F) -> IoResult<Option<T>>
    where
        F: FnOnce(&mut Self) -> IoResult<T>,
    {
        if self.read_bool()? {
            Ok(Some(read_fn(self)?))
        } else {
            Ok(None)
        }
    }

    fn read_vec<T, F>(&mut self, read_fn: F) -> IoResult<Vec<T>>
    where
        F: Fn(&mut Self) -> IoResult<T>,
    {
        let len = self.read_u32()? as usize;
        let mut vec = Vec::with_capacity(len);
        for _ in 0..len {
            vec.push(read_fn(self)?);
        }
        Ok(vec)
    }
}

impl<R: Read> BinaryReadExt for R {}

/// Extension trait for writing PolyGen binary format.
pub trait BinaryWriteExt: Write {
    fn write_u8(&mut self, value: u8) -> IoResult<()> {
        self.write_all(&[value])
    }

    fn write_u16(&mut self, value: u16) -> IoResult<()> {
        self.write_all(&value.to_le_bytes())
    }

    fn write_u32(&mut self, value: u32) -> IoResult<()> {
        self.write_all(&value.to_le_bytes())
    }

    fn write_u64(&mut self, value: u64) -> IoResult<()> {
        self.write_all(&value.to_le_bytes())
    }

    fn write_i8(&mut self, value: i8) -> IoResult<()> {
        self.write_all(&[value as u8])
    }

    fn write_i16(&mut self, value: i16) -> IoResult<()> {
        self.write_all(&value.to_le_bytes())
    }

    fn write_i32(&mut self, value: i32) -> IoResult<()> {
        self.write_all(&value.to_le_bytes())
    }

    fn write_i64(&mut self, value: i64) -> IoResult<()> {
        self.write_all(&value.to_le_bytes())
    }

    fn write_f32(&mut self, value: f32) -> IoResult<()> {
        self.write_all(&value.to_le_bytes())
    }

    fn write_f64(&mut self, value: f64) -> IoResult<()> {
        self.write_all(&value.to_le_bytes())
    }

    fn write_bool(&mut self, value: bool) -> IoResult<()> {
        self.write_u8(if value { 1 } else { 0 })
    }

    fn write_string(&mut self, value: &str) -> IoResult<()> {
        let bytes = value.as_bytes();
        self.write_u32(bytes.len() as u32)?;
        self.write_all(bytes)
    }

    fn write_bytes(&mut self, value: &[u8]) -> IoResult<()> {
        self.write_u32(value.len() as u32)?;
        self.write_all(value)
    }

    fn write_optional<T, F>(&mut self, value: &Option<T>, write_fn: F) -> IoResult<()>
    where
        F: FnOnce(&mut Self, &T) -> IoResult<()>,
    {
        match value {
            Some(v) => {
                self.write_bool(true)?;
                write_fn(self, v)
            }
            None => self.write_bool(false),
        }
    }

    fn write_vec<T, F>(&mut self, value: &[T], write_fn: F) -> IoResult<()>
    where
        F: Fn(&mut Self, &T) -> IoResult<()>,
    {
        self.write_u32(value.len() as u32)?;
        for item in value {
            write_fn(self, item)?;
        }
        Ok(())
    }
}

impl<W: Write> BinaryWriteExt for W {}

/// Trait for types that can be serialized to/from binary format.
/// Implement this for custom struct types.
pub trait BinaryIO: Sized {
    fn read_binary<R: Read>(reader: &mut R) -> IoResult<Self>;
    fn write_binary<W: Write>(&self, writer: &mut W) -> IoResult<()>;
}

// ============================================================================
// CSV Parsing Utilities
// ============================================================================

/// A parsed CSV row with header-based field access.
#[derive(Debug, Clone)]
pub struct CsvRow {
    headers: Vec<String>,
    values: Vec<String>,
}

impl CsvRow {
    pub fn new(headers: Vec<String>, values: Vec<String>) -> Self {
        Self { headers, values }
    }

    fn get_index(&self, field: &str) -> Option<usize> {
        self.headers.iter().position(|h| h == field)
    }

    pub fn get(&self, field: &str) -> Option<&str> {
        self.get_index(field).map(|i| self.values[i].as_str())
    }

    pub fn get_string(&self, field: &str) -> Option<String> {
        self.get(field).map(|s| s.to_string())
    }

    pub fn get_u8(&self, field: &str) -> Option<u8> {
        self.get(field).and_then(|s| s.parse().ok())
    }

    pub fn get_u16(&self, field: &str) -> Option<u16> {
        self.get(field).and_then(|s| s.parse().ok())
    }

    pub fn get_u32(&self, field: &str) -> Option<u32> {
        self.get(field).and_then(|s| s.parse().ok())
    }

    pub fn get_u64(&self, field: &str) -> Option<u64> {
        self.get(field).and_then(|s| s.parse().ok())
    }

    pub fn get_i8(&self, field: &str) -> Option<i8> {
        self.get(field).and_then(|s| s.parse().ok())
    }

    pub fn get_i16(&self, field: &str) -> Option<i16> {
        self.get(field).and_then(|s| s.parse().ok())
    }

    pub fn get_i32(&self, field: &str) -> Option<i32> {
        self.get(field).and_then(|s| s.parse().ok())
    }

    pub fn get_i64(&self, field: &str) -> Option<i64> {
        self.get(field).and_then(|s| s.parse().ok())
    }

    pub fn get_f32(&self, field: &str) -> Option<f32> {
        self.get(field).and_then(|s| s.parse().ok())
    }

    pub fn get_f64(&self, field: &str) -> Option<f64> {
        self.get(field).and_then(|s| s.parse().ok())
    }

    pub fn get_bool(&self, field: &str) -> Option<bool> {
        self.get(field).and_then(|s| match s.to_lowercase().as_str() {
            "true" | "1" | "yes" => Some(true),
            "false" | "0" | "no" => Some(false),
            _ => None,
        })
    }

    /// Get optional field - returns None if empty or missing
    pub fn get_optional(&self, field: &str) -> Option<&str> {
        self.get(field).filter(|s| !s.is_empty())
    }

    pub fn get_optional_string(&self, field: &str) -> Option<String> {
        self.get_optional(field).map(|s| s.to_string())
    }
}

/// Simple CSV reader that parses CSV content.
pub struct CsvReader {
    headers: Vec<String>,
    rows: Vec<Vec<String>>,
}

impl CsvReader {
    pub fn from_str(content: &str) -> Result<Self, String> {
        let mut lines = content.lines();

        let header_line = lines.next().ok_or("Empty CSV file")?;
        let headers: Vec<String> = Self::parse_line(header_line);

        let mut rows = Vec::new();
        for line in lines {
            if line.trim().is_empty() {
                continue;
            }
            rows.push(Self::parse_line(line));
        }

        Ok(Self { headers, rows })
    }

    pub fn from_file(path: &str) -> Result<Self, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read CSV file '{}': {}", path, e))?;
        Self::from_str(&content)
    }

    fn parse_line(line: &str) -> Vec<String> {
        let mut fields = Vec::new();
        let mut current = String::new();
        let mut in_quotes = false;

        for c in line.chars() {
            match c {
                '"' => in_quotes = !in_quotes,
                ',' if !in_quotes => {
                    fields.push(current.trim().to_string());
                    current = String::new();
                }
                _ => current.push(c),
            }
        }
        fields.push(current.trim().to_string());
        fields
    }

    pub fn rows(&self) -> impl Iterator<Item = CsvRow> + '_ {
        self.rows.iter().map(|values| CsvRow::new(self.headers.clone(), values.clone()))
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

// ============================================================================
// Error Types
// ============================================================================

/// Error type for data loading operations.
#[derive(Debug)]
pub enum LoadError {
    Io(std::io::Error),
    Parse(String),
    Json(String),
    Csv(String),
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadError::Io(e) => write!(f, "IO error: {}", e),
            LoadError::Parse(s) => write!(f, "Parse error: {}", s),
            LoadError::Json(s) => write!(f, "JSON error: {}", s),
            LoadError::Csv(s) => write!(f, "CSV error: {}", s),
        }
    }
}

impl std::error::Error for LoadError {}

impl From<std::io::Error> for LoadError {
    fn from(e: std::io::Error) -> Self {
        LoadError::Io(e)
    }
}

pub type LoadResult<T> = Result<T, LoadError>;
