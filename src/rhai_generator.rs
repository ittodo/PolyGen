use crate::ir_model::{
    AnnotationDef, AnnotationParam, EnumDef, EnumItem, EnumMember, FieldDef, FileDef, NamespaceDef,
    NamespaceItem, SchemaContext, StructDef, StructItem,
};
use rhai::{Array, Dynamic, Engine, EvalAltResult, NativeCallContext, Scope};
use std::any::TypeId;
use std::path::Path;

pub fn generate_code_with_rhai(
    schema_context: &SchemaContext,
    template_path: &Path,
) -> Result<String, String> {
    let mut engine = Engine::new();
    // Increase recursion/complexity limits for larger templates
    engine.set_max_expr_depths(2048, 2048);
    register_types_and_getters(&mut engine);
    register_csv_helpers(&mut engine);

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
    engine.register_get("files", |ctx: &mut SchemaContext| {
        ctx.files
            .iter()
            .map(|f| Dynamic::from(f.clone()))
            .collect::<Vec<Dynamic>>()
    });

    engine.register_type_with_name::<FileDef>("FileDef");
    engine.register_get("path", |f: &mut FileDef| f.path.clone());
    engine.register_get("namespaces", |f: &mut FileDef| {
        f.namespaces
            .iter()
            .map(|ns| Dynamic::from(ns.clone()))
            .collect::<Vec<Dynamic>>()
    });

    engine.register_type_with_name::<NamespaceDef>("NamespaceDef");
    engine.register_get("name", |ns: &mut NamespaceDef| ns.name.clone());
    engine.register_get("items", |ns: &mut NamespaceDef| {
        ns.items
            .iter()
            .map(|item| Dynamic::from(item.clone()))
            .collect::<Vec<Dynamic>>()
    });

    engine.register_type_with_name::<NamespaceItem>("NamespaceItem");
    engine.register_fn("is_struct", |item: &mut NamespaceItem| {
        matches!(item, NamespaceItem::Struct(_))
    });
    engine.register_fn(
        "as_struct",
        |item: &mut NamespaceItem| -> Result<StructDef, Box<EvalAltResult>> {
            match item {
                NamespaceItem::Struct(s) => Ok(s.clone()),
                _ => Err(Box::new(EvalAltResult::ErrorSystem(
                    "Cannot convert to StructDef".to_string(),
                    Box::new(rhai::LexError::UnterminatedString),
                ))),
            }
        },
    );
    engine.register_fn("is_enum", |item: &mut NamespaceItem| {
        matches!(item, NamespaceItem::Enum(_))
    });
    engine.register_fn(
        "as_enum",
        |item: &mut NamespaceItem| -> Result<EnumDef, Box<EvalAltResult>> {
            match item {
                NamespaceItem::Enum(e) => Ok(e.clone()),
                _ => Err(Box::new(EvalAltResult::ErrorSystem(
                    "Cannot convert to EnumDef".to_string(),
                    Box::new(rhai::LexError::UnterminatedString),
                ))),
            }
        },
    );

    engine.register_fn("is_comment", |item: &mut NamespaceItem| {
        matches!(item, NamespaceItem::Comment(_))
    });
    engine.register_fn(
        "as_comment",
        |item: &mut NamespaceItem| -> Result<String, Box<EvalAltResult>> {
            match item {
                NamespaceItem::Comment(c) => Ok(c.clone()),
                _ => Err(Box::new(EvalAltResult::ErrorSystem(
                    "Cannot convert to Comment".to_string(),
                    Box::new(rhai::LexError::UnterminatedString),
                ))),
            }
        },
    );

    engine.register_fn("is_namespace", |item: &mut NamespaceItem| {
        matches!(item, NamespaceItem::Namespace(_))
    });
    engine.register_fn(
        "as_namespace",
        |item: &mut NamespaceItem| -> Result<NamespaceDef, Box<EvalAltResult>> {
            match item {
                NamespaceItem::Namespace(ns) => Ok((**ns).clone()),
                _ => Err(Box::new(EvalAltResult::ErrorSystem(
                    "Cannot convert to NamespaceDef".to_string(),
                    Box::new(rhai::LexError::UnterminatedString),
                ))),
            }
        },
    );

    engine.register_type_with_name::<StructDef>("StructDef");
    engine.register_get("name", |s: &mut StructDef| s.name.clone());
    engine.register_get("header", |s: &mut StructDef| {
        s.header
            .iter()
            .map(|header| Dynamic::from(header.clone()))
            .collect::<Vec<Dynamic>>()
    });
    engine.register_get("items", |s: &mut StructDef| {
        s.items
            .iter()
            .map(|item| Dynamic::from(item.clone()))
            .collect::<Vec<Dynamic>>()
    });

    engine.register_type_with_name::<StructItem>("StructItem");
    engine.register_fn("is_field", |item: &mut StructItem| {
        matches!(item, StructItem::Field(_))
    });

    engine.register_fn("is_comment", |item: &mut StructItem| {
        matches!(item, StructItem::Comment(_))
    });

    engine.register_fn("is_annotation", |item: &mut StructItem| {
        matches!(item, StructItem::Annotation(_))
    });

    engine.register_fn("is_embedded_struct", |item: &mut StructItem| {
        matches!(item, StructItem::EmbeddedStruct(_))
    });

    engine.register_fn("is_inline_enum", |item: &mut StructItem| {
        matches!(item, StructItem::InlineEnum(_))
    });

    engine.register_fn(
        "as_field",
        |item: &mut StructItem| -> Result<FieldDef, Box<EvalAltResult>> {
            match item {
                StructItem::Field(f) => Ok(f.clone()),
                _ => Err(Box::new(EvalAltResult::ErrorSystem(
                    "Cannot convert to FieldDef".to_string(),
                    Box::new(rhai::LexError::UnterminatedString),
                ))),
            }
        },
    );

    engine.register_fn(
        "as_comment",
        |item: &mut StructItem| -> Result<String, Box<EvalAltResult>> {
            match item {
                StructItem::Comment(f) => Ok(f.clone()),
                _ => Err(Box::new(EvalAltResult::ErrorSystem(
                    "Cannot convert to FieldDef".to_string(),
                    Box::new(rhai::LexError::UnterminatedString),
                ))),
            }
        },
    );

    engine.register_fn(
        "as_annotation",
        |item: &mut StructItem| -> Result<AnnotationDef, Box<EvalAltResult>> {
            match item {
                StructItem::Annotation(f) => Ok(f.clone()),
                _ => Err(Box::new(EvalAltResult::ErrorSystem(
                    "Cannot convert to FieldDef".to_string(),
                    Box::new(rhai::LexError::UnterminatedString),
                ))),
            }
        },
    );

    engine.register_fn(
        "as_embedded_struct",
        |item: &mut StructItem| -> Result<StructDef, Box<EvalAltResult>> {
            match item {
                StructItem::EmbeddedStruct(s) => Ok(s.clone()),
                _ => Err(Box::new(EvalAltResult::ErrorSystem(
                    "Cannot convert to EmbeddedStruct".to_string(),
                    Box::new(rhai::LexError::UnterminatedString),
                ))),
            }
        },
    );

    engine.register_fn(
        "as_inline_enum",
        |item: &mut StructItem| -> Result<EnumDef, Box<EvalAltResult>> {
            match item {
                StructItem::InlineEnum(e) => Ok(e.clone()),
                _ => Err(Box::new(EvalAltResult::ErrorSystem(
                    "Cannot convert to InlineEnum".to_string(),
                    Box::new(rhai::LexError::UnterminatedString),
                ))),
            }
        },
    );

    engine.register_type_with_name::<FieldDef>("FieldDef");
    engine.register_get("name", |f: &mut FieldDef| f.name.clone());
    engine.register_get("field_type", |f: &mut FieldDef| f.field_type.clone());
    engine.register_get("attributes", |f: &mut FieldDef| {
        f.attributes
            .iter()
            .map(|attr| Dynamic::from(attr.clone()))
            .collect::<Vec<Dynamic>>()
    });

    engine.register_type_with_name::<EnumDef>("EnumDef");
    engine.register_get("name", |e: &mut EnumDef| e.name.clone());
    engine.register_get("items", |e: &mut EnumDef| {
        e.items
            .iter()
            .map(|item| Dynamic::from(item.clone()))
            .collect::<Vec<Dynamic>>()
    });

    engine.register_type_with_name::<EnumItem>("EnumItem");
    engine.register_fn("is_member", |item: &mut EnumItem| {
        matches!(item, EnumItem::Member(_))
    });

    engine.register_fn("is_comment", |item: &mut EnumItem| {
        matches!(item, EnumItem::Comment(_))
    });

    engine.register_fn(
        "as_member",
        |item: &mut EnumItem| -> Result<EnumMember, Box<EvalAltResult>> {
            match item {
                EnumItem::Member(m) => Ok(m.clone()),
                _ => Err(Box::new(EvalAltResult::ErrorSystem(
                    "Cannot convert to EnumMember".to_string(),
                    Box::new(rhai::LexError::UnterminatedString),
                ))),
            }
        },
    );

    engine.register_fn(
        "as_comment",
        |item: &mut EnumItem| -> Result<String, Box<EvalAltResult>> {
            match item {
                EnumItem::Comment(m) => Ok(m.clone()),
                _ => Err(Box::new(EvalAltResult::ErrorSystem(
                    "Cannot convert to EnumItem.Comment".to_string(),
                    Box::new(rhai::LexError::UnterminatedString),
                ))),
            }
        },
    );

    engine.register_type_with_name::<EnumMember>("EnumMember");
    engine.register_get("name", |m: &mut EnumMember| m.name.clone());

    engine.register_type_with_name::<AnnotationDef>("AnnotationDef");
    engine.register_get("name", |a: &mut AnnotationDef| a.name.clone());
    engine.register_get("params", |a: &mut AnnotationDef| {
        a.params
            .iter()
            .map(|p| Dynamic::from(p.clone()))
            .collect::<Vec<Dynamic>>()
    });

    engine.register_type_with_name::<AnnotationParam>("AnnotationParam");
    engine.register_get("key", |p: &mut AnnotationParam| p.key.clone());
    engine.register_get("value", |p: &mut AnnotationParam| p.value.clone());

    engine.register_fn(
        "render_items",
        |context: NativeCallContext,
         items: Array,
         template_path: &str,
         var_name: &str| 
         -> Result<String, Box<EvalAltResult>> {
            let template = match std::fs::read_to_string(template_path) {
                Ok(s) => s,
                Err(e) => return Err(format!("Failed to read template file '{}': {}", template_path, e).into()),
            };

            let engine = context.engine();
            let mut result = String::new();
            let template_literal = format!("`{}`", template);

            for item in items {
                let mut scope = Scope::new();
                scope.push(var_name, item);
                let rendered = engine.eval_with_scope::<String>(&mut scope, &template_literal)?;
                result.push_str(&rendered);
            }
            Ok(result)
        },
    );

    engine.register_fn(
        "include",
        |path: &str| -> Result<String, Box<EvalAltResult>> {
            match std::fs::read_to_string(path) {
                Ok(s) => Ok(s),
                Err(e) => Err(Box::new(EvalAltResult::ErrorSystem(
                    format!("File Read Error at path: {}", path),
                    e.to_string().into(),
                ))),
            }
        },
    );

    engine.register_fn(
        "write_file",
        |path: &str, content: &str| -> Result<(), Box<EvalAltResult>> {
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
        },
    );
}

