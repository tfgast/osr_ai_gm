#!/usr/bin/env python3
"""Annotate a dungeon map with colored location and symbol labels.

Overlays colored circles for locations and diamond markers for symbols.
Auto-detects coordinate scaling if Gemini reported different image dimensions.

Usage:
    python annotate_map.py <map_image> <step_output.json> [output_image]

    output_image defaults to annotated_map.png in the current directory.

Requires: pip install Pillow
"""

import argparse
import json
import sys
from pathlib import Path

try:
    from PIL import Image, ImageDraw, ImageFont
except ImportError:
    print("Error: Pillow is required. Install with: pip install Pillow", file=sys.stderr)
    sys.exit(1)


# Colors by location type (R, G, B)
LOCATION_COLORS = {
    "room": (30, 100, 255),
    "junction": (0, 200, 50),
    "river_waypoint": (220, 30, 30),
    "room_split": (255, 140, 0),
}
DEFAULT_LOCATION_COLOR = (150, 50, 200)

# Colors by symbol type
SYMBOL_COLORS = {
    "trap": (255, 50, 50),
    "pit_trap": (200, 0, 0),
    "secret_door": (255, 200, 0),
    "stairs": (0, 200, 200),
    "door": (180, 180, 180),
}
DEFAULT_SYMBOL_COLOR = (200, 100, 200)

LABEL_RADIUS = 18
SYMBOL_RADIUS = 10
OUTLINE_WIDTH = 2


def _load_fonts():
    """Try to load a reasonable font; fall back to default."""
    for font_name in [
        "/usr/share/fonts/TTF/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/dejavu-sans-fonts/DejaVuSans-Bold.ttf",
    ]:
        if Path(font_name).exists():
            return (
                ImageFont.truetype(font_name, 16),
                ImageFont.truetype(font_name, 12),
                ImageFont.truetype(font_name, 10),
            )
    default = ImageFont.load_default()
    return default, default, default


def _compute_scale(data: dict, actual_w: int, actual_h: int) -> tuple[float, float]:
    """Compute scale factors if Gemini reported different dimensions."""
    dims = data.get("map_features", {}).get("dimensions", {})
    reported_w = dims.get("width", actual_w)
    reported_h = dims.get("height", actual_h)

    sx = actual_w / reported_w if reported_w else 1.0
    sy = actual_h / reported_h if reported_h else 1.0

    return sx, sy


def annotate_map(
    map_path: str,
    data: dict,
    output_path: str,
):
    """Overlay colored labels for locations and symbols on a map image."""
    img = Image.open(map_path).convert("RGB")
    draw = ImageDraw.Draw(img)
    actual_w, actual_h = img.size
    font, font_small, font_tiny = _load_fonts()

    # Auto-detect coordinate scaling
    sx, sy = _compute_scale(data, actual_w, actual_h)
    if sx != 1.0 or sy != 1.0:
        print(f"  Scale correction: {sx:.3f}x, {sy:.3f}y")

    # Draw locations
    locations = data.get("locations", {})
    for loc_id, loc in locations.items():
        px = loc.get("pixel_x")
        py = loc.get("pixel_y")
        if px is None or py is None:
            continue

        loc_type = loc.get("type", "room")
        color = LOCATION_COLORS.get(loc_type, DEFAULT_LOCATION_COLOR)
        x = int(px * sx)
        y = int(py * sy)

        # Clamp to image bounds
        x = max(LABEL_RADIUS, min(actual_w - LABEL_RADIUS, x))
        y = max(LABEL_RADIUS, min(actual_h - LABEL_RADIUS, y))

        r = LABEL_RADIUS
        use_font = font if len(loc_id) <= 2 else font_small

        draw.ellipse(
            [x - r, y - r, x + r, y + r],
            fill=color,
            outline=(255, 255, 255),
            width=OUTLINE_WIDTH,
        )

        bbox = draw.textbbox((0, 0), loc_id, font=use_font)
        tw = bbox[2] - bbox[0]
        th = bbox[3] - bbox[1]
        draw.text(
            (x - tw / 2, y - th / 2),
            loc_id,
            fill=(255, 255, 255),
            font=use_font,
        )

    # Draw symbols as diamonds with short labels
    symbols = data.get("symbols", [])
    for sym in symbols:
        px = sym.get("pixel_x")
        py = sym.get("pixel_y")
        if px is None or py is None:
            continue

        sym_type = sym.get("type", "unknown")
        color = SYMBOL_COLORS.get(sym_type, DEFAULT_SYMBOL_COLOR)
        x = int(px * sx)
        y = int(py * sy)

        # Clamp to image bounds
        x = max(SYMBOL_RADIUS, min(actual_w - SYMBOL_RADIUS, x))
        y = max(SYMBOL_RADIUS, min(actual_h - SYMBOL_RADIUS, y))

        r = SYMBOL_RADIUS

        # Draw diamond shape
        diamond = [(x, y - r), (x + r, y), (x, y + r), (x - r, y)]
        draw.polygon(diamond, fill=color, outline=(255, 255, 255))

        # Short label text
        label = sym.get("label", sym_type[0].upper())
        if len(label) > 2:
            label = label[:2]
        bbox = draw.textbbox((0, 0), label, font=font_tiny)
        tw = bbox[2] - bbox[0]
        th = bbox[3] - bbox[1]
        draw.text(
            (x - tw / 2, y - th / 2),
            label,
            fill=(255, 255, 255),
            font=font_tiny,
        )

    img.save(output_path)
    return img.size


def main():
    parser = argparse.ArgumentParser(description="Annotate map with location and symbol labels")
    parser.add_argument("map_image", help="Path to original dungeon map image")
    parser.add_argument("step_json", help="Step output JSON with locations and symbols")
    parser.add_argument(
        "output", nargs="?", default="annotated_map.png", help="Output annotated image"
    )
    args = parser.parse_args()

    if not Path(args.map_image).exists():
        print(f"Error: map image not found: {args.map_image}", file=sys.stderr)
        sys.exit(1)
    if not Path(args.step_json).exists():
        print(f"Error: step output not found: {args.step_json}", file=sys.stderr)
        sys.exit(1)

    with open(args.step_json) as f:
        data = json.load(f)

    locations = data.get("locations", {})
    symbols = data.get("symbols", [])

    print(f"Annotating map")
    print(f"  Image: {args.map_image}")
    print(f"  Output: {args.output}")
    print(f"  Locations: {len(locations)}")
    print(f"  Symbols: {len(symbols)}")

    # Count locations by type
    type_counts = {}
    for loc in locations.values():
        t = loc.get("type", "room")
        type_counts[t] = type_counts.get(t, 0) + 1
    for t, c in sorted(type_counts.items()):
        color = LOCATION_COLORS.get(t, DEFAULT_LOCATION_COLOR)
        print(f"    {t}: {c} (rgb{color})")

    # Count symbols by type
    sym_counts = {}
    for sym in symbols:
        t = sym.get("type", "unknown")
        sym_counts[t] = sym_counts.get(t, 0) + 1
    for t, c in sorted(sym_counts.items()):
        color = SYMBOL_COLORS.get(t, DEFAULT_SYMBOL_COLOR)
        print(f"    {t}: {c} (rgb{color})")

    size = annotate_map(args.map_image, data, args.output)
    print(f"\n  Written: {args.output} ({size[0]}x{size[1]})")


if __name__ == "__main__":
    main()
