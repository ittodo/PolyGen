//! C# CSV Loader Code Generation
//!
//! This module generates C# code for CSV data loaders. It handles the complexity
//! of nested structs, lists, enums, and cross-namespace type references.
//!
//! ## Registered Rhai Functions
//!
//! | Function | Description |
//! |----------|-------------|
//! | `csv_headers_for_struct` | Collects flattened CSV column headers |
//! | `csv_append_code_for` | Generates code to append a value to CSV row |
//! | `csv_read_fields_for_struct` | Generates dictionary-based read code |
//! | `csv_read_fields_for_struct_indexed` | Generates indexed (array-based) read code |
//! | `csv_dynamic_methods_for_struct` | Generates dynamic methods for variable lists |
//!
//! ## Column Flattening
//!
//! Nested structures are flattened with dot notation:
//! ```text
//! struct Person { name: string, address: Address }
//! struct Address { city: string, zip: string }
//!
//! → Headers: ["name", "address.city", "address.zip"]
//! ```
//!
//! Lists use index notation:
//! ```text
//! struct Order { items: List<Item> }
//! struct Item { name: string, qty: i32 }
//!
//! → Headers: ["items[0].name", "items[0].qty"]
//! ```
//!
//! ## Read Modes
//!
//! - **Dictionary-based**: Uses `Dictionary<string, string>` for column lookup
//! - **Indexed**: Uses `string[]` with header index map for faster access
//!
//! ---
//!
//! 이 모듈은 CSV 데이터 로더를 위한 C# 코드를 생성합니다.
//! 중첩 구조체, 리스트, 열거형, 크로스 네임스페이스 타입 참조의 복잡성을 처리합니다.

use crate::ir_model::{FieldDef, FileDef, StructDef, StructItem};
use crate::rhai::common::{resolve_enum, resolve_struct, resolve_struct_with_ns, unwrap_option};
use crate::rhai::csharp::type_mapping::{cs_type_for, is_inline_enum_name, is_primitive_like, map_cs_primitive};
use rhai::{Array, Dynamic, Engine};

/// Registers CSV loader helper functions to the Rhai engine.
///
/// English: Call after `register_core` to enable CSV loader generation in templates.
///
/// 한국어: 템플릿에서 CSV 로더 생성을 활성화하려면 `register_core` 이후에 호출하세요.
pub fn register_csv_loaders(engine: &mut Engine) {
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
            generate_append_code(
                &ctx,
                &type_string,
                &expr_prefix,
                &current_ns_name,
                &files,
                &mut visited,
                depth as usize,
            )
        },
    );

    // Generate C# assignments to read fields of a struct from a row with prefix
    engine.register_fn(
        "csv_read_fields_for_struct",
        |s: StructDef,
         obj_expr: String,
         prefix: String,
         current_ns_name: String,
         all_files_dyn: Array|
         -> String {
            let files = array_to_files(&all_files_dyn);
            generate_read_fields_for_struct(
                &s,
                &obj_expr,
                &prefix,
                &current_ns_name,
                &files,
                &s.name,
            )
        },
    );

    // Indexed (header + row[] + map) variant
    engine.register_fn(
        "csv_read_fields_for_struct_indexed",
        |s: StructDef,
         obj_expr: String,
         prefix_var: String,
         current_ns_name: String,
         all_files_dyn: Array|
         -> String {
            let files = array_to_files(&all_files_dyn);
            generate_read_fields_for_struct_indexed(
                &s,
                &obj_expr,
                &prefix_var,
                &current_ns_name,
                &files,
                &s.name,
            )
        },
    );

    // Dynamic writer helpers
    engine.register_fn(
        "csv_dynamic_methods_for_struct",
        |s: StructDef, current_ns_name: String, all_files_dyn: Array| -> String {
            let files = array_to_files(&all_files_dyn);
            generate_dynamic_methods_for_struct(&s, &current_ns_name, &files)
        },
    );
}

// --- Helper Functions ---

fn array_to_files(arr: &Array) -> Vec<FileDef> {
    arr.iter()
        .filter_map(|d| d.clone().try_cast::<FileDef>())
        .collect()
}

fn find_embedded_struct<'a>(s: &'a StructDef, name: &str) -> Option<&'a StructDef> {
    s.items.iter().find_map(|it| match it {
        StructItem::EmbeddedStruct(es) if es.name == name => Some(es),
        _ => None,
    })
}

