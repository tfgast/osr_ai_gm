#!/usr/bin/env python3
"""Step 3: Connection detection via watershed segmentation.

Uses computer vision (threshold + morphological close/open + watershed)
to segment the dungeon map and detect room adjacencies. Classifies
connection types using step1 symbol data.

Dependencies: pip install opencv-python-headless numpy

Usage:
    python step3_connections.py <map_image> <step1_json> <step2_json> [output_json] [--model MODEL]

    map_image:   unlabeled map (preferred) or labeled map (fallback)
    step1_json:  reviewed step1 output (symbols for type classification)
    step2_json:  reviewed step2 output (location coordinates)
    output_json: output path (default: step3_output.json)
    --model:     ignored (accepted for backward compatibility)
"""

import argparse
import json
import sys
from pathlib import Path

import cv2
import numpy as np

WALL_LABEL = 9999

SYMBOL_TO_CONN_TYPE = {
    "door": "door",
    "stairs": "stairs",
    "secret_door": "secret",
    "pit_trap": "pit",
}


def load_and_preprocess(map_path, threshold=160, kernel_size=5):
    """Load map, threshold, morphological close+open. Return gray and binary mask.

    Closing (dilate->erode) bridges thin wall gaps: grid lines, door marks,
    stair marks that appear as thin dark lines cutting through white corridors.
    Opening (erode->dilate) then removes thin white protrusions/noise.
    """
    gray = cv2.imread(str(map_path), cv2.IMREAD_GRAYSCALE)
    if gray is None:
        print(f"Error: cannot load {map_path}", file=sys.stderr)
        sys.exit(1)
    print(f"  Map: {gray.shape[1]}x{gray.shape[0]}")

    _, binary = cv2.threshold(gray, threshold, 255, cv2.THRESH_BINARY)
    print(f"  Threshold at {threshold}: {np.count_nonzero(binary)} passable pixels")

    kernel = np.ones((kernel_size, kernel_size), np.uint8)
    closed = cv2.morphologyEx(binary, cv2.MORPH_CLOSE, kernel)
    opened = cv2.morphologyEx(closed, cv2.MORPH_OPEN, kernel)

    return gray, opened


def create_markers(binary, locations, loc_ids, seed_radius=3):
    """Create watershed marker array with location seeds and wall labels."""
    h, w = binary.shape
    markers = np.zeros((h, w), dtype=np.int32)
    markers[binary == 0] = WALL_LABEL

    loc_label = {}
    label_loc = {}
    for i, loc_id in enumerate(loc_ids, start=1):
        loc_label[loc_id] = i
        label_loc[i] = loc_id

    for loc_id in loc_ids:
        loc = locations[loc_id]
        try:
            px = int(round(float(loc["pixel_x"])))
            py = int(round(float(loc["pixel_y"])))
        except (KeyError, TypeError, ValueError):
            print(f"  WARNING: seed {loc_id} has invalid coordinates; skipping")
            continue

        if px < 0 or px >= w or py < 0 or py >= h:
            orig_px, orig_py = px, py
            px = min(max(px, 0), w - 1)
            py = min(max(py, 0), h - 1)
            print(
                f"  WARNING: seed {loc_id} clamped to ({px},{py}) "
                f"from out-of-bounds ({orig_px},{orig_py})"
            )
        label = loc_label[loc_id]

        if binary[py, px] == 0:
            found = False
            for r in range(1, 20):
                for dy in range(-r, r + 1):
                    for dx in range(-r, r + 1):
                        ny, nx = py + dy, px + dx
                        if 0 <= ny < h and 0 <= nx < w and binary[ny, nx] > 0:
                            px, py = nx, ny
                            found = True
                            break
                    if found:
                        break
                if found:
                    break
            if found:
                print(f"  WARNING: seed {loc_id} shifted to ({px},{py}) — original in wall")
            else:
                print(f"  WARNING: seed {loc_id} has no passable pixel within 20px!")
                continue

        cv2.circle(markers, (px, py), seed_radius, int(label), -1)

    return markers, loc_label, label_loc


def run_watershed(binary, markers):
    """Run OpenCV watershed on the binary mask."""
    bgr = cv2.cvtColor(binary, cv2.COLOR_GRAY2BGR)
    labels = markers.copy()
    cv2.watershed(bgr, labels)
    return labels


