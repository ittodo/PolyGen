import sqlite3

from .schema_sqlite_accessor import SqliteDb


def test_sqlite_accessor_loads_and_gets_rows() -> None:
    connection = sqlite3.connect(":memory:")
    connection.executescript(
        """
        CREATE TABLE test_sqlite_User (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            email TEXT,
            created_at INTEGER NOT NULL
        );
        CREATE TABLE test_sqlite_Post (
            id INTEGER PRIMARY KEY,
            user_id INTEGER NOT NULL,
            title TEXT NOT NULL,
            content TEXT
        );
        CREATE TABLE test_sqlite_Comment (
            id INTEGER PRIMARY KEY,
            post_id INTEGER NOT NULL,
            user_id INTEGER NOT NULL,
            content TEXT NOT NULL
        );
        CREATE TABLE test_sqlite_audit_LoginEvent (
            id INTEGER PRIMARY KEY,
            user_id INTEGER NOT NULL,
            ip_address TEXT NOT NULL
        );
        INSERT INTO test_sqlite_User (id, name, email, created_at)
            VALUES (1, 'Alice', 'alice@example.com', 10),
                   (2, 'Bob', NULL, 20);
        INSERT INTO test_sqlite_Post (id, user_id, title, content)
            VALUES (100, 1, 'Hello', NULL);
        INSERT INTO test_sqlite_Comment (id, post_id, user_id, content)
            VALUES (1000, 100, 2, 'Nice post');
        INSERT INTO test_sqlite_audit_LoginEvent (id, user_id, ip_address)
            VALUES (500, 1, '127.0.0.1');
        """
    )

    db = SqliteDb(connection)
    db.load_all()

    assert db.users.count() == 2
    assert db.posts.count() == 1
    assert db.comments.count() == 1
    assert db.login_events.count() == 1

    alice = db.get_user_by_id(1)
    assert alice is not None
    assert alice.name == "Alice"
    assert alice.email == "alice@example.com"

    bob = db.get_user_by_id(2)
    assert bob is not None
    assert bob.email is None

    missing = db.get_user_by_id(404)
    assert missing is None

    post = db.get_post_by_id(100)
    assert post is not None
    assert post.content is None


test_sqlite_accessor_loads_and_gets_rows()
