# PolyGen Agent Index (machine-oriented)

이 문서는 자동화/에이전트가 PolyGen을 유지보수할 때 **가장 빠르게 맥락을 잡고**, 변경 지점과 검증 루트를 찾기 위한 인덱스입니다.

## Project Flow (high level)
1) Parse: `.poly` → Pest parse tree (`src/polygen.pest`)
2) AST build: parse tree → AST (`src/ast_parser.rs`, `src/ast_model.rs`)
3) Validate: AST 논리 검증 (`src/validation.rs`)
4) IR build: AST → 템플릿 친화 IR (`src/ir_builder.rs`, `src/ir_model.rs`)
5) Generate: IR + Rhai templates → output (`src/rhai_generator.rs`, `templates/`)
6) Copy statics: 타겟별 정적 파일 복사(예: C# 공용 유틸 `static/csharp/*`)

## Checkpoints (what to open first)
- 문법/파싱 문제: `src/polygen.pest` → `src/ast_parser.rs`
- 이름/타입/제약 검증: `src/validation.rs`
- 타입 해석/IR 구조: `src/ir_builder.rs` → `src/ir_model.rs`
- 생성 코드 변경: `templates/<lang>/...` (Rhai 템플릿)
- 생성 결과 런타임 유틸: `static/<lang>/...`
- 회귀 검증: `tests/` (스냅샷 및 C# 컴파일/기능 테스트)

## Agent Docs (scoped)
- `src/agent.md`
- `templates/agent.md`
- `static/agent.md`
- `tests/agent.md`
- `examples/agent.md`
- `docs/agent.md`

## Verification Shortcuts
- Rust 스냅샷 테스트: `cargo test`
- 포맷/린트: `cargo fmt --all`, `cargo clippy -- -D warnings`

## Notes
- 실행 시 `output/`이 재생성될 수 있으므로, 중요한 경로를 `--output-dir`로 지정하지 마세요.
