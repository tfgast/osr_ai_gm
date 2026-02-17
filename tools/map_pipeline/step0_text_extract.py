#!/usr/bin/env python3
"""Step 0: Extract room info from module text.

Extracts room names, summaries, spatial clues, and expected map symbols
from the module text. This provides context for step 1 (vision-based
feature identification).

Usage:
    python step0_text_extract.py <module_text.json> [output_json] [--model MODEL]

    output_json defaults to step0_output.json in the current directory.
    module_text.json should be a JSON file with room descriptions keyed by
    room number (as produced by the module text extraction script).
"""

import argparse
import json
import sys
from pathlib import Path

from llm_api import call_llm_json


def load_prompt() -> str:
    prompt_path = Path(__file__).parent / "prompts" / "step0_text_extract.md"
    return prompt_path.read_text()


def main():
    parser = argparse.ArgumentParser(description="Step 0: Extract room info from text")
    parser.add_argument("module_text", help="JSON file with module room descriptions")
    parser.add_argument(
        "output", nargs="?", default="step0_output.json", help="Output JSON path"
    )
    parser.add_argument("--model", default="gemini-2.5-pro", help="Gemini model name")
    args = parser.parse_args()

    if not Path(args.module_text).exists():
        print(f"Error: module text not found: {args.module_text}", file=sys.stderr)
        sys.exit(1)

    with open(args.module_text) as f:
        module_text = f.read()

    prompt_template = load_prompt()
    prompt = prompt_template.replace("{module_text}", module_text)

    print(f"Step 0: Text Extraction")
    print(f"  Model: {args.model}")
    print(f"  Input: {args.module_text}")
    print(f"  Output: {args.output}")
    print()

    result = call_llm_json(
        prompt=prompt,
        model=args.model,
        # No image needed — text only
    )

    areas = result.get("areas", {})
    on_map = sum(1 for a in areas.values() if a.get("on_map", True))
    off_map = sum(1 for a in areas.values() if not a.get("on_map", True))
    print(f"  Areas extracted: {len(areas)}")
    print(f"  On map: {on_map}")
    print(f"  Off map: {off_map}")
    print(f"  Area labels: {sorted(areas.keys())}")

    dungeon_info = result.get("dungeon_info", {})
    if dungeon_info:
        print(f"  Dungeon: {dungeon_info.get('name', 'unknown')}")

    with open(args.output, "w") as f:
        json.dump(result, f, indent=2)
    print(f"\n  Written to: {args.output}")
    print(f"\n  Next: Review {args.output}, then run step1_features.py with it")


if __name__ == "__main__":
    main()
