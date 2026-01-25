//! Abstract Syntax Tree (AST) model for PolyGen schema files.
//!
//! This module defines the data structures that represent parsed schema definitions.
//! The AST is produced by the parser and consumed by the IR builder.
//!
//! # Structure Hierarchy
//!
//! ```text
//! AstRoot
//! ├── file_imports: Vec<String>
//! └── definitions: Vec<Definition>
//!     ├── Namespace { path, imports, definitions }
//!     ├── Table { metadata, name, members }
//!     ├── Enum { metadata, name, variants }
//!     ├── Embed { metadata, name, members }
//!     ├── Comment(String)
//!     └── Annotation { name, params }
//! ```

use std::fmt;
use std::path::PathBuf;

/// The root node of a parsed schema file.
///
/// Contains the file path, any file-level imports, and all top-level definitions.
#[derive(Debug, PartialEq, Clone, Default)]
pub struct AstRoot {
    /// Path to the source schema file.
    pub path: PathBuf,
    /// File-level imports (e.g., `import "other.poly";`).
    pub file_imports: Vec<String>,
    /// Top-level definitions in the file.
    pub definitions: Vec<Definition>,
    /// Rename rules imported from .renames files.
    pub renames: Vec<RenameRule>,
}

/// A rename rule from a .renames file.
///
/// Represents a migration rename operation:
/// - Table rename: `Player -> User`
/// - Field rename: `User.user_name -> name`
#[derive(Debug, PartialEq, Clone)]
pub struct RenameRule {
    /// Source path (e.g., `["Player"]` or `["User", "user_name"]`).
    pub from_path: Vec<String>,
    /// Target name (the new name after rename).
    pub to_name: String,
}

/// A top-level definition in a schema file.
///
/// Definitions can be namespaces (containing other definitions), tables (struct-like),
/// enums, embeds (reusable field groups), comments, or annotations.
#[derive(Debug, PartialEq, Clone)]
pub enum Definition {
    /// A namespace block grouping related definitions.
    Namespace(Namespace),
    /// A table (struct) definition.
    Table(Table),
    /// An enum definition.
    Enum(Enum),
    /// An embed (reusable field group) definition.
    Embed(Embed),
    /// A standalone comment.
    Comment(String),
    /// A standalone annotation.
    Annotation(Annotation),
}

/// A namespace block containing definitions.
///
/// Namespaces provide logical grouping and affect the fully-qualified names of types.
/// Example: `namespace game.common { ... }`
#[derive(Debug, PartialEq, Clone)]
pub struct Namespace {
    /// Metadata attached to this namespace (doc comments, annotations like @datasource).
    pub metadata: Vec<Metadata>,
    /// The namespace path segments (e.g., `["game", "common"]`).
    pub path: Vec<String>,
    /// Import statements within the namespace.
    pub imports: Vec<NamespaceImport>,
    /// Definitions within this namespace.
    pub definitions: Vec<Definition>,
}

/// An import statement within a namespace.
///
/// Example: `import game.common.*;` or `import game.common.Type;`
#[derive(Debug, PartialEq, Clone)]
pub struct NamespaceImport {
    /// The import path segments.
    pub path: Vec<String>,
    /// Whether this is a wildcard import (`.*`).
    pub all: bool,
}

/// A table (struct-like) definition.
///
/// Tables define data structures with fields, nested types, and metadata.
#[derive(Debug, PartialEq, Clone)]
pub struct Table {
    /// Metadata attached to this table (doc comments, annotations).
    pub metadata: Vec<Metadata>,
    /// The table name.
    pub name: Option<String>,
    /// Members of the table (fields, nested embeds, nested enums, comments).
    pub members: Vec<TableMember>,
}

/// A member within a table definition.
#[derive(Debug, PartialEq, Clone)]
pub enum TableMember {
    /// A field definition.
    Field(FieldDefinition),
    /// A nested embed definition.
    Embed(Embed),
    /// A nested enum definition.
    Enum(Enum),
    /// A comment.
    Comment(String),
}

