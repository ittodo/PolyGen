# templates/ - Agent Documentation

## Scope
Rhai 스크립팅 템플릿 엔진을 사용하여 다양한 타겟 언어의 코드를 생성하는 템플릿 파일들이 위치한 폴더입니다. 현재 C#과 MySQL 언어를 지원합니다.

## Structure
```
templates/
├── csharp/                    # C# 코드 생성 템플릿
│   ├── csharp_file.rhai      # 메인 C# 파일 생성
│   ├── csharp_namespace.rhai  # 네임스페이스 템플릿
│   ├── csharp_logic_namespace.rhai  # 로직 네임스페이스
│   ├── csharp_using.rhai      # using 문
│   ├── csharp_using_csv.rhai  # CSV 관련 using
│   ├── csharp_using_json.rhai  # JSON 관련 using
│   ├── csharp_using_readers.rhai  # 바이너리 리더 using
│   ├── csharp_using_writers.rhai  # 바이너리 라이터 using
│   ├── csharp_binary_readers.rhai  # 바이너리 리더
│   ├── csharp_binary_readers_file.rhai  # 바이너리 리더 파일
│   ├── csharp_binary_readers_namespace.rhai  # 바이너리 리더 네임스페이스
│   ├── csharp_binary_writers.rhai  # 바이너리 라이터
│   ├── csharp_binary_writers_file.rhai  # 바이너리 라이터 파일
│   ├── csharp_binary_writers_namespace.rhai  # 바이너리 라이터 네임스페이스
│   ├── csharp_csv_mappers.rhai  # CSV 매퍼
│   ├── csharp_csv_mappers_file.rhai  # CSV 매퍼 파일
│   ├── csharp_csv_mappers_namespace.rhai  # CSV 매퍼 네임스페이스
│   ├── csharp_csv_columns_file.rhai  # CSV 컬럼 정의 파일
│   ├── csharp_json_mappers.rhai  # JSON 매퍼
│   ├── csharp_json_mappers_file.rhai  # JSON 매퍼 파일
│   ├── csharp_json_mappers_namespace.rhai  # JSON 매퍼 네임스페이스
│   ├── struct/               # 구조체 관련 템플릿
│   │   ├── csharp_logic_struct.rhai
│   │   ├── csharp_logic_struct_header.rhai
│   │   ├── csharp_logic_struct_body.rhai
│   │   ├── csharp_binary_readers_struct.rhai
│   │   ├── csharp_binary_readers_struct.rhai
│   │   ├── csharp_binary_readers_struct_body.rhai
│   │   ├── csharp_binary_writers_struct.rhai
│   │   ├── csharp_binary_writers_struct_body.rhai
│   │   ├── csharp_csv_mappers_struct.rhai
│   │   ├── csharp_csv_mappers_struct_reader.rhai
│   │   ├── csharp_csv_mappers_struct_writer.rhai
│   │   └── csharp_json_mappers_struct.rhai
│   ├── enum/                # 열거형 관련 템플릿
│   │   ├── csharp_enum.rhai
│   │   └── csharp_enum_body.rhai
│   └── rhai_utils/          # Rhai 유틸리티 함수
│       ├── indent.rhai
│       ├── csv_helpers.rhai
│       ├── type_mapping.rhai
│       ├── type_info.rhai
│       ├── reader_helpers.rhai
│       └── read_mapping.rhai
├── mysql/                    # MySQL 코드 생성 템플릿 (초기 상태)
│   ├── mysql_file.rhai
│   ├── mysql_namespace_root.rhai
│   ├── mysql_namespace.rhai
│   ├── mysql_struct.rhai
│   └── rhai_utils/
│       └── type_mapping.rhai
├── mermaid/                  # Mermaid 다이어그램 생성 (플래그)
└── rhai_utils/              # 공통 Rhai 유틸리티
    └── indent.rhai
```

## Files

### C# 템플릿

#### 파일 레벨 템플릿
- **csharp_file.rhai**: 메인 C# 파일 생성
  - 전체 파일 구조 생성
  - 네임스페이스, 구조체, 열거형 순회
  - IR을 순회하며 코드 조립

- **csharp_namespace.rhai**: 네임스페이스 정의
  - `namespace <path>;` 생성
  - 중첩 네임스페이스 처리

- **csharp_logic_namespace.rhai**: 로직 네임스페이스
  - `Polygen.Logic` 네임스페이스
  - 구조체 생성 로직 포함

- **csharp_using.rhai**: 공통 using 문
  - `System`, `System.Collections.Generic` 등 기본 using

