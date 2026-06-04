# Pack Embed Example

> 최종 업데이트: 2026-06-03

여러 embed 필드를 단일 문자열로 직렬화하는 `@pack` 예제입니다.

## Run

```powershell
.\examples\run-feature-example.ps1 -Feature pack
```

직접 실행:

```powershell
.\polygen.exe --schema-path examples\features\pack_embed.poly --lang csharp --output-dir output\features\pack
```

## Schema

```poly
namespace examples.pack {
    @pack(separator: ",")
    embed Position {
        x: f32;
        y: f32;
        z: f32;
    }

    table SpawnPoint {
        id: u32 primary_key;
        name: string max_length(60);
        position: Position;
    }
}
```

## What It Shows

- `@pack(separator: ",")`: packed string representation for an embed
- generated `Pack`, `Unpack`, and `TryUnpack` style helpers where supported
- compact CSV-friendly representation for value objects such as coordinates

## Files

- [pack_embed.poly](../../examples/features/pack_embed.poly)
- [Feature index](features.md)
