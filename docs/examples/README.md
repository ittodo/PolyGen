# Examples

> 최종 업데이트: 2026-06-03

이 폴더는 README에 넣기에는 긴 사용 예제를 분리해 둡니다. README에는 짧은 소개와
최소 예제만 유지하고, 설명이 길어지는 경우 이 폴더에 예제를 추가합니다.

## 예제 목록

| 문서 | 설명 |
|------|------|
| `quickstart.md` | 런타임 바이너리로 바로 실행 가능한 최소 예제: `.poly`, `.sources.toml`, CSV 데이터 |
| `features.md` | 기능별 예제 허브 |
| `constraints.md` | 필드 제약조건 예제 |
| `relations-indexes.md` | 외래 키, reverse relation, index 예제 |
| `sources.md` | `.sources.toml` load path 분리 예제 |
| `pack-embed.md` | `@pack` embed 직렬화 예제 |
| `search.md` | field-level `@search` 예제 |
| `basic-schema.md` | table, enum, constraint, index를 포함한 기본 스키마와 생성 명령 |

## 추가 기준

- README에서 한 예제가 20줄을 넘으면 여기로 분리합니다.
- 코드 블록이 2개 이상 필요하면 별도 예제 문서로 작성합니다.
- 특정 타겟별 차이가 핵심이면 `../targets/` 문서와 연결합니다.
- 예제 스키마 파일을 추가하거나 바꾸면 `../../examples/agent.md`도 확인합니다.