// ------------------- CSV Helpers (Rust-backed for Rhai) -------------------
fn register_csv_helpers(engine: &mut Engine) {
    // headers_for_struct(struct, current_ns, all_files[]) -> string[]
    engine.register_fn(
        "csv_headers_for_struct",
        |s: StructDef, current_ns_name: String, all_files_dyn: Array| -> Array {
            let files = array_to_files(&all_files_dyn);
            let headers = collect_headers_for_struct(&s, &current_ns_name, &files);
            headers.into_iter().map(Dynamic::from).collect()
        },
    );

    // csv_append_code_for(ctx_struct, type_string, expr_prefix, current_ns, all_files[], visited[], depth) -> string
    engine.register_fn(
        "csv_append_code_for",
        |ctx: StructDef,
         type_string: String,
         expr_prefix: String,
         current_ns_name: String,
         all_files_dyn: Array,
         visited_dyn: Array,
         depth: i64|
         -> String {
            let files = array_to_files(&all_files_dyn);
            let mut visited: Vec<String> = visited_dyn
                .into_iter()
                .filter_map(|d| d.try_cast::<String>())
                .collect();
            generate_append_code(&ctx, &type_string, &expr_prefix, &current_ns_name, &files, &mut visited, depth as usize)
        },
    );

    // Generate C# assignments to read fields of a struct from a row with prefix
    engine.register_fn(
        "csv_read_fields_for_struct",
        |s: StructDef, obj_expr: String, prefix: String, current_ns_name: String, all_files_dyn: Array| -> String {
            let files = array_to_files(&all_files_dyn);
            generate_read_fields_for_struct(&s, &obj_expr, &prefix, &current_ns_name, &files, &s.name)
        },
    );

    // Indexed (header + row[] + map) variant; expects variables: `row: string[]`, `map: Dictionary<string,int>`, `gap: i32` (0=Break,1=Sparse)
    engine.register_fn(
        "csv_read_fields_for_struct_indexed",
        |s: StructDef, obj_expr: String, prefix_var: String, current_ns_name: String, all_files_dyn: Array| -> String {
            let files = array_to_files(&all_files_dyn);
            generate_read_fields_for_struct_indexed(&s, &obj_expr, &prefix_var, &current_ns_name, &files, &s.name)
        },
    );

    // Dynamic writer helpers: emit full C# methods for dynamic header/append/write
    engine.register_fn(
        "csv_dynamic_methods_for_struct",
        |s: StructDef, current_ns_name: String, all_files_dyn: Array| -> String {
            let files = array_to_files(&all_files_dyn);
            generate_dynamic_methods_for_struct(&s, &current_ns_name, &files)
        },
    );
}

fn array_to_files(arr: &Array) -> Vec<FileDef> {
    arr.iter().filter_map(|d| d.clone().try_cast::<FileDef>()).collect()
}

