use crate::ast_model::{self, Definition, Metadata, TableMember};
use crate::ir_model::{
    self, AnnotationDef, AnnotationParam, EnumDef, EnumItem, FieldDef, FileDef, ForeignKeyDef,
    IndexDef, NamespaceDef, NamespaceItem, RelationDef, RenameInfo, RenameKind, SchemaContext,
    StructDef, StructItem, TypeRef,
};
use crate::type_registry::{TypeKind, TypeRegistry};
use heck::ToPascalCase;

/// Extracts the @datasource value from metadata.
fn extract_datasource(metadata: &[Metadata]) -> Option<String> {
    extract_annotation_string(metadata, "datasource")
}

/// Extracts the @cache strategy from metadata.
fn extract_cache_strategy(metadata: &[Metadata]) -> Option<String> {
    extract_annotation_string(metadata, "cache")
}

/// Extracts the @soft_delete field name from metadata.
fn extract_soft_delete_field(metadata: &[Metadata]) -> Option<String> {
    extract_annotation_string(metadata, "soft_delete")
}

/// Checks if @readonly annotation is present.
fn is_readonly(metadata: &[Metadata]) -> bool {
    for meta in metadata {
        if let Metadata::Annotation(ann) = meta {
            if ann.name.as_deref() == Some("readonly") {
                return true;
            }
        }
    }
    false
}

/// Helper to extract a string value from an annotation.
fn extract_annotation_string(metadata: &[Metadata], annotation_name: &str) -> Option<String> {
    for meta in metadata {
        if let Metadata::Annotation(ann) = meta {
            if ann.name.as_deref() == Some(annotation_name) {
                // Get the first positional or the value parameter
                if let Some(arg) = ann.args.first() {
                    match arg {
                        ast_model::AnnotationArg::Positional(lit) => {
                            return Some(lit.to_string().trim_matches('"').to_string());
                        }
                        ast_model::AnnotationArg::Named(param) => {
                            return Some(param.value.to_string().trim_matches('"').to_string());
                        }
                    }
                }
                // For annotations without arguments (like @readonly), return empty string
                return Some(String::new());
            }
        }
    }
    None
}

/// Converts an AST RenameRule to an IR RenameInfo.
fn convert_rename(rename: &ast_model::RenameRule) -> RenameInfo {
    // Determine the kind based on path length:
    // - Length 2: Table rename (e.g., game.Player -> User)
    // - Length 3+: Field rename (e.g., game.User.hp -> health)
    let kind = if rename.from_path.len() <= 2 {
        RenameKind::Table
    } else {
        RenameKind::Field
    };

    RenameInfo {
        kind,
        from_path: rename.from_path.clone(),
        to_name: rename.to_name.clone(),
    }
}

/// Information extracted from field constraints.
#[derive(Debug, Default)]
struct ConstraintInfo {
    is_primary_key: bool,
    is_unique: bool,
    is_index: bool,
    foreign_key: Option<ForeignKeyDef>,
}

/// Extracts structured constraint information from AST constraints.
fn extract_constraint_info(constraints: &[ast_model::Constraint]) -> ConstraintInfo {
    let mut info = ConstraintInfo::default();

    for constraint in constraints {
        match constraint {
            ast_model::Constraint::PrimaryKey => info.is_primary_key = true,
            ast_model::Constraint::Unique => info.is_unique = true,
            ast_model::Constraint::ForeignKey(path, alias) => {
                // Parse the path: e.g., ["game", "character", "Player", "id"]
                // The last element is the field, everything before is the table FQN
                if path.len() >= 2 {
                    let target_field = path.last().unwrap().clone();
                    let target_table_fqn = path[..path.len() - 1].join(".");
                    info.foreign_key = Some(ForeignKeyDef {
                        target_table_fqn,
                        target_field,
                        alias: alias.clone(),
                    });
                    // FK fields get a GroupIndex automatically
                    info.is_index = true;
                }
            }
            ast_model::Constraint::Index => info.is_index = true,
            _ => {}
        }
    }

    info
}

/// Builds the template-friendly Intermediate Representation (IR) from the AST definitions.
pub fn build_ir(asts: &[ast_model::AstRoot]) -> ir_model::SchemaContext {
    let mut context = SchemaContext::default();

    for ast in asts {
        let file_name = ast
            .path
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_default();

        let mut top_level_namespaces = Vec::new();
        let mut global_items = Vec::new();

        // Process definitions at the root of the file.
        for def in &ast.definitions {
            // Top-level namespace blocks are handled specially to form the root of the hierarchy.
            if let Definition::Namespace(ns_ast) = def {
                let ns_name = ns_ast.path.join(".");
                let ns_datasource = extract_datasource(&ns_ast.metadata);
                let mut new_ns = NamespaceDef {
                    name: ns_name.clone(),
                    datasource: ns_datasource.clone(),
                    items: Vec::new(),
                };
                // Recurse into this new top-level namespace
                populate_items_recursively(&mut new_ns.items, &ns_ast.definitions, &ns_name, ns_datasource.as_deref());
                top_level_namespaces.push(new_ns);
            } else {
                // Any other item at the top level belongs to the "global" namespace.
                add_definition_to_items(&mut global_items, def, "", None);
            }
        }

        // If there were global items, create a global namespace for them (name is empty).
        if !global_items.is_empty() {
            let global_ns = NamespaceDef {
                name: "".to_string(),
                datasource: None,
                items: global_items,
            };
            top_level_namespaces.insert(0, global_ns);
        }

        // Convert AST renames to IR renames
        let renames = ast
            .renames
            .iter()
            .map(|r| convert_rename(r))
            .collect();

        let file_def = FileDef {
            path: file_name,
            namespaces: top_level_namespaces,
            renames,
        };
        context.files.push(file_def);
    }

    // After building the raw IR, resolve TypeRef flags (is_enum/is_struct) using the collected defs
    resolve_type_kinds(&mut context);

    // Resolve reverse relations from foreign_key ... as definitions
    resolve_relations(&mut context);

    context
}

/// Represents a pending relation to be added to a target table.
struct PendingRelation {
    /// FQN of the target table that should receive the relation.
    target_table_fqn: String,
    /// The relation definition to add.
    relation: RelationDef,
}

/// Resolves reverse relations from foreign_key definitions with aliases.
/// This must be called after all structs are built, as it needs to find
/// target tables that may be in different files or namespaces.
fn resolve_relations(context: &mut SchemaContext) {
    // Step 1: Collect all pending relations from foreign_key ... as fields
    let mut pending_relations: Vec<PendingRelation> = Vec::new();

    for file in &context.files {
        collect_relations_from_namespace(&file.namespaces, &mut pending_relations);
    }

    // Step 2: Apply relations to their target tables
    for pending in pending_relations {
        apply_relation_to_target(context, &pending);
    }
}

