<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";

  interface TemplateFileInfo {
    name: string;
    path: string;
    relative_path: string;
    is_directory: boolean;
    children: TemplateFileInfo[];
  }

  interface TemplateLanguageInfo {
    id: string;
    name: string;
    path: string;
    file_count: number;
  }

  interface Props {
    templatesDir?: string;
    selectedLang: string;
    selectedFile: string;
    refreshKey?: number;
    onSelectLang: (lang: string) => void;
    onSelectFile: (file: TemplateFileInfo) => void;
    onCreateLanguage: () => void;
  }

  let {
    templatesDir,
    selectedLang,
    selectedFile,
    refreshKey = 0,
    onSelectLang,
    onSelectFile,
    onCreateLanguage,
  }: Props = $props();

  let languages = $state<TemplateLanguageInfo[]>([]);
  let files = $state<TemplateFileInfo[]>([]);
  let expandedDirs = $state<Set<string>>(new Set());
  let loading = $state(false);
  let error = $state("");

  // Load languages on mount, when templatesDir changes, or when refreshKey changes
  $effect(() => {
    // Track refreshKey to trigger reload
    void refreshKey;
    loadLanguages();
  });

  // Load files when selectedLang changes
  $effect(() => {
    if (selectedLang) {
      loadFiles(selectedLang);
    }
  });

  async function loadLanguages() {
    loading = true;
    error = "";
    try {
      languages = await invoke<TemplateLanguageInfo[]>("list_template_languages", {
        templatesDir: templatesDir || null,
      });
      // Auto-select first language if none selected
      if (languages.length > 0 && !selectedLang) {
        onSelectLang(languages[0].id);
      }
    } catch (e) {
      error = `Failed to load languages: ${e}`;
      console.error(error);
    } finally {
      loading = false;
    }
  }

  async function loadFiles(lang: string) {
    loading = true;
    error = "";
    try {
      files = await invoke<TemplateFileInfo[]>("list_template_files", {
        lang,
        templatesDir: templatesDir || null,
      });
      // Expand root directories by default
      const newExpanded = new Set<string>();
      for (const file of files) {
        if (file.is_directory) {
          newExpanded.add(file.relative_path);
        }
      }
      expandedDirs = newExpanded;
    } catch (e) {
      error = `Failed to load files: ${e}`;
      console.error(error);
    } finally {
      loading = false;
    }
  }

  function toggleDir(path: string) {
    const newExpanded = new Set(expandedDirs);
    if (newExpanded.has(path)) {
      newExpanded.delete(path);
    } else {
      newExpanded.add(path);
    }
    expandedDirs = newExpanded;
  }

  function getFileIcon(file: TemplateFileInfo): string {
    if (file.is_directory) {
      return expandedDirs.has(file.relative_path) ? "folder-open" : "folder";
    }
    if (file.name.endsWith(".rhai")) return "code";
    if (file.name.endsWith(".ptpl")) return "template";
    if (file.name.endsWith(".toml")) return "settings";
    return "file";
  }

  function handleFileClick(file: TemplateFileInfo) {
    if (file.is_directory) {
      toggleDir(file.relative_path);
    } else {
      onSelectFile(file);
    }
  }
</script>

<div class="template-file-tree">
  <div class="tree-header">
    <h3>Templates</h3>
    <button class="icon-btn" onclick={onCreateLanguage} title="Create New Language">
      +
    </button>
  </div>

  {#if error}
    <div class="error">{error}</div>
  {/if}

  <div class="language-selector">
    <select
      value={selectedLang}
      onchange={(e) => onSelectLang((e.target as HTMLSelectElement).value)}
    >
      {#each languages as lang}
        <option value={lang.id}>{lang.name} ({lang.file_count})</option>
      {/each}
    </select>
  </div>

  <div class="file-list">
    {#if loading}
      <div class="loading">Loading...</div>
    {:else}
      {#each files as file}
        {@render fileItem(file, 0)}
      {/each}
    {/if}
  </div>
</div>

{#snippet fileItem(file: TemplateFileInfo, depth: number)}
  <div
    class="file-item"
    class:selected={file.relative_path === selectedFile}
    class:directory={file.is_directory}
    style="padding-left: {depth * 16 + 8}px"
    onclick={() => handleFileClick(file)}
    onkeydown={(e) => e.key === 'Enter' && handleFileClick(file)}
    role="treeitem"
    tabindex="0"
  >
    <span class="icon {getFileIcon(file)}"></span>
    <span class="name">{file.name}</span>
  </div>

  {#if file.is_directory && expandedDirs.has(file.relative_path)}
    {#each file.children as child}
      {@render fileItem(child, depth + 1)}
    {/each}
  {/if}
{/snippet}

<style>
  .template-file-tree {
    display: flex;
    flex-direction: column;
    height: 100%;
    background-color: var(--bg-secondary);
    border-radius: 8px;
    overflow: hidden;
  }

  .tree-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.75rem;
    border-bottom: 1px solid var(--border);
  }

  .tree-header h3 {
    margin: 0;
    font-size: 0.875rem;
    color: var(--text-primary);
  }

  .icon-btn {
    width: 24px;
    height: 24px;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    background-color: var(--accent);
    color: white;
    border: none;
    border-radius: 4px;
    cursor: pointer;
    font-size: 1rem;
    font-weight: bold;
  }

  .icon-btn:hover {
    background-color: var(--accent-hover);
  }

  .error {
    padding: 0.5rem 0.75rem;
    color: var(--error);
    font-size: 0.75rem;
  }

  .language-selector {
    padding: 0.5rem 0.75rem;
    border-bottom: 1px solid var(--border);
  }

  .language-selector select {
    width: 100%;
    padding: 0.375rem 0.5rem;
    font-size: 0.875rem;
    background-color: var(--bg-primary);
    color: var(--text-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
  }

  .file-list {
    flex: 1;
    overflow-y: auto;
    padding: 0.5rem 0;
  }

  .loading {
    padding: 1rem;
    text-align: center;
    color: var(--text-secondary);
  }

  .file-item {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    padding: 0.375rem 0.75rem;
    cursor: pointer;
    font-size: 0.8125rem;
    color: var(--text-primary);
  }

  .file-item:hover {
    background-color: var(--bg-hover);
  }

  .file-item.selected {
    background-color: var(--accent);
    color: white;
  }

  .file-item.directory {
    font-weight: 500;
  }

  .icon {
    width: 16px;
    height: 16px;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    font-size: 0.75rem;
  }

  .icon.folder::before {
    content: ">";
    color: var(--text-secondary);
  }

  .icon.folder-open::before {
    content: "v";
    color: var(--accent);
  }

  .icon.code::before {
    content: "#";
    color: var(--accent);
  }

  .icon.template::before {
    content: "%";
    color: var(--success, #4caf50);
  }

  .icon.settings::before {
    content: "*";
    color: var(--warning);
  }

  .icon.file::before {
    content: "-";
    color: var(--text-secondary);
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
