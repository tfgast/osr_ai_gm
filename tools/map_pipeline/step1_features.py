#!/usr/bin/env python3
"""Step 1: Feature identification.

Identifies and locates rooms, symbols, and map features on a dungeon map.

Usage:
    python step1_features.py <map_image> [output_json] [--step0 FILE] [--labels FILE]
                             [--model MODEL] [--refine]

    output_json defaults to step1_output.json in the current directory.
    --step0 provides room context extracted from module text (step 0 output).
    --labels provides known room positions from diff-based extraction (step 1a).
             When provided, the LLM only searches for symbols (doors, stairs, traps).
    --refine runs an additional pass: annotates the map with initial coordinates,
             then asks the model to review and correct its own placement.
"""

import argparse
import json
import subprocess
import sys
import tempfile
from pathlib import Path

from llm_api import call_llm_json


def load_prompt(name: str) -> str:
    prompt_path = Path(__file__).parent / "prompts" / name
    return prompt_path.read_text()


def annotate_map(map_image: str, step_json: dict, output_path: str) -> None:
    """Run annotate_map.py to overlay markers on the map."""
    # Write temp JSON for annotate_map to read
    with tempfile.NamedTemporaryFile(
        mode="w", suffix=".json", delete=False
    ) as tmp:
        json.dump(step_json, tmp, indent=2)
        tmp_json = tmp.name

    cmd = [
        sys.executable,
        str(Path(__file__).parent / "annotate_map.py"),
        map_image,
        tmp_json,
        output_path,
    ]
    try:
        subprocess.run(cmd, check=True)
    finally:
        Path(tmp_json).unlink(missing_ok=True)


def main():
    parser = argparse.ArgumentParser(description="Step 1: Feature identification")
    parser.add_argument("map_image", help="Path to dungeon map image (PNG/JPG)")
    parser.add_argument(
        "output", nargs="?", default="step1_output.json", help="Output JSON path"
    )
    parser.add_argument(
        "--step0",
        help="Step 0 output JSON (room context from module text)",
        default=None,
    )
    parser.add_argument(
        "--labels",
        help="Known room positions from diff-based extraction (step 1a output). "
        "When provided, the LLM only searches for symbols.",
        default=None,
    )
    parser.add_argument("--model", default="gemini-2.5-pro", help="Gemini model name")
    parser.add_argument(
        "--refine",
        action="store_true",
        help="Run a refinement pass: annotate map with initial results, "
        "then ask the model to review and correct coordinates",
    )
    args = parser.parse_args()

    if not Path(args.map_image).exists():
        print(f"Error: map image not found: {args.map_image}", file=sys.stderr)
        sys.exit(1)

    # Load known labels from diff-based extraction
    labels_data = None
    if args.labels:
        if not Path(args.labels).exists():
            print(f"Error: labels file not found: {args.labels}", file=sys.stderr)
            sys.exit(1)
        with open(args.labels) as f:
            labels_data = json.load(f)

    # Choose prompt based on whether labels are provided
    if labels_data:
        prompt = load_prompt("step1_symbols_only.md")
        # Build known labels summary for the prompt
        known_locations = labels_data.get("locations", {})
        label_lines = []
        for label in sorted(known_locations.keys(),
                            key=lambda l: (not l[0].isdigit(), l.zfill(5))):
            loc = known_locations[label]
            name = loc.get("name", loc.get("name_guess", ""))
            name_part = f' — "{name}"' if name else ""
            label_lines.append(
                f"  - Room {label}{name_part}: pixel ({loc['pixel_x']}, {loc['pixel_y']})"
            )
        prompt = prompt.replace("{known_labels}", "\n".join(label_lines))
    else:
        prompt = load_prompt("step1_features.md")

    # Inject step 0 context if provided
    if args.step0:
        if not Path(args.step0).exists():
            print(f"Error: step 0 output not found: {args.step0}", file=sys.stderr)
            sys.exit(1)
        with open(args.step0) as f:
            step0_data = json.load(f)
        step0_section = json.dumps(step0_data, indent=2)
        prompt = prompt.replace(
            "{step0_context}",
            f"The following room information was extracted from the module text. "
            f"Use it to help identify and name rooms on the map, and to know what "
            f"symbols to look for:\n\n```json\n{step0_section}\n```",
        )
    else:
        prompt = prompt.replace("{step0_context}", "No module text context provided.")

    mode = "symbols-only" if labels_data else "full"
    print(f"Step 1: Feature Identification ({mode})")
    print(f"  Model:  {args.model}")
    print(f"  Image:  {args.map_image}")
    print(f"  Step 0: {args.step0 or 'none'}")
    print(f"  Labels: {args.labels or 'none'}")
    print(f"  Refine: {args.refine}")
    print(f"  Output: {args.output}")
    print()

    # --- Initial pass ---
    print("  Pass 1: Initial feature identification...")
    result = call_llm_json(
        prompt=prompt,
        image_path=args.map_image,
        model=args.model,
    )

    # If labels provided, merge them into the result
    if labels_data:
        result["locations"] = labels_data["locations"]
        # Preserve diff-based dimensions if the LLM reported different ones
        if "map_features" in labels_data:
            diff_dims = labels_data["map_features"].get("dimensions")
            if diff_dims:
                result.setdefault("map_features", {})["dimensions"] = diff_dims

    locations = result.get("locations", {})
    symbols = result.get("symbols", [])
    print(f"  Locations: {len(locations)}")
    if not labels_data:
        print(f"  Location IDs: {sorted(locations.keys())}")
    print(f"  Symbols found: {len(symbols)}")

    # --- Refinement pass ---
    if args.refine:
        print()
        print("  Pass 2: Self-correction refinement...")

        # Annotate the map with initial coordinates
        with tempfile.NamedTemporaryFile(suffix=".png", delete=False) as tmp_img:
            annotated_path = tmp_img.name

        annotate_map(args.map_image, result, annotated_path)
        print(f"  Annotated map: {annotated_path}")

        # Build refinement prompt
        refine_prompt = load_prompt("step1_refine.md")
        refine_prompt = refine_prompt.replace(
            "{previous_json}", json.dumps(result, indent=2)
        )

        result = call_llm_json(
            prompt=refine_prompt,
            image_path=annotated_path,
            model=args.model,
        )

        # Clean up temp file
        Path(annotated_path).unlink(missing_ok=True)

        locations = result.get("locations", {})
        symbols = result.get("symbols", [])
        print(f"  Locations after refinement: {len(locations)}")
        print(f"  Symbols after refinement: {len(symbols)}")

    with open(args.output, "w") as f:
        json.dump(result, f, indent=2)
    print(f"\n  Written to: {args.output}")
    print(f"\n  Next: Review {args.output}, then run step2_locations.py")


if __name__ == "__main__":
    main()
