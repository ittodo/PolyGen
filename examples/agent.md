# examples/ - Agent Documentation

## Scope
PolyGen의 예제 스키마 파일이 위치한 폴더입니다. 이 예제들은 프로젝트의 기능을 보여주고, 사용자가 PolyGen을 사용하는 방법을 배우는 데 도움을 줍니다.

## Structure
```
examples/
├── character_types.poly     # 캐릭터 타입 정의 예제
├── game_schema.poly         # 게임 데이터 스키마 예제
├── common_types.poly        # 공통 타입 정의
├── csv_colum.txt            # CSV 컬럼 설명 문서
└── JsonToCsvDemo.cs         # JSON → CSV 변환 데모 C# 코드
```

## Files

### character_types.poly
- **용도**: 캐릭터 시스템의 기본 타입 정의 예제
- **내용**:
  - `Player` 테이블: 플레이어 캐릭터 정보
  - `Monster` 테이블: 몬스터 정보
  - 기본 필드 타입 및 제약조건 예제
- **실행 방법**:
  ```bash
  cargo run -- --schema-path examples/character_types.poly --lang csharp
  ```

### game_schema.poly
- **용도**: 완전한 게임 데이터 스키마 예제
- **크기**: 4.9KB
- **내용**:
  - `game.core` 네임스페이스:
    - `Player` 테이블 (ID, 이름, 레벨, 경험치 등)
    - `Monster` 테이블 (몬스터 기본 정보)
    - `Item` 테이블 (아이템 정보)
    - `Skill` 테이블 (스킬 정보)
  - 관계 정의:
    - Player ↔ Skill (N:M 관계)
    - Monster ↔ Item (드랍 아이템 관계)
  - 임베드 타입:
    - `Position` (좌표)
    - `StatBlock` (능력치)
  - 열거형:
    - `Element` (속성: FIRE, ICE, LIGHTNING)
    - `Rarity` (희귀도)
- **실행 방법**:
  ```bash
  cargo run -- --schema-path examples/game_schema.poly --lang csharp
  ```

### common_types.poly
- **용도**: 여러 스키마에서 재사용 가능한 공통 타입 정의
- **크기**: 704바이트
- **내용**:
  - `game.common` 네임스페이스
  - `StatBlock` 임베드: 캐릭터 능력치 (힘, 민첩, 지능 등)
  - `Position` 임베드: 좌표 (x, y, z)
- **사용 방법**:
  - 다른 스키마에서 `import game.common.*;`로 임포트하여 사용
  - 예: `stats: game.common.StatBlock`

### csv_colum.txt
- **용도**: CSV 컬럼 설명 문서
- **내용**:
  - 생성된 CSV 파일의 컬럼 구조 설명
  - 각 컬럼의 의미와 데이터 타입

### JsonToCsvDemo.cs
- **용도**: JSON → CSV 변환 기능 데모 (C#)
- **크기**: 3.5KB
- **내용**:
  - JSON 데이터를 CSV로 변환하는 C# 예제 코드
  - 생성된 `JsonCsvConverter` 클래스 사용 방법
  - `Polygen.Common` 네임스페이스 사용
- **관련 문서**: `docs/json-to-csv-conversion-spec.md`

## Key Concepts

### 예제 스키마의 목적
1. **학습**: PolyGen 문법과 기능 학습
2. **참조**: 실제 프로젝트에서 사용할 템플릿
3. **테스트**: 기능 테스트용 입력 데이터
4. **데모**: 데모 애플리케이션에서 사용

### 파일 임포트 패턴
```poly
// 다른 파일의 타입을 임포트
import game.common.*;

// 임포트된 타입 사용
table Player {
    stats: StatBlock;  // game.common.StatBlock
}
```

### 스키마 조직화
- **character_types.poly**: 단일 개념 (캐릭터)에 집중
- **game_schema.poly**: 전체 게임 시스템 포괄
- **common_types.poly**: 재사용 가능한 공통 타입

### 스키마 파일 실행
```bash
# 기본 실행
cargo run -- --schema-path examples/game_schema.poly

# 특정 언어 지정
cargo run -- --schema-path examples/game_schema.poly --lang csharp

# 출력 디렉토리 지정
cargo run -- --schema-path examples/game_schema.poly --output-dir my_output

# 템플릿 디렉토리 지정
cargo run -- --schema-path examples/game_schema.poly --templates-dir custom_templates
```

## Dependencies

### 외부 의존성
- 없음 (순수 `.poly` 파일)

### 내부 의존성
- `src/lib.rs`: 파싱 및 코드 생성
- `templates/`: 코드 생성 템플릿
- `static/`: 정적 C# 파일

## Development Guidelines

### 새로운 예제 추가 시
1. 스키마 파일 명명: `<feature>.poly`
2. 스키마 작성:
   - 네임스페이스 구조 고려
   - 다양한 기능 활용 (임베드, 열거형, 관계 등)
   - 주석으로 설명 추가
3. 테스트:
   ```bash
   cargo run -- --schema-path examples/<new_example>.poly --lang csharp
   ```
4. README 업데이트: 예제 사용법 추가

### 예제 스키마 작성 가이드
- **명확한 이름**: 파일 이름이 내용을 명확히 설명
- **주석 사용**: 복잡한 로직에 주석 추가
- **모듈화**: 큰 스키마는 여러 파일로 분리
- **다양한 기능**: PolyGen의 다양한 기능 보여주기

### 스키마 임포트 사용
```poly
// 전체 네임스페이스 임포트
import game.common.*;

// 특정 타입 임포트
import game.common.StatBlock;

// 다른 파일 임포트
import "path/to/other_schema.poly";
```

### 주의사항
- **예제 업데이트**: 프로젝트 기능 추가/변경 시 예제도 함께 업데이트
- **실행 가능**: 모든 예제는 실행 가능해야 함
- **문서화**: 복잡한 예제는 별도의 설명 문서 추가

### 데모 실행
```powershell
# PowerShell 스크립트 사용
./rundemo.ps1

# 또는 직접 실행
cargo run -- --schema-path examples/game_schema.poly --lang csharp
cd dist/run-csharp
dotnet run
```

### 예제 스키마 검증
```bash
# 파싱 오류 확인
cargo run -- --schema-path examples/game_schema.poly

# 생성된 코드 검토
# output/csharp/ 디렉토리 확인
```

### 예제 확장 아이디어
1. **MMO 스키마**: 길드, 퀘스트, 인벤토리 시스템
2. **UI 스키마**: 창, 위젯, 버튼 정의
3. **데이터 시트 스키마**: 엑셀/CSV 기반 데이터 구조
4. **네트워크 패킷 스키마**: 패킷 구조 정의
