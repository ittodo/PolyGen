// Test Case 07: Indexes
// Tests primary_key, unique, and foreign_key constraints

import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
    TestIndexes,
    fromTestIndexesCategoryBinary,
    toTestIndexesCategoryBinary,
} from '../generated/07_indexes/typescript/schema';
import { SchemaContainer as InMemorySchemaContainer, ValidationException } from '../generated/07_indexes/typescript/schema_container';
import {
    SchemaBinaryRefContext,
    type SchemaContainer as BinaryRefSchemaContainer,
} from '../generated/07_indexes/typescript/schema_binary_refs';

// Test User with primary key and unique constraint
function testUserWithConstraints(): void {
    console.log("  Testing User with primary_key and unique constraints...");

    const user: TestIndexes.User = {
        id: 1,
        username: "testuser",
        email: "test@example.com",
        displayName: "Test User",
    };

    console.assert(user.id === 1, "id should be 1");
    console.assert(user.username === "testuser", "username should be testuser");
    console.assert(user.email === "test@example.com", "email should match");

    console.log("    PASS");
}

// Test Category with simple primary key
function testCategoryWithPrimaryKey(): void {
    console.log("  Testing Category with primary_key...");

    const category: TestIndexes.Category = {
        id: 1,
        name: "Technology",
        description: "Tech related posts",
        rank: 7,
        kind: TestIndexes.CategoryKind.Public,
    };

    console.assert(category.id === 1, "id should be 1");
    console.assert(category.name === "Technology", "name should be Technology");
    console.assert(category.description === "Tech related posts", "description should match");

    // Test with optional description undefined
    const category2: TestIndexes.Category = {
        id: 2,
        name: "General",
        description: undefined,
        rank: 3,
        kind: TestIndexes.CategoryKind.Internal,
    };

    console.assert(category2.description === undefined, "description should be undefined");

    console.log("    PASS");
}

// Test Post with foreign keys
function testPostWithForeignKeys(): void {
    console.log("  Testing Post with foreign_key constraints...");

    const post: TestIndexes.Post = {
        id: 1,
        title: "Hello World",
        content: "This is my first post",
        authorId: 1,      // foreign_key(User.id)
        categoryId: 1,    // foreign_key(Category.id)
    };

    console.assert(post.id === 1, "id should be 1");
    console.assert(post.authorId === 1, "authorId should be 1");
    console.assert(post.categoryId === 1, "categoryId should be 1");

    console.log("    PASS");
}

// Test Comment with multiple foreign keys
function testCommentWithMultipleForeignKeys(): void {
    console.log("  Testing Comment with multiple foreign_key constraints...");

    const comment: TestIndexes.Comment = {
        id: 1,
        postId: 1,       // foreign_key(Post.id)
        authorId: 1,     // foreign_key(User.id)
        content: "Great post!",
        parentId: undefined,  // optional self-reference
    };

    console.assert(comment.id === 1, "id should be 1");
    console.assert(comment.postId === 1, "postId should be 1");
    console.assert(comment.authorId === 1, "authorId should be 1");
    console.assert(comment.parentId === undefined, "parentId should be undefined");

    // Reply comment with parent
    const reply: TestIndexes.Comment = {
        id: 2,
        postId: 1,
        authorId: 2,
        content: "Thanks!",
        parentId: 1,
    };

    console.assert(reply.parentId === 1, "parentId should be 1");

    console.log("    PASS");
}

// Test PostTag junction table
function testPostTagJunction(): void {
    console.log("  Testing PostTag junction table...");

    const postTag: TestIndexes.PostTag = {
        postId: 1,   // foreign_key(Post.id)
        tagId: 1,    // foreign_key(Tag.id)
    };

    console.assert(postTag.postId === 1, "postId should be 1");
    console.assert(postTag.tagId === 1, "tagId should be 1");

    console.log("    PASS");
}

