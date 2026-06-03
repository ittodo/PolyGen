# Poly Viewer

> 최종 업데이트: 2026-06-03

`tools/poly-viewer/`는 `.poly` 스키마를 브라우저에서 렌더링하고 syntax highlight하는 정적 웹 도구입니다.

## 실행

```bash
cd tools/poly-viewer
python -m http.server 8080
```

브라우저에서 `http://localhost:8080`을 엽니다.

## 기능

- `.poly` source rendering
- syntax highlighting
- file open / drag and drop
- dark/light theme
- rendered HTML copy

## 변경 시 확인

- `.poly` 문법/어노테이션 토큰 변경: `../schema-annotations.md`
- 로컬 도구 사용법 변경: `../../tools/poly-viewer/README.md`

