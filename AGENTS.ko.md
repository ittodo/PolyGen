# Repository Guidelines

## 프로젝트 구조와 모듈 구성
- `src/`: 핵심 러스트 소스(예: `ast_*`, `ir_*`, `rhai_generator.rs`, `validation.rs`, `polygen.pest`, `lib.rs`, `main.rs`).
- `templates/`: 언어별 Rhai 템플릿(`templates/csharp/...`, `rhai_utils/` 포함).
- `static/`: 생성 결과에 복사되는 정적 자산(예: `static/csharp/DataSource.cs`).
- `examples/`: 예제 스키마(`*.poly`).
- `tests/`: 스냅샷 테스트 입력(`tests/schemas`)과 스냅샷(`tests/snapshots`).
- `output/`: 실행 시 생성되며 매 실행마다 삭제 후 재생성.

## 빌드·실행·개발 명령어
- 빌드: `cargo build` (최적화: `cargo build --release`).
- 실행 예시:
  - `cargo run -- --schema-path examples/character_types.poly`
- 테스트: `cargo test` (스냅샷 테스트 실행).
- 린트: `cargo clippy -- -D warnings`
- 포맷: `cargo fmt --all`

## 코딩 스타일·네이밍 규칙
- Rust 2021, 스페이스 4칸, 합리적 선에서 100자 내외 권장.
- 모듈/파일: `snake_case`, 타입/트레이트: `PascalCase`, 함수/변수: `snake_case`.
- 공개 API는 가급적 `lib.rs`에 두고, `main.rs`는 CLI 파싱만 수행하여 `run`에 위임.
- PR 전 `cargo fmt`/`cargo clippy`로 경고 0 유지.

## 테스트 가이드
- 프레임워크: `insta` 스냅샷(파일: `tests/snapshot_tests.rs`).
- 새 케이스는 `tests/schemas/*.poly` 추가 후 테스트 실행.
- 스냅샷 갱신 의도 시(Windows PowerShell): `$env:INSTA_UPDATE='auto'; cargo test` 또는 `cargo insta review`(설치 시).
- 스냅샷 ID 규칙: `{schema}_ast`, `{schema}_ir`.

## 커밋·PR 가이드
- 커밋 규약: Conventional Commits 사용(예: `feat(parser): ...`, `fix(ir): ...`, `refactor(generator): ...`).
- 커밋은 작고 목적 지향적으로, 본문에 관련 모듈/파일 요약.
- PR에는 개요, 관련 이슈 링크, 재현/실행 방법, 코드생성 변경 전·후 샘플(`output/` 일부 첨부)을 포함.

## 보안·구성 팁
- 주의: 실행 시 기존 `output/`가 삭제됩니다. 중요한 경로를 `--output-dir`로 지정하지 마세요.
- 파이프라인: Parse(`polygen.pest`) → Validate → IR Build → Rhai Codegen.
- C# 대상 시 `static/csharp/DataSource.cs`가 `output/<lang>/Common/`으로 복사됩니다.
