<script lang="ts">
  /**
   * NewLanguageWizard - 4-step wizard for creating new language templates
   */
  import { invoke } from "@tauri-apps/api/core";
  import TypeMappingEditor from "./TypeMappingEditor.svelte";

  interface TemplateLanguageInfo {
    id: string;
    name: string;
    path: string;
    file_count: number;
  }

  interface Props {
    show: boolean;
    templatesDir?: string;
    existingLanguages?: TemplateLanguageInfo[];
    onClose?: () => void;
    onCreated?: (langId: string) => void;
    onLog?: (message: string) => void;
  }

  let {
    show = $bindable(false),
    templatesDir,
    existingLanguages = [],
    onClose,
    onCreated,
    onLog,
  }: Props = $props();

  // Wizard state
  let currentStep = $state(1);
  let isCreating = $state(false);
  let error = $state("");

  // Step 1: Basic info
  let langId = $state("");
  let langName = $state("");
  let fileExtension = $state("");

  // Step 2: Template type
  let templateType = $state<"ptpl" | "rhai">("ptpl");

  // Step 3: Starting point
  let startingPoint = $state<"empty" | "copy">("empty");
  let copyFromLang = $state("");

  // Step 4: Type mappings
  let typeMap = $state<Record<string, string>>({});
  let optionalFormat = $state("{{type}}?");
  let listFormat = $state("{{type}}[]");

  // Validation
  let step1Valid = $derived(
    langId.length > 0 &&
    /^[a-z][a-z0-9_]*$/.test(langId) &&
    langName.length > 0 &&
    fileExtension.length > 0
  );

  let step3Valid = $derived(
    startingPoint === "empty" || (startingPoint === "copy" && copyFromLang.length > 0)
  );

  // Pre-filled type maps for common languages
  const PRESET_TYPE_MAPS: Record<string, { typeMap: Record<string, string>; optional: string; list: string }> = {
    python: {
      typeMap: {
        string: "str",
        bool: "bool",
        bytes: "bytes",
        u8: "int", u16: "int", u32: "int", u64: "int",
        i8: "int", i16: "int", i32: "int", i64: "int",
        f32: "float", f64: "float",
      },
      optional: "Optional[{{type}}]",
      list: "List[{{type}}]",
    },
    kotlin: {
      typeMap: {
        string: "String",
        bool: "Boolean",
        bytes: "ByteArray",
        u8: "UByte", u16: "UShort", u32: "UInt", u64: "ULong",
        i8: "Byte", i16: "Short", i32: "Int", i64: "Long",
        f32: "Float", f64: "Double",
      },
      optional: "{{type}}?",
      list: "List<{{type}}>",
    },
    java: {
      typeMap: {
        string: "String",
        bool: "boolean",
        bytes: "byte[]",
        u8: "short", u16: "int", u32: "long", u64: "long",
        i8: "byte", i16: "short", i32: "int", i64: "long",
        f32: "float", f64: "double",
      },
      optional: "Optional<{{type}}>",
      list: "List<{{type}}>",
    },
    swift: {
      typeMap: {
        string: "String",
        bool: "Bool",
        bytes: "Data",
        u8: "UInt8", u16: "UInt16", u32: "UInt32", u64: "UInt64",
        i8: "Int8", i16: "Int16", i32: "Int32", i64: "Int64",
        f32: "Float", f64: "Double",
      },
      optional: "{{type}}?",
      list: "[{{type}}]",
    },
    lua: {
      typeMap: {
        string: "string",
        bool: "boolean",
        bytes: "string",
        u8: "number", u16: "number", u32: "number", u64: "number",
        i8: "number", i16: "number", i32: "number", i64: "number",
        f32: "number", f64: "number",
      },
      optional: "{{type}}|nil",
      list: "{{type}}[]",
    },
  };

  // Apply preset when langId changes
  $effect(() => {
    const lower = langId.toLowerCase();
    if (PRESET_TYPE_MAPS[lower]) {
      const preset = PRESET_TYPE_MAPS[lower];
      typeMap = { ...preset.typeMap };
      optionalFormat = preset.optional;
      listFormat = preset.list;
    }
  });

  // Navigation
  function nextStep() {
    if (currentStep < 4) {
      currentStep++;
    }
  }

  function prevStep() {
    if (currentStep > 1) {
      currentStep--;
    }
  }

  function canProceed(): boolean {
    switch (currentStep) {
      case 1: return step1Valid;
      case 2: return true;
      case 3: return step3Valid;
      case 4: return true;
      default: return false;
    }
  }

  // Reset wizard
  function reset() {
    currentStep = 1;
    langId = "";
    langName = "";
    fileExtension = "";
    templateType = "ptpl";
    startingPoint = "empty";
    copyFromLang = "";
    typeMap = {};
    optionalFormat = "{{type}}?";
    listFormat = "{{type}}[]";
    error = "";
    isCreating = false;
  }

  // Close handler
  function handleClose() {
    reset();
    show = false;
    onClose?.();
  }

  // Create language
  async function createLanguage() {
    if (isCreating) return;

    isCreating = true;
    error = "";

    try {
      await invoke("create_new_language_v2", {
        langId,
        langName,
        extension: fileExtension.startsWith(".") ? fileExtension : `.${fileExtension}`,
        templateType,
        copyFrom: startingPoint === "copy" ? copyFromLang : null,
        typeMap,
        optionalFormat,
        listFormat,
        templatesDir: templatesDir || null,
      });

      onLog?.(`Created new language: ${langName} (${langId})`);
      onCreated?.(langId);
      handleClose();
    } catch (e) {
      error = String(e);
      onLog?.(`ERROR: Failed to create language - ${e}`);
    } finally {
      isCreating = false;
    }
  }

  // Handle overlay click
  function handleOverlayClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      handleClose();
    }
  }
