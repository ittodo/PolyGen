<script lang="ts">
  import { createEventDispatcher, onDestroy, onMount } from "svelte";
  import { findNodeAtLocation, getLocation, parseTree } from "jsonc-parser";
  import * as monaco from "monaco-editor";
  import { shouldCommitAtCursor } from "./jsonCommit";

  export let value = "";
  export let readonly = false;
  export let formulaHints: Array<{
    path: Array<string | number>;
    formula: string;
  }> = [];

  type JsonCellSelection = {
    rowIndex?: number;
    field: string;
  };

  const dispatch = createEventDispatcher<{
    change: string;
    commit: string;
    select: JsonCellSelection;
  }>();
  let host: HTMLDivElement;
  let editor: monaco.editor.IStandaloneCodeEditor | undefined;
  let updating = false;
  let dirty = false;
  let editedPath = "";
  let formulaDecorations:
    | monaco.editor.IEditorDecorationsCollection
    | undefined;

  export function getValue() {
    return editor?.getValue() ?? value;
  }

  onMount(() => {
    editor = monaco.editor.create(host, {
      value,
      language: "json",
      theme: "vs-dark",
      automaticLayout: true,
      minimap: { enabled: false },
      fontSize: 13,
      readOnly: readonly,
      formatOnPaste: true,
      scrollBeyondLastLine: false,
    });
    formulaDecorations = editor.createDecorationsCollection();
    updateFormulaDecorations();
    editor.onDidChangeModelContent(() => {
      if (updating || !editor) return;
      const text = editor.getValue();
      updateFormulaDecorations();
      dirty = true;
      editedPath = pathAtCaret(editor);
      dispatch("change", text);
      queueMicrotask(() => commitIfValueExited(false));
    });
    editor.onDidChangeCursorPosition(() => {
      commitIfValueExited(true);
    });
    editor.onDidBlurEditorText(() => {
      commitIfDirty();
    });
    editor.onMouseDown((event) => {
      const model = editor?.getModel();
      const position = event.target.position;
      if (!model || !position) return;
      const path = getLocation(model.getValue(), model.getOffsetAt(position)).path;
      const rowIndex = typeof path[0] === "number" ? path[0] : undefined;
      const field = rowIndex === undefined ? path[0] : path[1];
      if (typeof rowIndex === "number" && typeof field === "string") {
        dispatch("select", { rowIndex, field });
      } else if (rowIndex === undefined && typeof field === "string") {
        dispatch("select", { field });
      }
    });
  });

  $: if (editor && editor.getValue() !== value) {
    updating = true;
    editor.setValue(value);
    updating = false;
    dirty = false;
    editedPath = "";
  }

  $: editor?.updateOptions({ readOnly: readonly });
  $: if (editor && formulaHints) updateFormulaDecorations();

  onDestroy(() => {
    formulaDecorations?.clear();
    editor?.dispose();
  });

  function updateFormulaDecorations() {
    const model = editor?.getModel();
    if (!model || !formulaDecorations) return;
    const root = parseTree(model.getValue());
    if (!root) {
      formulaDecorations.clear();
      return;
    }
    formulaDecorations.set(
      formulaHints.flatMap((hint) => {
        const node = findNodeAtLocation(root, hint.path);
        if (!node) return [];
        const position = model.getPositionAt(node.offset + node.length);
        return [
          {
            range: new monaco.Range(
              position.lineNumber,
              position.column,
              position.lineNumber,
              position.column,
            ),
            options: {
              after: {
                content: `  ƒ ${hint.formula}`,
                inlineClassName: "json-formula-hint",
              },
              hoverMessage: {
                value: `계산식: \`${hint.formula}\``,
              },
            },
          },
        ];
      }),
    );
  }

  function commitIfValueExited(checkPathChange: boolean) {
    if (!dirty || !editor) return;
    const model = editor.getModel();
    const position = editor.getPosition();
    if (!model || !position) return;
    const offset = model.getOffsetAt(position);
    const currentPath = pathAtCaret(editor);
    if (
      shouldCommitAtCursor(model.getValue(), offset) ||
      (checkPathChange && editedPath.length > 0 && currentPath !== editedPath)
    ) {
      commitIfDirty();
    }
  }

  function commitIfDirty() {
    if (!dirty || !editor) return;
    dirty = false;
    editedPath = "";
    dispatch("commit", editor.getValue());
  }

  function pathAtCaret(target: monaco.editor.IStandaloneCodeEditor) {
    const model = target.getModel();
    const position = target.getPosition();
    if (!model || !position) return "";
    return JSON.stringify(
      getLocation(model.getValue(), model.getOffsetAt(position)).path,
    );
  }
</script>

<div class="editor" bind:this={host}></div>

<style>
  .editor {
    width: 100%;
    height: 100%;
    min-height: 260px;
  }

  :global(.json-formula-hint) {
    margin-left: 8px;
    color: #6fd6b4;
    font-style: italic;
    opacity: 0.95;
  }
</style>
