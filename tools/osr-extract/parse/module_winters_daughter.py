#!/usr/bin/env python3
"""
Parse Winter's Daughter (OSE) from docling-extracted markdown into module.json format.

Usage:
    python module_winters_daughter.py [input.md] [output_dir]

    Defaults:
      input:  ~/.osr_data/extracted/Winters_Daughter_OSE_v3-1.md
      output: ~/.osr_data/modules/winters_daughter/

Produces:
    module.json   - ModuleDef for the game engine
    monsters.json - Module-specific monster stat blocks
"""

import json
import re
import sys
from pathlib import Path


# --- Stat block regex (module variant: no NA/TT fields) ---

# Module stat blocks often lack NA and TT, and may have HD without parenthesized hp.
# Full format: AC X [Y], HD Z (hp), Att ..., THAC0 N [+M], MV ..., SV ..., ML N, AL X, XP N
# Some have: HD 2, (no hp) or HD 4*, (no hp)
STAT_LINE_RE = re.compile(
    r"AC\s+(?P<ac>-?\d+)\s*\[(?P<ac_asc>\d+)\],\s*"
    r"HD\s+(?P<hd>[^,(]+?)\s*(?:\((?P<hp>[^)]+)\))?,\s*"
    r"Att\s+(?P<attacks>[^,]+(?:,\s*\d+\s*×[^,]+)*),\s*"
    r"THAC0\s+(?P<thac0>\d+)\s*\[(?P<thac0_bonus>[+-]?\d+)\],\s*"
    r"MV\s+(?P<movement>[^,]+),\s*"
    r"SV\s+(?P<saves>[^,]+),\s*"
    r"ML\s+(?P<morale>\d+),\s*"
    r"AL\s+(?P<alignment>[^,]+),\s*"
    r"XP\s+(?P<xp>[0-9,]+)"
    # Optionally capture NA and TT if present
    r"(?:,\s*NA\s+(?P<num_appearing>[^,]+),\s*TT\s+(?P<treasure>[A-Za-z0-9,() ]+))?"
)


def parse_attacks(att_str: str) -> list[dict]:
    """Parse attack string into structured format."""
    attacks = []
    parts = re.split(r",\s*(?=\d+\s*×)", att_str)
    for part in parts:
        part = part.strip()
        m = re.match(r"(\d+)\s*×\s*([^(]+)\s*\(([^)]+)\)", part)
        if m:
            count, name, damage = m.groups()
            attacks.append({
                "count": int(count),
                "name": name.strip(),
                "damage": damage.strip(),
            })
        else:
            attacks.append({"raw": part})
    return attacks


def parse_movement(mv_str: str) -> dict:
    """Parse movement string into structured format."""
    result = {}
    mv_str = mv_str.strip()
    fly_match = re.search(r"(\d+)'[^/]*flying", mv_str, re.IGNORECASE)
    if fly_match:
        result["fly"] = int(fly_match.group(1))
    base_match = re.match(r"(\d+)'", mv_str)
    if base_match:
        result["base"] = int(base_match.group(1))
    return result


def extract_stat_blocks(text: str) -> list[dict]:
    """Extract all monster stat blocks from a block of text."""
    monsters = []
    for m in STAT_LINE_RE.finditer(text):
        hp_raw = m.group("hp")
        monster = {
            "armor_class": int(m.group("ac")),
            "armor_class_ascending": int(m.group("ac_asc")),
            "hit_dice": m.group("hd").strip(),
            "hp_typical": hp_raw.strip() if hp_raw else "",
            "attacks": parse_attacks(m.group("attacks")),
            "thac0": int(m.group("thac0")),
            "thac0_bonus": int(m.group("thac0_bonus")),
            "movement": parse_movement(m.group("movement")),
            "saves": m.group("saves").strip(),
            "morale": int(m.group("morale")),
            "alignment": m.group("alignment").strip(),
            "xp_value": int(m.group("xp").replace(",", "")),
        }
        if m.group("num_appearing"):
            monster["num_appearing"] = m.group("num_appearing").strip()
        if m.group("treasure"):
            monster["treasure_type"] = m.group("treasure").strip()
        monsters.append(monster)
    return monsters


def extract_special_abilities(text: str, after_stat_line: bool = True) -> list[str]:
    """Extract bullet-point special abilities from text following a stat block."""
    abilities = []
    for line in text.split("\n"):
        line = line.strip()
        if line.startswith("- ") and ":" in line:
            abilities.append(line[2:].strip())
    return abilities


