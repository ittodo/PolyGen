const testModules = [
    './test_01_basic_types.ts',
    './test_02_imports.ts',
    './test_03_nested_namespaces.ts',
    './test_04_inline_enums.ts',
    './test_05_embedded_structs.ts',
    './test_06_arrays_and_optionals.ts',
    './test_07_indexes.ts',
    './test_08_complex_schema.ts',
    './test_09_sqlite.ts',
    './test_10_pack_embed.ts',
];

console.assert = (condition?: boolean, ...data: unknown[]): void => {
    if (!condition) {
        const message = data.length > 0 ? data.map(String).join(' ') : 'Assertion failed';
        throw new Error(message);
    }
};

for (const testModule of testModules) {
    await import(testModule);
}

console.log(`=== Executed ${testModules.length} TypeScript test modules ===`);

export {};
