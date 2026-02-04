#!/usr/bin/env python3
"""
Parse OSE spell definitions from docling-extracted markdown.

Usage:
    python spells.py [input.md] [output.json]

    Defaults:
      input:  ~/.osr_data/extracted/Advanced_Fantasy_Players_Tome_v1-3.md
      output: stdout (or specify file path)
"""

import json
import re
import sys
from pathlib import Path
from typing import Optional


# Spell list section markers
SPELL_SECTIONS = {
    "Cleric Spells": "cleric",
    "Druid Spells": "druid",
    "Illusionist Spells": "illusionist",
    "Magic-User Spells": "magic_user",
}

# Sections that mark the end of spell listings
END_SECTIONS = [
    "Adventuring",
    "Equipment",
    "Monsters",
    "Magic Items",
    "Treasure",
    "Dungeon Adventures",
    "Wilderness Adventures",
    "Alternative Reincarnation Tables",  # Reference table, not a spell
    "Reincarnation: Neutral Monsters",   # Reference table
    "Reincarnation: Chaotic Monsters",   # Reference table
]

# Level header pattern
LEVEL_RE = re.compile(r"^##\s*(\d+)(?:st|nd|rd|th)\s+Level(?:\s+Spells)?$", re.IGNORECASE)

# Duration/Range patterns - handles both single line and separate lines
DURATION_RE = re.compile(r"Duration:\s*(.+?)(?=\s+Range:|$)", re.IGNORECASE)
RANGE_RE = re.compile(r"Range:\s*(.+)", re.IGNORECASE)
COMBINED_RE = re.compile(r"##?\s*Duration:\s*(.+?)\s+Range:\s*(.+)", re.IGNORECASE)


def parse_spell_block(lines: list[str], spell_name: str, spell_list: str, level: int) -> dict:
    """Parse a single spell's content block."""
    spell = {
        "name": spell_name,
        "list": spell_list,
        "level": level,
        "duration": None,
        "range": None,
        "description": [],
        "reversible": False,
        "reversed_name": None,
        "reversed_description": None,
    }

    description_lines = []
    reversed_lines = []
    in_reversed = False

    for line in lines:
        line = line.strip()
        if not line:
            continue

        # Skip image references
        if line.startswith("![Image]"):
            continue

        # Check for reversed spell header
        if line.startswith("## Reversed:"):
            spell["reversible"] = True
            spell["reversed_name"] = line.replace("## Reversed:", "").strip()
            in_reversed = True
            continue

        # Check for combined Duration/Range (OCR artifact)
        combined = COMBINED_RE.match(line)
        if combined:
            spell["duration"] = combined.group(1).strip()
            spell["range"] = combined.group(2).strip()
            continue

        # Check for Duration line
        dur_match = DURATION_RE.match(line)
        if dur_match and spell["duration"] is None:
            spell["duration"] = dur_match.group(1).strip()
            # Check if range is on same line
            range_part = line[dur_match.end():]
            range_match = RANGE_RE.match(range_part.strip())
            if range_match:
                spell["range"] = range_match.group(1).strip()
            continue

        # Check for Range anywhere in line (if not already found)
        if spell["range"] is None:
            range_match = RANGE_RE.search(line)
            if range_match:
                spell["range"] = range_match.group(1).strip()
                # If Range was found mid-line, add the prefix as description
                prefix = line[:range_match.start()].strip()
                if prefix and not prefix.lower().startswith("duration"):
                    description_lines.append(prefix)
                continue

        # Everything else is description
        if in_reversed:
            reversed_lines.append(line)
        else:
            description_lines.append(line)

    # Join description lines
    spell["description"] = " ".join(description_lines) if description_lines else None
    if reversed_lines:
        spell["reversed_description"] = " ".join(reversed_lines)

    # Mark if this looks like a sub-entry (no duration/range, or name ends with ':')
    spell["_is_subentry"] = (
        (spell["duration"] is None and spell["range"] is None) or
        spell_name.endswith(":")
    )

    return spell


