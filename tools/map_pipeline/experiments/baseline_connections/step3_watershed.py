#!/usr/bin/env python3
"""Step 3 experiment: watershed segmentation for room connections.

Uses computer vision (threshold + morphological opening + watershed)
to segment the dungeon map and detect room adjacencies.

Dependencies: pip install opencv-python-headless numpy scipy

Usage:
    python step3_watershed.py [--threshold 160] [--kernel 3] [--debug]
"""

import argparse
import json
import subprocess
import sys
from pathlib import Path

import cv2
import numpy as np

# --- Paths ---
SCRIPT_DIR = Path(__file__).resolve().parent
PIPELINE_DIR = SCRIPT_DIR.parent.parent
TOOLS_DIR = PIPELINE_DIR.parent
PROJECT_DIR = TOOLS_DIR.parent

MAP_PATH = PROJECT_DIR / "morkaal_map_unlabeled.png"
STEP1_REVIEWED = PIPELINE_DIR / "pipeline_output" / "step1_output_reviewed.json"
STEP2_REVIEWED = PIPELINE_DIR / "pipeline_output" / "step2_output_reviewed.json"
GROUND_TRUTH = PIPELINE_DIR / "pipeline_output" / "step3_output_reviewed.json"
COMPARE_SCRIPT = TOOLS_DIR / "map_connection_experiment" / "compare.py"
OUTPUT_PATH = SCRIPT_DIR / "step3_watershed.json"
DEBUG_DIR = SCRIPT_DIR / "watershed_debug"

WALL_LABEL = 9999


def load_json(path):
    with open(path) as f:
        return json.load(f)


def load_and_preprocess(map_path, threshold, kernel_size, debug_dir=None):
    """Load map, threshold, morphological close+open. Return gray and binary mask.

    Closing (dilate→erode) bridges thin wall gaps: grid lines, door marks,
    stair marks that appear as thin dark lines cutting through white corridors.
    Opening (erode→dilate) then removes thin white protrusions/noise.
    """
    gray = cv2.imread(str(map_path), cv2.IMREAD_GRAYSCALE)
    if gray is None:
        print(f"Error: cannot load {map_path}", file=sys.stderr)
        sys.exit(1)
    print(f"  Loaded map: {gray.shape[1]}x{gray.shape[0]}")

    # Threshold: light = passable (255), dark = wall (0)
    _, binary = cv2.threshold(gray, threshold, 255, cv2.THRESH_BINARY)
    passable_count = np.count_nonzero(binary)
    print(f"  Threshold at {threshold}: {passable_count} passable pixels")

    if debug_dir:
        cv2.imwrite(str(debug_dir / "debug_01_threshold.png"), binary)

    # Closing: bridge thin wall gaps (grid lines, doors, stairs in corridors)
    kernel = np.ones((kernel_size, kernel_size), np.uint8)
    closed = cv2.morphologyEx(binary, cv2.MORPH_CLOSE, kernel)
    bridged = np.count_nonzero(closed) - passable_count
    print(f"  Closing kernel={kernel_size}: bridged {bridged} wall-gap pixels")

    if debug_dir:
        cv2.imwrite(str(debug_dir / "debug_02_closed.png"), closed)

    # Opening: remove thin white noise/protrusions
    opened = cv2.morphologyEx(closed, cv2.MORPH_OPEN, kernel)
    removed = np.count_nonzero(closed) - np.count_nonzero(opened)
    print(f"  Opening kernel={kernel_size}: removed {removed} thin-protrusion pixels")

    if debug_dir:
        cv2.imwrite(str(debug_dir / "debug_03_opened.png"), opened)

    return gray, opened


def create_markers(binary, locations, loc_ids, seed_radius=3):
    """Create watershed marker array with location seeds and wall labels."""
    h, w = binary.shape
    markers = np.zeros((h, w), dtype=np.int32)

    # Label all wall pixels (binary == 0) as WALL_LABEL
    markers[binary == 0] = WALL_LABEL

    # Build label mapping: loc_id -> integer label (1-based)
    loc_label = {}
    label_loc = {}
    for i, loc_id in enumerate(loc_ids, start=1):
        loc_label[loc_id] = i
        label_loc[i] = loc_id

    # Plant seeds at each location
    for loc_id in loc_ids:
        loc = locations[loc_id]
        px, py = loc["pixel_x"], loc["pixel_y"]
        label = loc_label[loc_id]

        # Verify seed is in passable space
        if binary[py, px] == 0:
            # Spiral search for nearest passable pixel
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

        # Plant a small circle as marker
        cv2.circle(markers, (px, py), seed_radius, int(label), -1)

    return markers, loc_label, label_loc


