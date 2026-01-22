import * as monaco from "monaco-editor";

export const POLY_LANGUAGE_ID = "poly";

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
}
