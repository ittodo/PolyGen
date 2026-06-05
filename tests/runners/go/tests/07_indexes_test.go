package polygen

import (
	"os"
	"path/filepath"
	"testing"
)

func TestIndexesAndForeignKeyValidation(t *testing.T) {
	container := NewSchemaContainer()
	user := &User{Id: 1, Username: "alice", Email: "alice@example.com", DisplayName: "Alice"}
	description := "Binary reference systems"
	category := &Category{Id: 10, Name: "news", Description: &description, Rank: 7, Kind: CategoryKindPublic}
	post := &Post{Id: 100, Title: "title", Content: "body", AuthorId: user.Id, CategoryId: category.Id}
	tag := &Tag{Id: 200, Name: "featured"}
	postTag := &PostTag{PostId: post.Id, TagId: tag.Id}

	container.Users.AddRow(user)
	container.Categorys.AddRow(category)
	container.Posts.AddRow(post)
	container.Tags.AddRow(tag)
	container.PostTags.AddRow(postTag)

	if container.Users.GetByUsername("alice") != user {
		t.Fatalf("unique username index did not return inserted row")
	}
	if got := container.Posts.GetByAuthorId(user.Id); len(got) != 1 || got[0] != post {
		t.Fatalf("group author index returned %#v", got)
	}
	if got := container.PostTags.GetByTagId(tag.Id); len(got) != 1 || got[0] != postTag {
		t.Fatalf("junction tag index returned %#v", got)
	}
	if got := container.GetPostAuthor(post); got != user {
		t.Fatalf("post author navigation returned %#v", got)
	}
	if got := container.GetPostCategory(post); got != category {
		t.Fatalf("post category navigation returned %#v", got)
	}
	if got := container.GetPostTagPost(postTag); got != post {
		t.Fatalf("post tag post navigation returned %#v", got)
	}
	if got := container.GetPostTagTag(postTag); got != tag {
		t.Fatalf("post tag tag navigation returned %#v", got)
	}
	if err := container.ValidateOrError(); err != nil {
		t.Fatalf("expected valid foreign keys: %v", err)
	}
}

func TestContainerSearchIndexes(t *testing.T) {
	container := NewSchemaContainer()
	description := "Binary reference systems"
	category := &Category{Id: 10, Name: "Technology", Description: &description, Rank: 7, Kind: CategoryKindPublic}
	post := &Post{Id: 100, Title: "Binary Reference Guide", Content: "body", AuthorId: 1, CategoryId: category.Id}
	other := &Post{Id: 101, Title: "General Indexes", Content: "body", AuthorId: 1, CategoryId: category.Id}

	container.Categorys.AddRow(category)
	container.Posts.AddRow(post)
	container.Posts.AddRow(other)

	if got := container.Categorys.SearchByName("technology"); len(got) != 1 || got[0] != category {
		t.Fatalf("exact string search returned %#v", got)
	}
	if got := container.Categorys.SearchByDescription("reference"); len(got) != 1 || got[0] != category {
		t.Fatalf("optional string token search returned %#v", got)
	}
	if got := container.Categorys.SearchByRank(7); len(got) != 1 || got[0] != category {
		t.Fatalf("numeric exact search returned %#v", got)
	}
	if got := container.Categorys.SearchByKind(CategoryKindPublic); len(got) != 1 || got[0] != category {
		t.Fatalf("enum exact search returned %#v", got)
	}
	if got := container.Posts.SearchByTitle("binary"); len(got) != 1 || got[0] != post {
		t.Fatalf("post title token search returned %#v", got)
	}
	if got := container.Posts.SearchByTitle("missing"); len(got) != 0 {
		t.Fatalf("missing token search returned %#v", got)
	}
	container.Clear()
	if got := container.Posts.SearchByTitle("binary"); len(got) != 0 {
		t.Fatalf("clear should remove search postings, got %#v", got)
	}
}

