import csv
import json
import struct
import tempfile
from pathlib import Path

from .schema import (
    TestIndexesCategory,
    TestIndexesCategoryKind,
    TestIndexesPost,
    TestIndexesPostTag,
    TestIndexesTag,
    TestIndexesUser,
    load_test_indexes_categorys_from_csv,
    load_test_indexes_categorys_from_json,
)
from .schema_binary_refs import BinaryRefDocument, SchemaBinaryRefContext, SchemaContainer as BinaryRefContainer
from .schema_container import SchemaContainer, ValidationException


def write_csv(path: Path, rows: list[dict[str, object]]) -> None:
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(rows[0].keys()))
        writer.writeheader()
        for row in rows:
            writer.writerow(row)


def read_i32(data: bytes | bytearray, offset: int) -> int:
    return struct.unpack_from("<i", data, offset)[0]


def patch_category_kind_payload(document: BinaryRefDocument, value: int) -> bytes:
    data = bytearray(document.to_bytes())
    row_offset = document.categorys.at(0)._row_offset
    kind_field_index = 4
    kind_relative_offset = read_i32(data, row_offset + 4 + kind_field_index * 4)
    if kind_relative_offset < 0:
        raise AssertionError("category kind field should be present")
    struct.pack_into("<i", data, row_offset + kind_relative_offset, value)
    return bytes(data)


def test_indexes_and_foreign_keys() -> None:
    container = SchemaContainer()
    user = TestIndexesUser(id=1, username="alice", email="alice@example.com", display_name="Alice")
    category = TestIndexesCategory(
        id=10,
        name="Technology",
        description="Binary reference systems",
        rank=7,
        kind=TestIndexesCategoryKind.Public,
    )
    post = TestIndexesPost(id=100, title="Binary Reference Guide", content="body", author_id=1, category_id=10)
    tag = TestIndexesTag(id=200, name="featured")
    post_tag = TestIndexesPostTag(post_id=100, tag_id=200)

    container.users.add_row(user)
    container.categorys.add_row(category)
    container.posts.add_row(post)
    container.tags.add_row(tag)
    container.post_tags.add_row(post_tag)

    assert container.users.get_by_username("alice") is user
    assert container.categorys.get_by_name("Technology") is category
    assert container.posts.find_by_author_id(1) == [post]
    assert container.post_tags.find_by_tag_id(200) == [post_tag]
    assert container.get_post_author(post) is user
    assert container.get_post_category(post) is category
    assert container.get_post_tag_post(post_tag) is post
    assert container.get_post_tag_tag(post_tag) is tag
    assert container.categorys.search_by_name(" technology ") == [category]
    assert container.categorys.search_by_description("reference") == [category]
    assert container.categorys.search_by_rank(7) == [category]
    assert container.categorys.search_by_kind(TestIndexesCategoryKind.Public) == [category]
    assert container.posts.search_by_title("binary") == [post]
    assert container.posts.search_by_title("missing") == []
    assert container.validate_all().is_valid()
    container.validate_or_raise()


def test_foreign_key_validation_rejects_missing_reference() -> None:
    container = SchemaContainer()
    container.users.add_row(TestIndexesUser(id=1, username="alice", email="alice@example.com", display_name="Alice"))
    container.posts.add_row(TestIndexesPost(id=100, title="bad", content="body", author_id=1, category_id=999))

    result = container.validate_all()
    assert not result.is_valid()
    assert result.error_count() == 1
    assert result.errors[0].field_name == "category_id"

    try:
        container.validate_or_raise()
    except ValidationException as exc:
        assert exc.result is result or exc.result.error_count() == 1
    else:
        raise AssertionError("validate_or_raise should reject missing foreign key")


def test_clear_resets_indexes() -> None:
    container = SchemaContainer()
    user = TestIndexesUser(id=1, username="alice", email="alice@example.com", display_name="Alice")
    container.users.add_row(user)
    assert container.users.get_by_id(1) is user
    container.clear()
    assert container.users.count() == 0
    assert container.users.get_by_id(1) is None


