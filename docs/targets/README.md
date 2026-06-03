# Targets

> 최종 업데이트: 2026-06-03

이 폴더는 PolyGen이 생성하는 언어, DB, descriptor 타겟의 지원 정책과 확장 가이드를 담습니다.

## 문서

| 문서 | 역할 |
|------|------|
| `language-support.md` | 새 언어/타겟 추가 절차와 언어별 구현 레벨 |
| `sql-support.md` | SQL/DB/datasource/migration 지원 상태와 설계 |

## 현재 타겟

| 분류 | 타겟 |
|------|------|
| 언어 | C#, C++, Rust, TypeScript, Go, Python, Kotlin, Swift, Unreal |
| DB/cache | SQLite, MySQL/MariaDB, PostgreSQL, Redis |
| descriptor/diagram | Protocol Buffers, MessagePack, Mermaid |

## 추가 기준

- 새 언어 또는 descriptor를 추가하면 `language-support.md`와 `../README.md`를 갱신합니다.
- DB/datasource/migration 동작을 바꾸면 `sql-support.md`를 갱신합니다.
- 특정 타겟 문서가 길어지면 `targets/<target>.md`로 분리합니다.

