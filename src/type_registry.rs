//! Type Registry for centralized type management.
//!
//! This module provides a central registry for tracking all types (enums, structs, embeds)
//! defined in a schema. It replaces the ad-hoc HashSet/HashMap usage in `ir_builder.rs`
//! and `validation.rs` with a unified, reusable structure.

use std::collections::HashMap;

/// Represents the kind of a type definition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeKind {
    Enum,
    Struct,
    Embed,
}

/// Information about a registered type.
#[derive(Debug, Clone)]
pub struct TypeInfo {
    /// Fully qualified name (e.g., "game.common.Status")
    pub fqn: String,
    /// The kind of this type
    pub kind: TypeKind,
    /// Namespace portion of the FQN (e.g., "game.common")
    pub namespace: String,
    /// Simple name without namespace (e.g., "Status")
    pub name: String,
}

/// Central registry for all type definitions in a schema.
///
/// Provides efficient lookups by FQN, name, and namespace.
#[derive(Debug, Default)]
pub struct TypeRegistry {
    /// FQN -> TypeInfo mapping
    types: HashMap<String, TypeInfo>,
    /// Simple name -> list of FQNs (for types with same name in different namespaces)
    by_name: HashMap<String, Vec<String>>,
    /// Namespace -> list of FQNs
    by_namespace: HashMap<String, Vec<String>>,
}

impl TypeRegistry {
    /// Creates a new empty TypeRegistry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a new type in the registry.
    ///
    /// # Arguments
    /// * `fqn` - Fully qualified name of the type
    /// * `kind` - The kind of type (Enum, Struct, Embed)
    ///
    /// # Returns
    /// `true` if the type was newly registered, `false` if it already existed.
    pub fn register(&mut self, fqn: &str, kind: TypeKind) -> bool {
        if self.types.contains_key(fqn) {
            return false;
        }

        let namespace = namespace_of(fqn);
        let name = last_segment(fqn);

        let info = TypeInfo {
            fqn: fqn.to_string(),
            kind,
            namespace: namespace.to_string(),
            name: name.to_string(),
        };

        self.types.insert(fqn.to_string(), info);
        self.by_name
            .entry(name.to_string())
            .or_default()
            .push(fqn.to_string());
        self.by_namespace
            .entry(namespace.to_string())
            .or_default()
            .push(fqn.to_string());

        true
    }

    /// Gets type information by FQN.
    pub fn get(&self, fqn: &str) -> Option<&TypeInfo> {
        self.types.get(fqn)
    }

    /// Finds all types with the given simple name.
    ///
    /// Returns an empty slice if no types match.
    pub fn find_by_name(&self, name: &str) -> &[String] {
        self.by_name.get(name).map(|v| v.as_slice()).unwrap_or(&[])
    }

