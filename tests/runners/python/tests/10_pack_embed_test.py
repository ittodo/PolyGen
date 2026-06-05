from .schema import (
    TestPackEmbedColor,
    TestPackEmbedColorAlpha,
    TestPackEmbedPosition,
    TestPackEmbedRange,
    TestPackEmbedSize,
    TestPackEmbedStats,
)


def test_pack_roundtrip() -> None:
    position = TestPackEmbedPosition(x=100.5, y=200.25)
    assert position.pack() == "100.5;200.25"
    assert TestPackEmbedPosition.unpack("100.5;200.25") == position

    color = TestPackEmbedColor(r=255, g=128, b=64)
    assert color.pack() == "255,128,64"
    assert TestPackEmbedColor.unpack("255,128,64") == color

    alpha = TestPackEmbedColorAlpha(r=255, g=255, b=255, a=128)
    assert alpha.pack() == "255|255|255|128"
    assert TestPackEmbedColorAlpha.unpack("255|255|255|128") == alpha

    size = TestPackEmbedSize(width=800, height=600)
    assert size.pack() == "800;600"
    assert TestPackEmbedSize.unpack("800;600") == size

    signed_range = TestPackEmbedRange(min=-100, max=100)
    assert signed_range.pack() == "-100~100"
    assert TestPackEmbedRange.unpack("-100~100") == signed_range


def test_try_unpack_rejects_invalid_input() -> None:
    assert TestPackEmbedPosition.try_unpack("1") is None
    assert TestPackEmbedPosition.try_unpack("nan;2") is None
    assert TestPackEmbedColor.try_unpack("-1,2,3") is None
    assert TestPackEmbedColor.try_unpack("red,2,3") is None

    try:
        TestPackEmbedSize.unpack("10")
    except ValueError as exc:
        assert "expected 2 fields" in str(exc)
    else:
        raise AssertionError("unpack should reject missing fields")


def test_non_packed_embed_has_no_pack_api() -> None:
    stats = TestPackEmbedStats(hp=10, mp=5, attack=3, defense=2)
    assert not hasattr(stats, "pack")
    assert not hasattr(TestPackEmbedStats, "unpack")


test_pack_roundtrip()
test_try_unpack_rejects_invalid_input()
test_non_packed_embed_has_no_pack_api()
