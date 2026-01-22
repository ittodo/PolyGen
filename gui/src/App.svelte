<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open } from "@tauri-apps/plugin-dialog";
  import PathSelector from "./lib/components/PathSelector.svelte";
  import LanguageSelector from "./lib/components/LanguageSelector.svelte";
  import LogPanel from "./lib/components/LogPanel.svelte";

  let schemaPath = $state("");
  let outputDir = $state("");
  let templatesDir = $state("");
  let selectedLanguages = $state<string[]>(["csharp"]);
  let logs = $state<string[]>([]);
  let isGenerating = $state(false);

  async function selectSchemaFile() {
    const selected = await open({
      multiple: false,
      filters: [{ name: "Poly Schema", extensions: ["poly"] }],
    });
    if (selected) {
      schemaPath = selected as string;
    }
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
    <h1>PolyGen</h1>
    <p>Polyglot Code Generator</p>
  </header>

  <div class="content">
    <section class="settings">
      <h2>Settings</h2>

      <PathSelector
        label="Schema File (.poly)"
        value={schemaPath}
        placeholder="Select schema file..."
        onSelect={selectSchemaFile}
      />

      <PathSelector
        label="Output Directory"
        value={outputDir}
        placeholder="Select output directory..."
        onSelect={selectOutputDir}
      />

      <PathSelector
        label="Templates Directory (optional)"
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
    padding: 1rem;
    gap: 1rem;
  }

  header {
    text-align: center;
    padding: 1rem 0;
  }

  header h1 {
    font-size: 1.5rem;
    color: var(--accent);
  }

  header p {
    font-size: 0.875rem;
    color: var(--text-secondary);
  }

  .content {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 1rem;
    flex: 1;
    min-height: 0;
  }

  section {
    background-color: var(--bg-secondary);
    border-radius: 8px;
    padding: 1rem;
    display: flex;
    flex-direction: column;
    gap: 1rem;
  }

  section h2 {
    font-size: 1rem;
    color: var(--text-primary);
    border-bottom: 1px solid var(--border);
    padding-bottom: 0.5rem;
  }

  .actions {
    margin-top: auto;
    padding-top: 1rem;
  }

  .actions button {
    width: 100%;
    padding: 0.75rem;
    font-size: 1rem;
  }

  .output {
    display: flex;
    flex-direction: column;
  }

  .output-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--border);
    padding-bottom: 0.5rem;
  }

  .output-header h2 {
    border: none;
    padding: 0;
  }

  @media (max-width: 768px) {
    .content {
      grid-template-columns: 1fr;
    }
  }
</style>
