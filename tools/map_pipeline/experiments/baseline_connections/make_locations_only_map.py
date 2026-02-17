#!/usr/bin/env python3
"""Create an annotated map with location circles only (no symbol diamonds).

Uses the same drawing logic as annotate_map.py but strips all symbols.
Intended for use with the unlabeled map as base to test whether
location-only annotations are sufficient for connection detection.
"""

import json
import sys
from pathlib import Path

# Add parent dirs to path for annotate_map import
sys.path.insert(0, str(Path(__file__).parent.parent.parent))
from annotate_map import annotate_map


def main():
    if len(sys.argv) < 3:
        print("Usage: make_locations_only_map.py <base_map> <step2_json> [output]", file=sys.stderr)
        sys.exit(1)

    base_map = sys.argv[1]
    step2_json = sys.argv[2]
    output = sys.argv[3] if len(sys.argv) > 3 else "annotated_locations_only.png"

    with open(step2_json) as f:
        data = json.load(f)

    # Strip symbols — keep only locations
    locations = data.get("locations", {})
    stripped_data = {
        "locations": locations,
        "symbols": [],
        "map_features": data.get("map_features", {}),
    }

    print(f"Creating locations-only annotated map")
    print(f"  Base: {base_map}")
    print(f"  Locations: {len(locations)}")
    print(f"  Symbols: 0 (stripped)")
    print(f"  Output: {output}")

    size = annotate_map(base_map, stripped_data, output)
    print(f"  Written: {output} ({size[0]}x{size[1]})")


if __name__ == "__main__":
    main()
