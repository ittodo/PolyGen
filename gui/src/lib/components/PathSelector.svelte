<script lang="ts">
  interface Props {
    label: string;
    value: string;
    placeholder: string;
    onSelect: () => void;
  }

  let { label, value, placeholder, onSelect }: Props = $props();
</script>

<div class="path-selector">
  <label>{label}</label>
  <div class="input-group">
    <input type="text" {value} {placeholder} readonly />
    <button class="secondary" onclick={onSelect}>Browse</button>
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
  }

  .input-group input {
    flex: 1;
  }

  .input-group button {
    white-space: nowrap;
  }
</style>
