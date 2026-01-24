<script lang="ts">
  interface RecentProject {
    path: string;
    name: string;
    timestamp: number;
    output_dir?: string;
    languages?: string[];
  }

  interface Props {
    label: string;
    value: string;
    placeholder: string;
    onSelect: () => void;
    recentProjects?: RecentProject[];
    onRecentSelect?: (project: RecentProject) => void;
  }

  let { label, value, placeholder, onSelect, recentProjects = [], onRecentSelect }: Props = $props();

  let showDropdown = $state(false);
  let dropdownRef = $state<HTMLDivElement | null>(null);

  function toggleDropdown() {
    if (recentProjects.length > 0) {
      showDropdown = !showDropdown;
    }
  }

  function selectRecent(project: RecentProject) {
    showDropdown = false;
    if (onRecentSelect) {
      onRecentSelect(project);
    }
  }

  function formatTime(timestamp: number): string {
    const now = Date.now();
    const diff = now - timestamp;

    const seconds = Math.floor(diff / 1000);
    const minutes = Math.floor(seconds / 60);
    const hours = Math.floor(minutes / 60);
    const days = Math.floor(hours / 24);

    if (days > 0) {
      return days === 1 ? "yesterday" : `${days} days ago`;
    }
    if (hours > 0) {
      return hours === 1 ? "1 hour ago" : `${hours} hours ago`;
    }
    if (minutes > 0) {
      return minutes === 1 ? "1 min ago" : `${minutes} mins ago`;
    }
    return "just now";
  }

  function handleClickOutside(event: MouseEvent) {
    if (dropdownRef && !dropdownRef.contains(event.target as Node)) {
      showDropdown = false;
    }
  }

  $effect(() => {
    if (showDropdown) {
      document.addEventListener("click", handleClickOutside);
      return () => {
        document.removeEventListener("click", handleClickOutside);
      };
    }
  });
</script>

<div class="path-selector">
  <label>{label}</label>
  <div class="input-group" bind:this={dropdownRef}>
    <input type="text" {value} {placeholder} readonly />
    {#if recentProjects.length > 0}
      <button
        class="dropdown-btn"
        class:active={showDropdown}
        onclick={toggleDropdown}
        title="Recent projects"
      >
        <span class="arrow">{showDropdown ? "\u25B2" : "\u25BC"}</span>
      </button>
    {/if}
    <button class="secondary" onclick={onSelect}>Browse</button>

    {#if showDropdown && recentProjects.length > 0}
      <div class="recent-dropdown">
        <div class="dropdown-header">Recent Projects</div>
        {#each recentProjects as project}
          <button class="recent-item" onclick={() => selectRecent(project)}>
            <span class="project-name">{project.name}</span>
            <span class="project-path" title={project.path}>{project.path}</span>
            <span class="project-time">{formatTime(project.timestamp)}</span>
          </button>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .path-selector {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }

  .input-group {
    display: flex;
    gap: 0.5rem;
    position: relative;
  }

  .input-group input {
    flex: 1;
    min-width: 0;
  }

  .input-group button {
    white-space: nowrap;
  }

  .dropdown-btn {
    padding: 0.375rem 0.5rem;
    background-color: var(--bg-primary);
    border: 1px solid var(--border);
    border-radius: 4px;
    color: var(--text-primary);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .dropdown-btn:hover {
    background-color: var(--bg-hover);
  }

  .dropdown-btn.active {
    background-color: var(--accent);
    color: white;
    border-color: var(--accent);
  }

  .arrow {
    font-size: 0.625rem;
  }

  .recent-dropdown {
    position: absolute;
    top: 100%;
    left: 0;
    right: 0;
    margin-top: 0.25rem;
    background-color: var(--bg-secondary);
    border: 1px solid var(--border);
    border-radius: 6px;
    box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
    z-index: 1000;
    max-height: 300px;
    overflow-y: auto;
  }

  .dropdown-header {
    padding: 0.5rem 0.75rem;
    font-size: 0.75rem;
    font-weight: 600;
    color: var(--text-secondary);
    border-bottom: 1px solid var(--border);
    background-color: var(--bg-primary);
  }

  .recent-item {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.125rem;
    width: 100%;
    padding: 0.5rem 0.75rem;
    background: none;
    border: none;
    border-bottom: 1px solid var(--border);
    cursor: pointer;
    text-align: left;
    color: var(--text-primary);
  }

  .recent-item:last-child {
    border-bottom: none;
  }

  .recent-item:hover {
    background-color: var(--bg-hover);
  }

  .project-name {
    font-weight: 500;
    font-size: 0.875rem;
    color: var(--accent);
  }

  .project-path {
    font-size: 0.75rem;
    color: var(--text-secondary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    width: 100%;
  }

  .project-time {
    font-size: 0.625rem;
    color: var(--text-secondary);
  }
</style>