// Test in-memory Container search indexes
function testContainerSearch(): void {
    console.log("  Testing Container @search indexes...");

    const container = new InMemorySchemaContainer({
        Users: [
            {
                id: 1,
                username: "alice",
                email: "alice@example.com",
                displayName: "Alice",
            },
        ],
        Categorys: [
            {
                id: 1,
                name: "Technology",
                description: "Binary reference systems",
                rank: 7,
                kind: TestIndexes.CategoryKind.Public,
            },
            {
                id: 2,
                name: "General",
                description: "General updates",
                rank: 3,
                kind: TestIndexes.CategoryKind.Internal,
            },
        ],
        Posts: [
            {
                id: 1,
                title: "Binary Reference Guide",
                content: "Lazy indexed rows",
                authorId: 1,
                categoryId: 1,
            },
            {
                id: 2,
                title: "General Indexes",
                content: "Secondary indexes",
                authorId: 1,
                categoryId: 2,
            },
        ],
        Tags: [
            {
                id: 50,
                name: "featured",
            },
        ],
        PostTags: [
            {
                postId: 1,
                tagId: 50,
            },
        ],
    });

    const firstPost = container.Posts.all()[0];
    const firstPostTag = container.PostTags.all()[0];
    console.assert(container.Categorys.count === 2, "category table should contain seeded rows");
    console.assert(container.Categorys.getByName("Technology")?.id === 1, "unique index should return Technology");
    console.assert(container.getPostAuthor(firstPost)?.username === "alice", "post author navigation should resolve user");
    console.assert(container.getPostCategory(firstPost)?.name === "Technology", "post category navigation should resolve category");
    console.assert(container.getPostTagPost(firstPostTag)?.id === 1, "post tag post navigation should resolve post");
    console.assert(container.getPostTagTag(firstPostTag)?.name === "featured", "post tag tag navigation should resolve tag");
    console.assert(container.Categorys.searchByName(" technology ").length === 1, "name exact search should normalize");
    console.assert(container.Categorys.searchByDescription("reference").length === 1, "description token search should match");
    console.assert(container.Categorys.searchByRank(7)[0].id === 1, "rank exact search should match");
    console.assert(container.Categorys.searchByKind(TestIndexes.CategoryKind.Internal)[0].id === 2, "enum exact search should match");
    console.assert(container.Posts.searchByTitle("binary")[0].id === 1, "post title token search should match");
    console.assert(container.Posts.searchByTitle("missing").length === 0, "missing search should be empty");

    console.log("    PASS");
}

function testContainerValidation(): void {
    console.log("  Testing Container foreign key validation...");

    const valid = new InMemorySchemaContainer({
        Users: [
            {
                id: 1,
                username: "alice",
                email: "alice@example.com",
                displayName: "Alice",
            },
        ],
        Categorys: [
            {
                id: 1,
                name: "Technology",
                description: "Binary reference systems",
                rank: 7,
                kind: TestIndexes.CategoryKind.Public,
            },
        ],
        Posts: [
            {
                id: 1,
                title: "Binary Reference Guide",
                content: "Lazy indexed rows",
                authorId: 1,
                categoryId: 1,
            },
        ],
        Tags: [
            {
                id: 50,
                name: "featured",
            },
        ],
        PostTags: [
            {
                postId: 1,
                tagId: 50,
            },
        ],
    });

    const validResult = valid.validateAll();
    console.assert(validResult.isValid, "valid container should pass foreign key validation");
    console.assert(validResult.errorCount === 0, "valid container should have no validation errors");
    valid.validateOrThrow();

    const invalid = new InMemorySchemaContainer({
        Categorys: [
            {
                id: 1,
                name: "Technology",
                description: undefined,
                rank: 7,
                kind: TestIndexes.CategoryKind.Public,
            },
        ],
        Posts: [
            {
                id: 2,
                title: "Broken Post",
                content: "Missing author",
                authorId: 999,
                categoryId: 1,
            },
        ],
    });

    const invalidResult = invalid.validateAll();
    console.assert(!invalidResult.isValid, "invalid container should fail foreign key validation");
    console.assert(invalidResult.errorCount === 1, "invalid container should report one foreign key error");
    console.assert(invalidResult.errors[0].fieldName === "author_id", "validation error should identify source field");
    console.assert(invalidResult.errors[0].constraintType === "ForeignKey", "validation error should identify constraint type");

    let threw = false;
    try {
        invalid.validateOrThrow();
    } catch (error) {
        threw = error instanceof ValidationException && error.result.errorCount === 1;
    }
    console.assert(threw, "validateOrThrow should throw ValidationException with validation result");

    const duplicate = new InMemorySchemaContainer({
        Users: [
            {
                id: 1,
                username: "alice",
                email: "alice@example.com",
                displayName: "Alice",
            },
            {
                id: 1,
                username: "bob",
                email: "bob@example.com",
                displayName: "Bob",
            },
        ],
    });
    const duplicateResult = duplicate.validateAll();
    console.assert(!duplicateResult.isValid, "duplicate primary key should fail unique validation");
    console.assert(duplicateResult.errorCount === 1, "duplicate primary key should report one validation error");
    console.assert(duplicateResult.errors[0].constraintType === "Unique", "duplicate error should identify unique constraint");

    console.log("    PASS");
}

