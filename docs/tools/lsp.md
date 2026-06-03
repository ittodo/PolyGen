# PolyGen LSP

> 최종 업데이트: 2026-06-03

`polygen-lsp/`는 `.poly` 스키마 언어용 Language Server입니다.

## 실행/빌드

```bash
cd polygen-lsp
cargo build --release
```

## 지원 기능

- Diagnostics
- Completion
- Hover
- Go to Definition
- Find References
- Document Symbols
- Rename

## VS Code 연동

VS Code 확장은 이 LSP 바이너리를 stdio로 실행합니다. 설정은 `vscode-extension.md`를 참고합니다.

```json
{
  "polygen.lsp.path": "polygen-lsp",
  "polygen.lsp.enabled": true
}
```

## 변경 시 확인

- `.poly` 문법 변경: `../schema-annotations.md`
- VS Code client 설정/기능 변경: `vscode-extension.md`
- GUI LSP 연동 변경: `gui.md`

