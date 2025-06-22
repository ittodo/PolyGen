use crate::csharp_model::*;
use askama::Template;

#[derive(Template)]
#[template(path = "csharp/main.cs.txt", escape = "none")]
struct CSharpTemplate<'a> {
    file: CSharpFile<'a>,
}

/// Askama 템플릿을 사용하여 C# 코드를 생성합니다.
pub fn generate_csharp_with_askama() -> String {
    let model = build_csharp_model();
    let template = CSharpTemplate { file: model };
    template.render().unwrap()
}

/// 생성할 C# 코드에 대한 전체 데이터 모델을 구축합니다.
/// 실제 프로젝트에서는 이 부분을 YAML/TOML 파일 파싱 로직으로 대체할 수 있습니다.
fn build_csharp_model<'a>() -> CSharpFile<'a> {
    CSharpFile {
        using_directives: vec!["System.Collections.Generic", "System.ComponentModel.DataAnnotations"],
        namespaces: vec![
            NamespaceDef {
                name: "game.common",
                types: vec![
                    TypeDef::Enum(EnumDef {
                        info: TypeInfo { name: "Element", comment: Some("스킬이나 공격의 속성을 나타내는 열거형입니다.") },
                        variants: vec!["PHYSICAL", "FIRE", "ICE", "LIGHTNING",],
                    }),
                    TypeDef::Struct(StructDef {
                        info: TypeInfo { name: "Position", comment: Some("게임 월드 내의 2D 좌표를 나타내는 복합 타입입니다.\n`embed`를 사용하여 여러 테이블에서 재사용할 수 있습니다.") },
                        properties: vec![PropertyDef { name: "X", type_name: "float", attributes: vec![] }, PropertyDef { name: "Y", type_name: "float", attributes: vec![] }],
                    }),
                    TypeDef::Struct(StructDef {
                        info: TypeInfo { name: "StatBlock", comment: Some("캐릭터의 기본 능력치를 묶은 구조체입니다.") },
                        properties: vec![
                            PropertyDef { name: "Health", type_name: "uint", attributes: vec![] },
                            PropertyDef { name: "Mana", type_name: "uint", attributes: vec![] },
                            PropertyDef { name: "Attack", type_name: "uint", attributes: vec![] },
                            PropertyDef { name: "Defense", type_name: "uint", attributes: vec![] },
                        ],
                    }),
                ],
            },
            NamespaceDef {
                name: "game.item",
                types: vec![
                    TypeDef::Enum(EnumDef {
                        info: TypeInfo { name: "ItemType", comment: Some("아이템의 종류를 나타내는 열거형입니다.") },
                        variants: vec!["WEAPON", "ARMOR", "POTION", "MATERIAL",],
                    }),
                    TypeDef::Class(ClassDef {
                        info: TypeInfo { name: "Item", comment: Some("아이템 정보를 정의하는 테이블입니다.") },
                        properties: vec![
                            PropertyDef { name: "Id", type_name: "uint", attributes: vec!["[Key]"] },
                            PropertyDef { name: "Name", type_name: "string", attributes: vec![] },
                            PropertyDef { name: "ItemType", type_name: "ItemType", attributes: vec![] },
                            PropertyDef { name: "Description", type_name: "string", attributes: vec![] },
                        ],
                        nested_classes: vec![],
                    }),
                ],
            },
            NamespaceDef {
                name: "game.character",
                types: vec![
                    TypeDef::Class(ClassDef {
                        info: TypeInfo { name: "Player", comment: Some("플레이어 캐릭터 정보를 정의하는 테이블입니다.") },
                        properties: vec![
                            PropertyDef { name: "Id", type_name: "uint", attributes: vec!["[Key]"] },
                            PropertyDef { name: "Name", type_name: "string", attributes: vec!["[MaxLength(30)]"] },
                            PropertyDef { name: "Level", type_name: "ushort", attributes: vec![] },
                            PropertyDef { name: "Stats", type_name: "StatBlock", attributes: vec![] },
                        ],
                        nested_classes: vec![],
                    }),
                    TypeDef::Class(ClassDef {
                        info: TypeInfo { name: "Monster", comment: Some("몬스터 정보를 정의하는 테이블입니다.\n모든 테이블은 `@taggable` 어노테이션을 통해 자유로운 태그를 붙일 수 있습니다.") },
                        properties: vec![
                            PropertyDef { name: "Id", type_name: "uint", attributes: vec!["[Key]"] },
                            PropertyDef { name: "Name", type_name: "string", attributes: vec![] },
                            PropertyDef { name: "Stats", type_name: "StatBlock", attributes: vec![] },
                            PropertyDef { name: "SpawnPoint", type_name: "Position", attributes: vec![] },
                            PropertyDef { name: "PatrolPoints", type_name: "List<Position>", attributes: vec![] },
                            PropertyDef { name: "DropItems", type_name: "List<DropItems>", attributes: vec![] },
                        ],
                        nested_classes: vec![ClassDef {
                            info: TypeInfo { name: "DropItems", comment: None },
                            properties: vec![
                                PropertyDef { name: "ItemId", type_name: "uint", attributes: vec![] },
                                PropertyDef { name: "DropChance", type_name: "float", attributes: vec![] },
                            ],
                            nested_classes: vec![],
                        }],
                    }),
                ],
            },
            NamespaceDef {
                name: "game.character.skill",
                types: vec![TypeDef::Class(ClassDef {
                    info: TypeInfo { name: "Skill", comment: Some("스킬 정보를 정의하는 테이블입니다.") },
                    properties: vec![
                        PropertyDef { name: "Id", type_name: "uint", attributes: vec!["[Key]"] },
                        PropertyDef { name: "Name", type_name: "string", attributes: vec![] },
                        PropertyDef { name: "Description", type_name: "string", attributes: vec![] },
                        PropertyDef { name: "Element", type_name: "Element", attributes: vec![] },
                        PropertyDef { name: "Power", type_name: "uint", attributes: vec![] },
                    ],
                    nested_classes: vec![],
                })],
            },
            NamespaceDef {
                name: "game.junction",
                types: vec![
                    TypeDef::Class(ClassDef {
                        info: TypeInfo { name: "PlayerSkill", comment: Some("플레이어와 스킬의 다대다(N:M) 관계를 위한 연결 테이블입니다.") },
                        properties: vec![PropertyDef { name: "PlayerId", type_name: "uint", attributes: vec![] }, PropertyDef { name: "SkillId", type_name: "uint", attributes: vec![] }, PropertyDef { name: "SkillLevel", type_name: "ushort", attributes: vec![] }],
                        nested_classes: vec![],
                    }),
                    TypeDef::Class(ClassDef {
                        info: TypeInfo { name: "InventoryItem", comment: Some("플레이어 인벤토리 항목을 나타내는 테이블입니다. (1:N 관계의 'N'쪽)") },
                        properties: vec![PropertyDef { name: "Id", type_name: "uint", attributes: vec!["[Key]"] }, PropertyDef { name: "PlayerId", type_name: "uint", attributes: vec![] }, PropertyDef { name: "ItemId", type_name: "uint", attributes: vec![] }, PropertyDef { name: "Quantity", type_name: "uint", attributes: vec![] }],
                        nested_classes: vec![],
                    }),
                ],
            },
        ],
    }
}
