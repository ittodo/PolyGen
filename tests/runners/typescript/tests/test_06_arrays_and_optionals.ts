// Test Case 06: Arrays and Optionals
// Tests array and optional field types

import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
    TestCollections,
    loadTestCollectionsArrayTestsFromCsv,
    loadTestCollectionsArrayTestsFromJson,
    loadTestCollectionsMixedTestsFromCsv,
    loadTestCollectionsOptionalTestsFromCsv,
    fromTestCollectionsArrayTestBinary,
    fromTestCollectionsMixedTestBinary,
    fromTestCollectionsOptionalTestBinary,
    toTestCollectionsArrayTestBinary,
    toTestCollectionsMixedTestBinary,
    toTestCollectionsOptionalTestBinary,
} from '../generated/06_arrays_and_optionals/typescript/schema';

// Test ArrayTest with various array types
function testArrayTypes(): void {
    console.log("  Testing ArrayTest with various array types...");

    const arr: TestCollections.ArrayTest = {
        id: 1,
        intList: [1, 2, 3, 4, 5],
        stringList: ["a", "b", "c"],
        floatList: [1.1, 2.2, 3.3],
        boolList: [true, false, true],
        tags: [
            { name: "important", color: "red" },
            { name: "review", color: "yellow" },
        ],
    };

    console.assert(arr.id === 1, "id should be 1");
    console.assert(arr.intList.length === 5, "intList should have 5 elements");
    console.assert(arr.intList[0] === 1, "intList[0] should be 1");
    console.assert(arr.stringList.length === 3, "stringList should have 3 elements");
    console.assert(arr.tags.length === 2, "tags should have 2 elements");
    console.assert(arr.tags[0].name === "important", "tags[0].name should be important");

    console.log("    PASS");
}

// Test OptionalTest with optional fields
function testOptionalFields(): void {
    console.log("  Testing OptionalTest with optional fields...");

    // All optionals undefined
    const opt1: TestCollections.OptionalTest = {
        id: 1,
        requiredName: "Test",
        optInt: undefined,
        optString: undefined,
        optFloat: undefined,
        optBool: undefined,
        optTag: undefined,
    };

    console.assert(opt1.id === 1, "id should be 1");
    console.assert(opt1.requiredName === "Test", "requiredName should be Test");
    console.assert(opt1.optInt === undefined, "optInt should be undefined");

    // All optionals set
    const opt2: TestCollections.OptionalTest = {
        id: 2,
        requiredName: "Full",
        optInt: 42,
        optString: "hello",
        optFloat: 3.14,
        optBool: true,
        optTag: { name: "tag", color: "blue" },
    };

    console.assert(opt2.optInt === 42, "optInt should be 42");
    console.assert(opt2.optString === "hello", "optString should be hello");
    console.assert(opt2.optFloat === 3.14, "optFloat should be 3.14");
    console.assert(opt2.optBool === true, "optBool should be true");
    console.assert(opt2.optTag?.name === "tag", "optTag.name should be tag");

    console.log("    PASS");
}

// Test MixedTest with arrays and optionals
function testMixedArraysAndOptionals(): void {
    console.log("  Testing MixedTest with arrays and optionals...");

    const mixed: TestCollections.MixedTest = {
        id: 1,
        optTags: [
            { name: "tag1", color: "red" },
            { name: "tag2", color: "blue" },
        ],
        meta: {
            createdBy: "admin",
            updatedBy: undefined,
            version: 1,
        },
        history: [
            { createdBy: "user1", updatedBy: "user2", version: 1 },
            { createdBy: "user2", updatedBy: undefined, version: 2 },
        ],
    };

    console.assert(mixed.id === 1, "id should be 1");
    console.assert(mixed.optTags.length === 2, "optTags should have 2 elements");
    console.assert(mixed.meta?.createdBy === "admin", "meta.createdBy should be admin");
    console.assert(mixed.meta?.updatedBy === undefined, "meta.updatedBy should be undefined");
    console.assert(mixed.history.length === 2, "history should have 2 elements");

    console.log("    PASS");
}

function csvJsonCell(value: unknown): string {
    return `"${JSON.stringify(value).replace(/"/g, '""')}"`;
}

