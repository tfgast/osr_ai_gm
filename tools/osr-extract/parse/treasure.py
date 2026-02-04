#!/usr/bin/env python3
"""
Parse OSE treasure type definitions from docling-extracted markdown.

Usage:
    python treasure.py [input.md] [output.json]

    Defaults:
      input:  ~/.osr_data/extracted/Advanced_Fantasy_Referees_Tome_v1-3.md
      output: stdout (or specify file path)
"""

import json
import re
import sys
from pathlib import Path
from typing import Optional


# Treasure type header pattern
# Matches: "## Type A (18,000gp average)"
TYPE_HEADER_RE = re.compile(r"^## Type ([A-Z]) \(([0-9,]+(?:\.[0-9]+)?)\s*gp average\)$")

# Entry patterns
# Matches: "- 25%: 1d6 × 1,000cp." or "▶ 30%: 1d8 × 1,000pp."
ENTRY_WITH_PERCENT_RE = re.compile(
    r"^[-▶]\s*(\d+)%:\s*(.+?)\.$",
    re.IGNORECASE
)
# Matches: "- 3d8cp." (no percentage, for individual treasure)
ENTRY_NO_PERCENT_RE = re.compile(
    r"^[-▶]\s*(\d+d\d+)\s*(cp|sp|ep|gp|pp)\.$",
    re.IGNORECASE
)

# Quantity patterns
QUANTITY_RE = re.compile(
    r"(\d+d\d+)\s*(?:×\s*([0-9,]+))?\s*(cp|sp|ep|gp|pp|gems?|pieces? of jewellery|jewellery|magic items?|potions?|scrolls?|sword|armou?r|weapon)",
    re.IGNORECASE
)

# Special magic item patterns
MAGIC_ITEM_SPECIAL_RE = re.compile(
    r"(\d+)\s*magic items?(?:\s*\(([^)]+)\))?(?:,?\s*plus\s*(\d+)\s*(potion|scroll))?(?:,?\s*plus\s*(\d+)\s*(potion|scroll))?",
    re.IGNORECASE
)


def parse_average_value(value_str: str) -> float:
    """Parse average value like '18,000' or '0.1' to float."""
    return float(value_str.replace(",", ""))


def normalize_item_type(item_type: str) -> str:
    """Normalize item type names."""
    item_type = item_type.lower().strip()
    if item_type in ("gem", "gems"):
        return "gems"
    if item_type in ("piece of jewellery", "pieces of jewellery", "jewellery"):
        return "jewellery"
    if item_type in ("magic item", "magic items"):
        return "magic_items"
    if item_type in ("potion", "potions"):
        return "potions"
    if item_type in ("scroll", "scrolls"):
        return "scrolls"
    if item_type in ("sword", "armour", "armor", "weapon"):
        return "magic_weapon"  # sword, armour, or weapon
    return item_type


