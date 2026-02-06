<script lang="ts">
  /**
   * LanguageSettingsPanel - Edit language configuration (type mappings, formats)
   */
  import { invoke } from "@tauri-apps/api/core";

  interface Props {
    lang: string;
    templatesDir?: string;
    onLog?: (message: string) => void;
  }

  let { lang, templatesDir, onLog }: Props = $props();

  // Configuration state
  let extension = $state("");
  let typeMap = $state<Record<string, string>>({});
  let optionalFormat = $state("");
  let listFormat = $state("");
  let nonPrimitiveFormat = $state("");

  // UI state
  let isLoading = $state(false);
  let isSaving = $state(false);
  let isModified = $state(false);
  let error = $state("");
  let originalConfig = $state<string>("");

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
    { name: "timestamp", desc: "Date/time" },
  ];

  // Load configuration when lang changes
  $effect(() => {
    if (lang) {
      loadConfig();
    }
  });

  // Check if modified
  $effect(() => {
    const currentConfig = JSON.stringify({
      extension,
      typeMap,
      optionalFormat,
      listFormat,
      nonPrimitiveFormat,
    });
    isModified = currentConfig !== originalConfig;
  });

  async function loadConfig() {
    if (!lang) return;

    isLoading = true;
    error = "";

    try {
      const config = await invoke<{
        extension: string;
        type_map: Record<string, string>;
        optional_format: string;
        list_format: string;
        non_primitive_format: string | null;
      }>("read_language_config", {
        lang,
        templatesDir: templatesDir || null,
      });

      extension = config.extension;
      typeMap = { ...config.type_map };
      optionalFormat = config.optional_format;
      listFormat = config.list_format;
      nonPrimitiveFormat = config.non_primitive_format || "";

      // Store original for comparison
      originalConfig = JSON.stringify({
        extension,
        typeMap,
        optionalFormat,
        listFormat,
        nonPrimitiveFormat,
      });

      onLog?.(`Loaded config for ${lang}`);
    } catch (e) {
      error = String(e);
      onLog?.(`ERROR: Failed to load config - ${e}`);
    } finally {
      isLoading = false;
    }
  }

  async function saveConfig() {
    if (!lang || isSaving) return;

    isSaving = true;
    error = "";

    try {
      await invoke("write_language_config", {
        lang,
        config: {
          extension,
          type_map: typeMap,
          optional_format: optionalFormat,
          list_format: listFormat,
          non_primitive_format: nonPrimitiveFormat || null,
        },
        templatesDir: templatesDir || null,
      });

      // Update original to mark as not modified
      originalConfig = JSON.stringify({
        extension,
        typeMap,
        optionalFormat,
        listFormat,
        nonPrimitiveFormat,
      });

      onLog?.(`Saved config for ${lang}`);
    } catch (e) {
      error = String(e);
      onLog?.(`ERROR: Failed to save config - ${e}`);
    } finally {
      isSaving = false;
    }
  }

  function handleTypeChange(polyType: string, value: string) {
    typeMap = { ...typeMap, [polyType]: value };
  }

  function resetConfig() {
    loadConfig();
  }
</script>

