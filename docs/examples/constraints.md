# Constraints Example

> 최종 업데이트: 2026-06-03

필드 제약조건을 한 번에 보는 작은 예제입니다.

## Run

```powershell
.\examples\run-feature-example.ps1 -Feature constraints
```

직접 실행:

```powershell
.\polygen.exe --schema-path examples\features\constraints.poly --lang csharp --output-dir output\features\constraints
```

## Schema

```poly
namespace examples.constraints {
    table Account {
        id: u32 primary_key;
        username: string unique max_length(32) regex("^[a-z0-9_]{3,32}$");
        level: u16 default(1) range(1, 100);
        email: string? max_length(120);
    }
}
```

## What It Shows

- `primary_key`: table row identity
- `unique`: duplicate value prevention
- `max_length`: string length metadata
- `regex`: string validation pattern
- `default`: generated default value metadata
- `range`: numeric min/max metadata
- `?`: optional field cardinality

## Files

- [constraints.poly](../../examples/features/constraints.poly)
- [Feature index](features.md)
