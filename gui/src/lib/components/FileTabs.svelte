<script lang="ts">
  interface FileTab {
    path: string;
    name: string;
    isMain: boolean;
    isModified: boolean;
  }

  interface Props {
    tabs: FileTab[];
    activeTab: string;
    onSelectTab: (path: string) => void;
    onCloseTab: (path: string) => void;
  }

  let { tabs, activeTab, onSelectTab, onCloseTab }: Props = $props();
</script>

<div class="tabs-container">
  {#each tabs as tab}
    <div
      class="tab"
      class:active={tab.path === activeTab}
      class:main={tab.isMain}
      onclick={() => onSelectTab(tab.path)}
      onkeydown={(e) => e.key === "Enter" && onSelectTab(tab.path)}
      title={tab.path}
      role="tab"
      tabindex="0"
    >
      <span class="tab-name">
        {tab.isMain ? "● " : ""}{tab.name}{tab.isModified ? " *" : ""}
      </span>
      {#if !tab.isMain}
        <span
          class="close-btn"
          onclick={(e) => {
            e.stopPropagation();
            onCloseTab(tab.path);
          }}
          onkeydown={(e) => {
            if (e.key === "Enter") {
              e.stopPropagation();
              onCloseTab(tab.path);
            }
          }}
          title="Close"
          role="button"
          tabindex="0"
        >
          ×
        </span>
      {/if}
    </div>
  {/each}
</div>

<style>
  .tabs-container {
    display: flex;
    gap: 2px;
    background-color: var(--bg-primary);
    padding: 4px 4px 0;
    overflow-x: auto;
    flex-shrink: 0;
  }

  .tab {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 12px;
    background-color: var(--bg-secondary);
    border: none;
    border-radius: 4px 4px 0 0;
    color: var(--text-secondary);
    cursor: pointer;
    font-size: 0.75rem;
    white-space: nowrap;
    transition: background-color 0.15s;
  }

  .tab:hover {
    background-color: var(--bg-hover);
  }

  .tab.active {
    background-color: var(--bg-tertiary, #2d2d2d);
    color: var(--text-primary);
  }

  .tab.main .tab-name {
    color: var(--accent);
  }

  .tab-name {
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 150px;
  }

  .close-btn {
    color: var(--text-secondary);
    cursor: pointer;
    padding: 0 2px;
    font-size: 1rem;
    line-height: 1;
    border-radius: 2px;
  }

  .close-btn:hover {
    background-color: var(--bg-hover);
    color: var(--text-primary);
  }
</style>