# --- Room data ---
# Winter's Daughter has 19 keyed areas. We define them with metadata
# extracted from reading the markdown, since each adventure module has
# unique formatting that doesn't lend itself to fully automated parsing.

ROOM_DEFS = {
    "approaching_mound": {
        "number": 1,
        "name": "Approaching the Burial Mound",
        "exits": [
            {"to": "whything_stones", "door": "open"},
            {"to": "tomb_entrance", "door": "open"},
            {"to": "worm_hole", "door": "open"},
        ],
    },
    "whything_stones": {
        "number": 2,
        "name": "The Whything Stones",
        "exits": [
            {"to": "approaching_mound", "door": "open"},
            {"to": "tomb_entrance", "door": "open"},
        ],
    },
    "tomb_entrance": {
        "number": 3,
        "name": "Tomb Entrance",
        "exits": [
            {"to": "whything_stones", "door": "closed"},
            {"to": "hall_of_guardians", "door": "open"},
        ],
        "notes": "Granite slab (cumulative STR bonus >= 4 to open). Stairs descend 20'.",
    },
    "worm_hole": {
        "number": 4,
        "name": "Worm Hole",
        "exits": [
            {"to": "approaching_mound", "door": "open"},
            {"to": "priests_quarters", "door": "open"},
        ],
        "notes": "2' wide hole, 15' down to area 10. Slime trails.",
    },
    "hall_of_guardians": {
        "number": 5,
        "name": "Hall of Guardians",
        "monsters": [{"name": "Religious Object", "count": 4}],
        "exits": [
            {"to": "tomb_entrance", "door": "open"},
            {"to": "blindfolded_statue", "door": "open"},
            {"to": "freezing_mirror", "door": "open"},
            {"to": "family_crypt", "door": "closed"},
            {"to": "statues_with_weapons", "door": "open"},
        ],
    },
    "blindfolded_statue": {
        "number": 6,
        "name": "Blindfolded Statue",
        "exits": [
            {"to": "hall_of_guardians", "door": "open"},
            {"to": "warded_pool", "door": "open"},
        ],
    },
    "freezing_mirror": {
        "number": 7,
        "name": "Freezing Mirror",
        "trap": "Full-length mirror: save vs paralysis or be frozen still. Bypass by covering mirror. Unfreeze with holy water, cure light wounds, or sunlight.",
        "exits": [
            {"to": "hall_of_guardians", "door": "open"},
            {"to": "warded_pool", "door": "open"},
        ],
        "treasure": [{"item": "Silver-framed mirror (1,000gp)"}],
    },
    "family_crypt": {
        "number": 8,
        "name": "Family Crypt",
        "monsters": [{"name": "Floating Skeleton", "count": 2}],
        "exits": [
            {"to": "hall_of_guardians", "door": "closed"},
            {"to": "chapel_st_sedge", "door": "open"},
        ],
        "treasure": [
            {"item": "Pearl necklace (500gp)"},
            {"item": "Gold medallion (500gp)"},
        ],
        "notes": "Fissure in floor leads to area 15 (Fairy) if one descends 15'+. Slime vapour makes touched creatures weightless (permanent).",
    },
    "chapel_st_sedge": {
        "number": 9,
        "name": "Chapel of St Sedge",
        "exits": [
            {"to": "family_crypt", "door": "open"},
            {"to": "priests_quarters", "door": "locked"},
        ],
    },
    "priests_quarters": {
        "number": 10,
        "name": "Abandoned Priest's Quarters",
        "monsters": [{"name": "Wormtongue", "count": 3}],
        "trap": "Locked metal box under loose flagstone: poison needle trap, save vs poison or 1d6 damage and unconscious 1d6 turns.",
        "exits": [
            {"to": "chapel_st_sedge", "door": "locked"},
            {"to": "worm_hole", "door": "open"},
        ],
        "treasure": [
            {"gp": 50},
            {"item": "Silver crucifix (50gp)"},
            {"item": "Clerical scroll of hold person"},
            {"item": "Prayer book of stamped gold leaf (500gp)"},
            {"item": "Box of 20 holy wafers (each cures 1hp)"},
        ],
    },
    "statues_with_weapons": {
        "number": 11,
        "name": "Statues With Weapons",
        "exits": [
            {"to": "hall_of_guardians", "door": "open"},
            {"to": "hall_of_hounds", "door": "open"},
        ],
        "treasure": [
            {"item": "Longsword +2 (fairy, compels wielder to attack largest foe)"},
        ],
        "notes": "7 statues with real weapons. Mould-patched walls (spores: save vs poison or 1d4 damage + choking 1 turn). Faded battle mural with clue: hound name 'Chedr'.",
    },
    "hall_of_hounds": {
        "number": 12,
        "name": "Hall of Hounds",
        "monsters": [{"name": "Stone Hound", "count": 2}],
        "exits": [
            {"to": "statues_with_weapons", "door": "open"},
            {"to": "knights_tomb", "door": "locked"},
        ],
        "notes": "Double doors inscribed 'Call to the Companions'. Speaking 'Flaegr and Chedr' opens them. Touching without password animates the stone hounds.",
    },
    "knights_tomb": {
        "number": 13,
        "name": "The Knight's Tomb",
        "monsters": [{"name": "Ghost of Sir Chyde", "count": 1}],
        "exits": [
            {"to": "hall_of_hounds", "door": "locked"},
        ],
        "treasure": [
            {"item": "Copper bracelets with amethyst owl eyes (1,000gp each, pair)"},
            {"item": "Ring of Soul-Binding (bronze band, moonstone, woven branches)"},
            {"item": "Portrait of Princess Snowfall-at-Dusk (1,500gp if restored)"},
            {"item": "Silver candlesticks, pair (200gp each if cleaned)"},
        ],
    },
    "warded_pool": {
        "number": 14,
        "name": "Warded Pool",
        "exits": [
            {"to": "blindfolded_statue", "door": "open"},
            {"to": "freezing_mirror", "door": "open"},
            {"to": "frozen_lake", "door": "open"},
        ],
        "notes": "Ghostly candles ward the passage. Passing through dissolves the mortal scene and transports to Fairy (area 15). Returning is possible but the ward blocks natives of Fairy. Dispel as 10th level cleric.",
    },
    "frozen_lake": {
        "number": 15,
        "name": "Tower on a Frozen Lake",
        "exits": [
            {"to": "warded_pool", "door": "open"},
            {"to": "entrance_hall", "door": "closed"},
        ],
        "notes": "Wintry glade in Fairy. Bitter cold, 2' deep snow. White marble tower on frozen lake. Purple crack in the sky. Paths into the forest lead back to area 14 through candle wards.",
    },
    "entrance_hall": {
        "number": 16,
        "name": "Entrance Hall",
        "monsters": [
            {"name": "Dolmenwood Troll", "count": 1},
            {"name": "Dolmenwood Goblin", "count": 1},
        ],
        "exits": [
            {"to": "frozen_lake", "door": "closed"},
            {"to": "fairy_kitchen", "door": "open"},
            {"to": "wedding_feast", "door": "open"},
        ],
        "notes": "Goblin doorman Griddlegrim rides troll Grimmlegridge. Uninvited guests may enter if they eat a shroom from the goblin's pouch.",
    },
    "fairy_kitchen": {
        "number": 17,
        "name": "Fairy Kitchen",
        "monsters": [{"name": "Frost Elf Cook", "count": 2}],
        "exits": [
            {"to": "entrance_hall", "door": "open"},
        ],
        "notes": "Pantry seems to expand the deeper one delves. 1-in-6 chance per person per turn of finding something worth 2d10x10gp.",
    },
    "wedding_feast": {
        "number": 18,
        "name": "Wedding Feast",
        "monsters": [
            {"name": "Frost Elf Knight", "count": 5},
            {"name": "Frost Elf Noble", "count": 7},
            {"name": "Frost Elf Guard", "count": 4},
        ],
        "exits": [
            {"to": "entrance_hall", "door": "open"},
            {"to": "princess_bedchamber", "door": "open"},
        ],
        "notes": "Eating or drinking: mortals save vs spells or forevermore yearn to return to Fairy.",
    },
    "princess_bedchamber": {
        "number": 19,
        "name": "The Princess's Bedchamber",
        "monsters": [{"name": "Princess Snowfall-at-Dusk", "count": 1}],
        "exits": [
            {"to": "wedding_feast", "door": "open"},
        ],
        "treasure": [
            {"gp": 6000},
            {"item": "30 ice-jewels (200gp each)"},
            {"item": "12 necklaces of fairy silver (150gp each)"},
            {"item": "Sapphire platinum brooch (1,000gp)"},
            {"item": "20 fur coats and 20 gowns (100gp each)"},
        ],
    },
}

