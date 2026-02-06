<script lang="ts">
  /**
   * TypeMappingEditor - Editable table for type mappings in the language wizard
   */

  interface Props {
    typeMap: Record<string, string>;
    optionalFormat: string;
    listFormat: string;
    onTypeMapChange?: (map: Record<string, string>) => void;
    onOptionalFormatChange?: (format: string) => void;
    onListFormatChange?: (format: string) => void;
  }

  let {
    typeMap = $bindable({}),
    optionalFormat = $bindable("{{type}}?"),
    listFormat = $bindable("{{type}}[]"),
    onTypeMapChange,
    onOptionalFormatChange,
    onListFormatChange,
  }: Props = $props();

  // Poly primitive types
  const POLY_TYPES = [
    { name: "string", desc: "Text" },
    { name: "bool", desc: "Boolean" },
    { name: "bytes", desc: "Binary data" },
    { name: "u8", desc: "Unsigned 8-bit" },
    { name: "u16", desc: "Unsigned 16-bit" },
    { name: "u32", desc: "Unsigned 32-bit" },
    { name: "u64", desc: "Unsigned 64-bit" },
    { name: "i8", desc: "Signed 8-bit" },
    { name: "i16", desc: "Signed 16-bit" },
    { name: "i32", desc: "Signed 32-bit" },
    { name: "i64", desc: "Signed 64-bit" },
    { name: "f32", desc: "32-bit float" },
    { name: "f64", desc: "64-bit float" },
  ];

  // Handle type mapping change
  function handleTypeChange(polyType: string, targetType: string) {
    typeMap = { ...typeMap, [polyType]: targetType };
    onTypeMapChange?.(typeMap);
  }

  // Handle optional format change
  function handleOptionalChange(e: Event) {
    const target = e.target as HTMLInputElement;
    optionalFormat = target.value;
    onOptionalFormatChange?.(optionalFormat);
  }

  // Handle list format change
  function handleListChange(e: Event) {
    const target = e.target as HTMLInputElement;
    listFormat = target.value;
    onListFormatChange?.(listFormat);
  }
</script>

<div class="type-mapping-editor">
  <div class="section">
    <h4>Primitive Type Mappings</h4>
    <p class="hint">Map PolyGen types to your target language types</p>

    <div class="type-table">
      <div class="type-header">
        <span class="col-poly">Poly Type</span>
        <span class="col-target">Target Type</span>
      </div>
      {#each POLY_TYPES as polyType}
        <div class="type-row">
          <span class="col-poly">
            <code>{polyType.name}</code>
            <span class="type-desc">{polyType.desc}</span>
          </span>
          <input
            class="col-target"
            type="text"
            value={typeMap[polyType.name] || ""}
            placeholder={polyType.name}
            oninput={(e) => handleTypeChange(polyType.name, (e.target as HTMLInputElement).value)}
          />
        </div>
      {/each}
    </div>
  </div>

  <div class="section">
    <h4>Wrapper Formats</h4>
    <p class="hint">Use <code>{"{{type}}"}</code> as placeholder for the inner type</p>

    <div class="format-row">
      <label for="optional-format">Optional:</label>
      <input
        id="optional-format"
        type="text"
        value={optionalFormat}
        placeholder={`{{type}}?`}
        oninput={handleOptionalChange}
      />
      <span class="example">e.g., <code>Optional&lt;string&gt;</code> or <code>string?</code></span>
    </div>

    <div class="format-row">
      <label for="list-format">List/Array:</label>
      <input
        id="list-format"
        type="text"
        value={listFormat}
        placeholder={`{{type}}[]`}
        oninput={handleListChange}
      />
      <span class="example">e.g., <code>List&lt;string&gt;</code> or <code>string[]</code></span>
    </div>
  </div>
</div>

<style>
  .type-mapping-editor {
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  .section h4 {
    margin: 0 0 0.25rem 0;
    font-size: 0.875rem;
    color: var(--text-primary);
  }

  .section .hint {
    margin: 0 0 0.75rem 0;
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .section .hint code {
    background-color: var(--bg-primary);
    padding: 0.125rem 0.25rem;
    border-radius: 3px;
  }

  .type-table {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border);
    border-radius: 6px;
    overflow: hidden;
  }

  .type-header {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.5rem;
    padding: 0.5rem 0.75rem;
    background-color: var(--bg-primary);
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-secondary);
    border-bottom: 1px solid var(--border);
  }

  .type-row {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.5rem;
    padding: 0.375rem 0.75rem;
    align-items: center;
    border-bottom: 1px solid var(--border);
  }

  .type-row:last-child {
    border-bottom: none;
  }

  .type-row:hover {
    background-color: var(--bg-hover);
  }

  .col-poly {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }

  .col-poly code {
    font-family: "Consolas", "Monaco", monospace;
    font-size: 0.8125rem;
    color: var(--accent);
    background-color: var(--bg-primary);
    padding: 0.125rem 0.375rem;
    border-radius: 3px;
  }

  .type-desc {
    font-size: 0.6875rem;
    color: var(--text-muted);
  }

  .col-target {
    font-family: "Consolas", "Monaco", monospace;
    font-size: 0.8125rem;
    padding: 0.375rem 0.5rem;
    background-color: var(--bg-primary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    width: 100%;
  }

  .col-target:focus {
    outline: none;
    border-color: var(--accent);
  }

  .format-row {
    display: grid;
    grid-template-columns: 100px 200px 1fr;
    gap: 0.75rem;
    align-items: center;
    margin-bottom: 0.5rem;
  }

  .format-row label {
    font-size: 0.8125rem;
    color: var(--text-secondary);
  }

  .format-row input {
    font-family: "Consolas", "Monaco", monospace;
    font-size: 0.8125rem;
    padding: 0.375rem 0.5rem;
    background-color: var(--bg-primary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
  }

  .format-row input:focus {
    outline: none;
    border-color: var(--accent);
  }

  .example {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .example code {
    background-color: var(--bg-primary);
    padding: 0.125rem 0.25rem;
    border-radius: 3px;
  }
</style>
