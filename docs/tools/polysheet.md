# PolySheet

> 최종 업데이트: 2026-09-10

PolySheet는 `.poly` 정의와 `.sources.toml`의 단일 JSON 파일을 스프레드시트로
편집하는 Windows 우선 데스크톱 도구다. Excel/XLSX 확장이 아니라 독립
Tauri 2 + Svelte 5 앱이며, Univer Sheets Core `0.25.1`의 오픈소스 범위만
사용한다.

## 핵심 원칙

- JSON 원본은 복제하지 않는다. 데이터 시트의 실제 행은 `.sources.toml`이
  가리키는 단일 JSON 배열에만 저장한다.
- `.polysheet/` 안에는 정의 연결, 수식, 서식, 숨은 행 ID, 자유 계산 시트만
  텍스트 파일로 저장한다.
- 새 문서는 경로나 스키마 없이 빈 계산 시트로 시작할 수 있다. 첫 저장에서
  `.polysheet/` 폴더를 고르고, `.poly`와 `.sources.toml`은 그때 또는 이후에
  연결한다. 스키마를 연결하기 전에는 자유 계산 시트만 사용할 수 있다. 새 문서의
  첫 계산 시트는 화면 컨테이너가 준비된 다음 초기화되어 생성 직후 바로 표시된다.
- 기존 문서를 열 때는 `workbook.toml`이 있는 `.polysheet` 폴더를 선택한다.
  연결된 JSON 데이터 폴더를 선택한 경우에는 선택 위치와 최대 세 단계의 상위
  폴더에서 가까운 `.polysheet` 프로젝트를 찾는다. 데이터 폴더명과 프로젝트명이
  일치하는 항목을 우선하고, 후보가 하나뿐인 경우에도 자동으로 그 프로젝트를 연다.
  후보가 여러 개라 결정할 수 없으면 사용자가 정확한 프로젝트 폴더를 선택해야 한다.
- 성공적으로 열거나 저장한 프로젝트는 시작 화면의 최근 프로젝트에 최대 8개까지
  최신 순서로 보관한다. 항목을 누르면 경로를 다시 찾지 않고 열 수 있으며,
  목록에서 제거해도 실제 프로젝트 파일은 삭제하지 않는다.
- 정렬과 필터는 보기 상태다. 명시적으로 순서를 적용하기 전에는 JSON 행 순서를
  바꾸지 않는다.
- 저장은 모든 관련 파일을 transaction journal에 먼저 기록한 뒤 임시 파일을 쓰고
  원자적으로 교체한다. 시트 삭제도 `workbook.toml` 갱신과 알려진 sidecar 삭제를
  같은 transaction으로 처리한다. 이때 연결된 데이터 JSON은 삭제하지 않고,
  알 수 없는 파일은 보존하며 시트 폴더는 완전히 비었을 때만 제거한다.
- 프로젝트 안의 journal도 신뢰하지 않는 입력이다. 다음 실행에서 중단된 journal을
  발견하면 파일을 자동 변경하지 않고 열기를 중단한다. 복구는 앱이 journal과
  독립적으로 산출하고 사용자가 확인한 모든 정확한 대상 파일을
  `recover_transaction_with_allowed_targets`에 전달했을 때만 롤백 또는 완료한다.
  Unix에서는 파일 교체·정리 뒤 부모 디렉터리도 동기화한다. Rust 표준 라이브러리가
  Windows 디렉터리 동기화를 제공하지 않는 환경에서는 각 파일 flush, rename,
  prepared/committed journal 순서로 프로세스 중단 복구 경계를 유지한다. 갑작스러운
  전원 차단 때의 디렉터리 metadata 내구성은 Windows와 파일시스템 동작에 의존한다.
- Stage는 사용자가 명시적으로 요청했을 때만 실행하며, JSON·수식·서식·행 ID
  sidecar와 `workbook.toml`을 함께 처리한다.

## 프로젝트 형식

```text
game.polysheet/
  workbook.toml
  sheets/
    <stable-sheet-id>/
      sheet.toml
      formulas.json
      format.json
      rowids.json
      cells.json
```

- `workbook.toml`: 포맷 버전, 선택적인 상대 `.poly`/`.sources.toml` 경로,
  데이터 루트, 시트 순서와 stable ID
- `sheet.toml`: 시트 이름, 종류, 연결한 table FQN
- `formulas.json`: 데이터 행 ID와 field path별 수식
- `format.json`: core diff/merge용 sparse 서식과 보기 상태. 데스크톱 v1은 기존
  sidecar를 보존하지만 UI round-trip이 완성될 때까지 대화형 서식 편집은 비활성화한다.
