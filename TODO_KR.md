# TODO (세션 요약)

작성일: 2025-08-28

## 오늘 한 일
- C# 코드 생성 확장 (외부 Readers/Writers)
  - 템플릿: `templates/csharp/csharp_binary_readers_*`, `templates/csharp/csharp_binary_writers_*`
  - 공용 유틸: `static/csharp/BinaryUtils.cs` (원시형/문자열 UTF-8+u32/열거형 i32/리스트/옵션 읽기·쓰기)
- JSON 매퍼
  - 템플릿: `templates/csharp/csharp_json_mappers_*`
  - 공용 유틸: `static/csharp/JsonUtils.cs`
  - 생성 코드의 프로퍼티 따옴표·중괄호 닫힘 이슈 수정
- CSV 컬럼 헤더 생성
  - 템플릿: `templates/csharp/csharp_csv_columns_file.rhai`
  - 규칙: 임베드 도트 전개, 리스트는 `[0]` 전개, Option은 내부 타입 기준, Enum 포함
  - 외부 struct 전개 지원(예: `game.common.Position` → `x,y`)
  - 순환 참조 감지 + 깊이 10 제한, 원형 네이밍 유지
- 모든 언어 실행 모드
  - `--lang` 생략 시 `--templates-dir` 하위 언어 자동 탐색 후 전부 실행

## 결정 사항
- 엔디언: Little-endian 고정(빅엔디언 미지원)
- Enum 기반형: i32
- 문자열(바이너리): UTF-8(무 BOM) + u32(LE) 바이트 길이 접두
- CSV 네이밍: 원형 유지
- CSV 리스트: `[0]` 한 개 원소 전개
- CSV 임베드: 도트 표기
- CSV Option: 별도 표시 없음(빈 셀 처리)

## 진행 중 / 보완 필요
- CSV Mappers(C#) 생성은 Rhai 제약(fn-in-eval)으로 러너에서 임시 비활성화
  - 전역 헬퍼(`templates/csharp/rhai_utils/csv_helpers.rhai`) 도입 완료
  - struct 템플릿이 전역 헬퍼만 호출하도록 마무리 필요
- 외부 struct 전개 시 ColumnCount 참조 등의 placeholder 정리 필요
- JSON→CSV 간단 변환 예제 유틸 부재

## 다음 액션
1) CSV Mappers 최종화 및 러너 재활성화
   - `struct/csharp_csv_mappers_struct_body.rhai`에서 전역 헬퍼만 사용하도록 정리
2) JSON→CSV 예제 유틸 추가
   - 임의 JSON 파서로 `IEnumerable<T>` 만들고 `<Type>Csv.WriteCsv(...)`로 저장
3) (선택) CSV 어노테이션
   - `@csv(name: ...)`, `@csv(ignore)`
4) 에러 처리 강화
   - CSV 깊이/순환 로깅, JSON 엄격/관대 모드
5) 샘플/테스트 추가

## 실행 방법
- 모든 언어 실행:
  - `cargo run -- --schema-path examples/game_schema.poly --templates-dir templates --output-dir output`
- 단일 언어(C#):
  - `cargo run -- --schema-path examples/game_schema.poly --templates-dir templates --output-dir output --lang csharp`

## 참고 경로
- Readers/Writers 템플릿: `templates/csharp/csharp_binary_{readers,writers}_*`
- JSON 매퍼 템플릿: `templates/csharp/csharp_json_mappers_*`
- CSV 컬럼: `templates/csharp/csharp_csv_columns_file.rhai`
- CSV 헬퍼(Rhai): `templates/csharp/rhai_utils/csv_helpers.rhai`
- 공용 유틸(C#): `static/csharp/BinaryUtils.cs`, `static/csharp/JsonUtils.cs`, `static/csharp/CsvUtils.cs`

