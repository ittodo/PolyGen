# PolyGen 리팩토링 TODO

## Phase 1: 즉시 해결 (코드 품질) ✅ 완료

- [x] 1.1 Clippy 경고 수정 - 미사용 import 제거
- [x] 1.2 Clippy 경고 수정 - 불필요한 mutable 변수 제거
- [x] 1.3 Clippy 경고 수정 - 기타 경고 해결
- [x] 1.4 패키지 이름 snake_case 적용 (PolyGen → polygen)

## Phase 2: 단기 개선 (에러 처리 & 모듈화) ✅ 완료

- [x] 2.1 CodeGenError 전용 에러 타입 생성
- [x] 2.2 rhai_generator.rs 에러 타입 적용
- [x] 2.3 ast_parser.rs 모듈 분리 - helpers.rs
- [x] 2.4 ast_parser.rs 모듈 분리 - literals.rs
- [x] 2.5 ast_parser.rs 모듈 분리 - metadata.rs
- [x] 2.6 ast_parser.rs 모듈 분리 - types.rs, definitions.rs, fields.rs
- [x] 2.7 파서 반복 코드 매크로 추출

## Phase 3: 중기 개선 (아키텍처) ✅ 완료

- [x] 3.1 CodeGenerator 구조체 생성
- [x] 3.2 CompilationPipeline 구조체 생성
- [x] 3.3 lib.rs run() 함수 리팩토링
- [x] 3.4 단위 테스트 추가 - validation (24개 테스트)
- [x] 3.5 단위 테스트 추가 - type resolution (20개 테스트)
- [x] 3.6 단위 테스트 추가 - parser (24개 테스트)

## Phase 4: 장기 개선 (성능 & 확장성)

- [ ] 4.1 TypeRegistry 구조체 설계
- [ ] 4.2 TypeRegistry ir_builder.rs에 적용
- [ ] 4.3 불필요한 clone() 제거 최적화
- [ ] 4.4 String interning 도입 검토
- [ ] 4.5 언어별 설정 파일 시스템 (languages.toml)
- [ ] 4.6 코드 문서화 (doc comments)

---

## 진행 상황

| Phase | 완료 | 전체 | 진행률 |
|-------|------|------|--------|
| Phase 1 | 4 | 4 | 100% |
| Phase 2 | 7 | 7 | 100% |
| Phase 3 | 6 | 6 | 100% |
| Phase 4 | 0 | 6 | 0% |
| **Total** | **17** | **23** | **74%** |

---

## 작업 로그

### 2026-01-17 (Phase 3)
- Phase 3 완료
  - CodeGenerator 구조체 생성 (codegen.rs)
    - `CodeGenerator` - 언어별 코드 생성기
    - `StaticFileConfig` - 정적 파일 복사 설정
    - `csharp_static_files()` - C# 정적 파일 목록
    - `discover_languages()` - 템플릿 디렉토리에서 언어 자동 검색
  - CompilationPipeline 구조체 생성 (pipeline.rs)
    - `PipelineConfig` - 파이프라인 설정
    - `CompilationPipeline` - 전체 컴파일 파이프라인
    - `parse_and_merge_schemas()` - 스키마 파싱 및 병합
  - lib.rs 리팩토링 (234줄 → 68줄)
  - 단위 테스트 추가 (총 68개 테스트)
    - validation 테스트 (24개): 중복 정의, 타입 참조, 네임스페이스 등
    - ir_builder 테스트 (20개): 타입 해석, enum/struct 구분, 카디널리티 등
    - ast_parser 테스트 (24개): 테이블, enum, 네임스페이스, 제약조건 등

### 2026-01-17 (Phase 1-2)
- Phase 1 완료
  - 미사용 import 제거 (rhai_generator.rs, csv.rs, csharp.rs)
  - 불필요한 mut 제거 (ir_builder.rs, csv.rs)
  - 기타 Clippy 경고 해결 (30개 → 0개)
  - 패키지 이름 변경 (PolyGen → polygen)

- Phase 2 완료
  - CodeGenError 에러 타입 생성 (error.rs)
  - rhai_generator.rs 에러 타입 적용 (Result<String, String> → Result<String, CodeGenError>)
  - ast_parser.rs 전체 모듈화 완료
    - `ast_parser/mod.rs` - 메인 엔트리포인트
    - `ast_parser/helpers.rs` - parse_path, extract_comment_content
    - `ast_parser/literals.rs` - parse_literal
    - `ast_parser/metadata.rs` - parse_metadata, parse_annotation
    - `ast_parser/types.rs` - parse_type_name, parse_type_with_cardinality
    - `ast_parser/definitions.rs` - parse_definition, parse_namespace, parse_table, parse_enum, parse_embed
    - `ast_parser/fields.rs` - parse_field_definition, parse_regular_field, parse_constraint 등
    - `ast_parser/macros.rs` - require_next!, unexpected_rule! 매크로 추가
  - 파서 매크로 추출 완료
    - `require_next!` - MissingElement 에러 처리
    - `unexpected_rule!` - UnexpectedRule 에러 처리
