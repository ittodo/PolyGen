use crate::ir_model::{FieldDef, TypeRef};
use rhai::Engine;

pub fn register_csharp(engine: &mut Engine) {
    engine.register_fn("cs_csv_header_name", cs_csv_header_name);
    engine.register_fn("cs_write_csv_expr", cs_write_csv_expr);
}

/// Generates the CSV header name for a field.
/// e.g., "tags" -> "tags[0]" if it's a list.
fn cs_csv_header_name(field: &mut FieldDef) -> String {
    let base_name = &field.name;
    if field.field_type.is_list {
        format!("{}[0]", base_name)
    } else {
        base_name.clone()
    }
}

/// Generates the C# expression to write a field to the CSV row list.
/// e.g., `cols.Add(obj.name);` or `cols.Add(CsvUtils.ToStringInvariant(obj.score));`
fn cs_write_csv_expr(field: &mut FieldDef, obj_var: &str) -> String {
    let field_name = &field.name;
    let access = format!("{}.{}", obj_var, field_name);
    let type_ref = &field.field_type;

    generate_write_logic(type_ref, &access)
}

fn generate_write_logic(t: &TypeRef, access: &str) -> String {
    if t.is_list {
        // List handling: check count and access [0]
        // If list is empty or null, write empty string.
        let inner = t.inner_type.as_ref().unwrap();
        let inner_access = format!("{}[0]", access);
        let inner_write = generate_value_write(inner, &inner_access);
        
        format!(
            "if ({access} != null && {access}.Count > 0) {{ {inner_write} }} else {{ cols.Add(string.Empty); }}"
        )
    } else if t.is_option {
        // Option handling: check null (if ref type) or HasValue (if struct)
        // But for simplicity in generated code, we often just check null or let CsvUtils handle it.
        // However, `TypeRef` doesn't strictly know if it's a C# struct or class without more info.
        // We'll assume standard null checks work for most things or use CsvUtils.
        
        let inner = t.inner_type.as_ref().unwrap();
        // For Option, we can try to unwrap or just pass to CsvUtils if it handles nullable.
        // Let's recurse but note that 'access' is the nullable value.
        
        // If it's a primitive option, CsvUtils.ToStringInvariant handles T? usually?
        // Let's look at how we want to write it.
        // If it's `int?`, `ToStringInvariant` might need an overload or we check `.HasValue`.
        
        // Simple approach: Use CsvUtils.ToStringInvariant which should handle nulls for objects.
        // For nullable structs, we might need `.Value`.
        // Let's stick to the previous logic pattern:
        // if (val != null) add(val); else add("");
        
        let _inner_write = generate_value_write(inner, access); // Recurse? No, inner logic might assume non-null.
        
        // Actually, let's just use the value write logic directly on the nullable type
        // if CsvUtils supports it.
        generate_value_write(t, access)
    } else {
        generate_value_write(t, access)
    }
}

fn generate_value_write(t: &TypeRef, access: &str) -> String {
    if t.is_primitive {
        if t.lang_type == "string" {
             // String: handle null
             format!("cols.Add(CsvUtils.Escape({} ?? string.Empty));", access)
        } else {
            // Numeric/Bool: use ToStringInvariant
            format!("cols.Add(CsvUtils.ToStringInvariant({}));", access)
        }
    } else if t.is_enum {
        // Enum: .ToString()
        format!("cols.Add({}.ToString());", access)
    } else if t.is_option {
        // Option (recursive case from list/option handling)
        // If it's `Option<int>`, access is `int?`.
        // `CsvUtils.ToStringInvariant` should ideally handle `object` or `IFormattable`.
        // Let's assume `CsvUtils.ToStringInvariant` is robust.
        format!("cols.Add(CsvUtils.ToStringInvariant({}));", access)
    } else {
        // Struct or unknown: empty placeholder for now as per previous logic
        "cols.Add(string.Empty);".to_string()
    }
}