fn unwrap_option(t: &str) -> &str {
    const P: &str = "Option<";
    if t.starts_with(P) && t.ends_with('>') {
        let inner = &t[P.len()..t.len() - 1];
        inner
    } else {
        t
    }
}

fn is_primitive_like(t: &str) -> bool {
    matches!(
        t,
        "u8" | "i8" | "u16" | "i16" | "u32" | "i32" | "u64" | "i64" | "f32" | "f64" | "bool" | "string"
    )
}

fn find_embedded_struct<'a>(s: &'a StructDef, name: &str) -> Option<&'a StructDef> {
    s.items.iter().find_map(|it| match it {
        StructItem::EmbeddedStruct(es) if es.name == name => Some(es),
        _ => None,
    })
}

fn find_struct_in_ns<'a>(ns: &'a NamespaceDef, target_name: &str) -> Option<&'a StructDef> {
    ns.items.iter().find_map(|item| match item {
        NamespaceItem::Struct(s) if s.name == target_name => Some(s),
        _ => None,
    })
}

fn find_enum_in_ns<'a>(ns: &'a NamespaceDef, target_name: &str) -> Option<&'a EnumDef> {
    ns.items.iter().find_map(|item| match item {
        NamespaceItem::Enum(e) if e.name == target_name => Some(e),
        _ => None,
    })
}

fn find_struct_in_tree<'a>(ns: &'a NamespaceDef, prefix: &str, target_ns: &str, target_name: &str) -> Option<&'a StructDef> {
    let fqn_string = if prefix.is_empty() {
        ns.name.clone()
    } else {
        format!("{}.{}", prefix, ns.name)
    };
    let fqn = fqn_string.as_str();
    if fqn == target_ns {
        if let Some(s) = find_struct_in_ns(ns, target_name) {
            return Some(s);
        }
    }
    for item in &ns.items {
        if let NamespaceItem::Namespace(child) = item {
            if let Some(s) = find_struct_in_tree(child, fqn, target_ns, target_name) {
                return Some(s);
            }
        }
    }
    None
}

fn find_enum_in_tree<'a>(ns: &'a NamespaceDef, prefix: &str, target_ns: &str, target_name: &str) -> Option<&'a EnumDef> {
    let fqn_string = if prefix.is_empty() {
        ns.name.clone()
    } else {
        format!("{}.{}", prefix, ns.name)
    };
    let fqn = fqn_string.as_str();
    if fqn == target_ns {
        if let Some(e) = find_enum_in_ns(ns, target_name) {
            return Some(e);
        }
    }
    for item in &ns.items {
        if let NamespaceItem::Namespace(child) = item {
            if let Some(e) = find_enum_in_tree(child, fqn, target_ns, target_name) {
                return Some(e);
            }
        }
    }
    None
}

fn get_struct_at<'a>(files: &'a [FileDef], target_ns: &str, target_name: &str) -> Option<&'a StructDef> {
    for file in files {
        for root_ns in &file.namespaces {
            if let Some(s) = find_struct_in_tree(root_ns, "", target_ns, target_name) {
                return Some(s);
            }
        }
    }
    None
}

fn get_enum_at<'a>(files: &'a [FileDef], target_ns: &str, target_name: &str) -> Option<&'a EnumDef> {
    for file in files {
        for root_ns in &file.namespaces {
            if let Some(e) = find_enum_in_tree(root_ns, "", target_ns, target_name) {
                return Some(e);
            }
        }
    }
    None
}

fn any_struct_named_in<'a>(ns: &'a NamespaceDef, target_name: &str) -> Option<&'a StructDef> {
    if let Some(s) = find_struct_in_ns(ns, target_name) {
        return Some(s);
    }
    for item in &ns.items {
        if let NamespaceItem::Namespace(child) = item {
            if let Some(s) = any_struct_named_in(child, target_name) {
                return Some(s);
            }
        }
    }
    None
}

fn any_enum_named_in<'a>(ns: &'a NamespaceDef, target_name: &str) -> Option<&'a EnumDef> {
    if let Some(e) = find_enum_in_ns(ns, target_name) {
        return Some(e);
    }
    for item in &ns.items {
        if let NamespaceItem::Namespace(child) = item {
            if let Some(e) = any_enum_named_in(child, target_name) {
                return Some(e);
            }
        }
    }
    None
}

fn any_struct_named<'a>(files: &'a [FileDef], target_name: &str) -> Option<&'a StructDef> {
    for file in files {
        for root_ns in &file.namespaces {
            if let Some(s) = find_struct_in_ns(root_ns, target_name) {
                return Some(s);
            }
            if let Some(s) = any_struct_named_in(root_ns, target_name) {
                return Some(s);
            }
        }
    }
    None
}

fn any_enum_named<'a>(files: &'a [FileDef], target_name: &str) -> Option<&'a EnumDef> {
    for file in files {
        for root_ns in &file.namespaces {
            if let Some(e) = find_enum_in_ns(root_ns, target_name) {
                return Some(e);
            }
            if let Some(e) = any_enum_named_in(root_ns, target_name) {
                return Some(e);
            }
        }
    }
    None
}

fn resolve_struct<'a>(files: &'a [FileDef], type_string: &str, current_ns_name: &str) -> Option<&'a StructDef> {
    let mut core = unwrap_option(type_string);
    if let Some(inner) = core.strip_prefix("List<").and_then(|s| s.strip_suffix('>')) {
        core = inner;
    }
    if core.contains('.') {
        let mut parts = core.split('.').collect::<Vec<_>>();
        let name = parts.pop().unwrap();
        let ns = parts.join(".");
        get_struct_at(files, &ns, name)
    } else {
        if !current_ns_name.is_empty() {
            if let Some(s) = get_struct_at(files, current_ns_name, core) {
                return Some(s);
            }
        }
        any_struct_named(files, core)
    }
}

fn find_struct_with_ns_in_tree<'a>(ns: &'a NamespaceDef, prefix: &str, target_ns: &str, target_name: &str) -> Option<(&'a StructDef, String)> {
    let fqn_string = if prefix.is_empty() {
        ns.name.clone()
    } else {
        format!("{}.{}", prefix, ns.name)
    };
    let fqn = fqn_string.as_str();
    if fqn == target_ns {
        if let Some(s) = find_struct_in_ns(ns, target_name) {
            return Some((s, fqn_string));
        }
    }
    for item in &ns.items {
        if let NamespaceItem::Namespace(child) = item {
            if let Some(res) = find_struct_with_ns_in_tree(child, fqn, target_ns, target_name) {
                return Some(res);
            }
        }
    }
    None
}

fn get_struct_with_ns_at<'a>(files: &'a [FileDef], target_ns: &str, target_name: &str) -> Option<(&'a StructDef, String)> {
    for file in files {
        for root_ns in &file.namespaces {
            if let Some(res) = find_struct_with_ns_in_tree(root_ns, "", target_ns, target_name) {
                return Some(res);
            }
        }
    }
    None
}