def run_watershed(binary, markers):
    """Run OpenCV watershed on the binary mask.

    Using the processed binary mask (not the original grayscale) ensures
    boundaries follow the passable/wall edges rather than image texture.
    """
    # cv2.watershed needs 8-bit 3-channel image
    bgr = cv2.cvtColor(binary, cv2.COLOR_GRAY2BGR)
    labels = markers.copy()
    cv2.watershed(bgr, labels)
    # Output: -1 = boundary, positive = region labels
    return labels


def extract_adjacencies(labels, label_loc):
    """Find adjacent region pairs by examining boundary pixel neighborhoods.

    cv2.watershed marks boundaries as -1, so adjacent regions never directly
    touch. Instead, we find all -1 pixels and collect the distinct region
    labels among their 8-neighbors.
    """
    h, w = labels.shape
    boundary_ys, boundary_xs = np.where(labels == -1)

    # Collect all (region_label, boundary_index) pairs from 8-neighbors
    neighbor_labels = []
    boundary_indices = []

    for dy in range(-1, 2):
        for dx in range(-1, 2):
            if dy == 0 and dx == 0:
                continue
            ny = boundary_ys + dy
            nx = boundary_xs + dx
            valid = (ny >= 0) & (ny < h) & (nx >= 0) & (nx < w)
            vals = np.full(len(boundary_ys), -1, dtype=np.int32)
            vals[valid] = labels[ny[valid], nx[valid]]
            # Keep only region labels (positive, not wall, not boundary)
            is_region = (vals > 0) & (vals != WALL_LABEL)
            neighbor_labels.append(vals)
            boundary_indices.append(np.arange(len(boundary_ys)))

    # For each boundary pixel, collect the set of distinct region labels
    # among its neighbors, then emit pairs
    adjacencies = set()

    # Stack all neighbor labels per boundary pixel
    all_neighbor_labels = np.stack(neighbor_labels, axis=1)  # shape: (n_boundary, 8)
    # Mask non-region values
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


