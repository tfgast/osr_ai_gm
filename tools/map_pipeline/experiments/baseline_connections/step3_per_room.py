#!/usr/bin/env python3
"""Step 3 experiment: per-room connection queries.

Instead of one big prompt with all 27 locations, sends one LLM call per
location with K=10 nearest candidates.  Aggregates results via bidirectional
voting (unanimous and any variants).

Usage:
    python step3_per_room.py [--model MODEL] [--k K]
"""

import argparse
import json
import math
import sys
from collections import defaultdict
from pathlib import Path

# Resolve paths relative to the map_pipeline root
SCRIPT_DIR = Path(__file__).resolve().parent
PIPELINE_DIR = SCRIPT_DIR.parent.parent
TOOLS_DIR = PIPELINE_DIR.parent
PROJECT_DIR = TOOLS_DIR.parent

sys.path.insert(0, str(PIPELINE_DIR))
from llm_api import call_llm_json

# --- Paths ---
STEP2_REVIEWED = PIPELINE_DIR / "pipeline_output" / "step2_output_reviewed.json"
GROUND_TRUTH = PIPELINE_DIR / "pipeline_output" / "step3_output_reviewed.json"
ANNOTATED_MAP = SCRIPT_DIR / "annotated_locations_only.png"
PROMPT_TEMPLATE = SCRIPT_DIR / "prompt_per_room.md"
RAW_DIR = SCRIPT_DIR / "per_room_raw"
COMPARE_SCRIPT = TOOLS_DIR / "map_connection_experiment" / "compare.py"

# --- Location color map (must match annotate_map.py) ---
TYPE_COLORS = {
    "room": "blue",
    "junction": "green",
    "river_waypoint": "red",
    "room_split": "orange",
}

# Connection type specificity for conflict resolution (higher = prefer)
TYPE_SPECIFICITY = {
    "secret": 6,
    "pit": 5,
    "stairs": 4,
    "door": 3,
    "river_crossing": 2,
    "open": 1,
}


def load_json(path):
    with open(path) as f:
        return json.load(f)


def pixel_dist(a, b):
    """Euclidean pixel distance between two location dicts."""
    return math.sqrt((a["pixel_x"] - b["pixel_x"])**2 +
                     (a["pixel_y"] - b["pixel_y"])**2)


def get_k_nearest(target_id, locations, k=10):
    """Return K nearest location IDs by pixel distance.

    River waypoints always see all other river waypoints regardless of distance.
    """
    target = locations[target_id]
    target_type = target.get("type", "room")

    distances = []
    for loc_id, loc in locations.items():
        if loc_id == target_id:
            continue
        distances.append((pixel_dist(target, loc), loc_id))
    distances.sort()

    # Start with K nearest
    candidates = set()
    for _, loc_id in distances[:k]:
        candidates.add(loc_id)

    # River augmentation: river waypoints always see all other river waypoints
    if target_type == "river_waypoint":
        for loc_id, loc in locations.items():
            if loc_id != target_id and loc.get("type") == "river_waypoint":
                candidates.add(loc_id)
    else:
        # Non-river locations also see any river waypoints in their K-nearest,
        # plus augment with all river waypoints if any are already in K-nearest
        has_river_neighbor = any(
            locations[c].get("type") == "river_waypoint" for c in candidates
        )
        if has_river_neighbor:
            for loc_id, loc in locations.items():
                if loc_id != target_id and loc.get("type") == "river_waypoint":
                    candidates.add(loc_id)

    return sorted(candidates)


def format_candidates_list(candidate_ids, locations):
    """Format candidate list for the prompt."""
    lines = []
    for cid in candidate_ids:
        loc = locations[cid]
        name = loc.get("name_guess", loc.get("name", ""))
        loc_type = loc.get("type", "room")
        color = TYPE_COLORS.get(loc_type, "blue")
        lines.append(f"- **{cid}** ({loc_type}, {color} circle): {name}")
    return "\n".join(lines)


def build_prompt(target_id, locations, k=10):
    """Build a per-room prompt for the given target location."""
    template = PROMPT_TEMPLATE.read_text()
    target = locations[target_id]
    target_type = target.get("type", "room")
    target_name = target.get("name_guess", target.get("name", ""))
    target_color = TYPE_COLORS.get(target_type, "blue")

    candidate_ids = get_k_nearest(target_id, locations, k=k)
    candidates_list = format_candidates_list(candidate_ids, locations)

    prompt = template.replace("{target_id}", target_id)
    prompt = prompt.replace("{target_name}", target_name)
    prompt = prompt.replace("{target_type}", target_type)
    prompt = prompt.replace("{target_color}", target_color)
    prompt = prompt.replace("{candidates_list}", candidates_list)

    return prompt


def query_room(target_id, locations, model, k=10):
    """Send a per-room query and return parsed result."""
    prompt = build_prompt(target_id, locations, k=k)
    result = call_llm_json(
        prompt=prompt,
        image_path=str(ANNOTATED_MAP),
        model=model,
    )
    return result


