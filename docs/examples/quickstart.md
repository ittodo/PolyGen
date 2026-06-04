# Quickstart Example

> 최종 업데이트: 2026-06-03

가장 먼저 실행해 볼 수 있는 최소 예제입니다. 스키마, CSV load source, 샘플 CSV를
모두 저장소에 포함합니다.

## Files

| 파일 | 역할 |
|------|------|
| `../../examples/quickstart.poly` | table, enum, constraint, index, datasource 예제 |
| `../../examples/quickstart.sources.toml` | CSV load path sidecar |
| `../../examples/data/items.csv` | 샘플 CSV 데이터 |
| `../../examples/run-quickstart.ps1` | Windows runtime 실행 스크립트 |

## Run With Runtime Binary

Download PolyGen from [Releases](https://github.com/ittodo/PolyGen/releases/latest), put
`polygen.exe` beside this repository or add it to `PATH`, then run:

```powershell
.\examples\run-quickstart.ps1
```

To generate several targets in one call, use separate output directories automatically:

```powershell
.\examples\run-quickstart.ps1 -Lang all -OutputDir output\quickstart
```

Or run each target directly:

```powershell
.\polygen.exe --schema-path examples\quickstart.poly --sources examples\quickstart.sources.toml --lang csharp --output-dir output\quickstart
.\polygen.exe --schema-path examples\quickstart.poly --sources examples\quickstart.sources.toml --lang rust --output-dir output\quickstart-rust
.\polygen.exe --schema-path examples\quickstart.poly --sources examples\quickstart.sources.toml --lang sqlite --output-dir output\quickstart-sqlite
```

On macOS/Linux:

```bash
./polygen --schema-path examples/quickstart.poly --sources examples/quickstart.sources.toml --lang csharp --output-dir output/quickstart
./polygen --schema-path examples/quickstart.poly --sources examples/quickstart.sources.toml --lang rust --output-dir output/quickstart-rust
./polygen --schema-path examples/quickstart.poly --sources examples/quickstart.sources.toml --lang sqlite --output-dir output/quickstart-sqlite
```

`--sources`를 생략해도 `examples/quickstart.sources.toml`을 자동으로 찾습니다.

```powershell
.\polygen.exe --schema-path examples\quickstart.poly --lang csharp --output-dir output\quickstart
```

## Run From Source

소스에서 개발 중일 때만 `cargo run`을 사용합니다.

```bash
cargo run -- --schema-path examples/quickstart.poly --sources examples/quickstart.sources.toml --lang csharp --output-dir output/quickstart
```

## Schema

```poly
@datasource("sqlite")
namespace demo.catalog {
    enum Rarity {
        Common = 1;
        Rare = 2;
        Epic = 3;
    }

    @index(rarity)
    table Item {
        id: u32 primary_key;
        name: string max_length(80);
        rarity: Rarity;
        price: u32 default(0) range(0, 999999);
    }
}
```

## Sources

```toml
[tables."demo.catalog.Item".load]
csv = "examples/data/items.csv"
```

## What To Check

- `output/quickstart/csharp/quickstart.cs`: C# model and enum
- `output/quickstart/csharp/quickstart.Container.cs`: table container and loader helpers
- `output/quickstart/sqlite/schema.sql`: SQLite DDL generated from `@datasource("sqlite")`
- `output/quickstart-sqlite/sqlite/schema.sql`: SQLite DDL when running the `sqlite` target directly

## Next

- [Basic schema example](basic-schema.md)
- [Feature examples](features.md)
- [Constraints](constraints.md)
- [Sources config](../sources-config.md)
- [Schema annotations](../schema-annotations.md)
