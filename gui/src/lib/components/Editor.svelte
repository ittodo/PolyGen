<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import * as monaco from "monaco-editor";
  import { invoke } from "@tauri-apps/api/core";
  import {
    registerPolyLanguage,
    POLY_LANGUAGE_ID,
  } from "../monaco/poly-language";

  import editorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";

  interface SchemaError {
    start_line: number;
    start_column: number;
    end_line: number;
    end_column: number;
    message: string;
    severity: string;
  }

  interface Props {
    value?: string;
    onChange?: (value: string) => void;
    readonly?: boolean;
  }

  let { value = $bindable(""), onChange, readonly = false }: Props = $props();

  let editorContainer: HTMLDivElement;
  let editor: monaco.editor.IStandaloneCodeEditor | undefined;
  let isUpdating = false;
  let validateTimeout: ReturnType<typeof setTimeout> | null = null;

  // Debounced validation function
  function scheduleValidation() {
    if (validateTimeout) {
      clearTimeout(validateTimeout);
    }
    validateTimeout = setTimeout(() => {
      validateContent();
    }, 500); // 500ms debounce
  }

  async function validateContent() {
    if (!editor) return;

    const content = editor.getValue();
    if (!content.trim()) {
      // Clear markers for empty content
      const model = editor.getModel();
      if (model) {
        monaco.editor.setModelMarkers(model, "polygen", []);
      }
      return;
    }

    try {
      const errors = await invoke<SchemaError[]>("validate_schema", { content });
      const model = editor.getModel();
      if (!model) return;

      const markers: monaco.editor.IMarkerData[] = errors.map((err) => ({
        startLineNumber: err.start_line,
        startColumn: err.start_column,
        endLineNumber: err.end_line,
        endColumn: err.end_column,
        message: err.message,
        severity:
          err.severity === "error"
            ? monaco.MarkerSeverity.Error
            : err.severity === "warning"
              ? monaco.MarkerSeverity.Warning
              : monaco.MarkerSeverity.Info,
      }));

      monaco.editor.setModelMarkers(model, "polygen", markers);
    } catch (e) {
      console.error("Validation failed:", e);
    }
  }

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
        scheduleValidation();
      }
    });

    // Initial validation
    scheduleValidation();
  });

  onDestroy(() => {
    if (validateTimeout) {
      clearTimeout(validateTimeout);
    }
    editor?.dispose();
  });

  // Update editor when value prop changes externally
  $effect(() => {
    if (editor && value !== editor.getValue()) {
      isUpdating = true;
      editor.setValue(value);
      isUpdating = false;
      scheduleValidation();
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
