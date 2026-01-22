<script lang="ts">
  const languages = [
    { id: "csharp", name: "C#" },
    { id: "cpp", name: "C++" },
    { id: "rust", name: "Rust" },
    { id: "typescript", name: "TypeScript" },
    { id: "sqlite", name: "SQLite" },
  ];

  interface Props {
    selected: string[];
  }

  let { selected = $bindable() }: Props = $props();

  function toggle(langId: string) {
    if (selected.includes(langId)) {
      selected = selected.filter((l) => l !== langId);
    } else {
      selected = [...selected, langId];
    }
  }
</script>

<div class="language-selector">
  <label>Target Languages</label>
  <div class="languages">
    {#each languages as lang}
      <label class="checkbox-label">
        <input
          type="checkbox"
          checked={selected.includes(lang.id)}
          onchange={() => toggle(lang.id)}
        />
        <span>{lang.name}</span>
      </label>
    {/each}
  </div>
</div>

<style>
  .language-selector {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .languages {
    display: flex;
    flex-wrap: wrap;
    gap: 0.75rem;
  }

  .checkbox-label {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    cursor: pointer;
    color: var(--text-primary);
  }

  .checkbox-label input {
    width: 1rem;
    height: 1rem;
    cursor: pointer;
  }

  .checkbox-label span {
    font-size: 0.875rem;
  }
</style>
