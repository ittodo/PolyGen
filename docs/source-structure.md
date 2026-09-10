# PolyGen 소스 구조

> 최종 업데이트: 2026-09-10

---

## 개요

PolyGen은 `.poly` 스키마 파일을 파싱하여 여러 언어의 코드를 생성하는 폴리글랏 코드 생성기입니다.

파일 수와 줄 수는 구현에 따라 자주 바뀌므로 이 문서는 안정적인 모듈 책임과
의존성만 기록한다.

---

## 아키텍처 레이어

```
┌─────────────────────────────────────────────────────────────┐
│  CLI Layer                                                  │
│  main.rs → lib.rs (진입점)                                  │
├─────────────────────────────────────────────────────────────┤
│  Orchestration Layer                                        │
│  pipeline.rs (컴파일 파이프라인 조율)                       │
├─────────────────────────────────────────────────────────────┤
│  Parsing Layer                                              │
│  ast_parser/* → ast_model.rs                                │
├─────────────────────────────────────────────────────────────┤
│  Validation Layer                                           │
│  validation.rs + symbol_table.rs                            │
├─────────────────────────────────────────────────────────────┤
│  IR Layer                                                   │
│  ir_builder.rs + ir_builder/* + ir_model.rs                  │
├─────────────────────────────────────────────────────────────┤
│  Code Generation Layer                                      │
│  codegen.rs + template/* + rhai_generator.rs + rhai/*        │
├─────────────────────────────────────────────────────────────┤
│  DB/Migration Layer                                         │
│  migration.rs + db_introspection.rs + schema_metadata.rs     │
│  Source Config Layer                                        │
│  sources_config.rs                                           │
│  Tooling API Layer                                           │
│  project_schema.rs                                           │
│  Analysis Layer                                             │
│  visualize.rs + schema_diff.rs + schema_lint.rs              │
│  schema_stats.rs                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## 컴파일 파이프라인 흐름

```
.poly 파일
    ↓
