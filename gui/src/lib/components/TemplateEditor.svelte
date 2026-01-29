<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import TemplateFileTree from "./TemplateFileTree.svelte";
  import RhaiEditor from "./RhaiEditor.svelte";

  interface TemplateFileInfo {
    name: string;
    path: string;
    relative_path: string;
    is_directory: boolean;
    children: TemplateFileInfo[];
  }

  interface PreviewFile {
    name: string;
    content: string;
    template?: string;
  }

  /** A block of code within a preview file, annotated with its source template */
  interface SourceBlock {
    /** The template that generated this block (undefined = unmarked code) */
    template?: string;
    /** The code content (with markers stripped) */
    content: string;
  }

  interface Props {
    templatesDir?: string;
    schemaPath?: string;
    onLog?: (message: string) => void;
  }

  let { templatesDir, schemaPath, onLog }: Props = $props();

  // State
  let selectedLang = $state("");
  let selectedFile = $state("");
  let editorContent = $state("");
  let originalContent = $state("");
  let isModified = $state(false);
  let isSaving = $state(false);
  let isGenerating = $state(false);
  let showNewLangDialog = $state(false);
  let newLangId = $state("");
  let newLangName = $state("");

  // Tab state: "editor" or "preview"
  let activeTab = $state<"editor" | "preview">("editor");

  // Preview state
  let previewFiles = $state<PreviewFile[]>([]);
  let selectedPreviewFile = $state<number>(0);

  // Source block hover state
  let hoveredBlockIndex = $state<number | null>(null);

  // Context menu state
  let contextMenu = $state<{ x: number; y: number; template: string } | null>(null);

  // Track current file info
  let currentFile = $state<TemplateFileInfo | null>(null);

  function log(message: string) {
    onLog?.(`[Template] ${message}`);
  }

  /** Parse source markers in file content into SourceBlocks.
   *  Supports nested markers from engine-level auto-marking.
   *  Each text segment is attributed to the innermost (deepest) template.
   *  Markers: /\*@source:template_name\*\/ ... /\*@\/source\*\/
   */
  function parseSourceBlocks(content: string): SourceBlock[] {
    // Collect all open/close marker positions
    interface Marker {
      type: "open" | "close";
      index: number;       // start of marker in content
      end: number;         // end of marker (after optional newline)
      template?: string;   // only for open markers
    }

    const markers: Marker[] = [];
    const openRe = /\/\*@source:(.+?)\*\/\n?/g;
    const closeRe = /\/\*@\/source\*\/\n?/g;

    let m: RegExpExecArray | null;
    while ((m = openRe.exec(content)) !== null) {
      markers.push({ type: "open", index: m.index, end: m.index + m[0].length, template: m[1] });
    }
    while ((m = closeRe.exec(content)) !== null) {
      markers.push({ type: "close", index: m.index, end: m.index + m[0].length });
    }

    // Sort by position
    markers.sort((a, b) => a.index - b.index);

    if (markers.length === 0) {
      // No markers at all
      return content.length > 0 ? [{ content }] : [];
    }

    const blocks: SourceBlock[] = [];
    const stack: string[] = []; // stack of template names (innermost on top)
    let cursor = 0;

    for (const marker of markers) {
      // Text between cursor and this marker
      if (marker.index > cursor) {
        const text = content.substring(cursor, marker.index);
        if (text.trim().length > 0) {
          const template = stack.length > 0 ? stack[stack.length - 1] : undefined;
          blocks.push({ template, content: text });
        }
      }

      if (marker.type === "open") {
        stack.push(marker.template!);
      } else {
        stack.pop();
      }

      cursor = marker.end;
    }

    // Remaining text after last marker
    if (cursor < content.length) {
      const remaining = content.substring(cursor);
      if (remaining.trim().length > 0) {
        const template = stack.length > 0 ? stack[stack.length - 1] : undefined;
        blocks.push({ template, content: remaining });
      }
    }

    return blocks;
  }

  /** Get source blocks for the current preview file (derived) */
  let currentSourceBlocks = $derived.by(() => {
    if (previewFiles.length === 0 || selectedPreviewFile >= previewFiles.length) return [];
    return parseSourceBlocks(previewFiles[selectedPreviewFile].content);
  });

  /** Get clean content (markers stripped) for display */
  function getCleanContent(file: PreviewFile): string {
    return file.content
      .replace(/\/\*@source:.+?\*\/\n?/g, "")
      .replace(/\/\*@\/source\*\/\n?/g, "");
  }

  /** Parse preview output into individual files.
   *  Format: `=== filename [template.rhai] ===` or `=== filename ===` (for static files)
   */
  function parsePreviewOutput(raw: string): PreviewFile[] {
    const files: PreviewFile[] = [];
    const parts = raw.split(/^=== (.+?) ===$/m);
    // parts: [preamble, header1, content1, header2, content2, ...]
    for (let i = 1; i < parts.length; i += 2) {
      const header = parts[i].trim();
      const content = (parts[i + 1] || "").trim();

      // Parse header: "filename [template.rhai]" or just "filename"
      const match = header.match(/^(.+?)\s+\[(.+?)\]$/);
      const name = match ? match[1] : header;
      const template = match ? match[2] : undefined;

      // Skip debug files
      if (name === "ast_debug.txt" || name === "ir_debug.txt" || name.startsWith("debug\\") || name.startsWith("debug/")) continue;
      if (content.length > 0) {
        files.push({ name, content, template });
      }
    }
    return files;
  }

  async function handleSelectLang(lang: string) {
    if (isModified) {
      const shouldSave = confirm("Save changes to current file?");
      if (shouldSave) await saveFile();
    }
    selectedLang = lang;
    selectedFile = "";
    editorContent = "";
    originalContent = "";
    previewFiles = [];
    selectedPreviewFile = 0;
    isModified = false;
    currentFile = null;
    activeTab = "editor";
    log(`Selected language: ${lang}`);
  }

  async function handleSelectFile(file: TemplateFileInfo) {
    if (file.is_directory) return;

    if (isModified) {
      const shouldSave = confirm("Save changes to current file?");
      if (shouldSave) await saveFile();
    }

    try {
      const content = await invoke<string>("read_template_file", {
        lang: selectedLang,
        relativePath: file.relative_path,
        templatesDir: templatesDir || null,
      });

      selectedFile = file.relative_path;
      editorContent = content;
      originalContent = content;
      isModified = false;
      currentFile = file;
      activeTab = "editor";
      log(`Opened: ${file.name}`);
    } catch (e) {
      log(`ERROR: Failed to open file - ${e}`);
    }
  }

  function handleEditorChange(value: string) {
    editorContent = value;
    isModified = value !== originalContent;
  }

  async function saveFile() {
    if (!currentFile || !selectedLang) return;

    isSaving = true;
    try {
      await invoke("write_template_file", {
        lang: selectedLang,
        relativePath: currentFile.relative_path,
        content: editorContent,
        templatesDir: templatesDir || null,
      });

      originalContent = editorContent;
      isModified = false;
      log(`Saved: ${currentFile.name}`);
    } catch (e) {
      log(`ERROR: Failed to save file - ${e}`);
    } finally {
      isSaving = false;
    }
  }

  async function generatePreview() {
    if (!schemaPath || !selectedLang) {
      log("ERROR: Please open a schema file and select a language");
      return;
    }

    if (isModified) await saveFile();

    isGenerating = true;
    try {
      const result = await invoke<string>("preview_template", {
        schemaPath,
        lang: selectedLang,
        templatesDir: templatesDir || null,
      });

      previewFiles = parsePreviewOutput(result);
      selectedPreviewFile = 0;
      activeTab = "preview";
      log(`Preview generated: ${previewFiles.length} files`);
    } catch (e) {
      previewFiles = [{ name: "Error", content: `Error generating preview:\n${e}` }];
      selectedPreviewFile = 0;
      activeTab = "preview";
      log(`ERROR: Preview failed - ${e}`);
    } finally {
      isGenerating = false;
    }
  }

  function openNewLangDialog() {
    showNewLangDialog = true;
    newLangId = "";
    newLangName = "";
  }

  async function createNewLanguage() {
    if (!newLangId || !newLangName) return;

    if (!/^[a-z][a-z0-9_]*$/.test(newLangId)) {
      log("ERROR: Language ID must be lowercase letters, numbers, and underscores, starting with a letter");
      return;
    }

    try {
      await invoke("create_new_language", {
        langId: newLangId,
        langName: newLangName,
        templatesDir: templatesDir || null,
      });

      log(`Created new language: ${newLangName} (${newLangId})`);
      showNewLangDialog = false;
      selectedLang = newLangId;
    } catch (e) {
      log(`ERROR: Failed to create language - ${e}`);
    }
  }

  function handleKeydown(event: KeyboardEvent) {
    if (event.ctrlKey || event.metaKey) {
      if (event.key === "s") {
        event.preventDefault();
        saveFile();
      }
    }
    // Close context menu on Escape
    if (event.key === "Escape") {
      contextMenu = null;
    }
  }

  /** Navigate to a template file in the editor */
  function navigateToTemplate(templateName: string) {
    contextMenu = null;
    // Find the template file in the file tree
    // Template names are like "csharp_logic_struct.rhai"
    // Try to find the file and open it
    if (selectedLang) {
      // Search for the file - it could be in subdirectories
      findAndOpenTemplate(templateName);
    }
  }

  async function findAndOpenTemplate(templateName: string) {
    try {
      // Try to read the file by searching common locations
      const locations = [
        templateName,
        `class/${templateName}`,
        `enum/${templateName}`,
        `rhai_utils/${templateName}`,
      ];

      for (const loc of locations) {
        try {
          const content = await invoke<string>("read_template_file", {
            lang: selectedLang,
            relativePath: loc,
            templatesDir: templatesDir || null,
          });

          selectedFile = loc;
          editorContent = content;
          originalContent = content;
          isModified = false;
          currentFile = {
            name: templateName,
            path: loc,
            relative_path: loc,
            is_directory: false,
            children: [],
          };
          activeTab = "editor";
          log(`Navigated to: ${templateName}`);
          return;
        } catch {
          // Try next location
        }
      }

      log(`Template not found: ${templateName}`);
    } catch (e) {
      log(`ERROR: Failed to navigate to template - ${e}`);
    }
  }

  /** Handle right-click on a source block */
  function handleBlockContextMenu(event: MouseEvent, template: string) {
    event.preventDefault();
    contextMenu = { x: event.clientX, y: event.clientY, template };
  }

  /** Close context menu when clicking elsewhere */
  function handleGlobalClick() {
    contextMenu = null;
  }

  /** Get file extension for syntax hint */
  function getFileExtension(name: string): string {
    const dot = name.lastIndexOf(".");
    return dot >= 0 ? name.substring(dot + 1) : "";
  }
