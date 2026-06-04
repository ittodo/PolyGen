# Relations And Indexes Example

> 최종 업데이트: 2026-06-03

외래 키, reverse relation alias, table-level index를 함께 보는 예제입니다.

## Run

```powershell
.\examples\run-feature-example.ps1 -Feature relations
```

직접 실행:

```powershell
.\polygen.exe --schema-path examples\features\relations_indexes.poly --lang csharp --output-dir output\features\relations
```

## Schema

```poly
namespace examples.relations {
    enum PostStatus {
        Draft = 1;
        Published = 2;
        Archived = 3;
    }

    table User {
        id: u32 primary_key;
        email: string unique max_length(120);
        display_name: string max_length(40);
    }

    @index(author_id, status)
    table Post {
        id: u32 primary_key;
        author_id: u32 foreign_key(User.id) as posts;
        status: PostStatus;
        title: string max_length(120);
    }
}
```

## What It Shows

- `foreign_key(User.id)`: reference another table's key
- `as posts`: generate reverse relation metadata
- `@index(author_id, status)`: composite table-level index
- enum field reference through `PostStatus`

## Files

- [relations_indexes.poly](../../examples/features/relations_indexes.poly)
- [Feature index](features.md)
