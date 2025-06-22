use serde::Deserialize;

/// 최상위 파일 구조를 나타냅니다.
#[derive(Debug, Deserialize)]
pub struct CSharpFile<'a> {
    #[serde(borrow)]
    pub using_directives: Vec<&'a str>,
    #[serde(borrow)]
    pub namespaces: Vec<NamespaceDef<'a>>,
}

/// 네임스페이스 정의를 나타냅니다.
#[derive(Debug, Deserialize)]
pub struct NamespaceDef<'a> {
    pub name: &'a str,
    #[serde(borrow)]
    pub types: Vec<TypeDef<'a>>,
}

/// 타입 정의(열거형, 구조체, 클래스)를 위한 열거형입니다.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TypeDef<'a> {
    Enum(#[serde(borrow)] EnumDef<'a>),
    Struct(#[serde(borrow)] StructDef<'a>),
    Class(#[serde(borrow)] ClassDef<'a>),
}

/// 타입이 공통으로 가지는 정보입니다.
#[derive(Debug, Deserialize)]
pub struct TypeInfo<'a> {
    pub name: &'a str,
    pub comment: Option<&'a str>,
}

/// 열거형(enum) 정의를 나타냅니다.
#[derive(Debug, Deserialize)]
pub struct EnumDef<'a> {
    #[serde(flatten)]
    #[serde(borrow)]
    pub info: TypeInfo<'a>,
    pub variants: Vec<&'a str>,
}

/// 구조체(struct) 정의를 나타냅니다.
#[derive(Debug, Deserialize)]
pub struct StructDef<'a> {
    #[serde(flatten)]
    #[serde(borrow)]
    pub info: TypeInfo<'a>,
    pub properties: Vec<PropertyDef<'a>>,
}

/// 클래스(class) 정의를 나타냅니다.
#[derive(Debug, Deserialize)]
pub struct ClassDef<'a> {
    #[serde(flatten)]
    #[serde(borrow)]
    pub info: TypeInfo<'a>,
    #[serde(default)]
    pub properties: Vec<PropertyDef<'a>>,
    #[serde(default)]
    pub nested_classes: Vec<ClassDef<'a>>,
}

/// 속성(property) 정의를 나타냅니다.
#[derive(Debug, Deserialize)]
pub struct PropertyDef<'a> {
    pub name: &'a str,
    pub type_name: &'a str,
    #[serde(default)]
    pub attributes: Vec<&'a str>,
}