function testContainerLoadFromCsv(): void {
    console.log("  Testing Container sources config CSV load...");

    const dir = mkdtempSync(join(tmpdir(), 'polygen-ts-07-'));
    try {
        writeFileSync(
            join(dir, 'users.csv'),
            [
                'id,username,email,display_name',
                '1,alice,alice@example.com,Alice',
            ].join('\n'),
            'utf8',
        );
        writeFileSync(
            join(dir, 'categories.csv'),
            [
                'id,name,description,rank,kind',
                '1,Technology,Binary reference systems,7,Public',
                '2,General,General updates,3,2',
            ].join('\n'),
            'utf8',
        );
        writeFileSync(
            join(dir, 'posts.csv'),
            [
                'id,title,content,author_id,category_id',
                '1,Binary Reference Guide,Lazy indexed rows,1,1',
                '2,General Indexes,Secondary indexes,1,2',
            ].join('\n'),
            'utf8',
        );
        writeFileSync(join(dir, 'comments.csv'), 'id,post_id,author_id,content,parent_id\n', 'utf8');
        writeFileSync(
            join(dir, 'tags.csv'),
            [
                'id,name',
                '50,featured',
            ].join('\n'),
            'utf8',
        );
        writeFileSync(
            join(dir, 'post_tags.csv'),
            [
                'post_id,tag_id',
                '1,50',
            ].join('\n'),
            'utf8',
        );

        const container = new InMemorySchemaContainer();
        container.loadFromCsv(dir);

        const firstPost = container.Posts.all()[0];
        const firstPostTag = container.PostTags.all()[0];
        console.assert(container.Users.count === 1, "users should load from sources config CSV path");
        console.assert(container.Categorys.count === 2, "categories should load from sources config CSV path");
        console.assert(container.Categorys.getByName("Technology")?.id === 1, "loaded unique index should resolve");
        console.assert(container.getPostAuthor(firstPost)?.username === "alice", "loaded post author navigation should resolve");
        console.assert(container.getPostTagTag(firstPostTag)?.name === "featured", "loaded post tag navigation should resolve");
        console.assert(container.Categorys.searchByDescription("reference").length === 1, "loaded search postings should match");
        console.assert(container.Categorys.searchByKind(TestIndexes.CategoryKind.Internal)[0].id === 2, "loaded enum search should match numeric enum CSV");
        console.assert(container.Posts.searchByTitle("binary")[0].id === 1, "loaded post title search should match");
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }

    console.log("    PASS");
}