def extract_spells(markdown_path: Path) -> list[dict]:
    """Extract all spells from markdown file."""
    text = markdown_path.read_text()
    lines = text.split("\n")

    spells = []
    current_list = None
    current_level = None
    current_spell_name = None
    current_spell_lines = []

    i = 0
    while i < len(lines):
        line = lines[i].strip()

        # Check for end-of-spells section
        for end_section in END_SECTIONS:
            if line == f"## {end_section}":
                # Save any pending spell
                if current_spell_name and current_list:
                    spell = parse_spell_block(
                        current_spell_lines, current_spell_name,
                        current_list, current_level or 1
                    )
                    if spell["description"]:
                        spells.append(spell)
                # Exit spell parsing mode
                current_list = None
                current_level = None
                current_spell_name = None
                current_spell_lines = []
                break

        # Check for spell list section header
        for section_name, list_id in SPELL_SECTIONS.items():
            if line == f"## {section_name}":
                # Save any pending spell
                if current_spell_name and current_list:
                    spell = parse_spell_block(
                        current_spell_lines, current_spell_name,
                        current_list, current_level or 1
                    )
                    if spell["description"] and not spell.get("_is_subentry"):
                        spells.append(spell)

                current_list = list_id
                current_level = None
                current_spell_name = None
                current_spell_lines = []
                break

        # Check for level header
        level_match = LEVEL_RE.match(line)
        if level_match and current_list:
            # Save any pending spell
            if current_spell_name:
                spell = parse_spell_block(
                    current_spell_lines, current_spell_name,
                    current_list, current_level or 1
                )
                if spell["description"] and not spell.get("_is_subentry"):
                    spells.append(spell)

            current_level = int(level_match.group(1))
            current_spell_name = None
            current_spell_lines = []
            i += 1
            continue

        # Check for spell name header (## Name but not ## Reversed: or ## Nth Level)
        if (line.startswith("## ") and
            current_list and
            not line.startswith("## Reversed:") and
            not LEVEL_RE.match(line) and
            line not in [f"## {s}" for s in SPELL_SECTIONS.keys()]):

            # Save any pending spell
            if current_spell_name:
                spell = parse_spell_block(
                    current_spell_lines, current_spell_name,
                    current_list, current_level or 1
                )
                if spell["description"] and not spell.get("_is_subentry"):
                    spells.append(spell)

            # Handle malformed headers like "## Duration: X Range: Y"
            if "Duration:" in line:
                # This is actually part of the previous spell's content, not a new header
                current_spell_lines.append(line)
            else:
                current_spell_name = line[3:].strip()  # Remove "## "
                current_spell_lines = []
            i += 1
            continue

        # Accumulate content for current spell
        if current_spell_name:
            current_spell_lines.append(line)

        i += 1

    # Save final spell
    if current_spell_name and current_list:
        spell = parse_spell_block(
            current_spell_lines, current_spell_name,
            current_list, current_level or 1
        )
        if spell["description"]:
            spells.append(spell)

    return spells


def main():
    # Default paths
    default_input = Path.home() / ".osr_data/extracted/Advanced_Fantasy_Players_Tome_v1-3.md"

    input_path = Path(sys.argv[1]) if len(sys.argv) > 1 else default_input
    output_path = Path(sys.argv[2]) if len(sys.argv) > 2 else None

    if not input_path.exists():
        print(f"Error: Input file not found: {input_path}", file=sys.stderr)
        print(f"Run extract.sh first to generate the markdown file.", file=sys.stderr)
        sys.exit(1)

    spells = extract_spells(input_path)

    # Remove internal fields
    for s in spells:
        s.pop("_is_subentry", None)

    # Group by list for summary
    by_list = {}
    for s in spells:
        by_list.setdefault(s["list"], []).append(s)

    result = {
        "source": input_path.name,
        "count": len(spells),
        "by_list": {k: len(v) for k, v in by_list.items()},
        "spells": spells
    }

    output = json.dumps(result, indent=2)

    if output_path:
        output_path.write_text(output)
        print(f"Wrote {len(spells)} spells to {output_path}", file=sys.stderr)
        for list_name, spells_in_list in by_list.items():
            print(f"  {list_name}: {len(spells_in_list)}", file=sys.stderr)
    else:
        print(output)


if __name__ == "__main__":
    main()
