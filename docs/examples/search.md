# Search Example

> 최종 업데이트: 2026-06-03

C# Container/BinaryRef에서 사용할 field-level search metadata 예제입니다.

## Run

```powershell
.\examples\run-feature-example.ps1 -Feature search
```

직접 실행:

```powershell
.\polygen.exe --schema-path examples\features\search.poly --lang csharp --output-dir output\features\search
```

## Schema

```poly
namespace examples.search {
    enum ItemKind {
        Weapon = 1;
        Armor = 2;
        Consumable = 3;
    }

    table Item {
        id: u32 primary_key;

        @search
        name: string max_length(80);

        @search(n: 3, normalize: lower_trim)
        description: string?;

        @search(mode: exact)
        kind: ItemKind;
    }
}
```

## What It Shows

- `@search` on string fields
- n-gram search with `n: 3`
- normalization with `normalize: lower_trim`
- exact search for enum/scalar fields

## Files

- [search.poly](../../examples/features/search.poly)
- [Feature index](features.md)