</script>

{#if show}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div class="wizard-overlay" onclick={handleOverlayClick}>
    <div class="wizard-dialog" role="dialog" aria-modal="true">
      <!-- Header -->
      <div class="wizard-header">
        <h2>Create New Language</h2>
        <div class="step-indicator">
          {#each [1, 2, 3, 4] as step}
            <div
              class="step-dot"
              class:active={currentStep === step}
              class:completed={currentStep > step}
            >
              {step}
            </div>
            {#if step < 4}
              <div class="step-line" class:completed={currentStep > step}></div>
            {/if}
          {/each}
        </div>
      </div>

      <!-- Content -->
      <div class="wizard-content">
        {#if currentStep === 1}
          <!-- Step 1: Basic Info -->
          <div class="step-content">
            <h3>Basic Information</h3>
            <p class="step-desc">Enter the basic details for your new language template.</p>

            <div class="form-group">
              <label for="lang-id">Language ID</label>
              <input
                id="lang-id"
                type="text"
                bind:value={langId}
                placeholder="e.g., python, kotlin, swift"
                class:error={langId.length > 0 && !/^[a-z][a-z0-9_]*$/.test(langId)}
              />
              <span class="hint">Lowercase letters, numbers, underscores. Used for folder name.</span>
            </div>

            <div class="form-group">
              <label for="lang-name">Display Name</label>
              <input
                id="lang-name"
                type="text"
                bind:value={langName}
                placeholder="e.g., Python, Kotlin, Swift"
              />
            </div>

            <div class="form-group">
              <label for="file-ext">File Extension</label>
              <input
                id="file-ext"
                type="text"
                bind:value={fileExtension}
                placeholder="e.g., .py, .kt, .swift"
              />
              <span class="hint">Include the dot (.) prefix.</span>
            </div>
          </div>

        {:else if currentStep === 2}
          <!-- Step 2: Template Type -->
          <div class="step-content">
            <h3>Template Type</h3>
            <p class="step-desc">Choose the template system for your language.</p>

            <div class="radio-group">
              <label class="radio-option" class:selected={templateType === "ptpl"}>
                <input
                  type="radio"
                  name="template-type"
                  value="ptpl"
                  bind:group={templateType}
                />
                <div class="radio-content">
                  <span class="radio-title">PolyTemplate (.ptpl)</span>
                  <span class="radio-badge recommended">Recommended</span>
                  <span class="radio-desc">
                    Declarative template language with built-in loops, conditionals, and filters.
                    Better for most use cases.
                  </span>
                </div>
              </label>

              <label class="radio-option" class:selected={templateType === "rhai"}>
                <input
                  type="radio"
                  name="template-type"
                  value="rhai"
                  bind:group={templateType}
                />
                <div class="radio-content">
                  <span class="radio-title">Rhai Script (.rhai)</span>
                  <span class="radio-badge legacy">Legacy</span>
                  <span class="radio-desc">
                    Full scripting language with more flexibility. Use for complex generation logic.
                  </span>
                </div>
              </label>
            </div>
          </div>

        {:else if currentStep === 3}
          <!-- Step 3: Starting Point -->
          <div class="step-content">
            <h3>Starting Point</h3>
            <p class="step-desc">Choose how to initialize your language templates.</p>

            <div class="radio-group">
              <label class="radio-option" class:selected={startingPoint === "empty"}>
                <input
                  type="radio"
                  name="starting-point"
                  value="empty"
                  bind:group={startingPoint}
                />
                <div class="radio-content">
                  <span class="radio-title">Empty Template</span>
                  <span class="radio-desc">
                    Start with a minimal template structure. Best for learning or simple outputs.
                  </span>
                </div>
              </label>

              <label class="radio-option" class:selected={startingPoint === "copy"}>
                <input
                  type="radio"
                  name="starting-point"
                  value="copy"
                  bind:group={startingPoint}
                />
                <div class="radio-content">
                  <span class="radio-title">Copy from Existing</span>
                  <span class="radio-desc">
                    Start with a copy of an existing language template. Useful for similar languages.
                  </span>
                </div>
              </label>
            </div>

            {#if startingPoint === "copy"}
              <div class="form-group" style="margin-top: 1rem;">
                <label for="copy-from">Copy from:</label>
                <select id="copy-from" bind:value={copyFromLang}>
                  <option value="">Select a language...</option>
                  {#each existingLanguages as lang}
                    <option value={lang.id}>{lang.name} ({lang.file_count} files)</option>
                  {/each}
                </select>
              </div>
            {/if}
          </div>

        {:else if currentStep === 4}
          <!-- Step 4: Type Mappings -->
          <div class="step-content">
            <h3>Type Mappings</h3>
            <p class="step-desc">Configure how PolyGen types map to your target language.</p>

            <TypeMappingEditor
              bind:typeMap
              bind:optionalFormat
              bind:listFormat
            />
          </div>
        {/if}

        {#if error}
          <div class="error-message">
            {error}
          </div>
        {/if}
      </div>

      <!-- Footer -->
      <div class="wizard-footer">
        <button class="btn secondary" onclick={handleClose}>
          Cancel
        </button>

        <div class="footer-right">
          {#if currentStep > 1}
            <button class="btn secondary" onclick={prevStep}>
              Back
            </button>
          {/if}

          {#if currentStep < 4}
            <button
              class="btn primary"
              onclick={nextStep}
              disabled={!canProceed()}
            >
              Next
            </button>
          {:else}
            <button
              class="btn primary"
              onclick={createLanguage}
              disabled={isCreating}
            >
              {isCreating ? "Creating..." : "Create Language"}
            </button>
          {/if}
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .wizard-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: rgba(0, 0, 0, 0.6);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .wizard-dialog {
    background-color: var(--bg-secondary);
    border-radius: 12px;
    width: 90%;
    max-width: 640px;
    max-height: 90vh;
    display: flex;
    flex-direction: column;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
  }

  /* Header */
  .wizard-header {
    padding: 1.25rem 1.5rem;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .wizard-header h2 {
    margin: 0 0 1rem 0;
    font-size: 1.125rem;
    color: var(--text-primary);
  }

  .step-indicator {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0;
  }

  .step-dot {
    width: 28px;
    height: 28px;
    border-radius: 50%;
    background-color: var(--bg-primary);
    border: 2px solid var(--border);
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-muted);
    transition: all 0.2s;
  }

  .step-dot.active {
    background-color: var(--accent);
    border-color: var(--accent);
    color: white;
  }

  .step-dot.completed {
    background-color: var(--success, #4caf50);
    border-color: var(--success, #4caf50);
    color: white;
  }

  .step-line {
    width: 40px;
    height: 2px;
    background-color: var(--border);
    transition: background-color 0.2s;
  }

  .step-line.completed {
    background-color: var(--success, #4caf50);
  }

  /* Content */
  .wizard-content {
    flex: 1;
    overflow-y: auto;
    padding: 1.5rem;
  }

  .step-content h3 {
    margin: 0 0 0.25rem 0;
    font-size: 1rem;
    color: var(--text-primary);
  }

  .step-desc {
    margin: 0 0 1.25rem 0;
    font-size: 0.8125rem;
    color: var(--text-secondary);
  }

  /* Form */
  .form-group {
    margin-bottom: 1rem;
  }

  .form-group label {
    display: block;
    margin-bottom: 0.375rem;
    font-size: 0.8125rem;
    font-weight: 500;
    color: var(--text-secondary);
  }

  .form-group input,
  .form-group select {
    width: 100%;
    padding: 0.625rem 0.75rem;
    font-size: 0.875rem;
    background-color: var(--bg-primary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 6px;
  }

  .form-group input:focus,
  .form-group select:focus {
    outline: none;
    border-color: var(--accent);
  }

  .form-group input.error {
    border-color: var(--error, #f44336);
  }

  .form-group .hint {
    display: block;
    margin-top: 0.25rem;
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  /* Radio group */
  .radio-group {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .radio-option {
    display: flex;
    align-items: flex-start;
    gap: 0.75rem;
    padding: 1rem;
    background-color: var(--bg-primary);
    border: 2px solid var(--border);
    border-radius: 8px;
    cursor: pointer;
    transition: all 0.15s;
  }

  .radio-option:hover {
    border-color: var(--text-muted);
  }

  .radio-option.selected {
    border-color: var(--accent);
    background-color: rgba(var(--accent-rgb, 66, 133, 244), 0.05);
  }

  .radio-option input[type="radio"] {
    margin-top: 0.125rem;
    accent-color: var(--accent);
  }

  .radio-content {
    flex: 1;
  }

  .radio-title {
    display: block;
    font-weight: 600;
    font-size: 0.875rem;
    color: var(--text-primary);
    margin-bottom: 0.25rem;
  }

  .radio-badge {
    display: inline-block;
    font-size: 0.625rem;
    font-weight: 600;
    text-transform: uppercase;
    padding: 0.125rem 0.375rem;
    border-radius: 3px;
    margin-left: 0.5rem;
    vertical-align: middle;
  }

  .radio-badge.recommended {
    background-color: var(--success, #4caf50);
    color: white;
  }

  .radio-badge.legacy {
    background-color: var(--warning, #ff9800);
    color: white;
  }

  .radio-desc {
    display: block;
    font-size: 0.75rem;
    color: var(--text-secondary);
    line-height: 1.4;
  }

  /* Error */
  .error-message {
    margin-top: 1rem;
    padding: 0.75rem 1rem;
    background-color: rgba(var(--error-rgb, 244, 67, 54), 0.1);
    border: 1px solid var(--error, #f44336);
    border-radius: 6px;
    color: var(--error, #f44336);
    font-size: 0.8125rem;
  }

  /* Footer */
  .wizard-footer {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 1rem 1.5rem;
    border-top: 1px solid var(--border);
    flex-shrink: 0;
  }

  .footer-right {
    display: flex;
    gap: 0.5rem;
  }

  .btn {
    padding: 0.5rem 1.25rem;
    font-size: 0.875rem;
    font-weight: 500;
    border-radius: 6px;
    cursor: pointer;
    transition: all 0.15s;
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

  .btn.secondary:hover {
    background-color: var(--bg-hover);
  }

  .btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
</style>
