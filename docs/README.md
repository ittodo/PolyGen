# PolyGen 문서 인덱스

> 최종 업데이트: 2026-06-03

`docs/`는 PolyGen의 스펙, 설계, 개발 가이드의 원본입니다. 개발 전에 관련 문서를
먼저 확인하고, 구현을 바꾸면 같은 작업 안에서 문서를 갱신합니다.

---

## 개발 전 필수 확인

| 작업 | 확인 문서 |
|------|-----------|
| 전체 구조 파악 | `source-structure.md`, `project-structure.md` |
| `.poly` 문법, 어노테이션, 제약조건 변경 | `schema-annotations.md` |
| PolyTemplate 문법/엔진 변경 | `polytemplate-guide.md`, `polytemplate-spec.md` |
| 템플릿 커스터마이징/Rhai helper 변경 | `template-customization.md` |
| 새 언어/타겟 추가 | `targets/language-support.md` |
| SQL, datasource, migration 변경 | `targets/sql-support.md` |
| GUI, LSP, VS Code, poly-viewer 변경 | `tools/README.md` |
| 기능 현황/남은 작업 확인 | `status.md` |

---

## 문서 역할

| 문서 | 역할 |
|------|------|
| `source-structure.md` | Rust 소스 모듈 구조와 책임 |
| `project-structure.md` | 저장소 디렉터리 구조 |
| `schema-annotations.md` | `.poly` 어노테이션/속성 스펙 |
| `polytemplate-guide.md` | PolyTemplate 작성 가이드 |
| `polytemplate-spec.md` | PolyTemplate 언어 상세 스펙 |
| `template-customization.md` | Rhai/템플릿 커스터마이징 가이드 |
| `targets/README.md` | 언어/DB/descriptor 타겟 인덱스 |
| `targets/language-support.md` | 새 언어 지원 추가 절차 |
| `targets/sql-support.md` | SQL/DB/migration 지원 상세 |
| `tools/README.md` | GUI/LSP/VS Code/poly-viewer 도구 인덱스 |
| `status.md` | 현재 기능 현황과 남은 작업 |
| `json-to-csv-spec.md` | JSON to CSV 변환 스펙 |

완료된 계획과 이전 세션 기록은 별도 문서로 보존하지 않습니다. 필요한 결과만 현재
문서에 반영하고, 과거 이력은 git history를 사용합니다.

---

## 문서 업데이트 규칙

- `README.md`는 사용자용 소개와 빠른 시작만 유지합니다.
- `AGENTS.md`는 에이전트 작업 지침만 유지합니다.
- 기능 스펙은 `docs/` 문서를 원본으로 둡니다.
- 새 문서를 추가할 때는 이 파일에 역할을 등록합니다.
- 문서를 삭제하거나 이동하면 `rg`로 깨진 참조를 확인합니다.

```bash
rg -n "삭제한_파일명|이동한_파일명" -g "*.md"
```
