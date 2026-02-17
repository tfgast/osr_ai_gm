#!/usr/bin/env python3
"""Step 2: Add unlabeled locations.

Suggests junctions, river waypoints, and room splits not on the original map
but important for player navigation.

Usage:
    python step2_locations.py <map_image> <step1_output.json> [output_json] [--model MODEL]

    output_json defaults to step2_output.json in the current directory.
"""

import argparse
import json
import sys
from pathlib import Path

from llm_api import call_llm_json


def load_prompt() -> str:
    prompt_path = Path(__file__).parent / "prompts" / "step2_locations.md"
    return prompt_path.read_text()


def main():
    parser = argparse.ArgumentParser(description="Step 2: Add unlabeled locations")
    parser.add_argument("map_image", help="Path to dungeon map image (PNG/JPG)")
    parser.add_argument("step1_json", help="Reviewed step 1 output JSON")
    parser.add_argument(
        "output", nargs="?", default="step2_output.json", help="Output JSON path"
    )
    parser.add_argument("--model", default="gemini-2.5-pro", help="Gemini model name")
    args = parser.parse_args()

    if not Path(args.map_image).exists():
        print(f"Error: map image not found: {args.map_image}", file=sys.stderr)
        sys.exit(1)
    if not Path(args.step1_json).exists():
        print(f"Error: step 1 output not found: {args.step1_json}", file=sys.stderr)
        sys.exit(1)

    with open(args.step1_json) as f:
        step1_data = json.load(f)

    prompt_template = load_prompt()
    prompt = prompt_template.replace("{step1_json}", json.dumps(step1_data, indent=2))

    print(f"Step 2: Add Unlabeled Locations")
    print(f"  Model: {args.model}")
    print(f"  Image: {args.map_image}")
    print(f"  Step 1 input: {args.step1_json}")
    print(f"  Output: {args.output}")
    print()

    result = call_llm_json(
        prompt=prompt,
        image_path=args.map_image,
        model=args.model,
    )

    # Preserve map_features from step 1 (Gemini may drop or empty it)
    if "map_features" in step1_data:
        result.setdefault("map_features", {})
        for k, v in step1_data["map_features"].items():
            if k not in result["map_features"] or not result["map_features"][k]:
                result["map_features"][k] = v

    locations = result.get("locations", {})
    added = result.get("added_locations", [])
    print(f"  Total locations: {len(locations)}")
    print(f"  New locations added: {len(added)}")
    if added:
        print(f"  New IDs: {added}")

    with open(args.output, "w") as f:
        json.dump(result, f, indent=2)
    print(f"\n  Written to: {args.output}")
    print(f"\n  Next: Review {args.output}, then run annotate_map.py + step3_connections.py")


if __name__ == "__main__":
    main()
