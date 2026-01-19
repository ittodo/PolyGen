# 템플릿 리팩토링 계획

## 현재 상태 분석

### 파일 구조 (41개 파일, 2,072줄)
```
templates/csharp/
├── 진입점 파일 (6개)
│   ├── csharp_file.rhai              # 클래스 생성
│   ├── csharp_csv_mappers_file.rhai  # CSV 로더
│   ├── csharp_json_mappers_file.rhai # JSON 로더
│   ├── csharp_binary_readers_file.rhai
│   ├── csharp_binary_writers_file.rhai
│   └── csharp_csv_columns_file.rhai  # CSV 컬럼 정의
│
├── rhai_utils/ (4개 유틸리티)
│   ├── type_mapping.rhai     (46줄)  # 타입 매핑
│   ├── type_info.rhai        (113줄) # 열거형 탐지
│   ├── read_mapping.rhai     (39줄)  # 바이너리 읽기 표현식
│   ├── reader_helpers.rhai   (8줄)   # 중복된 헬퍼
│   └── csv_helpers.rhai      (5줄)   # 임베디드 구조체 탐색
│
├── struct/ (14개)
│   ├── csharp_logic_struct*.rhai         # 클래스 본문
│   ├── csharp_csv_mappers_struct*.rhai   # CSV 매퍼 (490줄 writer!)
│   ├── csharp_json_mappers_struct*.rhai  # JSON 매퍼
│   ├── csharp_binary_readers_struct*.rhai
│   └── csharp_binary_writers_struct*.rhai
│
└── enum/ (2개)
    └── csharp_enum*.rhai
```

---

## 주요 문제점

### 1. 타입 매핑 로직 중복 (심각)

| 위치 | 파일 | 내용 |
|------|------|------|
| Rhai 유틸 | `rhai_utils/type_mapping.rhai` | 완전한 타입 매핑 |
| Rust | `src/rhai/csharp/type_mapping.rs` | 부분적 타입 매핑 |
| 인라인 | `csharp_json_mappers_struct_read_body.rhai` | 하드코딩된 switch문 |

**문제**: 3곳에서 동일한 로직을 별도로 관리

### 2. 헬퍼 함수 중복

`type_info.rhai`와 `reader_helpers.rhai`에 중복:
- `short_name()` - 동일
- `ns_prefix()` - 동일
- `unwrap_option()` / `unwrap_list()` - 동일

### 3. 필드 타입 판별 로직 반복 (12+ 파일)

```rhai
if inner.lang_type == "string" {
    // 문자열 처리
} else if inner.is_primitive {
    // 프리미티브 처리
} else if inner.is_enum || current_struct_inline_enums.contains(...) {
    // 열거형 처리
} else {
    // 커스텀 구조체 처리
}
```

이 패턴이 CSV/JSON/Binary 읽기/쓰기 모든 파일에 반복됨

### 4. 네임스페이스 순회 패턴 반복 (5개 파일)

```rhai
for item in ns.items {
    if item.is_struct() { ... }
    else if item.is_enum() { ... }
    else if item.is_namespace() { /* 재귀 */ }
}
```

### 5. 파일 레벨 루프 패턴 반복 (6개 파일)

```rhai
for file in schema.files {
    if file.path == () || file.path == "" { continue; }
    // 파일명 변환, 콘텐츠 생성, write_file
}
```

---

## 리팩토링 목표

1. **타입 매핑 통합**: Rust로 일원화
2. **유틸리티 통합**: 중복 제거
3. **패턴 추출**: 공통 로직을 재사용 가능한 모듈로
4. **Rust-Rhai 경계 명확화**: 복잡한 로직은 Rust, 템플릿 구조는 Rhai

---

## 제안 구조

```
templates/csharp/
├── rhai_utils/
│   ├── type_utils.rhai       # 통합된 타입 유틸리티 (type_info + reader_helpers)
│   └── binary_mapping.rhai   # 통합된 바이너리 읽기/쓰기 표현식
│
├── common/                   # 새로 추가: 공통 패턴
│   ├── file_loop.rhai        # 파일 레벨 루프 패턴
│   └── namespace_loop.rhai   # 네임스페이스 순회 패턴
│
├── class/                    # 이름 변경: struct/ → class/
│   ├── class_def.rhai        # csharp_logic_struct.rhai
│   └── class_body.rhai       # csharp_logic_struct_body.rhai
│
├── loaders/                  # 새로 추가: 로더별 그룹화
│   ├── csv/
│   │   ├── mapper_struct.rhai
│   │   ├── reader.rhai
│   │   └── writer.rhai
│   ├── json/
│   │   ├── mapper_struct.rhai
│   │   ├── reader.rhai
│   │   └── writer.rhai
│   └── binary/
│       ├── reader_struct.rhai
│       └── writer_struct.rhai
│
└── enum/                     # 유지
    └── enum_def.rhai
```

