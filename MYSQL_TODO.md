# MySQL 지원 확장 계획

> 상태: 검토 중 (2026-01-21)

---

## 현재 상태

- 기본 템플릿 존재 (`templates/mysql/`)
- CREATE TABLE DDL 생성 가능
- `mysql.toml` 설정 파일 없음

---

## 확장 필요 기능

### 1. 마이그레이션 & 버전 관리

**접근법**: DB에 `.poly` 스키마 저장하여 버전 관리

```sql
CREATE TABLE _polygen_migrations (
    version INT PRIMARY KEY AUTO_INCREMENT,
    poly_schema TEXT NOT NULL,      -- .poly 파일 전체 내용
    checksum VARCHAR(64),
    applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

**워크플로우**:
1. DB에서 마지막 `poly_schema` 조회
2. 현재 `.poly` 파일과 비교 (diff)
3. 변경사항 기반 마이그레이션 SQL 생성
4. 적용 후 새 스키마 저장

### 2. 이름 변경 지원

단순 diff로는 rename 감지 불가 → 어노테이션 필요

```poly
@renamed_from("OldPlayer")
table Player {
    @renamed_from("user_name")
    name: string;
}
```

생성 SQL:
```sql
RENAME TABLE OldPlayer TO Player;
ALTER TABLE Player RENAME COLUMN user_name TO name;
```

### 3. DDL 확장

| 기능 | 문법 예시 |
|------|----------|
| 복합 인덱스 | `@index("idx_name", [name, level])` |
| 유니크 인덱스 | `@index("idx_email", [email], unique: true)` |
| 풀텍스트 | `@fulltext("ft_desc", [description])` |
| 외래키 옵션 | `@foreign_key(user_id -> User.id, on_delete: CASCADE)` |
| 파티셔닝 | `@partition(type: RANGE, column: created_at)` |
| 엔진/문자셋 | `@engine(InnoDB)`, `@charset(utf8mb4)` |
| Auto increment | `id: u32 primary_key auto_increment;` |

### 4. 쿼리 정의 (검토 필요)

```poly
query GetPlayerById(id: u32) -> Player {
    SELECT * FROM Player WHERE id = ?
}

query GetPlayersByLevel(min_level: u16, max_level: u16) -> Player[] {
    SELECT * FROM Player
    WHERE level BETWEEN ? AND ?
    ORDER BY level DESC
}
```

### 5. 뷰 (View)

```poly
view PlayerSummary {
    SELECT id, name, level,
           (SELECT COUNT(*) FROM Inventory WHERE player_id = Player.id) as item_count
    FROM Player
}
```

### 6. 저장 프로시저 / 트리거

```poly
procedure TransferGold(from_id: u32, to_id: u32, amount: u32) {
    // SQL 본문
}

trigger BeforePlayerDelete on Player BEFORE DELETE {
    INSERT INTO PlayerArchive SELECT * FROM Player WHERE id = OLD.id;
}
```

---

## 설계 고려사항

### 문제점

1. 문법이 너무 복잡해짐 - 범용 스키마 언어의 장점 상실
2. SQL 전용 기능이 너무 많음 - 거의 SQL을 다시 만드는 꼴
3. 다른 언어와 호환 안됨 - C#, Rust에는 의미 없는 문법

### 대안 A: SQL 전용 블록 분리

```poly
namespace game {
    table Player { ... }
}

// SQL 전용 섹션 (다른 언어에서는 무시)
sql {
    @index Player(name, level);
    query GetPlayer(id: u32) -> Player { ... }
    view PlayerRanking { ... }
}
```

### 대안 B: 별도 파일

```
game.poly        -- 공통 스키마
game.sql.poly    -- SQL 전용 확장
```

### 대안 C: 범위 축소

- DDL + 기본 CRUD만 생성
- 복잡한 쿼리/프로시저는 수동 작성

---

## 변경 감지 매트릭스

| 변경 유형 | 감지 방법 | 생성 SQL |
|----------|----------|----------|
| 테이블 추가 | 새 이름 | `CREATE TABLE` |
| 테이블 삭제 | 이름 사라짐 | `DROP TABLE` (경고) |
| 테이블 이름변경 | `@renamed_from` | `RENAME TABLE` |
| 컬럼 추가 | 새 필드 | `ALTER ADD COLUMN` |
| 컬럼 삭제 | 필드 사라짐 | `ALTER DROP COLUMN` (경고) |
| 컬럼 이름변경 | `@renamed_from` | `ALTER RENAME COLUMN` |
| 타입 변경 | 동일 이름, 다른 타입 | `ALTER MODIFY COLUMN` |

---

## 결정 필요 사항

- [ ] SQL 전용 문법 분리 방식 결정 (블록 vs 파일 vs 축소)
- [ ] 마이그레이션 CLI 명령어 설계
- [ ] 쿼리/뷰/프로시저 지원 범위 결정
- [ ] 파괴적 변경 처리 정책 (경고? 차단? 확인?)

---

*최종 업데이트: 2026-01-21*
