#!/usr/bin/env python3
"""Step 1a: Extract room label positions by diffing labeled vs unlabeled map.

Computes the pixel difference between labeled and unlabeled versions of the
same map to isolate label text, then finds blob positions with image processing
and uses an LLM to identify what each blob says.

Usage:
    python step1_diff_labels.py <labeled_map> <unlabeled_map> [output_json]
        [--step0 FILE] [--model MODEL] [--threshold N] [--debug-dir DIR]

    output_json defaults to step1_labels.json in the current directory.
    --step0 provides expected room labels from module text extraction.
    --model selects the LLM for text identification (default: gemini-2.5-pro).
    --threshold sets the pixel difference threshold (default: 20).
    --debug-dir saves intermediate images for inspection.

Requires: pip install Pillow
"""

import argparse
import json
import sys
import tempfile
from pathlib import Path

try:
    from PIL import Image, ImageChops, ImageFilter
except ImportError:
    print("Error: Pillow is required. Install with: pip install Pillow", file=sys.stderr)
    sys.exit(1)

from llm_api import call_llm_json


# --- Image processing ---


def compute_diff_mask(labeled_path: str, unlabeled_path: str, threshold: int = 20):
    """Compute thresholded difference between labeled and unlabeled maps.

    Returns (mask, width, height) where mask is a binary PIL Image.
    """
    labeled = Image.open(labeled_path).convert("RGB")
    unlabeled = Image.open(unlabeled_path).convert("RGB")

    if labeled.size != unlabeled.size:
        raise ValueError(
            f"size mismatch: labeled={labeled.size} unlabeled={unlabeled.size}. "
            "Input maps must be identical dimensions."
        )

    diff = ImageChops.difference(labeled, unlabeled).convert("L")
    mask = diff.point(lambda p: 255 if p > threshold else 0)
    return mask, labeled.size[0], labeled.size[1]


def find_components(mask, min_pixels: int = 20):
    """Find connected components in a binary mask image.

    Returns list of components, each a list of (x, y) pixel coordinates.
    """
    w, h = mask.size
    data = mask.tobytes()
    visited = bytearray(len(data))
    components = []

    for start in range(len(data)):
        if data[start] == 0 or visited[start]:
            continue
        # BFS flood fill (8-connected)
        stack = [start]
        pixels = []
        while stack:
            idx = stack.pop()
            if visited[idx]:
                continue
            if data[idx] == 0:
                continue
            visited[idx] = 1
            x = idx % w
            y = idx // w
            pixels.append((x, y))
            for dx in (-1, 0, 1):
                for dy in (-1, 0, 1):
                    if dx == 0 and dy == 0:
                        continue
                    nx, ny = x + dx, y + dy
                    if 0 <= nx < w and 0 <= ny < h:
                        ni = ny * w + nx
                        if not visited[ni] and data[ni] > 0:
                            stack.append(ni)
        if len(pixels) >= min_pixels:
            components.append(pixels)

    return components


def bbox(comp):
    """Get (x0, y0, x1, y1) bounding box of a component."""
    xs = [p[0] for p in comp]
    ys = [p[1] for p in comp]
    return min(xs), min(ys), max(xs), max(ys)


def centroid(comp):
    """Get (cx, cy) centroid of a component."""
    xs = [p[0] for p in comp]
    ys = [p[1] for p in comp]
    return sum(xs) / len(xs), sum(ys) / len(ys)


def group_nearby_components(components, x_gap: int = 25, y_overlap_ratio: float = 0.3):
    """Group components that are horizontally adjacent (parts of same label).

    Two components are grouped if their bounding boxes are within x_gap
    pixels horizontally and have sufficient vertical overlap.
    """
    if not components:
        return []

    # Compute bbox for each component
    bboxes = [bbox(c) for c in components]

    # Sort by x position
    order = sorted(range(len(components)), key=lambda i: bboxes[i][0])

    groups = []
    used = set()

    for idx in order:
        if idx in used:
            continue
        group = [idx]
        used.add(idx)

        # Iteratively try to add nearby components
        changed = True
        while changed:
            changed = False
            # Group bounding box
            gx0 = min(bboxes[i][0] for i in group)
            gy0 = min(bboxes[i][1] for i in group)
            gx1 = max(bboxes[i][2] for i in group)
            gy1 = max(bboxes[i][3] for i in group)
            gh = gy1 - gy0

            for other in order:
                if other in used:
                    continue
                ox0, oy0, ox1, oy1 = bboxes[other]

                # Horizontal gap from group
                if ox0 > gx1 + x_gap:
                    continue
                if ox1 < gx0 - x_gap:
                    continue

                # Vertical overlap
                overlap_y0 = max(gy0, oy0)
                overlap_y1 = min(gy1, oy1)
                overlap = max(0, overlap_y1 - overlap_y0)
                other_h = oy1 - oy0
                if other_h > 0 and overlap / other_h >= y_overlap_ratio:
                    group.append(other)
                    used.add(other)
                    changed = True

        groups.append(group)

    # Merge pixel lists for each group
    merged = []
    for group in groups:
        pixels = []
        for idx in group:
            pixels.extend(components[idx])
        merged.append(pixels)

    return merged