<div class="settings-panel">
  {#if !lang}
    <div class="empty-state">
      <p>Select a language to edit settings</p>
    </div>
  {:else if isLoading}
    <div class="loading">
      <p>Loading configuration...</p>
    </div>
  {:else}
    <!-- Header -->
    <div class="panel-header">
      <div class="header-info">
        <span class="lang-name">{lang}</span>
        <span class="config-file">{lang}.toml</span>
      </div>
      <div class="header-actions">
        {#if isModified}
          <button
            class="btn secondary"
            onclick={resetConfig}
            disabled={isSaving}
          >
            Reset
          </button>
        {/if}
        <button
          class="btn primary"
          onclick={saveConfig}
          disabled={!isModified || isSaving}
        >
          {isSaving ? "Saving..." : "Save"}
        </button>
      </div>
    </div>

    {#if error}
      <div class="error-message">
        {error}
      </div>
    {/if}

    <div class="panel-content">
      <!-- Extension -->
      <div class="section">
        <h4>File Extension</h4>
        <input
          type="text"
          bind:value={extension}
          placeholder=".txt"
        />
      </div>

      <!-- Type Mappings -->
      <div class="section">
        <h4>Type Mappings</h4>
        <p class="hint">Map PolyGen types to {lang} types</p>

        <div class="type-table">
          {#each POLY_TYPES as polyType}
            <div class="type-row">
              <span class="poly-type">
                <code>{polyType.name}</code>
              </span>
              <input
                type="text"
                value={typeMap[polyType.name] || ""}
                placeholder={polyType.name}
                oninput={(e) => handleTypeChange(polyType.name, (e.target as HTMLInputElement).value)}
              />
            </div>
          {/each}
        </div>
      </div>

      <!-- Wrapper Formats -->
      <div class="section">
        <h4>Wrapper Formats</h4>
        <p class="hint">Use <code>{`{{type}}`}</code> as placeholder</p>

        <div class="format-group">
          <label>Optional:</label>
          <input
            type="text"
            bind:value={optionalFormat}
            placeholder={`{{type}}?`}
          />
        </div>

        <div class="format-group">
          <label>List/Array:</label>
          <input
            type="text"
            bind:value={listFormat}
            placeholder={`{{type}}[]`}
          />
        </div>

        <div class="format-group">
          <label>Non-primitive:</label>
          <input
            type="text"
            bind:value={nonPrimitiveFormat}
            placeholder={`{{type}}`}
          />
          <span class="format-hint">For custom types (optional)</span>
        </div>
      </div>
    </div>
  {/if}
</div>

<style>
  .settings-panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    background-color: var(--bg-secondary);
    border-left: 1px solid var(--border);
    font-size: 0.8125rem;
    overflow: hidden;
  }

  .empty-state,
  .loading {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-muted);
    text-align: center;
    padding: 1rem;
  }

  .panel-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .header-info {
    display: flex;
    flex-direction: column;
    gap: 0.125rem;
  }

  .lang-name {
    font-weight: 600;
    color: var(--accent);
    text-transform: uppercase;
    font-size: 0.75rem;
  }

  .config-file {
    font-family: "Consolas", "Monaco", monospace;
    font-size: 0.6875rem;
    color: var(--text-muted);
  }

  .header-actions {
    display: flex;
    gap: 0.375rem;
  }

  .btn {
    padding: 0.25rem 0.625rem;
    font-size: 0.75rem;
    border-radius: 4px;
    cursor: pointer;
  }

  .btn.primary {
    background-color: var(--accent);
    color: white;
    border: none;
  }

  .btn.primary:hover:not(:disabled) {
    background-color: var(--accent-hover);
  }

  .btn.secondary {
    background-color: var(--bg-primary);
    color: var(--text-primary);
    border: 1px solid var(--border);
  }

  .btn.secondary:hover:not(:disabled) {
    background-color: var(--bg-hover);
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .error-message {
    margin: 0.5rem;
    padding: 0.5rem;
    background-color: rgba(244, 67, 54, 0.1);
    border: 1px solid #f44336;
    border-radius: 4px;
    color: #f44336;
    font-size: 0.75rem;
  }

  .panel-content {
    flex: 1;
    overflow-y: auto;
    padding: 0.5rem;
  }

  .section {
    margin-bottom: 1rem;
    padding-bottom: 1rem;
    border-bottom: 1px solid var(--border);
  }

  .section:last-child {
    border-bottom: none;
  }

  .section h4 {
    margin: 0 0 0.25rem 0;
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-primary);
  }

  .section .hint {
    margin: 0 0 0.5rem 0;
    font-size: 0.6875rem;
    color: var(--text-muted);
  }

  .section .hint code {
    background-color: var(--bg-primary);
    padding: 0.0625rem 0.25rem;
    border-radius: 2px;
  }

  .section > input {
    width: 100%;
    padding: 0.375rem 0.5rem;
    font-size: 0.8125rem;
    font-family: "Consolas", "Monaco", monospace;
    background-color: var(--bg-primary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
  }

  .section > input:focus {
    outline: none;
    border-color: var(--accent);
  }

  .type-table {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .type-row {
    display: grid;
    grid-template-columns: 80px 1fr;
    gap: 0.5rem;
    align-items: center;
  }

  .poly-type code {
    font-family: "Consolas", "Monaco", monospace;
    font-size: 0.75rem;
    color: var(--accent);
  }

  .type-row input {
    padding: 0.25rem 0.375rem;
    font-size: 0.75rem;
    font-family: "Consolas", "Monaco", monospace;
    background-color: var(--bg-primary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 3px;
  }

  .type-row input:focus {
    outline: none;
    border-color: var(--accent);
  }

  .format-group {
    display: grid;
    grid-template-columns: 100px 1fr;
    gap: 0.5rem;
    align-items: center;
    margin-bottom: 0.375rem;
  }

  .format-group label {
    font-size: 0.75rem;
    color: var(--text-secondary);
  }

  .format-group input {
    padding: 0.25rem 0.375rem;
    font-size: 0.75rem;
    font-family: "Consolas", "Monaco", monospace;
    background-color: var(--bg-primary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 3px;
  }

  .format-group input:focus {
    outline: none;
    border-color: var(--accent);
  }

  .format-hint {
    grid-column: 2;
    font-size: 0.625rem;
    color: var(--text-muted);
  }
</style>