- `rowids.json`: primary key가 없는 데이터 시트의 UUIDv7 ID, 원본 내용 해시,
  행별 내용 지문
- `cells.json`: 자유 계산 시트의 stable row/column ID와 sparse cell map

JSON과 sidecar는 UTF-8/LF, 2칸 들여쓰기, 끝 줄바꿈으로 저장한다. 데이터 JSON
object는 `.poly` 필드 순서를 우선하며, 그 결과는 반복 저장해도 byte-identical하다.
기존 JSON이 이 형식과 다르면 첫 저장 전에 앱에서 기존/정규화 결과를 보여주고
명시적 승인을 요구한다. 최초 정규화는 별도 커밋으로 분리하는 것이 좋다.
처음 `다른 이름으로 저장`할 때도 같은 미리보기와 승인을 거치며, 승인을 취소하거나
저장이 실패하면 열린 세션의 기존 프로젝트 경로와 모델은 바꾸지 않는다.

JSON 패널은 스프레드시트 선택을 stable 행 ID와 field path로 해석하는 문맥형
인스펙터다.

- 데이터 시트에서 셀 편집을 확정하면 별도의 반영 버튼 없이 해당 stable 행만
  Rust 모델에 자동 동기화하고, 선택된 행 JSON 또는 전체 JSON 보기를 즉시 다시
  읽는다. 같은 행에 수식이 있으면 재계산된 materialized 값도 함께 반영한다.
  문자열 컬럼에 숫자처럼 보이는 값을 입력해도 스키마 타입에 맞춰 문자열로
  변환한다.
- 매핑된 컬럼명을 선택하면 타입, optional/null, primary key, enum/FK, 기본값,
  제약조건과 입력 예시를 보여준다.
- 기존 데이터 셀이나 행 하나를 선택하면 해당 행의 canonical JSON만 표시하며,
  Monaco에서 문자열의 닫는 따옴표 밖으로 캐럿이 이동하거나 scalar 값 뒤의
  쉼표를 통과하거나 다른 JSON 값으로 이동할 때 자동으로 검증해 그 행에만
  적용한다. 행 적용은 워크북을 재생성하지 않고 대응 셀만 갱신하므로 편집기의
  캐럿과 포커스를 유지한다. 문법·타입·스키마 검증에 실패하면 같은 시점에
  오류를 표시한다. 수식이 있는 값은 유효한 JSON 원문을 바꾸지 않고 JSON 편집기
  위의 `field · value · ƒ formula` 목록과 값 오른쪽 인라인 힌트로 함께 표시한다.
  편집 중 다른 셀을 선택해도 적용 또는 취소 전까지
  편집 대상을 고정해 변경을 잃지 않는다.
- 여러 기존 행을 선택하면 중첩 field path별 값을 비교하고 차이만 볼 수 있다.
  list/array는 원자 값이며 수식은 materialized 값과 별도로 표시한다. 대화형
  비교는 최대 100개 행이다.
- 빈 셀, 빈 행 또는 매핑되지 않은 영역은 스키마 기반 새 행 초안과 필수 필드를
  보여준다. 검증된 새 행은 정렬된 화면 위치가 아니라 JSON 원본 마지막에 추가한다.
- 전체 배열 편집은 자동 선택 상태가 아니라 명시적인 `전체 JSON` 보기로 유지한다.

행 JSON이나 전체 JSON의 필드명·값을 클릭하면 해당 stable 행 ID와 최상위
필드명을 찾아 스프레드시트의 대응 셀을 활성화한다. 정렬된 보기에서도 JSON 배열
인덱스를 화면 행 번호로 직접 사용하지 않는다. embed/list 내부 값을 클릭한
경우에는 해당 최상위 필드 셀을 선택한다.

## 행 ID와 수식

primary key가 있으면 `pk:<type>:<canonical-value>` 형식의 내부 ID를 사용한다.
primary key는 셀에서 직접 수정하거나 수식 대상으로 사용할 수 없다.

primary key가 없으면 JSON에 ID를 추가하지 않고 `rowids.json`에 UUIDv7을 둔다.
외부 JSON 변경 후 내용 지문이 유일하게 일치하는 행만 자동 재연결한다. 수정된
행 또는 중복 지문 때문에 모호한 행은 validation error로 처리되어 저장과 병합을
막는다.

데이터 시트 수식은 `row-id + field-path`로 저장하고 타입 검증된 계산값만 JSON에
materialize한다. `i64`/`u64`는 Rust에서 정확한 정수로 보관하고 프런트엔드에는
문자열로 전달한다. JavaScript safe integer 범위를 벗어나는 정수 수식은 `#NUM!`
오류로 저장을 차단한다.