/// Metadata attached to a definition (doc comments or annotations).
#[derive(Debug, PartialEq, Clone)]
pub enum Metadata {
    /// A documentation comment (`/// ...`).
    DocComment(String),
    /// An annotation (`@name(params)`).
    Annotation(Annotation),
}

/// An annotation with optional arguments.
///
/// Supports both positional arguments and key-value parameters:
/// - `@load(csv: "data.csv")` - key-value only
/// - `@index(name, level)` - positional only
/// - `@index(name, level, unique: true)` - mixed
#[derive(Debug, PartialEq, Clone)]
pub struct Annotation {
    /// The annotation name.
    pub name: Option<String>,
    /// Arguments (positional or named).
    pub args: Vec<AnnotationArg>,
}

/// An argument in an annotation (either positional or named).
#[derive(Debug, PartialEq, Clone)]
pub enum AnnotationArg {
    /// A positional argument (just a value).
    Positional(Literal),
    /// A named/keyed argument (key: value).
    Named(AnnotationParam),
}

/// A key-value parameter in an annotation.
#[derive(Debug, PartialEq, Clone)]
pub struct AnnotationParam {
    /// The parameter key.
    pub key: String,
    /// The parameter value.
    pub value: Literal,
}

/// A field definition within a table.
///
/// Fields can be regular (typed), inline embeds (anonymous structs), or inline enums.
#[derive(Debug, PartialEq, Clone)]
pub enum FieldDefinition {
    /// A regular typed field.
    Regular(RegularField),
    /// An inline embed field (anonymous nested struct).
    InlineEmbed(InlineEmbedField),
    /// An inline enum field.
    InlineEnum(InlineEnumField),
}

/// A regular field with a type and optional constraints.
#[derive(Debug, PartialEq, Clone)]
pub struct RegularField {
    /// Metadata attached to this field.
    pub metadata: Vec<Metadata>,
    /// The field name.
    pub name: Option<String>,
    /// The field type with cardinality.
    pub field_type: TypeWithCardinality,
    /// Constraints on this field (primary key, unique, etc.).
    pub constraints: Vec<Constraint>,
    /// Optional field number for serialization ordering.
    pub field_number: Option<u32>,
}

/// A type with optional cardinality modifier.
#[derive(Debug, PartialEq, Clone)]
pub struct TypeWithCardinality {
    /// The base type.
    pub base_type: TypeName,
    /// Optional cardinality (optional `?` or array `[]`).
    pub cardinality: Option<Cardinality>,
}

/// The name/path of a type.
#[derive(Debug, PartialEq, Clone)]
pub enum TypeName {
    /// A path to a named type (e.g., `["game", "common", "Status"]`).
    Path(Vec<String>),
    /// A built-in basic type.
    Basic(BasicType),
    /// An inline enum definition.
    InlineEnum(Enum),
}

/// Cardinality modifier for a type.
#[derive(Debug, PartialEq, Clone)]
pub enum Cardinality {
    /// Optional type (`?`), may be null/absent.
    Optional,
    /// Array type (`[]`), a list of values.
    Array,
}

/// Built-in primitive types.
#[derive(Debug, PartialEq, Clone)]
pub enum BasicType {
    String,
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    Bool,
    Bytes,
    Timestamp,
}

/// Timezone specification for auto-timestamp fields.
///
/// Supports multiple formats:
/// - `utc`: UTC timezone
/// - `local`: System local timezone
/// - Offset: UTC offset like `+9`, `-5`, `+5:30`
/// - Named: Windows TimeZone ID like `"Korea Standard Time"`
#[derive(Debug, PartialEq, Clone)]
pub enum Timezone {
    /// UTC timezone.
    Utc,
    /// System local timezone.
    Local,
    /// UTC offset (hours, minutes). Example: (+9, 0) for UTC+9, (+5, 30) for UTC+5:30.
    Offset { hours: i8, minutes: u8 },
    /// Named timezone (Windows TimeZone ID). Example: "Korea Standard Time".
    Named(String),
}

