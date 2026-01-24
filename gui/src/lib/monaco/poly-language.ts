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
}
