# PolyGen

PolyGen은 `.poly` 스키마를 단일 진실 공급원(SSOT)으로 삼아, 타겟 언어별 코드/유틸리티를 생성하는 코드 생성기입니다.

## 가능한 것(현재)
- `.poly` 스키마 파싱 → AST/IR 생성
- Rhai 템플릿 기반 코드 생성
- C# 타겟 코드 생성(생성 코드 + `static/csharp` 공용 유틸 복사)
- 스냅샷 테스트로 AST/IR(및 일부 코드 생성 결과) 검증

## 빠른 시작
```bash
cargo run -- --schema-path examples/game_schema.poly --lang csharp
```

## 실행 옵션(주요)
- `--schema-path`: 최상위 `.poly` 파일 경로
- `--lang`: 타겟 언어(예: `csharp`)
- `--templates-dir`: 템플릿 디렉토리(기본: `templates`)
- `--output-dir`: 출력 디렉토리(기본: `output`)

## 디렉토리 안내
- `src/`: 파서/검증/IR 빌드/템플릿 실행 등 코어 파이프라인
- `templates/`: 타겟 언어별 Rhai 템플릿
- `static/`: 생성 결과에 함께 복사되는 정적 파일(C# 공용 유틸 등)
- `docs/`: 설계/사양/제안 문서
- `examples/`: 예제 스키마 및 데모 코드
- `tests/`: 스냅샷 테스트 및 C# 컴파일/기능 테스트

## 문서 진입점
- 개발/아키텍처: `development_guide.md`
- JSON → CSV 변환 사양: `docs/json-to-csv-conversion-spec.md`

## 주의사항
- 기본 출력 경로인 `output/`은 실행 시 재생성될 수 있습니다. 중요한 경로를 `--output-dir`로 지정하지 마세요.
