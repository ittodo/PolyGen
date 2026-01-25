//! Intermediate Representation (IR) model for code generation.
//!
//! The IR is a template-friendly representation of the schema that can be easily
//! consumed by Rhai templates. All type references are fully resolved, and the
//! structure is optimized for code generation rather than parsing.
//!
//! # Key Differences from AST
//!
//! - All type references include resolved FQNs and type flags
//! - Nested types are flattened into their parent containers
//! - Metadata is pre-processed into template-friendly formats
//! - All structures implement `Serialize` for Rhai integration

use serde::Serialize;

/// The root context object passed to the template engine.
///
/// Contains all parsed and processed schema files with their namespaces and definitions.
#[derive(Serialize, Debug, Default, Clone)]
pub struct SchemaContext {
    /// All schema files that were processed.
    pub files: Vec<FileDef>,
}

// --- File Definition ---

/// Represents a single source schema file and its contents.
#[derive(Serialize, Debug, Default, Clone)]
pub struct FileDef {
    /// The file name (without directory path).
    pub path: String,
    /// Top-level namespaces defined in this file.
    pub namespaces: Vec<NamespaceDef>,
    /// Rename rules for migration (imported from .renames files).
    pub renames: Vec<RenameInfo>,
}

/// Represents a rename operation for schema migration.
///
/// Used to track table and field renames between schema versions.
#[derive(Serialize, Debug, Clone)]
pub struct RenameInfo {
    /// Type of rename operation.
    pub kind: RenameKind,
    /// Source path segments (e.g., ["Player"] or ["User", "user_name"]).
    pub from_path: Vec<String>,
    /// New name after the rename.
    pub to_name: String,
}

/// The type of rename operation.
#[derive(Serialize, Debug, Clone, PartialEq)]
pub enum RenameKind {
    /// Table/struct rename (e.g., Player -> User).
    Table,
    /// Field rename within a table (e.g., User.user_name -> name).
    Field,
}

// --- Namespace Definition ---

/// Represents a namespace containing various definitions.
///
/// Namespaces can be nested and contain structs, enums, and other namespaces.
#[derive(Serialize, Debug, Default, Clone)]
pub struct NamespaceDef {
    /// The fully-qualified namespace name (e.g., "game.character").
    pub name: String,
    /// Data source for this namespace (inherited by structs unless overridden).
    /// Set via `@datasource("name")` annotation.
    pub datasource: Option<String>,
    /// Items within this namespace, in declaration order.
    pub items: Vec<NamespaceItem>,
}

/// An item that can appear directly within a namespace.
#[derive(Serialize, Debug, Clone)]
pub enum NamespaceItem {
    /// A struct (table) definition.
    Struct(StructDef),
    /// An enum definition.
    Enum(EnumDef),
    /// A comment.
    Comment(String),
    /// A nested namespace.
    Namespace(Box<NamespaceDef>),
}

// --- Struct Definition ---

/// Represents a struct (class) definition.
///
/// Structs contain fields, nested types, and metadata. The `header` field
/// contains annotations and comments that apply to the struct itself.
#[derive(Serialize, Debug, Clone)]
pub struct StructDef {
    /// The simple name of the struct.
    pub name: String,
    /// Fully qualified name (e.g., "game.common.StatBlock").
    pub fqn: String,
    /// Whether this struct is an embed (reusable field group) rather than a table.
    pub is_embed: bool,
    /// Data source for this struct. Set via `@datasource("name")` annotation,
    /// or inherited from the parent namespace.
    pub datasource: Option<String>,
    /// Cache strategy for this struct. Set via `@cache("strategy")` annotation.
    /// Possible values: "full_load", "on_demand", "write_through", "write_back".
    pub cache_strategy: Option<String>,
    /// Whether this struct is read-only. Set via `@readonly` annotation.
    pub is_readonly: bool,
    /// Field name for soft delete. Set via `@soft_delete("field_name")` annotation.
    /// When set, deletes are logical (update field) rather than physical (DELETE).
    pub soft_delete_field: Option<String>,
    /// Pack separator for embed types. Set via `@pack` or `@pack(separator: ",")` annotation.
    /// When set, the embed's fields are serialized to/from a single string using this separator.
    /// Default separator is ";" if `@pack` is used without arguments.
    /// Only applicable when `is_embed` is true.
    pub pack_separator: Option<String>,
    /// Header items (struct-level annotations and comments).
    pub header: Vec<StructItem>,
    /// Body items (fields, nested types, inline comments).
    pub items: Vec<StructItem>,
    /// Indexes defined on this struct (from primary_key, unique, index, foreign_key).
    pub indexes: Vec<IndexDef>,
    /// Reverse relations pointing to this struct (from foreign_key ... as).
    pub relations: Vec<RelationDef>,
}