/// Recursively collects relations from namespaces.
fn collect_relations_from_namespace(
    namespaces: &[NamespaceDef],
    pending: &mut Vec<PendingRelation>,
) {
    for ns in namespaces {
        for item in &ns.items {
            match item {
                NamespaceItem::Struct(s) => {
                    collect_relations_from_struct(s, pending);
                }
                NamespaceItem::Namespace(nested_ns) => {
                    collect_relations_from_namespace(&[(**nested_ns).clone()], pending);
                }
                _ => {}
            }
        }
    }
}

/// Collects relations from a single struct's fields.
fn collect_relations_from_struct(struct_def: &StructDef, pending: &mut Vec<PendingRelation>) {
    for item in &struct_def.items {
        if let StructItem::Field(field) = item {
            if let Some(fk) = &field.foreign_key {
                if let Some(alias) = &fk.alias {
                    pending.push(PendingRelation {
                        target_table_fqn: fk.target_table_fqn.clone(),
                        relation: RelationDef {
                            name: alias.to_pascal_case(),
                            source_table_fqn: struct_def.fqn.clone(),
                            source_table_name: struct_def.name.clone(),
                            source_field: field.name.clone(),
                        },
                    });
                }
            }
        }
    }
}

/// Applies a pending relation to its target table.
fn apply_relation_to_target(context: &mut SchemaContext, pending: &PendingRelation) {
    for file in &mut context.files {
        if apply_relation_to_namespaces(&mut file.namespaces, pending) {
            return;
        }
    }
}

/// Recursively searches namespaces for the target table and applies the relation.
fn apply_relation_to_namespaces(
    namespaces: &mut [NamespaceDef],
    pending: &PendingRelation,
) -> bool {
    for ns in namespaces {
        for item in &mut ns.items {
            match item {
                NamespaceItem::Struct(s) => {
                    if s.fqn == pending.target_table_fqn {
                        s.relations.push(pending.relation.clone());
                        return true;
                    }
                }
                NamespaceItem::Namespace(nested_ns) => {
                    if apply_relation_to_namespaces(
                        &mut [(**nested_ns).clone()],
                        pending,
                    ) {
                        // Need to update the boxed namespace
                        // This is a bit awkward due to the Box
                        return true;
                    }
                }
                _ => {}
            }
        }
    }
    false
}

/// Recursively populates a list of items from AST definitions.
fn populate_items_recursively(
    items: &mut Vec<NamespaceItem>,
    definitions: &[ast_model::Definition],
    current_ns: &str,
    inherited_datasource: Option<&str>,
) {
    for def in definitions {
        add_definition_to_items(items, def, current_ns, inherited_datasource);
    }
}

/// Converts a single AST Definition into a NamespaceItem and adds it to a list.
/// This is the core of the recursive build process.
fn add_definition_to_items(
    items: &mut Vec<NamespaceItem>,
    def: &Definition,
    current_ns: &str,
    inherited_datasource: Option<&str>,
) {
    match def {
        Definition::Namespace(ns_ast) => {
            let this = ns_ast.path.join(".");
            let next_ns = if current_ns.is_empty() {
                this
            } else if this.is_empty() {
                current_ns.to_string()
            } else {
                format!("{}.{}", current_ns, this)
            };
            // Extract datasource from this namespace, or inherit from parent
            let ns_datasource = extract_datasource(&ns_ast.metadata)
                .or_else(|| inherited_datasource.map(String::from));
            let mut new_ns = NamespaceDef {
                name: next_ns.clone(),
                datasource: ns_datasource.clone(),
                items: Vec::new(),
            };
            populate_items_recursively(&mut new_ns.items, &ns_ast.definitions, &next_ns, ns_datasource.as_deref());
            items.push(NamespaceItem::Namespace(Box::new(new_ns)));
        }
        Definition::Table(table) => {
            items.push(NamespaceItem::Struct(convert_table_to_struct(table, current_ns, inherited_datasource)));
        }
        Definition::Enum(e) => {
            items.push(NamespaceItem::Enum(convert_enum_to_enum_def(e, None, current_ns)));
        }
        Definition::Embed(embed) => {
            items.push(NamespaceItem::Struct(convert_embed_to_struct(embed, current_ns)));
        }
        Definition::Comment(c) => {
            items.push(NamespaceItem::Comment(c.clone()));
        }
        Definition::Annotation(_) => { /* Annotations are handled within other items, not as top-level IR items */ }
    }
}

// Traverse the built IR and fix TypeRef.is_enum / is_struct flags based on declared types
fn resolve_type_kinds(ctx: &mut SchemaContext) {
    let mut registry = TypeRegistry::new();

    // Pass 1: Collect all types (enums and structs) from all files/namespaces
    for file in &ctx.files {
        for ns in &file.namespaces {
            collect_types_from_namespace_items(&ns.items, &mut registry);
        }
    }

    // Pass 2: Adjust all TypeRefs in struct fields recursively
    for file in &mut ctx.files {
        for ns in &mut file.namespaces {
            adjust_namespace_items_typerefs(&mut ns.items, &registry);
        }
    }
}

/// Recursively collects all type definitions from namespace items into the registry.
fn collect_types_from_namespace_items(items: &[NamespaceItem], registry: &mut TypeRegistry) {
    for item in items {
        match item {
            NamespaceItem::Enum(e) => {
                registry.register(&e.fqn, TypeKind::Enum);
            }
            NamespaceItem::Struct(s) => {
                registry.register(&s.fqn, TypeKind::Struct);
                collect_types_from_struct(s, registry);
            }
            NamespaceItem::Namespace(inner) => {
                collect_types_from_namespace_items(&inner.items, registry);
            }
            NamespaceItem::Comment(_) => {}
        }
    }
}

/// Recursively collects types from within a struct (inline enums, embedded structs).
fn collect_types_from_struct(s: &StructDef, registry: &mut TypeRegistry) {
    for item in &s.items {
        match item {
            StructItem::InlineEnum(e) => {
                registry.register(&e.fqn, TypeKind::Enum);
            }
            StructItem::EmbeddedStruct(sub) => {
                registry.register(&sub.fqn, TypeKind::Embed);
                collect_types_from_struct(sub, registry);
            }
            StructItem::Field(_) | StructItem::Annotation(_) | StructItem::Comment(_) => {}
        }
    }
}

