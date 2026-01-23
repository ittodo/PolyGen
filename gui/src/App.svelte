<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import PathSelector from "./lib/components/PathSelector.svelte";
  import LanguageSelector from "./lib/components/LanguageSelector.svelte";
  import LogPanel from "./lib/components/LogPanel.svelte";
  import Editor from "./lib/components/Editor.svelte";
  import FileTabs from "./lib/components/FileTabs.svelte";

  interface OpenFile {
    path: string;
    name: string;
    content: string;
    isMain: boolean;
    isModified: boolean;
  }

  let mainFilePath = $state("");
  let outputDir = $state("");
  let templatesDir = $state("");
  let selectedLanguages = $state<string[]>(["csharp"]);
  let logs = $state<string[]>([]);
  let isGenerating = $state(false);

  let openFiles = $state<OpenFile[]>([]);
  let activeFilePath = $state("");

  // Get active file
  let activeFile = $derived(openFiles.find((f) => f.path === activeFilePath));

  // Get tabs for display
  let tabs = $derived(
    openFiles.map((f) => ({
      path: f.path,
      name: f.name,
      isMain: f.isMain,
      isModified: f.isModified,
    }))
  );

  function getFileName(path: string): string {
    return path.split(/[/\\]/).pop() || "Untitled";
  }

  async function selectSchemaFile() {
    const selected = await open({
      multiple: false,
      filters: [{ name: "Poly Schema", extensions: ["poly"] }],
    });
    if (selected) {
      await openMainFile(selected as string);
    }
  }

  async function openMainFile(path: string) {
    // Close all files first
    openFiles = [];

    // Set main file path
    mainFilePath = path;

    // Load main file
    await loadFile(path, true);

    // Parse imports and load them
    try {
      const imports = await invoke<string[]>("parse_imports", { filePath: path });
      for (const importPath of imports) {
        await loadFile(importPath, false);
      }
      if (imports.length > 0) {
        addLog(`Loaded ${imports.length} imported file(s)`);
      }
    } catch (error) {
      addLog(`Warning: Could not parse imports - ${error}`);
    }
  }

  async function loadFile(path: string, isMain: boolean) {
    // Check if already open
    if (openFiles.some((f) => f.path === path)) {
      activeFilePath = path;
      return;
    }

    try {
      const content = await invoke<string>("read_file", { path });
      const newFile: OpenFile = {
        path,
        name: getFileName(path),
        content,
        isMain,
        isModified: false,
      };

      if (isMain) {
        openFiles = [newFile, ...openFiles];
      } else {
        openFiles = [...openFiles, newFile];
      }

      activeFilePath = path;
      addLog(`Loaded: ${path}`);
    } catch (error) {
      addLog(`ERROR: Failed to load file - ${error}`);
    }
  }

  async function saveFile() {
    if (!activeFile) return;

    if (!activeFile.path) {
      await saveFileAs();
      return;
    }

    try {
      await invoke("write_file", {
        path: activeFile.path,
        content: activeFile.content,
      });

      // Update modified status
      openFiles = openFiles.map((f) =>
        f.path === activeFile.path ? { ...f, isModified: false } : f
      );

      addLog(`Saved: ${activeFile.path}`);
    } catch (error) {
      addLog(`ERROR: Failed to save file - ${error}`);
    }
  }

  async function saveAllFiles() {
    for (const file of openFiles) {
      if (file.isModified) {
        try {
          await invoke("write_file", { path: file.path, content: file.content });
          addLog(`Saved: ${file.path}`);
        } catch (error) {
          addLog(`ERROR: Failed to save ${file.path} - ${error}`);
        }
      }
    }
    openFiles = openFiles.map((f) => ({ ...f, isModified: false }));
  }

  async function saveFileAs() {
    const selected = await save({
      filters: [{ name: "Poly Schema", extensions: ["poly"] }],
    });
    if (selected && activeFile) {
      const newPath = selected as string;

      // Update the file path
      openFiles = openFiles.map((f) =>
        f.path === activeFilePath
          ? { ...f, path: newPath, name: getFileName(newPath) }
          : f
      );

      if (activeFile.isMain) {
        mainFilePath = newPath;
      }

      activeFilePath = newPath;
      await saveFile();
    }
  }

  async function newFile() {
    // Clear all files
    openFiles = [];
    mainFilePath = "";

    const newFile: OpenFile = {
      path: "",
      name: "Untitled",
      content: `// New PolyGen Schema
namespace example {
    table Sample {
        id: u32 primary_key;
        name: string max_length(100);
    }
}
`,
      isMain: true,
      isModified: false,
    };

    openFiles = [newFile];
    activeFilePath = "";
  }

  function onSelectTab(path: string) {
    activeFilePath = path;
  }

  function onCloseTab(path: string) {
    const file = openFiles.find((f) => f.path === path);
    if (!file || file.isMain) return; // Can't close main file

    openFiles = openFiles.filter((f) => f.path !== path);

    // If closing active tab, switch to main file
    if (activeFilePath === path) {
      activeFilePath = mainFilePath;
    }
  }

  function onEditorChange(value: string) {
    openFiles = openFiles.map((f) =>
      f.path === activeFilePath ? { ...f, content: value, isModified: true } : f
    );
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

  async function generate() {
    if (!mainFilePath) {
      addLog("ERROR: 먼저 스키마 파일을 저장해주세요.");
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

    // Save all modified files before generate
    const hasModified = openFiles.some((f) => f.isModified);
    if (hasModified) {
      await saveAllFiles();
    }

    isGenerating = true;

    for (const lang of selectedLanguages) {
      addLog(`Generating ${lang}...`);
      try {
        const result = await invoke<string>("run_generate", {
          schemaPath: mainFilePath,
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
      <button class="toolbar-btn" onclick={saveAllFiles} title="Save All">Save All</button>
    </div>
    <div class="header-right">
      <span class="file-count">{openFiles.length} file(s)</span>
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
      {#if openFiles.length > 0}
        <FileTabs
          {tabs}
          activeTab={activeFilePath}
          {onSelectTab}
          {onCloseTab}
        />
        <div class="editor-wrapper">
          {#if activeFile}
            {#key activeFilePath}
              <Editor value={activeFile.content} onChange={onEditorChange} />
            {/key}
          {/if}
        </div>
      {:else}
        <div class="empty-editor">
          <p>No file open</p>
          <button class="secondary" onclick={selectSchemaFile}>Open Schema File</button>
        </div>
      {/if}
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

  .header-right .file-count {
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
    padding: 0;
    gap: 0;
  }

  .editor-wrapper {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }

  .empty-editor {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 1rem;
    color: var(--text-secondary);
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
