use crate::ir_model::{
    AnnotationDef, EnumDef, EnumItem, EnumMember, FieldDef, NamespaceDef, NamespaceItem, SchemaContext,
    StructDef, StructItem,
};
use rhai::{Engine, Scope, EvalAltResult, Dynamic};
use std::path::Path;
use std::collections::BTreeMap;

pub fn generate_code_with_rhai(
    schema_context: &SchemaContext,
    template_path: &Path,
) -> Result<String, String> {
    let mut engine = Engine::new();
    engine.set_max_expr_depths(256, 256);
    register_types_and_getters(&mut engine);

    let mut scope = Scope::new();

    // Register the schema context with the Rhai engine
    scope.push("schema", schema_context.clone());

    let script = match std::fs::read_to_string(template_path) {
        Ok(s) => s,
        Err(e) => return Err(e.to_string()),
    };

    match engine.eval_with_scope::<String>(&mut scope, &script) {
        Ok(result) => Ok(result),
        Err(e) => Err(e.to_string()),
    }
}

fn register_types_and_getters(engine: &mut Engine) {
    engine.register_type_with_name::<SchemaContext>("SchemaContext");
    engine.register_get("namespaces", |ctx: &mut SchemaContext| {
        ctx.namespaces.iter().map(|ns| Dynamic::from(ns.clone())).collect::<Vec<Dynamic>>()
    });

    engine.register_type_with_name::<NamespaceDef>("NamespaceDef");
    engine.register_get("name", |ns: &mut NamespaceDef| ns.name.clone());
    engine.register_get("items", |ns: &mut NamespaceDef| {
        ns.items.iter().map(|item| Dynamic::from(item.clone())).collect::<Vec<Dynamic>>()
    });

    engine.register_type_with_name::<NamespaceItem>("NamespaceItem");
    engine.register_fn("is_struct", |item: &mut NamespaceItem| matches!(item, NamespaceItem::Struct(_)));
    engine.register_fn("as_struct", |item: &mut NamespaceItem| -> Result<StructDef, Box<EvalAltResult>> { match item { NamespaceItem::Struct(s) => Ok(s.clone()),
        _ => Err(Box::new(EvalAltResult::ErrorSystem("Cannot convert to StructDef".to_string(), Box::new(rhai::LexError::UnterminatedString)))), } });
    engine.register_fn("is_enum", |item: &mut NamespaceItem| matches!(item, NamespaceItem::Enum(_)));
    engine.register_fn("as_enum", |item: &mut NamespaceItem| -> Result<EnumDef, Box<EvalAltResult>> { match item { NamespaceItem::Enum(e) => Ok(e.clone()),
        _ => Err(Box::new(EvalAltResult::ErrorSystem("Cannot convert to EnumDef".to_string(), Box::new(rhai::LexError::UnterminatedString)))), } });
        

    engine.register_type_with_name::<StructDef>("StructDef");
    engine.register_get("name", |s: &mut StructDef| s.name.clone());
    engine.register_get("items", |s: &mut StructDef| {
        s.items.iter().map(|item| Dynamic::from(item.clone())).collect::<Vec<Dynamic>>()
    });
    engine.register_get("is_embed", |s: &mut StructDef| s.is_embed);

    engine.register_type_with_name::<StructItem>("StructItem");
    engine.register_fn("is_field", |item: &mut StructItem| matches!(item, StructItem::Field(_)));
    engine.register_fn("as_field", |item: &mut StructItem| -> Result<FieldDef, Box<EvalAltResult>> { match item { StructItem::Field(f) => Ok(f.clone()),
        _ => Err(Box::new(EvalAltResult::ErrorSystem("Cannot convert to FieldDef".to_string(), Box::new(rhai::LexError::UnterminatedString)))), } });


    engine.register_type_with_name::<FieldDef>("FieldDef");
    engine.register_get("name", |f: &mut FieldDef| f.name.clone());
    engine.register_get("field_type", |f: &mut FieldDef| f.field_type.clone());
    engine.register_get("comment", |f: &mut FieldDef| f.comment.clone());
    engine.register_get("attributes", |f: &mut FieldDef| {
        f.attributes.iter().map(|attr| Dynamic::from(attr.clone())).collect::<Vec<Dynamic>>()
    });

    engine.register_type_with_name::<EnumDef>("EnumDef");
    engine.register_get("name", |e: &mut EnumDef| e.name.clone());
    engine.register_get("items", |e: &mut EnumDef| {
        e.items.iter().map(|item| Dynamic::from(item.clone())).collect::<Vec<Dynamic>>()
    });

    engine.register_type_with_name::<EnumItem>("EnumItem");
    engine.register_fn("is_member", |item: &mut EnumItem| matches!(item, EnumItem::Member(_)));
    engine.register_fn("as_member", |item: &mut EnumItem| -> Result<EnumMember, Box<EvalAltResult>> { match item { EnumItem::Member(m) => Ok(m.clone()),
        _ => Err(Box::new(EvalAltResult::ErrorSystem("Cannot convert to EnumMember".to_string(), Box::new(rhai::LexError::UnterminatedString)))), } });


    engine.register_type_with_name::<EnumMember>("EnumMember");
    engine.register_get("name", |m: &mut EnumMember| m.name.clone());
    engine.register_get("comment", |m: &mut EnumMember| m.comment.clone());

    engine.register_type_with_name::<AnnotationDef>("AnnotationDef");
    engine.register_get("name", |a: &mut AnnotationDef| a.name.clone());
    engine.register_get("params", |a: &mut AnnotationDef| {
        a.params.iter().map(|(k, v)| (k.clone(), v.clone())).collect::<BTreeMap<String, String>>()
    });

    engine.register_fn("include", |path: &str| -> Result<String, Box<EvalAltResult>> {
        match std::fs::read_to_string(path) {
            Ok(s) => Ok(s),
            Err(e) => Err(Box::new(EvalAltResult::ErrorSystem(
                "File Read Error".to_string(),
                e.to_string().into(),
            ))),
        }
    });

    engine.register_fn("write_file", |path: &str, content: &str| -> Result<(), Box<EvalAltResult>> {
        if let Some(p) = Path::new(path).parent() {
            if !p.exists() {
                if let Err(e) = std::fs::create_dir_all(p) {
                    return Err(Box::new(EvalAltResult::ErrorSystem(
                        "Directory Creation Error".to_string(),
                        e.to_string().into(),
                    )));
                }
            }
        }

        match std::fs::write(path, content) {
            Ok(_) => Ok(()),
            Err(e) => Err(Box::new(EvalAltResult::ErrorSystem(
                "File Write Error".to_string(),
                e.to_string().into(),
            ))),
        }
    });
}