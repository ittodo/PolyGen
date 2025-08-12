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

/// Represents a namespace containing various definitions.
#[derive(Serialize, Debug, Default, Clone)]
pub struct NamespaceDef {
    pub name: String,
    pub items: Vec<NamespaceItem>,
}

/// An enum representing any item that can appear directly within a namespace.
#[derive(Serialize, Debug, Clone)]
pub enum NamespaceItem {
    Struct(StructDef),
    Enum(EnumDef),
    Comment(String),
}

// --- Struct Definition ---

/// Represents a struct or class definition.
#[derive(Serialize, Debug, Clone)]
pub struct StructDef {
    pub name: String,
    pub header: Vec<StructItem>,
    pub items: Vec<StructItem>,
    pub is_embed: bool,
    pub embedded_structs: Vec<StructDef>,
    pub inline_enums: Vec<EnumDef>,
}

/// An enum representing any item that can appear within a struct.
#[derive(Serialize, Debug, Clone)]
pub enum StructItem {
    Field(FieldDef),
    Comment(String),
    Annotation(AnnotationDef),
}

/// Represents a field/property within a struct.
#[derive(Serialize, Debug, Clone)]
pub struct FieldDef {
    pub name: String,
    pub field_type: String, // A simplified, language-agnostic type string like "List<Position>" or "Option<u32>"
    pub attributes: Vec<String>, // For annotations like [Key], [MaxLength(30)]
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
}