fn list_inner_type(type_string: &str) -> Option<&str> {
    type_string
        .strip_prefix("List<")
        .and_then(|s| s.strip_suffix('>'))
}

fn is_list_type(type_string: &str) -> bool {
    list_inner_type(type_string).is_some()
}

// --- Column Collection ---

fn collect_columns_with<'a>(
    ctx_struct: &'a StructDef,
    prefix: &str,
    type_string: &str,
    visited: &mut Vec<String>,
    depth: usize,
    current_ns_name: &str,
    files: &'a [FileDef],
) -> Vec<String> {
    let mut cols = Vec::new();
    let t = unwrap_option(type_string).to_string();
    if depth >= 10 {
        cols.push(prefix.to_string());
        return cols;
    }
    if let Some(inner) = t.strip_prefix("List<").and_then(|s| s.strip_suffix('>')) {
        let np = if prefix.is_empty() {
            "[0]".to_string()
        } else {
            format!("{}[0]", prefix)
        };
        let sub = collect_columns_with(
            ctx_struct,
            &np,
            inner,
            visited,
            depth + 1,
            current_ns_name,
            files,
        );
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
                let np = if prefix.is_empty() {
                    f.name.clone()
                } else {
                    format!("{}.{}", prefix, f.name)
                };
                let mut v2 = visited.clone();
                let sub = collect_columns_with(
                    es,
                    &np,
                    &f.field_type.original,
                    &mut v2,
                    depth + 1,
                    current_ns_name,
                    files,
                );
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
                let np = if prefix.is_empty() {
                    f.name.clone()
                } else {
                    format!("{}.{}", prefix, f.name)
                };
                let mut v2 = visited.clone();
                let sub = collect_columns_with(
                    ext,
                    &np,
                    &f.field_type.original,
                    &mut v2,
                    depth + 1,
                    current_ns_name,
                    files,
                );
                cols.extend(sub);
            }
        }
        return cols;
    }
    cols.push(prefix.to_string());
    cols
}

fn collect_headers_for_struct(
    s: &StructDef,
    current_ns_name: &str,
    files: &[FileDef],
) -> Vec<String> {
    let mut headers = Vec::new();
    for it in &s.items {
        if let StructItem::Field(f) = it {
            let mut visited = vec![s.name.clone()];
            let sub = collect_columns_with(
                s,
                &f.name,
                &f.field_type.original,
                &mut visited,
                0,
                current_ns_name,
                files,
            );
            headers.extend(sub);
        }
    }
    headers
}

// --- Read Code Generation ---

