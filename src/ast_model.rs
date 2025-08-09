use std::fmt;
use std::path::PathBuf;

/// Represents the entire content of a single schema file.
#[derive(Debug, PartialEq, Clone, Default)]
pub struct AstRoot {
    pub path: PathBuf,
    pub file_imports: Vec<String>,
    pub definitions: Vec<Definition>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Definition {
    Namespace(Namespace),
    Table(Table),
    Enum(Enum),
    Embed(Embed),
    Comment(String),
    Annotation(Annotation),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Namespace {
    pub path: Vec<String>,
    pub imports: Vec<NamespaceImport>,
    pub definitions: Vec<Definition>,
}

/// Represents a `import game.common.*;` or `import game.common.Type;` statement.
#[derive(Debug, PartialEq, Clone)]
pub struct NamespaceImport {
    pub path: Vec<String>,
    pub all: bool, // true for `.*`
}

#[derive(Debug, PartialEq, Clone)]
pub struct Table {
    pub metadata: Vec<Metadata>,
    pub name: String,
    pub members: Vec<TableMember>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum TableMember {
    Field(FieldDefinition),
    Embed(Embed), // Named embed definition within a table
    Comment(String),
}

#[derive(Debug, PartialEq, Clone)]
pub enum Metadata {
    DocComment(String),
    Annotation(Annotation),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Annotation {
    pub name: String,
    pub params: Vec<AnnotationParam>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct AnnotationParam {
    pub key: String,
    pub value: Literal,
}

#[derive(Debug, PartialEq, Clone)]
pub enum FieldDefinition {
    Regular(RegularField),
    InlineEmbed(InlineEmbedField),
}

#[derive(Debug, PartialEq, Clone)]
pub struct RegularField {
    pub metadata: Vec<Metadata>,
    pub name: String,
    pub field_type: TypeWithCardinality,
    pub constraints: Vec<Constraint>,
    pub field_number: Option<u32>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct TypeWithCardinality {
    pub base_type: TypeName,
    pub cardinality: Option<Cardinality>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum TypeName {
    Path(Vec<String>),
    Basic(BasicType),
}

#[derive(Debug, PartialEq, Clone)]
pub enum Cardinality {
    Optional, // ?
    Array,    // []
}

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
}

#[derive(Debug, PartialEq, Clone)]
pub enum Constraint {
    PrimaryKey,
    Unique,
    MaxLength(u32),
    Default(Literal),
    Range(Literal, Literal),
    Regex(String),
    ForeignKey(Vec<String>, Option<String>), // path, optional 'as' identifier
}

#[derive(Debug, PartialEq, Clone)]
pub struct InlineEmbedField {
    pub metadata: Vec<Metadata>,
    pub name: String,
    pub fields: Vec<FieldDefinition>,
    pub cardinality: Option<Cardinality>,
    pub field_number: Option<u32>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct EnumVariant {
    pub metadata: Vec<Metadata>,
    pub name: String,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Enum {
    pub metadata: Vec<Metadata>,
    pub name: String,
    pub variants: Vec<EnumVariant>,
}

#[derive(Debug, PartialEq, Clone)]
pub struct Embed {
    pub metadata: Vec<Metadata>,
    pub name: String,
    pub fields: Vec<FieldDefinition>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Literal {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Identifier(String), // For annotation_param values that are identifiers
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