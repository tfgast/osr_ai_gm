#!/usr/bin/env python3
"""Extract JSON from raw gemini output, handling markdown fences and preamble."""

import json
import re
import sys


def extract_json(text: str) -> dict | None:
    """Try to extract a JSON object from text that may contain other content."""
    # Try 1: direct parse
    text = text.strip()
    try:
        return json.loads(text)
    except json.JSONDecodeError:
        pass

    # Try 2: strip markdown code fences
    m = re.search(r"```(?:json)?\s*\n(.*?)```", text, re.DOTALL)
    if m:
        try:
            return json.loads(m.group(1).strip())
        except json.JSONDecodeError:
            pass

    # Try 3: find first { ... last }
    first_brace = text.find("{")
    last_brace = text.rfind("}")
    if first_brace != -1 and last_brace > first_brace:
        try:
            return json.loads(text[first_brace : last_brace + 1])
        except json.JSONDecodeError:
            pass

    return None


def main():
    if len(sys.argv) < 3:
        print("Usage: extract_json.py <input_raw> <output_json>", file=sys.stderr)
        sys.exit(1)

    raw_path = sys.argv[1]
    out_path = sys.argv[2]

    with open(raw_path) as f:
        text = f.read()

    result = extract_json(text)
    if result is None:
        print(f"    WARNING: Could not extract JSON from {raw_path}", file=sys.stderr)
        # Save what we got for debugging
        with open(out_path + ".failed", "w") as f:
            f.write(text)
        sys.exit(1)

    with open(out_path, "w") as f:
        json.dump(result, f, indent=2)
    print(f"    JSON extracted: {out_path}")


if __name__ == "__main__":
    main()
