# PolyGen Language Server (polygen-lsp)

LSP (Language Server Protocol) implementation for the PolyGen schema language.

## Features

- **Diagnostics**: Real-time syntax and validation error reporting
- **Autocompletion**: Keywords, types, constraints, and annotations
- **Hover**: Documentation on hover for keywords and types

## Building

```bash
cd polygen-lsp
cargo build --release
```

The binary will be at `target/release/polygen-lsp`.

## Installation

### Option 1: Add to PATH

```bash
# Linux/macOS
cp target/release/polygen-lsp ~/.local/bin/

# Windows
copy target\release\polygen-lsp.exe %USERPROFILE%\bin\
```

### Option 2: Specify path in VS Code settings

```json
{
  "polygen.lsp.path": "/path/to/polygen-lsp"
}
```

## Usage with VS Code

See the `polygen-vscode` extension which uses this LSP server.

## Protocol Support

The server implements LSP 3.17 with the following capabilities:

| Feature | Status |
|---------|--------|
| textDocument/didOpen | ✅ |
| textDocument/didChange | ✅ |
| textDocument/didClose | ✅ |
| textDocument/completion | ✅ |
| textDocument/hover | ✅ |
| textDocument/publishDiagnostics | ✅ |

## Architecture

```
┌─────────────────┐       ┌─────────────────┐
│   VS Code       │◄─────►│  polygen-lsp    │
│   Extension     │ stdio │  (Rust binary)  │
└─────────────────┘       └────────┬────────┘
                                   │
                          ┌────────▼────────┐
                          │     polygen     │
                          │   (parser/IR)   │
                          └─────────────────┘
```