fn any_struct_named_with_ns_in<'a>(ns: &'a NamespaceDef, target_name: &str, prefix: &str) -> Option<(&'a StructDef, String)> {
    if let Some(s) = find_struct_in_ns(ns, target_name) {
        let fqn = if prefix.is_empty() { ns.name.clone() } else { format!("{}.{}", prefix, ns.name) };
        return Some((s, fqn));
    }
    for item in &ns.items {
        if let NamespaceItem::Namespace(child) = item {
            let next_prefix = if prefix.is_empty() { ns.name.clone() } else { format!("{}.{}", prefix, ns.name) };
            if let Some(res) = any_struct_named_with_ns_in(child, target_name, &next_prefix) {
                return Some(res);
            }
        }
    }
    None
}

fn any_struct_named_with_ns<'a>(files: &'a [FileDef], target_name: &str) -> Option<(&'a StructDef, String)> {
    for file in files {
        for root_ns in &file.namespaces {
            if let Some(s) = find_struct_in_ns(root_ns, target_name) {
                return Some((s, root_ns.name.clone()));
            }
            if let Some(res) = any_struct_named_with_ns_in(root_ns, target_name, &root_ns.name) {
                return Some(res);
            }
        }
    }
    None
}

fn resolve_struct_with_ns<'a>(files: &'a [FileDef], type_string: &str, current_ns_name: &str) -> Option<(&'a StructDef, String)> {
    let mut core = unwrap_option(type_string);
    if let Some(inner) = core.strip_prefix("List<").and_then(|s| s.strip_suffix('>')) {
        core = inner;
    }
    if core.contains('.') {
        let mut parts = core.split('.').collect::<Vec<_>>();
        let name = parts.pop().unwrap();
        let ns = parts.join(".");
        get_struct_with_ns_at(files, &ns, name)
    } else {
        if !current_ns_name.is_empty() {
            if let Some(res) = get_struct_with_ns_at(files, current_ns_name, core) {
                return Some(res);
            }
        }
        any_struct_named_with_ns(files, core)
    }
}

fn resolve_enum<'a>(files: &'a [FileDef], type_string: &str, current_ns_name: &str) -> Option<&'a EnumDef> {
    let mut core = unwrap_option(type_string);
    if let Some(inner) = core.strip_prefix("List<").and_then(|s| s.strip_suffix('>')) {
        core = inner;
    }
    if core.contains('.') {
        let mut parts = core.split('.').collect::<Vec<_>>();
        let name = parts.pop().unwrap();
        let ns = parts.join(".");
        get_enum_at(files, &ns, name)
    } else {
        if !current_ns_name.is_empty() {
            if let Some(e) = get_enum_at(files, current_ns_name, core) {
                return Some(e);
            }
        }
        any_enum_named(files, core)
    }
}

fn collect_columns_with<'a>(ctx_struct: &'a StructDef, prefix: &str, type_string: &str, visited: &mut Vec<String>, depth: usize, current_ns_name: &str, files: &'a [FileDef]) -> Vec<String> {
    let mut cols = Vec::new();
    let mut t = unwrap_option(type_string).to_string();
    if depth >= 10 {
        cols.push(prefix.to_string());
        return cols;
    }
    if let Some(inner) = t.strip_prefix("List<").and_then(|s| s.strip_suffix('>')) {
        let np = if prefix.is_empty() { "[0]".to_string() } else { format!("{}[0]", prefix) };
        let sub = collect_columns_with(ctx_struct, &np, inner, visited, depth + 1, current_ns_name, files);
        cols.extend(sub);
        return cols;
    }
    if let Some(es) = find_embedded_struct(ctx_struct, &t) {
        if visited.iter().any(|v| v == &es.name) {
            return cols;
        }
        visited.push(es.name.clone());
        for it in &es.items {
            if let StructItem::Field(f) = it {
                let np = if prefix.is_empty() { f.name.clone() } else { format!("{}.{}", prefix, f.name) };
                let mut v2 = visited.clone();
                let sub = collect_columns_with(es, &np, &f.field_type.original, &mut v2, depth + 1, current_ns_name, files);
                cols.extend(sub);
            }
        }
        return cols;
    }
    if let Some(ext) = resolve_struct(files, &t, current_ns_name) {
        if visited.iter().any(|v| v == &ext.name) {
            return cols;
        }
        visited.push(ext.name.clone());
        for it in &ext.items {
            if let StructItem::Field(f) = it {
                let np = if prefix.is_empty() { f.name.clone() } else { format!("{}.{}", prefix, f.name) };
                let mut v2 = visited.clone();
                let sub = collect_columns_with(ext, &np, &f.field_type.original, &mut v2, depth + 1, current_ns_name, files);
                cols.extend(sub);
            }
        }
        return cols;
    }
    cols.push(prefix.to_string());
    cols
}

fn collect_headers_for_struct(s: &StructDef, current_ns_name: &str, files: &[FileDef]) -> Vec<String> {
    let mut headers = Vec::new();
    for it in &s.items {
        if let StructItem::Field(f) = it {
            let mut visited = vec![s.name.clone()];
            let sub = collect_columns_with(s, &f.name, &f.field_type.original, &mut visited, 0, current_ns_name, files);
            headers.extend(sub);
        }
    }
    headers
}

fn is_inline_enum_name(name: &str) -> bool {
    name.ends_with("__Enum")
}

fn map_cs_primitive(t: &str) -> Option<&'static str> {
    match t {
        "u8" => Some("byte"),
        "i8" => Some("sbyte"),
        "u16" => Some("ushort"),
        "i16" => Some("short"),
        "u32" => Some("uint"),
        "i32" => Some("int"),
        "u64" => Some("ulong"),
        "i64" => Some("long"),
        "f32" => Some("float"),
        "f64" => Some("double"),
        "bool" => Some("bool"),
        "string" => Some("string"),
        _ => None,
    }
}

fn cs_type_for<'a>(files: &'a [FileDef], ctx_struct: &StructDef, current_ns_name: &str, type_string: &str) -> String {
    let core = unwrap_option(type_string);
    if let Some(inner) = core.strip_prefix("List<").and_then(|s| s.strip_suffix('>')) {
        let inner_cs = cs_type_for(files, ctx_struct, current_ns_name, inner);
        return format!("List<{}>", inner_cs);
    }
    if let Some(p) = map_cs_primitive(core) {
        return p.to_string();
    }
    // enum type
    if is_inline_enum_name(core) {
        return format!("{}.{}", ctx_struct.name, core);
    }
    // external or same-namespace struct/enum: assume type_string already usable as C# type (may be fully-qualified)
    core.to_string()
}

fn generate_read_fields_for_struct(s: &StructDef, obj_expr: &str, cprefix: &str, current_ns_name: &str, files: &[FileDef], owner_fqn: &str) -> String {
    let mut code = String::new();
    for it in &s.items {
        if let StructItem::Field(f) = it {
            code.push_str(&generate_read_assign_for_field(
                s,
                f,
                obj_expr,
                cprefix,
                current_ns_name,
                files,
                &mut vec![s.name.clone()],
                0,
                owner_fqn,
            ));
        }
    }
    code
}