[1. Parser] ─────────── polygen.pest + ast_parser/*
    ↓
[2. AST] ────────────── ast_model.rs
    ↓
[3. Validation] ─────── validation.rs + symbol_table.rs
    ↓
[4. IR Builder] ─────── ir_builder.rs + ir_builder/* → ir_model.rs
    ↓
[5. Type Registry] ──── type_registry.rs
    ↓
[6. Code Generator] ─── codegen.rs + template/renderer.rs
    ↓
[7. Templates] ───────── templates/<lang>/*.ptpl + rhai_utils/*.rhai
    ↓
Generated Code
```

---

## 모듈별 상세

### 1. 진입점 (Entry Point)

| 파일 | 역할 |
|------|------|
| `main.rs` | CLI 진입점, lib.rs 호출 |
| `lib.rs` | Clap CLI 정의, 명령어 라우팅, watch 모드 파일 감시 |

**주요 명령어:**
- `polygen generate` - 코드 생성
- `polygen migrate` - 마이그레이션 SQL 생성
- `polygen visualize` - ER 다이어그램 생성
- `polygen watch` - 스키마/템플릿 변경 감지 후 자동 재생성
- `polygen docs` - Markdown 스키마 문서 생성
- `polygen stats` - 스키마 통계 출력
- `polygen diff` - 스키마 변경 요약 출력
- `polygen lint` - 스키마 경고 출력

### 2. 파싱 레이어 (AST)

| 파일 | 역할 |
|------|------|
| `polygen.pest` | PEG 문법 정의 |
| `ast_model.rs` | AST 데이터 구조 |
| `ast_parser/mod.rs` | 메인 파서 (Pest → AST) |
| `ast_parser/definitions.rs` | table/enum/embed 파싱 |
| `ast_parser/fields.rs` | 필드 정의 파싱 |
| `ast_parser/types.rs` | 타입 파싱 |
| `ast_parser/metadata.rs` | 주석/어노테이션 파싱 |
| `ast_parser/literals.rs` | 리터럴 값 파싱 |
| `ast_parser/helpers.rs` | 유틸리티 함수 |
| `ast_parser/macros.rs` | 파싱 매크로 |

### 3. 검증 레이어 (Validation)

| 파일 | 역할 |
|------|------|
| `validation.rs` | AST 유효성 검사 |
| `symbol_table.rs` | 심볼 테이블, 이름 해석 |

**검증 항목:**
- 중복 정의 검사
- 타입 참조 유효성
- 순환 참조 감지
- 제약조건 검증

### 4. 중간 표현 (IR)

| 파일 | 역할 |
|------|------|
| `ir_model.rs` | IR 데이터 구조 |
| `ir_builder.rs` | AST → IR 변환 |
| `ir_builder/constraints.rs` | constraint/attribute/timezone IR 변환 헬퍼 |
| `ir_builder/indexes.rs` | index IR 생성 헬퍼 |
| `ir_builder/metadata.rs` | metadata/annotation IR 변환 헬퍼 |
| `ir_builder/refs.rs` | ref annotation IR 변환 헬퍼 |
| `ir_builder/renames.rs` | rename rule IR 변환 헬퍼 |
| `ir_builder/relations.rs` | foreign key reverse relation 후처리 |
| `ir_builder/type_resolution.rs` | TypeRef enum/struct flag 후처리 |
| `ir_builder/type_names.rs` | IR 타입/FQN 이름 헬퍼 |
| `type_registry.rs` | 타입 등록/조회 (FQN 관리) |

**IR 주요 구조:**
- `SchemaContext` - 전체 스키마
- `StructDef` - 구조체/클래스 정의
- `EnumDef` - Enum 정의
- `FieldDef` - 필드 정의
- `TypeRef` - 타입 참조

### 5. 코드 생성 (Code Generation)

| 파일 | 역할 |
|------|------|
| `codegen.rs` | 코드 생성 오케스트레이션 |
| `template/parser.rs` | PolyTemplate 파서 |
| `template/renderer.rs` | PolyTemplate 렌더러 |
| `template/context.rs` | 템플릿 값과 경로 조회 컨텍스트 |
| `template/expr.rs` | 템플릿 표현식/필터 파싱 |
| `template/filters.rs` | 내장 문자열 필터 |
| `template/rhai_bridge.rs` | `%logic`, `%if` Rhai 평가 브릿지 |
| `template/source_map.rs` | 생성물과 템플릿 원본 위치 매핑 |
| `rhai_generator.rs` | 레거시 Rhai 엔진 호환 레이어 |
| `lang_config.rs` | 언어별 설정 (TOML) |

**Rhai 모듈:**

| 파일 | 역할 |
|------|------|
| `rhai/mod.rs` | 모듈 진입점 |
| `rhai/registry.rs` | Rhai 함수/IR getter 등록 (`FileDef.all_tables` 포함) |
| `rhai/common/ir_lookup.rs` | IR 조회 헬퍼 |
| `rhai/csharp/type_mapping.rs` | C# 타입 매핑 |
| `rhai/csharp/loaders/csv.rs` | C# CSV 로더 생성 |

### 6. DB/마이그레이션

| 파일 | 역할 |
|------|------|
| `migration.rs` | 스키마 diff, ALTER SQL 생성, 파괴적 변경 정책 |
| `db_introspection.rs` | SQLite 스키마 읽기 |
| `schema_metadata.rs` | IR 기반 스키마 JSON/해시 생성 |
| `sources_config.rs` | `.sources.toml` CSV/JSON load path 파싱과 IR 병합 |
| `project_schema.rs` | import 파싱, 검증, lint, IR, sources 적용을 묶은 공개 tooling API |

### 7. 유틸리티

| 파일 | 역할 |
|------|------|
| `error.rs` | 에러 타입 정의 |
| `visualize.rs` | Mermaid ER 다이어그램/Markdown 문서 생성 |
| `schema_diff.rs` | 스키마 diff 요약 리포트 생성 |
| `schema_lint.rs` | 미사용 import, 순환 타입 참조 등 스키마 lint 경고 |
| `schema_stats.rs` | 스키마 통계 집계와 text 출력 |
| `pipeline.rs` | 컴파일 파이프라인 조율 |

---

## 모듈 의존성

```
main.rs
  └── lib.rs
        ├── pipeline.rs
        │     ├── ast_parser/* ──→ ast_model.rs
        │     ├── validation.rs ──→ symbol_table.rs
        │     ├── ir_builder.rs ──→ ir_model.rs
        │     │      ├── ir_builder/constraints.rs
        │     │      ├── ir_builder/indexes.rs
        │     │      ├── ir_builder/metadata.rs
        │     │      ├── ir_builder/refs.rs
        │     │      ├── ir_builder/renames.rs
        │     │      ├── ir_builder/relations.rs
        │     │      ├── ir_builder/type_resolution.rs
        │     │      ├── ir_builder/type_names.rs
        │     │      └── type_registry.rs
        │     ├── codegen.rs ──→ template/*
        │     │              ├── rhai_generator.rs
        │     │              └── lang_config.rs
        │     └── migration.rs ──→ db_introspection.rs
        │                └── schema_metadata.rs
        └── project_schema.rs
              ├── pipeline.rs + validation.rs + schema_lint.rs
              └── ir_builder.rs + sources_config.rs
```

---

## 작업별 파일 위치

| 작업 | 파일 |
|------|------|
| 문법/파싱 문제 | `polygen.pest` → `ast_parser/*` |
| 이름/타입 검증 | `validation.rs`, `symbol_table.rs` |
| 타입 해석/IR 구조 | `ir_builder.rs`, `ir_builder/*`, `ir_model.rs` |
| 생성 코드 변경 | `templates/<lang>/*.ptpl`, `templates/<lang>/rhai_utils/*.rhai` |
| PolyTemplate 엔진 변경 | `template/parser.rs`, `template/renderer.rs`, `template/rhai_bridge.rs`, `template/expr.rs` |
| Rhai 함수 추가 | `rhai/registry.rs`, `rhai/common/*` |
| 언어별 로직 | `rhai/<lang>/*` |
| DB 마이그레이션 | `migration.rs`, `db_introspection.rs`, `schema_metadata.rs` |
| CSV/JSON load source 설정 | `sources_config.rs`, `*.sources.toml` |
| GUI/외부 도구 스키마 로딩 | `project_schema.rs`의 `load_project_schema` |
| 스키마 분석/통계 | `visualize.rs`, `schema_diff.rs`, `schema_lint.rs`, `schema_stats.rs` |

---

## 분리 상태 평가

| 레이어 | 평가 | 비고 |
|--------|------|------|
| ast_parser/ | ⭐⭐⭐⭐⭐ | 8개 모듈로 완벽 분리 |
| validation | ⭐⭐⭐⭐ | symbol_table 분리됨 |
| ir_builder | ⭐⭐⭐⭐ | 하위 모듈 분리 진행 중 |
| codegen | ⭐⭐⭐⭐ | 역할별 분리 |
| rhai/ | ⭐⭐⭐ | csv.rs 분리 가능 |
| migration | ⭐⭐⭐⭐ | introspection 분리됨 |

---

*최종 업데이트: 2026-09-10*