/// A constraint on a field.
#[derive(Debug, PartialEq, Clone)]
pub enum Constraint {
    /// Primary key constraint.
    PrimaryKey,
    /// Unique constraint.
    Unique,
    /// Index constraint (creates a GroupIndex for lookup).
    Index,
    /// Maximum length constraint for strings/bytes.
    MaxLength(u32),
    /// Default value.
    Default(Literal),
    /// Value range constraint (min, max).
    Range(Literal, Literal),
    /// Regex pattern constraint for strings.
    Regex(String),
    /// Foreign key reference (path to target, optional alias).
    ForeignKey(Vec<String>, Option<String>),
    /// Auto-create timestamp (set on INSERT). None means use global default.
    AutoCreate(Option<Timezone>),
    /// Auto-update timestamp (set on UPDATE). None means use global default.
    AutoUpdate(Option<Timezone>),
}

/// An inline embed field (anonymous nested struct).
#[derive(Debug, PartialEq, Clone)]
pub struct InlineEmbedField {
    /// Metadata attached to this field.
    pub metadata: Vec<Metadata>,
    /// The field name.
    pub name: Option<String>,
    /// Members of the inline struct.
    pub members: Vec<TableMember>,
    /// Optional cardinality modifier.
    pub cardinality: Option<Cardinality>,
    /// Optional field number.
    pub field_number: Option<u32>,
}

/// An inline enum field.
#[derive(Debug, PartialEq, Clone)]
pub struct InlineEnumField {
    /// Metadata attached to this field.
    pub metadata: Vec<Metadata>,
    /// The field name.
    pub name: Option<String>,
    /// Enum variants.
    pub variants: Vec<EnumVariant>,
    /// Optional cardinality modifier.
    pub cardinality: Option<Cardinality>,
    /// Optional field number.
    pub field_number: Option<u32>,
}

/// A variant in an enum definition.
#[derive(Debug, PartialEq, Clone)]
pub struct EnumVariant {
    /// Metadata attached to this variant (doc comments, annotations).
    pub metadata: Vec<Metadata>,
    /// The variant name.
    pub name: Option<String>,
    /// Optional explicit integer value.
    pub value: Option<i64>,
    /// Optional inline comment (e.g., `Cash = 1; // 현금`).
    pub inline_comment: Option<String>,
}

/// An enum definition.
#[derive(Debug, PartialEq, Clone)]
pub struct Enum {
    /// Metadata attached to this enum.
    pub metadata: Vec<Metadata>,
    /// The enum name.
    pub name: Option<String>,
    /// The enum variants.
    pub variants: Vec<EnumVariant>,
}

/// An embed (reusable field group) definition.
///
/// Embeds are similar to tables but are meant to be embedded within other tables.
#[derive(Debug, PartialEq, Clone)]
pub struct Embed {
    /// Metadata attached to this embed.
    pub metadata: Vec<Metadata>,
    /// The embed name.
    pub name: Option<String>,
    /// Members of the embed.
    pub members: Vec<TableMember>,
}

/// A literal value used in annotations, defaults, and constraints.
#[derive(Debug, PartialEq, Clone)]
pub enum Literal {
    /// A string literal.
    String(String),
    /// An integer literal.
    Integer(i64),
    /// A floating-point literal.
    Float(f64),
    /// A boolean literal.
    Boolean(bool),
    /// An identifier (used for enum values in annotations).
    Identifier(String),
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Literal::String(s) => write!(f, "{}", s),
            Literal::Integer(i) => write!(f, "{}", i),
            Literal::Float(fl) => write!(f, "{}", fl),
            Literal::Boolean(b) => write!(f, "{}", b),
            Literal::Identifier(id) => write!(f, "{}", id),
        }
    }
}
