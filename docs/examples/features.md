# Feature Examples

> 최종 업데이트: 2026-06-03

기능별로 하나씩 볼 수 있는 작은 예제 모음입니다. 각 파일은 한 가지 개념을 먼저
보이도록 작게 유지합니다.

## Feature Pages

| 문서 | 보여주는 기능 |
|------|---------------|
| [constraints.md](constraints.md) | `primary_key`, `unique`, `max_length`, `regex`, `default`, `range`, optional field |
| [relations-indexes.md](relations-indexes.md) | `foreign_key(... ) as ...`, table-level `@index`, enum reference |
| [sources.md](sources.md) | `.poly`에는 schema만 두고 CSV load path는 sidecar로 분리 |
| [pack-embed.md](pack-embed.md) | `@pack` embed 직렬화 |
| [search.md](search.md) | C# Container/BinaryRef용 field-level `@search` |

## Run All

```powershell
.\examples\run-feature-example.ps1 -Feature all -OutputDir output\features
```

## Suggested Order

1. [Quickstart](quickstart.md)
2. [Constraints](constraints.md)
3. [Relations and indexes](relations-indexes.md)
4. [Sources config](sources.md)
5. [Pack embed](pack-embed.md)
6. [Search](search.md)

## Related Docs

- [Schema annotations](../schema-annotations.md)
- [Sources config](../sources-config.md)
- [Targets](../targets/README.md)
