${
    // Unreal namespace generator template
    // Note: USTRUCT/UENUM must be at global scope for Unreal reflection
    // Namespaces are flattened - types get prefixes based on namespace

    let struct_tmpl = include("templates/unreal/unreal_struct.rhai");
    let mod_tmpl = include("templates/unreal/unreal_mod.rhai");

    let result = "";

    // Current namespace FQN for type resolution
    let current_ns = ns.name;

    // Add namespace comment for organization
    if ns.name != "" {
        result += "// ============================================================================\n";
        result += "// Namespace: " + ns.name + "\n";
        result += "// ============================================================================\n\n";
    }

    // Process items in the namespace
    // Order: Enums first, then Structs (for forward declaration purposes)

    // First pass: Generate standalone enums (inline to avoid scope issues)
    for item in ns.items {
        if item.is_enum() {
            let e = item.as_enum();

            // Add E prefix for Unreal enum naming convention
            let enum_name = "E" + e.name;

            result += "UENUM(BlueprintType)\n";
            result += "enum class " + enum_name + " : uint8\n";
            result += "{\n";

            for enum_item in e.items {
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
        } else if item.is_comment() {
            let c = item.as_comment();
            result += "// " + c + "\n";
        }
    }

    // Second pass: Generate structs (which may contain inline enums)
    for item in ns.items {
        if item.is_struct() {
            let s = item.as_struct();
            let res = eval("let current_ns = \"" + current_ns + "\";\n`" + struct_tmpl + "`");
            result += res + "\n";
        }
    }

    // Third pass: Handle nested namespaces
    for item in ns.items {
        if item.is_namespace() {
            let child = item.as_namespace();
            let res = eval("let ns = child;\n`" + mod_tmpl + "`");
            result += res;
        }
    }

    result
}
