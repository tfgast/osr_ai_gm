#!/usr/bin/env python3
"""
Parse OSE NPC adventuring party generation tables from docling-extracted markdown.

Usage:
    python npc_parties.py [input.md] [output.json]

    Defaults:
      input:  ~/.osr_data/extracted/Advanced_Fantasy_Referees_Tome_v1-3.md
      output: stdout (or specify file path)
"""

import json
import re
import sys
from pathlib import Path
from typing import Optional


def parse_class_level_table(lines: list[str], start_idx: int) -> tuple[list[dict], int]:
    """Parse the NPC Adventurer Class and Level table.

    Format:
    | d20 | Class | Basic | Expert |
    | 1   | Acrobat | 1d3 | 1d6+4 |
    ...
    """
    entries = []
    i = start_idx

    # Find table start
    while i < len(lines) and "NPC Adventurer Class and Level" not in lines[i]:
        i += 1

    if i >= len(lines):
        return [], i

    # Skip header rows and separator
    i += 1
    while i < len(lines) and (lines[i].strip().startswith("|---") or
                               "Level" in lines[i] or
                               lines[i].strip() == "|"):
        i += 1

    # Skip the "d20 | Class | Basic | Expert" row
    if i < len(lines) and "d20" in lines[i]:
        i += 1

    # Parse data rows
    while i < len(lines):
        line = lines[i].strip()

        if not line.startswith("|"):
            break
        if line.startswith("|---"):
            i += 1
            continue

        cols = [c.strip() for c in line.split("|") if c.strip()]

        if len(cols) >= 4 and cols[0].isdigit():
            roll = int(cols[0])
            class_name = cols[1].rstrip(" *")  # Remove asterisk for demi-humans
            basic_level = cols[2]
            expert_level = cols[3]

            entry = {
                "roll": roll,
                "class": class_name,
                "basic_level_dice": basic_level,
                "expert_level_dice": expert_level,
            }

            # Mark demi-human classes
            if "*" in cols[1]:
                entry["demihuman"] = True
                entry["underworld_alternative"] = {
                    "Dwarf": "Duergar",
                    "Elf": "Drow",
                    "Gnome": "Svirfneblin"
                }.get(class_name)

            entries.append(entry)

        i += 1

    return entries, i


def parse_alignment_table(lines: list[str], start_idx: int) -> tuple[list[dict], int]:
    """Parse the NPC Adventurer Alignment table."""
    entries = []
    i = start_idx

    # Find table
    while i < len(lines) and "NPC Adventurer Alignment" not in lines[i]:
        i += 1

    if i >= len(lines):
        return [], i

    # Skip header and separator
    i += 1
    while i < len(lines) and lines[i].strip().startswith("|---"):
        i += 1

    # Parse rows
    while i < len(lines):
        line = lines[i].strip()

        if not line.startswith("|"):
            break

        cols = [c.strip() for c in line.split("|") if c.strip()]

        if len(cols) >= 2:
            roll_range = cols[0]
            alignment = cols[1]

            # Parse roll range like "1-2" or "5-6"
            if "-" in roll_range:
                parts = roll_range.split("-")
                min_roll = int(parts[0])
                max_roll = int(parts[1])
            else:
                min_roll = max_roll = int(roll_range)

            entries.append({
                "min_roll": min_roll,
                "max_roll": max_roll,
                "alignment": alignment
            })

        i += 1

    return entries, i


def parse_party_types(lines: list[str], start_idx: int) -> tuple[dict, int]:
    """Parse the party composition types (Basic, Expert, High-Level)."""
    party_types = {}
    i = start_idx

    # Find Basic Adventurers section
    while i < len(lines):
        line = lines[i].strip()

        if line == "## Basic Adventurers":
            party_types["basic_adventurers"] = {
                "party_size_dice": "1d4+4",
                "level_tier": "basic",
                "notes": ["Roll alignment per NPC or once for party"]
            }
            i += 1
            continue

        if line == "## Expert Adventurers":
            party_types["expert_adventurers"] = {
                "party_size_dice": "1d6+3",
                "level_tier": "expert",
                "mounted_chance": 75,
                "magic_item_chance_per_level": 5,
                "notes": [
                    "Roll alignment per NPC or once for party",
                    "75% chance mounted in wilderness",
                    "Magic item chance: 5% per level per suitable sub-table"
                ]
            }
            i += 1
            continue

        if line == "## High-Level Cleric":
            party_types["high_level_cleric"] = {
                "leader": {"class": "Cleric", "level_dice": "1d6+6"},
                "companions": [
                    {"class": "Cleric", "count_dice": "1d4", "level_dice": "1d4+1"},
                    {"class": "Fighter", "count_dice": "1d3", "level_dice": "1d6"}
                ],
                "alternatives": ["Bard", "Druid", "Paladin"],
                "notes": ["Mounts and magic items as Expert Adventurers"]
            }
            i += 1
            continue

        if line == "## High-Level Fighter":
            party_types["high_level_fighter"] = {
                "leader": {"class": "Fighter", "level_dice": "1d4+6"},
                "companions": [
                    {"class": "any", "count_dice": "2d4", "level_dice": "1d4+2"}
                ],
                "alternatives": ["Barbarian", "Knight", "Paladin", "Ranger"],
                "notes": [
                    "Retainers may be any class",
                    "Often on way to/from war",
                    "Mounts and magic items as Expert Adventurers"
                ]
            }
            i += 1
            continue

        if line == "## High-Level Magic-User":
            party_types["high_level_magic_user"] = {
                "leader": {"class": "Magic-User", "level_dice": "1d4+6"},
                "companions": [
                    {"class": "Magic-User", "count_dice": "1d4", "level_dice": "1d3", "role": "apprentice"},
                    {"class": "Fighter", "count_dice": "1d4", "level_dice": "1d4+1", "role": "mercenary"}
                ],
                "alternatives": ["Illusionist"],
                "notes": [
                    "Apprentices share leader's alignment",
                    "Mercenaries may differ in alignment",
                    "Often on quest for arcane lore",
                    "Mounts and magic items as Expert Adventurers"
                ]
            }
            i += 1
            continue

        # Stop at Strongholds section
        if line == "## Strongholds":
            break

        i += 1

    return party_types, i


