#!/usr/bin/env python3
"""
Parse OSE encounter tables from docling-extracted markdown.

Usage:
    python encounters.py [input.md] [output.json]

    Defaults:
      input:  ~/.osr_data/extracted/Advanced_Fantasy_Referees_Tome_v1-3.md
      output: stdout (or specify file path)
"""

import json
import re
import sys
from pathlib import Path
from typing import Optional


# Dungeon encounter entry pattern
# Matches: "1/0" at start of row, then entries like "Acolyte (1d8)"
DUNGEON_ROW_RE = re.compile(r"^\|\s*(\d)/(\d)\s*\|(.+)$")
MONSTER_ENTRY_RE = re.compile(r"([A-Za-z][^(]+?)\s*\(([^)]+)\)")

# Wilderness terrain table
TERRAIN_ROW_RE = re.compile(r"^\|\s*(\d)\s*\|(.+)$")
SUBTABLE_CODE_RE = re.compile(r"([A-Z0-9])-([A-Za-z]+)")

# Sub-table patterns
# Matches: "## Wilderness Encounter Sub-Table 1" or "## Wilderness Sub-Table O: Ocean"
SUBTABLE_HEADING_RE = re.compile(r"## Wilderness (?:Encounter )?Sub-Table ([A-Z0-9]+)")
# Matches table row with sub-table header like "| Wilderness Encounter Sub-Table 2 | ..."
SUBTABLE_TABLE_HEADER_RE = re.compile(r"Wilderness (?:Encounter )?Sub-Table ([A-Z0-9]+)")
SUBTABLE_D20_ROW_RE = re.compile(r"^\|\s*(\d+)\s*\|(.+)$")


def parse_monster_entry(entry: str) -> Optional[dict]:
    """Parse a monster entry like 'Acolyte (1d8)' or 'Ankheg 3 HD(1d6)'."""
    entry = entry.strip()
    if not entry or entry == "-":
        return None

    # Handle entries with HD notation like "Ankheg 3 HD(1d6)"
    # Also handles "Hydra, 1d4+4HD (1)" and similar
    hd_match = re.match(r"(.+?)\s*(\d+(?:d\d+(?:[+-]\d+)?)?)\s*HD\s*\(([^)]+)\)", entry)
    if hd_match:
        return {
            "monster": hd_match.group(1).strip().rstrip(','),
            "hd": hd_match.group(2),
            "number": hd_match.group(3).strip()
        }

    # Standard format: "Monster Name (number)"
    match = MONSTER_ENTRY_RE.match(entry)
    if match:
        return {
            "monster": match.group(1).strip().rstrip(','),
            "number": match.group(2).strip()
        }

    # Fallback - just return the name with no number
    if entry and entry != "-":
        return {"monster": entry, "number": "1"}

    return None


def parse_dungeon_table(lines: list[str], start_idx: int) -> tuple[dict, int]:
    """Parse a dungeon encounter table starting at the given index.

    Returns the table data and the line index after the table.
    """
    # First table is levels 1, 2, 3
    # Second table is levels 4-5, 6-7, 8+

    entries = {}
    i = start_idx

    # Find the header line
    while i < len(lines) and not lines[i].strip().startswith("| Dungeon d4/d10"):
        i += 1

    if i >= len(lines):
        return {}, i

    header_line = lines[i].strip()

    # Determine which level columns we have
    is_first_table = "Level 1" in header_line or "Level 2" in header_line

    # Skip header and separator
    i += 2

    while i < len(lines):
        line = lines[i].strip()

        # Stop at empty line or next section
        if not line or line.startswith("##") or line.startswith("| Dungeon d4/d10"):
            break

        match = DUNGEON_ROW_RE.match(line)
        if match:
            d4_val = int(match.group(1))
            d10_val = int(match.group(2))
            roll_key = f"{d4_val}/{d10_val}"

            # Parse the columns
            cols = [c.strip() for c in match.group(3).split("|")]

            if is_first_table:
                level_keys = ["1", "2", "3"]
            else:
                level_keys = ["4-5", "6-7", "8+"]

            for col_idx, level_key in enumerate(level_keys):
                if col_idx < len(cols):
                    entry = parse_monster_entry(cols[col_idx])
                    if entry:
                        if level_key not in entries:
                            entries[level_key] = {}
                        entries[level_key][roll_key] = entry

        i += 1

    return entries, i