수식의 A1 표현은 화면 표시용이다. `formulas.json`과 계산 시트 `cells.json`은
각 A1 참조에 대상 `sheet-id + row-id + column-id/field-path`, 원본 문자열 범위,
행·열 절대참조 플래그를 함께 저장한다. 계산 시트의 행·열 삽입은 새 UUIDv7을
`row_order` 또는 `column_order`에 넣으며 기존 셀과 수식 대상 ID는 바꾸지 않는다.
따라서 중간 열 삽입으로 화면의 `$A$1`이 `$B$1`로 바뀌어도 Git에서는 기존 셀
전체가 밀리지 않는다. 참조 대상이 삭제되면 `#REF!` 진단으로 저장과 Stage를
막는다. binding이 없는 이전 v1 sidecar는 열 때 현재 좌표를 기준으로 자동
보강한다.

## Diff와 3-way merge

기본 비교는 `HEAD`와 현재 in-memory working tree다. 임의 Git ref 두 개도 비교할
수 있으며 다음 변경을 분리한다.

과거 Git ref는 해당 revision에 커밋된 schema/import/sources/JSON을 기준으로
읽는다. 재현 가능성과 저장소 경계 보호를 위해 이 파일 연결은 Git 저장소 안의
상대 경로여야 하며, 절대 경로나 저장소 밖으로 벗어나는 연결은 비교하지 않는다.
revision 한 건의 논리적 읽기는 512 MiB와 2,000,000행으로 제한한다. 중복되거나
Windows에서 별칭이 되는 시트·소스 경로, 중복 행 ID, 같은 table/source의 중복
binding, 비어 있지 않은 keyless 데이터의 누락된 `rowids.json`도 비교 전에
거부한다. 선택 sidecar가 없을 때는 현재 프로젝트 열기와 같은 기본값을 사용한다.

- 시트/행 추가와 삭제
- 행 이동
- scalar 및 중첩 field path 변경
- 수식 변경
- stable 행·열 삽입과 수식 참조 대상 변경
- 서식 변경

3-way merge는 stable sheet/row/cell ID와 field path를 기준으로 한다. 서로 다른
필드 변경은 자동 병합한다. 같은 필드의 서로 다른 변경, delete-vs-edit, 같은
ID의 다른 행 추가, 양쪽의 충돌하는 순서 변경, 배열 양쪽 변경은 충돌로 남는다.
배열은 v1에서 원자 값이다. 병합된 수식은 계산값을 합치지 않고 Univer에서
재계산한 뒤 validation을 통과해야 저장·Stage할 수 있다.

## CLI

```powershell
cargo run --manifest-path polysheet/cli/Cargo.toml -- fmt game.polysheet --approve-normalization
cargo run --manifest-path polysheet/cli/Cargo.toml -- validate game.polysheet
cargo run --manifest-path polysheet/cli/Cargo.toml -- diff game.polysheet --base HEAD --target WORKTREE
cargo run --manifest-path polysheet/cli/Cargo.toml -- merge game.polysheet --base BASE --ours OURS --theirs THEIRS
# 정규화 미리보기를 별도로 확인하고 승인한 경우에만 추가
cargo run --manifest-path polysheet/cli/Cargo.toml -- merge game.polysheet --base BASE --ours OURS --theirs THEIRS --approve-normalization
cargo run --manifest-path polysheet/cli/Cargo.toml -- git-config game.polysheet
cargo run --manifest-path polysheet/cli/Cargo.toml -- git-config game.polysheet --apply
```

`git-config`는 먼저 제안할 `.gitattributes` 내용을 출력한다. `--apply`를 명시한
경우에만 저장소 로컬 `diff.polysheet.textconv` 설정과 데이터 경로별 attribute를
추가한다. 경로의 공백, `#`, 따옴표, glob 문자는 실제 한 파일만 일치하도록
escape한다. 자동 merge driver는 sidecar 묶음의 원자성을 보장하기 어려워 v1에서
등록하지 않는다.

`merge`는 기본적으로 최초 JSON 정규화를 승인하지 않는다. 정규화가 필요한
프로젝트에서는 먼저 diff를 검토하고 별도 저장으로 정규화를 분리하거나,
CLI에서 명시적으로 `--approve-normalization`을 전달해야 한다. 데스크톱 앱의
merge 적용도 정규화가 남아 있으면 파일을 바꾸지 않고 중단한다.

## Rust tooling API

PolySheet와 다른 편집 도구는 코드 생성을 실행하지 않고도 PolyGen의 공개 로더를
사용해 검증된 IR과 lint 결과를 얻을 수 있다.