fn generate_read_fields_for_struct_indexed(s: &StructDef, obj_expr: &str, prefix_var: &str, current_ns_name: &str, files: &[FileDef], owner_fqn: &str) -> String {
    fn gen_get_cell(expr_key: &str) -> String {
        format!(
            "{{ int __idx; string __cell=null; if (map.TryGetValue({key}, out __idx) && __idx >= 0 && __idx < row.Length) __cell = row[__idx]; ",
            key = expr_key
        )
    }

    fn fmt_key(prefix_var: &str, suffix: &str) -> String {
        format!("{pref} + \"{suf}\"", pref = prefix_var, suf = suffix)
    }

    let mut code = String::new();
    for it in &s.items {
        if let StructItem::Field(f) = it {
            code.push_str(&gen_read_assign_indexed(
                s,
                f,
                obj_expr,
                prefix_var,
                current_ns_name,
                files,
                &mut vec![s.name.clone()],
                0,
                owner_fqn,
            ));
        }
    }
    code
}

fn gen_read_assign_indexed(
    ctx_struct: &StructDef,
    field: &FieldDef,
    obj_expr: &str,
    prefix_var: &str,
    current_ns_name: &str,
    files: &[FileDef],
    visited: &mut Vec<String>,
    depth: usize,
    owner_fqn: &str,
) -> String {
    let mut code = String::new();
    let field_name = &field.name;
    let mut t = unwrap_option(&field.field_type.original).to_string();
    if depth >= 10 {
        return code;
    }

    // List<>
    if let Some(inner) = t.strip_prefix("List<").and_then(|s| s.strip_suffix('>')) {
        // primitive list
        if map_cs_primitive(inner).is_some() {
            code.push_str(&format!(
                "{{ var list = new List<{inner}>(); int i=0; for(;;i++) {{ int __idx; if (!map.TryGetValue({pref} + \"{fname}[\"+i+\"]\", out __idx)) {{ if (i==0) break; else break; }} if (__idx < 0 || __idx >= row.Length) break; var __cell = row[__idx]; if (string.IsNullOrEmpty(__cell)) {{ if (i==0 || gap==0) break; else continue; }} list.Add(DataSourceFactory.ConvertValue<{inner}>(__cell)); }} {obj}.{fname} = list; }}\n",
                inner = cs_type_for(files, ctx_struct, current_ns_name, inner),
                pref = prefix_var,
                fname = field_name,
                obj = obj_expr
            ));
            return code;
        }
        // enum list
        if is_inline_enum_name(inner) || resolve_enum(files, inner, current_ns_name).is_some() {
            let enum_ty = if is_inline_enum_name(inner) {
                format!("{}.{}", ctx_struct.name, inner)
            } else {
                inner.to_string()
            };
            code.push_str(&format!(
                "{{ var list = new List<{et}>(); int i=0; for(;;i++) {{ int __idx; if (!map.TryGetValue({pref} + \"{fname}[\"+i+\"]\", out __idx)) {{ if (i==0) break; else break; }} if (__idx < 0 || __idx >= row.Length) break; var __cell = row[__idx]; if (string.IsNullOrEmpty(__cell)) {{ if (i==0 || gap==0) break; else continue; }} list.Add(DataSourceFactory.ConvertValue<{et}>(__cell)); }} {obj}.{fname} = list; }}\n",
                et = enum_ty,
                pref = prefix_var,
                fname = field_name,
                obj = obj_expr
            ));
            return code;
        }
        // embedded struct list
        if let Some(es) = find_embedded_struct(ctx_struct, inner) {
            // existence by any sub header for index i
            code.push_str(&format!(
                "{{ var list = new List<{owner}.{ename}>(); int i=0; for(;;i++) {{ bool any=false; string __tmp; ",
                owner = owner_fqn,
                ename = es.name
            ));
            for it in &es.items {
                if let StructItem::Field(f2) = it {
                    let mut v2 = visited.clone();
                    let tails = collect_columns_with(es, &f2.name, &f2.field_type.original, &mut v2, depth + 1, current_ns_name, files);
                    for tail in tails {
                        code.push_str(&format!(
                            "{{ int __idx; if (map.TryGetValue({pref} + \"{fname}[\"+i+\"].{tail}\", out __idx) && __idx>=0 && __idx<row.Length && !string.IsNullOrEmpty(row[__idx])) any=true; }} ",
                            pref = prefix_var,
                            fname = field_name,
                            tail = tail
                        ));
                    }
                }
            }
            code.push_str("if (!any) { if (i==0 || gap==0) break; else continue; } var sub = new ");
            code.push_str(&format!("{}.{}();\n", owner_fqn, es.name));
            let next_owner = format!("{}.{}", owner_fqn, es.name);
            for it in &es.items {
                if let StructItem::Field(f2) = it {
                    code.push_str(&gen_read_assign_indexed(
                        es,
                        f2,
                        "sub",
                        &format!("{pref} + \"{fname}[\"+i+\"].\"", pref = prefix_var, fname = field_name),
                        current_ns_name,
                        files,
                        &mut visited.clone(),
                        depth + 1,
                        &next_owner,
                    ));
                }
            }
            code.push_str(&format!("list.Add(sub); }} {obj}.{fname} = list; }}\n", obj = obj_expr, fname = field_name));
            return code;
        }
        // external struct list
        if let Some((ext, ns_fqn)) = resolve_struct_with_ns(files, inner, current_ns_name) {
            code.push_str("{ var list = new System.Collections.Generic.List<");
            code.push_str(&format!("{}.{}>(); int i=0; for(;;i++) {{ bool any=false; string __tmp; ", ns_fqn, ext.name));
            for it in &ext.items {
                if let StructItem::Field(f2) = it {
                    let mut v2 = visited.clone();
                    let tails = collect_columns_with(ext, &f2.name, &f2.field_type.original, &mut v2, depth + 1, current_ns_name, files);
                    for tail in tails {
                        code.push_str(&format!(
                            "{{ int __idx; if (map.TryGetValue({pref} + \"{fname}[\"+i+\"].{tail}\", out __idx) && __idx>=0 && __idx<row.Length && !string.IsNullOrEmpty(row[__idx])) any=true; }} ",
                            pref = prefix_var,
                            fname = field_name,
                            tail = tail
                        ));
                    }
                }
            }
            code.push_str(&format!(
                "if (!any) {{ if (i==0 || gap==0) break; else continue; }} list.Add({ns}.{ty}Csv.FromRowWithPrefixAndHeader(header, row, {pref} + \"{fname}[\"+i+\"].\", gap)); }} {obj}.{fname} = list; }}\n",
                ns = ns_fqn,
                ty = ext.name,
                pref = prefix_var,
                fname = field_name,
                obj = obj_expr
            ));
            return code;
        }
        return code;
    }

    // primitive
    if let Some(p) = map_cs_primitive(&t) {
        code.push_str(&format!(
            "{{ int __idx; string __cell=null; if (map.TryGetValue({pref} + \"{fname}\", out __idx) && __idx>=0 && __idx < row.Length) __cell = row[__idx]; {obj}.{fname} = DataSourceFactory.ConvertValue<{ty}>(__cell); }}\n",
            pref = prefix_var,
            fname = field_name,
            obj = obj_expr,
            ty = p
        ));
        return code;
    }
    // enum
    if is_inline_enum_name(&t) || resolve_enum(files, &t, current_ns_name).is_some() {
        let enum_ty = if is_inline_enum_name(&t) {
            format!("{}.{}", ctx_struct.name, t)
        } else {
            t.clone()
        };
        code.push_str(&format!(
            "{{ int __idx; string __cell=null; if (map.TryGetValue({pref} + \"{fname}\", out __idx) && __idx>=0 && __idx < row.Length) __cell = row[__idx]; {obj}.{fname} = DataSourceFactory.ConvertValue<{ety}>(__cell); }}\n",
            pref = prefix_var,
            fname = field_name,
            obj = obj_expr,
            ety = enum_ty
        ));
        return code;
    }
    // embedded struct single
    if let Some(es) = find_embedded_struct(ctx_struct, &t) {
        // presence via sub headers
        code.push_str("{ bool any=false; ");
        for it in &es.items {
            if let StructItem::Field(f2) = it {
                let mut v2 = visited.clone();
                let tails = collect_columns_with(es, &f2.name, &f2.field_type.original, &mut v2, depth + 1, current_ns_name, files);
                for tail in tails {
                    code.push_str(&format!(
                        "{{ int __idx; if (map.TryGetValue({pref} + \"{fname}.{tail}\", out __idx) && __idx>=0 && __idx<row.Length && !string.IsNullOrEmpty(row[__idx])) any=true; }} ",
                        pref = prefix_var,
                        fname = field_name,
                        tail = tail
                    ));
                }
            }
        }
        code.push_str(&format!("if (!any) {{ {obj}.{fname} = null; }} else {{ var sub = new {owner}.{ename}(); ", obj = obj_expr, fname=field_name, owner = owner_fqn, ename=es.name));
        let next_owner = format!("{}.{}", owner_fqn, es.name);
        for it in &es.items {
            if let StructItem::Field(f2) = it {
                code.push_str(&gen_read_assign_indexed(
                    es,
                    f2,
                    "sub",
                    &format!("{pref} + \"{fname}.\"", pref=prefix_var, fname=field_name),
                    current_ns_name,
                    files,
                    &mut visited.clone(),
                    depth + 1,
                    &next_owner,
                ));
            }
        }
        code.push_str(&format!("{obj}.{fname} = sub; }} }}\n", obj=obj_expr, fname=field_name));
        return code;
    }
    // external struct single
    if let Some((ext, ns_fqn)) = resolve_struct_with_ns(files, &t, current_ns_name) {
        // presence via sub headers
        code.push_str("{ bool any=false; ");
        for it in &ext.items {
            if let StructItem::Field(f2) = it {
                let mut v2 = visited.clone();
                let tails = collect_columns_with(ext, &f2.name, &f2.field_type.original, &mut v2, depth + 1, current_ns_name, files);
                for tail in tails {
                    code.push_str(&format!(
                        "{{ int __idx; if (map.TryGetValue({pref} + \"{fname}.{tail}\", out __idx) && __idx>=0 && __idx<row.Length && !string.IsNullOrEmpty(row[__idx])) any=true; }} ",
                        pref = prefix_var,
                        fname = field_name,
                        tail = tail
                    ));
                }
            }
        }
        code.push_str(&format!(
            "if (!any) {{ {obj}.{fname} = null; }} else {{ {obj}.{fname} = {ns}.{ty}Csv.FromRowWithPrefixAndHeader(header, row, {pref} + \"{fname}.\", gap); }} }}\n",
            obj = obj_expr,
            fname = field_name,
            ns = ns_fqn,
            ty = ext.name,
            pref = prefix_var
        ));
        return code;
    }

    code
}

