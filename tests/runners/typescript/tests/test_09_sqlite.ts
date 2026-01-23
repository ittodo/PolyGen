// Test Case 09: SQLite Accessor
// Tests SQLite database accessor type generation
// Type-check only - validates generated types are correct

import { TestSqlite } from '../generated/09_sqlite/typescript/schema';
import { UserTable, PostTable, CommentTable, SqliteDb, DbTable } from '../generated/09_sqlite/typescript/schema_sqlite_accessor';

// Test entity type creation
function testEntityCreation(): void {
    console.log('  Testing entity creation...');

    const user: TestSqlite.User = {
        id: 1,
        name: 'TestUser',
        email: 'test@example.com',
        createdAt: 1700000000
    };

    console.assert(user.id === 1, 'user.id');
    console.assert(user.name === 'TestUser', 'user.name');
    console.assert(user.email === 'test@example.com', 'user.email');
    console.assert(user.createdAt === 1700000000, 'user.createdAt');

    const post: TestSqlite.Post = {
        id: 1,
        userId: 1,
        title: 'Test Title',
        content: 'Test Content'
    };

    console.assert(post.id === 1, 'post.id');
    console.assert(post.userId === 1, 'post.userId');
    console.assert(post.title === 'Test Title', 'post.title');

    const comment: TestSqlite.Comment = {
        id: 1,
        postId: 1,
        userId: 2,
        content: 'Test Comment'
    };

    console.assert(comment.id === 1, 'comment.id');
    console.assert(comment.postId === 1, 'comment.postId');
    console.assert(comment.userId === 2, 'comment.userId');

    console.log('    PASS');
}

// Test PostStatus enum
function testEnum(): void {
    console.log('  Testing PostStatus enum...');

    console.assert(TestSqlite.PostStatus.Draft === 0, 'PostStatus.Draft should be 0');
    console.assert(TestSqlite.PostStatus.Published === 1, 'PostStatus.Published should be 1');
    console.assert(TestSqlite.PostStatus.Archived === 2, 'PostStatus.Archived should be 2');

    console.log('    PASS');
}

// Test optional fields
function testOptionalFields(): void {
    console.log('  Testing optional fields...');

    // User with optional email undefined
    const userNoEmail: TestSqlite.User = {
        id: 2,
        name: 'Bob',
        createdAt: 1700000001
    };
    console.assert(userNoEmail.email === undefined, 'email should be undefined');

    // Post with optional content undefined
    const postNoContent: TestSqlite.Post = {
        id: 2,
        userId: 1,
        title: 'No Content Post'
    };
    console.assert(postNoContent.content === undefined, 'content should be undefined');

    console.log('    PASS');
}

// Test table accessor types
function testTableAccessorTypes(): void {
    console.log('  Testing table accessor types...');

    // These are type checks - we can't actually run SQLite in type-check mode
    // but we can verify the types compile correctly
    const userTable: UserTable = new UserTable();
    const postTable: PostTable = new PostTable();
    const commentTable: CommentTable = new CommentTable();

    // Verify DbTable<T> properties
    console.assert(typeof userTable.length === 'number', 'length should be number');
    console.assert(typeof userTable.isEmpty === 'boolean', 'isEmpty should be boolean');
    console.assert(Array.isArray(userTable.all), 'all should be array');

    console.log('    PASS');
}

// Test SqliteDb type
function testSqliteDbType(): void {
    console.log('  Testing SqliteDb type...');

    const db: SqliteDb = new SqliteDb();

    // Verify table accessors exist
    console.assert(db.users instanceof UserTable, 'users should be UserTable');
    console.assert(db.posts instanceof PostTable, 'posts should be PostTable');
    console.assert(db.comments instanceof CommentTable, 'comments should be CommentTable');

    // Verify isOpen property
    console.assert(typeof db.isOpen === 'boolean', 'isOpen should be boolean');
    console.assert(db.isOpen === false, 'isOpen should be false initially');

    console.log('    PASS');
}

// Main
console.log('=== Test Case 09: SQLite Accessor ===');
testEntityCreation();
testEnum();
testOptionalFields();
testTableAccessorTypes();
testSqliteDbType();
console.log('=== All tests passed! ===');
