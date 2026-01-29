import * as monaco from "monaco-editor";

export const RHAI_LANGUAGE_ID = "rhai";

export const rhaiLanguageConfig: monaco.languages.LanguageConfiguration = {
  comments: {
    lineComment: "//",
    blockComment: ["/*", "*/"],
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
    { open: "'", close: "'" },
    { open: "`", close: "`" },
  ],
  surroundingPairs: [
    { open: "{", close: "}" },
    { open: "[", close: "]" },
    { open: "(", close: ")" },
    { open: '"', close: '"' },
    { open: "'", close: "'" },
    { open: "`", close: "`" },
  ],
  indentationRules: {
    increaseIndentPattern: /^\s*(fn|if|else|while|for|loop)\b.*\{[^}]*$/,
    decreaseIndentPattern: /^\s*\}/,
  },
};

export const rhaiTokensProvider: monaco.languages.IMonarchLanguage = {
  keywords: [
    "let",
    "const",
    "fn",
    "if",
    "else",
    "while",
    "for",
    "in",
    "loop",
    "break",
    "continue",
    "return",
    "throw",
    "try",
    "catch",
    "import",
    "export",
    "as",
    "private",
    "this",
    "switch",
    "type_of",
    "print",
    "debug",
  ],

  builtinTypes: ["true", "false", "null"],

  // PolyGen-specific functions
  polygenFunctions: [
    // Case conversion
    "to_pascal_case",
    "to_snake_case",
    "to_camel_case",
    "to_screaming_snake_case",
    "to_kebab_case",
    // String utilities
    "indent",
    "indent_lines",
    "trim",
    "split",
    "join",
    "contains",
    "starts_with",
    "ends_with",
    "replace",
    "to_upper",
    "to_lower",
    // Array utilities
    "len",
    "push",
    "pop",
    "shift",
    "map",
    "filter",
    "reduce",
    "find",
    "sort",
    "reverse",
    "is_empty",
    // Type mapping
    "map_type",
    "get_default_value",
    "is_primitive",
    "is_nullable",
    // Generation helpers
    "emit",
    "emit_line",
    "include",
  ],

  // PolyGen IR properties
  polygenProperties: [
    // StructDef properties
    "name",
    "fqn",
    "fields",
    "doc_comment",
    "annotations",
    "is_table",
    "is_embed",
    "primary_key_field",
    "namespace",
    // FieldDef properties
    "field_name",
    "field_type",
    "is_optional",
    "is_array",
    "constraints",
    "default_value",
    "is_primary_key",
    "is_foreign_key",
    // EnumDef properties
    "variants",
    "variant_name",
    "variant_value",
    // Namespace properties
    "structs",
    "enums",
    "children",
  ],

  operators: [
    "=",
    ">",
    "<",
    "!",
    "~",
    "?",
    ":",
    "==",
    "<=",
    ">=",
    "!=",
    "&&",
    "||",
    "++",
    "--",
    "+",
    "-",
    "*",
    "/",
    "&",
    "|",
    "^",
    "%",
    "<<",
    ">>",
    "+=",
    "-=",
    "*=",
    "/=",
    "&=",
    "|=",
    "^=",
    "%=",
    "<<=",
    ">>=",
    "=>",
    "??",
    "?.",
  ],

  symbols: /[=><!~?:&|+\-*\/\^%]+/,
  escapes: /\\(?:[abfnrtv\\"']|x[0-9A-Fa-f]{1,4}|u[0-9A-Fa-f]{4}|U[0-9A-Fa-f]{8})/,

  tokenizer: {
    root: [
      // Comments
      [/\/\/.*$/, "comment"],
      [/\/\*/, "comment", "@comment"],

      // Strings
      [/"([^"\\]|\\.)*$/, "string.invalid"],
      [/'([^'\\]|\\.)*$/, "string.invalid"],
      [/`([^`\\]|\\.)*$/, "string.invalid"],
      [/"/, "string", "@string_double"],
      [/'/, "string", "@string_single"],
      [/`/, "string", "@string_backtick"],

      // Numbers
      [/\d*\.\d+([eE][\-+]?\d+)?/, "number.float"],
      [/0[xX][0-9a-fA-F]+/, "number.hex"],
      [/0[oO][0-7]+/, "number.octal"],
      [/0[bB][01]+/, "number.binary"],
      [/\d+/, "number"],

      // Identifiers and keywords
      [
        /[a-zA-Z_]\w*/,
        {
          cases: {
            "@keywords": "keyword",
            "@builtinTypes": "constant.language",
            "@polygenFunctions": "support.function",
            "@polygenProperties": "variable.property",
            "@default": "identifier",
          },
        },
      ],

      // Map literals
      [/#\{/, "delimiter.bracket", "@map"],

      // Delimiters
      [/[{}()\[\]]/, "@brackets"],
      [/[;,.]/, "delimiter"],

      // Operators
      [/@symbols/, "operator"],

      // Whitespace
      [/\s+/, "white"],
    ],

    comment: [
      [/[^\/*]+/, "comment"],
      [/\/\*/, "comment", "@push"],
      [/\*\//, "comment", "@pop"],
      [/[\/*]/, "comment"],
    ],

    string_double: [
      [/[^\\"]+/, "string"],
      [/@escapes/, "string.escape"],
      [/\\./, "string.escape.invalid"],
      [/"/, "string", "@pop"],
    ],

    string_single: [
      [/[^\\']+/, "string"],
      [/@escapes/, "string.escape"],
      [/\\./, "string.escape.invalid"],
      [/'/, "string", "@pop"],
    ],

    string_backtick: [
      [/\$\{/, "delimiter.bracket", "@interpolation"],
      [/[^\\`$]+/, "string"],
      [/@escapes/, "string.escape"],
      [/\\./, "string.escape.invalid"],
      [/`/, "string", "@pop"],
    ],

    interpolation: [
      [/[^}]+/, "identifier"],
      [/\}/, "delimiter.bracket", "@pop"],
    ],

    map: [
      [/[^}]+/, "identifier"],
      [/\}/, "delimiter.bracket", "@pop"],
    ],
  },
};

export const rhaiTheme: monaco.editor.IStandaloneThemeData = {
  base: "vs-dark",
  inherit: true,
  rules: [
    { token: "keyword", foreground: "569CD6", fontStyle: "bold" },
    { token: "constant.language", foreground: "569CD6" },
    { token: "support.function", foreground: "DCDCAA" },
    { token: "variable.property", foreground: "9CDCFE" },
    { token: "comment", foreground: "6A9955" },
    { token: "string", foreground: "CE9178" },
    { token: "string.escape", foreground: "D7BA7D" },
    { token: "number", foreground: "B5CEA8" },
    { token: "number.float", foreground: "B5CEA8" },
    { token: "number.hex", foreground: "B5CEA8" },
    { token: "identifier", foreground: "D4D4D4" },
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

let isRegistered = false;

export function registerRhaiLanguage() {
  if (isRegistered) return;

  monaco.languages.register({ id: RHAI_LANGUAGE_ID });
  monaco.languages.setLanguageConfiguration(RHAI_LANGUAGE_ID, rhaiLanguageConfig);
  monaco.languages.setMonarchTokensProvider(RHAI_LANGUAGE_ID, rhaiTokensProvider);
  monaco.editor.defineTheme("rhai-dark", rhaiTheme);

  // Register basic completion provider for PolyGen functions
  monaco.languages.registerCompletionItemProvider(RHAI_LANGUAGE_ID, {
    provideCompletionItems: (
      model: monaco.editor.ITextModel,
      position: monaco.Position
    ): monaco.languages.CompletionList => {
      const word = model.getWordUntilPosition(position);
      const range = {
        startLineNumber: position.lineNumber,
        endLineNumber: position.lineNumber,
        startColumn: word.startColumn,
        endColumn: word.endColumn,
      };

      const suggestions: monaco.languages.CompletionItem[] = [
        // Case conversion functions
        {
          label: "to_pascal_case",
          kind: monaco.languages.CompletionItemKind.Function,
          insertText: 'to_pascal_case(${1:text})',
          insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          detail: "Convert to PascalCase",
          documentation: "Converts a string to PascalCase (e.g., 'hello_world' -> 'HelloWorld')",
          range,
        },
        {
          label: "to_snake_case",
          kind: monaco.languages.CompletionItemKind.Function,
          insertText: 'to_snake_case(${1:text})',
          insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          detail: "Convert to snake_case",
          documentation: "Converts a string to snake_case (e.g., 'HelloWorld' -> 'hello_world')",
          range,
        },
        {
          label: "to_camel_case",
          kind: monaco.languages.CompletionItemKind.Function,
          insertText: 'to_camel_case(${1:text})',
          insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          detail: "Convert to camelCase",
          documentation: "Converts a string to camelCase (e.g., 'hello_world' -> 'helloWorld')",
          range,
        },
        {
          label: "to_screaming_snake_case",
          kind: monaco.languages.CompletionItemKind.Function,
          insertText: 'to_screaming_snake_case(${1:text})',
          insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          detail: "Convert to SCREAMING_SNAKE_CASE",
          documentation: "Converts a string to SCREAMING_SNAKE_CASE (e.g., 'helloWorld' -> 'HELLO_WORLD')",
          range,
        },
        // Indent functions
        {
          label: "indent",
          kind: monaco.languages.CompletionItemKind.Function,
          insertText: 'indent(${1:text}, ${2:level})',
          insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          detail: "Add indentation",
          documentation: "Adds indentation to text (level * 4 spaces by default)",
          range,
        },
        {
          label: "indent_lines",
          kind: monaco.languages.CompletionItemKind.Function,
          insertText: 'indent_lines(${1:text}, ${2:level})',
          insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          detail: "Indent each line",
          documentation: "Adds indentation to each line of text",
          range,
        },
        // Control flow snippets
        {
          label: "for-in",
          kind: monaco.languages.CompletionItemKind.Snippet,
          insertText: 'for ${1:item} in ${2:array} {\n\t${3}\n}',
          insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          detail: "For-in loop",
          documentation: "Creates a for-in loop",
          range,
        },
        {
          label: "if-else",
          kind: monaco.languages.CompletionItemKind.Snippet,
          insertText: 'if ${1:condition} {\n\t${2}\n} else {\n\t${3}\n}',
          insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          detail: "If-else statement",
          documentation: "Creates an if-else statement",
          range,
        },
        {
          label: "fn",
          kind: monaco.languages.CompletionItemKind.Snippet,
          insertText: 'fn ${1:name}(${2:params}) {\n\t${3}\n}',
          insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          detail: "Function definition",
          documentation: "Creates a function definition",
          range,
        },
        // PolyGen template snippets
        {
          label: "struct-template",
          kind: monaco.languages.CompletionItemKind.Snippet,
          insertText: [
            'for struct_def in structs {',
            '\tlet name = struct_def.name;',
            '\tlet fields = struct_def.fields;',
            '\t',
            '\t// Generate struct',
            '\t`struct ${name} {`',
            '\tfor field in fields {',
            '\t\t`    ${field.field_name}: ${map_type(field.field_type)},`',
            '\t}',
            '\t`}`',
            '}'
          ].join('\n'),
          insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          detail: "PolyGen struct template",
          documentation: "Template for generating struct definitions",
          range,
        },
        {
          label: "enum-template",
          kind: monaco.languages.CompletionItemKind.Snippet,
          insertText: [
            'for enum_def in enums {',
            '\tlet name = enum_def.name;',
            '\tlet variants = enum_def.variants;',
            '\t',
            '\t`enum ${name} {`',
            '\tfor variant in variants {',
            '\t\t`    ${variant.variant_name} = ${variant.variant_value},`',
            '\t}',
            '\t`}`',
            '}'
          ].join('\n'),
          insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          detail: "PolyGen enum template",
          documentation: "Template for generating enum definitions",
          range,
        },
      ];

      return { suggestions };
    },
  });

  isRegistered = true;
}