function testContainerLoadFromJson(): void {
    console.log("  Testing Container sources config JSON load...");

    const dir = mkdtempSync(join(tmpdir(), 'polygen-ts-07-json-'));
    try {
        writeFileSync(
            join(dir, 'users.json'),
            JSON.stringify([
                {
                    id: 1,
                    username: "alice",
                    email: "alice@example.com",
                    display_name: "Alice",
                },
            ]),
            'utf8',
        );
        writeFileSync(
            join(dir, 'categories.json'),
            JSON.stringify([
                {
                    id: 1,
                    name: "Technology",
                    description: "Binary reference systems",
                    rank: 7,
                    kind: "Public",
                },
                {
                    id: 2,
                    name: "General",
                    description: "General updates",
                    rank: 3,
                    kind: 2,
                },
            ]),
            'utf8',
        );
        writeFileSync(
            join(dir, 'posts.json'),
            JSON.stringify([
                {
                    id: 1,
                    title: "Binary Reference Guide",
                    content: "Lazy indexed rows",
                    author_id: 1,
                    category_id: 1,
                },
                {
                    id: 2,
                    title: "General Indexes",
                    content: "Secondary indexes",
                    author_id: 1,
                    category_id: 2,
                },
            ]),
            'utf8',
        );
        writeFileSync(join(dir, 'comments.json'), JSON.stringify([]), 'utf8');
        writeFileSync(
            join(dir, 'tags.json'),
            JSON.stringify([
                {
                    id: 50,
                    name: "featured",
                },
            ]),
            'utf8',
        );
        writeFileSync(
            join(dir, 'post_tags.json'),
            JSON.stringify([
                {
                    post_id: 1,
                    tag_id: 50,
                },
            ]),
            'utf8',
        );

        const container = new InMemorySchemaContainer();
        container.loadFromJson(dir);

        const firstPost = container.Posts.all()[0];
        const firstPostTag = container.PostTags.all()[0];
        console.assert(container.Users.count === 1, "users should load from sources config JSON path");
        console.assert(container.Categorys.count === 2, "categories should load from sources config JSON path");
        console.assert(container.Categorys.getByName("Technology")?.id === 1, "loaded JSON unique index should resolve");
        console.assert(container.getPostAuthor(firstPost)?.username === "alice", "loaded JSON post author navigation should resolve");
        console.assert(container.getPostTagTag(firstPostTag)?.name === "featured", "loaded JSON post tag navigation should resolve");
        console.assert(container.Categorys.searchByDescription("reference").length === 1, "loaded JSON search postings should match");
        console.assert(container.Categorys.searchByKind(TestIndexes.CategoryKind.Internal)[0].id === 2, "loaded JSON enum search should match numeric enum");
        console.assert(container.Posts.searchByTitle("binary")[0].id === 1, "loaded JSON post title search should match");
        console.assert(container.validateAll().isValid, "loaded JSON container should pass validation");
    } finally {
        rmSync(dir, { recursive: true, force: true });
    }

    console.log("    PASS");
}

// Test BinaryRef search indexes
function testBinaryRefSearch(): void {
    console.log("  Testing BinaryRef @search indexes...");

    const container: BinaryRefSchemaContainer = {
        Users: [
            {
                id: 1,
                username: "alice",
                email: "alice@example.com",
                displayName: "Alice",
            },
        ],
        Categorys: [
            {
                id: 1,
                name: "Technology",
                description: "Binary reference systems",
                rank: 7,
                kind: TestIndexes.CategoryKind.Public,
            },
        ],
        Posts: [
            {
                id: 1,
                title: "Binary Reference Guide",
                content: "Lazy indexed rows",
                authorId: 1,
                categoryId: 1,
            },
            {
                id: 2,
                title: "General Indexes",
                content: "Secondary indexes",
                authorId: 1,
                categoryId: 1,
            },
        ],
        Comments: [],
        Tags: [],
        PostTags: [],
    };

    const bytes = SchemaBinaryRefContext.saveBinary(container);
    const context = SchemaBinaryRefContext.openBinary(bytes);

    console.assert(context.Categorys.searchByName("technology").length === 1, "name exact search should match");
    console.assert(context.Categorys.searchByDescription("reference").length === 1, "description token search should match");
    console.assert(context.Categorys.searchByRank(7).length === 1, "rank exact search should match");
    console.assert(context.Categorys.searchByKind(TestIndexes.CategoryKind.Public).length === 1, "enum exact search should match");
    console.assert(context.Posts.searchByTitle("binary").length === 1, "title token search should match");
    console.assert(context.Posts.searchByTitle("missing").length === 0, "missing search should be empty");

    console.log("    PASS");
}

