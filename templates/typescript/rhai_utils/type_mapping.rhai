// TypeScript type mapping utilities

/// Map primitive types to TypeScript types
fn map_primitive(type_name) {
    switch type_name {
        "u8" => "number",
        "u16" => "number",
        "u32" => "number",
        "u64" => "number",
        "i8" => "number",
        "i16" => "number",
        "i32" => "number",
        "i64" => "number",
        "f32" => "number",
        "f64" => "number",
        "bool" => "boolean",
        "string" => "string",
        "bytes" => "Uint8Array",
        _ => ()
    }
}

/// Convert namespace FQN to TypeScript namespace path with dots
/// "game" -> "Game"
/// "game.character" -> "Game.GameCharacter"
/// "game.common" -> "Game.GameCommon"
fn ns_to_ts_path(ns_fqn) {
    let parts = ns_fqn.split(".");
    let result = "";
    let cumulative = "";
    for i in 0..parts.len {
        if i > 0 {
            result += ".";
            cumulative += "_";
        }
        cumulative += parts[i];
        // Each level is named after the full path up to that point
        // Use underscore to preserve word boundaries for PascalCase
        result += to_pascal_case(cumulative);
    }
    result
}

/// Extract the actual namespace from an FQN that might include struct/embed names
/// "game.character" -> "game.character"
/// "game.character.Player" -> "game.character" (Player is a struct)
/// "test.embed.Product.Dimensions" -> "test.embed" (Product and Dimensions are structs/embeds)
fn get_actual_namespace(fqn) {
    // Inline types have FQN like "namespace.StructName.EmbedName..."
    // We need to remove all struct/embed names (which start with uppercase)
    // to get the actual poly namespace

    let parts = fqn.split(".");
    if parts.len <= 1 {
        return fqn;
    }

    // Find the first part that starts with uppercase (struct/embed name)
    // Everything before that is the actual namespace
    let result = "";
    for i in 0..parts.len {
        let part = parts[i];
        if part.len > 0 {
            let first_char = part.sub_string(0, 1);
            // If it starts with uppercase, this is a struct/embed name - stop here
            if first_char == first_char.to_upper() {
                break;
            }
            // Add to namespace
            if result != "" {
                result += ".";
            }
            result += part;
        }
    }

    // If no namespace found (all parts are uppercase), return empty
    if result == "" {
        return "";
    }
    result
}

/// Compute relative namespace path from current_ns to ref_ns
/// Returns the TypeScript namespace prefix to use
fn compute_ns_prefix(current_ns, ref_ns) {
    if current_ns == ref_ns || ref_ns == "" {
        return "";
    }

    // Get actual namespace (strip struct name if present for inline types)
    let actual_ref_ns = get_actual_namespace(ref_ns);

    // If same actual namespace, no prefix needed
    if current_ns == actual_ref_ns {
        return "";
    }

    // For cross-namespace references, use the full namespace path
    ns_to_ts_path(actual_ref_ns) + "."
}

/// Map a TypeRef to its TypeScript type string
fn map_type(type_ref) {
    let base_type = "";

    // Check if it's a primitive type
    let primitive = map_primitive(type_ref.type_name);
    if primitive != () {
        base_type = primitive;
    } else {
        // Custom type - use the type name directly
        base_type = type_ref.type_name;
    }

    // Handle cardinality
    if type_ref.is_list {
        if type_ref.is_option {
            return base_type + "[] | null";
        } else {
            return base_type + "[]";
        }
    } else if type_ref.is_option {
        return base_type + " | null";
    }

    base_type
}

/// Map a TypeRef with namespace context
fn map_type_with_ns(type_ref, current_ns) {
    let base_type = "";

    // Check if it's a primitive type
    let primitive = map_primitive(type_ref.type_name);
    if primitive != () {
        base_type = primitive;
    } else if type_ref.is_enum || type_ref.is_struct {
        // For custom types, check if it's from a different namespace
        let ref_ns = "";
        if type_ref.namespace_fqn != () && type_ref.namespace_fqn != "" {
            ref_ns = type_ref.namespace_fqn;
        }

        // Add namespace prefix if from different namespace
        let prefix = compute_ns_prefix(current_ns, ref_ns);
        base_type = prefix + type_ref.type_name;
    } else {
        base_type = type_ref.type_name;
    }

    // Handle cardinality
    if type_ref.is_list {
        if type_ref.is_option {
            return base_type + "[] | null";
        } else {
            return base_type + "[]";
        }
    } else if type_ref.is_option {
        return base_type + " | null";
    }

    base_type
}