# --- LLM-based label identification ---


def build_crop_grid(mask, groups, cell_size: int = 80, cols: int = 8):
    """Build a grid image with each blob cropped into a labeled cell.

    Each cell shows the blob's white-on-black content with a green index number.
    Returns the grid PIL Image.
    """
    from PIL import ImageDraw, ImageFont

    rows = (len(groups) + cols - 1) // cols
    grid_w = cols * cell_size
    grid_h = rows * cell_size
    grid = Image.new("RGB", (grid_w, grid_h), (40, 40, 40))
    draw = ImageDraw.Draw(grid)

    font = None
    for font_path in [
        "/usr/share/fonts/TTF/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
    ]:
        if Path(font_path).exists():
            font = ImageFont.truetype(font_path, 11)
            break
    if font is None:
        font = ImageFont.load_default()

    w, h = mask.size

    for i, group in enumerate(groups):
        x0, y0, x1, y1 = bbox(group)
        pad = 4
        crop = mask.crop((max(0, x0 - pad), max(0, y0 - pad),
                          min(w, x1 + pad), min(h, y1 + pad)))

        # Scale crop to fit in cell with margin for the index label
        max_content = cell_size - 18  # leave space for index
        scale = min(max_content / max(crop.width, 1), max_content / max(crop.height, 1), 3.0)
        new_w = max(1, int(crop.width * scale))
        new_h = max(1, int(crop.height * scale))
        crop = crop.resize((new_w, new_h), Image.NEAREST)

        # Position in grid
        col = i % cols
        row = i // cols
        cell_x = col * cell_size
        cell_y = row * cell_size

        # Draw cell border
        draw.rectangle([cell_x, cell_y, cell_x + cell_size - 1, cell_y + cell_size - 1],
                        outline=(80, 80, 80))

        # Paste crop centered in cell, below the index label
        paste_x = cell_x + (cell_size - new_w) // 2
        paste_y = cell_y + 16 + (max_content - new_h) // 2
        grid.paste(crop.convert("RGB"), (paste_x, paste_y))

        # Draw index number in green at top of cell
        idx_text = str(i)
        draw.text((cell_x + 3, cell_y + 2), idx_text, fill=(0, 255, 0), font=font)

    return grid


IDENTIFY_PROMPT = """This image shows a grid of text crops extracted from a dungeon map. Each cell contains white text on a dark background, with a green index number in the top-left corner.

These crops were extracted by computing the pixel difference between labeled and unlabeled versions of the same map — so each crop shows text that only appears on the labeled version.

{known_labels_section}

For each cell (by its green index number), identify:
1. What text the white content shows
2. What category it belongs to

Categories:
- "room_label": Numbered room/area labels like 1, 2, 3A, 4A, 10, 15, etc.
- "trap": Single letter "T" marking a trap
- "pit_trap": Letters "PP" marking a pit trap
- "secret_door": Single letter "S" marking a secret door
- "legend": Map legend text (compass "N", scale text, etc.)
- "sub_label": Single letters A, B, C that mark sub-areas within rooms
- "other": Any other text

Return a JSON object:
{{
  "markers": {{
    "0": {{"text": "<text in cell 0>", "category": "<category>"}},
    "1": {{"text": "<text in cell 1>", "category": "<category>"}},
    ...
  }}
}}

Important:
- Read the WHITE text content, not the GREEN index numbers
- Use EXACT text as written (e.g., "3A" not "3a", "PP" not "P")
- If a cell is unclear, set text to "?" and category to "other"
- There are {num_markers} cells total (indices 0 through {max_marker})"""