// Test BinaryRef invalid enum rejection
function testBinaryRefRejectsInvalidEnumWrite(): void {
    console.log("  Testing BinaryRef invalid enum write rejection...");

    const container: BinaryRefSchemaContainer = {
        Users: [
            {
                id: 1,
                username: "alice",
                email: "alice@example.com",
                displayName: "Alice",
            },
        ],
        Categorys: [
            {
                id: 1,
                name: "InvalidKind",
                description: "Invalid enum category",
                rank: 1,
                kind: 999 as TestIndexes.CategoryKind,
            },
        ],
        Posts: [],
        Comments: [],
        Tags: [],
        PostTags: [],
    };

    let failed = false;
    try {
        SchemaBinaryRefContext.saveBinary(container);
    } catch {
        failed = true;
    }
    console.assert(failed, "invalid BinaryRef enum write should fail");

    console.log("    PASS");
}

function readI32(view: DataView, cursor: { offset: number }): number {
    const value = view.getInt32(cursor.offset, true);
    cursor.offset += 4;
    return value;
}

function readU8(bytes: Uint8Array, cursor: { offset: number }): number {
    return bytes[cursor.offset++];
}

function readString(view: DataView, bytes: Uint8Array, cursor: { offset: number }): string {
    const length = readI32(view, cursor);
    const value = new TextDecoder().decode(bytes.subarray(cursor.offset, cursor.offset + length));
    cursor.offset += length;
    return value;
}

function skipI32Index(view: DataView, cursor: { offset: number }): void {
    const entryCount = readI32(view, cursor);
    cursor.offset += entryCount * 8;
}

function skipStringIndex(view: DataView, bytes: Uint8Array, cursor: { offset: number }): void {
    const entryCount = readI32(view, cursor);
    for (let i = 0; i < entryCount; i++) {
        readString(view, bytes, cursor);
        cursor.offset += 4;
    }
}

function skipStringPostings(view: DataView, bytes: Uint8Array, cursor: { offset: number }): void {
    const entryCount = readI32(view, cursor);
    for (let i = 0; i < entryCount; i++) {
        readString(view, bytes, cursor);
        const valueCount = readI32(view, cursor);
        cursor.offset += valueCount * 4;
    }
}

function skipU8Postings(view: DataView, cursor: { offset: number }): void {
    const entryCount = readI32(view, cursor);
    for (let i = 0; i < entryCount; i++) {
        cursor.offset += 1;
        const valueCount = readI32(view, cursor);
        cursor.offset += valueCount * 4;
    }
}

function skipI32Postings(view: DataView, cursor: { offset: number }): void {
    const entryCount = readI32(view, cursor);
    for (let i = 0; i < entryCount; i++) {
        cursor.offset += 4;
        const valueCount = readI32(view, cursor);
        cursor.offset += valueCount * 4;
    }
}

function patchFirstCategoryKind(bytes: Uint8Array, value: number): void {
    const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
    const cursor = { offset: 12 };
    const tableCount = readI32(view, cursor);
    for (let tableIndex = 0; tableIndex < tableCount; tableIndex++) {
        const tableName = readString(view, bytes, cursor);
        const rowCount = readI32(view, cursor);
        const rowOffsetsOffset = cursor.offset;
        cursor.offset += rowCount * 4;
        const indexCount = readI32(view, cursor);
        for (let index = 0; index < indexCount; index++) {
            const indexName = readString(view, bytes, cursor);
            readU8(bytes, cursor);
            if (indexName === "ByUsername" || indexName === "ByEmail" || indexName === "ByName") {
                skipStringIndex(view, bytes, cursor);
            } else {
                skipI32Index(view, cursor);
            }
        }
        const searchIndexCount = readI32(view, cursor);
        for (let index = 0; index < searchIndexCount; index++) {
            const searchName = readString(view, bytes, cursor);
            readString(view, bytes, cursor);
            if (searchName === "Rank") {
                skipU8Postings(view, cursor);
            } else if (searchName === "Kind") {
                skipI32Postings(view, cursor);
            } else {
                skipStringPostings(view, bytes, cursor);
            }
        }
        const rowBlockLength = readI32(view, cursor);
        const rowBlockStart = cursor.offset;
        if (tableName === "test.indexes.Category") {
            const firstRowOffset = rowBlockStart + view.getInt32(rowOffsetsOffset, true);
            const kindFieldRelativeOffset = view.getInt32(firstRowOffset + 4 + 4 * 4, true);
            view.setInt32(firstRowOffset + kindFieldRelativeOffset, value, true);
            return;
        }
        cursor.offset += rowBlockLength;
    }
    throw new Error("Category table not found in BinaryRef document.");
}

