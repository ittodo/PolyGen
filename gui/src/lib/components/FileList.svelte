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
        <li
          class:open={openPaths.includes(file.path)}
          class:active={file.path === activePath}
          onclick={() => onFileClick(file.path)}
          onkeydown={(e) => e.key === "Enter" && onFileClick(file.path)}
          title={file.path}
          role="button"
          tabindex="0"
        >
          <span class="icon">{openPaths.includes(file.path) ? "📄" : "📁"}</span>
          <span class="name">{file.name}</span>
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

  li {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.75rem;
    color: var(--text-secondary);
    transition: background-color 0.15s;
  }

  li:hover {
    background-color: var(--bg-hover);
  }

  li.open {
    color: var(--text-primary);
  }

  li.active {
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
