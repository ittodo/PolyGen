use serde::Serialize;

/// The root context object passed to the template engine.
#[derive(Serialize, Debug, Default, Clone)]
pub struct SchemaContext {
    pub files: Vec<FileDef>,
}

// --- File Definition ---

/// Represents a single source schema file and its contents.
#[derive(Serialize, Debug, Default, Clone)]
pub struct FileDef {
    pub path: String,
    pub namespaces: Vec<NamespaceDef>,
}

// --- Namespace Definition ---

/// Represents a namespace containing various definitions. Can be nested.
#[derive(Serialize, Debug, Default, Clone)]
pub struct NamespaceDef {
    /// The name of the namespace as declared in the schema (e.g., "game.character").
    pub name: String,
    /// A list of items, including structs, enums, and nested namespaces, in declaration order.
    pub items: Vec<NamespaceItem>,
}

/// An enum representing any item that can appear directly within a namespace.
#[derive(Serialize, Debug, Clone)]
pub enum NamespaceItem {
    Struct(StructDef),
    Enum(EnumDef),
    Comment(String),
    Namespace(Box<NamespaceDef>), // A nested namespace is now an item.
}

// --- Struct Definition ---

/// Represents a struct or class definition.
#[derive(Serialize, Debug, Clone)]
pub struct StructDef {
    pub name: String,
    pub fqn: String, // Fully Qualified Name, e.g., game.common.StatBlock
    pub header: Vec<StructItem>,
    pub items: Vec<StructItem>,
}

/// An enum representing any item that can appear within a struct.
#[derive(Serialize, Debug, Clone)]
pub enum StructItem {
    Field(FieldDef),
    Comment(String),
    Annotation(AnnotationDef),
    EmbeddedStruct(StructDef),
    InlineEnum(EnumDef),
}

/// Represents a field/property within a struct.
#[derive(Serialize, Debug, Clone)]
pub struct FieldDef {
    pub name: String,
    pub field_type: TypeRef, // Changed from String to a structured TypeRef
    pub attributes: Vec<String>, // For annotations like [Key], [MaxLength(30)]
}

/// Represents a reference to a type, containing all resolved information.
#[derive(Serialize, Debug, Clone)]
pub struct TypeRef {
    /// The original name used in the schema, e.g., "List<StatBlock>".
    pub original: String,
    /// The fully qualified name of the core type, e.g., "game.common.StatBlock".
    pub fqn: String,
    /// The language-specific representation, e.g., "List<Game.Common.StatBlock>".
    pub lang_type: String,
    pub is_primitive: bool,
    pub is_option: bool,
    pub is_list: bool,
    /// For List<T> or Option<T>, this refers to the inner type T.
    pub inner_type: Option<Box<TypeRef>>,
}


/// Represents a single annotation for the template.
#[derive(Serialize, Debug, Clone)]
pub struct AnnotationDef {
    pub name: String,
    pub params: Vec<AnnotationParam>,
}

/// Represents a key-value parameter for an annotation.
#[derive(Serialize, Debug, Clone)]
pub struct AnnotationParam {
    pub key: String,
    pub value: String,
}

// --- Enum Definition ---

/// Represents an enum definition.
#[derive(Serialize, Debug, Clone)]
pub struct EnumDef {
    pub name: String,
    pub fqn: String, // Fully Qualified Name, e.g., game.common.Element
    pub items: Vec<EnumItem>,
}

/// An enum representing any item that can appear within an enum.
#[derive(Serialize, Debug, Clone)]
pub enum EnumItem {
    Member(EnumMember),
    Comment(String),
}

/// Represents a member of an enum.
#[derive(Serialize, Debug, Clone)]
pub struct EnumMember {
    pub name: String,
    pub value: Option<i64>,
}