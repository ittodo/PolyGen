/// Mermaid 클래스 다이어그램 생성을 위한 데이터 모델입니다.
use crate::ast::Constraint;

#[derive(Debug, Default)]
pub struct ClassDiagram<'a> {
    pub classes: Vec<Class<'a>>,
    pub enums: Vec<Enum<'a>>,
    pub relationships: Vec<Relationship>, // Direct relationships (e.g., field references, FKs)
    pub foreign_keys_for_reverse_lookup: Vec<(String, &'a Constraint)>, // (owner_fqn, ForeignKeyConstraint)
}

#[derive(Debug)]
pub struct Class<'a> {
    pub fqn: String, // Fully Qualified Name (e.g., "game.character.Player")
    pub name: &'a str,
    pub properties: Vec<Property<'a>>,
    pub annotations: Vec<String>,
}

#[derive(Debug)]
pub struct Enum<'a> {
    pub fqn: String,
    pub name: &'a str,
    pub variants: Vec<&'a str>,
}

#[derive(Debug)]
pub struct Property<'a> {
    pub name: &'a str,
    pub type_name: String, // e.g., "string", "List<Position>"
}

#[derive(Debug)]
pub struct Relationship {
    pub from: String,
    pub from_cardinality: String, // e.g., "1", "0..1", "*"
    pub to: String,
    pub to_cardinality: String, // e.g., "1", "0..1", "*"
    pub link_type: String,      // e.g., "--", "-->", "<--"
    pub label: String,
}
