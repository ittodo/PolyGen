use serde::Serialize;
use std::collections::BTreeMap;

/// The root context object passed to the template engine.
/// Using BTreeMap to ensure deterministic output order.
#[derive(Serialize, Debug, Default)]
pub struct SchemaContext {
    pub namespaces: BTreeMap<String, NamespaceDef>,
}

// --- Namespace Definition ---

/// Represents a namespace containing various definitions.
#[derive(Serialize, Debug, Default)]
pub struct NamespaceDef {
    pub name: String,
    pub items: Vec<NamespaceItem>,
}

/// An enum representing any item that can appear directly within a namespace.
#[derive(Serialize, Debug)]
pub enum NamespaceItem {
    Struct(StructDef),
    Enum(EnumDef),
    Comment(String),
}

// --- Struct Definition ---

/// Represents a struct or class definition.
#[derive(Serialize, Debug)]
pub struct StructDef {
    pub name: String,
    pub items: Vec<StructItem>,
}

/// An enum representing any item that can appear within a struct.
#[derive(Serialize, Debug)]
pub enum StructItem {
    Field(FieldDef),
    Comment(String),
    Annotation(AnnotationDef),
}

/// Represents a field/property within a struct.
#[derive(Serialize, Debug)]
pub struct FieldDef {
    pub name: String,
    pub field_type: String, // A simplified, language-agnostic type string like "List<Position>" or "Option<u32>"
    pub comment: Option<String>, // Comments attached to the end of a field line
    pub attributes: Vec<String>, // For annotations like [Key], [MaxLength(30)]
}

/// Represents a single annotation for the template.
#[derive(Serialize, Debug)]
pub struct AnnotationDef {
    pub name: String,
    pub params: BTreeMap<String, String>,
}


// --- Enum Definition ---

/// Represents an enum definition.
#[derive(Serialize, Debug)]
pub struct EnumDef {
    pub name: String,
    pub items: Vec<EnumItem>,
}

/// An enum representing any item that can appear within an enum.
#[derive(Serialize, Debug)]
pub enum EnumItem {
    Member(EnumMember),
    Comment(String),
}

/// Represents a member of an enum.
#[derive(Serialize, Debug)]
pub struct EnumMember {
    pub name: String,
    pub comment: Option<String>, // Comments attached to the end of a member line
}