/// Recursively adjusts TypeRef flags in namespace items using the registry.
fn adjust_namespace_items_typerefs(items: &mut [NamespaceItem], registry: &TypeRegistry) {
    for item in items {
        match item {
            NamespaceItem::Struct(ref mut s) => adjust_struct_typerefs(s, registry),
            NamespaceItem::Namespace(ref mut inner) => {
                adjust_namespace_items_typerefs(&mut inner.items, registry)
            }
            NamespaceItem::Enum(_) | NamespaceItem::Comment(_) => {}
        }
    }
}

/// Recursively adjusts TypeRef flags in a struct's fields.
fn adjust_struct_typerefs(s: &mut StructDef, registry: &TypeRegistry) {
    for item in &mut s.items {
        match item {
            StructItem::Field(ref mut f) => {
                adjust_typeref(&mut f.field_type, registry);
            }
            StructItem::EmbeddedStruct(ref mut sub) => adjust_struct_typerefs(sub, registry),
            StructItem::InlineEnum(_) | StructItem::Annotation(_) | StructItem::Comment(_) => {}
        }
    }
}

/// Adjusts a single TypeRef's is_enum/is_struct flags using the registry.
fn adjust_typeref(t: &mut TypeRef, registry: &TypeRegistry) {
    // First, recursively process inner types (for Option<T> or List<T>)
    if let Some(inner) = &mut t.inner_type {
        adjust_typeref(inner.as_mut(), registry);
    }

    // Skip primitives - they don't need resolution
    if t.is_primitive {
        return;
    }

    // Try to resolve the type using the registry
    // Strategy 1: Direct FQN match
    if registry.is_enum(&t.fqn) {
        t.is_enum = true;
        t.is_struct = false;
        return;
    }

    // Strategy 2: Resolve using namespace context
    if let Some(resolved_fqn) = registry.resolve(&t.type_name, &t.namespace_fqn) {
        if registry.is_enum(resolved_fqn) {
            t.fqn = resolved_fqn.to_string();
            t.namespace_fqn = namespace_of_owned(resolved_fqn);
            t.is_enum = true;
            t.is_struct = false;
            return;
        }
    }

    // Default: treat as struct (could be a struct reference or external type)
    t.is_enum = false;
    t.is_struct = true;
}

fn convert_table_to_struct(
    table: &ast_model::Table,
    current_ns: &str,
    inherited_datasource: Option<&str>,
) -> StructDef {
    let mut items = Vec::new();
    let mut header_items = Vec::new();
    let name = table.name.clone().unwrap();
    let fqn = if current_ns.is_empty() {
        name.clone()
    } else {
        format!("{}.{}", current_ns, name)
    };

    // Extract table-level datasource, or inherit from namespace
    let table_datasource = extract_datasource(&table.metadata)
        .or_else(|| inherited_datasource.map(String::from));

    // Extract advanced annotations
    let cache_strategy = extract_cache_strategy(&table.metadata);
    let readonly = is_readonly(&table.metadata);
    let soft_delete_field = extract_soft_delete_field(&table.metadata);

    // Process metadata for the struct header
    for meta in &table.metadata {
        match meta {
            Metadata::DocComment(c) => {
                header_items.push(StructItem::Comment(c.clone()));
            }
            Metadata::Annotation(a) => {
                let annotation_def = convert_annotation_to_ir(a);
                header_items.push(StructItem::Annotation(annotation_def));
            }
        }
    }

    for member in &table.members {
        match member {
            TableMember::Field(field) => {
                // Handle metadata (comments, annotations) associated with the field first
                let field_metadata = match field {
                    ast_model::FieldDefinition::Regular(rf) => &rf.metadata,
                    ast_model::FieldDefinition::InlineEmbed(ief) => &ief.metadata,
                    ast_model::FieldDefinition::InlineEnum(e) => &e.metadata,
                };
                for meta in field_metadata {
                    match meta {
                        Metadata::DocComment(c) => items.push(StructItem::Comment(c.clone())),
                        Metadata::Annotation(a) => {
                            let annotation_def = convert_annotation_to_ir(a);
                            items.push(StructItem::Annotation(annotation_def));
                        }
                    }
                }

                // Then handle the field itself
                let (field_def, new_nested_structs, new_nested_enums) =
                    convert_field_to_ir(field, &fqn, &fqn);
                items.push(StructItem::Field(field_def));
                // Add the new nested types to the items list
                items.extend(new_nested_structs.into_iter().map(StructItem::EmbeddedStruct));
                items.extend(new_nested_enums.into_iter().map(StructItem::InlineEnum));
            }
            TableMember::Embed(embed) => {
                items.push(StructItem::EmbeddedStruct(convert_embed_to_struct(embed, &fqn)));
            }
            TableMember::Enum(e) => {
                items.push(StructItem::InlineEnum(convert_enum_to_enum_def(e, None, &fqn)));
            }
            TableMember::Comment(c) => items.push(StructItem::Comment(c.clone())),
        }
    }

    // Build indexes from field constraints
    let mut indexes = build_indexes_from_items(&items);

    // Build indexes from @index annotations
    let annotation_indexes = build_indexes_from_annotations(&header_items, &items);
    indexes.extend(annotation_indexes);

    StructDef {
        name,
        fqn,
        datasource: table_datasource,
        cache_strategy,
        is_readonly: readonly,
        soft_delete_field,
        items,
        header: header_items,
        indexes,
        relations: Vec::new(), // Relations are populated in post-processing
    }
}

/// Builds index definitions from struct items by examining field constraints.
fn build_indexes_from_items(items: &[StructItem]) -> Vec<IndexDef> {
    let mut indexes = Vec::new();

    for item in items {
        if let StructItem::Field(field) = item {
            // Primary key or unique constraint creates a unique index
            if field.is_primary_key || field.is_unique {
                indexes.push(IndexDef {
                    name: format!("By{}", field.name.to_pascal_case()),
                    fields: vec![ir_model::IndexFieldDef {
                        name: field.name.clone(),
                        field_type: field.field_type.clone(),
                    }],
                    is_unique: true,
                    source: "constraint".to_string(),
                });
            }
            // Index constraint or foreign key creates a group index
            else if field.is_index {
                indexes.push(IndexDef {
                    name: format!("By{}", field.name.to_pascal_case()),
                    fields: vec![ir_model::IndexFieldDef {
                        name: field.name.clone(),
                        field_type: field.field_type.clone(),
                    }],
                    is_unique: false,
                    source: "constraint".to_string(),
                });
            }
        }
    }

    indexes
}