fn list_inner_type(type_string: &str) -> Option<&str> {
    type_string
        .strip_prefix("List<")
        .and_then(|s| s.strip_suffix('>'))
}

fn is_list_type(type_string: &str) -> bool {
    list_inner_type(type_string).is_some()
}

fn generate_dynamic_methods_for_struct(s: &StructDef, current_ns_name: &str, files: &[FileDef]) -> String {
    let mut code = String::new();

    // ComputeListMaxes
    code.push_str(&format!(
        "        public static System.Collections.Generic.Dictionary<string,int> ComputeListMaxes(System.Collections.Generic.IEnumerable<{}> items)\n        {{\n",
        s.name
    ));
    code.push_str(
        "            var d = new System.Collections.Generic.Dictionary<string,int>(System.StringComparer.OrdinalIgnoreCase);\n",
    );
    code.push_str("            foreach (var it in items) {\n");
    for it in &s.items {
        if let StructItem::Field(f) = it {
            if is_list_type(&f.field_type.original) {
                code.push_str(&format!(
                    "                var c_{fname} = (it.{fname} != null ? it.{fname}.Count : 0); if (!d.TryGetValue(\"{fname}\", out var m_{fname}) || c_{fname} > m_{fname}) d[\"{fname}\"] = c_{fname};\n",
                    fname = f.name
                ));
            }
        }
    }
    code.push_str("            }\n");
    code.push_str("            return d;\n        }\n");

    // GetDynamicHeader
    code.push_str(
        "        public static string[] GetDynamicHeader(System.Collections.Generic.Dictionary<string,int> listMaxes)\n        {\n            var cols = new System.Collections.Generic.List<string>();\n",
    );
    // Non-list columns
    for it in &s.items {
        if let StructItem::Field(f) = it {
            if !is_list_type(&f.field_type.original) {
                let mut visited = vec![s.name.clone()];
                let non_cols = collect_columns_with(s, &f.name, &f.field_type.original, &mut visited, 0, current_ns_name, files);
                for c in non_cols {
                    code.push_str(&format!("            cols.Add(\"{}\");\n", c));
                }
            }
        }
    }
    // List columns
    for it in &s.items {
        if let StructItem::Field(f) = it {
            if let Some(inner) = list_inner_type(&f.field_type.original) {
                let mut visited = vec![s.name.clone()];
                let tails = collect_columns_with(s, "", inner, &mut visited, 1, current_ns_name, files);
                code.push_str(&format!(
                    "            int __mx_{fname} = 0; if (listMaxes != null) listMaxes.TryGetValue(\"{fname}\", out __mx_{fname});\n            for (int __i=0; __i<__mx_{fname}; __i++) {{\n",
                    fname = f.name
                ));
                for tail in tails {
                    if tail.is_empty() {
                        code.push_str(&format!(
                            "                cols.Add(string.Format(\"{name}[{{0}}]\", __i));\n",
                            name = f.name
                        ));
                    } else {
                        code.push_str(&format!(
                            "                cols.Add(string.Format(\"{name}[{{0}}].{tail}\", __i));\n",
                            name = f.name,
                            tail = tail
                        ));
                    }
                }
                code.push_str("            }\n");
            }
        }
    }
    code.push_str("            return cols.ToArray();\n        }\n");

    // AppendRowDynamic
    code.push_str(&format!(
        "        public static void AppendRowDynamic({} obj, System.Collections.Generic.List<string> cols, System.Collections.Generic.Dictionary<string,int> listMaxes)\n        {{\n",
        s.name
    ));
    for it in &s.items {
        if let StructItem::Field(f) = it {
            if let Some(inner) = list_inner_type(&f.field_type.original) {
                let mut visited = vec![s.name.clone()];
                let tails = collect_columns_with(s, "", inner, &mut visited, 1, current_ns_name, files);
                code.push_str(&format!(
                    "            int __mx = 0; if (listMaxes != null) listMaxes.TryGetValue(\"{fname}\", out __mx);\n            for (int __i=0; __i<__mx; __i++) {{\n                if (obj.{fname} != null && obj.{fname}.Count > __i) {{\n",
                    fname = f.name
                ));
                code.push_str(&generate_append_code(
                    s,
                    inner,
                    &format!("obj.{}[__i]", f.name),
                    current_ns_name,
                    files,
                    &mut vec![s.name.clone()],
                    0,
                ));
                code.push_str("                } else {\n");
                for _ in tails {
                    code.push_str("                    cols.Add(string.Empty);\n");
                }
                code.push_str("                }\n            }\n");
            } else {
                // non-list
                code.push_str(&generate_append_code(
                    s,
                    &f.field_type.original,
                    &format!("obj.{}", f.name),
                    current_ns_name,
                    files,
                    &mut vec![s.name.clone()],
                    0,
                ));
            }
        }
    }
    code.push_str("        }\n");

    // WriteCsvDynamic wrapper
    code.push_str(&format!(
        "        public static void WriteCsvDynamic(System.Collections.Generic.IEnumerable<{}> items, string path, bool writeHeader = true, char sep = ',')\n        {{\n            var maxes = ComputeListMaxes(items);\n            using var sw = new System.IO.StreamWriter(path, false, new System.Text.UTF8Encoding(false));\n            if (writeHeader) {{ var hdr = GetDynamicHeader(maxes); sw.WriteLine(Polygen.Common.CsvUtils.Join(hdr, sep)); }}\n            foreach (var it in items) {{ var row = new System.Collections.Generic.List<string>(256); AppendRowDynamic(it, row, maxes); sw.WriteLine(Polygen.Common.CsvUtils.Join(row, sep)); }}\n        }}\n",
        s.name
    ));

    code
}