function testGeneratedLoaders(): void {
    console.log("  Testing generated CSV/JSON loaders...");

    const dir = mkdtempSync(join(tmpdir(), 'polygen-ts-06-'));
    try {
        const arrayCsv = join(dir, 'array_tests.csv');
        writeFileSync(
            arrayCsv,
            [
                'id,int_list,string_list,float_list,bool_list,tags',
                `1,"1,2,3","alpha,beta","1.5,2.5","yes,no,1",${csvJsonCell([
                    { name: 'important', color: 'red' },
                    { name: 'review', color: 'yellow' },
                ])}`,
            ].join('\n'),
            'utf8',
        );

        const optionalCsv = join(dir, 'optional_tests.csv');
        writeFileSync(
            optionalCsv,
            [
                'id,required_name,opt_int,opt_string,opt_float,opt_bool,opt_tag',
                `2,Full,42,hello,3.14,true,${csvJsonCell({ name: 'tag', color: 'blue' })}`,
                '3,Empty,,,,,',
            ].join('\n'),
            'utf8',
        );

        const mixedCsv = join(dir, 'mixed_tests.csv');
        writeFileSync(
            mixedCsv,
            [
                'id,opt_tags,meta,history',
                `4,${csvJsonCell([{ name: 'tag1', color: 'red' }])},${csvJsonCell({ created_by: 'admin', version: 1 })},${csvJsonCell([
                    { created_by: 'user1', updated_by: 'user2', version: 1 },
                    { created_by: 'user2', version: 2 },
                ])}`,
            ].join('\n'),
            'utf8',
        );

        const arrayJson = join(dir, 'array_tests.json');
        writeFileSync(
            arrayJson,
            JSON.stringify([
                {
                    id: 5,
                    int_list: [9, 8],
                    string_list: ['x', 'y'],
                    float_list: [1.25],
                    bool_list: [true, false],
                    tags: [{ name: 'json', color: 'green' }],
                },
            ]),
            'utf8',
        );

        const arraysFromCsv = loadTestCollectionsArrayTestsFromCsv(arrayCsv);
        console.assert(arraysFromCsv[0].intList[2] === 3, "CSV int list should parse");
        console.assert(arraysFromCsv[0].boolList[0] === true && arraysFromCsv[0].boolList[1] === false, "CSV bool list should parse yes/no");
        console.assert(arraysFromCsv[0].tags[1].color === 'yellow', "CSV JSON-cell embed list should parse");

        const optionalsFromCsv = loadTestCollectionsOptionalTestsFromCsv(optionalCsv);
        console.assert(optionalsFromCsv[0].optTag?.name === 'tag', "CSV optional embed should parse");
        console.assert(optionalsFromCsv[1].optInt === undefined, "empty optional scalar should be undefined");

        const mixedFromCsv = loadTestCollectionsMixedTestsFromCsv(mixedCsv);
        console.assert(mixedFromCsv[0].meta?.createdBy === 'admin', "CSV optional nested embed should parse");
        console.assert(mixedFromCsv[0].history[1].version === 2, "CSV nested embed list should parse");

        const arraysFromJson = loadTestCollectionsArrayTestsFromJson(arrayJson);
        console.assert(arraysFromJson[0].stringList[1] === 'y', "JSON array loader should parse snake_case fields");
        console.assert(arraysFromJson[0].tags[0].name === 'json', "JSON embed array should parse");
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }

    console.log("    PASS");
}

function testBinaryRoundtrip(): void {
    console.log("  Testing generated binary roundtrip...");

    const arrayRow: TestCollections.ArrayTest = {
        id: 10,
        intList: [10, 20, 30],
        stringList: ["alpha", "beta"],
        floatList: [1.5, 2.25],
        boolList: [true, false, true],
        tags: [{ name: "binary", color: "green" }],
    };
    const arrayLoaded = fromTestCollectionsArrayTestBinary(toTestCollectionsArrayTestBinary(arrayRow));
    console.assert(arrayLoaded.id === arrayRow.id, "binary array id should roundtrip");
    console.assert(arrayLoaded.intList.join(",") === "10,20,30", "binary int list should roundtrip");
    console.assert(arrayLoaded.stringList[1] === "beta", "binary string list should roundtrip");
    console.assert(Math.abs(arrayLoaded.floatList[1] - 2.25) < 0.0001, "binary f32 list should roundtrip");
    console.assert(arrayLoaded.boolList[1] === false, "binary bool list should roundtrip");
    console.assert(arrayLoaded.tags[0].name === "binary", "binary embed list should roundtrip");

    const optionalFull: TestCollections.OptionalTest = {
        id: 11,
        requiredName: "Full",
        optInt: -7,
        optString: "present",
        optFloat: 9.5,
        optBool: true,
        optTag: { name: "optional", color: "blue" },
    };
    const optionalLoaded = fromTestCollectionsOptionalTestBinary(toTestCollectionsOptionalTestBinary(optionalFull));
    console.assert(optionalLoaded.optInt === -7, "binary optional int should roundtrip");
    console.assert(optionalLoaded.optString === "present", "binary optional string should roundtrip");
    console.assert(optionalLoaded.optFloat === 9.5, "binary optional f64 should roundtrip");
    console.assert(optionalLoaded.optBool === true, "binary optional bool should roundtrip");
    console.assert(optionalLoaded.optTag?.color === "blue", "binary optional embed should roundtrip");

    const optionalEmpty: TestCollections.OptionalTest = {
        id: 12,
        requiredName: "Empty",
        optInt: undefined,
        optString: undefined,
        optFloat: undefined,
        optBool: undefined,
        optTag: undefined,
    };
    const emptyLoaded = fromTestCollectionsOptionalTestBinary(toTestCollectionsOptionalTestBinary(optionalEmpty));
    console.assert(emptyLoaded.optInt === undefined, "empty binary optional int should stay undefined");
    console.assert(emptyLoaded.optTag === undefined, "empty binary optional embed should stay undefined");

    const mixed: TestCollections.MixedTest = {
        id: 13,
        optTags: [{ name: "history", color: "purple" }],
        meta: { createdBy: "admin", updatedBy: undefined, version: 2 },
        history: [
            { createdBy: "a", updatedBy: "b", version: 1 },
            { createdBy: undefined, updatedBy: "c", version: 2 },
        ],
    };
    const mixedLoaded = fromTestCollectionsMixedTestBinary(toTestCollectionsMixedTestBinary(mixed));
    console.assert(mixedLoaded.meta?.createdBy === "admin", "binary optional nested embed should roundtrip");
    console.assert(mixedLoaded.meta?.updatedBy === undefined, "binary nested optional field should stay undefined");
    console.assert(mixedLoaded.history[1].updatedBy === "c", "binary nested embed list should roundtrip");

    console.log("    PASS");
}

// Main
console.log("=== Test Case 06: Arrays and Optionals ===");
testArrayTypes();
testOptionalFields();
testMixedArraysAndOptionals();
testGeneratedLoaders();
testBinaryRoundtrip();
console.log("=== All tests passed! ===");