/// Builds index definitions from @index annotations on a table.
fn build_indexes_from_annotations(
    header: &[StructItem],
    items: &[StructItem],
) -> Vec<IndexDef> {
    let mut indexes = Vec::new();

    // Build a map of field names to their types for lookup
    let field_types: std::collections::HashMap<String, TypeRef> = items
        .iter()
        .filter_map(|item| {
            if let StructItem::Field(field) = item {
                Some((field.name.clone(), field.field_type.clone()))
            } else {
                None
            }
        })
        .collect();

    for item in header {
        if let StructItem::Annotation(ann) = item {
            if ann.name == "index" && !ann.positional_args.is_empty() {
                // Check if "unique: true" is specified in params
                let is_unique = ann.params.iter().any(|p| {
                    p.key == "unique" && (p.value == "true" || p.value == "1")
                });

                // Build field definitions from positional args
                let fields: Vec<ir_model::IndexFieldDef> = ann
                    .positional_args
                    .iter()
                    .filter_map(|field_name| {
                        field_types.get(field_name).map(|field_type| {
                            ir_model::IndexFieldDef {
                                name: field_name.clone(),
                                field_type: field_type.clone(),
                            }
                        })
                    })
                    .collect();

                if !fields.is_empty() {
                    // Generate index name from field names
                    let name = format!(
                        "By{}",
                        fields
                            .iter()
                            .map(|f| f.name.to_pascal_case())
                            .collect::<Vec<_>>()
                            .join("")
                    );

                    indexes.push(IndexDef {
                        name,
                        fields,
                        is_unique,
                        source: "annotation".to_string(),
                    });
                }
            }
        }
    }

    indexes
}

/// Converts an `ast_model::FieldDefinition` into an `ir_model::FieldDef` and potential nested types (from inline embeds).
fn convert_field_to_ir(
    field: &ast_model::FieldDefinition,
    current_ns: &str,
    owner_fqn: &str,
) -> (FieldDef, Vec<StructDef>, Vec<EnumDef>) {
    match field {
        ast_model::FieldDefinition::Regular(rf) => {
            let attributes = convert_constraints_to_attributes(&rf.constraints);
            let mut inline_enums = Vec::new();

            let field_type: TypeRef = match &rf.field_type.base_type {
                ast_model::TypeName::InlineEnum(e) => {
                    // Generate a unique name for the inline enum
                    // For now, let's use FieldName_Enum. We'll need table context later for better names.
                    let generated_enum_name = format!("{}_Enum", rf.name.clone().expect("Regular field name must be present").to_pascal_case());

                    // Create the EnumDef using the generated name
                    let enum_fqn = if owner_fqn.is_empty() { generated_enum_name.clone() } else { format!("{}.{}", owner_fqn, generated_enum_name) };
                    let enum_def = convert_enum_to_enum_def(e, Some(generated_enum_name.clone()), owner_fqn);
                    inline_enums.push(enum_def);
                    build_type_ref_from_base(&enum_fqn, &generated_enum_name, &rf.field_type.cardinality, false)
                },
                _ => build_type_ref(&rf.field_type, current_ns),
            };

            // Extract constraint info for the new fields
            let constraint_info = extract_constraint_info(&rf.constraints);

            (
                FieldDef {
                    name: rf.name.clone().expect("Regular field name must be present"),
                    field_type,
                    attributes,
                    is_primary_key: constraint_info.is_primary_key,
                    is_unique: constraint_info.is_unique,
                    is_index: constraint_info.is_index,
                    foreign_key: constraint_info.foreign_key,
                },
                Vec::new(),
                inline_enums,
            )
        }
        ast_model::FieldDefinition::InlineEmbed(ief) => {
            let struct_name = ief.name.clone().expect("Inline embed field name must be present").to_pascal_case();
            let inline_struct = convert_table_to_struct(
                &ast_model::Table {
                    name: Some(struct_name.clone()),
                    metadata: ief.metadata.clone(),
                    members: ief.members.clone(),
                },
                owner_fqn,
                None, // Inline embeds don't inherit datasource
            );

            let nested_items = vec![inline_struct];

            let field_def = FieldDef {
                name: ief.name.clone().expect("Inline embed field name must be present"),
                field_type: build_type_ref_from_base(
                    &format!("{}.{}", owner_fqn, struct_name),
                    &struct_name,
                    &ief.cardinality,
                    false,
                ),
                attributes: Vec::new(),
                is_primary_key: false,
                is_unique: false,
                is_index: false,
                foreign_key: None,
            };
            (field_def, nested_items, Vec::new())
        }
        ast_model::FieldDefinition::InlineEnum(e) => {
            let generated_enum_name = format!("{}__Enum", e.name.clone().expect("Inline enum name must be present").to_pascal_case());
            
            // Create a temporary Enum from InlineEnumField
            let temp_enum = ast_model::Enum {
                metadata: e.metadata.clone(),
                name: e.name.clone(),
                variants: e.variants.clone(),
            };

            let enum_def = convert_enum_to_enum_def(&temp_enum, Some(generated_enum_name.clone()), owner_fqn);
            
            let field_def = FieldDef {
                name: e.name.clone().expect("Inline enum name must be present"),
                field_type: build_type_ref_from_base(
                    &format!("{}.{}", owner_fqn, generated_enum_name),
                    &generated_enum_name,
                    &e.cardinality,
                    false,
                ),
                attributes: Vec::new(),
                is_primary_key: false,
                is_unique: false,
                is_index: false,
                foreign_key: None,
            };
            (field_def, Vec::new(), vec![enum_def])
        }
    }
}

/// Converts field constraints from the AST into a vector of strings
/// suitable for C# attributes.
fn convert_constraints_to_attributes(constraints: &[ast_model::Constraint]) -> Vec<String> {
    constraints
        .iter()
        .filter_map(|c| match c {
            // `primary_key` is mapped to the `[Key]` attribute, common in ORMs like EF Core.
            ast_model::Constraint::PrimaryKey => Some("Key".to_string()),
            // `unique` can be mapped to an index attribute.
            ast_model::Constraint::Unique => Some("Index(IsUnique = true)".to_string()),
            ast_model::Constraint::MaxLength(len) => Some(format!("MaxLength({})", len)),
            // ForeignKey is a relationship, not a simple attribute, so we ignore it here.
            ast_model::Constraint::ForeignKey(_, _) => None,
            // Other constraints are not (yet) represented as attributes.
            _ => None,
        })
        .collect()
}