def parse_stronghold_tables(lines: list[str], start_idx: int) -> tuple[dict, int]:
    """Parse stronghold ruler, patrol, and reaction tables."""
    stronghold = {}
    i = start_idx

    # Find Strongholds section
    while i < len(lines) and "## Strongholds" not in lines[i]:
        i += 1

    if i >= len(lines):
        return {}, i

    # Parse ruler types
    stronghold["rulers"] = {
        "arcane": {"level_dice": "1d4+10", "examples": ["Illusionist", "Magic-User"]},
        "divine": {"level_dice": "1d8+6", "examples": ["Cleric", "Paladin"]},
        "martial": {"level_dice": "1d6+8", "examples": ["Fighter", "Knight"]}
    }

    # Parse patrol types
    stronghold["patrols"] = {
        "arcane": {
            "count_dice": "2d6",
            "type": "Heavy Footmen",
            "ac": {"descending": 4, "ascending": 15},
            "equipment": "Chainmail + shield, swords",
            "morale": 8
        },
        "divine": {
            "count_dice": "2d6",
            "type": "Medium Horsemen",
            "ac": {"descending": 5, "ascending": 14},
            "equipment": "Chainmail, lances",
            "morale": 9
        },
        "martial": {
            "count_dice": "2d6",
            "type": "Heavy Horsemen",
            "ac": {"descending": 3, "ascending": 16},
            "equipment": "Plate mail, lances, swords",
            "morale": 9
        }
    }

    # Find and parse reaction table
    while i < len(lines) and "Ruler Reaction By Class" not in lines[i]:
        i += 1

    if i < len(lines):
        stronghold["ruler_reactions"] = []
        i += 1  # Skip header
        while i < len(lines) and lines[i].strip().startswith("|---"):
            i += 1

        # Skip the d6/Arcane/Divine/Martial header row
        if i < len(lines) and "d6" in lines[i]:
            i += 1

        while i < len(lines):
            line = lines[i].strip()
            if not line.startswith("|"):
                break

            cols = [c.strip() for c in line.split("|") if c.strip()]
            if len(cols) >= 4 and cols[0].isdigit():
                stronghold["ruler_reactions"].append({
                    "roll": int(cols[0]),
                    "arcane": cols[1],
                    "divine": cols[2],
                    "martial": cols[3]
                })
            i += 1

        # Add reaction descriptions
        stronghold["reaction_descriptions"] = {
            "Chase": "Patrol chases intruders or demands toll. May attack, drive away, or imprison if toll refused.",
            "Ignore": "Patrol leaves PCs to their business.",
            "Invite": "Patrol brings invitation to stay at stronghold. Motive depends on ruler's personality."
        }

    return stronghold, i


def extract_npc_parties(markdown_path: Path) -> dict:
    """Extract all NPC party generation tables from markdown file."""
    text = markdown_path.read_text()
    lines = text.split("\n")

    result = {
        "source": markdown_path.name,
        "general_notes": {
            "spells": "If spell casters present, choose or roll memorized spells",
            "equipment": "Normal adventuring gear",
            "treasure": "Treasure types U+V, shared among group",
            "marching_order": "Decided by referee"
        }
    }

    # Parse class/level table
    class_entries, i = parse_class_level_table(lines, 0)
    result["class_level_table"] = class_entries

    # Parse alignment table
    alignment_entries, i = parse_alignment_table(lines, i)
    result["alignment_table"] = alignment_entries

    # Parse party types
    party_types, i = parse_party_types(lines, i)
    result["party_types"] = party_types

    # Parse stronghold tables
    stronghold, i = parse_stronghold_tables(lines, i)
    result["strongholds"] = stronghold

    return result


def count_entries(data: dict) -> dict:
    """Count entries in the NPC party data."""
    counts = {
        "class_entries": len(data.get("class_level_table", [])),
        "alignment_entries": len(data.get("alignment_table", [])),
        "party_types": len(data.get("party_types", {})),
        "ruler_reactions": len(data.get("strongholds", {}).get("ruler_reactions", []))
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

    npc_data = extract_npc_parties(input_path)
    counts = count_entries(npc_data)

    result = {
        "source": npc_data["source"],
        "counts": counts,
        **{k: v for k, v in npc_data.items() if k != "source"}
    }

    output = json.dumps(result, indent=2)

    if output_path:
        output_path.write_text(output)
        print(f"Wrote NPC party tables to {output_path}", file=sys.stderr)
        for k, v in counts.items():
            print(f"  {k}: {v}", file=sys.stderr)
    else:
        print(output)


if __name__ == "__main__":
    main()
