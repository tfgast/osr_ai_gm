#!/usr/bin/env python3
"""Step 4: Generate descriptions for locations.

Original rooms are copied verbatim from module_text.json.
The LLM is only called for split rooms (distributing parent text)
and generated locations (junctions, river waypoints).

Usage:
    python step4_descriptions.py <step3_output.json> [output_json] \
        [--module-text FILE] [--step2-json FILE] [--model MODEL]
"""

import argparse
import json
import re
import sys
from pathlib import Path

from llm_api import call_llm_json


def load_prompt() -> str:
    prompt_path = Path(__file__).parent / "prompts" / "step4_descriptions.md"
    return prompt_path.read_text()


def slugify(name: str) -> str:
    """Convert a location name to a snake_case slug."""
    s = name.lower().strip()
    s = re.sub(r"[^a-z0-9]+", "_", s)
    return s.strip("_")


def find_parent_id(split_id: str, module_rooms: dict) -> str | None:
    """Find the parent room ID for a split room.

    Finds the longest module_text room key that is a prefix of the split ID.
    E.g., "8W" -> "8", "13E" -> "13".
    """
    best = None
    for key in module_rooms:
        if split_id.startswith(key) and split_id != key:
            if best is None or len(key) > len(best):
                best = key
    return best


def classify_rooms(
    step3_rooms: dict,
    step2_locations: dict,
    module_rooms: dict,
) -> tuple[dict, dict, dict]:
    """Classify step3 rooms into original, split, and generated.

    Returns:
        (original, split, generated) dicts mapping room_id -> info
    """
    original = {}
    split = {}
    generated = {}

    for room_id in step3_rooms:
        step2_loc = step2_locations.get(room_id, {})
        loc_type = step2_loc.get("type", "")

        if room_id in module_rooms and loc_type == "room":
            original[room_id] = step3_rooms[room_id]
        elif loc_type == "room_split":
            parent_id = step2_loc.get("split_from")
            if not parent_id:
                parent_id = find_parent_id(room_id, module_rooms)
            split[room_id] = {
                **step3_rooms[room_id],
                "parent_id": parent_id,
            }
        else:
            generated[room_id] = step3_rooms[room_id]

    return original, split, generated


def copy_original(room_id: str, step3_entry: dict, module_entry: dict) -> dict:
    """Copy an original room verbatim from module_text."""
    result = {
        "key": slugify(module_entry.get("name", room_id)),
        "name": module_entry["name"],
        "source": "original",
    }
    for field in ("description", "features", "trap", "gm_notes", "tags", "monsters"):
        if field in module_entry:
            result[field] = module_entry[field]
    return result


def build_llm_prompt(
    split_rooms: dict,
    generated_rooms: dict,
    module_rooms: dict,
    step3_data: dict,
    module_data: dict,
) -> str:
    """Build the LLM prompt for split + generated rooms only."""
    prompt_template = load_prompt()

    # General notes and rules from module
    general_notes = module_data.get("general_notes", "")
    rules = module_data.get("rules", {})
    rules_text = json.dumps(rules, indent=2) if rules else "None"

    # Connection subset: only rooms that need LLM help
    llm_room_ids = set(split_rooms) | set(generated_rooms)
    connections_subset = {}
    all_rooms = step3_data.get("rooms", {})
    for room_id in llm_room_ids:
        if room_id in all_rooms:
            connections_subset[room_id] = all_rooms[room_id]

    # Split rooms data: parent room text + split names
    split_data = {}
    for room_id, info in split_rooms.items():
        parent_id = info.get("parent_id")
        parent_entry = module_rooms.get(parent_id, {}) if parent_id else {}
        split_data[room_id] = {
            "name": info.get("name", room_id),
            "parent_id": parent_id,
            "parent_room": parent_entry,
        }

    # Generated rooms data
    gen_data = {}
    for room_id, info in generated_rooms.items():
        gen_data[room_id] = {
            "name": info.get("name", room_id),
            "exits": info.get("exits", []),
        }

    prompt = prompt_template.replace("{general_notes}", general_notes)
    prompt = prompt.replace("{rules}", rules_text)
    prompt = prompt.replace("{connections_subset}", json.dumps(connections_subset, indent=2))
    prompt = prompt.replace("{split_rooms_data}", json.dumps(split_data, indent=2))
    prompt = prompt.replace("{generated_rooms_data}", json.dumps(gen_data, indent=2))

    return prompt


