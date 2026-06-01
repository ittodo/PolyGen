package polygen

import "testing"

func TestPackEmbedRoundTrip(t *testing.T) {
	position := Position{X: 100.5, Y: 200.3}
	if got := position.Pack(); got != "100.5;200.3" {
		t.Fatalf("Position.Pack() = %q", got)
	}
	unpackedPosition, err := UnpackPosition(position.Pack())
	if err != nil {
		t.Fatalf("UnpackPosition failed: %v", err)
	}
	if unpackedPosition.X != position.X || unpackedPosition.Y != position.Y {
		t.Fatalf("UnpackPosition = %#v", unpackedPosition)
	}

	color := Color{R: 255, G: 128, B: 64}
	if got := color.Pack(); got != "255,128,64" {
		t.Fatalf("Color.Pack() = %q", got)
	}
	unpackedColor, err := UnpackColor(color.Pack())
	if err != nil {
		t.Fatalf("UnpackColor failed: %v", err)
	}
	if unpackedColor != color {
		t.Fatalf("UnpackColor = %#v", unpackedColor)
	}

	bounds := Range{Min: -100, Max: 500}
	if got := bounds.Pack(); got != "-100~500" {
		t.Fatalf("Range.Pack() = %q", got)
	}
	unpackedBounds, ok := TryUnpackRange(bounds.Pack())
	if !ok {
		t.Fatalf("TryUnpackRange failed")
	}
	if unpackedBounds != bounds {
		t.Fatalf("TryUnpackRange = %#v", unpackedBounds)
	}
}

func TestPackEmbedRejectsInvalidInput(t *testing.T) {
	if _, err := UnpackPosition("1.0"); err == nil {
		t.Fatalf("UnpackPosition should reject missing field")
	}
	if _, err := UnpackPosition("nan;2.0"); err == nil {
		t.Fatalf("UnpackPosition should reject invalid float")
	}
	if _, err := UnpackColor("255,-1,64"); err == nil {
		t.Fatalf("UnpackColor should reject negative uint")
	}
	if _, ok := TryUnpackColor("255,999,64"); ok {
		t.Fatalf("TryUnpackColor should reject out-of-range uint8")
	}
}
