import * as monaco from "monaco-editor";

export const PTPL_LANGUAGE_ID = "polytemplate";

export const ptplLanguageConfig: monaco.languages.LanguageConfiguration = {
  comments: {
    lineComment: "%--",
  },
  brackets: [
    ["{", "}"],
    ["[", "]"],
    ["(", ")"],
    ["{{", "}}"],
  ],
  autoClosingPairs: [
    { open: "{", close: "}" },
    { open: "[", close: "]" },
    { open: "(", close: ")" },
    { open: "{{", close: "}}" },
    { open: '"', close: '"' },
    { open: "'", close: "'" },
  ],
  surroundingPairs: [
    { open: "{", close: "}" },
    { open: "[", close: "]" },
    { open: "(", close: ")" },
    { open: "{{", close: "}}" },
    { open: '"', close: '"' },
    { open: "'", close: "'" },
  ],
  folding: {
    markers: {
      start: /^\s*%(?:if|for|match|block|while|logic)\b/,
      end: /^\s*%(?:endif|endfor|endmatch|endblock|endwhile|endlogic)\b/,
    },
  },
  indentationRules: {
    increaseIndentPattern: /^\s*%(?:if|for|else|elif|match|when|block|while|logic)\b.*$/,
    decreaseIndentPattern: /^\s*%(?:endif|endfor|else|elif|endmatch|when|endblock|endwhile|endlogic)\b.*$/,
  },
};