/// Index definition for a struct.
///
/// Supports both single-field and composite indexes.
#[derive(Serialize, Debug, Clone)]
pub struct IndexDef {
    /// Index name (e.g., "ById", "ByNameLevel", "ByPlayerId").
    pub name: String,
    /// Fields that make up this index (1 for single, 2+ for composite).
    pub fields: Vec<IndexFieldDef>,
    /// Whether this is a unique index (single result) or group index (list result).
    pub is_unique: bool,
    /// Source of this index: "constraint" (from primary_key/unique/index/foreign_key)
    /// or "annotation" (from @index).
    pub source: String,
}

impl IndexDef {
    /// Returns the first field name (for single-field index compatibility).
    pub fn field_name(&self) -> &str {
        self.fields.first().map(|f| f.name.as_str()).unwrap_or("")
    }

    /// Returns the first field type (for single-field index compatibility).
    pub fn field_type(&self) -> Option<&TypeRef> {
        self.fields.first().map(|f| &f.field_type)
    }

    /// Returns true if this is a composite index (2+ fields).
    pub fn is_composite(&self) -> bool {
        self.fields.len() > 1
    }

    /// Returns the number of fields in this index.
    pub fn field_count(&self) -> usize {
        self.fields.len()
    }
}

/// A field within an index definition.
#[derive(Serialize, Debug, Clone)]
pub struct IndexFieldDef {
    /// The field name.
    pub name: String,
    /// The field type.
    pub field_type: TypeRef,
}

/// Reverse relation definition (created from foreign_key ... as).
#[derive(Serialize, Debug, Clone)]
pub struct RelationDef {
    /// Relation name (from `as` keyword, e.g., "Items", "Skills").
    pub name: String,
    /// Fully qualified name of the source table that has the foreign key.
    pub source_table_fqn: String,
    /// Simple name of the source table.
    pub source_table_name: String,
    /// The field in the source table that holds the foreign key.
    pub source_field: String,
}

/// An item that can appear within a struct.
#[derive(Serialize, Debug, Clone)]
pub enum StructItem {
    /// A field definition.
    Field(FieldDef),
    /// A comment.
    Comment(String),
    /// An annotation.
    Annotation(AnnotationDef),
    /// An embedded struct (nested type).
    EmbeddedStruct(StructDef),
    /// An inline enum (nested enum type).
    InlineEnum(EnumDef),
}

/// A field/property within a struct.
#[derive(Serialize, Debug, Clone)]
pub struct FieldDef {
    /// The field name.
    pub name: String,
    /// The resolved type reference.
    pub field_type: TypeRef,
    /// Attributes derived from constraints (e.g., `["Key", "MaxLength(30)"]`).
    pub attributes: Vec<String>,
    /// Whether this field is a primary key.
    pub is_primary_key: bool,
    /// Whether this field has a unique constraint.
    pub is_unique: bool,
    /// Whether this field has an index constraint.
    pub is_index: bool,
    /// Foreign key reference information, if this field references another table.
    pub foreign_key: Option<ForeignKeyDef>,
    /// Maximum length constraint for string/bytes fields.
    pub max_length: Option<u32>,
    /// Default value as a string representation.
    pub default_value: Option<String>,
    /// Range constraint (min, max) for numeric fields.
    pub range: Option<RangeDef>,
    /// Regex pattern constraint for string fields.
    pub regex_pattern: Option<String>,
    /// Auto-create timestamp configuration (set on INSERT).
    /// If present, this field's value is automatically set to the current time on insert.
    pub auto_create: Option<TimezoneRef>,
    /// Auto-update timestamp configuration (set on UPDATE).
    /// If present, this field's value is automatically updated to the current time on update.
    pub auto_update: Option<TimezoneRef>,
}