- **csharp_using_csv.rhai**: CSV 관련 using
  - `Polygen.Common.CsvUtils`, `Polygen.Common.DataSource` 등

- **csharp_using_json.rhai**: JSON 관련 using
  - `Polygen.Common.JsonUtils`, `Polygen.Common.JsonCsvConverter` 등

- **csharp_using_readers.rhai**: 바이너리 리더 using
  - `Polygen.Common.BinaryUtils` 등

- **csharp_using_writers.rhai**: 바이너리 라이터 using
  - `Polygen.Common.BinaryUtils` 등

#### 바이너리 입출력 템플릿
- **csharp_binary_readers.rhai**: 바이너리 리더
  - 바이너리 포맷에서 데이터를 읽는 코드 생성

- **csharp_binary_readers_file.rhai**: 바이너리 리더 파일
  - 전체 바이너리 리더 파일 구조

- **csharp_binary_readers_namespace.rhai**: 바이너리 리더 네임스페이스
  - `Polygen.BinaryReaders` 네임스페이스

- **csharp_binary_writers.rhai**: 바이너리 라이터
  - 데이터를 바이너리 포맷으로 쓰는 코드 생성

- **csharp_binary_writers_file.rhai**: 바이너리 라이터 파일
  - 전체 바이너리 라이터 파일 구조

- **csharp_binary_writers_namespace.rhai**: 바이너리 라이터 네임스페이스
  - `Polygen.BinaryWriters` 네임스페이스

#### CSV 매퍼 템플릿
- **csharp_csv_mappers.rhai**: CSV 매퍼
  - CSV 데이터를 읽고 쓰는 코드 생성

- **csharp_csv_mappers_file.rhai**: CSV 매퍼 파일
  - 전체 CSV 매퍼 파일 구조

- **csharp_csv_mappers_namespace.rhai**: CSV 매퍼 네임스페이스
  - `Csv.<namespace>` 네임스페이스

- **csharp_csv_columns_file.rhai**: CSV 컬럼 정의 파일
  - CSV 컬럼 정보 생성
  - 헤더 정의

#### JSON 매퍼 템플릿
- **csharp_json_mappers.rhai**: JSON 매퍼
  - JSON 데이터를 읽고 쓰는 코드 생성

- **csharp_json_mappers_file.rhai**: JSON 매퍼 파일
  - 전체 JSON 매퍼 파일 구조

- **csharp_json_mappers_namespace.rhai**: JSON 매퍼 네임스페이스
  - `Polygen.JsonMappers` 네임스페이스

#### 구조체 템플릿
- **csharp_logic_struct.rhai**: 논리 구조체
  - 기본 구조체 생성 (필드, 속성)

- **csharp_logic_struct_header.rhai**: 논리 구조체 헤더
  - 클래스 선언, 속성 등 헤더 부분

- **csharp_logic_struct_body.rhai**: 논리 구조체 본문
  - 필드 선언, 생성자 등

- **csharp_binary_readers_struct.rhai**: 바이너리 리더 구조체
  - 바이너리 데이터를 읽는 메서드 생성

- **csharp_binary_readers_struct_body.rhai**: 바이너리 리더 본문
  - 필드별 읽기 로직

- **csharp_binary_writers_struct.rhai**: 바이너리 라이터 구조체
  - 바이너리 데이터를 쓰는 메서드 생성

- **csharp_binary_writers_struct_body.rhai**: 바이너리 라이터 본문
  - 필드별 쓰기 로직

- **csharp_csv_mappers_struct.rhai**: CSV 매퍼 구조체
  - CSV 매퍼 클래스 생성

- **csharp_csv_mappers_struct_reader.rhai**: CSV 매퍼 리더
  - CSV에서 읽는 메서드 생성

- **csharp_csv_mappers_struct_writer.rhai**: CSV 매퍼 라이터
  - CSV에 쓰는 메서드 생성

- **csharp_json_mappers_struct.rhai**: JSON 매퍼 구조체
  - JSON 매퍼 클래스 생성

#### 열거형 템플릿
- **csharp_enum.rhai**: 열거형 생성
  - `enum` 정의

- **csharp_enum_body.rhai**: 열거형 본문
  - 열거형 값 생성

#### 유틸리티 템플릿
- **rhai_utils/indent.rhai**: 인덴트 유틸리티
  - 들여쓰기 레벨 관리

- **rhai_utils/csv_helpers.rhai**: CSV 헬퍼 함수
  - CSV 관련 유틸리티 함수