/// Map type to Zod schema with namespace context
fn map_type_to_zod_with_ns(type_ref, current_ns) {
    let base_schema = "";

    // Primitive types
    switch type_ref.type_name {
        "u8" => { base_schema = "z.number().int().min(0).max(255)"; }
        "u16" => { base_schema = "z.number().int().min(0).max(65535)"; }
        "u32" => { base_schema = "z.number().int().min(0)"; }
        "u64" => { base_schema = "z.number().int().min(0)"; }
        "i8" => { base_schema = "z.number().int().min(-128).max(127)"; }
        "i16" => { base_schema = "z.number().int().min(-32768).max(32767)"; }
        "i32" => { base_schema = "z.number().int()"; }
        "i64" => { base_schema = "z.number().int()"; }
        "f32" => { base_schema = "z.number()"; }
        "f64" => { base_schema = "z.number()"; }
        "bool" => { base_schema = "z.boolean()"; }
        "string" => { base_schema = "z.string()"; }
        "bytes" => { base_schema = "z.instanceof(Uint8Array)"; }
        _ => {
            // Custom type - reference its schema with namespace prefix
            let ref_ns = "";
            if type_ref.namespace_fqn != () && type_ref.namespace_fqn != "" {
                ref_ns = type_ref.namespace_fqn;
            }

            let prefix = compute_ns_prefix(current_ns, ref_ns);
            base_schema = prefix + type_ref.type_name + "Schema";
        }
    }

    // Handle cardinality
    if type_ref.is_list {
        if type_ref.is_option {
            return "z.array(" + base_schema + ").nullable()";
        } else {
            return "z.array(" + base_schema + ")";
        }
    } else if type_ref.is_option {
        return base_schema + ".nullable()";
    }

    base_schema
}

/// Map type to Zod schema (without namespace - legacy)
fn map_type_to_zod(type_ref) {
    map_type_to_zod_with_ns(type_ref, "")
}

/// Map a field to Zod schema with all constraints
fn map_field_to_zod_with_constraints(field, current_ns) {
    let type_ref = field.field_type;
    let base_schema = "";

    // Primitive types with constraint support
    switch type_ref.type_name {
        "u8" => { base_schema = "z.number().int().min(0).max(255)"; }
        "u16" => { base_schema = "z.number().int().min(0).max(65535)"; }
        "u32" => { base_schema = "z.number().int().min(0)"; }
        "u64" => { base_schema = "z.number().int().min(0)"; }
        "i8" => { base_schema = "z.number().int().min(-128).max(127)"; }
        "i16" => { base_schema = "z.number().int().min(-32768).max(32767)"; }
        "i32" => { base_schema = "z.number().int()"; }
        "i64" => { base_schema = "z.number().int()"; }
        "f32" => { base_schema = "z.number()"; }
        "f64" => { base_schema = "z.number()"; }
        "bool" => { base_schema = "z.boolean()"; }
        "string" => { base_schema = "z.string()"; }
        "bytes" => { base_schema = "z.instanceof(Uint8Array)"; }
        _ => {
            // Custom type - reference its schema with namespace prefix
            let ref_ns = "";
            if type_ref.namespace_fqn != () && type_ref.namespace_fqn != "" {
                ref_ns = type_ref.namespace_fqn;
            }

            let prefix = compute_ns_prefix(current_ns, ref_ns);
            base_schema = prefix + type_ref.type_name + "Schema";
        }
    }

    // Add field-level constraints for string type
    if type_ref.type_name == "string" {
        // MaxLength constraint
        if field.max_length != () {
            base_schema += ".max(" + field.max_length + ")";
        }

        // Regex constraint
        if field.regex_pattern != () {
            // Escape backslashes for JavaScript regex
            let pattern = field.regex_pattern;
            base_schema += ".regex(/" + pattern + "/)";
        }
    }

    // Add field-level Range constraint for numeric types
    if field.range != () {
        let is_numeric = type_ref.type_name == "u8" || type_ref.type_name == "u16" ||
                         type_ref.type_name == "u32" || type_ref.type_name == "u64" ||
                         type_ref.type_name == "i8" || type_ref.type_name == "i16" ||
                         type_ref.type_name == "i32" || type_ref.type_name == "i64" ||
                         type_ref.type_name == "f32" || type_ref.type_name == "f64";
        if is_numeric {
            base_schema += ".min(" + field.range.min + ").max(" + field.range.max + ")";
        }
    }

    // Add default value if specified
    if field.default_value != () {
        let default_val = field.default_value;
        let type_name = type_ref.type_name;

        // Format for JavaScript
        if type_name == "bool" {
            default_val = default_val.to_lower();
        } else if type_name == "string" {
            if !default_val.starts_with("\"") && !default_val.starts_with("'") {
                default_val = "'" + default_val + "'";
            }
        }

        base_schema += ".default(" + default_val + ")";
    }

    // Handle cardinality
    if type_ref.is_list {
        if type_ref.is_option {
            return "z.array(" + base_schema + ").nullable()";
        } else {
            return "z.array(" + base_schema + ")";
        }
    } else if type_ref.is_option {
        return base_schema + ".nullable()";
    }

    base_schema
}
