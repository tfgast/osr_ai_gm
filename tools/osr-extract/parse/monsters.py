#!/usr/bin/env python3
"""
Parse OSE monster stat blocks from docling-extracted markdown.

Usage:
    python monsters.py [input.md] [output.json]

    Defaults:
      input:  ~/.osr_data/extracted/Advanced_Fantasy_Referees_Tome_v1-3.md
      output: stdout (or specify file path)
"""

import json
import re
import sys
from pathlib import Path
from typing import Optional

# Stat line pattern - matches the consistent OSE format
# Example: AC 4 [15], HD 6+1** (28hp), Att 1 × bite (1d10), THAC0 13 [+6], MV 60' (20'), SV D10 W11 P12 B13 S14 (6), ML 9, AL Neutral, XP 950, NA 1d6 (1d6), TT F
STAT_LINE_RE = re.compile(
    r"AC\s+(?P<ac>-?\d+)\s*\[(?P<ac_asc>\d+)\],\s*"
    r"HD\s+(?P<hd>[^,]+?)\s*\((?P<hp>[^)]+)\),\s*"
    r"Att\s+(?P<attacks>[^,]+(?:,\s*\d+\s*×[^,]+)*),\s*"
    r"THAC0\s+(?P<thac0>\d+)\s*\[(?P<thac0_bonus>[+-]?\d+)\],\s*"
    r"MV\s+(?P<movement>[^,]+),\s*"
    r"SV\s+(?P<saves>[^,]+),\s*"
    r"ML\s+(?P<morale>\d+),\s*"
    r"AL\s+(?P<alignment>[^,]+),\s*"
    r"XP\s+(?P<xp>[0-9,/]+),\s*"
    r"NA\s+(?P<num_appearing>[^,]+),\s*"
    r"TT\s+(?P<treasure>[A-Za-z0-9,() ]+)"
)


def parse_hit_dice(hd_str: str) -> str:
    """Clean up hit dice string."""
    # Remove hp value if present, clean whitespace
    hd = hd_str.strip()
    # Normalize asterisks (special abilities marker)
    return hd


def parse_attacks(att_str: str) -> list[dict]:
    """Parse attack string into structured format."""
    attacks = []
    # Split on comma followed by digit (new attack count)
    # Pattern: "2 × claw (1d4), 1 × bite (1d8)"
    parts = re.split(r",\s*(?=\d+\s*×)", att_str)

    for part in parts:
        part = part.strip()
        # Match: "N × name (damage)"
        m = re.match(r"(\d+)\s*×\s*([^(]+)\s*\(([^)]+)\)", part)
        if m:
            count, name, damage = m.groups()
            attacks.append({
                "count": int(count),
                "name": name.strip(),
                "damage": damage.strip()
            })
        else:
            # Fallback - just store raw
            attacks.append({"raw": part})

    return attacks


def parse_movement(mv_str: str) -> dict:
    """Parse movement string into structured format."""
    result = {}
    mv_str = mv_str.strip()

    # Look for flying: "30' (10') / 180' (60') flying"
    fly_match = re.search(r"(\d+)'[^/]*flying", mv_str, re.IGNORECASE)
    if fly_match:
        result["fly"] = int(fly_match.group(1))

    # Look for burrowing
    burrow_match = re.search(r"(\d+)'[^/]*burrow", mv_str, re.IGNORECASE)
    if burrow_match:
        result["burrow"] = int(burrow_match.group(1))

    # Look for swimming
    swim_match = re.search(r"(\d+)'[^/]*swim", mv_str, re.IGNORECASE)
    if swim_match:
        result["swim"] = int(swim_match.group(1))

    # Base movement - first number
    base_match = re.match(r"(\d+)'", mv_str)
    if base_match:
        result["base"] = int(base_match.group(1))

    return result


def parse_xp(xp_str: str) -> int | list[int]:
    """Parse XP value(s)."""
    xp_str = xp_str.replace(",", "")
    if "/" in xp_str:
        # Multiple values for variable HD monsters
        return [int(x.strip()) for x in xp_str.split("/")]
    return int(xp_str)


def parse_alignment(al_str: str) -> str:
    """Normalize alignment string."""
    al = al_str.strip()
    # Normalize common variations
    if al.lower() in ("neutral or chaotic", "any"):
        return al
    return al.capitalize()