fn convert_enum_to_enum_def(e: &ast_model::Enum, name_override: Option<String>, ns_or_owner_fqn: &str) -> EnumDef {
    let mut items = Vec::new();
    let mut current_value: i64 = 0; // Initialize counter for sequential numbering

    // Extract enum's own comment and add to items
    for meta in &e.metadata {
        if let Metadata::DocComment(c) = meta {
            items.push(EnumItem::Comment(c.clone()));
            break; // Assuming only one doc comment for the enum itself
        }
    }

    for variant in &e.variants {
        for meta in &variant.metadata {
            if let Metadata::DocComment(c) = meta {
                items.push(EnumItem::Comment(c.clone()));
                println!("[ir_builder] comment 2: {:?}", c);
            }
        }

        let member_value = if let Some(explicit_value) = variant.value {
            // If an explicit value is provided, use it and update the counter
            current_value = explicit_value;
            Some(explicit_value)
        } else {
            // If no explicit value, use the current sequential value
            let value_to_assign = current_value;
            current_value += 1; // Increment for the next member
            Some(value_to_assign)
        };

        items.push(EnumItem::Member(ir_model::EnumMember {
            name: variant.name.clone().unwrap(),
            value: member_value,
        }));
    }

    let name = name_override.unwrap_or_else(|| e.name.clone().expect("Named enum must have a name"));
    let fqn = if ns_or_owner_fqn.is_empty() { name.clone() } else { format!("{}.{}", ns_or_owner_fqn, name) };
    EnumDef { name, fqn, items }
}

fn convert_embed_to_struct(embed: &ast_model::Embed, owner_fqn: &str) -> StructDef {
    convert_table_to_struct(
        &ast_model::Table {
            name: embed.name.clone(),
            metadata: embed.metadata.clone(),
            members: embed.members.clone(),
        },
        owner_fqn,
        None, // Embeds don't inherit datasource
    )
}

// Build TypeRef from an AST type in the given namespace context
fn build_type_ref(t: &ast_model::TypeWithCardinality, current_ns: &str) -> TypeRef {
    let (base_fqn, base_name, is_primitive, is_enum) = match &t.base_type {
        ast_model::TypeName::Path(p) => (p.join("."), p.join("."), false, false),
        ast_model::TypeName::Basic(b) => (basic_name(b).to_string(), basic_name(b).to_string(), true, false),
        ast_model::TypeName::InlineEnum(_) => ("__ANON_ENUM__".to_string(), "__ANON_ENUM__".to_string(), false, true),
    };
    match t.cardinality {
        Some(ast_model::Cardinality::Optional) => {
            let inner_fqn = if is_primitive { base_fqn } else { qualify(&base_fqn, current_ns) };
            let inner_ns = namespace_of_owned(&inner_fqn);
            let inner = TypeRef {
                original: base_name.clone(),
                fqn: inner_fqn.clone(),
                namespace_fqn: inner_ns.clone(),
                type_name: last_segment_owned(&inner_fqn),
                parent_type_path: parent_type_path_of(&inner_fqn, &inner_ns),
                lang_type: base_name.clone(),
                is_primitive,
                is_struct: !(is_primitive || is_enum),
                is_enum,
                is_option: false,
                is_list: false,
                inner_type: None,
            };
            let outer_ns = namespace_of_owned(&inner_fqn);
            TypeRef {
                original: format!("Option<{}>", base_name),
                fqn: inner_fqn.clone(),
                namespace_fqn: outer_ns.clone(),
                type_name: last_segment_owned(&inner_fqn),
                parent_type_path: parent_type_path_of(&inner_fqn, &outer_ns),
                lang_type: format!("Option<{}>", inner.lang_type),
                is_primitive: false,
                is_struct: !(is_primitive || is_enum),
                is_enum,
                is_option: true,
                is_list: false,
                inner_type: Some(Box::new(inner)),
            }
        }
        Some(ast_model::Cardinality::Array) => {
            let inner_fqn = if is_primitive { base_fqn } else { qualify(&base_fqn, current_ns) };
            let inner_ns = namespace_of_owned(&inner_fqn);
            let inner = TypeRef {
                original: base_name.clone(),
                fqn: inner_fqn.clone(),
                namespace_fqn: inner_ns.clone(),
                type_name: last_segment_owned(&inner_fqn),
                parent_type_path: parent_type_path_of(&inner_fqn, &inner_ns),
                lang_type: base_name.clone(),
                is_primitive,
                is_struct: !(is_primitive || is_enum),
                is_enum,
                is_option: false,
                is_list: false,
                inner_type: None,
            };
            let outer_ns = namespace_of_owned(&inner_fqn);
            TypeRef {
                original: format!("List<{}>", base_name),
                fqn: inner_fqn.clone(),
                namespace_fqn: outer_ns.clone(),
                type_name: last_segment_owned(&inner_fqn),
                parent_type_path: parent_type_path_of(&inner_fqn, &outer_ns),
                lang_type: format!("List<{}>", inner.lang_type),
                is_primitive: false,
                is_struct: !(is_primitive || is_enum),
                is_enum,
                is_option: false,
                is_list: true,
                inner_type: Some(Box::new(inner)),
            }
        }
        None => {
            let core_fqn = if is_primitive { base_fqn } else { qualify(&base_fqn, current_ns) };
            let core_ns = namespace_of_owned(&core_fqn);
            TypeRef {
                original: base_name.clone(),
                fqn: core_fqn.clone(),
                namespace_fqn: core_ns.clone(),
                type_name: last_segment_owned(&core_fqn),
                parent_type_path: parent_type_path_of(&core_fqn, &core_ns),
                lang_type: base_name,
                is_primitive,
                is_struct: !(is_primitive || is_enum),
                is_enum,
                is_option: false,
                is_list: false,
                inner_type: None,
            }
        }
    }
}

