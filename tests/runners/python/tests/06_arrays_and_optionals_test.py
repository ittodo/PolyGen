import csv
import json
import tempfile
from pathlib import Path

from .schema import (
    TestCollectionsArrayTest,
    TestCollectionsMixedTestMetadata,
    TestCollectionsOptionalTest,
    TestCollectionsTag,
    load_test_collections_array_tests_from_csv,
    load_test_collections_array_tests_from_json,
    load_test_collections_mixed_tests_from_csv,
    load_test_collections_optional_tests_from_csv,
)


def write_csv(path: Path, rows: list[dict[str, object]]) -> None:
    with path.open("w", encoding="utf-8", newline="") as handle:
        writer = csv.DictWriter(handle, fieldnames=list(rows[0].keys()))
        writer.writeheader()
        for row in rows:
            writer.writerow(row)


def test_from_dict_and_json_loader() -> None:
    with tempfile.TemporaryDirectory() as temp_dir:
        path = Path(temp_dir) / "arrays.json"
        path.write_text(
            json.dumps(
                [
                    {
                        "id": 1,
                        "int_list": [1, -2, 3],
                        "string_list": ["alpha", "beta"],
                        "float_list": [1.5, 2.25],
                        "bool_list": [True, False],
                        "tags": [{"name": "hot", "color": "red"}],
                    }
                ]
            ),
            encoding="utf-8",
        )

        rows = load_test_collections_array_tests_from_json(path)

    assert rows == [
        TestCollectionsArrayTest(
            id=1,
            int_list=[1, -2, 3],
            string_list=["alpha", "beta"],
            float_list=[1.5, 2.25],
            bool_list=[True, False],
            tags=[TestCollectionsTag(name="hot", color="red")],
        )
    ]


def test_csv_loader_parses_lists_optional_and_embeds() -> None:
    with tempfile.TemporaryDirectory() as temp_dir:
        root = Path(temp_dir)
        arrays_csv = root / "arrays.csv"
        optionals_csv = root / "optionals.csv"
        mixed_csv = root / "mixed.csv"

        write_csv(
            arrays_csv,
            [
                {
                    "id": 2,
                    "int_list": "1,-2,3",
                    "string_list": "alpha,beta",
                    "float_list": "1.5,2.25",
                    "bool_list": "yes,no,1,0",
                    "tags": json.dumps([{"name": "cold", "color": "blue"}]),
                }
            ],
        )
        write_csv(
            optionals_csv,
            [
                {
                    "id": 3,
                    "required_name": "optional-row",
                    "opt_int": "",
                    "opt_string": "present",
                    "opt_float": "3.5",
                    "opt_bool": "true",
                    "opt_tag": json.dumps({"name": "maybe", "color": "green"}),
                }
            ],
        )
        write_csv(
            mixed_csv,
            [
                {
                    "id": 4,
                    "opt_tags": json.dumps([{"name": "ui", "color": "white"}]),
                    "meta": json.dumps({"created_by": "alice", "updated_by": None, "version": 7}),
                    "history": json.dumps(
                        [
                            {"created_by": None, "updated_by": "bob", "version": 1},
                            {"created_by": "alice", "updated_by": "carol", "version": 2},
                        ]
                    ),
                }
            ],
        )

        arrays = load_test_collections_array_tests_from_csv(arrays_csv)
        optionals = load_test_collections_optional_tests_from_csv(optionals_csv)
        mixed = load_test_collections_mixed_tests_from_csv(mixed_csv)

    assert arrays[0].bool_list == [True, False, True, False]
    assert arrays[0].tags == [TestCollectionsTag(name="cold", color="blue")]
    assert optionals == [
        TestCollectionsOptionalTest(
            id=3,
            required_name="optional-row",
            opt_int=None,
            opt_string="present",
            opt_float=3.5,
            opt_bool=True,
            opt_tag=TestCollectionsTag(name="maybe", color="green"),
        )
    ]
    assert mixed[0].meta == TestCollectionsMixedTestMetadata(created_by="alice", updated_by=None, version=7)
    assert mixed[0].history[1].updated_by == "carol"


def test_loader_rejects_invalid_values() -> None:
    try:
        TestCollectionsArrayTest.from_dict(
            {
                "id": 1,
                "int_list": ["not-int"],
                "string_list": [],
                "float_list": [],
                "bool_list": [],
                "tags": [],
            }
        )
    except ValueError:
        pass
    else:
        raise AssertionError("invalid integer list item should be rejected")

    try:
        TestCollectionsOptionalTest.from_dict(
            {
                "id": 1,
                "required_name": "bad",
                "opt_int": None,
                "opt_string": None,
                "opt_float": "nan",
                "opt_bool": None,
                "opt_tag": None,
            }
        )
    except ValueError:
        pass
    else:
        raise AssertionError("non-finite optional float should be rejected")


test_from_dict_and_json_loader()
test_csv_loader_parses_lists_optional_and_embeds()
test_loader_rejects_invalid_values()


def test_binary_roundtrip_for_lists_optionals_and_embeds() -> None:
    array_row = TestCollectionsArrayTest(
        id=42,
        int_list=[1, -2, 3],
        string_list=["alpha", "beta"],
        float_list=[1.5, 2.25],
        bool_list=[True, False, True],
        tags=[TestCollectionsTag(name="hot", color="red")],
    )
    loaded_array = TestCollectionsArrayTest.from_binary(array_row.to_binary())
    assert loaded_array.id == 42
    assert loaded_array.int_list == [1, -2, 3]
    assert loaded_array.string_list == ["alpha", "beta"]
    assert loaded_array.bool_list == [True, False, True]
    assert loaded_array.tags == [TestCollectionsTag(name="hot", color="red")]

    optional_row = TestCollectionsOptionalTest(
        id=7,
        required_name="optional",
        opt_int=None,
        opt_string="present",
        opt_float=3.5,
        opt_bool=True,
        opt_tag=TestCollectionsTag(name="maybe", color="green"),
    )
    loaded_optional = TestCollectionsOptionalTest.from_binary(optional_row.to_binary())
    assert loaded_optional == optional_row


test_binary_roundtrip_for_lists_optionals_and_embeds()
