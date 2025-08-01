use serde::Serialize;
use std::collections::BTreeMap;

/// The root context object passed to the template engine.
/// Using BTreeMap to ensure deterministic output order.
#[derive(Serialize, Debug, Default)]
pub struct SchemaContext {
    pub namespaces: BTreeMap<String, NamespaceDef>,
}

/// Represents a namespace containing various type definitions.
#[derive(Serialize, Debug, Default)]
pub struct NamespaceDef {
    pub name: String,
    pub types: Vec<TypeDef>,
}

/// An enum representing any type definition (struct, enum).
#[derive(Serialize, Debug)]
pub enum TypeDef {
    Struct(StructDef),
    Enum(EnumDef),
}

/// Represents a field/property within a struct.
#[derive(Serialize, Debug)]
pub struct FieldDef {
    pub name: String,
    pub field_type: String, // A simplified, language-agnostic type string like "List<Position>" or "Option<u32>"
    pub comment: Option<String>,
    pub attributes: Vec<String>, // For annotations like [Key], [MaxLength(30)]
}

/// Represents a struct or class definition.
#[derive(Serialize, Debug)]
pub struct StructDef {
    pub name: String,
    pub comment: Option<String>,
    pub fields: Vec<FieldDef>,
    pub annotations: Vec<AnnotationDef>,
}

/// Represents a single annotation for the template.
#[derive(Serialize, Debug)]
pub struct AnnotationDef {
    pub name: String,
    pub params: BTreeMap<String, String>,
}


/// Represents a member of an enum.
#[derive(Serialize, Debug)]
pub struct EnumMember {
    pub name: String,
    pub comment: Option<String>,
}

/// Represents an enum definition.
#[derive(Serialize, Debug)]
pub struct EnumDef {
    pub name: String,
    pub comment: Option<String>,
    pub members: Vec<EnumMember>,
}