fn generate_read_assign_for_field(
    ctx_struct: &StructDef,
    field: &FieldDef,
    obj_expr: &str,
    cprefix: &str,
    current_ns_name: &str,
    files: &[FileDef],
    visited: &mut Vec<String>,
    depth: usize,
    owner_fqn: &str,
) -> String {
    let mut code = String::new();
    let field_name = &field.name;
    let mut t = unwrap_option(&field.field_type.original).to_string();
    if depth >= 10 {
        return code;
    }
    if let Some(inner) = t.strip_prefix("List<").and_then(|s| s.strip_suffix('>')) {
        // handle list with [0]
        let inner_cs = cs_type_for(files, ctx_struct, current_ns_name, inner);
        // list of primitives
        if map_cs_primitive(inner).is_some() {
            code.push_str(&format!(
                "{{ var list = new List<{inner}>(); var key = {pref} + \"{fn}[0]\"; string cell; if (row.TryGetValue(key, out cell) && !string.IsNullOrEmpty(cell)) {{ list.Add(DataSourceFactory.ConvertValue<{inner}>(cell)); }} {obj}.{fn} = list; }}\n",
                inner = inner_cs,
                fn = field_name,
                obj = obj_expr,
                pref = cprefix
            ));
            return code;
        }
        // list of enums
        if is_inline_enum_name(inner) || resolve_enum(files, inner, current_ns_name).is_some() {
            let enum_ty = if is_inline_enum_name(inner) {
                format!("{}.{}", ctx_struct.name, inner)
            } else {
                inner.to_string()
            };
            code.push_str(&format!(
                "{{ var list = new List<{et}>(); var key = {pref} + \"{fn}[0]\"; string cell; if (row.TryGetValue(key, out cell) && !string.IsNullOrEmpty(cell)) {{ list.Add(DataSourceFactory.ConvertValue<{et}>(cell)); }} {obj}.{fn} = list; }}\n",
                et = enum_ty,
                fn = field_name,
                obj = obj_expr,
                pref = cprefix
            ));
            return code;
        }
        // list of embedded struct
        if let Some(es) = find_embedded_struct(ctx_struct, inner) {
            // presence check
            let mut sub_headers = Vec::new();
            for it in &es.items { if let StructItem::Field(f2) = it { let mut v2 = visited.clone(); let sub = collect_columns_with(es, &f2.name, &f2.field_type.original, &mut v2, depth + 1, current_ns_name, files); sub_headers.extend(sub); } }
            code.push_str("{ bool any=false; string tmp; ");
            for h in &sub_headers { code.push_str(&format!("if (row.TryGetValue({pref} + \"{field}[0].{}\", out tmp) && !string.IsNullOrEmpty(tmp)) {{ any=true; }} ", h, pref=cprefix, field=field_name)); }
            code.push_str(&format!(
                "if (!any) {{ {obj}.{field} = new List<{owner}.{ename}>(); }} else {{ var sub = new {owner}.{ename}();\n",
                obj=obj_expr, field=field_name, owner=owner_fqn, ename=es.name
            ));
            let next_owner = format!("{}.{}", owner_fqn, es.name);
            for it in &es.items { if let StructItem::Field(f2) = it { code.push_str(&generate_read_assign_for_field(es, f2, "sub", &format!("{pref} + \"{fname}[0].\"", pref=cprefix, fname=field_name), current_ns_name, files, &mut visited.clone(), depth + 1, &next_owner)); } }
            code.push_str(&format!(
                "var list = new List<{owner}.{ename}>(); list.Add(sub); {obj}.{field} = list; }} }}\n",
                owner=owner_fqn, ename=es.name, obj=obj_expr, field=field_name
            ));
            return code;
        }
        // list of external struct
        if let Some((ext, ns_fqn)) = resolve_struct_with_ns(files, inner, current_ns_name) {
            let mut sub_headers = Vec::new();
            for it in &ext.items { if let StructItem::Field(f2) = it { let mut v2 = visited.clone(); let sub = collect_columns_with(ext, &f2.name, &f2.field_type.original, &mut v2, depth + 1, current_ns_name, files); sub_headers.extend(sub); } }
            code.push_str("{ bool any=false; string tmp; ");
            for h in &sub_headers { code.push_str(&format!("if (row.TryGetValue({pref} + \"{field}[0].{}\", out tmp) && !string.IsNullOrEmpty(tmp)) {{ any=true; }} ", h, pref=cprefix, field=field_name)); }
            code.push_str(&format!(
                "if (!any) {{ {obj}.{field} = new List<{ns}.{ty}>(); }} else {{ var list = new List<{ns}.{ty}>(); list.Add({ns}.{ty}Csv.FromRowWithPrefix(row, {pref} + \"{field}[0].\")); {obj}.{field} = list; }} }}\n",
                obj=obj_expr, field=field_name, ns=ns_fqn, ty=ext.name, pref=cprefix
            ));
            return code;
        }
        return code;
    }
    // primitive
    if let Some(p) = map_cs_primitive(&t) {
        code.push_str(&format!(
            "{obj}.{fn} = DataSourceFactory.ConvertSingleValue<{ty}>(row, {pref} + \"{fn}\");\n",
            obj = obj_expr,
            fn = field_name,
            ty = p,
            pref = cprefix
        ));
        return code;
    }
    // enum: inline or named
    if is_inline_enum_name(&t) || resolve_enum(files, &t, current_ns_name).is_some() {
        let enum_ty = if is_inline_enum_name(&t) {
            format!("{}.{}", ctx_struct.name, t)
        } else {
            // best-effort: t may be fully-qualified enum name; use it directly
            t.clone()
        };
        code.push_str(&format!(
            "{obj}.{fn} = DataSourceFactory.ConvertSingleValue<{ety}>(row, {pref} + \"{fn}\");\n",
            obj = obj_expr,
            fn = field_name,
            ety = enum_ty,
            pref = cprefix
        ));
        return code;
    }
    // embedded struct in current ctx
    if let Some(es) = find_embedded_struct(ctx_struct, &t) {
        // determine presence by checking sub headers
        let mut sub_headers = Vec::new();
        for it in &es.items {
            if let StructItem::Field(f2) = it {
                let mut v2 = visited.clone();
                let sub = collect_columns_with(es, &f2.name, &f2.field_type.original, &mut v2, depth + 1, current_ns_name, files);
                sub_headers.extend(sub);
            }
        }
        code.push_str(&format!(
            "{{ bool any=false; string tmp; "));
        for h in &sub_headers {
            code.push_str(&format!(
                "if (row.TryGetValue({pref} + \"{field}.{}\", out tmp) && !string.IsNullOrEmpty(tmp)) {{ any=true; }} ",
                h,
                pref = cprefix,
                field = field_name
            ));
        }
        code.push_str(&format!(
            "if (!any) {{ {obj}.{field} = null; }} else {{ var sub = new {owner}.{ename}();\n",
            obj = obj_expr,
            field = field_name,
            owner = owner_fqn,
            ename = es.name
        ));
        let next_owner = format!("{}.{}", owner_fqn, es.name);
        for it in &es.items { if let StructItem::Field(f2) = it { code.push_str(&generate_read_assign_for_field(es, f2, "sub", &format!("{pref} + \"{fname}.\"", pref=cprefix, fname=field_name), current_ns_name, files, &mut visited.clone(), depth + 1, &next_owner)); } }
        code.push_str(&format!(
            "{obj}.{field} = sub; }} }}\n",
            obj = obj_expr,
            field = field_name
        ));
        return code;
    }
    // external struct
    if let Some((ext, ns_fqn)) = resolve_struct_with_ns(files, &t, current_ns_name) {
        // presence check using ext headers
        let mut sub_headers = Vec::new();
        for it in &ext.items {
            if let StructItem::Field(f2) = it {
                let mut v2 = visited.clone();
                let sub = collect_columns_with(ext, &f2.name, &f2.field_type.original, &mut v2, depth + 1, current_ns_name, files);
                sub_headers.extend(sub);
            }
        }
        code.push_str("{ bool any=false; string tmp; ");
        for h in &sub_headers {
            code.push_str(&format!(
                "if (row.TryGetValue({pref} + \"{field}.{}\", out tmp) && !string.IsNullOrEmpty(tmp)) {{ any=true; }} ",
                h,
                pref = cprefix,
                field = field_name
            ));
        }
        code.push_str(&format!(
            "if (!any) {{ {obj}.{field} = null; }} else {{ {obj}.{field} = {ns}.{ty}Csv.FromRowWithPrefix(row, {pref} + \"{field}.\"); }} }}\n",
            obj = obj_expr,
            field = field_name,
            ns = ns_fqn,
            ty = ext.name,
            pref = cprefix
        ));
        return code;
    }
    // default
    code
}

