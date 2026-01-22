<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import PathSelector from "./lib/components/PathSelector.svelte";
  import LanguageSelector from "./lib/components/LanguageSelector.svelte";
  import LogPanel from "./lib/components/LogPanel.svelte";
  import Editor from "./lib/components/Editor.svelte";

  let schemaPath = $state("");
  let outputDir = $state("");
  let templatesDir = $state("");
  let selectedLanguages = $state<string[]>(["csharp"]);
  let logs = $state<string[]>([]);
  let isGenerating = $state(false);
  let editorContent = $state("");
  let isModified = $state(false);

  async function selectSchemaFile() {
    const selected = await open({
      multiple: false,
      filters: [{ name: "Poly Schema", extensions: ["poly"] }],
    });
    if (selected) {
      schemaPath = selected as string;
      await loadFile(schemaPath);
    }
  }

  async function loadFile(path: string) {
    try {
      const content = await invoke<string>("read_file", { path });
      editorContent = content;
      isModified = false;
      addLog(`Loaded: ${path}`);
    } catch (error) {
      addLog(`ERROR: Failed to load file - ${error}`);
    }
  }

  async function saveFile() {
    if (!schemaPath) {
      await saveFileAs();
      return;
    }
    try {
      await invoke("write_file", { path: schemaPath, content: editorContent });
      isModified = false;
      addLog(`Saved: ${schemaPath}`);
    } catch (error) {
      addLog(`ERROR: Failed to save file - ${error}`);
    }
  }

  async function saveFileAs() {
    const selected = await save({
      filters: [{ name: "Poly Schema", extensions: ["poly"] }],
    });
    if (selected) {
      schemaPath = selected as string;
      await saveFile();
    }
  }

  async function newFile() {
    if (isModified) {
      // TODO: Confirm dialog
    }
    schemaPath = "";
    editorContent = `// New PolyGen Schema
namespace example {
    table Sample {
        id: u32 primary_key;
        name: string max_length(100);
    }
}
`;
    isModified = false;
  }

  async function selectOutputDir() {
    const selected = await open({
      directory: true,
      multiple: false,
    });
    if (selected) {
      outputDir = selected as string;
    }
  }

  async function selectTemplatesDir() {
    const selected = await open({
      directory: true,
      multiple: false,
    });
    if (selected) {
      templatesDir = selected as string;
    }
  }

  function addLog(message: string) {
    logs = [...logs, `[${new Date().toLocaleTimeString()}] ${message}`];
  }

  function onEditorChange(value: string) {
    isModified = true;
  }

  async function generate() {
    // If editing in editor and no file saved, save to temp first
    if (!schemaPath && editorContent) {
      addLog("ERROR: 먼저 스키마 파일을 저장해주세요.");
      return;
    }
    if (!schemaPath) {
      addLog("ERROR: 스키마 파일을 선택해주세요.");
      return;
    }
    if (!outputDir) {
      addLog("ERROR: 출력 디렉토리를 선택해주세요.");
      return;
    }
    if (selectedLanguages.length === 0) {
      addLog("ERROR: 최소 하나의 언어를 선택해주세요.");
      return;
    }

    // Save before generate if modified
    if (isModified) {
      await saveFile();
    }

    isGenerating = true;

    for (const lang of selectedLanguages) {
      addLog(`Generating ${lang}...`);
      try {
        const result = await invoke<string>("run_generate", {
          schemaPath,
          lang,
          outputDir,
          templatesDir: templatesDir || null,
        });
        addLog(`SUCCESS: ${lang} - ${result}`);
      } catch (error) {
        addLog(`ERROR: ${lang} - ${error}`);
      }
    }

    isGenerating = false;
    addLog("Generation complete.");
  }

  function clearLogs() {
    logs = [];
  }
</script>

