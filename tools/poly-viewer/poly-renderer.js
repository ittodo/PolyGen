/**
 * PolyGen Schema Renderer
 * Tokenizes and renders .poly schema files with syntax highlighting
 */

const PolyRenderer = (function() {
    // Token types
    const TokenType = {
        KEYWORD: 'keyword',
        TYPE: 'type',
        ANNOTATION: 'annotation',
        CONSTRAINT: 'constraint',
        STRING: 'string',
        NUMBER: 'number',
        COMMENT: 'comment',
        PUNCTUATION: 'punctuation',
        IDENTIFIER: 'identifier',
        FIELD_NAME: 'field-name',
        WHITESPACE: 'whitespace',
        NEWLINE: 'newline',
    };

    // Language definitions
    const KEYWORDS = new Set([
        'namespace', 'table', 'enum', 'embed', 'import', 'output'
    ]);

    const TYPES = new Set([
        'string', 'bool', 'bytes',
        'u8', 'u16', 'u32', 'u64',
        'i8', 'i16', 'i32', 'i64',
        'f32', 'f64'
    ]);

    const CONSTRAINTS = new Set([
        'primary_key', 'unique', 'index',
        'max_length', 'default', 'range', 'regex',
        'foreign_key', 'auto_increment'
    ]);

    const ANNOTATIONS = new Set([
        'load', 'index', 'taggable', 'link_rows',
        'datasource', 'cache', 'readonly', 'soft_delete',
        'renamed_from', 'output', 'server', 'client'
    ]);

    // Tokenizer
    function tokenize(input) {
        const tokens = [];
        let pos = 0;
        const len = input.length;

        while (pos < len) {
            const char = input[pos];
            const remaining = input.slice(pos);

            // Newline
            if (char === '\n') {
                tokens.push({ type: TokenType.NEWLINE, value: '\n' });
                pos++;
                continue;
            }

            // Whitespace (except newline)
            if (/[ \t\r]/.test(char)) {
                let ws = '';
                while (pos < len && /[ \t\r]/.test(input[pos])) {
                    ws += input[pos];
                    pos++;
                }
                tokens.push({ type: TokenType.WHITESPACE, value: ws });
                continue;
            }

            // Comments (// or ///)
            if (remaining.startsWith('//')) {
                let comment = '';
                while (pos < len && input[pos] !== '\n') {
                    comment += input[pos];
                    pos++;
                }
                tokens.push({ type: TokenType.COMMENT, value: comment });
                continue;
            }

            // String literals
            if (char === '"') {
                let str = '"';
                pos++;
                while (pos < len && input[pos] !== '"') {
                    if (input[pos] === '\\' && pos + 1 < len) {
                        str += input[pos] + input[pos + 1];
                        pos += 2;
                    } else {
                        str += input[pos];
                        pos++;
                    }
                }
                if (pos < len) {
                    str += '"';
                    pos++;
                }
                tokens.push({ type: TokenType.STRING, value: str });
                continue;
            }

            // Annotation (@name)
            if (char === '@') {
                let ann = '@';
                pos++;
                while (pos < len && /[a-zA-Z0-9_]/.test(input[pos])) {
                    ann += input[pos];
                    pos++;
                }
                tokens.push({ type: TokenType.ANNOTATION, value: ann });
                continue;
            }

            // Numbers
            if (/[0-9]/.test(char) || (char === '-' && /[0-9]/.test(input[pos + 1]))) {
                let num = '';
                if (char === '-') {
                    num = '-';
                    pos++;
                }
                while (pos < len && /[0-9.]/.test(input[pos])) {
                    num += input[pos];
                    pos++;
                }
                tokens.push({ type: TokenType.NUMBER, value: num });
                continue;
            }

            // Punctuation
            if (/[{}()\[\];:,=?.]/.test(char)) {
                tokens.push({ type: TokenType.PUNCTUATION, value: char });
                pos++;
                continue;
            }

            // Identifiers and keywords
            if (/[a-zA-Z_]/.test(char)) {
                let ident = '';
                while (pos < len && /[a-zA-Z0-9_]/.test(input[pos])) {
                    ident += input[pos];
                    pos++;
                }

                // Check for type suffixes like `?` or `[]`
                let type = TokenType.IDENTIFIER;

                if (KEYWORDS.has(ident)) {
                    type = TokenType.KEYWORD;
                } else if (TYPES.has(ident)) {
                    type = TokenType.TYPE;
                } else if (CONSTRAINTS.has(ident)) {
                    type = TokenType.CONSTRAINT;
                }

                tokens.push({ type, value: ident });
                continue;
            }

            // Unknown character - just pass through
            tokens.push({ type: TokenType.IDENTIFIER, value: char });
            pos++;
        }

        return tokens;
    }

    // Context-aware post-processing
    function processTokens(tokens) {
        const processed = [];
        let i = 0;

        while (i < tokens.length) {
            const token = tokens[i];
            const prev = processed[processed.length - 1];
            const prevNonWs = findPrevNonWhitespace(processed);

            // Field name detection: identifier followed by `:` (not `::`)
            if (token.type === TokenType.IDENTIFIER) {
                const next = findNextNonWhitespace(tokens, i + 1);
                if (next && next.type === TokenType.PUNCTUATION && next.value === ':') {
                    // Check it's not `::` (namespace separator)
                    const afterColon = tokens[findNextIndex(tokens, i + 1) + 1];
                    if (!afterColon || afterColon.value !== ':') {
                        processed.push({ type: TokenType.FIELD_NAME, value: token.value });
                        i++;
                        continue;
                    }
                }
            }

            // Type reference: identifier after `:` that's not a keyword/constraint
            if (token.type === TokenType.IDENTIFIER && prevNonWs) {
                if (prevNonWs.type === TokenType.PUNCTUATION && prevNonWs.value === ':') {
                    // Check if it looks like a custom type (starts with uppercase)
                    if (/^[A-Z]/.test(token.value)) {
                        processed.push({ type: TokenType.TYPE, value: token.value });
                        i++;
                        continue;
                    }
                }
            }

            processed.push(token);
            i++;
        }

        return processed;
    }

    function findPrevNonWhitespace(tokens) {
        for (let i = tokens.length - 1; i >= 0; i--) {
            if (tokens[i].type !== TokenType.WHITESPACE && tokens[i].type !== TokenType.NEWLINE) {
                return tokens[i];
            }
        }
        return null;
    }

    function findNextNonWhitespace(tokens, start) {
        for (let i = start; i < tokens.length; i++) {
            if (tokens[i].type !== TokenType.WHITESPACE && tokens[i].type !== TokenType.NEWLINE) {
                return tokens[i];
            }
        }
        return null;
    }

    function findNextIndex(tokens, start) {
        for (let i = start; i < tokens.length; i++) {
            if (tokens[i].type !== TokenType.WHITESPACE && tokens[i].type !== TokenType.NEWLINE) {
                return i;
            }
        }
        return tokens.length;
    }

    // HTML escape
    function escapeHtml(text) {
        return text
            .replace(/&/g, '&amp;')
            .replace(/</g, '&lt;')
            .replace(/>/g, '&gt;')
            .replace(/"/g, '&quot;');
    }

    // Render tokens to HTML
    function tokensToHtml(tokens, showLineNumbers = true) {
        let html = '';
        let lineNumber = 1;
        let lineContent = '';
        let isLineStart = true;

        function flushLine() {
            if (showLineNumbers) {
                html += `<span class="poly-line"><span class="poly-line-number">${lineNumber}</span>${lineContent}</span>`;
            } else {
                html += `<span class="poly-line">${lineContent}</span>`;
            }
            lineContent = '';
            lineNumber++;
            isLineStart = true;
        }

        for (const token of tokens) {
            if (token.type === TokenType.NEWLINE) {
                lineContent += '\n';
                flushLine();
                continue;
            }

            const escaped = escapeHtml(token.value);

            if (token.type === TokenType.WHITESPACE) {
                lineContent += escaped;
            } else {
                lineContent += `<span class="poly-${token.type}">${escaped}</span>`;
            }
            isLineStart = false;
        }

        // Flush remaining content
        if (lineContent) {
            flushLine();
        }

        return html;
    }

    // Main render function
    function render(input, options = {}) {
        const { showLineNumbers = true } = options;

        if (!input || !input.trim()) {
            return '<span class="poly-comment">// .poly 스키마를 입력하세요</span>';
        }

        const tokens = tokenize(input);
        const processed = processTokens(tokens);
        return tokensToHtml(processed, showLineNumbers);
    }

    // Export public API
    return {
        render,
        tokenize,
        TokenType
    };
})();

// Export for Node.js environment
if (typeof module !== 'undefined' && module.exports) {
    module.exports = PolyRenderer;
}