# --- Monster definitions (stat blocks from the module text) ---

MONSTER_DEFS = {
    "Religious Object": {
        "special_abilities": [
            "Scolding: When animated, speaks in shrill, sanctimonious tone.",
            "When killed: Cloud of mould spores. Anyone in melee: save vs poison or 1d2 damage and choke 1 round.",
            "Animated: Attacks non-Lawful characters who enter the room.",
        ],
    },
    "Floating Skeleton": {
        "special_abilities": [
            "Undead: Unaffected by charms and mind control.",
            "Hits in melee: Weapon becomes weightless for a moment (-1 to attack roll next round).",
        ],
    },
    "Wormtongue": {
        "special_abilities": [
            "Acid: Causes 1 damage per round until washed off (e.g. with water).",
        ],
    },
    "Stone Hound": {
        "special_abilities": [
            "Chained: Can't leave this room.",
            "Constructs: Only harmed by magic. Unaffected by charms and mind control.",
        ],
    },
    "Ghost of Sir Chyde": {
        "special_abilities": [
            "Aging touch: Target ages 2d20 years. Fairies don't age but save vs spells or terror 1d6 rounds.",
            "Incorporeal undead: Only harmed by magic or silver. Unaffected by charms and mind control.",
            "Turning: If turned, disappears for 24 hours.",
            "Tethered: To the ring in the coffer. Cannot exist more than 10' from it.",
        ],
    },
    "Dolmenwood Troll": {
        "special_abilities": [
            "Moss growth: A mortal touched sprouts moss at site of contact.",
            "Regenerates: 3hp per round. Reforms unless killed with fire or acid.",
        ],
    },
    "Dolmenwood Goblin": {
        "special_abilities": [
            "Spells: Charm person, darkness, sleep, phantasmal force.",
        ],
    },
    "Frost Elf Cook": {
        "special_abilities": [],
    },
    "Frost Elf Guard": {
        "special_abilities": [],
    },
    "Frost Elf Knight": {
        "special_abilities": [],
        "notes": "Uses same stat block as Frost Elf Guard.",
    },
    "Frost Elf Noble": {
        "special_abilities": [
            "Spells: Sleep, hold person.",
        ],
    },
    "Princess Snowfall-at-Dusk": {
        "special_abilities": [
            "Spells: Charm person, sleep, hold person, invisibility.",
            "Wish: Has the power to grant another's wish. May only use once ever.",
            "Magical ban: Cannot leave the glade unless she renounces her love for Sir Chyde.",
        ],
    },
}