- **rhai_utils/type_mapping.rhai**: 타입 매핑
  - IR 타입 → C# 타입 변환

- **rhai_utils/type_info.rhai**: 타입 정보
  - 타입 메타데이터 처리

- **rhai_utils/reader_helpers.rhai**: 리더 헬퍼
  - 읽기 관련 헬퍼 함수

- **rhai_utils/read_mapping.rhai**: 읽기 매핑
  - 읽기 로직 매핑

### MySQL 템플릿 (초기 상태)
- **mysql_file.rhai**: MySQL 파일 생성
- **mysql_namespace_root.rhai**: 최상위 네임스페이스
- **mysql_namespace.rhai**: 네임스페이스
- **mysql_struct.rhai**: 구조체
- **rhai_utils/type_mapping.rhai**: 타입 매핑

## Key Concepts

### Rhai 템플릿 엔진
- Rust에서 임베딩 가능한 스크립팅 언어
- IR 객체를 템플릿에 주입하여 코드 생성
- `src/rhai_generator.rs`가 Rhai 엔진을 설정하고 템플릿을 실행합니다

### 템플릿 실행 흐름
1. `lib.rs`에서 타겟 언어 템플릿 파일 경로 결정
2. `rhai_generator.rs`가 IR을 Rhai 엔진에 주입
3. Rhai 엔진이 템플릿을 실행하고 코드를 생성
4. 생성된 코드를 `output/<lang>/` 디렉토리에 저장

### C# 코드 생성 패턴
- **네임스페이스**: `Polygen` 루트 하위에 기능별 네임스페이스
- **CSV 매퍼**: `Csv.<namespace>` 구조
- **바이너리 리더/라이터**: `Polygen.BinaryReaders`, `Polygen.BinaryWriters`
- **JSON 매퍼**: `Polygen.JsonMappers`

### 템플릿 파일 명명 규칙
- `<lang>_file.rhai`: 전체 파일 템플릿
- `<lang>_namespace.rhai`: 네임스페이스 템플릿
- `<lang>_<feature>_file.rhai`: 특정 기능 파일 템플릿
- `<lang>_<feature>_namespace.rhai`: 특정 기능 네임스페이스 템플릿

## Dependencies

### 외부 의존성
- **Rhai 1.22.2**: 템플릿 엔진
- **IR (ir_model.rs)**: 템플릿에 주입되는 데이터
- **Rhai Functions (src/rhai/*.rs)**: 템플릿에서 호출 가능한 커스텀 함수

### 내부 의존성
- 템플릿은 서로 참조 가능
- `rhai_utils/` 하위의 유틸리티 함수를 공유
- C# 템플릿은 `static/csharp/`의 정적 파일과 함께 사용

## Development Guidelines

### 새로운 타겟 언어 추가 시
1. `templates/<lang>/` 폴더 생성
2. `<lang>_file.rhai` 템플릿 생성 (최소 요구사항)
3. 네임스페이스, 구조체, 열거형 템플릿 생성
4. `templates/<lang>/rhai_utils/type_mapping.rhai`에 타입 매핑 정의
5. `lib.rs`에서 언어 자동 검출 로직 확인

### 템플릿 작성 규칙 (agent.md 참고)
- **백틱 문자열**: 템플릿 리터럴에 사용
- **보간**: `${expr}` 형식으로 변수 사용
- **새줄 처리**: 각 줄 끝에 `+ "\n"` 추가
- **최소 부작용**: `${...}` 내의 표현식은 단순하게 유지

### C# 코드 스타일 (생성된 코드)
- **중괄호**: Allman 스타일 (여는 중괄호 다음 줄)
- **인덴트**: 4 스페이스
- **한 줄에 한 문장**: 복합문 사용
- **공백**: 연산자 주변에 공백 추가

### 테스트
- 템플릿 변경 후 `cargo test` 실행하여 스냅샷 검증
- `tests/snapshots/`에 생성된 코드 스냅샷 저장
- 스냅샷 검토로 코드 생성 로직 확인

### 디버깅
- `output/<lang>/` 디렉토리에서 생성된 코드 확인
- `ir_debug.txt`에서 IR 구조 확인
- Rhai 템플릿의 `print()` 함수로 디버그 출력 가능

### 주의사항
- **템플릿 복잡도**: 너무 복잡한 로직은 `src/rhai/*.rs`에서 함수로 구현
- **성능**: 템플릿 내에서 무거운 연산은 피하세요
- **유지보수성**: 템플릿을 작은 단위로 분리하여 관리
