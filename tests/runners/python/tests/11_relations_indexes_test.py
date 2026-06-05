from .schema import (
    ExamplesRelationsPost,
    ExamplesRelationsPostStatus,
    ExamplesRelationsUser,
)
from .schema_binary_refs import BinaryRefDocument
from .schema_container import SchemaContainer


def test_relations_indexes_container() -> None:
    container = SchemaContainer()
    user = ExamplesRelationsUser(id=1, email="author@example.com", display_name="Author")
    post = ExamplesRelationsPost(
        id=10,
        author_id=user.id,
        status=ExamplesRelationsPostStatus.Published,
        title="Relations",
    )

    container.users.add_row(user)
    container.posts.add_row(post)

    assert container.posts.find_by_author_id_status((user.id, ExamplesRelationsPostStatus.Published)) == [post]
    assert container.get_post_author(post) is user
    assert container.find_user_posts(user) == [post]
    assert container.validate_all().is_valid()
    container.validate_or_raise()

    draft = ExamplesRelationsPost(
        id=12,
        author_id=user.id,
        status=ExamplesRelationsPostStatus.Draft,
        title="Draft",
    )
    container.posts.add_row(draft)
    document = BinaryRefDocument.from_container(container)
    refs = document.posts.find_by_author_id_status((user.id, ExamplesRelationsPostStatus.Published))
    assert len(refs) == 1
    assert refs[0].id == post.id
    assert refs[0].status == ExamplesRelationsPostStatus.Published
    assert refs[0].title == "Relations"
    assert document.posts.find_by_author_id_status((404, ExamplesRelationsPostStatus.Published)) == []

    invalid = SchemaContainer()
    invalid.posts.add_row(
        ExamplesRelationsPost(
            id=11,
            author_id=404,
            status=ExamplesRelationsPostStatus.Draft,
            title="Missing author",
        )
    )
    result = invalid.validate_all()
    assert not result.is_valid()
    assert result.error_count() == 1
    assert result.errors[0].field_name == "author_id"


test_relations_indexes_container()