function testBinaryRefRejectsInvalidEnumRead(): void {
    console.log("  Testing BinaryRef invalid enum read rejection...");

    const container: BinaryRefSchemaContainer = {
        Users: [
            {
                id: 1,
                username: "alice",
                email: "alice@example.com",
                displayName: "Alice",
            },
        ],
        Categorys: [
            {
                id: 1,
                name: "Technology",
                description: "Binary reference systems",
                rank: 7,
                kind: TestIndexes.CategoryKind.Public,
            },
        ],
        Posts: [],
        Comments: [],
        Tags: [],
        PostTags: [],
    };

    const bytes = SchemaBinaryRefContext.saveBinary(container);
    patchFirstCategoryKind(bytes, 999);
    const context = SchemaBinaryRefContext.openBinary(bytes);

    let failed = false;
    try {
        context.Categorys.at(0).kind;
    } catch {
        failed = true;
    }
    console.assert(failed, "invalid BinaryRef enum getter should fail");

    console.log("    PASS");
}

function testGeneratedBinaryIo(): void {
    console.log("  Testing generated row Binary I/O...");

    const category: TestIndexes.Category = {
        id: 42,
        name: "Binary",
        description: "Row binary enum",
        rank: 9,
        kind: TestIndexes.CategoryKind.Internal,
    };

    const loaded = fromTestIndexesCategoryBinary(toTestIndexesCategoryBinary(category));
    console.assert(loaded.id === 42, "binary category id should roundtrip");
    console.assert(loaded.name === "Binary", "binary category name should roundtrip");
    console.assert(loaded.description === "Row binary enum", "binary optional string should roundtrip");
    console.assert(loaded.rank === 9, "binary u8 should roundtrip");
    console.assert(loaded.kind === TestIndexes.CategoryKind.Internal, "binary enum should roundtrip");

    const withoutDescription = fromTestIndexesCategoryBinary(toTestIndexesCategoryBinary({
        ...category,
        description: undefined,
        kind: TestIndexes.CategoryKind.Public,
    }));
    console.assert(withoutDescription.description === undefined, "binary empty optional string should stay undefined");
    console.assert(withoutDescription.kind === TestIndexes.CategoryKind.Public, "binary public enum should roundtrip");

    const invalid = toTestIndexesCategoryBinary(category).slice();
    new DataView(invalid.buffer, invalid.byteOffset, invalid.byteLength).setInt32(invalid.byteLength - 4, 999, true);
    let failed = false;
    try {
        fromTestIndexesCategoryBinary(invalid);
    } catch {
        failed = true;
    }
    console.assert(failed, "invalid row binary enum discriminant should fail");

    console.log("    PASS");
}

// Main
console.log("=== Test Case 07: Indexes ===");
testUserWithConstraints();
testCategoryWithPrimaryKey();
testPostWithForeignKeys();
testCommentWithMultipleForeignKeys();
testPostTagJunction();
testContainerSearch();
testContainerValidation();
testContainerLoadFromCsv();
testContainerLoadFromJson();
testBinaryRefSearch();
testBinaryRefRejectsInvalidEnumWrite();
testBinaryRefRejectsInvalidEnumRead();
testGeneratedBinaryIo();
console.log("=== All tests passed! ===");