    /// Finds all types in the given namespace.
    ///
    /// Returns an empty slice if no types are in the namespace.
    pub fn find_by_namespace(&self, namespace: &str) -> &[String] {
        self.by_namespace
            .get(namespace)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Resolves a type reference to its FQN.
    ///
    /// Resolution strategy:
    /// 1. If `type_ref` is already a valid FQN, return it.
    /// 2. Try qualifying with the current namespace.
    /// 3. If the name is unique across all namespaces, return the unique FQN.
    ///
    /// # Arguments
    /// * `type_ref` - The type reference to resolve (may be simple name or FQN)
    /// * `current_namespace` - The namespace context for resolution
    ///
    /// # Returns
    /// The resolved FQN if found, `None` otherwise.
    pub fn resolve(&self, type_ref: &str, current_namespace: &str) -> Option<String> {
        // Strategy 1: Direct FQN match
        if self.types.contains_key(type_ref) {
            return Some(type_ref.to_string());
        }

        // Strategy 2: Qualify with current namespace
        if !current_namespace.is_empty() {
            let qualified = format!("{}.{}", current_namespace, type_ref);
            if self.types.contains_key(&qualified) {
                return Some(qualified);
            }
        }

        // Strategy 3: Unique name resolution
        let simple_name = last_segment(type_ref);
        if let Some(fqns) = self.by_name.get(simple_name) {
            if fqns.len() == 1 {
                return Some(fqns[0].clone());
            }
        }

        None
    }

    /// Checks if a type exists (by FQN).
    pub fn contains(&self, fqn: &str) -> bool {
        self.types.contains_key(fqn)
    }

    /// Checks if a type is an enum.
    pub fn is_enum(&self, fqn: &str) -> bool {
        self.types
            .get(fqn)
            .map(|info| info.kind == TypeKind::Enum)
            .unwrap_or(false)
    }

    /// Checks if a type is a struct.
    pub fn is_struct(&self, fqn: &str) -> bool {
        self.types
            .get(fqn)
            .map(|info| info.kind == TypeKind::Struct)
            .unwrap_or(false)
    }

    /// Checks if a type is an embed.
    pub fn is_embed(&self, fqn: &str) -> bool {
        self.types
            .get(fqn)
            .map(|info| info.kind == TypeKind::Embed)
            .unwrap_or(false)
    }

    /// Returns the kind of a type, if it exists.
    pub fn get_kind(&self, fqn: &str) -> Option<TypeKind> {
        self.types.get(fqn).map(|info| info.kind)
    }

    /// Returns an iterator over all registered types.
    pub fn iter(&self) -> impl Iterator<Item = &TypeInfo> {
        self.types.values()
    }

    /// Returns the number of registered types.
    pub fn len(&self) -> usize {
        self.types.len()
    }

    /// Returns true if no types are registered.
    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }

    /// Returns all enum FQNs.
    pub fn all_enums(&self) -> Vec<&str> {
        self.types
            .values()
            .filter(|info| info.kind == TypeKind::Enum)
            .map(|info| info.fqn.as_str())
            .collect()
    }

    /// Returns all struct FQNs.
    pub fn all_structs(&self) -> Vec<&str> {
        self.types
            .values()
            .filter(|info| info.kind == TypeKind::Struct)
            .map(|info| info.fqn.as_str())
            .collect()
    }
}

/// Extracts the namespace portion from an FQN.
/// For "game.common.Status", returns "game.common".
/// For "Status", returns "".
fn namespace_of(fqn: &str) -> &str {
    match fqn.rfind('.') {
        Some(i) => &fqn[..i],
        None => "",
    }
}

