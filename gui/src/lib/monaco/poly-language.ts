import * as monaco from "monaco-editor";
import { invoke } from "@tauri-apps/api/core";
import { getFilePath } from "./file-path-store";

// Custom event for cross-file navigation
export interface GoToFileEvent {
  filePath: string;
  line: number;
  column: number;
}

// Event emitter for cross-file navigation
let goToFileHandler: ((event: GoToFileEvent) => void) | null = null;

export function setGoToFileHandler(handler: (event: GoToFileEvent) => void) {
  goToFileHandler = handler;
}

export function clearGoToFileHandler() {
  goToFileHandler = null;
}

export const POLY_LANGUAGE_ID = "poly";

interface DefinitionLocation {
  start_line: number;
  start_column: number;
  end_line: number;
  end_column: number;
  name: string;
  kind: string;
  file_path: string | null;
}

interface ReferenceLocation {
  start_line: number;
  start_column: number;
  end_line: number;
  end_column: number;
  is_definition: boolean;
  file_path: string | null;
}

interface HoverInfo {
  content: string;
  start_line: number;
  start_column: number;
  end_line: number;
  end_column: number;
}

interface CompletionItem {
  label: string;
  kind: string;
  detail: string;
  insert_text: string;
  is_snippet: boolean;
}

interface DocumentSymbol {
  name: string;
  kind: string;
  fqn: string;
  start_line: number;
  start_column: number;
  end_line: number;
  end_column: number;
  children: DocumentSymbol[];
}

interface RenameEdit {
  file_path: string;
  start_line: number;
  start_column: number;
  end_line: number;
  end_column: number;
  new_text: string;
}

// Cache for file content models
const fileModelCache = new Map<string, monaco.editor.ITextModel>();

async function getOrCreateModel(filePath: string): Promise<monaco.editor.ITextModel | null> {
  const uri = monaco.Uri.file(filePath);
  const uriString = uri.toString();

  // Check if model already exists
  const existing = monaco.editor.getModel(uri);
  if (existing) {
    return existing;
  }

  // Check cache
  if (fileModelCache.has(uriString)) {
    return fileModelCache.get(uriString) || null;
  }

  // Fetch content from Tauri
  try {
    const content = await invoke<string>("read_file", { path: filePath });
    const model = monaco.editor.createModel(content, POLY_LANGUAGE_ID, uri);
    fileModelCache.set(uriString, model);
    return model;
  } catch (e) {
    console.error("Failed to load file for preview:", filePath, e);
    return null;
  }
}

export const polyLanguageConfig: monaco.languages.LanguageConfiguration = {
  comments: {
    lineComment: "//",
  },
  brackets: [
    ["{", "}"],
    ["[", "]"],
    ["(", ")"],
  ],
  autoClosingPairs: [
    { open: "{", close: "}" },
    { open: "[", close: "]" },
    { open: "(", close: ")" },
    { open: '"', close: '"' },
  ],
  surroundingPairs: [
    { open: "{", close: "}" },
    { open: "[", close: "]" },
    { open: "(", close: ")" },
    { open: '"', close: '"' },
  ],
};