/// Range constraint definition for numeric fields.
#[derive(Serialize, Debug, Clone)]
pub struct RangeDef {
    /// Minimum value as string representation.
    pub min: String,
    /// Maximum value as string representation.
    pub max: String,
    /// Type of the literal: "integer" or "float".
    pub literal_type: String,
}

/// Timezone reference for auto-timestamp fields.
///
/// Represents a timezone specification that can be:
/// - `utc`: UTC timezone
/// - `local`: System local timezone
/// - Offset: UTC offset like `+9`, `-5`, `+5:30`
/// - Named: Windows TimeZone ID like `"Korea Standard Time"`
#[derive(Serialize, Debug, Clone)]
pub struct TimezoneRef {
    /// The timezone kind: "utc", "local", "offset", or "named".
    pub kind: String,
    /// For offset: hours component (e.g., 9 for UTC+9, -5 for UTC-5).
    pub offset_hours: Option<i8>,
    /// For offset: minutes component (e.g., 30 for UTC+5:30).
    pub offset_minutes: Option<u8>,
    /// For named: the Windows TimeZone ID (e.g., "Korea Standard Time").
    pub name: Option<String>,
}

/// Foreign key reference definition.
#[derive(Serialize, Debug, Clone)]
pub struct ForeignKeyDef {
    /// Fully qualified name of the target table (e.g., "game.character.Player").
    pub target_table_fqn: String,
    /// Target field name (e.g., "id").
    pub target_field: String,
    /// Alias for reverse relation (from `as` keyword), if specified.
    /// This creates a navigation property on the target table.
    pub alias: Option<String>,
}

/// A fully-resolved type reference.
///
/// Contains all information needed for code generation, including the FQN,
/// namespace, type flags, and inner types for generic containers.
#[derive(Serialize, Debug, Clone)]
pub struct TypeRef {
    /// The original type name as written in the schema.
    pub original: String,
    /// Fully qualified name of the core type (e.g., "game.common.StatBlock").
    pub fqn: String,
    /// Namespace portion of the FQN (empty for primitives).
    pub namespace_fqn: String,
    /// Simple type name (last segment of FQN).
    pub type_name: String,
    /// Parent type path for nested types (e.g., "Monster.DropItems").
    /// Empty for top-level types.
    pub parent_type_path: String,
    /// Language-specific type representation.
    pub lang_type: String,
    /// Whether this is a built-in primitive type.
    pub is_primitive: bool,
    /// Whether this references a struct type.
    pub is_struct: bool,
    /// Whether this references an enum type.
    pub is_enum: bool,
    /// Whether this is an optional type (`?`).
    pub is_option: bool,
    /// Whether this is a list/array type (`[]`).
    pub is_list: bool,
    /// For container types (Option, List), the inner type.
    pub inner_type: Option<Box<TypeRef>>,
}

/// An annotation definition for templates.
///
/// Supports both positional arguments and named parameters:
/// - `@index(name, level)` → positional_args: ["name", "level"]
/// - `@load(csv: "data.csv")` → params: [{key: "csv", value: "data.csv"}]
/// - `@index(name, unique: true)` → both populated
#[derive(Serialize, Debug, Clone)]
pub struct AnnotationDef {
    /// The annotation name.
    pub name: String,
    /// Positional arguments (values only, as strings).
    pub positional_args: Vec<String>,
    /// Named parameters (key-value pairs).
    pub params: Vec<AnnotationParam>,
}

/// A key-value parameter in an annotation.
#[derive(Serialize, Debug, Clone)]
pub struct AnnotationParam {
    /// The parameter key.
    pub key: String,
    /// The parameter value (as a string).
    pub value: String,
}

// --- Enum Definition ---

/// Represents an enum definition.
#[derive(Serialize, Debug, Clone)]
pub struct EnumDef {
    /// The simple name of the enum.
    pub name: String,
    /// Fully qualified name (e.g., "game.common.Element").
    pub fqn: String,
    /// Enum items (members and comments).
    pub items: Vec<EnumItem>,
}

/// An item within an enum definition.
#[derive(Serialize, Debug, Clone)]
pub enum EnumItem {
    /// An enum member/variant.
    Member(EnumMember),
    /// A comment.
    Comment(String),
}

/// A member (variant) of an enum.
#[derive(Serialize, Debug, Clone)]
pub struct EnumMember {
    /// The member name.
    pub name: String,
    /// Optional explicit integer value.
    pub value: Option<i64>,
}