```rust
use polygen::load_project_schema;

let loaded = load_project_schema("schemas/game.poly", None)?;
println!("{}", loaded.schema_path.display());
```

`None`은 입력 스키마 옆의 `<name>.sources.toml`을 자동 탐색하고, 명시적인 sources
경로는 기본 sidecar보다 우선한다. import와 기본 sidecar의 상대 경로 기준은 CLI와
같이 호출자가 전달한 스키마 경로이며, 반환되는 `schema_path`와 `sources_path`는
정규화된 절대 경로다. 이 API는 import와 `.renames`를 병합하고 validation, lint,
IR 구성, sources 적용까지 수행하지만 생성 파일은 만들지 않는다.

## 개발과 빌드

```powershell
cd polysheet
npm install
npm run tauri:dev
npm run build
npm run tauri:build
npm run velopack:build
# 같은 버전을 다시 패키징할 때만 기존 로컬 release를 지운다.
npm run velopack:build -- -Clean

cargo test --manifest-path core/Cargo.toml
cargo test --manifest-path cli/Cargo.toml
npm run test
```

VS Code에서는 저장소 루트를 연 뒤 `Run and Debug`에서
`PolySheet: Full Stack`을 선택하고 `F5`를 누른다. 이 구성은 Vite 개발 서버,
Tauri Rust 프로세스, WebView2의 Svelte 디버거를 함께 시작한다.
Rust 중단점은 `polysheet/src-tauri/`와 `polysheet/core/`에, 프런트엔드 중단점은
`polysheet/src/App.svelte` 등에 설정할 수 있다. 개별 프로세스만 확인하려면
`PolySheet: Rust (Tauri)` 또는 `PolySheet: Svelte (WebView2)` 구성을 사용한다.
Rust 디버깅에는 VS Code의 CodeLLDB 확장이 필요하다.

Rust 코어는 `polysheet/core`, CLI는 `polysheet/cli`, Tauri command 계층은
`polysheet/src-tauri`에 있다. UI와 CLI는 serialization, validation, diff,
merge, transaction, Git 로직을 직접 복제하지 않고 `polysheet-core`만 사용한다.

Windows 정식 배포 패키지는 PATH에서 실행 가능한 Velopack CLI `vpk 1.2.0`으로
만든다. 패키징 전 `vpk --version`으로 설치와 버전을 확인한다.
`npm run velopack:build`는 먼저 Tauri x64 release를 bundle 없이 빌드하고,
`PolyGen.PolySheet` 패키지 ID와 `win10-x64` runtime으로 다음 산출물을
`src-tauri/target/x86_64-pc-windows-msvc/release/velopack/`에 생성한다.

- `assets.win.json`, `RELEASES`: Velopack 릴리스 메타데이터
- `PolyGen.PolySheet-win-Setup.exe`: 사용자별 one-click 설치 프로그램
- `PolyGen.PolySheet-<version>-full.nupkg`: 전체 업데이트 패키지
- `releases.win.json`: 업데이트 feed 인덱스

설치 위치는 기본적으로 `%LocalAppData%\PolyGen.PolySheet`이며 바탕 화면과
시작 메뉴에 `PolySheet` 바로가기를 만든다. 현재 Rust/Tauri 앱에는 Velopack
클라이언트 SDK를 연결하지 않았으므로 `--skipVeloAppCheck`로 패키징하되,
설치·업데이트·제거 시 전달되는 `--veloapp-*` fast-exit hook은 Rust 진입점에서
UI를 열지 않고 즉시 처리한다. 따라서 설치와 향후 update package 생성 형식은
지원하지만, 앱 내부의 업데이트 확인·다운로드·재시작 UI는 후속 기능이다.
정식 외부 배포 전에는 실행 파일과 설치 프로그램에 코드 서명을 적용해야 한다.

## v1 범위

편집 가능한 데이터 시트는 단일 JSON 파일 source만 지원한다. `@readonly`,
wildcard/directory JSON, CSV-only source의 스키마 정의는 확인할 수 있지만 해당
source를 데이터 시트로 열거나 편집할 수 없다. Excel-DNA, XLSX 호환, 차트, 피벗,
실시간 협업, 100만 행, CSV 직접 편집, 자동 Git merge driver는 v1 범위가 아니다.
Univer의 기본 시트 bar, 구조 변경 context menu, 서식 toolbar는 stable ID와
`format.json` 보존 경로가 완성될 때까지 숨기며 관련 command도 실행 전에 차단한다.
시트·행·열 구조는 PolySheet가 제공하는 stable-ID 전용 동작으로만 변경한다.