export const ptplTokensProvider: monaco.languages.IMonarchLanguage = {
  defaultToken: "",
  tokenPostfix: ".ptpl",

  // Block directives (with matching end)
  blockDirectives: [
    "if", "for", "match", "block", "while", "logic",
  ],

  // End directives
  endDirectives: [
    "endif", "endfor", "endmatch", "endblock", "endwhile", "endlogic",
  ],

  // Control flow directives
  controlDirectives: [
    "else", "elif", "when",
  ],

  // Simple directives
  simpleDirectives: [
    "include", "let", "set", "render", "blank",
  ],

  // Filters available in interpolation
  filters: [
    "lang_type", "pascal_case", "camel_case", "snake_case", "screaming_snake_case",
    "kebab_case", "upper", "lower", "trim", "count", "join", "first", "last",
    "default", "escape", "raw", "indent",
  ],

  // Rhai keywords (for %logic blocks)
  rhaiKeywords: [
    "let", "const", "fn", "if", "else", "while", "for", "in", "loop",
    "break", "continue", "return", "throw", "try", "catch", "switch",
    "true", "false", "null",
  ],

  // PolyGen functions
  polygenFunctions: [
    "to_pascal_case", "to_snake_case", "to_camel_case", "to_screaming_snake_case",
    "to_kebab_case", "indent", "indent_lines", "trim", "split", "join",
    "contains", "starts_with", "ends_with", "replace", "to_upper", "to_lower",
    "len", "push", "pop", "map", "filter", "find", "sort", "reverse", "is_empty",
    "emit", "emit_line", "include",
  ],

  // IR properties
  irProperties: [
    "name", "fqn", "fields", "doc_comment", "annotations", "is_table", "is_embed",
    "primary_key_field", "namespace", "field_name", "field_type", "is_optional",
    "is_array", "constraints", "default_value", "is_primary_key", "is_foreign_key",
    "variants", "variant_name", "variant_value", "structs", "enums", "children",
    "namespaces", "imports", "path", "type_ref", "base_type", "pack_separator",
    "indexes", "inline_comment", "type_params",
  ],

  operators: [
    "=", ">", "<", "!", "~", "?", ":", "==", "<=", ">=", "!=",
    "&&", "||", "+", "-", "*", "/", "&", "|", "^", "%", "=>",
  ],

  symbols: /[=><!~?:&|+\-*\/\^%]+/,
  escapes: /\\(?:[abfnrtv\\"']|x[0-9A-Fa-f]{1,4}|u[0-9A-Fa-f]{4})/,

  tokenizer: {
    root: [
      // Comments: %-- ...
      [/%--.*$/, "comment"],

      // Block directives: %if, %for, etc.
      [/%(?:if|for|match|block|while|logic)\b/, { token: "keyword.directive.block", next: "@directiveArgs" }],

      // End directives: %endif, %endfor, etc.
      [/%(?:endif|endfor|endmatch|endblock|endwhile|endlogic)\b/, "keyword.directive.end"],

      // Control flow: %else, %elif, %when
      [/%(?:else|elif|when)\b/, { token: "keyword.directive.control", next: "@directiveArgs" }],

      // Simple directives: %include, %let, %set, %render, %blank
      [/%(?:include|let|set|render|blank)\b/, { token: "keyword.directive.simple", next: "@directiveArgs" }],

      // Interpolation start: {{
      [/\{\{/, { token: "delimiter.interpolation", next: "@interpolation" }],

      // Everything else is literal text
      [/[^%{]+/, "string.literal"],
      [/[%{]/, "string.literal"],
    ],

    directiveArgs: [
      // End of line ends directive args
      [/$/, { token: "", next: "@pop" }],

      // Strings
      [/"([^"\\]|\\.)*"/, "string"],
      [/'([^'\\]|\\.)*'/, "string"],

      // Numbers
      [/\d+(\.\d+)?/, "number"],

      // Boolean and null
      [/\b(true|false|null)\b/, "constant.language"],

      // Operators
      [/[=<>!&|]+/, "operator"],

      // Identifiers (variables, properties)
      [/[a-zA-Z_]\w*/, {
        cases: {
          "@rhaiKeywords": "keyword",
          "@polygenFunctions": "support.function",
          "@irProperties": "variable.property",
          "@default": "identifier",
        },
      }],

      // Property access
      [/\./, "delimiter"],

      // Parentheses, brackets
      [/[()[\]]/, "@brackets"],

      // Comma
      [/,/, "delimiter"],

      // Whitespace
      [/\s+/, "white"],
    ],

    interpolation: [
      // End interpolation
      [/\}\}/, { token: "delimiter.interpolation", next: "@pop" }],

      // Filter separator
      [/\|/, "delimiter.filter"],

      // Filters
      [/[a-zA-Z_]\w*/, {
        cases: {
          "@filters": "support.filter",
          "@irProperties": "variable.property",
          "@default": "identifier",
        },
      }],

      // Property access
      [/\./, "delimiter"],

      // Strings (for filter arguments)
      [/"([^"\\]|\\.)*"/, "string"],
      [/'([^'\\]|\\.)*'/, "string"],

      // Numbers
      [/\d+(\.\d+)?/, "number"],

      // Parentheses for filter arguments
      [/[()]/, "@brackets"],

      // Comma
      [/,/, "delimiter"],

      // Whitespace
      [/\s+/, "white"],
    ],

    logicBlock: [
      // End of logic block
      [/%endlogic\b/, { token: "keyword.directive.end", next: "@pop" }],

      // Comments
      [/\/\/.*$/, "comment"],
      [/\/\*/, "comment", "@blockComment"],

      // Strings
      [/"([^"\\]|\\.)*"/, "string"],
      [/'([^'\\]|\\.)*'/, "string"],
      [/`([^`\\]|\\.)*`/, "string.template"],

      // Numbers
      [/\d+(\.\d+)?/, "number"],

      // Keywords
      [/\b(let|const|fn|if|else|while|for|in|loop|break|continue|return|throw|try|catch|switch)\b/, "keyword"],
      [/\b(true|false|null)\b/, "constant.language"],

      // Functions
      [/[a-zA-Z_]\w*(?=\s*\()/, "support.function"],

      // Identifiers
      [/[a-zA-Z_]\w*/, {
        cases: {
          "@polygenFunctions": "support.function",
          "@irProperties": "variable.property",
          "@default": "identifier",
        },
      }],

      // Operators
      [/@symbols/, "operator"],

      // Brackets
      [/[{}()\[\]]/, "@brackets"],

      // Delimiters
      [/[;,.]/, "delimiter"],

      // Whitespace
      [/\s+/, "white"],
    ],

    blockComment: [
      [/[^\/*]+/, "comment"],
      [/\/\*/, "comment", "@push"],
      [/\*\//, "comment", "@pop"],
      [/[\/*]/, "comment"],
    ],
  },
};

export const ptplTheme: monaco.editor.IStandaloneThemeData = {
  base: "vs-dark",
  inherit: true,
  rules: [
    // Directives
    { token: "keyword.directive.block", foreground: "C586C0", fontStyle: "bold" },
    { token: "keyword.directive.end", foreground: "C586C0", fontStyle: "bold" },
    { token: "keyword.directive.control", foreground: "C586C0", fontStyle: "bold" },
    { token: "keyword.directive.simple", foreground: "C586C0" },

    // Interpolation
    { token: "delimiter.interpolation", foreground: "FFCC00", fontStyle: "bold" },
    { token: "delimiter.filter", foreground: "FFCC00" },

    // Filters
    { token: "support.filter", foreground: "4EC9B0" },

    // Variables and properties
    { token: "variable.property", foreground: "9CDCFE" },
    { token: "identifier", foreground: "D4D4D4" },

    // Rhai/Logic keywords
    { token: "keyword", foreground: "569CD6", fontStyle: "bold" },
    { token: "constant.language", foreground: "569CD6" },

    // Functions
    { token: "support.function", foreground: "DCDCAA" },

    // Literals
    { token: "string", foreground: "CE9178" },
    { token: "string.literal", foreground: "D4D4D4" },
    { token: "string.template", foreground: "CE9178" },
    { token: "number", foreground: "B5CEA8" },

    // Comments
    { token: "comment", foreground: "6A9955" },

    // Operators and delimiters
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

export function registerPtplLanguage() {
  if (isRegistered) return;

  monaco.languages.register({ id: PTPL_LANGUAGE_ID });
  monaco.languages.setLanguageConfiguration(PTPL_LANGUAGE_ID, ptplLanguageConfig);
  monaco.languages.setMonarchTokensProvider(PTPL_LANGUAGE_ID, ptplTokensProvider);
  monaco.editor.defineTheme("ptpl-dark", ptplTheme);

  // Register completion provider for PolyTemplate
  monaco.languages.registerCompletionItemProvider(PTPL_LANGUAGE_ID, {
    triggerCharacters: ["%", "{", "|", "."],
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

      const lineContent = model.getLineContent(position.lineNumber);
      const beforeCursor = lineContent.substring(0, position.column - 1);

      const suggestions: monaco.languages.CompletionItem[] = [];

      // Check if we're at the start of a directive (after %)
      if (beforeCursor.endsWith("%")) {
        // Block directives
        const blockDirectives = [
          { label: "if", detail: "Conditional block", snippet: "if ${1:condition}\n\t$0\n%endif" },
          { label: "for", detail: "Loop block", snippet: "for ${1:item} in ${2:items}\n\t$0\n%endfor" },
          { label: "match", detail: "Match expression", snippet: "match ${1:value}\n%when ${2:pattern}\n\t$0\n%endmatch" },
          { label: "logic", detail: "Rhai logic block", snippet: "logic\n$0\n%endlogic" },
          { label: "block", detail: "Named block", snippet: "block ${1:name}\n$0\n%endblock" },
          { label: "while", detail: "While loop", snippet: "while ${1:condition}\n\t$0\n%endwhile" },
        ];

        for (const dir of blockDirectives) {
          suggestions.push({
            label: dir.label,
            kind: monaco.languages.CompletionItemKind.Keyword,
            insertText: dir.snippet,
            insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
            detail: dir.detail,
            documentation: `Block directive: %${dir.label}`,
            range,
          });
        }

        // Control directives
        const controlDirectives = [
          { label: "else", detail: "Else branch" },
          { label: "elif", detail: "Else-if branch", snippet: "elif ${1:condition}" },
          { label: "when", detail: "Match case", snippet: "when ${1:pattern}" },
        ];

        for (const dir of controlDirectives) {
          suggestions.push({
            label: dir.label,
            kind: monaco.languages.CompletionItemKind.Keyword,
            insertText: dir.snippet || dir.label,
            insertTextRules: dir.snippet ? monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet : undefined,
            detail: dir.detail,
            range,
          });
        }

        // Simple directives
        const simpleDirectives = [
          { label: "include", detail: "Include template", snippet: 'include "${1:path}"' },
          { label: "let", detail: "Define variable", snippet: "let ${1:name} = ${2:value}" },
          { label: "set", detail: "Set variable", snippet: "set ${1:name} = ${2:value}" },
          { label: "render", detail: "Render block", snippet: "render ${1:block_name}" },
          { label: "blank", detail: "Output blank line" },
        ];

        for (const dir of simpleDirectives) {
          suggestions.push({
            label: dir.label,
            kind: monaco.languages.CompletionItemKind.Keyword,
            insertText: dir.snippet || dir.label,
            insertTextRules: dir.snippet ? monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet : undefined,
            detail: dir.detail,
            range,
          });
        }

        // End directives
        const endDirectives = ["endif", "endfor", "endmatch", "endblock", "endwhile", "endlogic"];
        for (const dir of endDirectives) {
          suggestions.push({
            label: dir,
            kind: monaco.languages.CompletionItemKind.Keyword,
            insertText: dir,
            detail: `End ${dir.slice(3)} block`,
            range,
          });
        }

        // Comment
        suggestions.push({
          label: "--",
          kind: monaco.languages.CompletionItemKind.Snippet,
          insertText: "-- ${1:comment}",
          insertTextRules: monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet,
          detail: "Comment",
          range,
        });
      }

      // Check if we're in interpolation and after a pipe (filter context)
      const inInterpolation = /\{\{[^}]*$/.test(beforeCursor);
      const afterPipe = /\|\s*\w*$/.test(beforeCursor);

      if (inInterpolation && afterPipe) {
        const filters = [
          { label: "lang_type", detail: "Convert to language type" },
          { label: "pascal_case", detail: "Convert to PascalCase" },
          { label: "camel_case", detail: "Convert to camelCase" },
          { label: "snake_case", detail: "Convert to snake_case" },
          { label: "screaming_snake_case", detail: "Convert to SCREAMING_SNAKE_CASE" },
          { label: "kebab_case", detail: "Convert to kebab-case" },
          { label: "upper", detail: "Convert to uppercase" },
          { label: "lower", detail: "Convert to lowercase" },
          { label: "trim", detail: "Trim whitespace" },
          { label: "count", detail: "Get array length" },
          { label: "join", detail: "Join array elements", snippet: 'join("${1:,}")' },
          { label: "first", detail: "Get first element" },
          { label: "last", detail: "Get last element" },
          { label: "default", detail: "Default value if empty", snippet: 'default("${1:value}")' },
          { label: "escape", detail: "Escape special characters" },
          { label: "raw", detail: "Output raw without escaping" },
          { label: "indent", detail: "Add indentation", snippet: "indent(${1:level})" },
        ];

        for (const filter of filters) {
          suggestions.push({
            label: filter.label,
            kind: monaco.languages.CompletionItemKind.Function,
            insertText: filter.snippet || filter.label,
            insertTextRules: filter.snippet ? monaco.languages.CompletionItemInsertTextRule.InsertAsSnippet : undefined,
            detail: filter.detail,
            documentation: `Filter: ${filter.label}`,
            range,
          });
        }
      }

      // IR properties (after a dot or in interpolation)
      if (inInterpolation || beforeCursor.endsWith(".")) {
        const irProps = [
          { label: "name", detail: "Name of the item" },
          { label: "fqn", detail: "Fully qualified name" },
          { label: "fields", detail: "List of fields" },
          { label: "field_type", detail: "Type of the field" },
          { label: "is_optional", detail: "Whether field is optional" },
          { label: "is_array", detail: "Whether field is an array" },
          { label: "is_primary_key", detail: "Whether field is primary key" },
          { label: "default_value", detail: "Default value if set" },
          { label: "constraints", detail: "Field constraints" },
          { label: "doc_comment", detail: "Documentation comment" },
          { label: "annotations", detail: "Annotations list" },
          { label: "variants", detail: "Enum variants" },
          { label: "structs", detail: "Struct definitions" },
          { label: "enums", detail: "Enum definitions" },
          { label: "namespaces", detail: "Namespace list" },
          { label: "children", detail: "Child namespaces" },
          { label: "indexes", detail: "Index definitions" },
          { label: "base_type", detail: "Base type name" },
          { label: "type_ref", detail: "Type reference" },
          { label: "pack_separator", detail: "@pack separator" },
        ];

        for (const prop of irProps) {
          suggestions.push({
            label: prop.label,
            kind: monaco.languages.CompletionItemKind.Property,
            insertText: prop.label,
            detail: prop.detail,
            range,
          });
        }
      }

      return { suggestions };
    },
  });

  isRegistered = true;
}