func TestBinaryRefSearchIndexes(t *testing.T) {
	description := "Binary reference systems"
	category := &Category{Id: 10, Name: "Technology", Description: &description, Rank: 7, Kind: CategoryKindPublic}
	post := &Post{Id: 100, Title: "Binary Reference Guide", Content: "body", AuthorId: 1, CategoryId: category.Id}
	other := &Post{Id: 101, Title: "General Indexes", Content: "body", AuthorId: 1, CategoryId: category.Id}

	data, err := SaveSchemaBinaryRefContext(&SchemaBinaryRefContainer{
		Users: []*User{
			{Id: 1, Username: "alice", Email: "alice@example.com", DisplayName: "Alice"},
		},
		Categorys: []*Category{category},
		Posts:     []*Post{post, other},
		Comments:  []*Comment{},
		Tags:      []*Tag{},
		PostTags:  []*PostTag{},
	})
	if err != nil {
		t.Fatalf("save binary ref: %v", err)
	}

	ctx, err := OpenSchemaBinaryRefContext(data)
	if err != nil {
		t.Fatalf("open binary ref: %v", err)
	}

	if got := ctx.Categorys.SearchByName("technology"); len(got) != 1 {
		t.Fatalf("binary ref exact string search returned %#v", got)
	} else {
		name, err := got[0].Name()
		if err != nil || name != "Technology" {
			t.Fatalf("binary ref name getter = %q, %v", name, err)
		}
	}
	if got := ctx.Categorys.SearchByDescription("reference"); len(got) != 1 {
		t.Fatalf("binary ref optional string token search returned %#v", got)
	}
	if got := ctx.Categorys.SearchByRank(7); len(got) != 1 {
		t.Fatalf("binary ref numeric exact search returned %#v", got)
	} else {
		rank, err := got[0].Rank()
		if err != nil || rank != 7 {
			t.Fatalf("binary ref rank getter = %d, %v", rank, err)
		}
	}
	if got := ctx.Categorys.SearchByKind(CategoryKindPublic); len(got) != 1 {
		t.Fatalf("binary ref enum exact search returned %#v", got)
	}
	if got := ctx.Posts.SearchByTitle("binary"); len(got) != 1 {
		t.Fatalf("binary ref post title token search returned %#v", got)
	} else {
		title, err := got[0].Title()
		if err != nil || title != "Binary Reference Guide" {
			t.Fatalf("binary ref title getter = %q, %v", title, err)
		}
	}
	if got := ctx.Posts.SearchByTitle("missing"); len(got) != 0 {
		t.Fatalf("binary ref missing token search returned %#v", got)
	}
}

func TestBinaryRefRejectsInvalidEnumWrite(t *testing.T) {
	description := "Invalid enum category"
	_, err := SaveSchemaBinaryRefContext(&SchemaBinaryRefContainer{
		Users: []*User{
			{Id: 1, Username: "alice", Email: "alice@example.com", DisplayName: "Alice"},
		},
		Categorys: []*Category{
			{Id: 10, Name: "InvalidKind", Description: &description, Rank: 1, Kind: CategoryKind(999)},
		},
		Posts:    []*Post{},
		Comments: []*Comment{},
		Tags:     []*Tag{},
		PostTags: []*PostTag{},
	})
	if err == nil {
		t.Fatalf("expected invalid BinaryRef enum write to fail")
	}
}

