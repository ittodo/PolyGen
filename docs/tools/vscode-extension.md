# VS Code Extension

> 최종 업데이트: 2026-06-03

`polygen-vscode/`는 `.poly` 파일용 VS Code 확장입니다.

## 기능

- TextMate syntax highlighting
- LSP diagnostics/completion/hover
- Go to Definition
- Find References
- Document Symbols
- Rename

## 개발/패키징

```bash
cd polygen-vscode
npm install
npm run compile
npm run package
```

## 설정

| Setting | Default | 설명 |
|---------|---------|------|
| `polygen.lsp.path` | `polygen-lsp` | LSP server executable 경로 |
| `polygen.lsp.enabled` | `true` | LSP 기능 활성화 |

## 변경 시 확인

- LSP protocol 기능 변경: `lsp.md`
- 문법 하이라이팅 변경: `../schema-annotations.md`
- 패키징 또는 릴리즈 절차 변경: `polygen-vscode/README.md`