def test_enum_csv_json_loaders() -> None:
    with tempfile.TemporaryDirectory() as temp_dir:
        root = Path(temp_dir)
        json_path = root / "categories.json"
        csv_path = root / "categories.csv"

        json_path.write_text(
            json.dumps(
                [
                    {
                        "id": 1,
                        "name": "Public",
                        "description": "Visible category",
                        "rank": 5,
                        "kind": "Public",
                    }
                ]
            ),
            encoding="utf-8",
        )
        write_csv(
            csv_path,
            [
                {
                    "id": "2",
                    "name": "Internal",
                    "description": "Private category",
                    "rank": "8",
                    "kind": "2",
                }
            ],
        )

        from_json = load_test_indexes_categorys_from_json(json_path)
        from_csv = load_test_indexes_categorys_from_csv(csv_path)

    assert from_json == [
        TestIndexesCategory(
            id=1,
            name="Public",
            description="Visible category",
            rank=5,
            kind=TestIndexesCategoryKind.Public,
        )
    ]
    assert from_csv == [
        TestIndexesCategory(
            id=2,
            name="Internal",
            description="Private category",
            rank=8,
            kind=TestIndexesCategoryKind.Internal,
        )
    ]


def test_container_load_from_sources_csv() -> None:
    with tempfile.TemporaryDirectory() as temp_dir:
        root = Path(temp_dir)
        write_csv(
            root / "users.csv",
            [{"id": 1, "username": "alice", "email": "alice@example.com", "display_name": "Alice"}],
        )
        write_csv(
            root / "categories.csv",
            [{"id": 10, "name": "Technology", "description": "Binary reference systems", "rank": 7, "kind": "Public"}],
        )
        write_csv(
            root / "posts.csv",
            [{"id": 100, "title": "Binary Reference Guide", "content": "body", "author_id": 1, "category_id": 10}],
        )
        write_csv(
            root / "comments.csv",
            [{"id": 1000, "post_id": 100, "author_id": 1, "content": "nice", "parent_id": ""}],
        )
        write_csv(root / "tags.csv", [{"id": 200, "name": "featured"}])
        write_csv(root / "post_tags.csv", [{"post_id": 100, "tag_id": 200}])

        container = SchemaContainer()
        container.load_from_csv(root)

    post = container.posts.get_by_id(100)
    assert post is not None
    assert container.users.get_by_username("alice") is not None
    assert container.categorys.search_by_name(" technology ") == [container.categorys.get_by_id(10)]
    assert container.posts.search_by_title("binary") == [post]
    assert container.get_post_author(post).username == "alice"
    assert container.validate_all().is_valid()


def test_container_load_from_sources_json() -> None:
    with tempfile.TemporaryDirectory() as temp_dir:
        root = Path(temp_dir)
        (root / "users.json").write_text(
            json.dumps(
                [{"id": 1, "username": "alice", "email": "alice@example.com", "display_name": "Alice"}]
            ),
            encoding="utf-8",
        )
        (root / "categories.json").write_text(
            json.dumps(
                [
                    {
                        "id": 10,
                        "name": "Technology",
                        "description": "Binary reference systems",
                        "rank": 7,
                        "kind": "Public",
                    }
                ]
            ),
            encoding="utf-8",
        )
        (root / "posts.json").write_text(
            json.dumps(
                [{"id": 100, "title": "Binary Reference Guide", "content": "body", "author_id": 1, "category_id": 10}]
            ),
            encoding="utf-8",
        )
        (root / "comments.json").write_text(
            json.dumps([{"id": 1000, "post_id": 100, "author_id": 1, "content": "nice", "parent_id": None}]),
            encoding="utf-8",
        )
        (root / "tags.json").write_text(json.dumps([{"id": 200, "name": "featured"}]), encoding="utf-8")
        (root / "post_tags.json").write_text(json.dumps([{"post_id": 100, "tag_id": 200}]), encoding="utf-8")

        container = SchemaContainer()
        container.load_from_json(root)

    post = container.posts.get_by_id(100)
    assert post is not None
    assert container.users.get_by_username("alice") is not None
    assert container.categorys.search_by_name(" technology ") == [container.categorys.get_by_id(10)]
    assert container.posts.search_by_title("binary") == [post]
    assert container.get_post_author(post).username == "alice"
    assert container.validate_all().is_valid()


