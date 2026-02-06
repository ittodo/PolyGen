<script lang="ts">
  /**
   * ContextReferencePanel - Shows context-appropriate variables, filters, and functions
   * based on the currently open template file.
   */

  interface Props {
    filePath: string;
    onInsert?: (text: string) => void;
  }

  let { filePath, onInsert }: Props = $props();

  // Search query
  let searchQuery = $state("");

  // Expanded sections
  let expandedSections = $state({
    variables: true,
    filters: true,
    functions: false,
  });

  // Type definitions for IR model
  interface PropertyDef {
    name: string;
    type: string;
    desc: string;
  }

  interface TypeDef {
    name: string;
    desc: string;
    properties: PropertyDef[];
  }

  // Template context inferred from file path
  interface TemplateContext {
    contextType: string;
    description: string;
    variables: Array<{ name: string; type: string }>;
    isRhai: boolean;
    isToml: boolean;
  }

  // IR Type definitions
  const IR_TYPES: Record<string, TypeDef> = {
    FileDef: {
      name: "FileDef",
      desc: "Root file object with namespaces",
      properties: [
        { name: "path", type: "string", desc: "File path" },
        { name: "namespaces", type: "NamespaceDef[]", desc: "Namespace list" },
        { name: "imports", type: "string[]", desc: "Import paths" },
      ],
    },
    SchemaContext: {
      name: "SchemaContext",
      desc: "Full schema context across all files",
      properties: [
        { name: "files", type: "FileDef[]", desc: "All files" },
        { name: "all_structs", type: "StructDef[]", desc: "All structs" },
        { name: "all_enums", type: "EnumDef[]", desc: "All enums" },
      ],
    },
    NamespaceDef: {
      name: "NamespaceDef",
      desc: "Namespace containing types",
      properties: [
        { name: "name", type: "string", desc: "Namespace name" },
        { name: "fqn", type: "string", desc: "Fully qualified name" },
        { name: "structs", type: "StructDef[]", desc: "Struct definitions" },
        { name: "enums", type: "EnumDef[]", desc: "Enum definitions" },
        { name: "children", type: "NamespaceDef[]", desc: "Child namespaces" },
        { name: "doc_comment", type: "string?", desc: "Documentation comment" },
        { name: "annotations", type: "Annotation[]", desc: "Annotations" },
      ],
    },
    StructDef: {
      name: "StructDef",
      desc: "Struct/class/embed definition",
      properties: [
        { name: "name", type: "string", desc: "Struct name" },
        { name: "fqn", type: "string", desc: "Fully qualified name" },
        { name: "fields", type: "FieldDef[]", desc: "Field list" },
        { name: "is_embed", type: "bool", desc: "Is embed type" },
        { name: "is_table", type: "bool", desc: "Is table type" },
        { name: "primary_key_field", type: "FieldDef?", desc: "Primary key field" },
        { name: "pack_separator", type: "string?", desc: "@pack separator" },
        { name: "indexes", type: "IndexDef[]", desc: "Index definitions" },
        { name: "doc_comment", type: "string?", desc: "Documentation comment" },
        { name: "annotations", type: "Annotation[]", desc: "Annotations" },
        { name: "inline_enums", type: "EnumDef[]", desc: "Inline enum definitions" },
      ],
    },
    FieldDef: {
      name: "FieldDef",
      desc: "Field definition within a struct",
      properties: [
        { name: "name", type: "string", desc: "Field name" },
        { name: "field_type", type: "TypeRef", desc: "Field type reference" },
        { name: "is_optional", type: "bool", desc: "Is optional (?)" },
        { name: "is_array", type: "bool", desc: "Is array ([])" },
        { name: "is_primary_key", type: "bool", desc: "Is primary key" },
        { name: "is_foreign_key", type: "bool", desc: "Is foreign key" },
        { name: "default_value", type: "string?", desc: "Default value" },
        { name: "constraints", type: "Constraint[]", desc: "Constraints" },
        { name: "doc_comment", type: "string?", desc: "Documentation comment" },
        { name: "inline_comment", type: "string?", desc: "Inline comment" },
      ],
    },
    TypeRef: {
      name: "TypeRef",
      desc: "Type reference with metadata",
      properties: [
        { name: "base_type", type: "string", desc: "Base type name" },
        { name: "fqn", type: "string?", desc: "Fully qualified name if custom" },
        { name: "is_optional", type: "bool", desc: "Is optional" },
        { name: "is_array", type: "bool", desc: "Is array" },
        { name: "is_primitive", type: "bool", desc: "Is primitive type" },
        { name: "type_params", type: "TypeRef[]", desc: "Generic type parameters" },
      ],
    },
    EnumDef: {
      name: "EnumDef",
      desc: "Enum definition",
      properties: [
        { name: "name", type: "string", desc: "Enum name" },
        { name: "fqn", type: "string", desc: "Fully qualified name" },
        { name: "variants", type: "EnumVariant[]", desc: "Variants" },
        { name: "doc_comment", type: "string?", desc: "Documentation comment" },
        { name: "annotations", type: "Annotation[]", desc: "Annotations" },
      ],
    },
    EnumVariant: {
      name: "EnumVariant",
      desc: "Enum variant",
      properties: [
        { name: "name", type: "string", desc: "Variant name" },
        { name: "value", type: "i64", desc: "Numeric value" },
        { name: "doc_comment", type: "string?", desc: "Documentation comment" },
        { name: "inline_comment", type: "string?", desc: "Inline comment" },
      ],
    },
    IndexDef: {
      name: "IndexDef",
      desc: "Index definition",
      properties: [
        { name: "name", type: "string", desc: "Index name" },
        { name: "fields", type: "string[]", desc: "Field names" },
        { name: "unique", type: "bool", desc: "Is unique index" },
      ],
    },
    RenderConfig: {
      name: "RenderConfig",
      desc: "Language configuration from .toml",
      properties: [
        { name: "type_map", type: "Map<string, string>", desc: "Type mappings" },
        { name: "binary_read", type: "Map<string, string>", desc: "Binary read expressions" },
        { name: "binary_write", type: "Map<string, string>", desc: "Binary write expressions" },
        { name: "csv_read", type: "Map<string, string>", desc: "CSV read expressions" },
        { name: "extension", type: "string", desc: "File extension" },
        { name: "indent", type: "string", desc: "Indent string" },
        { name: "namespace_separator", type: "string", desc: "Namespace separator" },
      ],
    },
  };

  // Filter definitions
  const FILTERS = [
    { name: "lang_type", desc: "Convert to language-specific type", usage: "| lang_type" },
    { name: "pascal_case", desc: "Convert to PascalCase", usage: "| pascal_case" },
    { name: "camel_case", desc: "Convert to camelCase", usage: "| camel_case" },
    { name: "snake_case", desc: "Convert to snake_case", usage: "| snake_case" },
    { name: "screaming_snake_case", desc: "Convert to SCREAMING_SNAKE", usage: "| screaming_snake_case" },
    { name: "kebab_case", desc: "Convert to kebab-case", usage: "| kebab_case" },
    { name: "upper", desc: "Convert to uppercase", usage: "| upper" },
    { name: "lower", desc: "Convert to lowercase", usage: "| lower" },
    { name: "trim", desc: "Trim whitespace", usage: "| trim" },
    { name: "count", desc: "Get array length", usage: "| count" },
    { name: "join", desc: "Join array with separator", usage: '| join(",")' },
    { name: "first", desc: "Get first element", usage: "| first" },
    { name: "last", desc: "Get last element", usage: "| last" },
    { name: "default", desc: "Default value if empty", usage: '| default("value")' },
    { name: "indent", desc: "Add indentation levels", usage: "| indent(1)" },
  ];

  // Rhai function definitions
  const RHAI_FUNCTIONS = [
    { name: "to_pascal_case(s)", desc: "Convert string to PascalCase" },
    { name: "to_camel_case(s)", desc: "Convert string to camelCase" },
    { name: "to_snake_case(s)", desc: "Convert string to snake_case" },
    { name: "to_screaming_snake_case(s)", desc: "Convert string to SCREAMING_SNAKE" },
    { name: "to_kebab_case(s)", desc: "Convert string to kebab-case" },
    { name: "indent_text(text, level)", desc: "Indent text by level" },
    { name: "emit(text)", desc: "Output text to result" },
    { name: "include(path)", desc: "Include another template" },
    { name: "len(x)", desc: "Get length of array/string" },
    { name: "is_empty(x)", desc: "Check if empty" },
    { name: "contains(arr, item)", desc: "Check if array contains item" },
    { name: "starts_with(s, prefix)", desc: "Check string prefix" },
    { name: "ends_with(s, suffix)", desc: "Check string suffix" },
    { name: "split(s, delimiter)", desc: "Split string into array" },
    { name: "join(arr, separator)", desc: "Join array into string" },
    { name: "type_of(x)", desc: "Get type of value as string" },
    { name: "print(x)", desc: "Print to debug console" },
  ];

  // Infer context from file path
  function inferContext(path: string): TemplateContext {
    if (!path) {
      return {
        contextType: "unknown",
        description: "No file selected",
        variables: [],
        isRhai: false,
        isToml: false,
      };
    }

    const lower = path.toLowerCase().replace(/\\/g, "/");

    // TOML config file
    if (lower.endsWith(".toml")) {
      return {
        contextType: "config",
        description: "Language configuration file",
        variables: [],
        isRhai: false,
        isToml: true,
      };
    }

    // Rhai utility file
    if (lower.includes("rhai_utils/") || lower.endsWith(".rhai")) {
      return {
        contextType: "rhai_utils",
        description: "Rhai utility functions",
        variables: [{ name: "config", type: "RenderConfig" }],
        isRhai: true,
        isToml: false,
      };
    }

    // Main template file (*_file.ptpl)
    if (lower.endsWith("_file.ptpl")) {
      return {
        contextType: "main",
        description: "Main template - file level rendering",
        variables: [
          { name: "file", type: "FileDef" },
          { name: "schema", type: "SchemaContext" },
          { name: "config", type: "RenderConfig" },
        ],
        isRhai: false,
        isToml: false,
      };
    }

    // Namespace section
    if (lower.includes("section/namespace") || lower.includes("namespace_block")) {
      return {
        contextType: "namespace",
        description: "Namespace block rendering",
        variables: [
          { name: "namespace", type: "NamespaceDef" },
          { name: "config", type: "RenderConfig" },
        ],
        isRhai: false,
        isToml: false,
      };
    }

    // Struct/class section
    if (lower.includes("section/struct") || lower.includes("section/class") ||
        lower.includes("struct_block") || lower.includes("class_block")) {
      return {
        contextType: "struct",
        description: "Struct/class block rendering",
        variables: [
          { name: "struct", type: "StructDef" },
          { name: "config", type: "RenderConfig" },
        ],
        isRhai: false,
        isToml: false,
      };
    }

    // Enum section
    if (lower.includes("section/enum") || lower.includes("enum_block")) {
      return {
        contextType: "enum",
        description: "Enum block rendering",
        variables: [
          { name: "enum", type: "EnumDef" },
          { name: "config", type: "RenderConfig" },
        ],
        isRhai: false,
        isToml: false,
      };
    }

    // Field detail
    if (lower.includes("detail/field") || lower.includes("struct_field") ||
        lower.includes("field_") || lower.includes("_field.")) {
      return {
        contextType: "field",
        description: "Field rendering",
        variables: [
          { name: "field", type: "FieldDef" },
          { name: "struct", type: "StructDef" },
          { name: "config", type: "RenderConfig" },
        ],
        isRhai: false,
        isToml: false,
      };
    }

    // Struct detail
    if (lower.includes("detail/struct") || lower.includes("struct_body") ||
        lower.includes("class_body")) {
      return {
        contextType: "struct_detail",
        description: "Struct detail rendering",
        variables: [
          { name: "struct", type: "StructDef" },
          { name: "config", type: "RenderConfig" },
        ],
        isRhai: false,
        isToml: false,
      };
    }

    // Enum variant detail
    if (lower.includes("detail/variant") || lower.includes("enum_variant")) {
      return {
        contextType: "variant",
        description: "Enum variant rendering",
        variables: [
          { name: "variant", type: "EnumVariant" },
          { name: "enum", type: "EnumDef" },
          { name: "config", type: "RenderConfig" },
        ],
        isRhai: false,
        isToml: false,
      };
    }

    // Container template
    if (lower.includes("container")) {
      return {
        contextType: "container",
        description: "Container class rendering",
        variables: [
          { name: "file", type: "FileDef" },
          { name: "schema", type: "SchemaContext" },
          { name: "config", type: "RenderConfig" },
        ],
        isRhai: false,
        isToml: false,
      };
    }

    // Loaders template
    if (lower.includes("loader")) {
      return {
        contextType: "loader",
        description: "Data loader rendering",
        variables: [
          { name: "struct", type: "StructDef" },
          { name: "config", type: "RenderConfig" },
        ],
        isRhai: false,
        isToml: false,
      };
    }

    // Default for .ptpl files
    if (lower.endsWith(".ptpl")) {
      return {
        contextType: "template",
        description: "Template file",
        variables: [
          { name: "file", type: "FileDef" },
          { name: "schema", type: "SchemaContext" },
          { name: "config", type: "RenderConfig" },
        ],
        isRhai: false,
        isToml: false,
      };
    }

    return {
      contextType: "unknown",
      description: "Unknown file type",
      variables: [],
      isRhai: false,
      isToml: false,
    };
  }

  // Computed context
  let context = $derived(inferContext(filePath));

  // Filter items by search query
  function matchesSearch(text: string): boolean {
    if (!searchQuery) return true;
    return text.toLowerCase().includes(searchQuery.toLowerCase());
  }

  // Handle insert click
  function handleInsert(text: string, isFilter = false) {
    if (context.isRhai) {
      // In Rhai files, insert as plain text
      onInsert?.(text);
    } else if (context.isToml) {
      onInsert?.(text);
    } else {
      // In ptpl files, wrap in interpolation for variables
      if (isFilter) {
        onInsert?.(` ${text}`);
      } else {
        onInsert?.(`{{${text}}}`);
      }
    }
  }

  // Copy to clipboard
  function copyToClipboard(text: string) {
    navigator.clipboard.writeText(text);
  }

  // Toggle section
  function toggleSection(section: keyof typeof expandedSections) {
    expandedSections[section] = !expandedSections[section];
  }
