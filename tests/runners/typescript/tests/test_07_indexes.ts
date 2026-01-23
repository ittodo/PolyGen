// Test Case 07: Indexes
// Tests primary_key, unique, and foreign_key constraints

import { TestIndexes } from '../generated/07_indexes/typescript/schema';

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
    };

    console.assert(category.id === 1, "id should be 1");
    console.assert(category.name === "Technology", "name should be Technology");
    console.assert(category.description === "Tech related posts", "description should match");

    // Test with optional description undefined
    const category2: TestIndexes.Category = {
        id: 2,
        name: "General",
        description: undefined,
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

// Main
console.log("=== Test Case 07: Indexes ===");
testUserWithConstraints();
testCategoryWithPrimaryKey();
testPostWithForeignKeys();
testCommentWithMultipleForeignKeys();
testPostTagJunction();
console.log("=== All tests passed! ===");