def test_binary_roundtrip_and_invalid_enum_discriminant() -> None:
    category = TestIndexesCategory(
        id=10,
        name="Technology",
        description="Binary reference systems",
        rank=7,
        kind=TestIndexesCategoryKind.Public,
    )
    loaded_category = TestIndexesCategory.from_binary(category.to_binary())
    assert loaded_category == category

    post = TestIndexesPost(id=100, title="Binary Reference Guide", content="body", author_id=1, category_id=10)
    loaded_post = TestIndexesPost.from_binary(post.to_binary())
    assert loaded_post == post

    payload = bytearray(category.to_binary())
    payload[-4:] = struct.pack("<i", 99)
    try:
        TestIndexesCategory.from_binary(payload)
    except ValueError:
        pass
    else:
        raise AssertionError("invalid enum discriminant should be rejected")

    invalid_category = TestIndexesCategory(
        id=11,
        name="Invalid",
        description=None,
        rank=1,
        kind=99,
    )
    try:
        invalid_category.to_binary()
    except ValueError:
        pass
    else:
        raise AssertionError("invalid enum discriminant should be rejected on write")


def test_binary_ref_document_roundtrip_and_lazy_indexes() -> None:
    container = SchemaContainer()
    user = TestIndexesUser(id=1, username="alice", email="alice@example.com", display_name="Alice")
    category = TestIndexesCategory(
        id=10,
        name="Technology",
        description="Binary reference systems",
        rank=7,
        kind=TestIndexesCategoryKind.Public,
    )
    post = TestIndexesPost(id=100, title="Binary Reference Guide", content="body", author_id=1, category_id=10)
    tag = TestIndexesTag(id=200, name="featured")
    post_tag = TestIndexesPostTag(post_id=100, tag_id=200)

    container.users.add_row(user)
    container.categorys.add_row(category)
    container.posts.add_row(post)
    container.tags.add_row(tag)
    container.post_tags.add_row(post_tag)

    document = BinaryRefDocument.from_container(container)
    assert document.users.count() == 1
    assert document.users.get_by_id(1).get() == user
    assert document.users.get_by_username("alice").get() == user
    assert document.posts.find_by_author_id(1)[0].get() == post

    reopened = BinaryRefDocument.from_bytes(document.to_bytes())
    category_ref = reopened.categorys.get_by_name("Technology")
    assert category_ref is not None
    assert category_ref.get() == category
    assert [item.get() for item in reopened.categorys.search_by_name(" technology ")] == [category]
    assert [item.get() for item in reopened.categorys.search_by_description("reference")] == [category]
    assert [item.get() for item in reopened.categorys.search_by_rank(7)] == [category]
    assert [item.get() for item in reopened.categorys.search_by_kind(TestIndexesCategoryKind.Public)] == [category]
    assert [item.get() for item in reopened.posts.search_by_title("binary")] == [post]
    assert reopened.posts.search_by_title("missing") == []
    assert reopened.tags.get_by_id(200).get() == tag
    assert reopened.post_tags.count() == 1

    invalid_container = BinaryRefContainer(
        categorys=[
            TestIndexesCategory(
                id=11,
                name="Invalid",
                description=None,
                rank=1,
                kind=99,
            )
        ]
    )
    try:
        SchemaBinaryRefContext.save_binary(invalid_container)
    except ValueError:
        pass
    else:
        raise AssertionError("invalid BinaryRef enum discriminant should be rejected on write")

    patched = BinaryRefDocument.from_bytes(patch_category_kind_payload(document, 99))
    try:
        _ = patched.categorys.at(0).kind
    except ValueError:
        pass
    else:
        raise AssertionError("invalid BinaryRef enum discriminant should be rejected on lazy read")


test_indexes_and_foreign_keys()
test_foreign_key_validation_rejects_missing_reference()
test_clear_resets_indexes()
test_enum_csv_json_loaders()
test_container_load_from_sources_csv()
test_container_load_from_sources_json()
test_binary_roundtrip_and_invalid_enum_discriminant()
test_binary_ref_document_roundtrip_and_lazy_indexes()
