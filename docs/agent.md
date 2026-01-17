# docs/ - Agent Documentation

## Scope
PolyGen 프로젝트의 추가 문서가 위치한 폴더입니다. README.md와 함께 프로젝트의 사용법, 설계, 개발 가이드 등을 제공합니다.

## Structure
```
docs/
├── json-to-csv-conversion-spec.md    # JSON → CSV 변환 사양
└── json_table_widget_proposal.md      # JSON 테이블 위젯 제안
```

## Files

### json-to-csv-conversion-spec.md
- **크기**: 6.1KB
- **용도**: JSON 데이터를 CSV 형식으로 변환하는 사양 문서
- **주요 내용**:
  - 변환 규칙 정의
  - 중첩 구조 처리 방법
  - 임베드 타입 처리
  - 리스트/배열 처리
  - 열거형 값 변환
  - 필드 순서 및 컬럼 명명
  - null/옵셔널 필드 처리
- **관련 코드**:
  - `static/csharp/JsonCsvConverter.cs`
  - `examples/JsonToCsvDemo.cs`
- **예제**:
  ```json
  // JSON 입력 예제
  {
    "id": 1,
    "name": "Player1",
    "stats": {
      "strength": 10,
      "dexterity": 15
    },
    "skills": [1, 2, 3]
  }

  // CSV 출력 예제
  id,name,stats.strength,stats.dexterity,skills[0],skills[1],skills[2]
  1,Player1,10,15,1,2,3
  ```

### json_table_widget_proposal.md
- **크기**: 8.7KB
- **용도**: JSON 기반 테이블 위젯 UI 제안서
- **주요 내용**:
  - 테이블 위젯의 UI/UX 디자인
  - JSON 데이터를 테이블로 표시하는 방법
  - 편집 기능 (실시간 업데이트)
  - 정렬, 필터링, 검색 기능
  - 데이터 유효성 검사
  - CSV/JSON 내보내기 기능
- **목적**:
  - PolyGen으로 생성된 데이터를 효율적으로 편집/관리하는 UI
  - 데이터 시트 편집기 (Excel, Google Sheets 유사)

## Key Concepts

### 문서의 목적
1. **사양 정의**: 기능의 동작과 제약조건 명시
2. **제안**: 새로운 기능의 설계 제안
3. **구현 가이드**: 개발자를 위한 구현 방법 설명
4. **사용자 가이드**: 최종 사용자를 위한 사용법 설명

### JSON → CSV 변환
- **평탄화(flatting)**: 중첩 JSON을 평탄한 CSV로 변환
- **점 표기법(dot notation)**: 중첩 필드 경로 (예: `stats.strength`)
- **배열 인덱싱**: 리스트/배열 필드 처리 (예: `skills[0]`)
- **nullable 처리**: null 값을 빈 문자열 또는 특정 마커로 변환

### 테이블 위젯 UI
- **데이터 바인딩**: JSON 데이터와 UI 동기화
- **실시간 편집**: 변경 사항 즉시 적용
- **검증**: 스키마 기반 데이터 유효성 검사
- **내보내기**: 편집된 데이터를 JSON/CSV로 저장

## Dependencies

### 외부 의존성
- 없음 (순수 마크다운 문서)

### 내부 의존성
- `examples/`: 예제 코드 참조
- `static/csharp/`: 구현된 코드 참조
- `README.md`: 메인 문서와 연동

## Development Guidelines

### 새로운 문서 추가 시
1. 파일 명명: `<feature>-<type>.md`
   - `<feature>`: 기능 이름 (예: `json-to-csv`)
   - `<type>`: 문서 유형 (예: `spec`, `proposal`, `guide`)
2. 내용 구조:
   - 개요 (Overview)
   - 사양/요구사항 (Specification/Requirements)
   - 구현 (Implementation)
   - 예제 (Examples)
   - 참고 (References)
3. README.md에 링크 추가
4. 관련 코드와 문서 연동

### 문서 작성 규칙
- **마크다운 형식**: GitHub Flavored Markdown 사용
- **코드 블록**: 언어 지정 (```csharp, ```json, ```poly 등)
- **다이어그램**: 필요한 경우 Mermaid 또는 ASCII 아트 사용
- **예제**: 구체적인 예제 포함
- **버전**: 문서 마지막에 업데이트 날짜 및 버전 명시

### 사양 문서 (spec) 작성 시
- **명확성**: 애매한 표현 피하기
- **완전성**: 모든 케이스 고려
- **테스트 가능성**: 사양이 테스트 가능해야 함
- **예제**: 다양한 예제 포함

### 제안서 (proposal) 작성 시
- **문제 정의**: 해결하려는 문제 명확히 설명
- **해결책**: 구체적인 해결책 제시
- **장단점**: 장단점 분석
- **대안**: 다른 가능한 접근법 검토

### 문서 유지보수
- **코드 변경 시**: 관련 문서 업데이트
- **기능 추가/삭제**: 문서 반영
- **버전 관리**: 변경 사항 기록

### 문서 검증
- **코드와 일치성**: 문서와 실제 코드가 일치하는지 확인
- **예제 실행**: 문서의 예제가 실행 가능한지 확인
- **피드백 반영**: 사용자 피드백 반영하여 개선

### 주의사항
- **중복 최소화**: README.md와 중복 내용 피하기
- **최신화**: 항상 최신 상태 유지
- **링크 확인**: 내외부 링크가 유효한지 확인
- **언어**: 한국어/영어 중 하나 선택 후 통일

### 문서 확장 아이디어
1. **architecture.md**: 프로젝트 아키텍처 문서
2. **performance.md**: 성능 최적화 가이드
3. **debugging.md**: 디버깅 가이드
4. **contributing.md**: 기여자 가이드
5. **migration.md**: 버전 마이그레이션 가이드
6. **api-reference.md**: API 레퍼런스
7. **language-guide.md**: 타겟 언어별 가이드
8. **faq.md**: 자주 묻는 질문

### 문서 템플릿
```markdown
# <Title>

## Overview
[개요]

## Specification
[사양 정의]

## Implementation
[구현 방법]

## Examples
[예제]

## References
[참고 자료]

---

*Updated: YYYY-MM-DD*
```

### 문서 검토 프로세스
1. **초안 작성**: 기능 개발 시작 전 초안 작성
2. **검토**: 팀 멤버 검토 및 피드백
3. **수정**: 피드백 반영
4. **최종 승인**: 검토 완료 후 최본 승인
5. **지속 업데이트**: 개발 진행에 따라 업데이트

### 문서 도구
- **마크다운 에디터**: VS Code, Typora 등
- **다이어그램**: Mermaid, draw.io, PlantUML
- **버전 관리**: Git 이력으로 문서 변경 추적