def parse_wilderness_terrain_table(lines: list[str], start_idx: int) -> tuple[dict, int]:
    """Parse the wilderness terrain lookup table (d8 -> sub-table codes).

    Format:
    | Wilderness Encounter By Terrain | ... (repeated header) |
    |------|------|
    | d8   | Terrain1 | Terrain2 | ...
    | 1    | Code1    | Code2    | ...
    | ...
    | d8   | Terrain3 | Terrain4 | ...  (second set of terrains)
    | 1    | Code1    | Code2    | ...
    """
    terrains = {}
    i = start_idx

    # Find the table header
    while i < len(lines) and "Wilderness Encounter By Terrain" not in lines[i]:
        i += 1

    if i >= len(lines):
        return {}, i

    # Skip header and separator
    i += 1
    while i < len(lines) and lines[i].strip().startswith("|---"):
        i += 1

    # Process rows
    terrain_names = []
    while i < len(lines):
        line = lines[i].strip()

        # Stop at next section
        if not line or line.startswith("##"):
            break

        if not line.startswith("|"):
            i += 1
            continue

        cols = [c.strip() for c in line.split("|")]
        # Filter empty columns from split
        cols = [c for c in cols if c]

        if not cols:
            i += 1
            continue

        if cols[0] == "d8":
            # This is a header row with terrain names
            terrain_names = cols[1:]
        elif cols[0].isdigit():
            # Data row with d8 roll and codes
            d8_roll = cols[0]
            for col_idx, code in enumerate(cols[1:]):
                if col_idx < len(terrain_names):
                    terrain = terrain_names[col_idx]
                    if terrain not in terrains:
                        terrains[terrain] = {}
                    terrains[terrain][d8_roll] = code.strip()

        i += 1

    return terrains, i


def parse_subtable_clean(lines: list[str], start_idx: int, subtable_name: str) -> tuple[dict, int]:
    """Parse a clean-formatted sub-table (proper markdown table).

    Two formats exist:
    1. Header has category names directly: "| d20 | Dragon | Flyer | Insect |"
    2. Header has sub-table name repeated, categories in "d20" row:
       "| Sub-Table Name | Sub-Table Name | ... |"
       "| d20 | Category1 | Category2 | ... |"
    """
    entries = {}
    i = start_idx

    # Skip to header row
    while i < len(lines) and not lines[i].strip().startswith("|"):
        i += 1

    if i >= len(lines):
        return {}, i

    # Check first table row format
    header_cols = [c.strip() for c in lines[i].split("|") if c.strip()]
    categories = []

    # Detect format: if first row contains "Sub-Table", categories are in d20 row
    first_row_is_subtable_header = any("Sub-Table" in col for col in header_cols)

    if first_row_is_subtable_header:
        # Skip header row and separator
        i += 1
        while i < len(lines) and lines[i].strip().startswith("|---"):
            i += 1

        # Now we should be at the "d20 | Category1 | Category2" row
        if i < len(lines):
            d20_row_cols = [c.strip() for c in lines[i].split("|") if c.strip()]
            if d20_row_cols and d20_row_cols[0].lower() == "d20":
                categories = d20_row_cols[1:]
            i += 1
    else:
        # Format 1: first row has categories (with d20 in first column)
        # e.g., "| d20 | Dragon | Flyer | Insect |"
        if header_cols and header_cols[0].lower() == "d20":
            categories = header_cols[1:]
        else:
            # No d20 column, all columns are categories
            categories = header_cols

        # Skip header and separator
        i += 1
        while i < len(lines) and lines[i].strip().startswith("|---"):
            i += 1

    # Parse data rows
    while i < len(lines):
        line = lines[i].strip()

        if not line.startswith("|") or line.startswith("|---"):
            break

        cols = [c.strip() for c in line.split("|") if c.strip()]

        # First column should be d20 roll
        if cols and cols[0].isdigit():
            d20_roll = cols[0]
            for col_idx, monster_name in enumerate(cols[1:]):
                if col_idx < len(categories) and monster_name.strip():
                    cat = categories[col_idx]
                    if cat not in entries:
                        entries[cat] = {}
                    entries[cat][d20_roll] = monster_name.strip()

        i += 1

    return entries, i


def parse_subtable_vertical(lines: list[str], start_idx: int) -> tuple[dict, int]:
    """Parse a vertically-split sub-table (malformed OCR output).

    These tables have the format:
    d20
    1
    2
    ...
    Category1
    Entry1
    Entry2
    ...
    Category2
    Entry1
    ...
    """
    entries = {}
    i = start_idx

    # Skip past "d20" header
    while i < len(lines) and lines[i].strip() != "d20":
        i += 1
    i += 1  # Skip "d20"

    # Collect d20 roll numbers (1-20), skipping blank lines
    d20_values = []
    while i < len(lines):
        line = lines[i].strip()
        if line.isdigit():
            d20_values.append(line)
            i += 1
        elif line == "":
            # Skip blank lines between numbers
            i += 1
        else:
            # Hit a non-digit, non-blank line (category name)
            break

    # Now parse categories and their entries
    # Categories and entries also have blank lines between them in OCR output
    current_category = None
    category_entries = []

    while i < len(lines):
        line = lines[i].strip()

        # Stop at next section
        if line.startswith("##") or line.startswith("|"):
            break

        if not line:
            i += 1
            continue

        # Check if this is a category header
        # Category headers are typically single words like "Animal", "Human", etc.
        is_category = (
            line in ("Animal", "Human", "Humanoid", "Monster", "Swimmer") or
            line.startswith("Human,")  # "Human, City" etc.
        )

        if is_category:
            # Save previous category
            if current_category and category_entries:
                entries[current_category] = {}
                # Only take entries up to the number of d20 values
                for idx, entry in enumerate(category_entries[:len(d20_values)]):
                    entries[current_category][d20_values[idx]] = entry

            current_category = line.replace(", ", "_").replace(",", "_")
            category_entries = []
        elif current_category:
            # This is a monster entry for the current category
            category_entries.append(line)

        i += 1

    # Save last category
    if current_category and category_entries:
        entries[current_category] = {}
        for idx, entry in enumerate(category_entries[:len(d20_values)]):
            entries[current_category][d20_values[idx]] = entry

    return entries, i


