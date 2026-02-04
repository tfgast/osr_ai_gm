#!/usr/bin/env python3
"""
Parse OSE magic item definitions from docling-extracted markdown.

Usage:
    python magic_items.py [input.md] [output.json]

    Defaults:
      input:  ~/.osr_data/extracted/Advanced_Fantasy_Referees_Tome_v1-3.md
      output: stdout (or specify file path)
"""

import json
import re
import sys
from pathlib import Path
from typing import Optional


# Magic item category section markers with their line ranges
# These will be populated by scanning the document
CATEGORY_HEADERS = {
    "Armour and Shields": "armor",
    "Miscellaneous Items": "miscellaneous",
    "Potions": "potion",
    "Rings": "ring",
    "Rods, Staves, Wands": "rod_staff_wand",  # Combined section, split by item name
    # Note: "Rods", "Staves", "Wands" are subsection headers, not standalone categories
    "Scrolls and Maps": "scroll",
    "Swords": "sword",
    "Weapons": "weapon",
}

# Headers to skip (tables, general rules, not individual items)
SKIP_PATTERNS = [
    r"^Rolling",
    r"^Basic and Expert",
    r"^Identifying",
    r"^Using Magic",
    r"^Command Words",
    r"^Wishes$",
    r"^Adjudication",
    r"^Example",
    r"^Gems",
    r"^Jewellery",
    r"^Damaged",
    r"^Combining",
    r"^Magic Items$",
    r"^Armour and Shields$",
    r"^Miscellaneous Items$",
    r"^Miscellaneous Magic Items",
    r"^Usage$",
    r"^Potions$",
    r"^Rings$",
    r"^Rods$",
    r"^Staves$",
    r"^Wands$",
    r"^Scrolls and Maps$",
    r"^Swords$",
    r"^Weapons$",
    r"^Type of Armour",
    r"^Cursed Armour and Shields$",
    r"^Enchanted Armour and Shields$",
    r"^Enchanted Armour$",
    r"^Enchanted Shields$",
    r"^Potion Descriptions",
    r"^Mixing Potions",
    r"^Ring Descriptions",
    r"^Scroll Descriptions",
    r"^Protection Scrolls$",
    r"^Spell Scrolls$",
    r"^Treasure Maps$",
    r"^Sword Descriptions",
    r"^Sentient Swords$",
    r"^Swords With a Special Purpose",
    r"^Rolling a Sentient",
    r"^Languages Spoken",
    r"^Alignment$",
    r"^Special Purpose$",
    r"^Powers$",
    r"^Ego$",
    r"^Conflict Between",
    r"^Weapon Descriptions",
    r"^General Properties",
    r"^Determining Weapon",
    r"^Charges$",
    r"^Cursed Scrolls$",
    r"^Cursed Swords$",
    r"^Enchanted Swords$",
    r"^Cursed Weapons$",
    r"^Enchanted Weapons$",
    r"^Type ",
    r"^Guardians$",
    r"^Commanding Animals$",
    r"^Commanding Humans$",
    r"^Commanding Plants$",
    r"^Rods, Staves, Wands$",
]


def should_skip(name: str) -> bool:
    """Check if this header should be skipped."""
    for pattern in SKIP_PATTERNS:
        if re.match(pattern, name):
            return True
    return False


def parse_item_block(lines: list[str], item_name: str, category: str) -> dict:
    """Parse a single magic item's content block."""
    item = {
        "name": item_name,
        "category": category,
        "description": None,
        "properties": [],
        "cursed": "cursed" in item_name.lower(),
    }

    description_lines = []
    current_property = None

    for line in lines:
        line = line.strip()
        if not line:
            continue

        # Skip image references
        if line.startswith("![Image]"):
            continue

        # Skip table rows
        if line.startswith("|") and "|" in line[1:]:
            continue

        # Check for bullet point (property)
        if line.startswith("- "):
            # Save previous property
            if current_property:
                item["properties"].append(current_property)

            # Parse new property
            prop_text = line[2:].strip()
            if ":" in prop_text:
                key, _, value = prop_text.partition(":")
                current_property = {"key": key.strip(), "value": value.strip()}
            else:
                current_property = {"key": None, "value": prop_text}
        elif current_property and line and not line.startswith("#"):
            # Continuation of previous property
            current_property["value"] += " " + line
        elif not line.startswith("#"):
            # Regular description text
            description_lines.append(line)

    # Save last property
    if current_property:
        item["properties"].append(current_property)

    # Join description lines
    item["description"] = " ".join(description_lines).strip() if description_lines else None

    return item


