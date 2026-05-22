use crate::ast_model::BasicType;

pub(super) fn qualify(base_fqn: &str, current_ns: &str) -> String {
    if base_fqn.contains('.') || current_ns.is_empty() {
        base_fqn.to_string()
    } else {
        format!("{}.{}", current_ns, base_fqn)
    }
}

pub(super) fn namespace_of_owned(fqn: &str) -> String {
    match fqn.rfind('.') {
        Some(i) => fqn[..i].to_string(),
        None => String::new(),
    }
}

pub(super) fn last_segment_owned(fqn: &str) -> String {
    match fqn.rfind('.') {
        Some(i) => fqn[i + 1..].to_string(),
        None => fqn.to_string(),
    }
}

/// Extracts the parent type path from an FQN.
/// For "game.character.Monster.DropItems.Enchantment", returns "Monster.DropItems".
/// For top-level types or primitives, returns empty string.
pub(super) fn parent_type_path_of(fqn: &str, namespace_fqn: &str) -> String {
    // If there's no namespace, the type is either primitive or top-level
    if namespace_fqn.is_empty() {
        return String::new();
    }

    // Remove namespace prefix from FQN to get the type path
    let type_path = if fqn.starts_with(namespace_fqn) && fqn.len() > namespace_fqn.len() {
        &fqn[namespace_fqn.len() + 1..] // +1 to skip the dot
    } else {
        fqn
    };

    // Find the last dot in the type path
    match type_path.rfind('.') {
        Some(i) => type_path[..i].to_string(),
        None => String::new(), // Top-level type in namespace
    }
}

pub(super) fn basic_name(b: &BasicType) -> &'static str {
    match b {
        BasicType::String => "string",
        BasicType::I8 => "i8",
        BasicType::I16 => "i16",
        BasicType::I32 => "i32",
        BasicType::I64 => "i64",
        BasicType::U8 => "u8",
        BasicType::U16 => "u16",
        BasicType::U32 => "u32",
        BasicType::U64 => "u64",
        BasicType::F32 => "f32",
        BasicType::F64 => "f64",
        BasicType::Bool => "bool",
        BasicType::Bytes => "bytes",
        BasicType::Timestamp => "timestamp",
    }
}
