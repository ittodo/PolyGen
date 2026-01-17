# PolyGen Phase 4: 성능 & 확장성 개선

## 목표
- 타입 관리 중앙화로 코드 단순화 및 성능 향상
- 메모리 사용 최적화
- 다국어 지원 확장성 개선
- 코드 문서화

---

## 작업 항목

### 4.1 TypeRegistry 구조체 설계
**목표:** 타입 정보를 중앙에서 관리하는 구조체 설계

**현재 문제:**
- `ir_builder.rs`의 `resolve_type_kinds()`에서 임시 HashSet/HashMap 사용
- `validation.rs`에서도 비슷한 타입 수집 로직 중복

**구현 계획:**
- [ ] `src/type_registry.rs` 모듈 생성
- [ ] `TypeRegistry` 구조체 정의
  ```rust
  pub struct TypeRegistry {
      types: HashMap<String, TypeInfo>,      // FQN -> TypeInfo
      by_name: HashMap<String, Vec<String>>, // 이름 -> FQN 목록
      by_namespace: HashMap<String, Vec<String>>, // 네임스페이스 -> FQN 목록
  }

  pub struct TypeInfo {
      pub fqn: String,
      pub kind: TypeKind,
      pub namespace: String,
      pub name: String,
  }

  pub enum TypeKind {
      Enum,
      Struct,
      Embed,
  }
  ```
- [ ] `TypeRegistry` 메서드 구현
  - `register()` - 타입 등록
  - `get()` - FQN으로 조회
  - `find_by_name()` - 이름으로 조회
  - `resolve()` - 상대 경로를 FQN으로 해석
  - `is_enum()`, `is_struct()` - 타입 종류 확인
- [ ] 단위 테스트 추가

### 4.2 TypeRegistry ir_builder.rs에 적용
**목표:** ir_builder에서 TypeRegistry 사용

**구현 계획:**
- [ ] `build_ir()` 함수에서 TypeRegistry 생성
- [ ] AST 순회하며 타입 등록 (1차 패스)
- [ ] 타입 참조 해석 (2차 패스)
- [ ] `resolve_type_kinds()` 함수 TypeRegistry 기반으로 리팩토링
- [ ] `adjust_typeref()` 함수 단순화
- [ ] 기존 테스트 통과 확인

### 4.3 불필요한 clone() 제거 최적화
**목표:** 불필요한 메모리 복사 제거

**현재 문제:**
- AST/IR 구조체에서 `String` clone이 빈번함
- 큰 스키마에서 메모리 사용량 증가

**구현 계획:**
- [ ] `cargo clippy`로 불필요한 clone 식별
- [ ] 참조(`&str`, `&String`)로 대체 가능한 곳 수정
- [ ] `Cow<'_, str>` 도입 검토
- [ ] 벤치마크로 개선 확인

### 4.4 String interning 도입 검토
**목표:** 중복 문자열 메모리 최적화

**배경:**
- 타입 이름, 네임스페이스 경로 등이 반복적으로 사용됨
- String interning으로 동일 문자열은 한 번만 저장

**구현 계획:**
- [ ] `string-interner` 또는 `lasso` 크레이트 평가
- [ ] 도입 시 예상 이점/비용 분석
- [ ] 프로토타입 구현 및 벤치마크
- [ ] 채택 여부 결정

### 4.5 언어별 설정 파일 시스템 (languages.toml)
**목표:** 하드코딩된 언어 설정을 설정 파일로 분리

**현재 문제:**
- `csharp_static_files()` 등이 코드에 하드코딩
- 새 언어 추가 시 Rust 코드 수정 필요

**구현 계획:**
- [ ] `languages.toml` 포맷 설계
  ```toml
  [csharp]
  extension = ".cs"
  static_files = [
      { source = "static/csharp/DataSource.cs", dest = "Common/DataSource.cs" },
  ]
  templates = ["csharp_file.rhai", "csharp_binary_readers_file.rhai"]

  [mysql]
  extension = ".sql"
  templates = ["mysql_file.rhai"]
  ```
- [ ] 설정 파일 파싱 구현
- [ ] `CodeGenerator`에서 설정 파일 사용
- [ ] 기본 설정 파일 생성

### 4.6 코드 문서화 (doc comments)
**목표:** 공개 API에 문서화 추가

**구현 계획:**
- [ ] `lib.rs` 모듈 문서화
- [ ] `ast_model.rs` 구조체/enum 문서화
- [ ] `ir_model.rs` 구조체/enum 문서화
- [ ] `pipeline.rs` 공개 함수 문서화
- [ ] `codegen.rs` 공개 함수 문서화
- [ ] `cargo doc --open`으로 문서 확인

---

## 우선순위 제안

| 순위 | 항목 | 이유 |
|------|------|------|
| 1 | 4.1, 4.2 TypeRegistry | 코드 품질 향상, 향후 확장 기반 |
| 2 | 4.6 문서화 | 유지보수성 향상 |
| 3 | 4.5 languages.toml | 확장성 개선 |
| 4 | 4.3, 4.4 최적화 | 성능이 문제될 때 진행 |

---

## 참고 파일

- 현재 타입 해석: `src/ir_builder.rs` (112-258줄)
- 타입 검증: `src/validation.rs`
- 코드 생성: `src/codegen.rs`, `src/pipeline.rs`
- IR 모델: `src/ir_model.rs`
- AST 모델: `src/ast_model.rs`

---

## 진행 상황

| 항목 | 상태 | 비고 |
|------|------|------|
| 4.1 TypeRegistry 설계 | ✅ 완료 | `src/type_registry.rs` 생성, TypeRegistry/TypeInfo/TypeKind 구조체 구현 |
| 4.2 TypeRegistry 적용 | ✅ 완료 | `ir_builder.rs`의 resolve_type_kinds() 리팩토링 완료 |
| 4.3 clone() 최적화 | ✅ 완료 | clippy로 redundant clone 6개 제거 (ir_builder.rs, rhai/csv.rs) |
| 4.4 String interning | ⏸️ 보류 | 현재 성능 이슈 없음, 필요시 추후 적용 |
| 4.5 languages.toml | ✅ 완료 | `src/lang_config.rs` 생성, `templates/csharp/csharp.toml` 설정 파일 생성, CodeGenerator 연동 |
| 4.6 문서화 | ✅ 완료 | lib.rs, ast_model.rs, ir_model.rs, pipeline.rs, codegen.rs 문서화 완료 |