fn build_type_ref_from_base(base_fqn: &str, base_name: &str, c: &Option<ast_model::Cardinality>, primitive_hint: bool) -> TypeRef {
    match c {
        Some(ast_model::Cardinality::Optional) => {
            let inner_ns = namespace_of_owned(base_fqn);
            let inner = TypeRef {
                original: base_name.to_string(),
                fqn: base_fqn.to_string(),
                namespace_fqn: inner_ns.clone(),
                type_name: last_segment_owned(base_fqn),
                parent_type_path: parent_type_path_of(base_fqn, &inner_ns),
                lang_type: base_name.to_string(),
                is_primitive: primitive_hint,
                is_struct: !primitive_hint,
                is_enum: false,
                is_option: false,
                is_list: false,
                inner_type: None,
            };
            let outer_ns = namespace_of_owned(base_fqn);
            TypeRef {
                original: format!("Option<{}>", base_name),
                fqn: base_fqn.to_string(),
                namespace_fqn: outer_ns.clone(),
                type_name: last_segment_owned(base_fqn),
                parent_type_path: parent_type_path_of(base_fqn, &outer_ns),
                lang_type: format!("Option<{}>", base_name),
                is_primitive: false,
                is_struct: !primitive_hint,
                is_enum: false,
                is_option: true,
                is_list: false,
                inner_type: Some(Box::new(inner)),
            }
        }
        Some(ast_model::Cardinality::Array) => {
            let inner_ns = namespace_of_owned(base_fqn);
            let inner = TypeRef {
                original: base_name.to_string(),
                fqn: base_fqn.to_string(),
                namespace_fqn: inner_ns.clone(),
                type_name: last_segment_owned(base_fqn),
                parent_type_path: parent_type_path_of(base_fqn, &inner_ns),
                lang_type: base_name.to_string(),
                is_primitive: primitive_hint,
                is_struct: !primitive_hint,
                is_enum: false,
                is_option: false,
                is_list: false,
                inner_type: None,
            };
            let outer_ns = namespace_of_owned(base_fqn);
            TypeRef {
                original: format!("List<{}>", base_name),
                fqn: base_fqn.to_string(),
                namespace_fqn: outer_ns.clone(),
                type_name: last_segment_owned(base_fqn),
                parent_type_path: parent_type_path_of(base_fqn, &outer_ns),
                lang_type: format!("List<{}>", base_name),
                is_primitive: false,
                is_struct: !primitive_hint,
                is_enum: false,
                is_option: false,
                is_list: true,
                inner_type: Some(Box::new(inner)),
            }
        }
        None => {
            let ns = namespace_of_owned(base_fqn);
            TypeRef {
                original: base_name.to_string(),
                fqn: base_fqn.to_string(),
                namespace_fqn: ns.clone(),
                type_name: last_segment_owned(base_fqn),
                parent_type_path: parent_type_path_of(base_fqn, &ns),
                lang_type: base_name.to_string(),
                is_primitive: primitive_hint,
                is_struct: !primitive_hint,
                is_enum: false,
                is_option: false,
                is_list: false,
                inner_type: None,
            }
        }
    }
}

fn qualify(base_fqn: &str, current_ns: &str) -> String {
    if base_fqn.contains('.') || current_ns.is_empty() {
        base_fqn.to_string()
    } else {
        format!("{}.{}", current_ns, base_fqn)
    }
}

fn namespace_of_owned(fqn: &str) -> String {
    match fqn.rfind('.') {
        Some(i) => fqn[..i].to_string(),
        None => String::new(),
    }
}

fn last_segment_owned(fqn: &str) -> String {
    match fqn.rfind('.') {
        Some(i) => fqn[i + 1..].to_string(),
        None => fqn.to_string(),
    }
}

/// Extracts the parent type path from an FQN.
/// For "game.character.Monster.DropItems.Enchantment", returns "Monster.DropItems".
/// For top-level types or primitives, returns empty string.
fn parent_type_path_of(fqn: &str, namespace_fqn: &str) -> String {
    // If there's no namespace, the type is either primitive or top-level
    if namespace_fqn.is_empty() {
        return String::new();
    }
    
    // Remove namespace prefix from FQN to get the type path
    let type_path = if fqn.starts_with(namespace_fqn) && fqn.len() > namespace_fqn.len() {
        &fqn[namespace_fqn.len() + 1..]  // +1 to skip the dot
    } else {
        fqn
    };
    
    // Find the last dot in the type path
    match type_path.rfind('.') {
        Some(i) => type_path[..i].to_string(),
        None => String::new(),  // Top-level type in namespace
    }
}

fn basic_name(b: &ast_model::BasicType) -> &'static str {
    use ast_model::BasicType::*;
    match b {
        String => "string",
        I8 => "i8",
        I16 => "i16",
        I32 => "i32",
        I64 => "i64",
        U8 => "u8",
        U16 => "u16",
        U32 => "u32",
        U64 => "u64",
        F32 => "f32",
        F64 => "f64",
        Bool => "bool",
        Bytes => "bytes",
    }
}

