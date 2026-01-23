# PolyGen 프로젝트 구조

이 문서는 PolyGen 프로젝트의 폴더 구조와 각 디렉토리의 역할을 설명합니다.

## 디렉토리 구조

```
polygen/
├── src/                    # Rust 소스 코드 (핵심 로직)
├── templates/              # 코드 생성 템플릿 (Rhai 스크립트)
├── static/                 # 정적 유틸리티 파일 (언어별)
├── tests/                  # 테스트
├── examples/               # 예제 스키마
├── docs/                   # 문서
├── gui/                    # Tauri GUI 앱
├── polygen-lsp/            # LSP 서버
├── polygen-vscode/         # VS Code 확장
├── tools/                  # 추가 도구
├── target/                 # Rust 빌드 출력 (gitignore)
├── .cargo/                 # Cargo 설정
├── .github/                # GitHub Actions
├── .vscode/                # VS Code 설정
└── [설정 파일들]
```

## 핵심 디렉토리

### `src/` - Rust 소스 코드

PolyGen의 핵심 로직이 포함된 Rust 소스 코드입니다.

```
src/
├── main.rs               # CLI 진입점
├── lib.rs                # 라이브러리 진입점
├── polygen.pest          # Pest 문법 정의 (.poly 파싱)
├── ast_model.rs          # AST 데이터 구조
├── ast_parser/           # AST 파서 모듈
├── validation.rs         # AST 유효성 검사
├── ir_model.rs           # IR 데이터 구조
├── ir_builder.rs         # AST → IR 변환
├── type_registry.rs      # 타입 레지스트리
├── pipeline.rs           # 컴파일 파이프라인
├── codegen.rs            # 코드 생성 유틸리티
├── rhai_generator.rs     # Rhai 템플릿 엔진
├── error.rs              # 에러 타입 정의
└── rhai/                 # Rhai 함수 모듈
    ├── registry.rs       # 함수 등록
    ├── common/           # 공통 함수
    └── csharp/           # C# 전용 함수
```

### `templates/` - 코드 생성 템플릿

Rhai 스크립트로 작성된 코드 생성 템플릿입니다.

```
templates/
├── csharp/               # C# 템플릿
├── cpp/                  # C++ 템플릿
├── rust/                 # Rust 템플릿
├── typescript/           # TypeScript 템플릿
├── go/                   # Go 템플릿
├── mysql/                # MySQL DDL 템플릿
├── sqlite/               # SQLite 템플릿
├── mermaid/              # Mermaid 다이어그램
├── unreal/               # Unreal Engine 템플릿
└── rhai_utils/           # 공용 유틸리티
```

### `static/` - 정적 유틸리티 파일

생성된 코드와 함께 복사되는 런타임 유틸리티입니다.

```
static/
├── csharp/               # C# 유틸리티 (CsvUtils, JsonUtils, BinaryUtils 등)
├── cpp/                  # C++ 유틸리티 (polygen_support.hpp)
└── rust/                 # Rust 유틸리티
```

### `tests/` - 테스트

단위 테스트, 스냅샷 테스트, 통합 테스트가 포함됩니다.

```
tests/
├── integration/          # 통합 테스트 스키마 (8개 케이스)
│   ├── 01_basic_types/
│   ├── 02_imports/
│   ├── 03_nested_namespaces/
│   ├── 04_inline_enums/
│   ├── 05_embedded_structs/
│   ├── 06_arrays_and_optionals/
│   ├── 07_indexes/
│   └── 08_complex_schema/
├── runners/              # 언어별 테스트 러너
│   ├── csharp/           # C# 테스트 러너 + 테스트 코드
│   ├── rust/             # Rust 테스트 러너
│   ├── typescript/       # TypeScript 테스트 러너
│   ├── go/               # Go 테스트 러너
│   └── cpp/              # C++ 테스트 러너
├── schemas/              # 스냅샷 테스트용 스키마
├── snapshots/            # Insta 스냅샷
├── output/               # 스냅샷 테스트 출력
├── test_data/            # 테스트 데이터
└── snapshot_tests.rs     # 스냅샷 테스트 코드
```

### `gui/` - Tauri GUI 앱

웹 기반 GUI 애플리케이션입니다.

```
gui/
├── src/                  # Svelte 프론트엔드
├── src-tauri/            # Tauri 백엔드 (Rust)
└── public/               # 정적 리소스
```

### `polygen-lsp/` - LSP 서버

.poly 파일을 위한 Language Server Protocol 구현입니다.

### `polygen-vscode/` - VS Code 확장

.poly 파일 구문 강조 및 LSP 클라이언트를 제공합니다.

### `tools/` - 추가 도구

```
tools/
└── poly-viewer/          # .poly 스키마 뷰어
```

## 빌드 출력

### `target/` (gitignore)

Rust 컴파일러 출력 폴더입니다. `cargo build` 실행 시 자동 생성됩니다.

```
target/
├── debug/                # 디버그 빌드 출력
│   └── polygen.exe       # 디버그 바이너리
├── release/              # 릴리즈 빌드 출력
│   └── polygen.exe       # 릴리즈 바이너리 (최적화)
└── [의존성 캐시]
```

**참고:** 이 폴더는 `.gitignore`에 포함되어 있어 버전 관리되지 않습니다.

## 설정 파일

| 파일 | 설명 |
|------|------|
| `Cargo.toml` | Rust 패키지 설정 및 의존성 |
| `Cargo.lock` | 의존성 버전 잠금 |
| `.gitignore` | Git 무시 패턴 |
| `README.md` | 프로젝트 소개 |
| `LICENSE` | 라이선스 (MIT) |
| `CLAUDE.md` | AI 어시스턴트 가이드 |
| `agent.md` | 에이전트용 빠른 인덱스 |
| `development_guide.md` | 개발 워크플로우 가이드 |

## 문서 (`docs/`)

| 파일 | 설명 |
|------|------|
| `PROJECT_STRUCTURE.md` | 프로젝트 구조 설명 (이 문서) |
| `LANGUAGE_SUPPORT_GUIDE.md` | 새 언어 지원 추가 가이드 |
| `IMPLEMENTATION_ROADMAP.md` | 구현 로드맵 |
| `TODO.md` | 할 일 목록 |
| `*_TODO.md`, `*_PLAN.md` | 완료된 리팩토링 계획들 |

---

*최종 업데이트: 2025-01-23*
