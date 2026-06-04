# AGENTS.md - PolyGen AI Assistant Guide

> 최종 업데이트: 2026-06-03

이 문서는 Codex 및 기타 AI 어시스턴트가 PolyGen에서 작업할 때 따라야 하는
작업 지침입니다. 기능 스펙과 설계 문서의 원본은 `docs/`에 있습니다.

---

## 핵심 원칙

- `AGENTS.md`는 작업 지침이고, 스펙 원본은 `docs/`입니다.
- 개발 전에 관련 `docs/` 문서를 먼저 확인합니다.
- 코드, 템플릿, 테스트 동작을 바꾸면 같은 작업 안에서 관련 문서를 업데이트합니다.
- 오래된 문서를 그대로 복사해 새 문서를 만들지 않습니다. 원본 문서를 갱신하거나 링크합니다.
- `output/`은 재생성되는 출력 폴더이므로 중요한 파일을 저장하지 않습니다.
- 사용자가 명시적으로 요청하지 않은 코드 변경은 하지 않습니다.

---

## 프로젝트 개요

PolyGen은 `.poly` 스키마 파일을 단일 진실 공급원으로 사용해 여러 타겟의 코드를
생성하는 폴리글랏 코드 생성기입니다.

컴파일 파이프라인:

```text
.poly schema
  -> Pest parser (`src/polygen.pest`)
  -> AST builder (`src/ast_parser/`)
  -> validation (`src/validation.rs`, `src/symbol_table.rs`)
  -> IR builder (`src/ir_builder.rs`, `src/ir_builder/`)
  -> PolyTemplate renderer (`src/template/`, `templates/`)
  -> generated code + static utilities
```

주요 진입점:

| 파일 | 역할 |
|------|------|
| `src/main.rs` | CLI 진입점 |
| `src/lib.rs` | Clap CLI 정의, 명령어 라우팅 |
| `src/pipeline.rs` | 파싱, 검증, IR, 코드 생성 파이프라인 조율 |
| `src/codegen.rs` | 템플릿 기반 코드 생성 오케스트레이션 |

---

## 개발 전 필수 확인 문서

작업을 시작하기 전에 변경 유형에 맞는 문서를 확인합니다.

| 변경 유형 | 먼저 볼 문서 |
|----------|--------------|
| 소스 구조, 모듈 책임 변경 | `docs/source-structure.md` |
| `.poly` 문법, 어노테이션, 제약조건 변경 | `docs/schema-annotations.md` |
| CSV/JSON load path 설정 변경 | `docs/sources-config.md` |
| PolyTemplate 문법/렌더러 변경 | `docs/polytemplate-guide.md`, `docs/polytemplate-spec.md` |
| 템플릿 커스터마이징/Rhai helper 변경 | `docs/template-customization.md`, `templates/agent.md` |
| 새 언어/타겟 추가 | `docs/targets/language-support.md` |
| SQL, migration, datasource 변경 | `docs/targets/sql-support.md` |
| GUI/LSP/VS Code/poly-viewer 변경 | `docs/tools/` |
| README보다 긴 사용 예제 추가 | `docs/examples/` |
| 테스트/runner 변경 | `tests/agent.md`, `tests/runners/` |
| 정적 런타임 유틸리티 변경 | `static/agent.md` |
| 예제 스키마 변경 | `examples/agent.md` |

문서 목록과 역할은 `docs/README.md`를 기준으로 확인합니다.

---

## 작업별 코드 위치

| 작업 | 위치 |
|------|------|
| 문법/파싱 문제 | `src/polygen.pest`, `src/ast_parser/` |
| 이름/타입/제약 검증 | `src/validation.rs`, `src/symbol_table.rs` |
| 타입 해석/IR 구조 | `src/ir_builder.rs`, `src/ir_builder/`, `src/ir_model.rs` |
| 생성 코드 변경 | `templates/<lang>/` |
| PolyTemplate 엔진 변경 | `src/template/` |
| Rhai helper 등록 | `src/rhai/registry.rs`, `src/rhai/` |
| DB migration/introspection | `src/migration.rs`, `src/db_introspection.rs`, `src/schema_metadata.rs` |
| CSV/JSON load source config | `src/sources_config.rs`, `*.sources.toml` |
| 스키마 분석/문서/통계 | `src/visualize.rs`, `src/schema_diff.rs`, `src/schema_lint.rs`, `src/schema_stats.rs` |
| 통합 runner | `tests/runners/` |

