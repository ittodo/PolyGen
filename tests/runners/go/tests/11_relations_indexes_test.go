package polygen

import "testing"

func TestRelationsIndexesContainer(t *testing.T) {
	container := NewSchemaContainer()
	user := &User{Id: 1, Email: "author@example.com", DisplayName: "Author"}
	post := &Post{Id: 10, AuthorId: user.Id, Status: PostStatusPublished, Title: "Relations"}

	container.Users.AddRow(user)
	container.Posts.AddRow(post)

	if got := container.Posts.GetByAuthorIdStatus(PostByAuthorIdStatusKey{AuthorId: user.Id, Status: PostStatusPublished}); len(got) != 1 || got[0] != post {
		t.Fatalf("composite author/status index returned %#v", got)
	}
	if got := container.GetPostAuthor(post); got != user {
		t.Fatalf("post author navigation returned %#v", got)
	}
	if got := container.FindUserPosts(user); len(got) != 1 || got[0] != post {
		t.Fatalf("reverse user posts navigation returned %#v", got)
	}
	if err := container.ValidateOrError(); err != nil {
		t.Fatalf("expected valid relations: %v", err)
	}

	invalid := NewSchemaContainer()
	invalid.Posts.AddRow(&Post{Id: 11, AuthorId: 404, Status: PostStatusDraft, Title: "Missing author"})
	result := invalid.ValidateAll()
	if result.IsValid() {
		t.Fatalf("expected missing author foreign key to fail validation")
	}
	if result.ErrorCount() != 1 {
		t.Fatalf("expected one validation error, got %d: %s", result.ErrorCount(), result.String())
	}
}

func TestRelationsIndexesBinaryRefCompositeLookup(t *testing.T) {
	user := &User{Id: 1, Email: "author@example.com", DisplayName: "Author"}
	post := &Post{Id: 10, AuthorId: user.Id, Status: PostStatusPublished, Title: "Relations"}
	draft := &Post{Id: 11, AuthorId: user.Id, Status: PostStatusDraft, Title: "Draft"}

	data, err := SaveSchemaBinaryRefContext(&SchemaBinaryRefContainer{
		Users: []*User{user},
		Posts: []*Post{post, draft},
	})
	if err != nil {
		t.Fatalf("save binary ref: %v", err)
	}

	ctx, err := OpenSchemaBinaryRefContext(data)
	if err != nil {
		t.Fatalf("open binary ref: %v", err)
	}

	refs := ctx.Posts.FindByAuthorIdStatus(PostByAuthorIdStatusKey{AuthorId: user.Id, Status: PostStatusPublished})
	if len(refs) != 1 {
		t.Fatalf("binary ref composite author/status index returned %#v", refs)
	}
	id, err := refs[0].Id()
	if err != nil || id != post.Id {
		t.Fatalf("binary ref composite lookup id = %d, %v", id, err)
	}
	status, err := refs[0].Status()
	if err != nil || status != PostStatusPublished {
		t.Fatalf("binary ref composite lookup status = %v, %v", status, err)
	}
	if got := ctx.Posts.FindByAuthorIdStatus(PostByAuthorIdStatusKey{AuthorId: 404, Status: PostStatusPublished}); len(got) != 0 {
		t.Fatalf("missing binary ref composite key returned %#v", got)
	}
}
