# PolyTemplate (.ptpl) Language Specification

PolyTemplate is a declarative template language designed for code generation from PolyGen IR (Intermediate Representation). Templates use the `.ptpl` extension and produce one output line per template line, enabling precise line-level source mapping.

---

## Table of Contents

1. [Overview](#1-overview)
2. [Directives](#2-directives)
3. [Expression Interpolation](#3-expression-interpolation)
4. [Filters](#4-filters)
5. [Conditions](#5-conditions)
6. [Include System](#6-include-system)
7. [Context Model](#7-context-model)
8. [TOML Configuration](#8-toml-configuration)
9. [Source Maps](#9-source-maps)
10. [Template Organization](#10-template-organization)
11. [Examples](#11-examples)

---

## 1. Overview

### Design Principles

- **One line in, one line out** — each template line maps to exactly one output line
- **Declarative** — logic is expressed through property access and filters, not imperative code
- **Composable** — templates are split into small, reusable files via `%include`
- **Traceable** — every output line tracks which template and IR node produced it

### Minimal Example

```ptpl
%-- This is a comment (not emitted)
package {{package_name}}
%blank
%for struct in file.all_tables
type {{struct.name}} struct {
%for field in struct.fields
	{{field.name | pascal_case}} {{field.field_type | lang_type}}
%endfor
}
%blank
%endfor
```

---

## 2. Directives

All directives begin with `%` at the start of a line. Directives are not emitted to output.

### 2.1 Conditional: `%if` / `%elif` / `%else` / `%endif`

```ptpl
%if field.is_primary_key
	// This is the primary key
%elif field.is_unique
	// This is a unique field
%else
	// Regular field
%endif
```

- `%elif` can appear zero or more times.
- `%else` is optional and must be the last branch before `%endif`.
- Conditions follow [truthiness rules](#54-truthiness).

### 2.2 Loop: `%for` / `%endfor`

```ptpl
%for field in struct.fields
	{{field.name | pascal_case}} {{field.field_type | lang_type}}
%endfor
```

- The loop variable (`field`) is scoped to the loop body.
- The loop variable is also accessible in nested `%include` templates.
- Iterates over `List` context values.

### 2.3 Include: `%include`

```ptpl
%include "section/struct_block.ptpl" with struct
%include "detail/field.ptpl" with field, indent=1
```

See [Section 6: Include System](#6-include-system) for full details.

### 2.4 Blank Line: `%blank`

```ptpl
}
%blank
func main() {
```

Emits an empty line. Use this instead of literal empty lines to clearly express intent.

### 2.5 Comment: `%--`

```ptpl
%-- This comment is ignored during rendering
%-- Use for template-internal notes
```

Not emitted to output. Not tracked in source maps.

---

## 3. Expression Interpolation

Expressions are enclosed in `{{ }}` within output lines.

### 3.1 Property Access

Dot-separated paths resolve against the current context:

```ptpl
{{struct.name}}                        %-- "Player"
{{field.field_type.type_name}}         %-- "u32"
{{field.foreign_key.target_table_fqn}} %-- "game.character.Skill"
```

If a property does not exist, the expression resolves to an empty string.

### 3.2 Filters (Pipe Syntax)

Filters transform values using the `|` operator. Multiple filters chain left-to-right:

```ptpl
{{field.name | pascal_case}}                       %-- "player_id" -> "PlayerId"
{{struct.name | suffix("Table")}}                  %-- "Item" -> "ItemTable"
{{idx.field_name | pascal_case | prefix("GetBy")}} %-- "player_id" -> "GetByPlayerId"
```

See [Section 4: Filters](#4-filters) for the full list.

---

## 4. Filters

### 4.1 Case Conversion

| Filter | Input | Output | Description |
|--------|-------|--------|-------------|
| `pascal_case` | `player_name` | `PlayerName` | UpperCamelCase |
| `snake_case` | `PlayerName` | `player_name` | lower_snake_case |
| `camel_case` | `player_name` | `playerName` | lowerCamelCase |
| `upper` | `hello` | `HELLO` | UPPERCASE |
| `lower` | `HELLO` | `hello` | lowercase |

### 4.2 String Manipulation

| Filter | Syntax | Input | Output | Description |
|--------|--------|-------|--------|-------------|
| `quote` | `quote` | `value` | `"value"` | Wrap in double quotes |
| `suffix` | `suffix("Table")` | `Item` | `ItemTable` | Append string |
| `prefix` | `prefix("Get")` | `ById` | `GetById` | Prepend string |

### 4.3 Collection Filters

| Filter | Syntax | Description | Example |
|--------|--------|-------------|---------|
| `count` | `count` | Number of items in a list | `{{struct.fields \| count}}` |
| `join` | `join(", ")` | Join list items with separator | `{{names \| join(", ")}}` |

### 4.4 Type Mapping (TOML-Dependent)

These filters look up values from the language's `.toml` configuration file:

| Filter | TOML Section | Description |
|--------|-------------|-------------|
| `lang_type` | `[type_map]` | Map poly type to target language type |
| `binary_read` | `[binary_read]` | Binary deserialization expression |
| `binary_read_option` | `[binary_read.option]` | Binary read for optional types |
| `binary_read_list` | `[binary_read.list]` | Binary read for list types |
| `binary_read_struct` | `[binary_read.struct]` | Binary read for struct types |
| `csv_read` | `[csv_read]` | CSV parse expression |

### 4.5 Semantic Filters

| Filter | Description |
|--------|-------------|
| `format` | Format a default value for the target language |
| `is_embedded` | Returns `"true"` if the struct FQN is used as an embedded type |

### 4.6 Filter Chaining

Filters apply left-to-right. Each filter receives the output of the previous one:

```ptpl
{{field.name | pascal_case | prefix("by")}}
%-- "player_id" -> "PlayerId" -> "byPlayerId"
```

---

## 5. Conditions

### 5.1 Operators

| Operator | Syntax | Precedence |
|----------|--------|-----------|
| `!` (NOT) | `!expr` | Highest |
| `&&` (AND) | `left && right` | Medium |
| `\|\|` (OR) | `left \|\| right` | Lowest |

### 5.2 Simple Property Check

```ptpl
%if field.is_primary_key
%if struct.has_foreign_keys
%if !field.field_type.is_option
```

### 5.3 Compound Conditions

```ptpl
%if field.is_primary_key && field.is_unique
%if field.field_type.is_float || field.field_type.is_unsigned
%if !field.has_foreign_key && field.is_index
```

### 5.4 Truthiness

| Value Type | Truthy when... |
|-----------|----------------|
| `Bool` | `true` |
| `String` | Non-empty |
| `Int` | Non-zero |
| `Float` | Non-zero |
| `List` | Non-empty |
| `Null` | Never (always falsy) |
| IR Objects | Always truthy (existence = true) |

### 5.5 Filter in Condition

```ptpl
%if struct.fqn | is_embedded
	// This struct is embedded in another struct
%endif
```

---

## 6. Include System

### 6.1 Basic Syntax

```
%include "path/to/template" with bindings, indent=N
```

- Path is relative to the language template directory.
- `.ptpl` extension can be omitted.
- Maximum include depth: **16 levels** (circular includes are detected).

### 6.2 Focus Binding

Passes a single context value and auto-infers the binding name:

```ptpl
%include "section/struct_block" with struct
```

The included template can access:
- `struct` — the bound value (inferred from type: Struct, Field, Enum, etc.)
- All parent context bindings are inherited

**Auto-inferred names by type:**

| ContextValue Type | Inferred Name |
|-------------------|---------------|
| Struct | `struct` |
| Field | `field` |
| Namespace | `namespace` |
| Enum | `enum` |
| EnumMember | `member` |
| File | `file` |
| Schema | `schema` |
| TypeRef | `type` |

If the path is a single segment (e.g., `with table`), it's also bound under the original variable name.

### 6.3 Key-Value Bindings

```ptpl
%include "detail/constructor" with struct, indent=1
%include "section/body" with ns, depth=2
```

### 6.4 Indent Parameter

```ptpl
%include "section/methods" with struct, indent=1
```

- `indent=N` adds N levels of indentation (4 spaces each) to **all** output from the included template.
- Indentation is cumulative through nested includes.
- Empty lines remain empty (no padding applied).

### 6.5 Context Inheritance

Included templates inherit all bindings from the parent context:

```ptpl
%-- parent template has: container_name = "GameSchemaContainer"
%for table in file.all_tables
%include "container/validate_fk" with table
%endfor
```

Inside `validate_fk.ptpl`, both `struct` (from `with table`) and `container_name` (inherited) are accessible.

---

## 7. Context Model

The template engine provides typed access to the PolyGen IR model. Every property returns a `ContextValue` which can be:
- Displayed as a string in `{{ }}`
- Tested for truthiness in `%if`
- Iterated in `%for` (if it's a List)

### 7.1 Schema

| Property | Type | Description |
|----------|------|-------------|
| `files` | List\<File\> | All schema files |

### 7.2 File

| Property | Type | Description |
|----------|------|-------------|
| `path` / `file_name` | String | Relative file path |
| `namespaces` | List\<Namespace\> | Top-level namespaces |
| `all_tables` | List\<Struct\> | Flattened list of all structs (excludes `__Enum`) |

### 7.3 Namespace

| Property | Type | Description |
|----------|------|-------------|
| `name` | String | Namespace name |
| `datasource` | String? | Data source annotation value |
| `items` | List\<NamespaceItem\> | Contents (structs, enums, child namespaces, comments) |

### 7.4 NamespaceItem

| Property | Type | Description |
|----------|------|-------------|
| `is_struct` | Bool | True if this item is a struct |
| `is_enum` | Bool | True if this item is an enum |
| `is_comment` | Bool | True if this item is a comment |
| `is_namespace` | Bool | True if this item is a child namespace |
| `as_struct` | Struct | Cast to Struct |
| `as_enum` | Enum | Cast to Enum |
| `as_comment` | String | Cast to comment text |
| `as_namespace` | Namespace | Cast to child Namespace |

### 7.5 Struct

| Property | Type | Description |
|----------|------|-------------|
| `name` | String | Struct name |
| `fqn` | String | Fully qualified name |
| `is_embed` | Bool | True if defined as `embed` |
| `is_readonly` | Bool | True if `@readonly` |
| `datasource` | String? | `@datasource` value |
| `cache_strategy` | String? | `@cache` value |
| `soft_delete_field` | String? | `@soft_delete` value |
| `pack_separator` | String? | `@pack` separator value |
| `header` | List\<StructItem\> | Items before fields (annotations, comments) |
| `items` | List\<StructItem\> | All items including fields |
| `indexes` | List\<Index\> | Index definitions |
| `relations` | List\<Relation\> | Relation definitions |
| **Convenience** | | |
| `fields` | List\<Field\> | Only field items (filtered) |
| `doc_comments` | List\<String\> | Comment strings from header |
| `default_fields` | List\<Field\> | Fields with default values |
| `has_defaults` | Bool | True if any field has a default |
| `has_foreign_keys` | Bool | True if any field has a foreign key |
| `fk_fields` | List\<Field\> | Fields with foreign keys |
| `pk_field_name` | String | Primary key field name (defaults to `"Id"`) |

### 7.6 StructItem

| Property | Type | Description |
|----------|------|-------------|
| `is_field` | Bool | True if field |
| `is_comment` | Bool | True if comment |
| `is_annotation` | Bool | True if annotation |
| `is_embedded_struct` | Bool | True if embedded struct |
| `is_inline_enum` | Bool | True if inline enum |
| `as_field` | Field | Cast to Field |
| `as_comment` | String | Cast to comment text |
| `as_annotation` | Annotation | Cast to Annotation |
| `as_embedded_struct` | Struct | Cast to embedded Struct |
| `as_inline_enum` | Enum | Cast to inline Enum |

### 7.7 Field

| Property | Type | Description |
|----------|------|-------------|
| `name` / `field_name` | String | Field name (snake_case) |
| `field_type` | TypeRef | Type information |
| `attributes` | List\<String\> | Constraint attributes |
| `is_primary_key` | Bool | Has `primary_key` constraint |
| `is_unique` | Bool | Has `unique` constraint |
| `is_index` | Bool | Has `index` constraint |
| `foreign_key` | ForeignKey? | Foreign key definition |
| `has_foreign_key` | Bool | True if foreign key exists |
| `max_length` | Int? | `max_length(N)` value |
| `has_max_length` | Bool | True if max_length constraint exists |
| `default_value` | String? | `default(V)` value |
| `has_default_value` | Bool | True if default exists |
| `range` | Range? | `range(min, max)` definition |
| `has_range` | Bool | True if range constraint exists |
| `regex_pattern` | String? | `regex("pattern")` value |
| `has_regex_pattern` | Bool | True if regex constraint exists |
| `auto_create` | Timezone? | `auto_create` timestamp config |
| `auto_update` | Timezone? | `auto_update` timestamp config |
| `has_auto_update` | Bool | True if auto_update exists |

### 7.8 TypeRef

| Property | Type | Description |
|----------|------|-------------|
| `original` | String | Original syntax (e.g., `"string?"`) |
| `fqn` | String | Fully qualified name |
| `namespace_fqn` | String | Namespace part of FQN |
| `type_name` | String | Base type name (e.g., `"u32"`, `"Player"`) |
| `parent_type_path` | String | Parent type path |
| `is_primitive` | Bool | True for built-in types |
| `is_struct` | Bool | True for struct references |
| `is_enum` | Bool | True for enum references |
| `is_option` | Bool | True if optional (`?`) |
| `is_list` | Bool | True if array (`[]`) |
| `is_float` | Bool | True if `f32` or `f64` |
| `is_unsigned` | Bool | True if `u8`-`u64` |
| `is_string` | Bool | True if `string` |
| `inner_type` / `inner` | TypeRef? | Inner type for Option/List |

### 7.9 Enum

| Property | Type | Description |
|----------|------|-------------|
| `name` | String | Enum name |
| `fqn` | String | Fully qualified name |
| `items` | List\<EnumItem\> | All items (members + comments) |
| `members` | List\<EnumMember\> | Only member items |
| `use_iota` | Bool | True if values are sequential from 0 |
| `first_member_name` | String? | Name of the first member |
| `items_before_first_member` | List\<EnumItem\> | Comments before first member |
| `items_after_first_member` | List\<EnumItem\> | Items after the first member |

### 7.10 EnumMember

| Property | Type | Description |
|----------|------|-------------|
| `name` | String | Member name |
| `value` | Int? | Explicit value |
| `has_value` | Bool | True if value is assigned |

### 7.11 Index

| Property | Type | Description |
|----------|------|-------------|
| `name` | String | Index name (e.g., `"ById"`) |
| `is_unique` | Bool | True for unique indexes |
| `is_composite` | Bool | True for multi-field indexes |
| `field_count` | Int | Number of fields in index |
| `source` | String | Index source (`"annotation"`, etc.) |
| `field_name` | String | First field name (snake_case) |
| `field_type` | TypeRef? | First field's type |

### 7.12 ForeignKey

| Property | Type | Description |
|----------|------|-------------|
| `target_table_fqn` | String | Full target path (e.g., `"game.character.Player"`) |
| `target_field` | String | Target field name (e.g., `"id"`) |
| `target_table_name` | String | Last segment of FQN (e.g., `"Player"`) |
| `alias` | String? | FK alias |

### 7.13 Range

| Property | Type | Description |
|----------|------|-------------|
| `min` | String | Minimum value |
| `max` | String | Maximum value |
| `literal_type` | String | Value type |

### 7.14 Relation

| Property | Type | Description |
|----------|------|-------------|
| `name` | String | Relation name |
| `source_table_fqn` | String | Source table FQN |
| `source_table_name` | String | Source table name |
| `source_field` | String | Source field name |

### 7.15 Annotation

| Property | Type | Description |
|----------|------|-------------|
| `name` | String | Annotation name |
| `positional_args` | List\<String\> | Positional arguments |

### 7.16 Timezone

| Property | Type | Description |
|----------|------|-------------|
| `kind` | String | Timezone kind (e.g., `"utc"`, `"local"`) |
| `name` | String? | Named timezone |

---

## 8. TOML Configuration

Each language has a `{lang}.toml` file that configures type mappings and template settings.

### 8.1 Type Mapping

```toml
[type_map]
u8 = "uint8"
u32 = "uint32"
string = "string"
bool = "bool"

[type_map.optional]
format = "*{{type}}"

[type_map.list]
format = "[]{{type}}"
```

The `lang_type` filter uses this mapping:

1. Look up `type_name` in `[type_map]` → base type
2. If `is_option`, apply `[type_map.optional].format` with `{{type}}` replaced
3. If `is_list`, apply `[type_map.list].format` with `{{type}}` replaced
4. If type is not found in map, use original `type_name`

### 8.2 Template Configuration

```toml
[templates]
main = "go_file.ptpl"
extra = ["go_container_file.ptpl"]
```

- `main`: Entry point template, rendered once per schema file.
- `extra`: Additional templates, each rendered once per schema file with a derived output suffix.

### 8.3 Extra Template Output Naming

The output suffix is derived from the template filename:

| Template | Language | Suffix | Output |
|----------|---------|--------|--------|
| `go_container_file.ptpl` | `go` | `_container` | `game_schema_container.go` |
| `csharp_readers_file.ptpl` | `csharp` | `_readers` | `game_schema_readers.cs` |

Rule: strip `{lang}_` prefix, strip `_file` suffix → `_remainder`.

---

## 9. Source Maps

Every output line is tracked in a `.ptpl.map` JSON file:

```json
{
  "entries": [
    {
      "template_file": "go_container_file.ptpl",
      "template_line": 3,
      "include_stack": [
        "go_container_file.ptpl",
        "container/data_table.ptpl"
      ],
      "ir_path": "game.character.Player"
    }
  ]
}
```

Source maps are generated in **preview mode** only and enable:
- Click-to-navigate from generated code to template source
- Debugging which template produced each output line
- Understanding the full include chain

---

## 10. Template Organization

Recommended directory structure for a language:

```
templates/
└── go/
    ├── go.toml                    # Language configuration
    ├── go_file.ptpl               # Main template (entry point)
    ├── go_container_file.ptpl     # Extra template (entry point)
    ├── section/                   # Top-level code blocks
    │   ├── package_block.ptpl
    │   ├── struct_block.ptpl
    │   └── enum_block.ptpl
    ├── detail/                    # Fine-grained components
    │   ├── field_declaration.ptpl
    │   ├── constructor.ptpl
    │   └── enum_members_iota.ptpl
    └── container/                 # Container-specific templates
        ├── data_table.ptpl
        ├── constructor.ptpl
        ├── accessors.ptpl
        ├── validate.ptpl
        └── validate_fk.ptpl
```

**Naming conventions:**
- Entry points: `{lang}_{purpose}_file.ptpl`
- Sections: `section/{name}.ptpl` — major code blocks
- Details: `detail/{name}.ptpl` — small, reusable fragments
- Feature dirs: `container/`, `loader/` — feature-specific templates

---

## 11. Examples

### 11.1 Simple Struct Generation

```ptpl
%-- go_file.ptpl
// Code generated by PolyGen. DO NOT EDIT.
// Source: {{source_path}}
%blank
package {{package_name}}
%blank
%for ns in file.namespaces
%for item in ns.items
%if item.is_struct
%include "section/struct_block" with item.as_struct
%endif
%if item.is_enum
%include "section/enum_block" with item.as_enum
%endif
%endfor
%endfor
```

### 11.2 Field with Type Mapping

```ptpl
%-- detail/field_declaration.ptpl
%if field.field_type.is_option
	{{field.name | pascal_case}} {{field.field_type | lang_type}} // optional
%else
	{{field.name | pascal_case}} {{field.field_type | lang_type}}
%endif
```

### 11.3 Validation with Type-Based Branching

```ptpl
%if field.has_range
%if field.field_type.is_float
		if !ValidateRangeFloat(row.{{field.name | pascal_case}}, {{field.range.min}}, {{field.range.max}}) {
%elif field.field_type.is_unsigned
		if !ValidateRangeUint(row.{{field.name | pascal_case}}, {{field.range.min}}, {{field.range.max}}) {
%else
		if !ValidateRangeInt(row.{{field.name | pascal_case}}, {{field.range.min}}, {{field.range.max}}) {
%endif
			result.AddError(RangeError("{{struct.name}}", "{{field.name | pascal_case}}", rowKey, {{field.range.min}}, {{field.range.max}}, row.{{field.name | pascal_case}}))
		}
%endif
```

### 11.4 Filter Chaining for Naming

```ptpl
%-- Combining multiple filters to build complex names
{{idx.field_name | pascal_case | prefix("by")}}     %-- "player_id" -> "byPlayerId"
{{struct.name | suffix("Table")}}                    %-- "Item" -> "ItemTable"
{{struct.name | pascal_case | suffix("s")}}          %-- "Item" -> "Items"
{{fk.target_field | pascal_case | prefix("GetBy")}}  %-- "id" -> "GetById"
```

### 11.5 Foreign Key Validation

```ptpl
%if struct.has_foreign_keys
	for _, row := range t.rows {
		rowKey := fmt.Sprintf("%v", row.{{struct.pk_field_name | pascal_case}})
%for field in struct.fk_fields
		if container.{{field.foreign_key.target_table_name | suffix("s")}}.{{field.foreign_key.target_field | pascal_case | prefix("GetBy")}}(row.{{field.name | pascal_case}}) == nil {
			result.AddError(ForeignKeyError("{{struct.name}}", "{{field.name | pascal_case}}", rowKey, "{{field.foreign_key.target_table_name}}", fmt.Sprintf("%v", row.{{field.name | pascal_case}})))
		}
%endfor
	}
%else
	_ = container // Suppress unused variable warning
%endif
```

---

## Appendix: Quick Reference Card

```
DIRECTIVES
  %if condition          Conditional start
  %elif condition        Additional branch
  %else                  Fallback branch
  %endif                 Close conditional
  %for var in list       Loop start
  %endfor                Close loop
  %include "path" ...    Include template
  %blank                 Empty output line
  %-- text               Comment (not emitted)

EXPRESSIONS
  {{path.to.prop}}                 Property access
  {{expr | filter}}                Single filter
  {{expr | f1 | f2 | f3}}         Filter chain

FILTERS (String)
  pascal_case  snake_case  camel_case  upper  lower
  quote  suffix("s")  prefix("Get")

FILTERS (TOML)
  lang_type  binary_read  csv_read  format

FILTERS (Collection)
  count  join(", ")

CONDITIONS
  property.path          Truthiness check
  !expr                  Negation
  a && b                 Logical AND
  a || b                 Logical OR

INCLUDE
  %include "path"                          Basic
  %include "path" with struct              Focus binding
  %include "path" with struct, indent=1    With indentation
```

---

*Specification version: 1.0 — 2026-01-31*
