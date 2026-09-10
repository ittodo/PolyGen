# PolySheet Demo

`examples/polysheet_demo.polysheet/`는 PolySheet를 설치한 뒤 바로 열어 볼 수 있는
작은 게임 데이터 프로젝트다.

## 포함 내용

- `polysheet_demo.poly`: enum, embed, optional, list, foreign key가 포함된 스키마
- `polysheet_demo.sources.toml`: 세 개의 JSON source 연결
- `data/polysheet_demo/*.json`: Category, Item, Quest 원본 데이터
- `polysheet_demo.polysheet/`: 세 개의 데이터 시트와 계산 시트

데이터 시트에도 수식을 넣어 두었다. Items의 `enabled`는 각 행의 `price`가
1000 이상인지 계산하고, Quests의 `reward_count`는 `IF(repeatable, 3, 1)`로
계산한다. 계산값은 JSON에 저장하고 수식과 stable 참조 바인딩은 각 시트의
`formulas.json`에 저장한다.

계산 시트의 `B3`에는 `=B1*B2` 수식과 stable 행·열 참조가 저장되어 있다. 계산
시트의 첫 행이나 첫 열 앞에 새 행·열을 삽입하면 화면 좌표는 바뀌지만 수식 대상은
같은 stable ID를 계속 가리킨다.

## 열기

PolySheet 시작 화면에서 다음 폴더를 선택한다.

```text
examples/polysheet_demo.polysheet
```

CLI로 먼저 검증할 수도 있다.

```powershell
cargo run --manifest-path polysheet/cli/Cargo.toml -- validate examples/polysheet_demo.polysheet
```

JSON을 편집한 뒤에는 Items와 Quests 시트에서 enum, FK, `null`, list/embed 검증을
확인하고, Diff 탭에서 필드 단위 변경을 확인할 수 있다.