</script>

<div class="context-panel">
  <!-- Header -->
  <div class="panel-header">
    <div class="context-info">
      <span class="context-type">{context.contextType}</span>
      <span class="context-desc">{context.description}</span>
    </div>
    {#if filePath}
      <div class="file-path" title={filePath}>
        {filePath.split(/[/\\]/).pop()}
      </div>
    {/if}
  </div>

  <!-- Search -->
  <div class="search-box">
    <input
      type="text"
      placeholder="Search..."
      bind:value={searchQuery}
    />
  </div>

  <div class="panel-content">
    {#if context.isToml}
      <!-- TOML config guidance -->
      <div class="toml-guide">
        <p>Language configuration file. Common sections:</p>
        <ul>
          <li><code>[language]</code> - ID, name, version</li>
          <li><code>[code_generation]</code> - extension, indent</li>
          <li><code>[type_map]</code> - Type mappings</li>
          <li><code>[binary_read]</code> - Binary read expressions</li>
          <li><code>[csv_read]</code> - CSV read expressions</li>
        </ul>
      </div>
    {:else}
      <!-- Variables Section -->
      {#if context.variables.length > 0}
        <div class="section">
          <button
            class="section-header"
            onclick={() => toggleSection("variables")}
          >
            <span class="section-icon">{expandedSections.variables ? "▼" : "▶"}</span>
            <span class="section-title">Variables</span>
            <span class="section-count">{context.variables.length}</span>
          </button>

          {#if expandedSections.variables}
            <div class="section-content">
              {#each context.variables as variable}
                {@const typeDef = IR_TYPES[variable.type]}
                {#if matchesSearch(variable.name) || (typeDef && typeDef.properties.some(p => matchesSearch(p.name)))}
                  <div class="variable-item">
                    <div class="variable-header">
                      <button
                        class="variable-name"
                        onclick={() => handleInsert(variable.name)}
                        title="Click to insert"
                      >
                        {variable.name}
                      </button>
                      <span class="variable-type">{variable.type}</span>
                      <button
                        class="copy-btn"
                        onclick={() => copyToClipboard(variable.name)}
                        title="Copy to clipboard"
                      >
                        📋
                      </button>
                    </div>

                    {#if typeDef}
                      <div class="properties">
                        {#each typeDef.properties.filter(p => matchesSearch(p.name) || matchesSearch(variable.name)) as prop}
                          <button
                            class="property-item"
                            onclick={() => handleInsert(`${variable.name}.${prop.name}`)}
                            title={prop.desc}
                          >
                            <span class="prop-name">.{prop.name}</span>
                            <span class="prop-type">{prop.type}</span>
                          </button>
                        {/each}
                      </div>
                    {/if}
                  </div>
                {/if}
              {/each}
            </div>
          {/if}
        </div>
      {/if}

      <!-- Filters Section (only for ptpl) -->
      {#if !context.isRhai}
        <div class="section">
          <button
            class="section-header"
            onclick={() => toggleSection("filters")}
          >
            <span class="section-icon">{expandedSections.filters ? "▼" : "▶"}</span>
            <span class="section-title">Filters</span>
            <span class="section-count">{FILTERS.length}</span>
          </button>

          {#if expandedSections.filters}
            <div class="section-content">
              {#each FILTERS.filter(f => matchesSearch(f.name) || matchesSearch(f.desc)) as filter}
                <button
                  class="filter-item"
                  onclick={() => handleInsert(filter.usage, true)}
                  title={filter.desc}
                >
                  <span class="filter-name">{filter.name}</span>
                  <span class="filter-desc">{filter.desc}</span>
                </button>
              {/each}
            </div>
          {/if}
        </div>
      {/if}

      <!-- Functions Section -->
      <div class="section">
        <button
          class="section-header"
          onclick={() => toggleSection("functions")}
        >
          <span class="section-icon">{expandedSections.functions ? "▼" : "▶"}</span>
          <span class="section-title">{context.isRhai ? "Rhai Functions" : "Functions"}</span>
          <span class="section-count">{RHAI_FUNCTIONS.length}</span>
        </button>

        {#if expandedSections.functions}
          <div class="section-content">
            {#each RHAI_FUNCTIONS.filter(f => matchesSearch(f.name) || matchesSearch(f.desc)) as func}
              <button
                class="function-item"
                onclick={() => {
                  const funcName = func.name.split("(")[0];
                  onInsert?.(funcName);
                }}
                title={func.desc}
              >
                <span class="func-name">{func.name}</span>
                <span class="func-desc">{func.desc}</span>
              </button>
            {/each}
          </div>
        {/if}
      </div>
    {/if}
  </div>
</div>

<style>
  .context-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background-color: var(--bg-secondary);
    border-left: 1px solid var(--border);
    font-size: 0.8125rem;
    overflow: hidden;
  }

  .panel-header {
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .context-info {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .context-type {
    font-weight: 600;
    color: var(--accent);
    text-transform: uppercase;
    font-size: 0.6875rem;
    padding: 0.125rem 0.375rem;
    background-color: var(--bg-primary);
    border-radius: 3px;
  }

  .context-desc {
    color: var(--text-secondary);
    font-size: 0.75rem;
  }

  .file-path {
    font-family: "Consolas", "Monaco", monospace;
    font-size: 0.6875rem;
    color: var(--text-muted);
    margin-top: 0.25rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .search-box {
    padding: 0.5rem;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .search-box input {
    width: 100%;
    padding: 0.375rem 0.5rem;
    font-size: 0.75rem;
    background-color: var(--bg-primary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
  }

  .search-box input:focus {
    outline: none;
    border-color: var(--accent);
  }

  .panel-content {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
  }

  .section {
    border-bottom: 1px solid var(--border);
  }

  .section-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
    padding: 0.5rem 0.75rem;
    background: none;
    border: none;
    cursor: pointer;
    text-align: left;
    color: var(--text-primary);
    font-weight: 500;
    font-size: 0.75rem;
  }

  .section-header:hover {
    background-color: var(--bg-hover);
  }

  .section-icon {
    font-size: 0.625rem;
    color: var(--text-muted);
  }

  .section-title {
    flex: 1;
  }

  .section-count {
    font-size: 0.6875rem;
    color: var(--text-muted);
    background-color: var(--bg-primary);
    padding: 0.0625rem 0.375rem;
    border-radius: 10px;
  }

  .section-content {
    padding: 0.25rem 0;
  }

  /* Variable items */
  .variable-item {
    padding: 0.25rem 0.5rem 0.5rem;
    border-bottom: 1px solid var(--border);
  }

  .variable-item:last-child {
    border-bottom: none;
  }

  .variable-header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.25rem;
  }

  .variable-name {
    font-family: "Consolas", "Monaco", monospace;
    font-weight: 600;
    color: var(--accent);
    background: none;
    border: none;
    cursor: pointer;
    padding: 0.125rem 0.25rem;
    border-radius: 3px;
    font-size: 0.8125rem;
  }

  .variable-name:hover {
    background-color: var(--bg-hover);
  }

  .variable-type {
    font-family: "Consolas", "Monaco", monospace;
    font-size: 0.6875rem;
    color: var(--text-muted);
  }

  .copy-btn {
    background: none;
    border: none;
    cursor: pointer;
    padding: 0.125rem;
    font-size: 0.75rem;
    opacity: 0.5;
    margin-left: auto;
  }

  .copy-btn:hover {
    opacity: 1;
  }

  .properties {
    display: flex;
    flex-direction: column;
    gap: 0.125rem;
    margin-left: 1rem;
  }

  .property-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.25rem 0.5rem;
    background: none;
    border: none;
    cursor: pointer;
    text-align: left;
    border-radius: 3px;
    width: 100%;
  }

  .property-item:hover {
    background-color: var(--bg-hover);
  }

  .prop-name {
    font-family: "Consolas", "Monaco", monospace;
    color: var(--text-primary);
    font-size: 0.75rem;
  }

  .prop-type {
    font-family: "Consolas", "Monaco", monospace;
    font-size: 0.625rem;
    color: var(--text-muted);
    margin-left: auto;
  }

  /* Filter and function items */
  .filter-item,
  .function-item {
    display: flex;
    flex-direction: column;
    gap: 0.125rem;
    padding: 0.375rem 0.75rem;
    background: none;
    border: none;
    cursor: pointer;
    text-align: left;
    width: 100%;
  }

  .filter-item:hover,
  .function-item:hover {
    background-color: var(--bg-hover);
  }

  .filter-name,
  .func-name {
    font-family: "Consolas", "Monaco", monospace;
    color: var(--accent);
    font-size: 0.75rem;
  }

  .filter-desc,
  .func-desc {
    font-size: 0.6875rem;
    color: var(--text-muted);
  }

  /* TOML guide */
  .toml-guide {
    padding: 0.75rem;
    color: var(--text-secondary);
    font-size: 0.75rem;
  }

  .toml-guide p {
    margin: 0 0 0.5rem 0;
  }

  .toml-guide ul {
    margin: 0;
    padding-left: 1.25rem;
  }

  .toml-guide li {
    margin: 0.25rem 0;
  }

  .toml-guide code {
    background-color: var(--bg-primary);
    padding: 0.125rem 0.25rem;
    border-radius: 3px;
    font-size: 0.6875rem;
  }
</style>