def parse_module(md_path: Path) -> tuple[dict, list[dict]]:
    """Parse the Winter's Daughter markdown into module.json and monsters.json."""
    text = md_path.read_text(encoding="utf-8")

    # Extract all stat blocks from the entire document
    stat_blocks = extract_stat_blocks(text)

    # Map stat blocks to monster names by finding the heading before each stat block
    named_stats = assign_stat_block_names(text, stat_blocks)

    # Build module.json
    module = build_module(text)

    # Build monsters.json with stat blocks + special abilities
    monsters = build_monsters(named_stats)

    return module, monsters


def assign_stat_block_names(text: str, stat_blocks: list[dict]) -> dict[str, dict]:
    """Try to match each stat block to a monster name from the preceding heading."""
    result = {}
    lines = text.split("\n")

    for match in STAT_LINE_RE.finditer(text):
        stat_start = match.start()
        # Find the text before this stat block
        preceding = text[:stat_start]
        preceding_lines = preceding.split("\n")

        # Look backwards for a heading (## Name) or a bold name
        name = None
        for line in reversed(preceding_lines[-10:]):
            line = line.strip()
            # Check for ## heading
            heading_match = re.match(r"^##\s+(.+)", line)
            if heading_match:
                candidate = heading_match.group(1).strip()
                # Skip numbered room headings
                if re.match(r"^\d+\.", candidate):
                    continue
                # Skip generic headings
                if candidate.lower() in ("atmospherics", "furnishings", "guests",
                                         "referee's note", "the princess's plea",
                                         "the drune", "inside the coffers",
                                         "brass plaques on coffers", "stone coffers",
                                         "random shroom effects",
                                         "effects on the bound persons"):
                    continue
                name = candidate
                break

        if name:
            # Parse this stat block
            hp_raw = match.group("hp")
            monster = {
                "armor_class": int(match.group("ac")),
                "armor_class_ascending": int(match.group("ac_asc")),
                "hit_dice": match.group("hd").strip(),
                "hp_typical": hp_raw.strip() if hp_raw else "",
                "attacks": parse_attacks(match.group("attacks")),
                "thac0": int(match.group("thac0")),
                "thac0_bonus": int(match.group("thac0_bonus")),
                "movement": parse_movement(match.group("movement")),
                "saves": match.group("saves").strip(),
                "morale": int(match.group("morale")),
                "alignment": match.group("alignment").strip(),
                "xp_value": int(match.group("xp").replace(",", "")),
            }

            # Collect special abilities from bullet points after the stat block
            after_text = text[match.end():match.end() + 1000]
            abilities = []
            for line in after_text.split("\n"):
                line = line.strip()
                if line.startswith("- ") and ":" in line[:40]:
                    abilities.append(line[2:].strip())
                elif line.startswith("##"):
                    break
            monster["special_abilities"] = abilities

            result[name] = monster

    return result


