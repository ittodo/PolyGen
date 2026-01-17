# static/ - Agent Documentation

## Scope
생성된 코드에서 사용되는 정적(static) C# 파일들이 위치한 폴더입니다. 이 파일들은 템플릿에서 생성되는 것이 아니라, 미리 작성된 코드로서 생성된 코드의 기반을 제공합니다.

## Structure
```
static/
└── csharp/                    # C# 정적 파일
    ├── DataSource.cs           # 데이터 소스 관리
    ├── BinaryUtils.cs          # 바이너리 입출력 유틸리티
    ├── CsvUtils.cs             # CSV 입출력 유틸리티
    ├── JsonUtils.cs            # JSON 입출력 유틸리티
    ├── JsonCsvConverter.cs     # JSON ↔ CSV 변환기
    └── PolygenAttributes.cs    # Polygen 어트리뷰트 정의
```

## Files

### DataSource.cs
- **크기**: 8.1KB
- **용도**: 다양한 데이터 소스에서 데이터를 로드하는 기능 제공
- **주요 기능**:
  - `DataSource` 클래스: 데이터 소스 관리
  - `LoadFromCsv()`: CSV 파일에서 데이터 로드
  - `LoadFromJson()`: JSON 데이터에서 로드
  - 데이터 캐싱 및 관리
- **네임스페이스**: `Polygen.Common`

### BinaryUtils.cs
- **크기**: 3.9KB
- **용도**: 바이너리 데이터 직렬화/역직렬화 유틸리티
- **주요 기능**:
  - `ReadByte()`, `WriteByte()`: 바이트 단위 입출력
  - `ReadInt16()`, `WriteInt16()`: 16비트 정수 입출력
  - `ReadInt32()`, `WriteInt32()`: 32비트 정수 입출력
  - `ReadFloat()`, `WriteFloat()`: 부동소수점 입출력
  - 엔디언 처리
- **네임스페이스**: `Polygen.Common`

### CsvUtils.cs
- **크기**: 3.8KB
- **용도**: CSV 데이터 입출력 유틸리티
- **주요 기능**:
  - `ReadLine()`: CSV 라인 읽기
  - `ParseCsvLine()`: CSV 라인 파싱
  - `EscapeField()`: 필드 이스케이프 처리
  - 콤마, 따옴표, 개행문자 처리
- **네임스페이스**: `Polygen.Common`

### JsonUtils.cs
- **크기**: 3.5KB
- **용도**: JSON 데이터 입출력 유틸리티
- **주요 기능**:
  - `ParseJson()`: JSON 문자열 파싱
  - `ToJson()`: 객체를 JSON으로 변환
  - `JsonPath` 지원
- **네임스페이스**: `Polygen.Common`

### JsonCsvConverter.cs
- **크기**: 12KB
- **용도**: JSON과 CSV 간의 변환 기능
- **주요 기능**:
  - `JsonToCsv()`: JSON 데이터를 CSV로 변환
  - `CsvToJson()`: CSV 데이터를 JSON으로 변환
  - 중첩 구조 처리 (임베드, 리스트)
  - 열거형 값 매핑
- **사용 예제**: `examples/JsonToCsvDemo.cs`
- **네임스페이스**: `Polygen.Common`
- **관련 문서**: `docs/json-to-csv-conversion-spec.md`

### PolygenAttributes.cs
- **크기**: 2.7KB
- **용도**: Polygen에서 사용하는 C# 어트리뷰트 정의
- **주요 기능**:
  - `[PolygenField]`: 필드 메타데이터
  - `[PolygenIgnore]`: 코드 생성에서 제외
  - `[PolygenPrimary]`: 기본 키 필드
  - `[PolygenRequired]`: 필수 필드
- **네임스페이스**: `Polygen.Common`

## Key Concepts

### 정적 파일 vs 생성 파일
- **정적 파일** (`static/`): 미리 작성된 코드, 공통 유틸리티
- **생성 파일** (`templates/`에서 생성): 스키마에 따라 동적으로 생성된 코드

### 파일 복사 프로세스
```rust
// lib.rs에서 정적 파일 복사 로직
if lang == "csharp" {
    let dest_dir = lang_output_dir.join("Common");
    fs::create_dir_all(&dest_dir)?;

    let static_files = [
        (Path::new("static/csharp/DataSource.cs"), "DataSource.cs"),
        (Path::new("static/csharp/BinaryUtils.cs"), "BinaryUtils.cs"),
        // ...
    ];
    for (src, name) in static_files {
        let dest_path = dest_dir.join(name);
        fs::copy(src, &dest_path)?;
    }
}
```

### 생성된 코드에서의 사용
```csharp
// 생성된 코드에서 정적 유틸리티 사용
using Polygen.Common;

public class Player {
    // CSV에서 읽기
    public static Player FromCsv(string csvLine) {
        var fields = CsvUtils.ParseCsvLine(csvLine);
        return new Player { /* ... */ };
    }

    // 바이너리에서 읽기
    public static Player Read(BinaryReader reader) {
        return new Player {
            Id = BinaryUtils.ReadInt32(reader),
            Name = BinaryUtils.ReadString(reader)
        };
    }
}
```

## Dependencies

### 외부 의존성
- 없음 (순수 C# 코드)

### 내부 의존성
- `templates/csharp/`: 정적 파일을 사용하는 생성 코드
- `src/lib.rs`: 정적 파일 복사 로직

## Development Guidelines

### 새로운 정적 파일 추가 시
1. `static/csharp/<new_file>.cs` 생성
2. `lib.rs`의 `static_files` 배열에 추가
3. 필요한 경우 `templates/csharp/csharp_using_*.rhai`에 using 문 추가
4. 테스트: 빌드하여 사용 가능한지 확인

### 정적 파일 작성 규칙
- **네임스페이스**: `Polygen.Common` 사용
- **접근 제어**: 필요한 경우 `public`으로 설정
- **문서화**: XML 주석으로 API 문서화
- **예외 처리**: 적절한 예외 처리

### 정적 파일 vs 템플릿 선택
- **정적 파일 사용**:
  - 공통 유틸리티
  - 스키마와 무관한 기능
  - 복잡한 로직 (템플릿으로 구현하기 어려운 경우)

- **템플릿 사용**:
  - 스키마에 따라 달라지는 코드
  - 구조체/클래스 정의
  - 데이터 매퍼

### 테스트
- 정적 파일은 유닛 테스트 작성 권장
- `tests/game_schema_test/`에서 통합 테스트 가능

### 디버깅
- 생성된 코드에서 정적 파일 사용을 확인:
  ```bash
  cargo run -- --schema-path examples/game_schema.poly --lang csharp
  grep -r "using Polygen.Common" output/csharp/
  ```

### 주의사항
- **파일명**: PascalCase 사용 (예: `BinaryUtils.cs`)
- **네임스페이스**: 모든 정적 파일은 `Polygen.Common` 네임스페이스
- **복사 로직**: 정적 파일 추가 시 `lib.rs`의 복사 로직 업데이트 필요
- **의존성**: 정적 파일 간의 의존성 최소화

### 버전 관리
- 정적 파일은 git에 커밋됩니다
- 변경 시 생성된 코드의 호환성 확인 필요
- breaking change가 있을 경우 버전 번호 업데이트

### 정적 파일 확장 아이디어
1. **XmlUtils.cs**: XML 데이터 처리
2. **ExcelUtils.cs**: Excel 파일 처리
3. **NetworkUtils.cs**: 네트워크 패킷 처리
4. **CompressionUtils.cs**: 데이터 압축/압축 해제
5. **ValidationUtils.cs**: 데이터 유효성 검사