fn generate_append_code(
    ctx_struct: &StructDef,
    type_string: &str,
    expr_prefix: &str,
    current_ns_name: &str,
    files: &[FileDef],
    visited: &mut Vec<String>,
    depth: usize,
) -> String {
    let mut code = String::new();
    let mut t = unwrap_option(type_string).to_string();
    if depth >= 10 {
        code.push_str("cols.Add(string.Empty);\n");
        return code;
    }
    if let Some(inner) = t.strip_prefix("List<").and_then(|s| s.strip_suffix('>')) {
        code.push_str(&format!(
            "if ({0} != null && {0}.Count > 0) {{\n",
            expr_prefix
        ));
        code.push_str(&generate_append_code(
            ctx_struct,
            inner,
            &format!("{}[0]", expr_prefix),
            current_ns_name,
            files,
            visited,
            depth + 1,
        ));
        code.push_str("} else {\n");
        let tmp_headers = collect_columns_with(ctx_struct, "", inner, &mut visited.clone(), depth + 1, current_ns_name, files);
        for _ in tmp_headers {
            code.push_str("cols.Add(string.Empty);\n");
        }
        code.push_str("}\n");
        return code;
    }
    if is_primitive_like(&t) {
        code.push_str(&format!(
            "cols.Add(CsvUtils.ToStringInvariant({}));\n",
            expr_prefix
        ));
        return code;
    }
    if t.ends_with("__Enum") || resolve_enum(files, &t, current_ns_name).is_some() {
        code.push_str(&format!("cols.Add(({}).ToString());\n", expr_prefix));
        return code;
    }
    if let Some(es) = find_embedded_struct(ctx_struct, &t) {
        // compute column count by flattening es fields
        let mut es_headers = Vec::new();
        for it in &es.items {
            if let StructItem::Field(f) = it {
                let mut v2 = visited.clone();
                let sub = collect_columns_with(es, &f.name, &f.field_type.original, &mut v2, depth + 1, current_ns_name, files);
                es_headers.extend(sub);
            }
        }
        let count = es_headers.len();
        code.push_str(&format!(
            "if ({0} == null) {{ for (int i=0;i< {1}; i++) cols.Add(string.Empty); }} else {{\n",
            expr_prefix, count
        ));
        for it in &es.items {
            if let StructItem::Field(f) = it {
                code.push_str(&generate_append_code(
                    es,
                    &f.field_type.original,
                    &format!("{}.{}", expr_prefix, f.name),
                    current_ns_name,
                    files,
                    &mut visited.clone(),
                    depth + 1,
                ));
            }
        }
        code.push_str("}\n");
        return code;
    }
    if let Some((ext, ns_fqn)) = resolve_struct_with_ns(files, &t, current_ns_name) {
        if visited.iter().any(|v| v == &ext.name) {
            code.push_str(&format!(
                "for (int i=0;i< {0}.{1}Csv.ColumnCount_{1}(); i++) cols.Add(string.Empty);\n",
                ns_fqn, ext.name
            ));
            return code;
        }
        // We do not know the exact namespace here; ColumnCount/AppendRow should be resolved by using or fully-qualified.
        code.push_str(&format!(
            "if ({0} == null) {{ for (int i=0;i< {1}.{2}Csv.ColumnCount_{2}(); i++) cols.Add(string.Empty); }} else {{ {1}.{2}Csv.AppendRow({0}, cols); }}\n",
            expr_prefix, ns_fqn, ext.name
        ));
        return code;
    }
    code.push_str("cols.Add(string.Empty);\n");
    code
}