---

## 필수 명령어

```bash
# 빌드
cargo build
cargo build --release

# 코드 생성
cargo run -- generate --schema-path examples/game_schema.poly --lang csharp --output-dir output
cargo run -- --schema-path examples/game_schema.poly --lang csharp

# Watch 모드
cargo run -- watch --schema examples/game_schema.poly --lang csharp

# 스키마 문서/분석
cargo run -- docs --schema-path examples/game_schema.poly --output docs/schema.md
cargo run -- stats --schema-path examples/game_schema.poly
cargo run -- diff --old old.poly --new new.poly
cargo run -- lint --schema-path examples/game_schema.poly

# 마이그레이션
cargo run -- migrate --baseline old.poly --schema-path new.poly
cargo run -- migrate --db game.db --schema-path schema.poly

# Rust 테스트/품질
cargo test
cargo clippy -- -D warnings
cargo fmt --all
cargo fmt --all -- --check
```

통합 runner:

```bash
tests\runners\run_all.bat --list
tests\runners\run_all.bat --verify
tests\runners\run_all.bat sqlite rust

bash tests/runners/run_all.sh --list
bash tests/runners/run_all.sh --verify
bash tests/runners/run_all.sh sqlite rust
```

---

## 문서 동기화 규칙

소스, 템플릿, 테스트, CLI 동작을 바꾸면 관련 문서를 같은 작업에서 갱신합니다.

| 변경 | 업데이트 대상 |
|------|---------------|
| `.rs` 파일 추가/삭제, 모듈 책임 변경 | `docs/source-structure.md` |
| 공개 CLI/API 변경 | `README.md`, `docs/README.md`, 해당 코드 doc comment |
| `.poly` 문법/어노테이션/제약조건 변경 | `docs/schema-annotations.md` |
| `.sources.toml` load 설정 변경 | `docs/sources-config.md` |
| PolyTemplate 문법/렌더러 변경 | `docs/polytemplate-guide.md`, `docs/polytemplate-spec.md` |
| 템플릿 구조/생성물 변경 | `templates/agent.md`, 관련 `docs/` |
| 새 언어/타겟 지원 | `README.md`, `docs/targets/language-support.md`, `docs/README.md` |
| 사용자용 예제 추가 | `README.md`, `docs/examples/README.md`, 필요 시 `examples/agent.md` |
| 테스트 케이스/runner 추가 | `tests/agent.md`, 필요 시 `docs/README.md` |
| 정적 런타임 유틸리티 변경 | `static/agent.md` |
| 기능 완료/우선순위 변경 | `docs/status.md` |

체크리스트:

```text
□ 개발 전 관련 docs 문서를 확인했는가?
□ 구현 변경과 문서 변경이 같은 작업에 포함되었는가?
□ 삭제/이동한 문서를 가리키는 링크가 남아 있지 않은가?
□ README는 사용자 관점, AGENTS는 에이전트 지침, docs는 스펙 원본 역할을 지키는가?
```

---

## 문서 배치 규칙

- 루트 `README.md`: 사용자용 소개, 빠른 시작, 주요 기능.
- 루트 `AGENTS.md`: 에이전트 작업 지침.
- `docs/README.md`: 문서 인덱스와 개발 전 필수 확인 경로.
- `docs/examples/`: README보다 긴 사용 예제.
- `docs/targets/`: 언어, DB, descriptor 타겟 문서.
- `docs/tools/`: GUI, LSP, VS Code 확장, 보조 도구 문서.
- `docs/*.md`: 공통 스펙, 설계, 개발 가이드, 현황 문서.
- 하위 `agent.md`: 해당 디렉터리 안에서만 필요한 짧은 작업 지침.

---

## 커밋 규칙

- 커밋 메시지는 영어로 작성합니다.
- 변경 유형 prefix를 사용합니다: `feat:`, `fix:`, `refactor:`, `docs:`, `test:`.