/// Converts a single AST annotation to IR annotation definition.
fn convert_annotation_to_ir(ast_ann: &ast_model::Annotation) -> AnnotationDef {
    let mut positional_args = Vec::new();
    let mut params = Vec::new();

    for arg in &ast_ann.args {
        match arg {
            ast_model::AnnotationArg::Positional(lit) => {
                positional_args.push(lit.to_string());
            }
            ast_model::AnnotationArg::Named(p) => {
                params.push(AnnotationParam {
                    key: p.key.clone(),
                    value: p.value.to_string(),
                });
            }
        }
    }

    AnnotationDef {
        name: ast_ann.name.clone().unwrap(),
        positional_args,
        params,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast_model::*;
    use std::path::PathBuf;

    /// Helper to create a minimal AstRoot
    fn make_ast(definitions: Vec<Definition>) -> AstRoot {
        AstRoot {
            path: PathBuf::from("test.schema"),
            file_imports: vec![],
            definitions,
            renames: vec![],
        }
    }

    /// Helper to create a simple table
    fn make_table(name: &str, members: Vec<TableMember>) -> Definition {
        Definition::Table(Table {
            metadata: vec![],
            name: Some(name.to_string()),
            members,
        })
    }

    /// Helper to create a simple enum
    fn make_enum(name: &str, variants: Vec<&str>) -> Definition {
        Definition::Enum(Enum {
            metadata: vec![],
            name: Some(name.to_string()),
            variants: variants
                .into_iter()
                .map(|v| EnumVariant {
                    metadata: vec![],
                    name: Some(v.to_string()),
                    value: None,
                    inline_comment: None,
                })
                .collect(),
        })
    }

    /// Helper to create a namespace
    fn make_namespace(path: Vec<&str>, definitions: Vec<Definition>) -> Definition {
        Definition::Namespace(Namespace {
            metadata: vec![],
            path: path.into_iter().map(String::from).collect(),
            imports: vec![],
            definitions,
        })
    }

    /// Helper to create a regular field with path type
    fn make_field_path(name: &str, type_path: Vec<&str>) -> TableMember {
        TableMember::Field(FieldDefinition::Regular(RegularField {
            metadata: vec![],
            name: Some(name.to_string()),
            field_type: TypeWithCardinality {
                base_type: TypeName::Path(type_path.into_iter().map(String::from).collect()),
                cardinality: None,
            },
            constraints: vec![],
            field_number: None,
        }))
    }

    /// Helper to create a regular field with basic type
    fn make_field_basic(name: &str, basic_type: BasicType) -> TableMember {
        TableMember::Field(FieldDefinition::Regular(RegularField {
            metadata: vec![],
            name: Some(name.to_string()),
            field_type: TypeWithCardinality {
                base_type: TypeName::Basic(basic_type),
                cardinality: None,
            },
            constraints: vec![],
            field_number: None,
        }))
    }

    /// Helper to find a field in a struct by name
    fn find_field<'a>(struct_def: &'a StructDef, name: &str) -> Option<&'a FieldDef> {
        struct_def.items.iter().find_map(|item| {
            if let StructItem::Field(f) = item {
                if f.name == name {
                    return Some(f);
                }
            }
            None
        })
    }

    /// Helper to find a struct in a namespace by name
    fn find_struct<'a>(ns: &'a NamespaceDef, name: &str) -> Option<&'a StructDef> {
        ns.items.iter().find_map(|item| {
            if let NamespaceItem::Struct(s) = item {
                if s.name == name {
                    return Some(s);
                }
            }
            None
        })
    }

    /// Helper to find an enum in a namespace by name
    fn find_enum<'a>(ns: &'a NamespaceDef, name: &str) -> Option<&'a EnumDef> {
        ns.items.iter().find_map(|item| {
            if let NamespaceItem::Enum(e) = item {
                if e.name == name {
                    return Some(e);
                }
            }
            None
        })
    }

    // ========== Helper Function Tests ==========

    #[test]
    fn test_namespace_of_owned() {
        assert_eq!(namespace_of_owned("game.common.Status"), "game.common");
        assert_eq!(namespace_of_owned("Status"), "");
        assert_eq!(namespace_of_owned("game.Status"), "game");
    }

    #[test]
    fn test_last_segment_owned() {
        assert_eq!(last_segment_owned("game.common.Status"), "Status");
        assert_eq!(last_segment_owned("Status"), "Status");
        assert_eq!(last_segment_owned("game.Status"), "Status");
    }

    #[test]
    fn test_qualify() {
        assert_eq!(qualify("Status", "game.common"), "game.common.Status");
        assert_eq!(qualify("game.common.Status", "other"), "game.common.Status");
        assert_eq!(qualify("Status", ""), "Status");
    }

    #[test]
    fn test_parent_type_path_of() {
        // Top-level type in namespace
        assert_eq!(parent_type_path_of("game.Monster", "game"), "");
        // Nested type one level deep
        assert_eq!(parent_type_path_of("game.Monster.Stats", "game"), "Monster");
        // Nested type two levels deep
        assert_eq!(
            parent_type_path_of("game.Monster.Stats.Buffs", "game"),
            "Monster.Stats"
        );
        // No namespace
        assert_eq!(parent_type_path_of("Status", ""), "");
    }

    #[test]
    fn test_basic_name() {
        assert_eq!(basic_name(&BasicType::String), "string");
        assert_eq!(basic_name(&BasicType::I32), "i32");
        assert_eq!(basic_name(&BasicType::Bool), "bool");
        assert_eq!(basic_name(&BasicType::Bytes), "bytes");
    }

    // ========== IR Building Tests ==========

    #[test]
    fn test_build_ir_empty_ast() {
        let asts = vec![make_ast(vec![])];
        let ctx = build_ir(&asts);
        assert_eq!(ctx.files.len(), 1);
        assert!(ctx.files[0].namespaces.is_empty());
    }

    #[test]
    fn test_build_ir_single_table() {
        let asts = vec![make_ast(vec![make_table("User", vec![])])];
        let ctx = build_ir(&asts);

        assert_eq!(ctx.files.len(), 1);
        assert_eq!(ctx.files[0].namespaces.len(), 1); // Global namespace
        let ns = &ctx.files[0].namespaces[0];
        assert_eq!(ns.name, ""); // Global namespace has empty name

        let user_struct = find_struct(ns, "User");
        assert!(user_struct.is_some());
        assert_eq!(user_struct.unwrap().fqn, "User");
    }

    #[test]
    fn test_build_ir_single_enum() {
        let asts = vec![make_ast(vec![make_enum("Status", vec!["Active", "Inactive"])])];
        let ctx = build_ir(&asts);

        let ns = &ctx.files[0].namespaces[0];
        let status_enum = find_enum(ns, "Status");
        assert!(status_enum.is_some());

        let e = status_enum.unwrap();
        assert_eq!(e.fqn, "Status");
        assert_eq!(e.items.len(), 2);
    }

    #[test]
    fn test_build_ir_namespaced_types() {
        let asts = vec![make_ast(vec![make_namespace(
            vec!["game", "common"],
            vec![make_table("User", vec![]), make_enum("Status", vec!["Active"])],
        )])];
        let ctx = build_ir(&asts);

        assert_eq!(ctx.files[0].namespaces.len(), 1);
        let ns = &ctx.files[0].namespaces[0];
        assert_eq!(ns.name, "game.common");

        let user = find_struct(ns, "User");
        assert!(user.is_some());
        assert_eq!(user.unwrap().fqn, "game.common.User");

        let status = find_enum(ns, "Status");
        assert!(status.is_some());
        assert_eq!(status.unwrap().fqn, "game.common.Status");
    }

    // ========== Type Resolution Tests ==========

    #[test]
    fn test_primitive_type_stays_primitive() {
        let asts = vec![make_ast(vec![make_table(
            "User",
            vec![make_field_basic("name", BasicType::String)],
        )])];
        let ctx = build_ir(&asts);

        let ns = &ctx.files[0].namespaces[0];
        let user = find_struct(ns, "User").unwrap();
        let name_field = find_field(user, "name").unwrap();

        assert!(name_field.field_type.is_primitive);
        assert!(!name_field.field_type.is_enum);
        assert!(!name_field.field_type.is_struct);
    }

    #[test]
    fn test_enum_type_is_resolved() {
        let asts = vec![make_ast(vec![
            make_enum("Status", vec!["Active", "Inactive"]),
            make_table("User", vec![make_field_path("status", vec!["Status"])]),
        ])];
        let ctx = build_ir(&asts);

        let ns = &ctx.files[0].namespaces[0];
        let user = find_struct(ns, "User").unwrap();
        let status_field = find_field(user, "status").unwrap();

        assert!(!status_field.field_type.is_primitive);
        assert!(status_field.field_type.is_enum);
        assert!(!status_field.field_type.is_struct);
    }

    #[test]
    fn test_struct_type_is_resolved() {
        let asts = vec![make_ast(vec![
            make_table("Address", vec![]),
            make_table("User", vec![make_field_path("address", vec!["Address"])]),
        ])];
        let ctx = build_ir(&asts);

        let ns = &ctx.files[0].namespaces[0];
        let user = find_struct(ns, "User").unwrap();
        let address_field = find_field(user, "address").unwrap();

        assert!(!address_field.field_type.is_primitive);
        assert!(!address_field.field_type.is_enum);
        assert!(address_field.field_type.is_struct);
    }

    #[test]
    fn test_namespaced_enum_resolution() {
        let asts = vec![make_ast(vec![make_namespace(
            vec!["game"],
            vec![
                make_enum("Status", vec!["Active"]),
                make_table("User", vec![make_field_path("status", vec!["Status"])]),
            ],
        )])];
        let ctx = build_ir(&asts);

        let ns = &ctx.files[0].namespaces[0];
        let user = find_struct(ns, "User").unwrap();
        let status_field = find_field(user, "status").unwrap();

        assert!(status_field.field_type.is_enum);
        assert_eq!(status_field.field_type.fqn, "game.Status");
    }

    #[test]
    fn test_cross_namespace_enum_resolution_with_fqn() {
        let asts = vec![make_ast(vec![
            make_namespace(vec!["common"], vec![make_enum("Status", vec!["Active"])]),
            make_namespace(
                vec!["user"],
                vec![make_table(
                    "User",
                    vec![make_field_path("status", vec!["common", "Status"])],
                )],
            ),
        ])];
        let ctx = build_ir(&asts);

        // Find user namespace
        let user_ns = ctx.files[0]
            .namespaces
            .iter()
            .find(|ns| ns.name == "user")
            .unwrap();
        let user = find_struct(user_ns, "User").unwrap();
        let status_field = find_field(user, "status").unwrap();

        assert!(status_field.field_type.is_enum);
        assert_eq!(status_field.field_type.fqn, "common.Status");
    }

    #[test]
    fn test_unique_enum_name_resolution() {
        // If an enum name is unique across all namespaces, it should resolve
        let asts = vec![make_ast(vec![
            make_namespace(vec!["common"], vec![make_enum("UniqueStatus", vec!["Active"])]),
            make_namespace(
                vec!["user"],
                vec![make_table(
                    "User",
                    vec![make_field_path("status", vec!["UniqueStatus"])],
                )],
            ),
        ])];
        let ctx = build_ir(&asts);

        let user_ns = ctx.files[0]
            .namespaces
            .iter()
            .find(|ns| ns.name == "user")
            .unwrap();
        let user = find_struct(user_ns, "User").unwrap();
        let status_field = find_field(user, "status").unwrap();

        // Since UniqueStatus is unique, it should be resolved as enum
        assert!(status_field.field_type.is_enum);
    }

    // ========== Inline Enum Tests ==========

    #[test]
    fn test_inline_enum_in_struct() {
        let inline_enum_field = TableMember::Field(FieldDefinition::InlineEnum(InlineEnumField {
            metadata: vec![],
            name: Some("role".to_string()),
            variants: vec![
                EnumVariant {
                    metadata: vec![],
                    name: Some("Admin".to_string()),
                    value: None,
                    inline_comment: None,
                },
                EnumVariant {
                    metadata: vec![],
                    name: Some("User".to_string()),
                    value: None,
                    inline_comment: None,
                },
            ],
            cardinality: None,
            field_number: None,
        }));

        let asts = vec![make_ast(vec![make_table("Account", vec![inline_enum_field])])];
        let ctx = build_ir(&asts);

        let ns = &ctx.files[0].namespaces[0];
        let account = find_struct(ns, "Account").unwrap();

        // Find the inline enum (uses double underscore in the name: Role__Enum)
        let has_inline_enum = account.items.iter().any(|item| {
            matches!(item, StructItem::InlineEnum(e) if e.name == "Role__Enum")
        });
        assert!(has_inline_enum);

        // The field should reference the inline enum
        let role_field = find_field(account, "role").unwrap();
        assert!(role_field.field_type.is_enum);
    }

    // ========== Cardinality Tests ==========

    #[test]
    fn test_optional_type() {
        let optional_field = TableMember::Field(FieldDefinition::Regular(RegularField {
            metadata: vec![],
            name: Some("nickname".to_string()),
            field_type: TypeWithCardinality {
                base_type: TypeName::Basic(BasicType::String),
                cardinality: Some(Cardinality::Optional),
            },
            constraints: vec![],
            field_number: None,
        }));

        let asts = vec![make_ast(vec![make_table("User", vec![optional_field])])];
        let ctx = build_ir(&asts);

        let ns = &ctx.files[0].namespaces[0];
        let user = find_struct(ns, "User").unwrap();
        let nickname_field = find_field(user, "nickname").unwrap();

        assert!(nickname_field.field_type.is_option);
        assert!(!nickname_field.field_type.is_list);
        assert!(nickname_field.field_type.inner_type.is_some());
    }

    #[test]
    fn test_array_type() {
        let array_field = TableMember::Field(FieldDefinition::Regular(RegularField {
            metadata: vec![],
            name: Some("tags".to_string()),
            field_type: TypeWithCardinality {
                base_type: TypeName::Basic(BasicType::String),
                cardinality: Some(Cardinality::Array),
            },
            constraints: vec![],
            field_number: None,
        }));

        let asts = vec![make_ast(vec![make_table("Item", vec![array_field])])];
        let ctx = build_ir(&asts);

        let ns = &ctx.files[0].namespaces[0];
        let item = find_struct(ns, "Item").unwrap();
        let tags_field = find_field(item, "tags").unwrap();

        assert!(!tags_field.field_type.is_option);
        assert!(tags_field.field_type.is_list);
        assert!(tags_field.field_type.inner_type.is_some());
    }

    // ========== Constraint to Attribute Tests ==========

    #[test]
    fn test_constraints_to_attributes() {
        let constraints = vec![
            Constraint::PrimaryKey,
            Constraint::Unique,
            Constraint::MaxLength(100),
        ];
        let attributes = convert_constraints_to_attributes(&constraints);

        assert!(attributes.contains(&"Key".to_string()));
        assert!(attributes.contains(&"Index(IsUnique = true)".to_string()));
        assert!(attributes.contains(&"MaxLength(100)".to_string()));
    }

    #[test]
    fn test_foreign_key_not_in_attributes() {
        let constraints = vec![Constraint::ForeignKey(vec!["other".to_string()], None)];
        let attributes = convert_constraints_to_attributes(&constraints);

        assert!(attributes.is_empty());
    }
}
