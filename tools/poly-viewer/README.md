# PolyGen Schema Viewer

`.poly` 스키마 파일을 웹 브라우저에서 렌더링하고 신택스 하이라이팅을 제공하는 도구입니다.

## 사용법

### 로컬에서 실행

```bash
# 디렉토리로 이동
cd tools/poly-viewer

# 간단한 HTTP 서버 실행 (Python 3)
python -m http.server 8080

# 또는 Node.js
npx serve .
```

브라우저에서 `http://localhost:8080` 접속

### 기능

- **실시간 렌더링**: 입력 창에 `.poly` 스키마를 입력하면 즉시 렌더링
- **파일 로드**: 파일 열기 버튼 또는 드래그 앤 드롭으로 `.poly` 파일 로드
- **다크/라이트 모드**: 테마 토글 버튼으로 전환
- **HTML 복사**: 렌더링된 HTML을 클립보드에 복사

## 신택스 하이라이팅

| 요소 | 색상 (다크) | 예시 |
|------|------------|------|
| 키워드 | 보라색 | `namespace`, `table`, `enum`, `embed`, `import` |
| 타입 | 파란색 | `u32`, `string`, `bool`, `PlayerClass` |
| 어노테이션 | 노란색 | `@load`, `@index`, `@taggable` |
| 제약조건 | 초록색 | `primary_key`, `unique`, `max_length` |
| 문자열 | 분홍색 | `"data/file.csv"` |
| 숫자 | 주황색 | `100`, `1.5` |
| 주석 | 회색 | `// comment` |
| 필드명 | 청록색 | `id:`, `name:` |

## 파일 구조

```
poly-viewer/
├── index.html         # 메인 HTML
├── style.css          # 스타일 (다크/라이트 테마)
├── poly-renderer.js   # 토크나이저 및 렌더러
└── README.md          # 이 파일
```

## API 사용 (JavaScript)

```javascript
// 기본 사용
const html = PolyRenderer.render(polySource);

// 줄 번호 없이 렌더링
const html = PolyRenderer.render(polySource, { showLineNumbers: false });

// 토큰만 얻기
const tokens = PolyRenderer.tokenize(polySource);
```

## 지원 요소

### 키워드
- `namespace`, `table`, `enum`, `embed`, `import`, `output`

### 기본 타입
- `string`, `bool`, `bytes`
- `u8`, `u16`, `u32`, `u64`
- `i8`, `i16`, `i32`, `i64`
- `f32`, `f64`

### 어노테이션
- `@load`, `@index`, `@taggable`, `@link_rows`
- `@datasource`, `@cache`, `@readonly`, `@soft_delete`
- `@renamed_from`, `@output`, `@server`, `@client`

### 제약조건
- `primary_key`, `unique`, `index`
- `max_length`, `default`, `range`, `regex`
- `foreign_key`, `auto_increment`
