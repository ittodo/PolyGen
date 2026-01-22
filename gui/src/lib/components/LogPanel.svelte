<script lang="ts">
  import { tick } from "svelte";

  interface Props {
    logs: string[];
  }

  let { logs }: Props = $props();
  let logContainer: HTMLDivElement;

  $effect(() => {
    if (logs.length > 0) {
      tick().then(() => {
        logContainer?.scrollTo({ top: logContainer.scrollHeight });
      });
    }
  });
</script>

<div class="log-panel" bind:this={logContainer}>
  {#if logs.length === 0}
    <p class="empty">No output yet.</p>
  {:else}
    {#each logs as log}
      <p class="log-line" class:error={log.includes("ERROR")} class:success={log.includes("SUCCESS")}>
        {log}
      </p>
    {/each}
  {/if}
</div>

<style>
  .log-panel {
    flex: 1;
    background-color: var(--bg-primary);
    border-radius: 4px;
    padding: 0.75rem;
    overflow-y: auto;
    font-family: "Consolas", "Monaco", monospace;
    font-size: 0.75rem;
    line-height: 1.5;
  }

  .empty {
    color: var(--text-secondary);
    font-style: italic;
  }

  .log-line {
    margin: 0;
    padding: 0.125rem 0;
    word-break: break-all;
  }

  .log-line.error {
    color: var(--error);
  }

  .log-line.success {
    color: var(--success);
  }
</style>