def extract_adjacencies(labels, label_loc):
    """Find adjacent region pairs by examining boundary pixel neighborhoods."""
    h, w = labels.shape
    boundary_ys, boundary_xs = np.where(labels == -1)

    neighbor_labels = []
    for dy in range(-1, 2):
        for dx in range(-1, 2):
            if dy == 0 and dx == 0:
                continue
            ny = boundary_ys + dy
            nx = boundary_xs + dx
            valid = (ny >= 0) & (ny < h) & (nx >= 0) & (nx < w)
            vals = np.full(len(boundary_ys), -1, dtype=np.int32)
            vals[valid] = labels[ny[valid], nx[valid]]
            neighbor_labels.append(vals)

    adjacencies = set()
    all_neighbor_labels = np.stack(neighbor_labels, axis=1)
    mask = (all_neighbor_labels > 0) & (all_neighbor_labels != WALL_LABEL)
    all_neighbor_labels[~mask] = 0

    for i in range(len(boundary_ys)):
        region_labels = set(all_neighbor_labels[i][all_neighbor_labels[i] > 0])
        if len(region_labels) >= 2:
            region_list = sorted(region_labels)
            for ai in range(len(region_list)):
                for bi in range(ai + 1, len(region_list)):
                    a_id = label_loc.get(int(region_list[ai]))
                    b_id = label_loc.get(int(region_list[bi]))
                    if a_id and b_id:
                        adjacencies.add(frozenset((a_id, b_id)))

    return adjacencies


def _point_to_segment_dist(px, py, ax, ay, bx, by):
    """Distance from point (px,py) to line segment (ax,ay)-(bx,by)."""
    dx, dy = bx - ax, by - ay
    seg_len_sq = dx * dx + dy * dy
    if seg_len_sq == 0:
        return ((px - ax) ** 2 + (py - ay) ** 2) ** 0.5
    t = max(0.0, min(1.0, ((px - ax) * dx + (py - ay) * dy) / seg_len_sq))
    proj_x = ax + t * dx
    proj_y = ay + t * dy
    return ((px - proj_x) ** 2 + (py - proj_y) ** 2) ** 0.5


def classify_connections(adjacencies, symbols, locations):
    """Assign connection types based on location types and step1 symbols.

    Rules:
    1. Any connection to a river_waypoint -> river_crossing
    2. Greedy match nearest door/stairs/secret/pit symbol within 40px
       of the line segment between endpoints.
    3. Everything else -> open
    """
    types = {pair: "open" for pair in adjacencies}

    for pair in adjacencies:
        for loc_id in pair:
            if locations.get(loc_id, {}).get("type") == "river_waypoint":
                types[pair] = "river_crossing"
                break

    max_dist = 40
    candidates = []
    for sym in symbols:
        conn_type = SYMBOL_TO_CONN_TYPE.get(sym["type"])
        if not conn_type:
            continue
        sx, sy = sym["pixel_x"], sym["pixel_y"]
        for pair in adjacencies:
            a_id, b_id = pair
            a, b = locations[a_id], locations[b_id]
            d = _point_to_segment_dist(
                sx, sy, a["pixel_x"], a["pixel_y"],
                b["pixel_x"], b["pixel_y"],
            )
            if d < max_dist:
                candidates.append((d, conn_type, pair))

    candidates.sort()
    assigned_pairs = set()
    for d, conn_type, pair in candidates:
        if pair not in assigned_pairs and types[pair] == "open":
            types[pair] = conn_type
            assigned_pairs.add(pair)

    return types


def build_step3_output(adjacencies, locations, conn_types=None):
    """Convert adjacency pairs to step3 JSON format."""
    if conn_types is None:
        conn_types = {}

    rooms = {}
    for loc_id, loc in locations.items():
        rooms[loc_id] = {
            "name": loc.get("name") or loc.get("name_guess", loc_id),
            "exits": [],
        }

    for pair in adjacencies:
        a, b = sorted(pair)
        ct = conn_types.get(pair, "open")
        rooms[a]["exits"].append({
            "to": b,
            "connection_type": ct,
            "notes": "watershed adjacency",
        })
        rooms[b]["exits"].append({
            "to": a,
            "connection_type": ct,
            "notes": "watershed adjacency",
        })

    for r in rooms.values():
        r["exits"].sort(key=lambda e: e["to"])

    return {"rooms": rooms, "observations": "Watershed CV segmentation"}