def extract_magic_items(markdown_path: Path) -> list[dict]:
    """Extract all magic items from markdown file."""
    text = markdown_path.read_text()
    lines = text.split("\n")

    # First pass: find all category section boundaries
    category_ranges = []
    for i, line in enumerate(lines):
        stripped = line.strip()
        if stripped.startswith("## "):
            header = stripped[3:].strip()
            if header in CATEGORY_HEADERS:
                category_ranges.append((i, CATEGORY_HEADERS[header]))

    # Sort by line number and compute end ranges
    category_ranges.sort(key=lambda x: x[0])
    ranges_with_end = []
    for idx, (start, cat) in enumerate(category_ranges):
        if idx + 1 < len(category_ranges):
            end = category_ranges[idx + 1][0]
        else:
            # Last category - find where monsters section starts or end of file
            end = len(lines)
            for j in range(start, len(lines)):
                if lines[j].strip() in ["# Monsters", "## Monsters", "# Part 5: Monsters"]:
                    end = j
                    break
        ranges_with_end.append((start, end, cat))

    # Second pass: extract items within each category range
    items = []

    for range_start, range_end, category in ranges_with_end:
        current_item_name = None
        current_item_lines = []

        for i in range(range_start, range_end):
            line = lines[i].strip()

            # Check for item header
            if line.startswith("## "):
                header = line[3:].strip()

                # Save previous item
                if current_item_name:
                    item = parse_item_block(current_item_lines, current_item_name, category)
                    if item["description"] or item["properties"]:
                        items.append(item)

                # Check if this is a valid item header
                if should_skip(header):
                    current_item_name = None
                    current_item_lines = []
                else:
                    current_item_name = header
                    current_item_lines = []
            elif current_item_name:
                current_item_lines.append(line)

        # Save last item in category
        if current_item_name:
            item = parse_item_block(current_item_lines, current_item_name, category)
            if item["description"] or item["properties"]:
                items.append(item)

    return items


def main():
    # Default paths
    default_input = Path.home() / ".osr_data/extracted/Advanced_Fantasy_Referees_Tome_v1-3.md"

    input_path = Path(sys.argv[1]) if len(sys.argv) > 1 else default_input
    output_path = Path(sys.argv[2]) if len(sys.argv) > 2 else None

    if not input_path.exists():
        print(f"Error: Input file not found: {input_path}", file=sys.stderr)
        print(f"Run extract.sh first to generate the markdown file.", file=sys.stderr)
        sys.exit(1)

    items = extract_magic_items(input_path)

    # Post-process: split rod_staff_wand category by item name prefix
    for item in items:
        if item["category"] == "rod_staff_wand":
            name = item["name"].lower()
            if name.startswith("rod ") or name.startswith("immovable rod"):
                item["category"] = "rod"
            elif name.startswith("staff "):
                item["category"] = "staff"
            elif name.startswith("wand "):
                item["category"] = "wand"
            # else keep as rod_staff_wand (shouldn't happen)

    # Group by category for summary
    by_category = {}
    for item in items:
        by_category.setdefault(item["category"], []).append(item)

    result = {
        "source": input_path.name,
        "count": len(items),
        "by_category": {k: len(v) for k, v in by_category.items()},
        "items": items
    }

    output = json.dumps(result, indent=2)

    if output_path:
        output_path.write_text(output)
        print(f"Wrote {len(items)} magic items to {output_path}", file=sys.stderr)
        for cat_name, cat_items in by_category.items():
            print(f"  {cat_name}: {len(cat_items)}", file=sys.stderr)
    else:
        print(output)


if __name__ == "__main__":
    main()