---

## 작업 순서

### Phase 1: 타입 유틸리티 통합

1. `reader_helpers.rhai`를 `type_info.rhai`에 병합
2. 중복 함수 제거 (`short_name`, `ns_prefix`, `unwrap_*`)
3. `csv_helpers.rhai`의 `find_embedded_struct`를 통합
4. 파일명 변경: `type_info.rhai` → `type_utils.rhai`
5. 모든 import 업데이트

### Phase 2: 바이너리 매핑 통합

1. `read_mapping.rhai`에 write 표현식 추가
2. 파일명 변경: `read_mapping.rhai` → `binary_mapping.rhai`
3. `csharp_binary_writers_struct_body.rhai`에서 인라인 표현식 제거
4. 통합된 함수 사용하도록 업데이트

### Phase 3: 타입 매핑 Rust 이관

1. `src/rhai/csharp/type_mapping.rs` 완성
   - `global::` 접두사 처리 추가
   - `cs_map_type()` Rhai 함수 등록
2. `rhai_utils/type_mapping.rhai` 삭제
3. 모든 템플릿에서 Rust 함수 사용

### Phase 4: 공통 패턴 추출

1. `common/namespace_loop.rhai` 생성
   - 네임스페이스 순회 로직 추출
   - 콜백 기반 처리
2. 5개 네임스페이스 템플릿 단순화
3. 파일 레벨 루프는 보류 (복잡도 대비 이득 적음)

### Phase 5: 디렉토리 재구성

1. `struct/` → `class/` 이름 변경 + 파일명 단순화
2. 로더 관련 파일 `loaders/` 하위로 이동
3. 진입점 파일명 단순화
4. 모든 include/import 경로 업데이트

### Phase 6: 테스트 및 정리

1. 기존 테스트/스냅샷 확인
2. 삭제된 파일 정리
3. 문서화

---

## 최종 결과

### 파일 수 변화
| 항목 | 리팩토링 전 | 리팩토링 후 |
|------|-------------|-------------|
| 유틸리티 파일 | 5개 | 2개 ✅ |
| 클래스 템플릿 | 14개 | 13개 |
| 총 파일 수 | 41개 | 38개 |

### 코드 중복 감소
- 타입 매핑: 3곳 → 1곳 (Rust `cs_map_type()`) ✅
- 헬퍼 함수: 2곳 → 1곳 (`type_utils.rhai`) ✅
- 바이너리 표현식: 2곳 → 1곳 (`binary_mapping.rhai`) ✅

### 삭제된 파일
- `rhai_utils/csv_helpers.rhai` - type_utils.rhai로 통합
- `rhai_utils/reader_helpers.rhai` - type_utils.rhai로 통합
- `rhai_utils/type_info.rhai` - type_utils.rhai로 통합
- `rhai_utils/read_mapping.rhai` - binary_mapping.rhai로 통합
- `rhai_utils/type_mapping.rhai` - Rust cs_map_type()으로 이관

### Rust-Rhai 경계

```
┌─────────────────────────────────────────────────────────┐
│  Rust (src/rhai/csharp/)                                │
│    - type_mapping.rs: cs_map_type() 함수 등록           │
│    - loaders/csv.rs: CSV 로더 헬퍼                      │
├─────────────────────────────────────────────────────────┤
│  Rhai (templates/csharp/)                               │
│    - 템플릿 구조 및 렌더링                              │
│    - 네임스페이스/구조체 순회                           │
│    - Rust 함수 호출하여 타입 변환                       │
└─────────────────────────────────────────────────────────┘
```

---

## 주의사항

1. **하위 호환성**: 생성되는 C# 코드는 동일해야 함
2. **스냅샷 테스트**: 각 단계마다 확인
3. **단계적 진행**: 한 번에 너무 많이 변경하지 않기
4. **include 경로**: 변경 시 모든 참조 업데이트

---

## 진행 상황

| Phase | 상태 | 비고 |
|-------|------|------|
| Phase 1: 타입 유틸리티 통합 | ✅ 완료 | type_utils.rhai로 통합, 중복 함수 제거, 모든 import 업데이트 완료 |
| Phase 2: 바이너리 매핑 통합 | ✅ 완료 | binary_mapping.rhai 생성, read_mapping.rhai 삭제 |
| Phase 3: 타입 매핑 Rust 이관 | ✅ 완료 | cs_map_type() Rust 함수 구현, type_mapping.rhai 삭제 |
| Phase 4: 공통 패턴 추출 | ✅ 완료 | namespace 유틸리티 함수 추가, 템플릿 단순화 |
| Phase 5: 디렉토리 재구성 | ✅ 완료 | struct/ → class/ 변경, loaders/ 재구성은 보류 |
| Phase 6: 테스트 및 정리 | ✅ 완료 | 모든 테스트 통과 (88개), 문서 업데이트 완료 |