SYMBOL_TO_CONN_TYPE = {
    "door": "door",
    "stairs": "stairs",
    "secret_door": "secret",
    "pit_trap": "pit",
}


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
    1. Any connection to a river_waypoint → river_crossing
    2. For each connection, find the nearest door/stairs/secret symbol
       within 80px of the line segment between endpoints → assign that type.
    3. Everything else → open
    """
    types = {pair: "open" for pair in adjacencies}

    # Rule 1: river crossings
    for pair in adjacencies:
        for loc_id in pair:
            if locations.get(loc_id, {}).get("type") == "river_waypoint":
                types[pair] = "river_crossing"
                break

    # Rule 2: greedy match symbols to connections by distance
    max_dist = 40  # px

    # Build all (distance, symbol, connection) candidates
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

    # Greedy assign: closest first, each connection assigned at most once
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

    # Sort exits for deterministic output
    for r in rooms.values():
        r["exits"].sort(key=lambda e: e["to"])

    return {"rooms": rooms, "observations": "Watershed CV segmentation"}


def save_debug_images(debug_dir, gray, labels, locations,
                      adjacencies, loc_label):
    """Save visualization images for inspection."""
    # Watershed regions (random colors)
    n_labels = len(loc_label)
    np.random.seed(42)
    colors = np.random.randint(80, 220, size=(n_labels + 1, 3), dtype=np.uint8)
    colors[0] = [0, 0, 0]  # unknown

    h, w = labels.shape
    vis = np.zeros((h, w, 3), dtype=np.uint8)
    for label_val in range(1, n_labels + 1):
        mask = labels == label_val
        vis[mask] = colors[label_val]

    # Boundaries in white
    vis[labels == -1] = [255, 255, 255]
    # Wall in dark gray
    vis[labels == WALL_LABEL] = [40, 40, 40]

    # Draw location labels
    for loc_id, loc in locations.items():
        px, py = loc["pixel_x"], loc["pixel_y"]
        cv2.circle(vis, (px, py), 6, (255, 255, 255), -1)
        cv2.putText(vis, loc_id, (px + 8, py - 4),
                    cv2.FONT_HERSHEY_SIMPLEX, 0.35, (255, 255, 255), 1)

    cv2.imwrite(str(debug_dir / "debug_04_watershed.png"), vis)

    # Connection graph over original map
    conn_vis = cv2.cvtColor(gray, cv2.COLOR_GRAY2BGR)
    for pair in adjacencies:
        a_id, b_id = sorted(pair)
        a = locations[a_id]
        b = locations[b_id]
        pt1 = (a["pixel_x"], a["pixel_y"])
        pt2 = (b["pixel_x"], b["pixel_y"])
        cv2.line(conn_vis, pt1, pt2, (0, 200, 0), 2)

    for loc_id, loc in locations.items():
        px, py = loc["pixel_x"], loc["pixel_y"]
        cv2.circle(conn_vis, (px, py), 8, (0, 0, 255), -1)
        cv2.putText(conn_vis, loc_id, (px + 10, py - 5),
                    cv2.FONT_HERSHEY_SIMPLEX, 0.4, (0, 0, 255), 1)

    cv2.imwrite(str(debug_dir / "debug_05_connections.png"), conn_vis)
    print(f"  Debug images saved to {debug_dir}/")


def main():
    parser = argparse.ArgumentParser(description="Watershed connection detection")
    parser.add_argument("--threshold", type=int, default=160,
                        help="Grayscale threshold (default: 160)")
    parser.add_argument("--kernel", type=int, default=5,
                        help="Morphological close+open kernel size (default: 5)")
    parser.add_argument("--seed-radius", type=int, default=3,
                        help="Marker seed radius in pixels (default: 3)")
    parser.add_argument("--debug", action="store_true", default=True,
                        help="Save debug images (default: true)")
    parser.add_argument("--no-debug", action="store_false", dest="debug")
    args = parser.parse_args()

    print(f"Watershed Connection Detection")
    print(f"  Map: {MAP_PATH}")
    print(f"  Threshold: {args.threshold}, Kernel: {args.kernel}")

    # Create debug dir
    if args.debug:
        DEBUG_DIR.mkdir(exist_ok=True)

    # 1. Load step1 symbols and step2 locations
    step1 = load_json(STEP1_REVIEWED)
    symbols = step1.get("symbols", [])
    step2 = load_json(STEP2_REVIEWED)
    locations = step2["locations"]
    loc_ids = sorted(locations.keys())
    print(f"  Locations: {len(loc_ids)}, Symbols: {len(symbols)}")

    # 2. Preprocess map
    gray, binary = load_and_preprocess(
        MAP_PATH, args.threshold, args.kernel,
        debug_dir=DEBUG_DIR if args.debug else None,
    )

    # 3. Create markers
    markers, loc_label, label_loc = create_markers(
        binary, locations, loc_ids, seed_radius=args.seed_radius,
    )
    print(f"  Markers planted: {len(loc_label)} locations + wall label")

    # 4. Watershed
    print(f"  Running watershed...")
    labels = run_watershed(binary, markers)

    # Count region sizes
    for loc_id in loc_ids:
        lbl = loc_label[loc_id]
        size = np.count_nonzero(labels == lbl)
        if size == 0:
            print(f"  WARNING: {loc_id} has 0 pixels after watershed")

    # 5. Extract adjacencies
    adjacencies = extract_adjacencies(labels, label_loc)
    print(f"  Connections found: {len(adjacencies)}")
    for pair in sorted(adjacencies, key=lambda p: sorted(p)):
        a, b = sorted(pair)
        print(f"    {a} <-> {b}")

    # 6. Classify connection types
    conn_types = classify_connections(adjacencies, symbols, locations)
    type_counts = {}
    for ct in conn_types.values():
        type_counts[ct] = type_counts.get(ct, 0) + 1
    print(f"  Connection types: {type_counts}")

    # 7. Build and save output
    output = build_step3_output(adjacencies, locations, conn_types)
    with open(OUTPUT_PATH, "w") as f:
        json.dump(output, f, indent=2)
    print(f"\n  Written to {OUTPUT_PATH}")

    # 8. Debug images
    if args.debug:
        save_debug_images(DEBUG_DIR, gray, labels, locations,
                          adjacencies, loc_label)

    # 9. Compare against ground truth
    print(f"\n--- Comparison vs Ground Truth ---")
    result = subprocess.run(
        [sys.executable, str(COMPARE_SCRIPT), str(GROUND_TRUTH), str(OUTPUT_PATH)],
        capture_output=True, text=True,
    )
    print(result.stdout)
    if result.stderr:
        print(result.stderr, file=sys.stderr)


if __name__ == "__main__":
    main()