export const polyTokensProvider: monaco.languages.IMonarchLanguage = {
  keywords: [
    "namespace",
    "table",
    "enum",
    "embed",
    "import",
  ],

  typeKeywords: [
    "string",
    "bool",
    "bytes",
    "u8",
    "u16",
    "u32",
    "u64",
    "i8",
    "i16",
    "i32",
    "i64",
    "f32",
    "f64",
  ],

  constraints: [
    "primary_key",
    "unique",
    "max_length",
    "default",
    "range",
    "regex",
    "foreign_key",
  ],

  annotations: [
    "load",
    "taggable",
    "link_rows",
    "cache",
    "readonly",
    "soft_delete",
    "datasource",
  ],

  operators: ["=", "?", ":"],

  symbols: /[=><!~?:&|+\-*\/\^%]+/,

  tokenizer: {
    root: [
      // Annotations
      [/@[a-zA-Z_]\w*/, "annotation"],

      // Comments
      [/\/\/.*$/, "comment"],
      [/\/\/\/.*$/, "comment.doc"],

      // Strings
      [/"([^"\\]|\\.)*$/, "string.invalid"],
      [/"/, { token: "string.quote", bracket: "@open", next: "@string" }],

      // Numbers
      [/\d+/, "number"],

      // Identifiers and keywords
      [
        /[a-zA-Z_]\w*/,
        {
          cases: {
            "@keywords": "keyword",
            "@typeKeywords": "type",
            "@constraints": "keyword.constraint",
            "@default": "identifier",
          },
        },
      ],

      // Delimiters
      [/[{}()\[\]]/, "@brackets"],
      [/[;,.]/, "delimiter"],

      // Operators
      [/@symbols/, "operator"],

      // Whitespace
      [/\s+/, "white"],
    ],

    string: [
      [/[^\\"]+/, "string"],
      [/\\./, "string.escape"],
      [/"/, { token: "string.quote", bracket: "@close", next: "@pop" }],
    ],
  },
};

export const polyTheme: monaco.editor.IStandaloneThemeData = {
  base: "vs-dark",
  inherit: true,
  rules: [
    { token: "keyword", foreground: "569CD6", fontStyle: "bold" },
    { token: "type", foreground: "4EC9B0" },
    { token: "keyword.constraint", foreground: "DCDCAA" },
    { token: "annotation", foreground: "C586C0" },
    { token: "comment", foreground: "6A9955" },
    { token: "comment.doc", foreground: "6A9955", fontStyle: "italic" },
    { token: "string", foreground: "CE9178" },
    { token: "number", foreground: "B5CEA8" },
    { token: "identifier", foreground: "9CDCFE" },
    { token: "operator", foreground: "D4D4D4" },
    { token: "delimiter", foreground: "D4D4D4" },
  ],
  colors: {
    "editor.background": "#1E1E1E",
    "editor.foreground": "#D4D4D4",
    "editorLineNumber.foreground": "#858585",
    "editorCursor.foreground": "#AEAFAD",
    "editor.selectionBackground": "#264F78",
  },
};

export function registerPolyLanguage() {
  monaco.languages.register({ id: POLY_LANGUAGE_ID });
  monaco.languages.setLanguageConfiguration(POLY_LANGUAGE_ID, polyLanguageConfig);
  monaco.languages.setMonarchTokensProvider(POLY_LANGUAGE_ID, polyTokensProvider);
  monaco.editor.defineTheme("poly-dark", polyTheme);

  // Register Go to Definition provider
  monaco.languages.registerDefinitionProvider(POLY_LANGUAGE_ID, {
    provideDefinition: async (
      model: monaco.editor.ITextModel,
      position: monaco.Position,
      _token: monaco.CancellationToken
    ): Promise<monaco.languages.Definition | null> => {
      const content = model.getValue();
      // Monaco uses 1-indexed lines and columns
      const line = position.lineNumber;
      const column = position.column;

      try {
        const filePath = getFilePath(model.uri.toString());
        const result = await invoke<DefinitionLocation | null>("goto_definition", {
          content,
          line,
          column,
          filePath,
        });

        if (result) {
          const currentFilePath = getFilePath(model.uri.toString());

          // Check if definition is in a different file
          if (result.file_path && result.file_path !== currentFilePath) {
            // Trigger cross-file navigation
            if (goToFileHandler) {
              goToFileHandler({
                filePath: result.file_path,
                line: result.start_line,
                column: result.start_column,
              });
            }
            // Return null to prevent Monaco from navigating within current file
            return null;
          }

          return {
            uri: model.uri,
            range: new monaco.Range(
              result.start_line,
              result.start_column,
              result.end_line,
              result.end_column
            ),
          };
        }
      } catch (e) {
        console.error("Go to definition failed:", e);
      }

      return null;
    },
  });

  // Register Find All References provider
  monaco.languages.registerReferenceProvider(POLY_LANGUAGE_ID, {
    provideReferences: async (
      model: monaco.editor.ITextModel,
      position: monaco.Position,
      context: monaco.languages.ReferenceContext,
      _token: monaco.CancellationToken
    ): Promise<monaco.languages.Location[] | null> => {
      const content = model.getValue();
      // Monaco uses 1-indexed lines and columns
      const line = position.lineNumber;
      const column = position.column;

      try {
        const filePath = getFilePath(model.uri.toString());
        const results = await invoke<ReferenceLocation[]>("find_references", {
          content,
          line,
          column,
          includeDefinition: context.includeDeclaration,
          filePath,
        });

        if (results && results.length > 0) {
          const currentFilePath = getFilePath(model.uri.toString());

          // Pre-load models for files that have references (for peek preview)
          const otherFiles = new Set<string>();
          for (const ref of results) {
            if (ref.file_path && ref.file_path !== currentFilePath) {
              otherFiles.add(ref.file_path);
            }
          }

          // Create models for other files (don't await all, just start loading)
          await Promise.all(
            Array.from(otherFiles).map((fp) => getOrCreateModel(fp))
          );

          return results.map((ref) => {
            // Determine the URI for this reference
            let uri: monaco.Uri;
            if (ref.file_path && ref.file_path !== currentFilePath) {
              // Reference is in a different file - create a file URI
              uri = monaco.Uri.file(ref.file_path);
            } else {
              // Reference is in the current file
              uri = model.uri;
            }

            return {
              uri,
              range: new monaco.Range(
                ref.start_line,
                ref.start_column,
                ref.end_line,
                ref.end_column
              ),
            };
          });
        }
      } catch (e) {
        console.error("Find all references failed:", e);
      }

      return null;
    },
  });

  // Register Hover provider
  monaco.languages.registerHoverProvider(POLY_LANGUAGE_ID, {
    provideHover: async (
      model: monaco.editor.ITextModel,
      position: monaco.Position,
      _token: monaco.CancellationToken
    ): Promise<monaco.languages.Hover | null> => {
      const content = model.getValue();
      const line = position.lineNumber;
      const column = position.column;

      try {
        const filePath = getFilePath(model.uri.toString());
        const result = await invoke<HoverInfo | null>("get_hover_info", {
          content,
          line,
          column,
          filePath,
        });

        if (result) {
          return {
            contents: [{ value: result.content }],
            range: new monaco.Range(
              result.start_line,
              result.start_column,
              result.end_line,
              result.end_column
            ),
          };
        }
      } catch (e) {
        console.error("Hover failed:", e);
      }

      return null;
    },
  });

  // Register Completion provider
  monaco.languages.registerCompletionItemProvider(POLY_LANGUAGE_ID, {
    triggerCharacters: [".", ":", "@"],
    provideCompletionItems: async (
      model: monaco.editor.ITextModel,
      position: monaco.Position,
      _context: monaco.languages.CompletionContext,
      _token: monaco.CancellationToken
    ): Promise<monaco.languages.CompletionList | null> => {
      const content = model.getValue();
      const line = position.lineNumber;
      const column = position.column;

      try {
        const filePath = getFilePath(model.uri.toString());
        const items = await invoke<CompletionItem[]>("get_completions", {
          content,
          line,
          column,
          filePath,
        });

        if (items && items.length > 0) {
          const completionItems: monaco.languages.CompletionItem[] = items.map((item) => {
            let kind: monaco.languages.CompletionItemKind;
            switch (item.kind) {
              case "keyword":
                kind = monaco.languages.CompletionItemKind.Keyword;
                break;
              case "type":
                kind = monaco.languages.CompletionItemKind.TypeParameter;
                break;
              case "class":
                kind = monaco.languages.CompletionItemKind.Class;
                break;
              case "enum":
                kind = monaco.languages.CompletionItemKind.Enum;
                break;
              case "struct":
                kind = monaco.languages.CompletionItemKind.Struct;
                break;
              case "property":
                kind = monaco.languages.CompletionItemKind.Property;
                break;
              case "snippet":
                kind = monaco.languages.CompletionItemKind.Snippet;
                break;
              default:
                kind = monaco.languages.CompletionItemKind.Text;
            }

            return {
              label: item.label,
              kind,
              detail: item.detail,
              insertText: item.insert_text,
              insertTextRules: item.is_snippet
                ? monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet
                : undefined,
              range: undefined as unknown as monaco.IRange,
            };
          });

          return { suggestions: completionItems };
        }
      } catch (e) {
        console.error("Completion failed:", e);
      }

      return null;
    },
  });

  // Register Document Symbol provider (for Outline view)
  monaco.languages.registerDocumentSymbolProvider(POLY_LANGUAGE_ID, {
    provideDocumentSymbols: async (
      model: monaco.editor.ITextModel,
      _token: monaco.CancellationToken
    ): Promise<monaco.languages.DocumentSymbol[] | null> => {
      const content = model.getValue();

      try {
        const filePath = getFilePath(model.uri.toString());
        const symbols = await invoke<DocumentSymbol[]>("get_document_symbols", {
          content,
          filePath,
        });

        if (symbols && symbols.length > 0) {
          const mapSymbol = (sym: DocumentSymbol): monaco.languages.DocumentSymbol => {
            let kind: monaco.languages.SymbolKind;
            switch (sym.kind) {
              case "namespace":
                kind = monaco.languages.SymbolKind.Namespace;
                break;
              case "table":
                kind = monaco.languages.SymbolKind.Class;
                break;
              case "enum":
                kind = monaco.languages.SymbolKind.Enum;
                break;
              case "embed":
                kind = monaco.languages.SymbolKind.Struct;
                break;
              case "field":
                kind = monaco.languages.SymbolKind.Field;
                break;
              default:
                kind = monaco.languages.SymbolKind.Variable;
            }

            const range = new monaco.Range(
              sym.start_line,
              sym.start_column,
              sym.end_line,
              sym.end_column
            );

            return {
              name: sym.name,
              detail: sym.fqn,
              kind,
              range,
              selectionRange: range,
              children: sym.children ? sym.children.map(mapSymbol) : [],
              tags: [],
            };
          };

          return symbols.map(mapSymbol);
        }
      } catch (e) {
        console.error("Document symbols failed:", e);
      }

      return null;
    },
  });

  // Register Rename provider
  monaco.languages.registerRenameProvider(POLY_LANGUAGE_ID, {
    provideRenameEdits: async (
      model: monaco.editor.ITextModel,
      position: monaco.Position,
      newName: string,
      _token: monaco.CancellationToken
    ): Promise<monaco.languages.WorkspaceEdit | null> => {
      const content = model.getValue();
      const line = position.lineNumber;
      const column = position.column;

      try {
        const filePath = getFilePath(model.uri.toString());
        const edits = await invoke<RenameEdit[]>("rename_symbol", {
          content,
          line,
          column,
          newName,
          filePath,
        });

        if (edits && edits.length > 0) {
          const currentFilePath = getFilePath(model.uri.toString());

          // Pre-load models for files that have edits
          const otherFiles = new Set<string>();
          for (const edit of edits) {
            if (edit.file_path && edit.file_path !== currentFilePath) {
              otherFiles.add(edit.file_path);
            }
          }
          await Promise.all(
            Array.from(otherFiles).map((fp) => getOrCreateModel(fp))
          );

          const workspaceEdit: monaco.languages.WorkspaceEdit = {
            edits: edits.map((edit) => {
              let uri: monaco.Uri;
              if (edit.file_path && edit.file_path !== currentFilePath) {
                uri = monaco.Uri.file(edit.file_path);
              } else {
                uri = model.uri;
              }

              return {
                resource: uri,
                textEdit: {
                  range: new monaco.Range(
                    edit.start_line,
                    edit.start_column,
                    edit.end_line,
                    edit.end_column
                  ),
                  text: edit.new_text,
                },
                versionId: undefined,
              };
            }),
          };

          return workspaceEdit;
        }
      } catch (e) {
        console.error("Rename failed:", e);
      }

      return null;
    },

    resolveRenameLocation: async (
      model: monaco.editor.ITextModel,
      position: monaco.Position,
      _token: monaco.CancellationToken
    ): Promise<monaco.languages.RenameLocation | null> => {
      const content = model.getValue();
      const line = position.lineNumber;
      const column = position.column;

      try {
        const filePath = getFilePath(model.uri.toString());
        const result = await invoke<{ text: string; start_line: number; start_column: number; end_line: number; end_column: number } | null>(
          "prepare_rename",
          { content, line, column, filePath }
        );

        if (result) {
          return {
            text: result.text,
            range: new monaco.Range(
              result.start_line,
              result.start_column,
              result.end_line,
              result.end_column
            ),
          };
        }
      } catch (e) {
        console.error("Prepare rename failed:", e);
      }

      return null;
    },
  });
}
