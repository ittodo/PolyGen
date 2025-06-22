/// Mermaid 클래스 다이어그램 생성을 위한 데이터 모델입니다.
#[derive(Debug, Default)]
pub struct ClassDiagram<'a> {
    pub classes: Vec<Class<'a>>,
    pub enums: Vec<Enum<'a>>,
    pub relationships: Vec<Relationship>,
}

#[derive(Debug)]
pub struct Class<'a> {
    pub fqn: String, // Fully Qualified Name (e.g., "game.character.Player")
    pub name: &'a str,
    pub properties: Vec<Property<'a>>,
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
    pub to: String,
    pub link: String, // e.g., `*-- "1"`
    pub label: String,
}