def identify_labels_with_llm(mask, groups, known_labels: list[str], model: str) -> dict:
    """Build crop grid and send to LLM for text identification.

    Returns dict mapping cell index to {"text": ..., "category": ...}.
    """
    # Build the crop grid
    grid = build_crop_grid(mask, groups)

    # Save to current directory (LLM CLIs may restrict /tmp access)
    grid_path = str(Path.cwd() / ".diff_crop_grid_tmp.png")
    grid.save(grid_path)

    if known_labels:
        known_section = (
            f"The following room labels are expected: {known_labels}\n"
            f"Most cells should contain one of these labels, a trap marker (T, PP, S), "
            f"or legend text. Use this list to double-check your readings."
        )
    else:
        known_section = "No expected label list provided."

    prompt = IDENTIFY_PROMPT.format(
        known_labels_section=known_section,
        num_markers=len(groups),
        max_marker=len(groups) - 1,
    )

    result = call_llm_json(prompt=prompt, image_path=grid_path, model=model)

    # Clean up
    Path(grid_path).unlink(missing_ok=True)

    return result.get("markers", {})


def build_matched_labels(markers: dict, groups: list) -> tuple[list[dict], list[dict]]:
    """Convert marker identifications + group positions into matched labels.

    Returns (matched_labels, unidentified_groups).
    """
    matched = []
    unidentified = []

    for i, group in enumerate(groups):
        cx, cy = centroid(group)
        bb = bbox(group)
        marker_key = str(i)
        info = markers.get(marker_key)

        if info and info.get("text") and info["text"] != "?":
            matched.append({
                "text": info["text"],
                "category": info.get("category", "other"),
                "pixel_x": round(cx),
                "pixel_y": round(cy),
                "blob_pixels": len(group),
                "blob_bbox": bb,
                "marker_idx": i,
            })
        else:
            unidentified.append({
                "marker_idx": i,
                "pixel_x": round(cx),
                "pixel_y": round(cy),
                "blob_pixels": len(group),
                "blob_bbox": bb,
            })

    return matched, unidentified


# --- Debug output ---


def save_debug_images(mask, groups, matched_labels, debug_dir, labeled_path):
    """Save annotated debug images for inspection."""
    from PIL import ImageDraw, ImageFont

    debug_dir = Path(debug_dir)
    debug_dir.mkdir(parents=True, exist_ok=True)

    # Save raw mask
    mask.save(debug_dir / "diff_mask.png")

    # Save annotated original with matched labels
    img = Image.open(labeled_path).convert("RGB")
    draw = ImageDraw.Draw(img)

    font = None
    for font_path in [
        "/usr/share/fonts/TTF/DejaVuSans-Bold.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
    ]:
        if Path(font_path).exists():
            font = ImageFont.truetype(font_path, 14)
            break
    if font is None:
        font = ImageFont.load_default()

    colors = {
        "room_label": (0, 255, 0),
        "trap": (255, 50, 50),
        "pit_trap": (200, 0, 0),
        "secret_door": (255, 200, 0),
        "legend": (100, 100, 100),
        "other": (150, 150, 255),
    }

    for label in matched_labels:
        color = colors.get(label["category"], (200, 200, 200))
        x, y = label["pixel_x"], label["pixel_y"]
        bb = label.get("blob_bbox")
        if bb:
            draw.rectangle([bb[0] - 2, bb[1] - 2, bb[2] + 2, bb[3] + 2],
                           outline=color, width=2)
        draw.text((x - 5, y - 18), label["text"], fill=color, font=font)

    img.save(debug_dir / "labels_annotated.png")
    print(f"  Debug images saved to {debug_dir}/")


# --- Main ---