def save_debug_images(debug_dir, binary, labels, locations, loc_label):
    """Write debug visualizations to debug_dir.

    Produces:
      step3_debug_binary.png    — thresholded+morphed binary mask
      step3_debug_watershed.png — colored watershed regions with room labels
    """
    debug_dir = Path(debug_dir)
    debug_dir.mkdir(parents=True, exist_ok=True)

    # Binary mask
    cv2.imwrite(str(debug_dir / "step3_debug_binary.png"), binary)

    # Watershed regions (random colors per label)
    n_labels = len(loc_label)
    np.random.seed(42)
    colors = np.random.randint(80, 220, size=(n_labels + 1, 3), dtype=np.uint8)
    colors[0] = [0, 0, 0]

    h, w = labels.shape
    vis = np.zeros((h, w, 3), dtype=np.uint8)
    for label_val in range(1, n_labels + 1):
        mask = labels == label_val
        vis[mask] = colors[label_val]

    vis[labels == -1] = [255, 255, 255]  # boundaries
    vis[labels == WALL_LABEL] = [40, 40, 40]  # walls

    for loc_id, loc in locations.items():
        px, py = loc["pixel_x"], loc["pixel_y"]
        cv2.circle(vis, (px, py), 6, (255, 255, 255), -1)
        cv2.putText(vis, loc_id, (px + 8, py - 4),
                    cv2.FONT_HERSHEY_SIMPLEX, 0.35, (255, 255, 255), 1)

    cv2.imwrite(str(debug_dir / "step3_debug_watershed.png"), vis)
    print(f"  Debug images saved to {debug_dir}/")


def main():
    parser = argparse.ArgumentParser(description="Step 3: Connection detection (watershed CV)")
    parser.add_argument("map_image", help="Path to map image (unlabeled preferred)")
    parser.add_argument("step1_json", help="Reviewed step 1 output JSON (symbols)")
    parser.add_argument("step2_json", help="Reviewed step 2 output JSON (locations)")
    parser.add_argument(
        "output", nargs="?", default="step3_output.json", help="Output JSON path"
    )
    parser.add_argument("--model", default=None, help="Ignored (backward compatibility)")
    parser.add_argument("--debug-dir", default=None,
                        help="Directory to write debug images (binary mask, watershed regions)")
    parser.add_argument("--threshold", type=int, default=160,
                        help="Grayscale threshold for binarization (default: 160)")
    parser.add_argument("--kernel", type=int, default=5,
                        help="Morphological kernel size (default: 5)")
    args = parser.parse_args()

    if not Path(args.map_image).exists():
        print(f"Error: map image not found: {args.map_image}", file=sys.stderr)
        sys.exit(1)
    if not Path(args.step1_json).exists():
        print(f"Error: step 1 output not found: {args.step1_json}", file=sys.stderr)
        sys.exit(1)
    if not Path(args.step2_json).exists():
        print(f"Error: step 2 output not found: {args.step2_json}", file=sys.stderr)
        sys.exit(1)

    with open(args.step1_json) as f:
        step1_data = json.load(f)
    symbols = step1_data.get("symbols", [])

    with open(args.step2_json) as f:
        step2_data = json.load(f)
    locations = step2_data["locations"]
    loc_ids = sorted(locations.keys())

    print(f"Step 3: Connection Detection (Watershed CV)")
    print(f"  Image: {args.map_image}")
    print(f"  Locations: {len(loc_ids)}, Symbols: {len(symbols)}")
    print(f"  Output: {args.output}")
    print()

    gray, binary = load_and_preprocess(args.map_image, threshold=args.threshold,
                                       kernel_size=args.kernel)

    markers, loc_label, label_loc = create_markers(binary, locations, loc_ids)
    print(f"  Markers planted: {len(loc_label)} locations + wall label")

    print(f"  Running watershed...")
    labels = run_watershed(binary, markers)

    for loc_id in loc_ids:
        lbl = loc_label[loc_id]
        size = np.count_nonzero(labels == lbl)
        if size == 0:
            print(f"  WARNING: {loc_id} has 0 pixels after watershed")

    adjacencies = extract_adjacencies(labels, label_loc)
    print(f"  Connections found: {len(adjacencies)}")

    if args.debug_dir:
        save_debug_images(args.debug_dir, binary, labels, locations, loc_label)

    conn_types = classify_connections(adjacencies, symbols, locations)
    type_counts = {}
    for ct in conn_types.values():
        type_counts[ct] = type_counts.get(ct, 0) + 1
    print(f"  Connection types: {type_counts}")

    output = build_step3_output(adjacencies, locations, conn_types)

    rooms = output.get("rooms", {})
    total_exits = sum(len(r.get("exits", [])) for r in rooms.values())
    print(f"  Rooms in output: {len(rooms)}")
    print(f"  Total exits: {total_exits}")

    with open(args.output, "w") as f:
        json.dump(output, f, indent=2)
    print(f"\n  Written to: {args.output}")
    print(f"\n  Next: Review {args.output}, then run step4_descriptions.py")


if __name__ == "__main__":
    main()
