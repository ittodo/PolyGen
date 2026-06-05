package polygen

import (
	"context"
	"database/sql"
	"database/sql/driver"
	"io"
	"testing"

	_ "modernc.org/sqlite"
)

const fakeSqliteDriverName = "polygen_fake_sqlite"

func init() {
	sql.Register(fakeSqliteDriverName, fakeSqliteDriver{})
}

type fakeSqliteDriver struct{}

func (fakeSqliteDriver) Open(string) (driver.Conn, error) {
	return fakeSqliteConn{}, nil
}

type fakeSqliteConn struct{}

func (fakeSqliteConn) Prepare(string) (driver.Stmt, error) {
	return nil, driver.ErrSkip
}

func (fakeSqliteConn) Close() error {
	return nil
}

func (fakeSqliteConn) Begin() (driver.Tx, error) {
	return nil, driver.ErrSkip
}

func (fakeSqliteConn) QueryContext(_ context.Context, query string, args []driver.NamedValue) (driver.Rows, error) {
	rows := map[string]fakeRows{
		"SELECT id, name, email, created_at FROM test_sqlite_User": {
			columns: []string{"id", "name", "email", "created_at"},
			values: [][]driver.Value{
				{int64(1), "Alice", "alice@example.com", int64(1000)},
				{int64(2), "Bob", nil, int64(2000)},
			},
		},
		"SELECT id, user_id, title, content FROM test_sqlite_Post": {
			columns: []string{"id", "user_id", "title", "content"},
			values: [][]driver.Value{
				{int64(10), int64(1), "First", "body"},
				{int64(11), int64(2), "Second", nil},
			},
		},
		"SELECT id, post_id, user_id, content FROM test_sqlite_Comment": {
			columns: []string{"id", "post_id", "user_id", "content"},
			values: [][]driver.Value{
				{int64(100), int64(10), int64(1), "comment"},
			},
		},
		"SELECT id, user_id, ip_address FROM test_sqlite_audit_LoginEvent": {
			columns: []string{"id", "user_id", "ip_address"},
			values: [][]driver.Value{
				{int64(1000), int64(1), "127.0.0.1"},
			},
		},
	}

	if query == "SELECT id, name, email, created_at FROM test_sqlite_User WHERE id = ?" {
		id := args[0].Value.(int64)
		if id == 1 {
			return &fakeRows{
				columns: []string{"id", "name", "email", "created_at"},
				values:  [][]driver.Value{{int64(1), "Alice", "alice@example.com", int64(1000)}},
			}, nil
		}
		return &fakeRows{columns: []string{"id", "name", "email", "created_at"}}, nil
	}

	if query == "SELECT id, user_id, title, content FROM test_sqlite_Post WHERE id = ?" {
		return &fakeRows{
			columns: []string{"id", "user_id", "title", "content"},
			values:  [][]driver.Value{{int64(10), int64(1), "First", "body"}},
		}, nil
	}

	if query == "SELECT id, post_id, user_id, content FROM test_sqlite_Comment WHERE id = ?" {
		return &fakeRows{
			columns: []string{"id", "post_id", "user_id", "content"},
			values:  [][]driver.Value{{int64(100), int64(10), int64(1), "comment"}},
		}, nil
	}

	if query == "SELECT id, user_id, ip_address FROM test_sqlite_audit_LoginEvent WHERE id = ?" {
		return &fakeRows{
			columns: []string{"id", "user_id", "ip_address"},
			values:  [][]driver.Value{{int64(1000), int64(1), "127.0.0.1"}},
		}, nil
	}

	if rows, ok := rows[query]; ok {
		return &rows, nil
	}
	return &fakeRows{}, nil
}

type fakeRows struct {
	columns []string
	values  [][]driver.Value
	index   int
}

func (r fakeRows) Columns() []string {
	return r.columns
}

func (r fakeRows) Close() error {
	return nil
}

func (r *fakeRows) Next(dest []driver.Value) error {
	if r.index >= len(r.values) {
		return io.EOF
	}
	copy(dest, r.values[r.index])
	r.index++
	return nil
}

func TestSqliteAccessorTypes(t *testing.T) {
	db := NewSqliteDb(nil)

	if db.Users.tableName != "test_sqlite_User" {
		t.Fatalf("unexpected users table name: %s", db.Users.tableName)
	}
	if db.LoginEvents.tableName != "test_sqlite_audit_LoginEvent" {
		t.Fatalf("unexpected login events table name: %s", db.LoginEvents.tableName)
	}
	if db.Users.Count() != 0 {
		t.Fatalf("new users table should be empty")
	}

	var _ func() error = db.LoadAll
	var _ func() error = db.LoadUsers
	var _ func(uint32) (*User, error) = db.GetUserById
	var _ func(uint32) (*Post, error) = db.GetPostById
	var _ func(uint32) (*Comment, error) = db.GetCommentById
	var _ func(uint32) (*LoginEvent, error) = db.GetLoginEventById
}