def main():
    parser = argparse.ArgumentParser(
        description="Extract room label positions by diffing labeled vs unlabeled maps"
    )
    parser.add_argument("labeled_map", help="Path to labeled map image")
    parser.add_argument("unlabeled_map", help="Path to unlabeled map image")
    parser.add_argument(
        "output", nargs="?", default="step1_labels.json", help="Output JSON path"
    )
    parser.add_argument(
        "--step0",
        help="Step 0 output JSON (known room labels from module text)",
        default=None,
    )
    parser.add_argument(
        "--model",
        default="gemini-2.5-pro",
        help="LLM model for label identification (default: gemini-2.5-pro)",
    )
    parser.add_argument(
        "--threshold", type=int, default=20, help="Pixel difference threshold (default: 20)"
    )
    parser.add_argument(
        "--debug-dir", help="Save debug images to this directory", default=None
    )
    args = parser.parse_args()

    for path, name in [
        (args.labeled_map, "labeled map"),
        (args.unlabeled_map, "unlabeled map"),
    ]:
        if not Path(path).exists():
            print(f"Error: {name} not found: {path}", file=sys.stderr)
            sys.exit(1)

    # Load known labels from step 0
    known_labels = []
    known_names = {}
    if args.step0:
        if not Path(args.step0).exists():
            print(f"Error: step 0 output not found: {args.step0}", file=sys.stderr)
            sys.exit(1)
        with open(args.step0) as f:
            step0 = json.load(f)
        areas = step0.get("areas", {})
        if isinstance(areas, dict):
            areas = list(areas.values())
        for area in areas:
            if area.get("on_map", True):
                label = area["label"]
                known_labels.append(label)
                known_names[label] = area.get("name", "")

    print(f"Step 1a: Diff-based label extraction")
    print(f"  Labeled:   {args.labeled_map}")
    print(f"  Unlabeled: {args.unlabeled_map}")
    print(f"  Model:     {args.model}")
    print(f"  Threshold: {args.threshold}")
    if known_labels:
        print(f"  Expected:  {known_labels}")
    print()

    # --- Phase 1: Image processing ---
    print("  Phase 1: Image processing")

    print("    Computing pixel difference...")
    try:
        mask, img_w, img_h = compute_diff_mask(
            args.labeled_map, args.unlabeled_map, args.threshold
        )
    except ValueError as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)
    print(f"    Image size: {img_w}x{img_h}")

    print("    Finding connected components...")
    raw_components = find_components(mask, min_pixels=20)
    print(f"    Raw components: {len(raw_components)}")

    print("    Grouping nearby characters...")
    groups = group_nearby_components(raw_components)
    print(f"    Label groups: {len(groups)}")
    print()

    # --- Phase 2: LLM identification ---
    print("  Phase 2: LLM label identification")
    print(f"    Annotating diff mask with {len(groups)} numbered markers...")
    print(f"    Sending to {args.model} for text reading...")

    markers = identify_labels_with_llm(mask, groups, known_labels, args.model)
    print(f"    LLM identified: {len(markers)} markers")

    # --- Phase 3: Build results ---
    print()
    print("  Phase 3: Building results")

    matched_labels, unmatched_blobs = build_matched_labels(markers, groups)

    # Separate by category
    room_labels = [l for l in matched_labels if l["category"] == "room_label"]
    symbols = [l for l in matched_labels if l["category"] in ("trap", "pit_trap", "secret_door")]
    legend = [l for l in matched_labels if l["category"] == "legend"]
    other = [l for l in matched_labels if l["category"] == "other"]

    print(f"    Room labels: {len(room_labels)}")
    for rl in sorted(room_labels, key=lambda l: (not l["text"][0].isdigit(), l["text"].zfill(5))):
        print(f"      {rl['text']:>4s}: ({rl['pixel_x']:4d}, {rl['pixel_y']:4d})  "
              f"marker={rl['marker_idx']}")

    if symbols:
        print(f"    Symbols: {len(symbols)}")
        for s in symbols:
            print(f"      {s['text']:>4s}: ({s['pixel_x']:4d}, {s['pixel_y']:4d})  "
                  f"type={s['category']}  marker={s['marker_idx']}")

    if legend:
        print(f"    Legend elements: {len(legend)}")

    if other:
        print(f"    Other: {len(other)}")

    if unmatched_blobs:
        print(f"    Unidentified blobs: {len(unmatched_blobs)}")

    if known_labels:
        found = {rl["text"] for rl in room_labels}
        missing = [l for l in known_labels if l not in found]
        if missing:
            print(f"    Missing expected labels: {missing}")

    # --- Build output ---
    locations = {}
    for rl in room_labels:
        text = rl["text"]
        locations[text] = {
            "label": text,
            "name": known_names.get(text, ""),
            "type": "room",
            "pixel_x": rl["pixel_x"],
            "pixel_y": rl["pixel_y"],
            "source": "diff",
        }

    symbol_list = []
    type_map = {"trap": "trap", "pit_trap": "pit_trap", "secret_door": "secret_door"}
    for s in symbols:
        symbol_list.append({
            "type": type_map.get(s["category"], s["category"]),
            "label": s["text"],
            "pixel_x": s["pixel_x"],
            "pixel_y": s["pixel_y"],
            "source": "diff",
        })

    output = {
        "locations": locations,
        "symbols": symbol_list,
        "map_features": {
            "dimensions": {"width": img_w, "height": img_h},
        },
        "diff_stats": {
            "threshold": args.threshold,
            "raw_components": len(raw_components),
            "groups": len(groups),
            "markers_identified": len(markers),
            "room_labels": len(room_labels),
            "symbols": len(symbols),
            "legend_elements": len(legend),
            "unidentified_blobs": len(unmatched_blobs),
        },
        "all_matched": matched_labels,
        "unidentified_blobs": unmatched_blobs,
    }

    # Save debug images
    if args.debug_dir:
        save_debug_images(mask, groups, matched_labels, args.debug_dir, args.labeled_map)

    # Write output
    with open(args.output, "w") as f:
        json.dump(output, f, indent=2)
    print(f"\n  Written to: {args.output}")


if __name__ == "__main__":
    main()
