# GUI App

> 최종 업데이트: 2026-06-03

`gui/`는 PolyGen의 Tauri 기반 데스크톱 GUI입니다.

## 위치

```text
gui/
├── src/         Svelte frontend
├── src-tauri/   Tauri/Rust backend
└── public/      static assets
```

## 실행

```bash
cd gui
npm install
npm run tauri:build
```

## 주요 기능

- `.poly` 스키마 편집
- 실시간 문법/검증 오류 표시
- LSP 기반 탐색과 자동완성
- 템플릿 편집 및 생성 프리뷰
- 스키마 시각화, diff, migration 보조 기능

## 변경 시 확인

- GUI에서 CLI 옵션을 노출하면 루트 `README.md`와 `../status.md`도 확인합니다.
- 스키마 문법 또는 LSP 동작 변경은 `../schema-annotations.md`, `lsp.md`와 함께 갱신합니다.

