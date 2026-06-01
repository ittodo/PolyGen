<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import PathSelector from "./lib/components/PathSelector.svelte";
  import LanguageSelector from "./lib/components/LanguageSelector.svelte";
  import LogPanel from "./lib/components/LogPanel.svelte";
  import Editor from "./lib/components/Editor.svelte";
  import type { GoToFileEvent } from "./lib/monaco/poly-language";
  import FileTabs from "./lib/components/FileTabs.svelte";
  import FileList from "./lib/components/FileList.svelte";
  import SchemaVisualization from "./lib/components/SchemaVisualization.svelte";
  import TemplateEditor from "./lib/components/TemplateEditor.svelte";

  interface OpenFile {
    path: string;
    name: string;
    content: string;
    isMain: boolean;
    isModified: boolean;
  }

  interface ImportedFile {
    path: string;
    name: string;
  }

  interface RecentProject {
    path: string;
    name: string;
    timestamp: number;
    output_dir?: string;
    templates_dir?: string;
    languages?: string[];
    migration_source?: string;
    baseline_path?: string;
    db_path?: string;
    migration_target?: string;
    schema_hash_policy?: string;
  }

  interface SchemaDiffChange {
    kind: string;
    tableName: string;
    namespace: string;
    columnName: string | null;
    columnType: string | null;
    oldType: string | null;
    newType: string | null;
    isNullable: boolean | null;
    fieldCount: number | null;
  }

  interface SchemaDiffResult {
    changes: SchemaDiffChange[];
    warnings: string[];
  }

  interface ProjectPreset {
    schemaPath: string;
    outputDir: string;
    templatesDir: string;
    languages: string[];
    migrationSource: "baseline" | "db";
    baselinePath: string;
    dbPath: string;
    migrationTarget: string;
    schemaHashPolicy: SchemaHashPolicy;
  }

  type SchemaHashPolicy = "warn" | "fail" | "force";

  const hashPolicyDetails: Record<
    SchemaHashPolicy,
    { label: string; detail: string; tone: "warn" | "fail" | "force" }
  > = {
    warn: {
      label: "Warn",
      detail: "Hash mismatch is logged and migration generation continues.",
      tone: "warn",
    },
    fail: {
      label: "Fail",
      detail: "Hash mismatch stops migration generation before writing SQL.",
      tone: "fail",
    },
    force: {
      label: "Force",
      detail: "Hash mismatch is acknowledged and generation continues explicitly.",
      tone: "force",
    },
  };

  let mainFilePath = $state("");
  let outputDir = $state("");
  let templatesDir = $state("");
  let selectedLanguages = $state<string[]>(["csharp"]);
  let logs = $state<string[]>([]);
  let isGenerating = $state(false);
  let isMigrating = $state(false);
  let migrationSource = $state<"baseline" | "db">("baseline");
  let baselinePath = $state("");
  let dbPath = $state("");
  let migrationTarget = $state("sqlite");
  let schemaHashPolicy = $state<SchemaHashPolicy>("warn");
  let migrationStatus = $derived(getMigrationStatus());
  let copiedCommand = $state<string | null>(null);
  let cliCommands = $derived(getCliCommands());
  let schemaDiff = $state<SchemaDiffResult | null>(null);
  let schemaDiffError = $state<string | null>(null);
  let isComparingSchemas = $state(false);

  let openFiles = $state<OpenFile[]>([]);
  let activeFilePath = $state("");
  let importedFiles = $state<ImportedFile[]>([]); // Always keep track of imports
  let recentProjects = $state<RecentProject[]>([]);

  // View mode: "editor", "visualize", "compare", or "template"
  let viewMode = $state<"editor" | "visualize" | "compare" | "template">("editor");

  // Load recent projects on initialization
  $effect(() => {
    loadRecentProjects();
  });

  async function loadRecentProjects() {
    try {
      recentProjects = await invoke<RecentProject[]>("get_recent_projects");
    } catch (error) {
      console.error("Failed to load recent projects:", error);
    }
  }

  async function addToRecentProjects(path: string) {
    try {
      const project: RecentProject = {
        path,
        name: getFileName(path),
        timestamp: Date.now(),
        output_dir: outputDir || undefined,
        templates_dir: templatesDir || undefined,
        languages: selectedLanguages.length > 0 ? [...selectedLanguages] : undefined,
        migration_source: migrationSource,
        baseline_path: baselinePath || undefined,
        db_path: dbPath || undefined,
        migration_target: migrationTarget || undefined,
        schema_hash_policy: schemaHashPolicy,
      };
      await invoke("add_recent_project", { project });
      await loadRecentProjects();
    } catch (error) {
      console.error("Failed to add recent project:", error);
    }
  }

  async function onRecentProjectSelect(project: RecentProject) {
    // Restore settings from the project if available
    if (project.output_dir) {
      outputDir = project.output_dir;
    }
    if (project.templates_dir) {
      templatesDir = project.templates_dir;
    }
    if (project.languages && project.languages.length > 0) {
      selectedLanguages = [...project.languages];
    }
    if (project.migration_source === "db" || project.migration_source === "baseline") {
      migrationSource = project.migration_source;
    }
    baselinePath = project.baseline_path || baselinePath;
    dbPath = project.db_path || dbPath;
    migrationTarget = project.migration_target || migrationTarget;
    if (
      project.schema_hash_policy === "warn" ||
      project.schema_hash_policy === "fail" ||
      project.schema_hash_policy === "force"
    ) {
      schemaHashPolicy = project.schema_hash_policy;
    }

    // Open the project
    await openMainFile(project.path);
  }

  function buildProjectPreset(): ProjectPreset {
    return {
      schemaPath: mainFilePath,
      outputDir,
      templatesDir,
      languages: [...selectedLanguages],
      migrationSource,
      baselinePath,
      dbPath,
      migrationTarget,
      schemaHashPolicy,
    };
  }

  function applyProjectPreset(preset: ProjectPreset) {
    outputDir = preset.outputDir || "";
    templatesDir = preset.templatesDir || "";
    selectedLanguages =
      Array.isArray(preset.languages) && preset.languages.length > 0
        ? [...preset.languages]
        : ["csharp"];
    migrationSource = preset.migrationSource === "db" ? "db" : "baseline";
    baselinePath = preset.baselinePath || "";
    dbPath = preset.dbPath || "";
    migrationTarget = preset.migrationTarget || "sqlite";
    schemaHashPolicy =
      preset.schemaHashPolicy === "fail" || preset.schemaHashPolicy === "force"
        ? preset.schemaHashPolicy
        : "warn";
  }

  async function saveProjectPreset() {
    if (!mainFilePath) {
      addLog("ERROR: 저장할 스키마 파일을 먼저 열어주세요.");
      return;
    }

    const selected = await save({
      defaultPath: `${getFileName(mainFilePath).replace(/\.poly$/i, "")}.polygen.json`,
      filters: [{ name: "PolyGen Project", extensions: ["json"] }],
    });

    if (!selected) return;

    try {
      await invoke("write_file", {
        path: selected as string,
        content: JSON.stringify(buildProjectPreset(), null, 2),
      });
      addLog(`Saved project preset: ${selected}`);
    } catch (error) {
      addLog(`ERROR: Failed to save project preset - ${error}`);
    }
  }

  async function loadProjectPreset() {
    const selected = await open({
      multiple: false,
      filters: [{ name: "PolyGen Project", extensions: ["json"] }],
    });

    if (!selected) return;

    try {
      const content = await invoke<string>("read_file", { path: selected as string });
      const preset = JSON.parse(content) as ProjectPreset;
      applyProjectPreset(preset);

      if (preset.schemaPath) {
        await openMainFile(preset.schemaPath);
      }

      addLog(`Loaded project preset: ${selected}`);
    } catch (error) {
      addLog(`ERROR: Failed to load project preset - ${error}`);
    }
  }

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

  // Get open paths for FileList
  let openPaths = $derived(openFiles.map((f) => f.path));

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
    importedFiles = [];

    // Set main file path
    mainFilePath = path;

    // Load main file
    await loadFile(path, true);

    // Parse imports and load them
    try {
      const imports = await invoke<string[]>("parse_imports", { filePath: path });

      // Store all imported files
      importedFiles = imports.map((p) => ({
        path: p,
        name: getFileName(p),
      }));

      // Load all imported files
      for (const importPath of imports) {
        await loadFile(importPath, false);
      }

      if (imports.length > 0) {
        addLog(`Loaded ${imports.length} imported file(s)`);
      }
    } catch (error) {
      addLog(`Warning: Could not parse imports - ${error}`);
    }

    // Add to recent projects
    await addToRecentProjects(path);
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
      addLog(`Loaded: ${getFileName(path)}`);
    } catch (error) {
      addLog(`ERROR: Failed to load file - ${error}`);
    }
  }

  async function onImportedFileClick(path: string) {
    // If already open, just switch to it
    if (openFiles.some((f) => f.path === path)) {
      activeFilePath = path;
      return;
    }

    // Otherwise, reload it
    await loadFile(path, false);
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

      addLog(`Saved: ${activeFile.name}`);
    } catch (error) {
      addLog(`ERROR: Failed to save file - ${error}`);
    }
  }

  async function saveAllFiles() {
    for (const file of openFiles) {
      if (file.isModified) {
        try {
          await invoke("write_file", { path: file.path, content: file.content });
          addLog(`Saved: ${file.name}`);
        } catch (error) {
          addLog(`ERROR: Failed to save ${file.name} - ${error}`);
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
    importedFiles = [];
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

  // Store pending navigation for after file load
  let pendingNavigation = $state<{ line: number; column: number } | null>(null);

  async function onGoToFile(event: GoToFileEvent) {
    // Check if file is already open
    const existingFile = openFiles.find((f) => f.path === event.filePath);

    // Set pending navigation - this will be passed to the Editor component
    pendingNavigation = { line: event.line, column: event.column };

    if (existingFile) {
      // File is open, just switch to it
      activeFilePath = event.filePath;
    } else {
      // File is not open, load it first
      await loadFile(event.filePath, false);
    }

    // Clear pending navigation after a short delay
    setTimeout(() => {
      pendingNavigation = null;
    }, 200);
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

  async function selectBaselineFile() {
    const selected = await open({
      multiple: false,
      filters: [{ name: "Poly Schema", extensions: ["poly"] }],
    });
    if (selected) {
      baselinePath = selected as string;
      migrationSource = "baseline";
    }
  }

  async function selectDbFile() {
    const selected = await open({
      multiple: false,
      filters: [{ name: "SQLite Database", extensions: ["db", "sqlite", "sqlite3"] }],
    });
    if (selected) {
      dbPath = selected as string;
      migrationSource = "db";
    }
  }

  function addLog(message: string) {
    logs = [...logs, `[${new Date().toLocaleTimeString()}] ${message}`];
  }

  function getMigrationStatus() {
    if (!mainFilePath) {
      return { label: "Missing schema", detail: "Open a schema before migration.", tone: "blocked" };
    }
    if (!outputDir) {
      return { label: "Missing output", detail: "Select an output directory.", tone: "blocked" };
    }
    if (migrationSource === "baseline" && !baselinePath) {
      return { label: "Missing baseline", detail: "Select the previous .poly schema.", tone: "blocked" };
    }
    if (migrationSource === "db" && !dbPath) {
      return { label: "Missing database", detail: "Select the SQLite database.", tone: "blocked" };
    }
    if (migrationSource === "db") {
      return {
        label: `${hashPolicyDetails[schemaHashPolicy].label} policy`,
        detail: hashPolicyDetails[schemaHashPolicy].detail,
        tone: hashPolicyDetails[schemaHashPolicy].tone,
      };
    }

    return {
      label: "Ready",
      detail: "Baseline schema comparison is ready.",
      tone: "ready",
    };
  }

  function quoteArg(value: string): string {
    if (!value) return "\"\"";
    return `"${value.replace(/`/g, "``").replace(/"/g, "`\"").replace(/\$/g, "`$")}"`;
  }

  function buildGenerateCommand(lang: string): string {
    const parts = [
      "cargo run -- generate",
      "--schema-path",
      quoteArg(mainFilePath),
      "--lang",
      lang,
    ];

    if (outputDir) {
      parts.push("--output-dir", quoteArg(outputDir));
    }
    if (templatesDir) {
      parts.push("--templates-dir", quoteArg(templatesDir));
    }

    return parts.join(" ");
  }

  function buildWatchCommand(): string {
    const lang = selectedLanguages[0] || "csharp";
    const parts = [
      "cargo run -- watch",
      "--schema",
      quoteArg(mainFilePath),
      "--lang",
      lang,
    ];

    if (outputDir) {
      parts.push("--output-dir", quoteArg(outputDir));
    }
    if (templatesDir) {
      parts.push("--templates-dir", quoteArg(templatesDir));
    }

    return parts.join(" ");
  }

  function buildMigrationCommand(): string {
    const parts = [
      "cargo run -- migrate",
      "--schema-path",
      quoteArg(mainFilePath),
      "--output-dir",
      quoteArg(outputDir),
      "--target",
      migrationTarget || "sqlite",
    ];

    if (migrationSource === "baseline") {
      parts.push("--baseline", quoteArg(baselinePath));
    } else {
      parts.push("--db", quoteArg(dbPath));
      parts.push("--schema-hash-policy", schemaHashPolicy);
    }

    return parts.join(" ");
  }

  function getCliCommands() {
    const generate =
      mainFilePath && selectedLanguages.length > 0
        ? selectedLanguages.map((lang) => ({ lang, command: buildGenerateCommand(lang) }))
        : [];

    const watch = mainFilePath ? buildWatchCommand() : "";
    const migrationReady =
      mainFilePath &&
      outputDir &&
      ((migrationSource === "baseline" && baselinePath) ||
        (migrationSource === "db" && dbPath));
    const migration = migrationReady ? buildMigrationCommand() : "";

    return { generate, watch, migration };
  }

  async function copyCommand(id: string, command: string) {
    if (!command) return;

    try {
      await navigator.clipboard.writeText(command);
      copiedCommand = id;
      addLog(`Copied command: ${id}`);
      setTimeout(() => {
        if (copiedCommand === id) {
          copiedCommand = null;
        }
      }, 1500);
    } catch (error) {
      addLog(`ERROR: Failed to copy command - ${error}`);
    }
  }

  function getDiffSummary(diff: SchemaDiffResult | null) {
    const summary = {
      added: 0,
      removed: 0,
      changed: 0,
      warnings: diff?.warnings.length ?? 0,
    };

    if (!diff) return summary;

    for (const change of diff.changes) {
      if (change.kind.endsWith("_added")) {
        summary.added++;
      } else if (change.kind.endsWith("_removed")) {
        summary.removed++;
      } else {
        summary.changed++;
      }
    }

    return summary;
  }

  function getDiffChangeTone(kind: string): "added" | "removed" | "changed" {
    if (kind.endsWith("_added")) return "added";
    if (kind.endsWith("_removed")) return "removed";
    return "changed";
  }

  function getDiffChangeTitle(change: SchemaDiffChange): string {
    switch (change.kind) {
      case "table_added":
        return `Table added: ${change.tableName}`;
      case "table_removed":
        return `Table removed: ${change.tableName}`;
      case "column_added":
        return `Column added: ${change.tableName}.${change.columnName}`;
      case "column_removed":
        return `Column removed: ${change.tableName}.${change.columnName}`;
      case "column_type_changed":
        return `Column type changed: ${change.tableName}.${change.columnName}`;
      default:
        return `${change.kind}: ${change.tableName}`;
    }
  }

  function getDiffChangeDetail(change: SchemaDiffChange): string {
    switch (change.kind) {
      case "table_added":
        return `${change.namespace || "(root)"} · ${change.fieldCount ?? 0} fields`;
      case "table_removed":
        return `${change.namespace || "(root)"} · destructive`;
      case "column_added":
        return `${change.columnType}${change.isNullable ? " nullable" : " required"}`;
      case "column_removed":
        return "destructive";
      case "column_type_changed":
        return `${change.oldType} -> ${change.newType}`;
      default:
        return change.namespace || "(root)";
    }
  }

  async function compareSchemas() {
    if (!mainFilePath) {
      schemaDiffError = "Open a current schema file first.";
      return;
    }
    if (!baselinePath) {
      schemaDiffError = "Select a baseline schema file first.";
      return;
    }

    const hasModified = openFiles.some((f) => f.isModified);
    if (hasModified) {
      await saveAllFiles();
    }

    isComparingSchemas = true;
    schemaDiffError = null;

    try {
      schemaDiff = await invoke<SchemaDiffResult>("get_schema_diff", {
        baselinePath,
        schemaPath: mainFilePath,
      });
      addLog(`Schema diff loaded: ${schemaDiff.changes.length} change(s)`);
    } catch (error) {
      schemaDiff = null;
      schemaDiffError = String(error);
      addLog(`ERROR: schema diff failed - ${error}`);
    } finally {
      isComparingSchemas = false;
    }
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

  async function migrate() {
    if (!mainFilePath) {
      addLog("ERROR: 먼저 스키마 파일을 열어주세요.");
      return;
    }
    if (!outputDir) {
      addLog("ERROR: 출력 디렉토리를 선택해주세요.");
      return;
    }
    if (migrationSource === "baseline" && !baselinePath) {
      addLog("ERROR: 기준 스키마 파일을 선택해주세요.");
      return;
    }
    if (migrationSource === "db" && !dbPath) {
      addLog("ERROR: SQLite DB 파일을 선택해주세요.");
      return;
    }

    const hasModified = openFiles.some((f) => f.isModified);
    if (hasModified) {
      await saveAllFiles();
    }

    isMigrating = true;
    addLog(
      `Generating migration from ${
        migrationSource === "baseline" ? "baseline schema" : "SQLite DB"
      }...`
    );

    try {
      const result = await invoke<string>("run_migrate", {
        baselinePath: migrationSource === "baseline" ? baselinePath : null,
        dbPath: migrationSource === "db" ? dbPath : null,
        schemaPath: mainFilePath,
        outputDir,
        target: migrationTarget || null,
        schemaHashPolicy: migrationSource === "db" ? schemaHashPolicy : null,
      });
      addLog(`SUCCESS: migration - ${result}`);
    } catch (error) {
      addLog(`ERROR: migration - ${error}`);
    } finally {
      isMigrating = false;
    }
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
      <span class="toolbar-separator"></span>
      <button
        class="toolbar-btn"
        class:active={viewMode === "editor"}
        onclick={() => (viewMode = "editor")}
        title="Editor View"
      >Editor</button>
      <button
        class="toolbar-btn"
        class:active={viewMode === "visualize"}
        onclick={() => (viewMode = "visualize")}
        title="Schema Visualization"
      >Visualize</button>
      <button
        class="toolbar-btn"
        class:active={viewMode === "compare"}
        onclick={() => (viewMode = "compare")}
        title="Schema Compare"
      >Compare</button>
      <button
        class="toolbar-btn"
        class:active={viewMode === "template"}
        onclick={() => (viewMode = "template")}
        title="Template Editor"
      >Templates</button>
    </div>
    <div class="header-right">
      <span class="file-count">{openFiles.length} file(s) open</span>
    </div>
  </header>

  <div class="content">
    <section class="settings">
      <h2>Settings</h2>

      <PathSelector
        label="Schema Path"
        value={mainFilePath}
        placeholder="Open a schema file..."
        onSelect={selectSchemaFile}
        {recentProjects}
        onRecentSelect={onRecentProjectSelect}
      />

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

      <div class="project-panel">
        <h2>Project Preset</h2>
        <div class="project-actions">
          <button class="secondary" onclick={loadProjectPreset} type="button">
            Load
          </button>
          <button
            class="secondary"
            onclick={saveProjectPreset}
            disabled={!mainFilePath}
            type="button"
          >
            Save
          </button>
        </div>
        <p>Save schema, output, template, language, and migration settings.</p>
      </div>

      <div class="actions">
        <button class="primary" onclick={generate} disabled={isGenerating}>
          {isGenerating ? "Generating..." : "Generate"}
        </button>
      </div>

      <div class="migration-panel">
        <h2>Migration</h2>

        <div class="source-toggle">
          <label class:active={migrationSource === "baseline"}>
            <input
              type="radio"
              name="migration-source"
              value="baseline"
              checked={migrationSource === "baseline"}
              onchange={() => (migrationSource = "baseline")}
            />
            <span>Baseline</span>
          </label>
          <label class:active={migrationSource === "db"}>
            <input
              type="radio"
              name="migration-source"
              value="db"
              checked={migrationSource === "db"}
              onchange={() => (migrationSource = "db")}
            />
            <span>SQLite DB</span>
          </label>
        </div>

        <div class={`migration-status ${migrationStatus.tone}`}>
          <span class="status-dot" aria-hidden="true"></span>
          <div>
            <strong>{migrationStatus.label}</strong>
            <p>{migrationStatus.detail}</p>
          </div>
        </div>

        {#if migrationSource === "baseline"}
          <PathSelector
            label="Baseline Schema"
            value={baselinePath}
            placeholder="Select old .poly file..."
            onSelect={selectBaselineFile}
          />
        {:else}
          <PathSelector
            label="Database File"
            value={dbPath}
            placeholder="Select SQLite database..."
            onSelect={selectDbFile}
          />
        {/if}

        <label class="target-select">
          <span>Target DB</span>
          <select bind:value={migrationTarget}>
            <option value="sqlite">SQLite</option>
            <option value="mysql">MySQL</option>
          </select>
        </label>

        {#if migrationSource === "db"}
          <label class="target-select">
            <span>Hash Policy</span>
            <select bind:value={schemaHashPolicy}>
              <option value="warn">Warn</option>
              <option value="fail">Fail</option>
              <option value="force">Force</option>
            </select>
          </label>

          <div class={`policy-note ${hashPolicyDetails[schemaHashPolicy].tone}`}>
            {hashPolicyDetails[schemaHashPolicy].detail}
          </div>
        {/if}

        <button class="secondary migrate-btn" onclick={migrate} disabled={isMigrating}>
          {isMigrating ? "Migrating..." : "Generate Migration"}
        </button>
      </div>

      <div class="cli-panel">
        <h2>CLI</h2>

        {#if cliCommands.generate.length > 0}
          <div class="command-group">
            <div class="command-group-title">Generate</div>
            {#each cliCommands.generate as item}
              <div class="command-row">
                <pre>{item.command}</pre>
                <button
                  class="copy-btn"
                  onclick={() => copyCommand(`generate-${item.lang}`, item.command)}
                  type="button"
                >
                  {copiedCommand === `generate-${item.lang}` ? "Copied" : "Copy"}
                </button>
              </div>
            {/each}
          </div>
        {/if}

        {#if cliCommands.watch}
          <div class="command-group">
            <div class="command-group-title">Watch</div>
            <div class="command-row">
              <pre>{cliCommands.watch}</pre>
              <button
                class="copy-btn"
                onclick={() => copyCommand("watch", cliCommands.watch)}
                type="button"
              >
                {copiedCommand === "watch" ? "Copied" : "Copy"}
              </button>
            </div>
          </div>
        {/if}

        {#if cliCommands.migration}
          <div class="command-group">
            <div class="command-group-title">Migration</div>
            <div class="command-row">
              <pre>{cliCommands.migration}</pre>
              <button
                class="copy-btn"
                onclick={() => copyCommand("migration", cliCommands.migration)}
                type="button"
              >
                {copiedCommand === "migration" ? "Copied" : "Copy"}
              </button>
            </div>
          </div>
        {/if}

        {#if cliCommands.generate.length === 0 && !cliCommands.watch && !cliCommands.migration}
          <div class="command-empty">Open a schema file.</div>
        {/if}
      </div>

      {#if importedFiles.length > 0}
        <FileList
          files={importedFiles}
          {openPaths}
          activePath={activeFilePath}
          onFileClick={onImportedFileClick}
        />
      {/if}
    </section>

    <section class="editor-section">
      {#if viewMode === "editor"}
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
                <Editor
                  value={activeFile.content}
                  onChange={onEditorChange}
                  filePath={activeFile.path}
                  {onGoToFile}
                  initialPosition={pendingNavigation}
                />
              {/key}
            {/if}
          </div>
        {:else}
          <div class="empty-editor">
            <p>No file open</p>
            <button class="secondary" onclick={selectSchemaFile}>Open Schema File</button>
          </div>
        {/if}
      {:else if viewMode === "visualize"}
        <!-- Visualization View -->
        <div class="visualization-wrapper">
          {#if mainFilePath}
            <SchemaVisualization schemaPath={mainFilePath} />
          {:else}
            <div class="empty-editor">
              <p>Open a schema file to visualize</p>
              <button class="secondary" onclick={selectSchemaFile}>Open Schema File</button>
            </div>
          {/if}
        </div>
      {:else if viewMode === "compare"}
        <div class="compare-wrapper">
          <div class="compare-toolbar">
            <div>
              <h2>Schema Compare</h2>
              <p>Compare the selected baseline schema against the current schema.</p>
            </div>
            <button class="primary" onclick={compareSchemas} disabled={isComparingSchemas}>
              {isComparingSchemas ? "Comparing..." : "Compare Schemas"}
            </button>
          </div>

          <div class="compare-paths">
            <div class="compare-path">
              <span>Baseline</span>
              <strong title={baselinePath}>{baselinePath || "Not selected"}</strong>
            </div>
            <div class="compare-path">
              <span>Current</span>
              <strong title={mainFilePath}>{mainFilePath || "Not selected"}</strong>
            </div>
          </div>

          {#if schemaDiffError}
            <div class="compare-error">{schemaDiffError}</div>
          {:else if schemaDiff}
            {@const diffSummary = getDiffSummary(schemaDiff)}
            <div class="compare-summary">
              <div class="diff-stat added">
                <strong>{diffSummary.added}</strong>
                <span>Added</span>
              </div>
              <div class="diff-stat removed">
                <strong>{diffSummary.removed}</strong>
                <span>Removed</span>
              </div>
              <div class="diff-stat changed">
                <strong>{diffSummary.changed}</strong>
                <span>Changed</span>
              </div>
              <div class="diff-stat warnings">
                <strong>{diffSummary.warnings}</strong>
                <span>Warnings</span>
              </div>
            </div>

            {#if schemaDiff.warnings.length > 0}
              <div class="compare-warnings">
                {#each schemaDiff.warnings as warning}
                  <div class="warning-item">{warning}</div>
                {/each}
              </div>
            {/if}

            {#if schemaDiff.changes.length === 0}
              <div class="compare-empty">No schema changes detected.</div>
            {:else}
              <div class="diff-list">
                {#each schemaDiff.changes as change}
                  {@const tone = getDiffChangeTone(change.kind)}
                  <div class={`diff-item ${tone}`}>
                    <span class="diff-kind">{tone}</span>
                    <div class="diff-body">
                      <strong>{getDiffChangeTitle(change)}</strong>
                      <span>{getDiffChangeDetail(change)}</span>
                    </div>
                  </div>
                {/each}
              </div>
            {/if}
          {:else}
            <div class="compare-empty">
              Select a baseline schema in the Migration panel, then compare.
            </div>
          {/if}
        </div>
      {:else}
        <!-- Template Editor View -->
        <div class="template-wrapper">
          <TemplateEditor
            templatesDir={templatesDir}
            schemaPath={mainFilePath}
            onLog={addLog}
          />
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

  .toolbar-btn.active {
    background-color: var(--accent);
    color: white;
    border-color: var(--accent);
  }

  .toolbar-separator {
    width: 1px;
    height: 1.5rem;
    background-color: var(--border);
    margin: 0 0.25rem;
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
    padding-top: 0.75rem;
  }

  .actions button {
    width: 100%;
    padding: 0.625rem;
    font-size: 0.875rem;
  }

  .project-panel {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    padding: 0.75rem;
    background-color: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
  }

  .project-panel h2 {
    margin: 0;
    padding: 0;
    border: 0;
    font-size: 0.875rem;
  }

  .project-panel p {
    margin: 0;
    color: var(--text-secondary);
    font-size: 0.75rem;
    line-height: 1.35;
  }

  .project-actions {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.5rem;
  }

  .project-actions button {
    width: 100%;
  }

  .migration-panel {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    padding-top: 0.75rem;
    border-top: 1px solid var(--border);
  }

  .source-toggle {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.5rem;
  }

  .source-toggle label {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0.375rem;
    min-height: 2rem;
    padding: 0.375rem 0.5rem;
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
    background-color: var(--bg-primary);
    cursor: pointer;
    font-size: 0.8125rem;
  }

  .source-toggle label.active {
    border-color: var(--accent);
    background-color: var(--bg-hover);
  }

  .source-toggle input {
    width: 0.875rem;
    height: 0.875rem;
    margin: 0;
  }

  .migration-status {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 0.5rem;
    align-items: start;
    min-height: 3.25rem;
    padding: 0.625rem;
    color: var(--text-primary);
    background-color: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
  }

  .migration-status strong {
    display: block;
    margin-bottom: 0.125rem;
    font-size: 0.8125rem;
  }

  .migration-status p,
  .policy-note {
    color: var(--text-secondary);
    font-size: 0.75rem;
    line-height: 1.35;
  }

  .status-dot {
    width: 0.5rem;
    height: 0.5rem;
    margin-top: 0.3125rem;
    border-radius: 50%;
    background-color: var(--success);
  }

  .migration-status.blocked .status-dot {
    background-color: var(--error);
  }

  .migration-status.warn .status-dot,
  .policy-note.warn {
    color: var(--warning);
  }

  .migration-status.warn .status-dot {
    background-color: var(--warning);
  }

  .migration-status.fail .status-dot,
  .policy-note.fail {
    color: var(--error);
  }

  .migration-status.fail .status-dot {
    background-color: var(--error);
  }

  .migration-status.force .status-dot,
  .policy-note.force {
    color: var(--accent);
  }

  .migration-status.force .status-dot {
    background-color: var(--accent);
  }

  .policy-note {
    margin-top: -0.5rem;
    padding: 0.5rem;
    background-color: var(--bg-primary);
    border-left: 2px solid currentColor;
    border-radius: 4px;
  }

  .target-select {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    color: var(--text-primary);
    font-size: 0.875rem;
  }

  .target-select select {
    padding: 0.5rem;
    color: var(--text-primary);
    background-color: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
  }

  .migrate-btn {
    width: 100%;
    padding: 0.625rem;
    font-size: 0.875rem;
  }

  .cli-panel {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
    padding-top: 0.75rem;
    border-top: 1px solid var(--border);
  }

  .command-group {
    display: flex;
    flex-direction: column;
    gap: 0.375rem;
  }

  .command-group-title {
    color: var(--text-secondary);
    font-size: 0.75rem;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }

  .command-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 0.375rem;
    align-items: stretch;
  }

  .command-row pre {
    min-width: 0;
    margin: 0;
    padding: 0.5rem;
    overflow-x: auto;
    color: var(--text-primary);
    background-color: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    font-size: 0.75rem;
    line-height: 1.4;
    white-space: pre;
  }

  .copy-btn {
    min-width: 4.25rem;
    padding: 0.375rem 0.5rem;
    color: var(--text-primary);
    background-color: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.75rem;
  }

  .copy-btn:hover {
    border-color: var(--accent);
    background-color: var(--bg-hover);
  }

  .command-empty {
    padding: 0.5rem;
    color: var(--text-secondary);
    background-color: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    font-size: 0.8125rem;
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

  .visualization-wrapper {
    flex: 1;
    min-height: 0;
    overflow: hidden;
    background-color: var(--bg-primary);
    border-radius: 0 0 8px 8px;
  }

  .compare-wrapper {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 1rem;
    background-color: var(--bg-primary);
  }

  .compare-toolbar {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 1rem;
    padding-bottom: 1rem;
    border-bottom: 1px solid var(--border);
  }

  .compare-toolbar h2 {
    margin: 0;
    padding: 0;
    border: 0;
  }

  .compare-toolbar p {
    margin: 0.25rem 0 0;
    color: var(--text-secondary);
    font-size: 0.875rem;
  }

  .compare-paths {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 0.75rem;
    margin-top: 1rem;
  }

  .compare-path {
    min-width: 0;
    padding: 0.75rem;
    background-color: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
  }

  .compare-path span {
    display: block;
    margin-bottom: 0.25rem;
    color: var(--text-secondary);
    font-size: 0.75rem;
    text-transform: uppercase;
  }

  .compare-path strong {
    display: block;
    overflow: hidden;
    color: var(--text-primary);
    font-size: 0.8125rem;
    font-family: monospace;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .compare-summary {
    display: grid;
    grid-template-columns: repeat(4, minmax(0, 1fr));
    gap: 0.75rem;
    margin-top: 1rem;
  }

  .diff-stat {
    padding: 0.75rem;
    background-color: var(--bg-secondary);
    border: 1px solid var(--border);
    border-left-width: 3px;
    border-radius: 6px;
  }

  .diff-stat strong {
    display: block;
    font-size: 1.125rem;
  }

  .diff-stat span {
    color: var(--text-secondary);
    font-size: 0.75rem;
  }

  .diff-stat.added {
    border-left-color: var(--success);
  }

  .diff-stat.removed,
  .diff-stat.warnings {
    border-left-color: var(--error);
  }

  .diff-stat.changed {
    border-left-color: var(--warning);
  }

  .compare-warnings,
  .diff-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    margin-top: 1rem;
  }

  .warning-item,
  .compare-error,
  .compare-empty {
    padding: 0.75rem;
    background-color: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    color: var(--text-secondary);
  }

  .compare-error,
  .warning-item {
    border-left: 3px solid var(--error);
    color: var(--error);
  }

  .diff-item {
    display: grid;
    grid-template-columns: 5rem minmax(0, 1fr);
    gap: 0.75rem;
    align-items: center;
    padding: 0.75rem;
    background-color: var(--bg-secondary);
    border: 1px solid var(--border);
    border-left-width: 3px;
    border-radius: 6px;
  }

  .diff-item.added {
    border-left-color: var(--success);
  }

  .diff-item.removed {
    border-left-color: var(--error);
  }

  .diff-item.changed {
    border-left-color: var(--warning);
  }

  .diff-kind {
    color: var(--text-secondary);
    font-size: 0.7rem;
    font-weight: 700;
    text-transform: uppercase;
  }

  .diff-body {
    min-width: 0;
  }

  .diff-body strong,
  .diff-body span {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .diff-body span {
    margin-top: 0.125rem;
    color: var(--text-secondary);
    font-size: 0.8125rem;
  }

  .template-wrapper {
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

    .compare-paths,
    .compare-summary {
      grid-template-columns: 1fr;
    }
  }
</style>
