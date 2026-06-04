# Basic Schema Example

> 최종 업데이트: 2026-06-03

이 예제는 `.poly` 스키마 하나에서 언어 모델, 로더, 인덱스, DB 출력으로 확장되는
기본 흐름을 보여줍니다.

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

## Generate Code

Optional runtime load paths live beside the schema:

```toml
[tables."demo.catalog.Item".load]
csv = "examples/data/items.csv"
```

```bash
./polygen --schema-path examples/quickstart.poly --sources examples/quickstart.sources.toml --lang csharp --output-dir output/quickstart
./polygen --schema-path examples/quickstart.poly --sources examples/quickstart.sources.toml --lang rust --output-dir output/quickstart-rust
./polygen --schema-path examples/quickstart.poly --sources examples/quickstart.sources.toml --lang sqlite --output-dir output/quickstart-sqlite
```

## What To Expect

- C#/Rust model types for `Item` and `Rarity`
- validation metadata from `primary_key`, `max_length`, `default`, and `range`
- index metadata from `@index(rarity)`
- loader metadata from `schema.sources.toml`
- SQLite DDL when generating the `sqlite` target

## Related Docs

- [Schema annotations](../schema-annotations.md)
- [Quickstart](quickstart.md)
- [Sources config](../sources-config.md)
- [Targets](../targets/README.md)
- [PolyTemplate guide](../polytemplate-guide.md)
