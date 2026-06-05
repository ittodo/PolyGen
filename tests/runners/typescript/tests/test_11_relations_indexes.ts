// Test Case 11: Relations and composite indexes
// Tests reverse relation aliases, FK navigation helpers, and composite indexes.

import { ExamplesRelations } from '../generated/11_relations_indexes/typescript/schema';
import { SchemaContainer } from '../generated/11_relations_indexes/typescript/schema_container';
import {
    SchemaBinaryRefContext,
    type SchemaContainer as BinaryRefSchemaContainer,
} from '../generated/11_relations_indexes/typescript/schema_binary_refs';

console.log("=== Test Case 11: Relations and Composite Indexes ===");

const container = new SchemaContainer({
    Users: [
        {
            id: 1,
            email: "author@example.com",
            displayName: "Author",
        },
    ],
    Posts: [
        {
            id: 10,
            authorId: 1,
            status: ExamplesRelations.PostStatus.Published,
            title: "Relations",
        },
    ],
});

const post = container.Posts.getById(10);
console.assert(post !== undefined, "post should exist");
if (post === undefined) throw new Error("post should exist");

const composite = container.Posts.findByAuthorIdStatus([
    1,
    ExamplesRelations.PostStatus.Published,
]);
console.assert(composite.length === 1, "composite index should match one post");
console.assert(composite[0].id === 10, "composite index should return the post");

const author = container.getPostAuthor(post);
console.assert(author !== undefined, "post author should resolve");
if (author === undefined) throw new Error("post author should resolve");
console.assert(author.email === "author@example.com", "post author email should match");

const posts = container.findUserPosts(author);
console.assert(posts.length === 1, "reverse relation should return one post");
console.assert(posts[0].title === "Relations", "reverse relation should return the post");

const valid = container.validateAll();
console.assert(valid.isValid, "valid relation container should pass FK validation");

const invalid = new SchemaContainer({
    Posts: [
        {
            id: 11,
            authorId: 404,
            status: ExamplesRelations.PostStatus.Draft,
            title: "Missing author",
        },
    ],
});
const invalidResult = invalid.validateAll();
console.assert(!invalidResult.isValid, "missing author should fail FK validation");
console.assert(invalidResult.errorCount === 1, "missing author should produce one FK error");

const binaryContainer: BinaryRefSchemaContainer = {
    Users: [
        {
            id: 1,
            email: "author@example.com",
            displayName: "Author",
        },
    ],
    Posts: [
        {
            id: 10,
            authorId: 1,
            status: ExamplesRelations.PostStatus.Published,
            title: "Relations",
        },
        {
            id: 11,
            authorId: 1,
            status: ExamplesRelations.PostStatus.Draft,
            title: "Draft",
        },
    ],
};

const binaryBytes = SchemaBinaryRefContext.saveBinary(binaryContainer);
const binaryContext = SchemaBinaryRefContext.openBinary(binaryBytes);
const binaryComposite = binaryContext.Posts.findByAuthorIdStatus([
    1,
    ExamplesRelations.PostStatus.Published,
]);
console.assert(binaryComposite.length === 1, "BinaryRef composite index should match one post");
console.assert(binaryComposite[0].title === "Relations", "BinaryRef composite index should return the published post");
console.assert(
    binaryContext.Posts.findByAuthorIdStatus([2, ExamplesRelations.PostStatus.Published]).length === 0,
    "BinaryRef composite index should return empty for a missing tuple",
);

console.log("=== All tests passed! ===");