def extract_monsters(markdown_path: Path) -> list[dict]:
    """Extract all monsters from markdown file."""
    text = markdown_path.read_text()
    monsters = []

    # Find all level-2 headers (## Monster Name)
    # But skip non-monster sections
    skip_sections = {
        "monsters and npcs", "choose monsters", "defeated monsters",
        "wandering monsters", "monster descriptions", "monster reactions",
        "monster saving throws", "monster entries"
    }

    # Split by ## headers
    sections = re.split(r"^## ", text, flags=re.MULTILINE)

    current_parent = None  # For sub-entries like "## Giant Bat" under "## Bat"

    for section in sections[1:]:  # Skip text before first ##
        lines = section.strip().split("\n")
        if not lines:
            continue

        name = lines[0].strip()
        name_lower = name.lower()

        # Skip non-monster sections
        if name_lower in skip_sections:
            continue
        if name_lower.startswith("▶"):  # Skip procedure entries
            continue

        # Find stat line
        stat_line = None
        description_lines = []
        special_abilities = []

        in_description = True
        for line in lines[1:]:
            line = line.strip()
            if not line:
                continue

            # Check for stat line
            if line.startswith("AC ") and "HD " in line and "THAC0 " in line:
                stat_line = line
                in_description = False
            elif line.startswith("- "):
                # Special ability bullet point
                special_abilities.append(line[2:].strip())
            elif line.startswith("![Image]"):
                # Skip image references
                continue
            elif in_description and not stat_line:
                description_lines.append(line)

        if not stat_line:
            # This might be a parent entry (like "## Bat" or "## Bear")
            # that just has shared abilities, no stats
            if special_abilities:
                current_parent = {
                    "name": name,
                    "shared_abilities": special_abilities
                }
            continue

        # Parse the stat line
        match = STAT_LINE_RE.match(stat_line)
        if not match:
            print(f"WARNING: Could not parse stat line for {name}: {stat_line[:80]}...",
                  file=sys.stderr)
            continue

        d = match.groupdict()

        monster = {
            "name": name,
            "description": " ".join(description_lines) if description_lines else None,
            "armor_class": int(d["ac"]),
            "armor_class_ascending": int(d["ac_asc"]),
            "hit_dice": parse_hit_dice(d["hd"]),
            "hp_typical": d["hp"],
            "attacks": parse_attacks(d["attacks"]),
            "thac0": int(d["thac0"]),
            "thac0_bonus": int(d["thac0_bonus"]),
            "movement": parse_movement(d["movement"]),
            "saves": d["saves"].strip(),
            "morale": int(d["morale"]),
            "alignment": parse_alignment(d["alignment"]),
            "xp_value": parse_xp(d["xp"]),
            "num_appearing": d["num_appearing"].strip(),
            "treasure_type": d["treasure"].strip(),
        }

        # Add special abilities
        if special_abilities:
            monster["special_abilities"] = special_abilities

        # Inherit from parent if applicable
        if current_parent and current_parent["name"] in name:
            # This is a sub-entry, inherit shared abilities
            inherited = current_parent.get("shared_abilities", [])
            if inherited:
                existing = monster.get("special_abilities", [])
                # Add reference to parent abilities
                monster["special_abilities"] = existing + [
                    f"See {current_parent['name']}: {ab}" for ab in inherited
                    if not any(ab in e for e in existing)
                ]

        monsters.append(monster)

    return monsters


def main():
    # Default paths
    default_input = Path.home() / ".osr_data/extracted/Advanced_Fantasy_Referees_Tome_v1-3.md"

    input_path = Path(sys.argv[1]) if len(sys.argv) > 1 else default_input
    output_path = Path(sys.argv[2]) if len(sys.argv) > 2 else None

    if not input_path.exists():
        print(f"Error: Input file not found: {input_path}", file=sys.stderr)
        print(f"Run extract.sh first to generate the markdown file.", file=sys.stderr)
        sys.exit(1)

    monsters = extract_monsters(input_path)

    result = {
        "source": input_path.name,
        "count": len(monsters),
        "monsters": monsters
    }

    output = json.dumps(result, indent=2)

    if output_path:
        output_path.write_text(output)
        print(f"Wrote {len(monsters)} monsters to {output_path}", file=sys.stderr)
    else:
        print(output)


if __name__ == "__main__":
    main()