fn generate_read_fields_for_struct(
    s: &StructDef,
    obj_expr: &str,
    cprefix: &str,
    current_ns_name: &str,
    files: &[FileDef],
    owner_fqn: &str,
) -> String {
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

fn generate_read_fields_for_struct_indexed(
    s: &StructDef,
    obj_expr: &str,
    prefix_var: &str,
    current_ns_name: &str,
    files: &[FileDef],
    owner_fqn: &str,
) -> String {
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

#[allow(clippy::too_many_arguments, clippy::ptr_arg)]
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
    let t = unwrap_option(&field.field_type.original).to_string();
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
            code.push_str(&format!(
                "{{ var list = new List<{owner}.{ename}>(); int i=0; for(;;i++) {{ bool any=false; string __tmp; ",
                owner = owner_fqn,
                ename = es.name
            ));
            for it in &es.items {
                if let StructItem::Field(f2) = it {
                    let mut v2 = visited.clone();
                    let tails = collect_columns_with(
                        es,
                        &f2.name,
                        &f2.field_type.original,
                        &mut v2,
                        depth + 1,
                        current_ns_name,
                        files,
                    );
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
                        &format!(
                            "{pref} + \"{fname}[\"+i+\"].\"",
                            pref = prefix_var,
                            fname = field_name
                        ),
                        current_ns_name,
                        files,
                        &mut visited.clone(),
                        depth + 1,
                        &next_owner,
                    ));
                }
            }
            code.push_str(&format!(
                "list.Add(sub); }} {obj}.{fname} = list; }}\n",
                obj = obj_expr,
                fname = field_name
            ));
            return code;
        }
        // external struct list
        if let Some((ext, ns_fqn)) = resolve_struct_with_ns(files, inner, current_ns_name) {
            code.push_str("{ var list = new System.Collections.Generic.List<");
            code.push_str(&format!(
                "{}.{}>(); int i=0; for(;;i++) {{ bool any=false; string __tmp; ",
                ns_fqn, ext.name
            ));
            for it in &ext.items {
                if let StructItem::Field(f2) = it {
                    let mut v2 = visited.clone();
                    let tails = collect_columns_with(
                        ext,
                        &f2.name,
                        &f2.field_type.original,
                        &mut v2,
                        depth + 1,
                        current_ns_name,
                        files,
                    );
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
            t
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
        code.push_str("{ bool any=false; ");
        for it in &es.items {
            if let StructItem::Field(f2) = it {
                let mut v2 = visited.clone();
                let tails = collect_columns_with(
                    es,
                    &f2.name,
                    &f2.field_type.original,
                    &mut v2,
                    depth + 1,
                    current_ns_name,
                    files,
                );
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
            "if (!any) {{ {obj}.{fname} = null; }} else {{ var sub = new {owner}.{ename}(); ",
            obj = obj_expr,
            fname = field_name,
            owner = owner_fqn,
            ename = es.name
        ));
        let next_owner = format!("{}.{}", owner_fqn, es.name);
        for it in &es.items {
            if let StructItem::Field(f2) = it {
                code.push_str(&gen_read_assign_indexed(
                    es,
                    f2,
                    "sub",
                    &format!(
                        "{pref} + \"{fname}.\"",
                        pref = prefix_var,
                        fname = field_name
                    ),
                    current_ns_name,
                    files,
                    &mut visited.clone(),
                    depth + 1,
                    &next_owner,
                ));
            }
        }
        code.push_str(&format!(
            "{obj}.{fname} = sub; }} }}\n",
            obj = obj_expr,
            fname = field_name
        ));
        return code;
    }
    // external struct single
    if let Some((ext, ns_fqn)) = resolve_struct_with_ns(files, &t, current_ns_name) {
        code.push_str("{ bool any=false; ");
        for it in &ext.items {
            if let StructItem::Field(f2) = it {
                let mut v2 = visited.clone();
                let tails = collect_columns_with(
                    ext,
                    &f2.name,
                    &f2.field_type.original,
                    &mut v2,
                    depth + 1,
                    current_ns_name,
                    files,
                );
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

#[allow(clippy::too_many_arguments, clippy::ptr_arg)]
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
    let t = unwrap_option(&field.field_type.original).to_string();
    if depth >= 10 {
        return code;
    }
    if let Some(inner) = t.strip_prefix("List<").and_then(|s| s.strip_suffix('>')) {
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
            let mut sub_headers = Vec::new();
            for it in &es.items {
                if let StructItem::Field(f2) = it {
                    let mut v2 = visited.clone();
                    let sub = collect_columns_with(
                        es,
                        &f2.name,
                        &f2.field_type.original,
                        &mut v2,
                        depth + 1,
                        current_ns_name,
                        files,
                    );
                    sub_headers.extend(sub);
                }
            }
            code.push_str("{ bool any=false; string tmp; ");
            for h in &sub_headers {
                code.push_str(&format!("if (row.TryGetValue({pref} + \"{field}[0].{}\", out tmp) && !string.IsNullOrEmpty(tmp)) {{ any=true; }} ", h, pref=cprefix, field=field_name));
            }
            code.push_str(&format!(
                "if (!any) {{ {obj}.{field} = new List<{owner}.{ename}>(); }} else {{ var sub = new {owner}.{ename}();\n",
                obj=obj_expr, field=field_name, owner=owner_fqn, ename=es.name
            ));
            let next_owner = format!("{}.{}", owner_fqn, es.name);
            for it in &es.items {
                if let StructItem::Field(f2) = it {
                    code.push_str(&generate_read_assign_for_field(
                        es,
                        f2,
                        "sub",
                        &format!(
                            "{pref} + \"{fname}[0].\"",
                            pref = cprefix,
                            fname = field_name
                        ),
                        current_ns_name,
                        files,
                        &mut visited.clone(),
                        depth + 1,
                        &next_owner,
                    ));
                }
            }
            code.push_str(&format!(
                "var list = new List<{owner}.{ename}>(); list.Add(sub); {obj}.{field} = list; }} }}\n",
                owner=owner_fqn, ename=es.name, obj=obj_expr, field=field_name
            ));
            return code;
        }
        // list of external struct
        if let Some((ext, ns_fqn)) = resolve_struct_with_ns(files, inner, current_ns_name) {
            let mut sub_headers = Vec::new();
            for it in &ext.items {
                if let StructItem::Field(f2) = it {
                    let mut v2 = visited.clone();
                    let sub = collect_columns_with(
                        ext,
                        &f2.name,
                        &f2.field_type.original,
                        &mut v2,
                        depth + 1,
                        current_ns_name,
                        files,
                    );
                    sub_headers.extend(sub);
                }
            }
            code.push_str("{ bool any=false; string tmp; ");
            for h in &sub_headers {
                code.push_str(&format!("if (row.TryGetValue({pref} + \"{field}[0].{}\", out tmp) && !string.IsNullOrEmpty(tmp)) {{ any=true; }} ", h, pref=cprefix, field=field_name));
            }
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
    // enum
    if is_inline_enum_name(&t) || resolve_enum(files, &t, current_ns_name).is_some() {
        let enum_ty = if is_inline_enum_name(&t) {
            format!("{}.{}", ctx_struct.name, t)
        } else {
            t
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
    // embedded struct
    if let Some(es) = find_embedded_struct(ctx_struct, &t) {
        let mut sub_headers = Vec::new();
        for it in &es.items {
            if let StructItem::Field(f2) = it {
                let mut v2 = visited.clone();
                let sub = collect_columns_with(
                    es,
                    &f2.name,
                    &f2.field_type.original,
                    &mut v2,
                    depth + 1,
                    current_ns_name,
                    files,
                );
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
            "if (!any) {{ {obj}.{field} = null; }} else {{ var sub = new {owner}.{ename}();\n",
            obj = obj_expr,
            field = field_name,
            owner = owner_fqn,
            ename = es.name
        ));
        let next_owner = format!("{}.{}", owner_fqn, es.name);
        for it in &es.items {
            if let StructItem::Field(f2) = it {
                code.push_str(&generate_read_assign_for_field(
                    es,
                    f2,
                    "sub",
                    &format!("{pref} + \"{fname}.\"", pref = cprefix, fname = field_name),
                    current_ns_name,
                    files,
                    &mut visited.clone(),
                    depth + 1,
                    &next_owner,
                ));
            }
        }
        code.push_str(&format!(
            "{obj}.{field} = sub; }} }}\n",
            obj = obj_expr,
            field = field_name
        ));
        return code;
    }
    // external struct
    if let Some((ext, ns_fqn)) = resolve_struct_with_ns(files, &t, current_ns_name) {
        let mut sub_headers = Vec::new();
        for it in &ext.items {
            if let StructItem::Field(f2) = it {
                let mut v2 = visited.clone();
                let sub = collect_columns_with(
                    ext,
                    &f2.name,
                    &f2.field_type.original,
                    &mut v2,
                    depth + 1,
                    current_ns_name,
                    files,
                );
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
    code
}

// --- Write/Append Code Generation ---

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
    let t = unwrap_option(type_string).to_string();
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
        let tmp_headers = collect_columns_with(
            ctx_struct,
            "",
            inner,
            &mut visited.clone(),
            depth + 1,
            current_ns_name,
            files,
        );
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
        let mut es_headers = Vec::new();
        for it in &es.items {
            if let StructItem::Field(f) = it {
                let mut v2 = visited.clone();
                let sub = collect_columns_with(
                    es,
                    &f.name,
                    &f.field_type.original,
                    &mut v2,
                    depth + 1,
                    current_ns_name,
                    files,
                );
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
        code.push_str(&format!(
            "if ({0} == null) {{ for (int i=0;i< global::{1}.{2}Csv.ColumnCount_{2}(); i++) cols.Add(string.Empty); }} else {{ global::{1}.{2}Csv.AppendRow({0}, cols); }}\n",
            expr_prefix, ns_fqn, ext.name
        ));
        return code;
    }
    code.push_str("cols.Add(string.Empty);\n");
    code
}

// --- Dynamic Methods Generation ---

fn generate_dynamic_methods_for_struct(
    s: &StructDef,
    current_ns_name: &str,
    files: &[FileDef],
) -> String {
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
                let non_cols = collect_columns_with(
                    s,
                    &f.name,
                    &f.field_type.original,
                    &mut visited,
                    0,
                    current_ns_name,
                    files,
                );
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
                let tails =
                    collect_columns_with(s, "", inner, &mut visited, 1, current_ns_name, files);
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
                let tails =
                    collect_columns_with(s, "", inner, &mut visited, 1, current_ns_name, files);
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
