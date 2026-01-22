<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import * as monaco from "monaco-editor";
  import {
    registerPolyLanguage,
    POLY_LANGUAGE_ID,
  } from "../monaco/poly-language";

  import editorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";

  interface Props {
    value?: string;
    onChange?: (value: string) => void;
    readonly?: boolean;
  }

  let { value = $bindable(""), onChange, readonly = false }: Props = $props();

  let editorContainer: HTMLDivElement;
  let editor: monaco.editor.IStandaloneCodeEditor | undefined;
  let isUpdating = false;

  onMount(() => {
    // Set up Monaco environment
    self.MonacoEnvironment = {
      getWorker: function () {
        return new editorWorker();
      },
    };

    // Register .poly language
    registerPolyLanguage();

    // Create editor
    editor = monaco.editor.create(editorContainer, {
      value: value,
      language: POLY_LANGUAGE_ID,
      theme: "poly-dark",
      automaticLayout: true,
      minimap: { enabled: false },
      fontSize: 14,
      lineNumbers: "on",
      scrollBeyondLastLine: false,
      wordWrap: "on",
      tabSize: 4,
      insertSpaces: true,
      readOnly: readonly,
      renderWhitespace: "selection",
      bracketPairColorization: { enabled: true },
    });

    // Listen for changes
    editor.onDidChangeModelContent(() => {
      if (!isUpdating) {
        const newValue = editor?.getValue() ?? "";
        value = newValue;
        onChange?.(newValue);
      }
    });
  });

  onDestroy(() => {
    editor?.dispose();
  });

  // Update editor when value prop changes externally
  $effect(() => {
    if (editor && value !== editor.getValue()) {
      isUpdating = true;
      editor.setValue(value);
      isUpdating = false;
    }
  });

  export function focus() {
    editor?.focus();
  }

  export function getEditor() {
    return editor;
  }
</script>

<div class="editor-container" bind:this={editorContainer}></div>

<style>
  .editor-container {
    width: 100%;
    height: 100%;
    min-height: 200px;
    border-radius: 4px;
    overflow: hidden;
  }
</style>