# Name normalization: map heading names to our canonical monster names
NAME_MAP = {
    "4 Religious Objects": "Religious Object",
    "Religious Objects": "Religious Object",
    "Floating Skeletons": "Floating Skeleton",
    "2 Floating Skeletons": "Floating Skeleton",
    "Wormtongues": "Wormtongue",
    "3 Wormtongues": "Wormtongue",
    "Stone Hounds": "Stone Hound",
    "2 Stone Hounds": "Stone Hound",
    "The Ghost of Sir Chyde": "Ghost of Sir Chyde",
    "Dolmenwood Troll": "Dolmenwood Troll",
    "Grimmlegridge (Troll)": "Dolmenwood Troll",
    "Dolmenwood Goblin": "Dolmenwood Goblin",
    "Griddlegrim (Goblin)": "Dolmenwood Goblin",
    "2 Frost Elf Cooks": "Frost Elf Cook",
    "Frost Elf Cooks": "Frost Elf Cook",
    "Frost Elf Guards and Knights": "Frost Elf Guard",
    "4 Frost Elf Guards": "Frost Elf Guard",
    "Frost Elf Nobles": "Frost Elf Noble",
    "Princess Snowfall-at-Dusk": "Princess Snowfall-at-Dusk",
}


def build_module(text: str) -> dict:
    """Build the ModuleDef JSON structure."""
    rooms = {}
    for key, room_def in ROOM_DEFS.items():
        room = {
            "name": room_def["name"],
        }

        # Extract description from the markdown for this room
        desc = extract_room_description(text, room_def["number"], room_def["name"])
        room["description"] = desc

        if room_def.get("monsters"):
            room["monsters"] = room_def["monsters"]
        if room_def.get("treasure"):
            room["treasure"] = room_def["treasure"]
        if room_def.get("trap"):
            room["trap"] = room_def["trap"]
        room["exits"] = room_def["exits"]

        rooms[key] = room

    return {
        "name": "Winter's Daughter",
        "level_range": [1, 3],
        "entry_room": "approaching_mound",
        "rooms": rooms,
    }


def is_stat_block_line(line: str) -> bool:
    """Check if a line looks like a stat block."""
    return bool(re.search(r"AC\s+-?\d+\s*\[\d+\]", line))


def extract_room_description(text: str, number: int, name: str) -> str:
    """Extract a room's description from the markdown text."""
    # Try to find the room section by its numbered heading, or by name alone
    patterns = [
        rf"##\s+{number}\.\s+{re.escape(name)}\s*\n",
        rf"##\s+{number}\.\s+[^\n]+\n",
    ]
    # Some rooms don't have numbered headings (e.g., "## Vaulted Chamber" for room 14)
    alt_names = ROOM_ALT_NAMES.get(number, [])
    for alt in alt_names:
        patterns.append(rf"##\s+{re.escape(alt)}\s*\n")

    for pattern in patterns:
        m = re.search(pattern, text)
        if m:
            start = m.end()
            # Collect text until next numbered room heading or major section
            section_end = re.search(
                r"\n##\s+(?:\d+\.\s+|The Burial Mound|The Fairy Prison|Epilogue|Magic Items)",
                text[start:],
            )
            if section_end:
                section = text[start:start + section_end.start()]
            else:
                section = text[start:start + 2000]

            # Extract description lines, filtering out stat blocks and images
            desc_lines = []
            for line in section.split("\n"):
                stripped = line.strip()
                if not stripped:
                    continue
                # Skip headings
                if stripped.startswith("##"):
                    continue
                # Skip images
                if stripped.startswith("!["):
                    continue
                # Skip stat block lines
                if is_stat_block_line(stripped):
                    continue
                # Skip lines that are just numbers or table fragments
                if re.match(r"^\|", stripped) or re.match(r"^\d+\.\s+", stripped):
                    continue
                desc_lines.append(stripped)
                if len(desc_lines) >= 6:
                    break

            return " ".join(desc_lines) if desc_lines else ""

    return ""