func TestGeneratedLoaders(t *testing.T) {
	dir := t.TempDir()
	write := func(name string, content string) {
		t.Helper()
		if err := os.WriteFile(filepath.Join(dir, name), []byte(content), 0644); err != nil {
			t.Fatalf("write %s: %v", name, err)
		}
	}

	write("users.csv", "id,username,email,display_name\n1,alice,alice@example.com,Alice\n")
	write("categories.csv", "id,name,description,rank,kind\n10,Technology,Binary reference systems,7,Public\n11,Internal,Private systems,3,CategoryKindInternal\n")
	write("posts.csv", "id,title,content,author_id,category_id\n100,Binary Reference Guide,body,1,10\n")
	write("comments.csv", "id,post_id,author_id,content,parent_id\n1,100,1,hello,\n")
	write("tags.csv", "id,name\n200,featured\n")
	write("post_tags.csv", "post_id,tag_id\n100,200\n")

	container := NewSchemaContainer()
	if err := container.LoadFromCsv(dir); err != nil {
		t.Fatalf("LoadFromCsv: %v", err)
	}
	if got := container.Users.Count(); got != 1 {
		t.Fatalf("loaded users = %d", got)
	}
	if got := container.Categorys.SearchByKind(CategoryKindInternal); len(got) != 1 || got[0].Name != "Internal" {
		t.Fatalf("loaded enum name search returned %#v", got)
	}
	if got := container.Posts.SearchByTitle("binary"); len(got) != 1 || got[0].Title != "Binary Reference Guide" {
		t.Fatalf("loaded post search returned %#v", got)
	}
	if err := container.ValidateOrError(); err != nil {
		t.Fatalf("loaded CSV data should validate: %v", err)
	}

	write("users.json", `[{"id":1,"username":"alice","email":"alice@example.com","display_name":"Alice"}]`)
	write("categories.json", `[{"id":12,"name":"JsonInternal","description":"Private systems","rank":4,"kind":"Internal"},{"id":13,"name":"JsonPublic","description":"Public systems","rank":5,"kind":1}]`)
	write("posts.json", `[{"id":101,"title":"JSON Binary Reference Guide","content":"body","author_id":1,"category_id":12}]`)
	write("comments.json", `[{"id":2,"post_id":101,"author_id":1,"content":"json hello","parent_id":null}]`)
	write("tags.json", `[{"id":201,"name":"json-tag"}]`)
	write("post_tags.json", `[{"post_id":101,"tag_id":201}]`)
	sourcesJsonContainer := NewSchemaContainer()
	if err := sourcesJsonContainer.LoadFromJson(dir); err != nil {
		t.Fatalf("LoadFromJson: %v", err)
	}
	if got := sourcesJsonContainer.Users.Count(); got != 1 {
		t.Fatalf("loaded JSON users = %d", got)
	}
	if got := sourcesJsonContainer.Categorys.SearchByKind(CategoryKindInternal); len(got) != 1 || got[0].Name != "JsonInternal" {
		t.Fatalf("sources JSON enum name search returned %#v", got)
	}
	if got := sourcesJsonContainer.Categorys.SearchByKind(CategoryKindPublic); len(got) != 1 || got[0].Name != "JsonPublic" {
		t.Fatalf("sources JSON numeric enum search returned %#v", got)
	}
	if got := sourcesJsonContainer.Posts.SearchByTitle("json binary"); len(got) != 1 || got[0].Title != "JSON Binary Reference Guide" {
		t.Fatalf("sources JSON post search returned %#v", got)
	}
	post := sourcesJsonContainer.Posts.GetById(101)
	if post == nil {
		t.Fatalf("sources JSON post id lookup failed")
	}
	if got := sourcesJsonContainer.GetPostAuthor(post); got == nil || got.Username != "alice" {
		t.Fatalf("sources JSON post author navigation returned %#v", got)
	}
	if got := sourcesJsonContainer.PostTags.GetByTagId(201); len(got) != 1 || got[0].PostId != 101 {
		t.Fatalf("sources JSON post tag index returned %#v", got)
	}
	if err := sourcesJsonContainer.ValidateOrError(); err != nil {
		t.Fatalf("loaded sources JSON data should validate: %v", err)
	}

	write("Category.json", `[{"id":12,"name":"JsonInternal","description":"Private systems","rank":4,"kind":"Internal"},{"id":13,"name":"JsonPublic","description":"Public systems","rank":5,"kind":1}]`)
	write("Tag.json", `[{"id":201,"name":"json-tag"}]`)
	jsonContainer := NewSchemaContainer()
	if err := jsonContainer.LoadFromJsonDirectory(dir); err != nil {
		t.Fatalf("LoadFromJsonDirectory: %v", err)
	}
	if got := jsonContainer.Categorys.SearchByKind(CategoryKindInternal); len(got) != 1 || got[0].Name != "JsonInternal" {
		t.Fatalf("loaded JSON enum name search returned %#v", got)
	}
	if got := jsonContainer.Categorys.SearchByKind(CategoryKindPublic); len(got) != 1 || got[0].Name != "JsonPublic" {
		t.Fatalf("loaded JSON numeric enum search returned %#v", got)
	}
	if got := jsonContainer.Tags.GetByName("json-tag"); got == nil || got.Id != 201 {
		t.Fatalf("loaded JSON tag lookup returned %#v", got)
	}

	write("bad_categories.csv", "id,name,description,rank,kind\n11,Bad,,not-a-number,1\n")
	if _, err := LoadCategorysFromCsv(filepath.Join(dir, "bad_categories.csv")); err == nil {
		t.Fatalf("expected invalid CSV numeric value to return an error")
	}

	write("bad_category_kind.csv", "id,name,description,rank,kind\n12,BadKind,,1,MissingKind\n")
	if _, err := LoadCategorysFromCsv(filepath.Join(dir, "bad_category_kind.csv")); err == nil {
		t.Fatalf("expected invalid CSV enum name to return an error")
	}

	write("bad_category_kind.json", `[{"id":14,"name":"BadJsonKind","description":"","rank":1,"kind":"MissingKind"}]`)
	if _, err := LoadCategorysFromJson(filepath.Join(dir, "bad_category_kind.json")); err == nil {
		t.Fatalf("expected invalid JSON enum name to return an error")
	}
}

func TestForeignKeyValidationRejectsMissingReference(t *testing.T) {
	container := NewSchemaContainer()
	container.Users.AddRow(&User{Id: 1, Username: "alice", Email: "alice@example.com", DisplayName: "Alice"})
	container.Posts.AddRow(&Post{Id: 100, Title: "title", Content: "body", AuthorId: 1, CategoryId: 999})

	result := container.ValidateAll()
	if result.IsValid() {
		t.Fatalf("expected missing category foreign key to be rejected")
	}
	if result.ErrorCount() != 1 {
		t.Fatalf("expected one validation error, got %d: %s", result.ErrorCount(), result.String())
	}
}
