/// 최상위 파일 구조를 나타냅니다.
#[derive(Debug, Default)]
pub struct CSharpFile {
    pub using_directives: Vec<String>,
    pub namespaces: Vec<NamespaceDef>,
}

/// 네임스페이스 정의를 나타냅니다.
#[derive(Debug)]
pub struct NamespaceDef {
    pub name: String,
    pub types: Vec<TypeDef>,
    pub nested_namespaces: Vec<NamespaceDef>,
}

/// 타입 정의(열거형, 구조체, 클래스)를 위한 열거형입니다.
#[derive(Debug)]
pub enum TypeDef {
    Enum(EnumDef),
    Struct(StructDef),
    Class(ClassDef),
}

/// 타입이 공통으로 가지는 정보입니다.
#[derive(Debug)]
pub struct TypeInfo {
    pub name: String,
    pub comment: Option<String>,
}

/// 열거형(enum) 정의를 나타냅니다.
#[derive(Debug)]
pub struct EnumDef {
    pub info: TypeInfo,
    pub variants: Vec<String>,
}

/// 구조체(struct) 정의를 나타냅니다.
#[derive(Debug)]
pub struct StructDef {
    pub info: TypeInfo,
    pub properties: Vec<PropertyDef>,
}

/// 클래스(class) 정의를 나타냅니다.
#[derive(Debug)]
pub struct ClassDef {
    pub info: TypeInfo,
    pub properties: Vec<PropertyDef>,
    pub nested_classes: Vec<ClassDef>,
}

/// 속성(property) 정의를 나타냅니다.
#[derive(Debug)]
pub struct PropertyDef {
    pub name: String,
    pub type_name: String,
    pub attributes: Vec<String>,
    pub comment: Option<String>,
}