def aggregate_connections(raw_results, locations):
    """Aggregate per-room results into unanimous and any connection sets.

    Returns:
        (unanimous_data, any_data) — both in step3 output format
    """
    # Collect all claimed connections with metadata
    # Key: frozenset({A, B}), Value: dict with direction info
    claims = defaultdict(lambda: {"directions": [], "types": [], "confidences": [], "notes": []})

    for target_id, result in raw_results.items():
        for conn in result.get("connections", []):
            to_id = conn["to"].strip().upper()
            from_id = target_id.strip().upper()
            key = frozenset({from_id, to_id})
            claims[key]["directions"].append(from_id)
            claims[key]["types"].append(
                (conn.get("connection_type", "open")).lower()
            )
            claims[key]["confidences"].append(
                (conn.get("confidence", "medium")).lower()
            )
            claims[key]["notes"].append(conn.get("notes", ""))

    # Build connection sets
    unanimous = {}  # both directions claimed
    any_conn = {}   # at least one direction claimed

    for pair, info in claims.items():
        a, b = sorted(pair)
        n_directions = len(set(info["directions"]))

        # Resolve connection type: prefer more specific
        best_type = max(info["types"], key=lambda t: TYPE_SPECIFICITY.get(t, 0))
        best_note = info["notes"][0] if info["notes"] else ""

        conn_entry = {
            "connection_type": best_type,
            "notes": best_note,
            "directions_claimed": n_directions,
        }

        any_conn[(a, b)] = conn_entry
        if n_directions >= 2:
            unanimous[(a, b)] = conn_entry

    # Build step3-format output
    def build_output(conn_dict, label):
        rooms = {}
        for loc_id, loc in sorted(locations.items()):
            lid = loc_id.strip().upper()
            name = loc.get("name_guess", loc.get("name", ""))
            exits = []
            for (a, b), entry in sorted(conn_dict.items()):
                if a == lid:
                    exits.append({
                        "to": b,
                        "connection_type": entry["connection_type"],
                        "notes": entry["notes"],
                    })
                elif b == lid:
                    exits.append({
                        "to": a,
                        "connection_type": entry["connection_type"],
                        "notes": entry["notes"],
                    })
            rooms[lid] = {"name": name, "exits": exits}
        return {
            "rooms": rooms,
            "observations": f"Per-room queries aggregated with {label} voting.",
        }

    return build_output(unanimous, "unanimous"), build_output(any_conn, "any")


def main():
    parser = argparse.ArgumentParser(
        description="Step 3 per-room connection queries"
    )
    parser.add_argument("--model", default="opus", help="Model name (default: opus)")
    parser.add_argument("--k", type=int, default=10, help="K nearest candidates (default: 10)")
    args = parser.parse_args()

    # Validate inputs
    if not ANNOTATED_MAP.exists():
        print(f"Error: annotated map not found: {ANNOTATED_MAP}", file=sys.stderr)
        print(f"  Run: python make_locations_only_map.py first", file=sys.stderr)
        sys.exit(1)
    if not STEP2_REVIEWED.exists():
        print(f"Error: step2 output not found: {STEP2_REVIEWED}", file=sys.stderr)
        sys.exit(1)

    step2 = load_json(STEP2_REVIEWED)
    locations = step2.get("locations", {})

    print(f"=== Per-Room Connection Queries ===")
    print(f"  Model: {args.model}")
    print(f"  K nearest: {args.k}")
    print(f"  Locations: {len(locations)}")
    print(f"  Map: {ANNOTATED_MAP}")
    print()

    # Create raw output directory
    RAW_DIR.mkdir(exist_ok=True)

    # Query each room
    raw_results = {}
    loc_ids = sorted(locations.keys())

    for i, loc_id in enumerate(loc_ids, 1):
        raw_path = RAW_DIR / f"{loc_id}.json"

        # Resume support: skip if already done
        if raw_path.exists():
            print(f"  [{i}/{len(loc_ids)}] {loc_id}: cached")
            raw_results[loc_id] = load_json(raw_path)
            continue

        loc = locations[loc_id]
        name = loc.get("name_guess", loc.get("name", ""))
        candidates = get_k_nearest(loc_id, locations, k=args.k)
        print(f"  [{i}/{len(loc_ids)}] {loc_id} ({name}) — {len(candidates)} candidates")

        try:
            result = query_room(loc_id, locations, args.model, k=args.k)
            raw_results[loc_id] = result

            with open(raw_path, "w") as f:
                json.dump(result, f, indent=2)
            n_conn = len(result.get("connections", []))
            print(f"           → {n_conn} connections found")
        except Exception as e:
            print(f"           → ERROR: {e}", file=sys.stderr)
            continue

    print(f"\n  Completed: {len(raw_results)}/{len(loc_ids)} rooms")

    # Aggregate
    print(f"\n--- Aggregating with bidirectional voting ---")
    unanimous_data, any_data = aggregate_connections(raw_results, locations)

    unanimous_conns = sum(len(r.get("exits", [])) for r in unanimous_data["rooms"].values()) // 2
    any_conns = sum(len(r.get("exits", [])) for r in any_data["rooms"].values()) // 2
    print(f"  Unanimous (both directions): {unanimous_conns} connections")
    print(f"  Any (at least one direction): {any_conns} connections")

    # Save aggregated outputs
    unanimous_path = SCRIPT_DIR / "step3_per_room_unanimous.json"
    any_path = SCRIPT_DIR / "step3_per_room_any.json"

    with open(unanimous_path, "w") as f:
        json.dump(unanimous_data, f, indent=2)
    print(f"\n  Written: {unanimous_path}")

    with open(any_path, "w") as f:
        json.dump(any_data, f, indent=2)
    print(f"  Written: {any_path}")

    # Compare against ground truth
    if GROUND_TRUTH.exists() and COMPARE_SCRIPT.exists():
        print(f"\n--- Comparison: Unanimous ---")
        import subprocess
        subprocess.run(
            [sys.executable, str(COMPARE_SCRIPT), str(GROUND_TRUTH), str(unanimous_path)]
        )
        print(f"\n--- Comparison: Any ---")
        subprocess.run(
            [sys.executable, str(COMPARE_SCRIPT), str(GROUND_TRUTH), str(any_path)]
        )


if __name__ == "__main__":
    main()
