# Sources Config Example

> 최종 업데이트: 2026-06-03

런타임 CSV/JSON load path를 `.poly`에서 분리하는 예제입니다.

## Run

```powershell
.\examples\run-feature-example.ps1 -Feature sources
```

직접 실행:

```powershell
.\polygen.exe --schema-path examples\features\sources.poly --sources examples\features\sources.sources.toml --lang csharp --output-dir output\features\sources
```

`--sources`를 생략해도 `examples\features\sources.sources.toml`을 자동으로 찾습니다.

## Schema

```poly
namespace examples.sources {
    table Item {
        id: u32 primary_key;
        sku: string unique max_length(32);
        name: string max_length(80);
        price: u32 default(0);
    }
}
```

## Sources

```toml
[tables."examples.sources.Item".load]
csv = "examples/features/data/items.csv"
```

## What It Shows

- `.poly`는 schema shape만 유지
- CSV load path는 sidecar TOML에 저장
- table FQN으로 load source를 연결

## Files

- [sources.poly](../../examples/features/sources.poly)
- [sources.sources.toml](../../examples/features/sources.sources.toml)
- [items.csv](../../examples/features/data/items.csv)
- [Sources config spec](../sources-config.md)
- [Feature index](features.md)