func TestSqliteAccessorRuntimeWithDatabaseSqlDriver(t *testing.T) {
	db, err := OpenSqliteDb(fakeSqliteDriverName, "")
	if err != nil {
		t.Fatalf("open fake sqlite db: %v", err)
	}
	defer db.Close()

	if err := db.LoadAll(); err != nil {
		t.Fatalf("LoadAll failed: %v", err)
	}

	if db.Users.Count() != 2 {
		t.Fatalf("expected 2 users, got %d", db.Users.Count())
	}
	if db.Posts.Count() != 2 {
		t.Fatalf("expected 2 posts, got %d", db.Posts.Count())
	}
	if db.Comments.Count() != 1 {
		t.Fatalf("expected 1 comment, got %d", db.Comments.Count())
	}
	if db.LoginEvents.Count() != 1 {
		t.Fatalf("expected 1 login event, got %d", db.LoginEvents.Count())
	}

	users := db.Users.All()
	if users[0].Email == nil || *users[0].Email != "alice@example.com" {
		t.Fatalf("expected first user email to be loaded")
	}
	if users[1].Email != nil {
		t.Fatalf("expected nil email to scan as nil pointer")
	}

	user, err := db.GetUserById(1)
	if err != nil {
		t.Fatalf("GetUserById failed: %v", err)
	}
	if user == nil || user.Name != "Alice" {
		t.Fatalf("unexpected user lookup result: %#v", user)
	}

	missingUser, err := db.GetUserById(99)
	if err != nil {
		t.Fatalf("missing GetUserById failed: %v", err)
	}
	if missingUser != nil {
		t.Fatalf("missing user should return nil")
	}

	post, err := db.GetPostById(10)
	if err != nil {
		t.Fatalf("GetPostById failed: %v", err)
	}
	if post == nil || post.Title != "First" {
		t.Fatalf("unexpected post lookup result: %#v", post)
	}
}

func TestSqliteAccessorRuntimeWithModerncSqlite(t *testing.T) {
	db, err := OpenSqliteDb("sqlite", ":memory:")
	if err != nil {
		t.Fatalf("open in-memory sqlite db: %v", err)
	}
	defer db.Close()
	db.db.SetMaxOpenConns(1)

	statements := []string{
		`CREATE TABLE test_sqlite_User (id INTEGER PRIMARY KEY, name TEXT NOT NULL, email TEXT NULL, created_at INTEGER NOT NULL)`,
		`CREATE TABLE test_sqlite_Post (id INTEGER PRIMARY KEY, user_id INTEGER NOT NULL, title TEXT NOT NULL, content TEXT NULL)`,
		`CREATE TABLE test_sqlite_Comment (id INTEGER PRIMARY KEY, post_id INTEGER NOT NULL, user_id INTEGER NOT NULL, content TEXT NOT NULL)`,
		`CREATE TABLE test_sqlite_audit_LoginEvent (id INTEGER PRIMARY KEY, user_id INTEGER NOT NULL, ip_address TEXT NOT NULL)`,
		`INSERT INTO test_sqlite_User (id, name, email, created_at) VALUES (1, 'Alice', 'alice@example.com', 1000), (2, 'Bob', NULL, 2000)`,
		`INSERT INTO test_sqlite_Post (id, user_id, title, content) VALUES (10, 1, 'First', 'body'), (11, 2, 'Second', NULL)`,
		`INSERT INTO test_sqlite_Comment (id, post_id, user_id, content) VALUES (100, 10, 1, 'comment')`,
		`INSERT INTO test_sqlite_audit_LoginEvent (id, user_id, ip_address) VALUES (1000, 1, '127.0.0.1')`,
	}
	for _, statement := range statements {
		if _, err := db.db.Exec(statement); err != nil {
			t.Fatalf("exec sqlite setup %q: %v", statement, err)
		}
	}

	if err := db.LoadAll(); err != nil {
		t.Fatalf("LoadAll with real sqlite failed: %v", err)
	}

	if db.Users.Count() != 2 || db.Posts.Count() != 2 || db.Comments.Count() != 1 || db.LoginEvents.Count() != 1 {
		t.Fatalf("unexpected loaded counts: users=%d posts=%d comments=%d loginEvents=%d",
			db.Users.Count(), db.Posts.Count(), db.Comments.Count(), db.LoginEvents.Count())
	}

	users := db.Users.All()
	if users[0].Email == nil || *users[0].Email != "alice@example.com" {
		t.Fatalf("expected first user email to be loaded from real sqlite")
	}
	if users[1].Email != nil {
		t.Fatalf("expected real sqlite NULL email to scan as nil pointer")
	}

	user, err := db.GetUserById(1)
	if err != nil {
		t.Fatalf("GetUserById with real sqlite failed: %v", err)
	}
	if user == nil || user.Name != "Alice" {
		t.Fatalf("unexpected real sqlite user lookup result: %#v", user)
	}

	missingUser, err := db.GetUserById(99)
	if err != nil {
		t.Fatalf("missing GetUserById with real sqlite failed: %v", err)
	}
	if missingUser != nil {
		t.Fatalf("missing real sqlite user should return nil")
	}

	loginEvent, err := db.GetLoginEventById(1000)
	if err != nil {
		t.Fatalf("GetLoginEventById with real sqlite failed: %v", err)
	}
	if loginEvent == nil || loginEvent.IpAddress != "127.0.0.1" {
		t.Fatalf("unexpected real sqlite nested table lookup result: %#v", loginEvent)
	}
}