</script>

<svelte:window onkeydown={handleKeydown} onclick={handleGlobalClick} />

<div class="template-editor">
  <!-- Left Panel: File Tree -->
  <div class="file-tree-panel">
    <TemplateFileTree
      {templatesDir}
      {selectedLang}
      {selectedFile}
      onSelectLang={handleSelectLang}
      onSelectFile={handleSelectFile}
      onCreateLanguage={openNewLangDialog}
    />
  </div>

  <!-- Main Panel: Tabbed Editor / Preview -->
  <div class="main-panel">
    <!-- Tab bar -->
    <div class="tab-bar">
      <div class="tab-buttons">
        <button
          class="tab-btn"
          class:active={activeTab === "editor"}
          onclick={() => (activeTab = "editor")}
        >
          Editor
          {#if currentFile}
            <span class="tab-file-name">
              - {currentFile.name}{#if isModified}<span class="modified">*</span>{/if}
            </span>
          {/if}
        </button>
        <button
          class="tab-btn"
          class:active={activeTab === "preview"}
          onclick={() => (activeTab = "preview")}
        >
          Preview
          {#if previewFiles.length > 0}
            <span class="tab-badge">{previewFiles.length}</span>
          {/if}
        </button>
      </div>
      <div class="tab-actions">
        {#if activeTab === "editor"}
          <button
            class="action-btn secondary"
            onclick={saveFile}
            disabled={!isModified || isSaving}
          >
            {isSaving ? "Saving..." : "Save"}
          </button>
        {/if}
        <button
          class="action-btn primary"
          onclick={generatePreview}
          disabled={!schemaPath || !selectedLang || isGenerating}
        >
          {isGenerating ? "Generating..." : "Generate Preview"}
        </button>
      </div>
    </div>

    <!-- Tab content -->
    <div class="tab-content">
      {#if activeTab === "editor"}
        <div class="editor-content">
          {#if currentFile}
            {#key selectedFile}
              <RhaiEditor
                value={editorContent}
                onChange={handleEditorChange}
                filePath={currentFile.path}
              />
            {/key}
          {:else}
            <div class="empty-state">
              <p>Select a template file to edit</p>
              <p class="hint">Files ending in .rhai are template scripts</p>
            </div>
          {/if}
        </div>
      {:else}
        <!-- Preview tab -->
        <div class="preview-container">
          {#if previewFiles.length > 0}
            <!-- File list sidebar -->
            <div class="preview-file-list">
              {#each previewFiles as file, i}
                <button
                  class="preview-file-item"
                  class:active={selectedPreviewFile === i}
                  onclick={() => { selectedPreviewFile = i; hoveredBlockIndex = null; }}
                  title={file.template ? `${file.name}\nTemplate: ${file.template}` : file.name}
                >
                  <span class="preview-file-ext">{getFileExtension(file.name)}</span>
                  <span class="preview-file-name">{file.name}</span>
                  {#if file.template}
                    <span class="preview-file-template">{file.template}</span>
                  {/if}
                </button>
              {/each}
            </div>
            <!-- File content with source blocks -->
            <div class="preview-content">
              <div class="preview-header">
                <span class="preview-path">{previewFiles[selectedPreviewFile].name}</span>
                {#if previewFiles[selectedPreviewFile].template}
                  <span class="preview-template-badge">{previewFiles[selectedPreviewFile].template}</span>
                {/if}
              </div>
              <div class="preview-code-container">
                {#if currentSourceBlocks.length > 0 && currentSourceBlocks.some(b => b.template)}
                  <!-- Source-block aware rendering -->
                  {#each currentSourceBlocks as block, i}
                    {#if block.template}
                      <!-- svelte-ignore a11y_no_static_element_interactions -->
                      <pre
                        class="preview-block"
                        class:hovered={hoveredBlockIndex === i}
                        onmouseenter={() => (hoveredBlockIndex = i)}
                        onmouseleave={() => (hoveredBlockIndex = null)}
                        oncontextmenu={(e) => handleBlockContextMenu(e, block.template!)}
                      ><span class="block-source-tag">{block.template}</span>{block.content}</pre>
                    {:else}
                      <pre class="preview-block unmarked">{block.content}</pre>
                    {/if}
                  {/each}
                {:else}
                  <!-- No source markers - plain rendering -->
                  <pre class="preview-block unmarked">{getCleanContent(previewFiles[selectedPreviewFile])}</pre>
                {/if}
              </div>
            </div>
          {:else}
            <div class="empty-state">
              <p>Click "Generate Preview" to see output</p>
              {#if !schemaPath}
                <p class="hint">Open a schema file first</p>
              {/if}
            </div>
          {/if}
        </div>
      {/if}
    </div>
  </div>
</div>

<!-- Context Menu -->
{#if contextMenu}
  <div
    class="context-menu"
    style="left: {contextMenu.x}px; top: {contextMenu.y}px"
    role="menu"
  >
    <button
      class="context-menu-item"
      onclick={() => navigateToTemplate(contextMenu!.template)}
      role="menuitem"
    >
      Open {contextMenu.template}
    </button>
  </div>
{/if}

<!-- New Language Dialog -->
{#if showNewLangDialog}
  <div class="dialog-overlay" onclick={() => (showNewLangDialog = false)} role="presentation">
    <div
      class="dialog"
      onclick={(e) => e.stopPropagation()}
      role="dialog"
      aria-modal="true"
    >
      <h3>Create New Language</h3>
      <div class="form-group">
        <label for="lang-id">Language ID</label>
        <input
          id="lang-id"
          type="text"
          bind:value={newLangId}
          placeholder="e.g., python, kotlin"
        />
        <span class="hint">Lowercase letters, numbers, underscores</span>
      </div>
      <div class="form-group">
        <label for="lang-name">Display Name</label>
        <input
          id="lang-name"
          type="text"
          bind:value={newLangName}
          placeholder="e.g., Python, Kotlin"
        />
      </div>
      <div class="dialog-actions">
        <button class="action-btn secondary" onclick={() => (showNewLangDialog = false)}>
          Cancel
        </button>
        <button
          class="action-btn primary"
          onclick={createNewLanguage}
          disabled={!newLangId || !newLangName}
        >
          Create
        </button>
      </div>
    </div>
  </div>
{/if}

<style>
  .template-editor {
    display: grid;
    grid-template-columns: 240px 1fr;
    gap: 0.5rem;
    height: 100%;
    min-height: 0;
  }

  .file-tree-panel {
    min-height: 0;
    overflow: hidden;
  }

  .main-panel {
    display: flex;
    flex-direction: column;
    min-height: 0;
    background-color: var(--bg-secondary);
    border-radius: 8px;
    overflow: hidden;
  }

  /* Tab bar */
  .tab-bar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0 0.5rem;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    height: 40px;
  }

  .tab-buttons {
    display: flex;
    gap: 0;
    height: 100%;
  }

  .tab-btn {
    padding: 0 1rem;
    height: 100%;
    font-size: 0.8125rem;
    font-weight: 500;
    color: var(--text-secondary);
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    cursor: pointer;
    display: flex;
    align-items: center;
    gap: 0.375rem;
    white-space: nowrap;
  }

  .tab-btn:hover {
    color: var(--text-primary);
    background-color: var(--bg-hover);
  }

  .tab-btn.active {
    color: var(--accent);
    border-bottom-color: var(--accent);
  }

  .tab-file-name {
    font-weight: 400;
    color: var(--text-muted);
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .tab-btn.active .tab-file-name {
    color: var(--text-secondary);
  }

  .modified {
    color: var(--warning);
    font-weight: bold;
  }

  .tab-badge {
    font-size: 0.6875rem;
    background-color: var(--accent);
    color: white;
    padding: 0.0625rem 0.375rem;
    border-radius: 10px;
    min-width: 1.125rem;
    text-align: center;
  }

  .tab-actions {
    display: flex;
    gap: 0.375rem;
    align-items: center;
  }

  /* Tab content */
  .tab-content {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .editor-content {
    height: 100%;
    overflow: hidden;
  }

  /* Preview layout */
  .preview-container {
    display: grid;
    grid-template-columns: 240px 1fr;
    height: 100%;
    min-height: 0;
  }

  .preview-file-list {
    border-right: 1px solid var(--border);
    overflow-y: auto;
    background-color: var(--bg-primary);
  }

  .preview-file-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
    padding: 0.375rem 0.75rem;
    font-size: 0.75rem;
    color: var(--text-secondary);
    background: none;
    border: none;
    cursor: pointer;
    text-align: left;
  }

  .preview-file-item:hover {
    background-color: var(--bg-hover);
    color: var(--text-primary);
  }

  .preview-file-item.active {
    background-color: var(--bg-selected, var(--bg-hover));
    color: var(--accent);
  }

  .preview-file-ext {
    font-size: 0.625rem;
    font-weight: 600;
    text-transform: uppercase;
    color: var(--text-muted);
    background-color: var(--bg-secondary);
    padding: 0.0625rem 0.25rem;
    border-radius: 3px;
    flex-shrink: 0;
    min-width: 2rem;
    text-align: center;
  }

  .preview-file-item.active .preview-file-ext {
    background-color: var(--accent);
    color: white;
  }

  .preview-file-name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    min-width: 0;
  }

  .preview-file-template {
    font-size: 0.5625rem;
    color: var(--text-muted);
    background-color: var(--bg-secondary);
    padding: 0.0625rem 0.25rem;
    border-radius: 2px;
    flex-shrink: 0;
    white-space: nowrap;
    opacity: 0.7;
  }

  .preview-file-item.active .preview-file-template {
    color: var(--accent);
    opacity: 1;
  }

  .preview-template-badge {
    font-size: 0.6875rem;
    color: var(--accent);
    background-color: var(--bg-primary);
    padding: 0.125rem 0.375rem;
    border-radius: 3px;
    font-family: "Consolas", "Monaco", monospace;
    flex-shrink: 0;
  }

  .preview-content {
    display: flex;
    flex-direction: column;
    min-height: 0;
    overflow: hidden;
  }

  .preview-header {
    padding: 0.375rem 0.75rem;
    display: flex;
    align-items: center;
    gap: 0.5rem;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }

  .preview-path {
    font-size: 0.75rem;
    color: var(--text-muted);
    font-family: "Consolas", "Monaco", monospace;
  }

  /* Source-block aware code display */
  .preview-code-container {
    overflow: auto;
    flex: 1;
    min-height: 0;
    background-color: var(--bg-primary);
  }

  .preview-block {
    margin: 0;
    padding: 0 0.75rem;
    font-size: 0.8125rem;
    font-family: "Consolas", "Monaco", monospace;
    white-space: pre;
    border-left: 3px solid transparent;
    position: relative;
    transition: background-color 0.15s, border-color 0.15s;
  }

  .preview-block:not(.unmarked) {
    cursor: context-menu;
  }

  .preview-block:not(.unmarked):hover,
  .preview-block.hovered {
    background-color: rgba(var(--accent-rgb, 66, 133, 244), 0.08);
    border-left-color: var(--accent);
  }

  .block-source-tag {
    display: none;
    position: absolute;
    top: 0;
    right: 0.5rem;
    font-size: 0.625rem;
    color: var(--accent);
    background-color: var(--bg-secondary);
    padding: 0.0625rem 0.375rem;
    border-radius: 0 0 3px 3px;
    font-family: "Consolas", "Monaco", monospace;
    opacity: 0.9;
    pointer-events: none;
  }

  .preview-block:not(.unmarked):hover .block-source-tag,
  .preview-block.hovered .block-source-tag {
    display: block;
  }

  /* Context menu */
  .context-menu {
    position: fixed;
    z-index: 2000;
    background-color: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    padding: 0.25rem;
    min-width: 180px;
  }

  .context-menu-item {
    display: block;
    width: 100%;
    padding: 0.375rem 0.75rem;
    font-size: 0.8125rem;
    color: var(--text-primary);
    background: none;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    text-align: left;
    white-space: nowrap;
  }

  .context-menu-item:hover {
    background-color: var(--accent);
    color: white;
  }

  /* Shared */
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-secondary);
    text-align: center;
    padding: 2rem;
  }

  .empty-state p {
    margin: 0.25rem 0;
  }

  .hint {
    font-size: 0.75rem;
    color: var(--text-muted);
  }

  .action-btn {
    padding: 0.25rem 0.625rem;
    font-size: 0.75rem;
    border-radius: 4px;
    cursor: pointer;
  }

  .action-btn.primary {
    background-color: var(--accent);
    color: white;
    border: none;
  }

  .action-btn.primary:hover:not(:disabled) {
    background-color: var(--accent-hover);
  }

  .action-btn.secondary {
    background-color: var(--bg-primary);
    color: var(--text-primary);
    border: 1px solid var(--border);
  }

  .action-btn.secondary:hover:not(:disabled) {
    background-color: var(--bg-hover);
  }

  .action-btn:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  /* Dialog styles */
  .dialog-overlay {
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    bottom: 0;
    background-color: rgba(0, 0, 0, 0.5);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .dialog {
    background-color: var(--bg-secondary);
    border-radius: 8px;
    padding: 1.5rem;
    min-width: 320px;
    max-width: 90%;
  }

  .dialog h3 {
    margin: 0 0 1rem 0;
    font-size: 1rem;
    color: var(--text-primary);
  }

  .form-group {
    margin-bottom: 1rem;
  }

  .form-group label {
    display: block;
    margin-bottom: 0.25rem;
    font-size: 0.875rem;
    color: var(--text-secondary);
  }

  .form-group input {
    width: 100%;
    padding: 0.5rem;
    font-size: 0.875rem;
    background-color: var(--bg-primary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
  }

  .form-group .hint {
    display: block;
    margin-top: 0.25rem;
  }

  .dialog-actions {
    display: flex;
    justify-content: flex-end;
    gap: 0.5rem;
    margin-top: 1.5rem;
  }

  @media (max-width: 1024px) {
    .template-editor {
      grid-template-columns: 1fr;
      grid-template-rows: auto 1fr;
    }

    .file-tree-panel {
      max-height: 200px;
    }

    .preview-container {
      grid-template-columns: 180px 1fr;
    }
  }
</style>