/// Extracts the last segment (simple name) from an FQN.
/// For "game.common.Status", returns "Status".
/// For "Status", returns "Status".
fn last_segment(fqn: &str) -> &str {
    match fqn.rfind('.') {
        Some(i) => &fqn[i + 1..],
        None => fqn,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_register_and_get() {
        let mut registry = TypeRegistry::new();

        assert!(registry.register("game.common.Status", TypeKind::Enum));
        assert!(!registry.register("game.common.Status", TypeKind::Enum)); // duplicate

        let info = registry.get("game.common.Status").unwrap();
        assert_eq!(info.fqn, "game.common.Status");
        assert_eq!(info.namespace, "game.common");
        assert_eq!(info.name, "Status");
        assert_eq!(info.kind, TypeKind::Enum);
    }

    #[test]
    fn test_global_namespace() {
        let mut registry = TypeRegistry::new();
        registry.register("GlobalEnum", TypeKind::Enum);

        let info = registry.get("GlobalEnum").unwrap();
        assert_eq!(info.namespace, "");
        assert_eq!(info.name, "GlobalEnum");
    }

    #[test]
    fn test_find_by_name() {
        let mut registry = TypeRegistry::new();
        registry.register("game.Status", TypeKind::Enum);
        registry.register("system.Status", TypeKind::Enum);
        registry.register("game.User", TypeKind::Struct);

        let statuses = registry.find_by_name("Status");
        assert_eq!(statuses.len(), 2);
        assert!(statuses.contains(&"game.Status".to_string()));
        assert!(statuses.contains(&"system.Status".to_string()));

        let users = registry.find_by_name("User");
        assert_eq!(users.len(), 1);
        assert_eq!(users[0], "game.User");

        let unknown = registry.find_by_name("Unknown");
        assert!(unknown.is_empty());
    }

    #[test]
    fn test_find_by_namespace() {
        let mut registry = TypeRegistry::new();
        registry.register("game.Status", TypeKind::Enum);
        registry.register("game.User", TypeKind::Struct);
        registry.register("system.Config", TypeKind::Struct);

        let game_types = registry.find_by_namespace("game");
        assert_eq!(game_types.len(), 2);

        let system_types = registry.find_by_namespace("system");
        assert_eq!(system_types.len(), 1);
    }

    #[test]
    fn test_resolve_direct_fqn() {
        let mut registry = TypeRegistry::new();
        registry.register("game.common.Status", TypeKind::Enum);

        let resolved = registry.resolve("game.common.Status", "other");
        assert_eq!(resolved, Some("game.common.Status".to_string()));
    }

    #[test]
    fn test_resolve_with_namespace() {
        let mut registry = TypeRegistry::new();
        registry.register("game.common.Status", TypeKind::Enum);

        let resolved = registry.resolve("Status", "game.common");
        assert_eq!(resolved, Some("game.common.Status".to_string()));
    }

    #[test]
    fn test_resolve_unique_name() {
        let mut registry = TypeRegistry::new();
        registry.register("game.common.UniqueType", TypeKind::Enum);

        // UniqueType is unique, so it should resolve even from different namespace
        let resolved = registry.resolve("UniqueType", "other.namespace");
        assert_eq!(resolved, Some("game.common.UniqueType".to_string()));
    }

    #[test]
    fn test_resolve_ambiguous_name_fails() {
        let mut registry = TypeRegistry::new();
        registry.register("game.Status", TypeKind::Enum);
        registry.register("system.Status", TypeKind::Enum);

        // Status is ambiguous, should not resolve from unrelated namespace
        let resolved = registry.resolve("Status", "other");
        assert_eq!(resolved, None);
    }

    #[test]
    fn test_is_enum_and_is_struct() {
        let mut registry = TypeRegistry::new();
        registry.register("game.Status", TypeKind::Enum);
        registry.register("game.User", TypeKind::Struct);
        registry.register("game.Address", TypeKind::Embed);

        assert!(registry.is_enum("game.Status"));
        assert!(!registry.is_struct("game.Status"));

        assert!(registry.is_struct("game.User"));
        assert!(!registry.is_enum("game.User"));

        assert!(registry.is_embed("game.Address"));
        assert!(!registry.is_struct("game.Address"));
    }

    #[test]
    fn test_get_kind() {
        let mut registry = TypeRegistry::new();
        registry.register("Enum1", TypeKind::Enum);
        registry.register("Struct1", TypeKind::Struct);

        assert_eq!(registry.get_kind("Enum1"), Some(TypeKind::Enum));
        assert_eq!(registry.get_kind("Struct1"), Some(TypeKind::Struct));
        assert_eq!(registry.get_kind("Unknown"), None);
    }

    #[test]
    fn test_all_enums_and_structs() {
        let mut registry = TypeRegistry::new();
        registry.register("Enum1", TypeKind::Enum);
        registry.register("Enum2", TypeKind::Enum);
        registry.register("Struct1", TypeKind::Struct);

        let enums = registry.all_enums();
        assert_eq!(enums.len(), 2);

        let structs = registry.all_structs();
        assert_eq!(structs.len(), 1);
    }

    #[test]
    fn test_namespace_of() {
        assert_eq!(namespace_of("game.common.Status"), "game.common");
        assert_eq!(namespace_of("game.Status"), "game");
        assert_eq!(namespace_of("Status"), "");
    }

    #[test]
    fn test_last_segment() {
        assert_eq!(last_segment("game.common.Status"), "Status");
        assert_eq!(last_segment("game.Status"), "Status");
        assert_eq!(last_segment("Status"), "Status");
    }
}