<main>
  <header>
    <div class="header-left">
      <h1>PolyGen</h1>
    </div>
    <div class="header-center">
      <button class="toolbar-btn" onclick={newFile} title="New File">New</button>
      <button class="toolbar-btn" onclick={selectSchemaFile} title="Open File">Open</button>
      <button class="toolbar-btn" onclick={saveFile} title="Save File">Save</button>
      <button class="toolbar-btn" onclick={saveFileAs} title="Save As">Save As</button>
    </div>
    <div class="header-right">
      {#if schemaPath}
        <span class="filename">{schemaPath.split(/[/\\]/).pop()}{isModified ? " *" : ""}</span>
      {:else}
        <span class="filename">Untitled{isModified ? " *" : ""}</span>
      {/if}
    </div>
  </header>

  <div class="content">
    <section class="settings">
      <h2>Settings</h2>

      <PathSelector
        label="Output Directory"
        value={outputDir}
        placeholder="Select output directory..."
        onSelect={selectOutputDir}
      />

      <PathSelector
        label="Templates (optional)"
        value={templatesDir}
        placeholder="Default templates"
        onSelect={selectTemplatesDir}
      />

      <LanguageSelector bind:selected={selectedLanguages} />

      <div class="actions">
        <button class="primary" onclick={generate} disabled={isGenerating}>
          {isGenerating ? "Generating..." : "Generate"}
        </button>
      </div>
    </section>

    <section class="editor-section">
      <div class="editor-header">
        <h2>Schema Editor</h2>
      </div>
      <div class="editor-wrapper">
        <Editor bind:value={editorContent} onChange={onEditorChange} />
      </div>
    </section>

    <section class="output">
      <div class="output-header">
        <h2>Output</h2>
        <button class="secondary" onclick={clearLogs}>Clear</button>
      </div>
      <LogPanel {logs} />
    </section>
  </div>
</main>

<style>
  main {
    display: flex;
    flex-direction: column;
    height: 100vh;
    padding: 0.5rem;
    gap: 0.5rem;
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0.5rem 1rem;
    background-color: var(--bg-secondary);
    border-radius: 8px;
  }

  .header-left h1 {
    font-size: 1.25rem;
    color: var(--accent);
    margin: 0;
  }

  .header-center {
    display: flex;
    gap: 0.5rem;
  }

  .toolbar-btn {
    padding: 0.375rem 0.75rem;
    font-size: 0.875rem;
    background-color: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
    cursor: pointer;
  }

  .toolbar-btn:hover {
    background-color: var(--bg-hover);
  }

  .header-right .filename {
    font-size: 0.875rem;
    color: var(--text-secondary);
  }

  .content {
    display: grid;
    grid-template-columns: 280px 1fr 300px;
    gap: 0.5rem;
    flex: 1;
    min-height: 0;
  }

  section {
    background-color: var(--bg-secondary);
    border-radius: 8px;
    padding: 0.75rem;
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    min-height: 0;
  }

  section h2 {
    font-size: 0.875rem;
    color: var(--text-primary);
    border-bottom: 1px solid var(--border);
    padding-bottom: 0.5rem;
    margin: 0;
  }

  .settings {
    overflow-y: auto;
  }

  .actions {
    margin-top: auto;
    padding-top: 0.75rem;
  }

  .actions button {
    width: 100%;
    padding: 0.625rem;
    font-size: 0.875rem;
  }

  .editor-section {
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .editor-header {
    flex-shrink: 0;
  }

  .editor-wrapper {
    flex: 1;
    min-height: 0;
    border-radius: 4px;
    overflow: hidden;
  }

  .output {
    display: flex;
    flex-direction: column;
    min-height: 0;
  }

  .output-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--border);
    padding-bottom: 0.5rem;
    flex-shrink: 0;
  }

  .output-header h2 {
    border: none;
    padding: 0;
  }

  @media (max-width: 1024px) {
    .content {
      grid-template-columns: 1fr;
      grid-template-rows: auto 1fr auto;
    }

    .editor-wrapper {
      min-height: 300px;
    }
  }
</style>
