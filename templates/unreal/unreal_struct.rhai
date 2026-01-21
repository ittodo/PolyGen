${
    // Unreal USTRUCT generator template
    // Expects 's' to be a StructDef object and 'current_ns' to be set

    // Main generation
    let ns_fqn = if current_ns != () { current_ns } else { "" };
    let result = "";

    // Add F prefix for struct name
    let base_struct_name = s.name;
    let first_char = base_struct_name.sub_string(0, 1);
    let struct_name = if first_char == "F" { base_struct_name } else { "F" + base_struct_name };

    // Generate embedded structs FIRST (they must be declared before parent struct uses them)
    for item in s.items {
        if item.is_embedded_struct() {
            let es = item.as_embedded_struct();
            let struct_tmpl = include("templates/unreal/unreal_struct.rhai");
            let res = eval("let s = es; let current_ns = \"" + ns_fqn + "\";\n`" + struct_tmpl + "`");
            result += res + "\n";
        }
    }

    // Generate inline enums FIRST (they must be declared before the struct that uses them)
    for item in s.items {
        if item.is_inline_enum() {
            let ie = item.as_inline_enum();

            // Add E prefix for enum name
            let base_enum_name = ie.name;
            let enum_first_char = base_enum_name.sub_string(0, 1);
            let enum_name = if enum_first_char == "E" { base_enum_name } else { "E" + base_enum_name };

            result += "UENUM(BlueprintType)\n";
            result += "enum class " + enum_name + " : uint8\n";
            result += "{\n";

            for enum_item in ie.items {
                if enum_item.is_member() {
                    let m = enum_item.as_member();
                    if m.value != () {
                        result += "    " + m.name + " = " + m.value + " UMETA(DisplayName = \"" + m.name + "\"),\n";
                    } else {
                        result += "    " + m.name + " UMETA(DisplayName = \"" + m.name + "\"),\n";
                    }
                } else if enum_item.is_comment() {
                    result += "    // " + enum_item.as_comment() + "\n";
                }
            }

            result += "};\n\n";
        }
    }

    // Generate doc comments from header
    for item in s.header {
        if item.is_comment() {
            result += "/** " + item.as_comment() + " */\n";
        }
    }

    // USTRUCT declaration with BlueprintType
    result += "USTRUCT(BlueprintType)\n";
    result += "struct " + struct_name + "\n";
    result += "{\n";
    result += "    GENERATED_BODY()\n\n";

    // Generate fields with UPROPERTY
    for item in s.items {
        if item.is_field() {
            let f = item.as_field();
            let t = f.field_type;

            // Inline type mapping
            let ue_type = "";
            let type_name = t.type_name;

            // Check if primitive
            let is_prim = false;
            if type_name == "string" || type_name == "bool" || type_name == "bytes" ||
               type_name == "u8" || type_name == "u16" || type_name == "u32" || type_name == "u64" ||
               type_name == "i8" || type_name == "i16" || type_name == "i32" || type_name == "i64" ||
               type_name == "f32" || type_name == "f64" {
                is_prim = true;
            }

            if t.is_primitive || is_prim {
                // Map primitive to Unreal type
                if type_name == "string" { ue_type = "FString"; }
                else if type_name == "bool" { ue_type = "bool"; }
                else if type_name == "u8" { ue_type = "uint8"; }
                else if type_name == "u16" { ue_type = "uint16"; }
                else if type_name == "u32" { ue_type = "uint32"; }
                else if type_name == "u64" { ue_type = "uint64"; }
                else if type_name == "i8" { ue_type = "int8"; }
                else if type_name == "i16" { ue_type = "int16"; }
                else if type_name == "i32" { ue_type = "int32"; }
                else if type_name == "i64" { ue_type = "int64"; }
                else if type_name == "f32" { ue_type = "float"; }
                else if type_name == "f64" { ue_type = "double"; }
                else if type_name == "bytes" { ue_type = "TArray<uint8>"; }
                else { ue_type = type_name; }
            } else {
                // Custom type - determine if enum or struct
                let type_first = type_name.sub_string(0, 1);

                // Check if enum by is_enum flag or common enum patterns
                // Always add E prefix for known enum names, F prefix otherwise
                if t.is_enum || type_name == "Element" || type_name == "Status" || type_name == "ItemType" ||
                   type_name == "PlayerClass" || type_name == "Status__Enum" {
                    ue_type = "E" + type_name;
                } else if type_name.contains("Type") || type_name.contains("Status") || type_name.contains("Kind") ||
                          type_name.contains("Mode") || type_name.contains("State") {
                    ue_type = "E" + type_name;
                } else {
                    // Struct type - add F prefix if not already present
                    if type_first == "F" {
                        ue_type = type_name;
                    } else {
                        ue_type = "F" + type_name;
                    }
                }
            }

            // Apply array wrapper
            if t.is_list {
                ue_type = "TArray<" + ue_type + ">";
            }

            // UPROPERTY macro with specifiers
            let uprop_spec = "EditAnywhere, BlueprintReadWrite, Category = \"Data\"";

            // Handle optional fields with meta
            if t.is_option {
                uprop_spec += ", meta = (Optional)";
            }

            result += "    UPROPERTY(" + uprop_spec + ")\n";
            result += "    " + ue_type + " " + f.name;

            // Default value (only for primitives, arrays auto-initialize)
            if !t.is_list && !t.is_option {
                let default_val = "";
                if t.is_primitive || is_prim {
                    if type_name == "string" { default_val = "TEXT(\"\")"; }
                    else if type_name == "bool" { default_val = "false"; }
                    else if type_name == "u8" || type_name == "u16" || type_name == "u32" || type_name == "u64" ||
                            type_name == "i8" || type_name == "i16" || type_name == "i32" || type_name == "i64" {
                        default_val = "0";
                    }
                    else if type_name == "f32" || type_name == "f64" { default_val = "0.0f"; }
                }
                if default_val != "" {
                    result += " = " + default_val;
                }
            }

            result += ";\n\n";
        } else if item.is_comment() {
            result += "    // " + item.as_comment() + "\n";
        }
    }

    // Default constructor
    result += "    " + struct_name + "() = default;\n";

    result += "};\n";

    result
}
