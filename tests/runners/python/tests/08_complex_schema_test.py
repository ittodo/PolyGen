from .schema import (
    GameCharacterPlayer,
    GameCharacterPlayerStatus,
    GameCharacterStats,
    GameCommonVec3,
)
from .schema_container import SchemaContainer


def make_stats() -> GameCharacterStats:
    return GameCharacterStats(
        hp=100,
        max_hp=100,
        mp=50,
        max_mp=50,
        strength=10,
        agility=8,
        intelligence=5,
        vitality=12,
    )


def make_player(player_id: int, name: str, level: int) -> GameCharacterPlayer:
    return GameCharacterPlayer(
        id=player_id,
        name=name,
        level=level,
        experience=0,
        stats=make_stats(),
        position=GameCommonVec3(x=0.0, y=0.0, z=0.0),
        status=GameCharacterPlayerStatus.Online,
        guild_id=None,
    )


def test_container_field_and_unique_validation() -> None:
    invalid_fields = SchemaContainer()
    invalid_fields.players.add_row(
        make_player(
            1,
            "A name that is definitely longer than thirty two chars",
            101,
        )
    )

    invalid_fields_result = invalid_fields.validate_all()
    assert not invalid_fields_result.is_valid()
    assert invalid_fields_result.error_count() == 2
    assert any(
        error.constraint_type == "MaxLength" and error.field_name == "name"
        for error in invalid_fields_result.errors
    )
    assert any(
        error.constraint_type == "Range" and error.field_name == "level"
        for error in invalid_fields_result.errors
    )

    duplicate = SchemaContainer()
    duplicate.players.add_row(make_player(1, "Hero A", 10))
    duplicate.players.add_row(make_player(1, "Hero B", 10))

    duplicate_result = duplicate.validate_all()
    assert not duplicate_result.is_valid()
    assert any(error.constraint_type == "Unique" for error in duplicate_result.errors)


test_container_field_and_unique_validation()
