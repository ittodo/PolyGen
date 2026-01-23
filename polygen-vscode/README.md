# PolyGen VS Code Extension

Visual Studio Code extension for PolyGen schema language (`.poly` files).

## Features

- **Syntax Highlighting**: Full TextMate grammar for `.poly` files
- **Diagnostics**: Real-time error detection via LSP
- **Autocompletion**: Keywords, types, constraints
- **Hover Documentation**: Inline documentation for language elements

## Installation

### From VSIX (Recommended)

1. Build the LSP server:
   ```bash
   cd ../polygen-lsp
   cargo build --release
   ```

2. Install `polygen-lsp` to PATH or configure its path in settings.

3. Build and install the extension:
   ```bash
   cd polygen-vscode
   npm install
   npm run compile
   npm run package
   ```

4. Install the generated `.vsix` file in VS Code:
   - Open VS Code
   - Press `Ctrl+Shift+P` → "Extensions: Install from VSIX..."
   - Select the generated `polygen-vscode-0.1.0.vsix`

### From Source (Development)

```bash
cd polygen-vscode
npm install
npm run watch
```

Then press `F5` in VS Code to launch Extension Development Host.

## Configuration

| Setting | Default | Description |
|---------|---------|-------------|
| `polygen.lsp.path` | `polygen-lsp` | Path to LSP server executable |
| `polygen.lsp.enabled` | `true` | Enable/disable LSP features |

## Syntax Highlighting Examples

```poly
// File imports
import "common/types.poly";

namespace game.character {
    // Enum with explicit values
    enum Status {
        Active = 1
        Inactive = 2
    }

    @load(csv: "data/players.csv")
    @cache
    table Player {
        id: u32 primary_key;
        name: string max_length(100);
        status: Status;
        level: u16 default(1) range(1, 100);
        email: string? unique;
        skills: Skill[];
    }
}
```

## Requirements

- VS Code 1.85.0 or later
- `polygen-lsp` binary (for LSP features)

## Known Issues

- LSP features require the `polygen-lsp` binary to be installed separately
- Go to definition not yet implemented

## Roadmap

- [ ] Go to definition
- [ ] Find all references
- [ ] Rename symbol
- [ ] Code formatting
- [ ] Snippets
