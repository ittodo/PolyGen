<script lang="ts">
  interface ImportedFile {
    path: string;
    name: string;
  }

  interface Props {
    files: ImportedFile[];
    openPaths: string[];
    activePath: string;
    onFileClick: (path: string) => void;
  }

  let { files, openPaths, activePath, onFileClick }: Props = $props();
</script>

<div class="file-list">
  <h3>Imported Files</h3>
  {#if files.length === 0}
    <p class="empty">No imported files</p>
  {:else}
    <ul>
      {#each files as file}
        <li>
          <button
            class="file-entry"
            class:open={openPaths.includes(file.path)}
            class:active={file.path === activePath}
            onclick={() => onFileClick(file.path)}
            title={file.path}
            type="button"
          >
            <span class="icon">{openPaths.includes(file.path) ? "📄" : "📁"}</span>
            <span class="name">{file.name}</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .file-list {
    border-top: 1px solid var(--border);
    padding-top: 0.75rem;
  }

  h3 {
    font-size: 0.75rem;
    color: var(--text-secondary);
    margin: 0 0 0.5rem 0;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .empty {
    font-size: 0.75rem;
    color: var(--text-secondary);
    font-style: italic;
  }

  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .file-entry {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 4px 8px;
    border-radius: 4px;
    border: 0;
    background: none;
    cursor: pointer;
    font-size: 0.75rem;
    color: var(--text-secondary);
    text-align: left;
    transition: background-color 0.15s;
  }

  .file-entry:hover {
    background-color: var(--bg-hover);
  }

  .file-entry.open {
    color: var(--text-primary);
  }

  .file-entry.active {
    background-color: var(--bg-hover);
    color: var(--accent);
  }

  .icon {
    font-size: 0.875rem;
  }

  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
