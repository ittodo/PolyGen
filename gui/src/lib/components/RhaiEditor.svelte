<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import * as monaco from "monaco-editor";
  import { invoke } from "@tauri-apps/api/core";
  import { registerRhaiLanguage, RHAI_LANGUAGE_ID } from "../monaco/rhai-language";
  import { registerPtplLanguage, PTPL_LANGUAGE_ID } from "../monaco/ptpl-language";
  import editorWorker from "monaco-editor/esm/vs/editor/editor.worker?worker";

  interface RhaiError {
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
    onInsertText?: (text: string) => void;
    readonly?: boolean;
    filePath?: string;
    initialPosition?: { line: number; column: number } | null;
  }

  let {
    value = $bindable(""),
    onChange,
    onInsertText,
    readonly = false,
    filePath = "",
    initialPosition = null,
  }: Props = $props();

  let editorContainer: HTMLDivElement;
  let editor: monaco.editor.IStandaloneCodeEditor | undefined;
  let isUpdating = false;
  let validateTimeout: ReturnType<typeof setTimeout> | null = null;
  let currentLanguage = $state(RHAI_LANGUAGE_ID);

  // Determine language from file path
  function getLanguageFromPath(path: string): string {
    if (!path) return RHAI_LANGUAGE_ID;
    const lower = path.toLowerCase();
    if (lower.endsWith(".ptpl")) return PTPL_LANGUAGE_ID;
    if (lower.endsWith(".toml")) return "toml";
    if (lower.endsWith(".rhai")) return RHAI_LANGUAGE_ID;
    return RHAI_LANGUAGE_ID;
  }

  // Get theme from language
  function getThemeFromLanguage(lang: string): string {
    switch (lang) {
      case PTPL_LANGUAGE_ID:
        return "ptpl-dark";
      case "toml":
        return "vs-dark";
      default:
        return "rhai-dark";
    }
  }

  function scheduleValidation() {
    // Only validate Rhai files
    if (currentLanguage !== RHAI_LANGUAGE_ID) return;

    if (validateTimeout) {
      clearTimeout(validateTimeout);
    }
    validateTimeout = setTimeout(() => {
      validateContent();
    }, 500);
  }

  async function validateContent() {
    if (!editor || currentLanguage !== RHAI_LANGUAGE_ID) return;

    const content = editor.getValue();
    if (!content.trim()) {
      const model = editor.getModel();
      if (model) {
        monaco.editor.setModelMarkers(model, "rhai", []);
      }
      return;
    }

    try {
      const errors = await invoke<RhaiError[]>("validate_rhai_script", {
        content,
      });

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

      monaco.editor.setModelMarkers(model, "rhai", markers);
    } catch (e) {
      console.error("Rhai validation failed:", e);
    }
  }

  onMount(() => {
    // Set up Monaco environment
    self.MonacoEnvironment = {
      getWorker: function () {
        return new editorWorker();
      },
    };

    // Register languages
    registerRhaiLanguage();
    registerPtplLanguage();

    // Determine initial language
    currentLanguage = getLanguageFromPath(filePath);
    const theme = getThemeFromLanguage(currentLanguage);

    // Create editor
    editor = monaco.editor.create(editorContainer, {
      value: value,
      language: currentLanguage,
      theme: theme,
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

    // Navigate to initial position if provided
    if (initialPosition && editor) {
      editor.setPosition({
        lineNumber: initialPosition.line,
        column: initialPosition.column,
      });
      editor.revealLineInCenter(initialPosition.line);
      editor.focus();
    }

    // Initial validation (only for Rhai)
    if (currentLanguage === RHAI_LANGUAGE_ID) {
      scheduleValidation();
    }
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

  // Update language when filePath changes
  $effect(() => {
    if (editor && filePath) {
      const newLanguage = getLanguageFromPath(filePath);
      if (newLanguage !== currentLanguage) {
        currentLanguage = newLanguage;
        const model = editor.getModel();
        if (model) {
          monaco.editor.setModelLanguage(model, newLanguage);
        }
        const theme = getThemeFromLanguage(newLanguage);
        monaco.editor.setTheme(theme);

        // Clear markers if not Rhai
        if (newLanguage !== RHAI_LANGUAGE_ID && model) {
          monaco.editor.setModelMarkers(model, "rhai", []);
        } else {
          scheduleValidation();
        }
      }
    }
  });

  export function focus() {
    editor?.focus();
  }

  export function getEditor() {
    return editor;
  }

  // Insert text at cursor position
  export function insertAtCursor(text: string) {
    if (!editor) return;

    const selection = editor.getSelection();
    if (selection) {
      editor.executeEdits("insert", [
        {
          range: selection,
          text: text,
          forceMoveMarkers: true,
        },
      ]);
      editor.focus();
    }
  }
</script>

<div class="rhai-editor-container" bind:this={editorContainer}></div>

<style>
  .rhai-editor-container {
    width: 100%;
    height: 100%;
    min-height: 200px;
    border-radius: 4px;
    overflow: hidden;
  }
</style>