def main():
    parser = argparse.ArgumentParser(description="Step 4: Generate descriptions")
    parser.add_argument("step3_json", help="Reviewed step 3 output JSON (connections)")
    parser.add_argument(
        "output", nargs="?", default="step4_output.json", help="Output JSON path"
    )
    parser.add_argument(
        "--module-text",
        help="JSON file with original room descriptions",
        default=None,
    )
    parser.add_argument(
        "--step2-json",
        help="Reviewed step 2 output JSON (locations with type info)",
        default=None,
    )
    parser.add_argument("--model", default="gemini-2.5-pro", help="Gemini model name")
    args = parser.parse_args()

    if not Path(args.step3_json).exists():
        print(f"Error: step 3 output not found: {args.step3_json}", file=sys.stderr)
        sys.exit(1)

    with open(args.step3_json) as f:
        step3_data = json.load(f)

    # Load module text
    module_data = {}
    module_rooms = {}
    if args.module_text:
        if not Path(args.module_text).exists():
            print(f"Error: module text not found: {args.module_text}", file=sys.stderr)
            sys.exit(1)
        with open(args.module_text) as f:
            module_data = json.load(f)
        module_rooms = module_data.get("rooms", {})

    # Load step2 data
    step2_locations = {}
    if args.step2_json:
        if not Path(args.step2_json).exists():
            print(f"Error: step 2 output not found: {args.step2_json}", file=sys.stderr)
            sys.exit(1)
        with open(args.step2_json) as f:
            step2_data = json.load(f)
        step2_locations = step2_data.get("locations", {})

    step3_rooms = step3_data.get("rooms", {})

    print("Step 4: Generate Descriptions")
    print(f"  Model: {args.model}")
    print(f"  Step 3 input: {args.step3_json}")
    print(f"  Step 2 input: {args.step2_json or 'none'}")
    print(f"  Module text: {args.module_text or 'none'}")
    print(f"  Output: {args.output}")
    print()

    # Classify rooms
    original, split, generated = classify_rooms(
        step3_rooms, step2_locations, module_rooms
    )

    print(f"  Total locations: {len(step3_rooms)}")
    print(f"  Original (verbatim copy): {len(original)}")
    print(f"  Split (LLM distributes parent): {len(split)}")
    print(f"  Generated (LLM creates new): {len(generated)}")
    print()

    # Deterministic copy for original rooms
    locations = {}
    for room_id in original:
        locations[room_id] = copy_original(
            room_id, step3_rooms[room_id], module_rooms[room_id]
        )

    # LLM call for split + generated rooms (only if needed)
    if split or generated:
        print("  Calling LLM for split/generated rooms...")
        prompt = build_llm_prompt(
            split, generated, module_rooms, step3_data, module_data
        )

        llm_result = call_llm_json(
            prompt=prompt,
            model=args.model,
        )

        # Merge LLM results
        llm_locations = llm_result if isinstance(llm_result, dict) else {}
        # Handle both {"locations": {...}} and flat {...} responses
        if "locations" in llm_locations:
            llm_locations = llm_locations["locations"]

        for room_id in list(split) + list(generated):
            if room_id in llm_locations:
                entry = llm_locations[room_id]
                entry["source"] = "generated"
                if "key" not in entry:
                    name = entry.get("name", room_id)
                    entry["key"] = slugify(name)
                locations[room_id] = entry
            else:
                print(f"  Warning: LLM did not return data for {room_id}")
    else:
        print("  No split or generated rooms — skipping LLM call.")

    result = {"locations": locations}

    with open(args.output, "w") as f:
        json.dump(result, f, indent=2)

    print(f"\n  Original preserved: {len(original)}")
    print(f"  LLM-generated: {len(split) + len(generated)}")
    print(f"  Written to: {args.output}")


if __name__ == "__main__":
    main()