def parse_entry(line: str) -> Optional[dict]:
    """Parse a single treasure entry line."""
    line = line.strip()
    if not line:
        return None

    # Try entry with percentage
    match = ENTRY_WITH_PERCENT_RE.match(line)
    if match:
        percent = int(match.group(1))
        content = match.group(2).strip()

        # Check for special magic item entries like "3 magic items plus 1 potion"
        magic_match = MAGIC_ITEM_SPECIAL_RE.match(content)
        if magic_match:
            entries = []
            count = int(magic_match.group(1))
            restriction = magic_match.group(2)  # e.g., "not weapons"
            entries.append({
                "chance": percent,
                "quantity": str(count),
                "type": "magic_items",
                "restriction": restriction,
            })
            # Plus items
            if magic_match.group(3):
                plus_count = int(magic_match.group(3))
                plus_type = normalize_item_type(magic_match.group(4))
                entries.append({
                    "chance": percent,
                    "quantity": str(plus_count),
                    "type": plus_type,
                })
            if magic_match.group(5):
                plus_count2 = int(magic_match.group(5))
                plus_type2 = normalize_item_type(magic_match.group(6))
                entries.append({
                    "chance": percent,
                    "quantity": str(plus_count2),
                    "type": plus_type2,
                })
            return entries

        # Standard quantity pattern
        qty_match = QUANTITY_RE.search(content)
        if qty_match:
            dice = qty_match.group(1)
            multiplier = qty_match.group(2)
            item_type = normalize_item_type(qty_match.group(3))

            if multiplier:
                quantity = f"{dice} × {multiplier.replace(',', '')}"
            else:
                quantity = dice

            return {
                "chance": percent,
                "quantity": quantity,
                "type": item_type,
            }

        # Fallback: try to parse simpler formats
        # e.g., "1 magic sword, suit of armour, or weapon"
        if "magic" in content.lower():
            return {
                "chance": percent,
                "quantity": "1",
                "type": "magic_weapon",
                "note": content,
            }

    # Try entry without percentage (individual treasure P-T)
    match = ENTRY_NO_PERCENT_RE.match(line)
    if match:
        dice = match.group(1)
        coin_type = match.group(2).lower()
        return {
            "chance": 100,  # Always present for individual treasure
            "quantity": dice,
            "type": coin_type,
        }

    return None


def determine_category(letter: str) -> str:
    """Determine treasure category based on type letter."""
    if letter in "ABCDEFGHIJKLMNO":
        return "hoard"
    elif letter in "PQRST":
        return "individual"
    elif letter in "UV":
        return "group"
    return "unknown"


def extract_treasure_types(markdown_path: Path) -> list[dict]:
    """Extract all treasure types from markdown file."""
    text = markdown_path.read_text()
    lines = text.split("\n")

    treasure_types = []
    current_type = None
    current_entries = []

    for line in lines:
        line_stripped = line.strip()

        # Check for treasure type header
        header_match = TYPE_HEADER_RE.match(line_stripped)
        if header_match:
            # Save previous type
            if current_type:
                current_type["entries"] = current_entries
                treasure_types.append(current_type)

            letter = header_match.group(1)
            avg_value = parse_average_value(header_match.group(2))

            current_type = {
                "letter": letter,
                "average_gp": avg_value,
                "category": determine_category(letter),
                "entries": [],
            }
            current_entries = []
            continue

        # Parse entries for current type
        if current_type and (line_stripped.startswith("-") or line_stripped.startswith("▶")):
            entry = parse_entry(line_stripped)
            if entry:
                if isinstance(entry, list):
                    current_entries.extend(entry)
                else:
                    current_entries.append(entry)

    # Save last type
    if current_type:
        current_type["entries"] = current_entries
        treasure_types.append(current_type)

    return treasure_types


def main():
    # Default paths
    default_input = Path.home() / ".osr_data/extracted/Advanced_Fantasy_Referees_Tome_v1-3.md"

    input_path = Path(sys.argv[1]) if len(sys.argv) > 1 else default_input
    output_path = Path(sys.argv[2]) if len(sys.argv) > 2 else None

    if not input_path.exists():
        print(f"Error: Input file not found: {input_path}", file=sys.stderr)
        print(f"Run extract.sh first to generate the markdown file.", file=sys.stderr)
        sys.exit(1)

    treasure_types = extract_treasure_types(input_path)

    # Group by category for summary
    by_category = {}
    for t in treasure_types:
        by_category.setdefault(t["category"], []).append(t["letter"])

    result = {
        "source": input_path.name,
        "count": len(treasure_types),
        "by_category": {k: len(v) for k, v in by_category.items()},
        "treasure_types": treasure_types
    }

    output = json.dumps(result, indent=2)

    if output_path:
        output_path.write_text(output)
        print(f"Wrote {len(treasure_types)} treasure types to {output_path}", file=sys.stderr)
        for cat_name, letters in by_category.items():
            print(f"  {cat_name}: {', '.join(letters)}", file=sys.stderr)
    else:
        print(output)


if __name__ == "__main__":
    main()