# Alternate heading names for rooms without numbered headings
ROOM_ALT_NAMES = {
    14: ["Warded Pool", "Vaulted Chamber", "At the Bottom of the Stairs"],
    16: ["Entrance Hall", "The Doormen"],
}


def build_monsters(named_stats: dict[str, dict]) -> list[dict]:
    """Build the monsters.json list, merging extracted stats with manual metadata."""
    monsters = []

    for heading_name, stats in named_stats.items():
        canonical = NAME_MAP.get(heading_name, heading_name)

        # Merge with our manual special abilities if available
        manual = MONSTER_DEFS.get(canonical, {})
        if manual.get("special_abilities"):
            # Prefer manually curated abilities over auto-extracted
            stats["special_abilities"] = manual["special_abilities"]

        stats["name"] = canonical
        # Deduplicate: keep first occurrence (usually the one with hp values)
        if not any(m["name"] == canonical for m in monsters):
            monsters.append(stats)

    # Add any monsters from MONSTER_DEFS that weren't found in stat extraction
    found_names = {m["name"] for m in monsters}
    for name, manual in MONSTER_DEFS.items():
        if name not in found_names and name == "Frost Elf Knight":
            # Knights use the Guard stat block
            guard = next((m for m in monsters if m["name"] == "Frost Elf Guard"), None)
            if guard:
                knight = dict(guard)
                knight["name"] = "Frost Elf Knight"
                monsters.append(knight)

    return monsters


def main():
    osr_data = Path.home() / ".osr_data"
    default_input = osr_data / "extracted" / "Winters_Daughter_OSE_v3-1.md"
    default_output = osr_data / "modules" / "winters_daughter"

    md_path = Path(sys.argv[1]) if len(sys.argv) > 1 else default_input
    out_dir = Path(sys.argv[2]) if len(sys.argv) > 2 else default_output

    if not md_path.exists():
        print(f"Error: input file not found: {md_path}", file=sys.stderr)
        sys.exit(1)

    out_dir.mkdir(parents=True, exist_ok=True)

    print(f"Parsing: {md_path}")
    module, monsters = parse_module(md_path)

    # Validate room exits
    room_keys = set(module["rooms"].keys())
    for key, room in module["rooms"].items():
        for exit in room.get("exits", []):
            if exit["to"] not in room_keys:
                print(f"WARNING: room '{key}' has exit to unknown room '{exit['to']}'")

    # Write module.json
    module_path = out_dir / "module.json"
    with open(module_path, "w") as f:
        json.dump(module, f, indent=2, ensure_ascii=False)
    print(f"Wrote: {module_path} ({len(module['rooms'])} rooms)")

    # Write monsters.json
    monsters_path = out_dir / "monsters.json"
    with open(monsters_path, "w") as f:
        json.dump(monsters, f, indent=2, ensure_ascii=False)
    print(f"Wrote: {monsters_path} ({len(monsters)} monsters)")

    # Summary
    print(f"\nModule: {module['name']}")
    print(f"Level range: {module['level_range']}")
    print(f"Entry room: {module['entry_room']}")
    print(f"Rooms: {len(module['rooms'])}")
    print(f"Monsters defined: {len(monsters)}")

    # List rooms with their monsters/treasure
    for key in sorted(module["rooms"], key=lambda k: ROOM_DEFS[k]["number"]):
        room = module["rooms"][key]
        parts = [f"  {ROOM_DEFS[key]['number']:2d}. {room['name']}"]
        if room.get("monsters"):
            mon_str = ", ".join(
                f"{m['count']}x {m['name']}" for m in room["monsters"]
            )
            parts.append(f"  M:[{mon_str}]")
        if room.get("treasure"):
            parts.append(f"  T:[{len(room['treasure'])} items]")
        if room.get("trap"):
            parts.append("  TRAP")
        parts.append(f"  exits:{len(room.get('exits', []))}")
        print("".join(parts))


if __name__ == "__main__":
    main()