def extract_encounters(markdown_path: Path) -> dict:
    """Extract all encounter tables from markdown file."""
    text = markdown_path.read_text()
    lines = text.split("\n")

    result = {
        "source": markdown_path.name,
        "dungeon": {},
        "wilderness": {
            "terrain_table": {},
            "sub_tables": {}
        }
    }

    i = 0
    while i < len(lines):
        line = lines[i].strip()

        # Parse dungeon tables
        if line.startswith("| Dungeon d4/d10"):
            entries, i = parse_dungeon_table(lines, i)
            result["dungeon"].update(entries)
            continue

        # Parse wilderness terrain table (starts with "| Wilderness Encounter By Terrain")
        # Only parse if we haven't already found it (avoids matching index entries later)
        if (line.startswith("|") and "Wilderness Encounter By Terrain" in line
                and "Sub-Table" not in line
                and not result["wilderness"]["terrain_table"]):
            terrains, i = parse_wilderness_terrain_table(lines, i)
            result["wilderness"]["terrain_table"] = terrains
            continue

        # Parse wilderness sub-tables - two formats:
        # 1. "## Wilderness Encounter Sub-Table X" heading followed by table
        # 2. Table row starting with "| Wilderness Encounter Sub-Table X | ..."

        subtable_id = None

        # Check for ## heading format
        subtable_match = SUBTABLE_HEADING_RE.match(line)
        if subtable_match:
            subtable_id = subtable_match.group(1)
            i += 1
            # Skip to next content
            while i < len(lines) and not lines[i].strip():
                i += 1

        # Check for table header format (Sub-Table in first row of table)
        elif line.startswith("|") and "Sub-Table" in line:
            table_match = SUBTABLE_TABLE_HEADER_RE.search(line)
            if table_match:
                subtable_id = table_match.group(1)
                # Don't increment i - we're at the table header

        if subtable_id and subtable_id not in result["wilderness"]["sub_tables"]:
            if i < len(lines):
                next_line = lines[i].strip()
                if next_line.startswith("|"):
                    # Clean table format
                    entries, i = parse_subtable_clean(lines, i, subtable_id)
                elif next_line == "d20":
                    # Vertical format (malformed OCR)
                    entries, i = parse_subtable_vertical(lines, i)
                else:
                    i += 1
                    continue

                result["wilderness"]["sub_tables"][subtable_id] = entries
            continue

        i += 1

    return result


def count_entries(data: dict) -> dict:
    """Count entries in the encounter data."""
    counts = {
        "dungeon_levels": len(data["dungeon"]),
        "dungeon_entries": sum(len(v) for v in data["dungeon"].values()),
        "terrain_types": len(data["wilderness"]["terrain_table"]),
        "sub_tables": len(data["wilderness"]["sub_tables"]),
    }
    return counts


def main():
    # Default paths
    default_input = Path.home() / ".osr_data/extracted/Advanced_Fantasy_Referees_Tome_v1-3.md"

    input_path = Path(sys.argv[1]) if len(sys.argv) > 1 else default_input
    output_path = Path(sys.argv[2]) if len(sys.argv) > 2 else None

    if not input_path.exists():
        print(f"Error: Input file not found: {input_path}", file=sys.stderr)
        print(f"Run extract.sh first to generate the markdown file.", file=sys.stderr)
        sys.exit(1)

    encounters = extract_encounters(input_path)
    counts = count_entries(encounters)

    result = {
        "source": encounters["source"],
        "counts": counts,
        "dungeon": encounters["dungeon"],
        "wilderness": encounters["wilderness"]
    }

    output = json.dumps(result, indent=2)

    if output_path:
        output_path.write_text(output)
        print(f"Wrote encounter tables to {output_path}", file=sys.stderr)
        for k, v in counts.items():
            print(f"  {k}: {v}", file=sys.stderr)
    else:
        print(output)


if __name__ == "__main__":
    main()
