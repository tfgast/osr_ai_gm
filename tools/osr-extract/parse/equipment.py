#!/usr/bin/env python3
"""
Parse OSE equipment tables from docling-extracted markdown.

Extracts:
- Adventuring gear with descriptions
- Weapons with combat stats (damage, qualities, ranges)
- Ammunition
- Armour
- Poisons (bloodstream and ingested)
- Animals of burden
- Dogs
- Tack and harness
- Land vehicles
- Water vessels (seaworthy and unseaworthy)
- Ship weaponry

Usage:
    python equipment.py [input.md] [output.json]

    Defaults:
      input:  ~/.osr_data/extracted/Advanced_Fantasy_Players_Tome_v1-3.md
      output: stdout (or specify file path)
"""

import json
import re
import sys
from pathlib import Path
from typing import Optional


def parse_cost(cost_str: str) -> dict:
    """Parse cost string like '5', 'Free', '1gp per square foot' into structured data."""
    cost_str = cost_str.strip()
    if cost_str.lower() in ("free", "-", ""):
        return {"gp": 0}
    # Handle "1gp per square foot" style
    if "per" in cost_str.lower():
        return {"gp": 0, "note": cost_str}
    # Handle comma-separated numbers like "1,000"
    cleaned = cost_str.replace(",", "").replace("gp", "").strip()
    try:
        return {"gp": int(cleaned)}
    except ValueError:
        try:
            return {"gp": float(cleaned)}
        except ValueError:
            return {"gp": 0, "note": cost_str}


def parse_weight(weight_str: str) -> Optional[int]:
    """Parse weight in coins."""
    weight_str = weight_str.strip()
    if not weight_str or weight_str == "-":
        return None
    cleaned = weight_str.replace(",", "").strip()
    try:
        return int(cleaned)
    except ValueError:
        return None


def parse_ac(ac_str: str) -> dict:
    """Parse AC string like '7 [12]' or '+1 bonus' into structured data."""
    ac_str = ac_str.strip()

    # Shield case: "+1 bonus"
    if "bonus" in ac_str.lower():
        return {"bonus": 1, "is_shield": True}

    # Standard AC: "7 [12]"
    match = re.match(r"(\d+)\s*\[(\d+)\]", ac_str)
    if match:
        return {
            "descending": int(match.group(1)),
            "ascending": int(match.group(2)),
        }

    # Just a number
    try:
        ac = int(ac_str)
        return {"descending": ac}
    except ValueError:
        return {"note": ac_str}


def parse_range(qualities_str: str) -> Optional[dict]:
    """Extract range bands from qualities string like 'Missile (5'-80' / 81'-160' / 161'-240')'."""
    # Pattern: Missile (5'-80' / 81'-160' / 161'-240')
    match = re.search(r"Missile\s*\((\d+)['\"]?-(\d+)['\"]?\s*/\s*(\d+)['\"]?-(\d+)['\"]?\s*/\s*(\d+)['\"]?-(\d+)['\"]?\)", qualities_str)
    if match:
        return {
            "short": [int(match.group(1)), int(match.group(2))],
            "medium": [int(match.group(3)), int(match.group(4))],
            "long": [int(match.group(5)), int(match.group(6))],
        }
    return None


def parse_qualities(qualities_str: str) -> list[str]:
    """Parse weapon qualities string into list of quality names."""
    # Remove range info for cleaner parsing
    cleaned = re.sub(r"\([^)]+\)", "", qualities_str)
    # Split on comma and clean up
    qualities = [q.strip() for q in cleaned.split(",") if q.strip()]
    # Normalize
    normalized = []
    for q in qualities:
        q_lower = q.lower()
        if q_lower == "melee":
            normalized.append("melee")
        elif q_lower == "missile":
            normalized.append("missile")
        elif q_lower == "blunt":
            normalized.append("blunt")
        elif q_lower in ("two-handed", "two handed"):
            normalized.append("two_handed")
        elif q_lower == "slow":
            normalized.append("slow")
        elif q_lower == "brace":
            normalized.append("brace")
        elif q_lower == "charge":
            normalized.append("charge")
        elif q_lower == "reload":
            normalized.append("reload")
        elif q_lower == "splash weapon":
            normalized.append("splash")
    return normalized


def parse_table_row(line: str) -> Optional[list[str]]:
    """Parse a markdown table row into cells."""
    if not line.strip().startswith("|"):
        return None
    # Split by | and clean up
    cells = [c.strip() for c in line.split("|")]
    # Remove empty first/last cells from |...|
    if cells and cells[0] == "":
        cells = cells[1:]
    if cells and cells[-1] == "":
        cells = cells[:-1]
    return cells if cells else None


def is_table_separator(line: str) -> bool:
    """Check if line is a table separator (|---|---|)."""
    return bool(re.match(r"^\|[-:\s|]+\|$", line.strip()))


def extract_adventuring_gear(lines: list[str], start_idx: int) -> tuple[list[dict], int]:
    """Extract adventuring gear table and descriptions."""
    items = []
    descriptions = {}
    i = start_idx

    # Find the table
    while i < len(lines) and "Adventuring Gear Item" not in lines[i]:
        i += 1

    if i >= len(lines):
        return items, start_idx

    # Skip header and separator
    i += 1
    if i < len(lines) and is_table_separator(lines[i]):
        i += 1

    # Parse table rows
    while i < len(lines):
        line = lines[i].strip()
        if not line.startswith("|") or is_table_separator(line):
            break

        cells = parse_table_row(line)
        if cells and len(cells) >= 2:
            name = cells[0].strip()
            cost = parse_cost(cells[1])
            if name and not name.startswith("-"):
                items.append({
                    "name": name,
                    "cost": cost,
                    "category": "gear",
                })
        i += 1

    # Now look for descriptions section
    while i < len(lines) and not lines[i].strip().startswith("## Descriptions"):
        i += 1

    if i < len(lines):
        i += 1  # Skip header
        current_item = None
        current_desc = []

        while i < len(lines):
            line = lines[i].strip()
            # Stop at next major section
            if line.startswith("## ") and "Descriptions" not in line:
                break

            # Check for item: description pattern
            if ":" in line and not line.startswith("-") and not line.startswith("▶"):
                # Save previous
                if current_item and current_desc:
                    descriptions[current_item.lower()] = " ".join(current_desc)

                parts = line.split(":", 1)
                current_item = parts[0].strip()
                current_desc = [parts[1].strip()] if len(parts) > 1 and parts[1].strip() else []
            elif current_item and line and not line.startswith("!["):
                current_desc.append(line)

            i += 1

        # Save last item
        if current_item and current_desc:
            descriptions[current_item.lower()] = " ".join(current_desc)

    # Attach descriptions to items
    for item in items:
        name_lower = item["name"].lower()
        # Try exact match first, then partial
        if name_lower in descriptions:
            item["description"] = descriptions[name_lower]
        else:
            # Try matching first word
            first_word = name_lower.split()[0] if name_lower else ""
            for key, desc in descriptions.items():
                if key.startswith(first_word):
                    item["description"] = desc
                    break

    return items, i


def extract_weapons(lines: list[str], start_idx: int) -> tuple[list[dict], int]:
    """Extract weapons from both the basic table and combat stats table."""
    weapons = {}
    i = start_idx

    # Find weapons table
    while i < len(lines) and "Weapons Weapon" not in lines[i]:
        i += 1

    if i >= len(lines):
        return list(weapons.values()), start_idx

    # Skip header and separator
    i += 1
    if i < len(lines) and is_table_separator(lines[i]):
        i += 1

    # Parse basic weapons table (name, cost, weight)
    while i < len(lines):
        line = lines[i].strip()
        if not line.startswith("|") or is_table_separator(line):
            break

        cells = parse_table_row(line)
        if cells and len(cells) >= 3:
            name = cells[0].strip()
            cost = parse_cost(cells[1])
            weight = parse_weight(cells[2])
            if name and not name.startswith("-"):
                weapons[name.lower()] = {
                    "name": name,
                    "cost": cost,
                    "weight_coins": weight,
                    "category": "weapon",
                }
        i += 1

    # Find ammunition table
    while i < len(lines) and "Ammunition Ammunition" not in lines[i] and "## Armour" not in lines[i]:
        i += 1

    ammo = []
    if i < len(lines) and "Ammunition" in lines[i]:
        i += 1
        if i < len(lines) and is_table_separator(lines[i]):
            i += 1

        while i < len(lines):
            line = lines[i].strip()
            if not line.startswith("|") or is_table_separator(line):
                break

            cells = parse_table_row(line)
            if cells and len(cells) >= 2:
                name = cells[0].strip()
                cost = parse_cost(cells[1])
                if name and not name.startswith("-"):
                    ammo.append({
                        "name": name,
                        "cost": cost,
                        "category": "ammunition",
                    })
            i += 1

    # Find weapon combat stats table
    while i < len(lines) and "Weapon Combat Stats Weapon" not in lines[i]:
        i += 1

    if i < len(lines):
        i += 1
        if i < len(lines) and is_table_separator(lines[i]):
            i += 1

        while i < len(lines):
            line = lines[i].strip()
            if not line.startswith("|") or is_table_separator(line):
                break

            cells = parse_table_row(line)
            if cells and len(cells) >= 3:
                name = cells[0].strip()
                damage = cells[1].strip()
                qualities_str = cells[2].strip()

                if name:
                    name_lower = name.lower()
                    qualities = parse_qualities(qualities_str)
                    range_info = parse_range(qualities_str)

                    if name_lower in weapons:
                        weapons[name_lower]["damage"] = damage
                        weapons[name_lower]["qualities"] = qualities
                        if range_info:
                            weapons[name_lower]["range"] = range_info
                    else:
                        # Items like "Holy water vial", "Oil flask, burning", "Torch"
                        # that appear in combat stats but not in weapons table
                        item = {
                            "name": name,
                            "damage": damage,
                            "qualities": qualities,
                            "category": "improvised_weapon",
                        }
                        if range_info:
                            item["range"] = range_info
                        weapons[name_lower] = item
            i += 1

    return list(weapons.values()), ammo, i


def extract_armour(lines: list[str], start_idx: int) -> tuple[list[dict], int]:
    """Extract armour table."""
    items = []
    i = start_idx

    # Find armour table (look for the one with AC column)
    while i < len(lines):
        if "Armour Armour" in lines[i] or (lines[i].strip().startswith("|") and "AC" in lines[i] and "Weight" in lines[i]):
            break
        i += 1

    if i >= len(lines):
        return items, start_idx

    # Skip header and separator
    i += 1
    if i < len(lines) and is_table_separator(lines[i]):
        i += 1

    # Parse table rows
    while i < len(lines):
        line = lines[i].strip()
        if not line.startswith("|") or is_table_separator(line):
            break

        cells = parse_table_row(line)
        if cells and len(cells) >= 4:
            name = cells[0].strip()
            ac = parse_ac(cells[1])
            cost = parse_cost(cells[2])
            weight = parse_weight(cells[3])

            if name and not name.startswith("-"):
                item = {
                    "name": name,
                    "ac": ac,
                    "cost": cost,
                    "category": "armour",
                }
                if weight is not None:
                    item["weight_coins"] = weight
                items.append(item)
        i += 1

    return items, i


def extract_poisons(lines: list[str], start_idx: int) -> tuple[list[dict], int]:
    """Extract poison tables (bloodstream and ingested)."""
    poisons = []
    i = start_idx

    # Find bloodstream poisons section header
    while i < len(lines) and "## Bloodstream Poisons" not in lines[i]:
        i += 1

    if i >= len(lines):
        return poisons, start_idx

    i += 1  # Skip header

    # Find the table header row (Type | Cost...)
    while i < len(lines) and not (lines[i].strip().startswith("|") and "Type" in lines[i]):
        i += 1

    if i >= len(lines):
        return poisons, start_idx

    i += 1  # Skip header row
    if i < len(lines) and is_table_separator(lines[i]):
        i += 1  # Skip separator

    # Parse bloodstream poisons
    while i < len(lines):
        line = lines[i].strip()
        if not line.startswith("|") or is_table_separator(line) or "Ingested" in line or not line:
            break

        cells = parse_table_row(line)
        if cells and len(cells) >= 7:
            type_num = cells[0].strip()
            cost = parse_cost(cells[1])
            save_mod = cells[2].strip()
            detection = cells[3].strip()
            onset = cells[4].strip()
            effect_success = cells[5].strip()
            effect_fail = cells[6].strip()

            if type_num and type_num in ("I", "II", "III", "IV", "V"):
                poisons.append({
                    "name": f"Bloodstream Poison Type {type_num}",
                    "type": "bloodstream",
                    "tier": type_num,
                    "cost": cost,
                    "save_modifier": save_mod,
                    "detection_chance": detection,
                    "onset_time": onset,
                    "effect_on_save": effect_success,
                    "effect_on_fail": effect_fail,
                    "category": "poison",
                })
        i += 1

    # Find ingested poisons table - it starts with a header row of repeated "Ingested Poisons"
    while i < len(lines) and "Ingested Poisons" not in lines[i]:
        i += 1

    if i < len(lines):
        i += 1  # Skip "Ingested Poisons" header row
        if i < len(lines) and is_table_separator(lines[i]):
            i += 1  # Skip separator
        # Skip the column names row (Type | Cost...)
        if i < len(lines) and "Type" in lines[i]:
            i += 1

        # Parse ingested poisons
        while i < len(lines):
            line = lines[i].strip()
            if not line.startswith("|") or is_table_separator(line) or not line:
                break

            cells = parse_table_row(line)
            if cells and len(cells) >= 7:
                type_num = cells[0].strip()
                cost = parse_cost(cells[1])
                save_mod = cells[2].strip()
                detection = cells[3].strip()
                onset = cells[4].strip()
                effect_success = cells[5].strip()
                effect_fail = cells[6].strip()

                if type_num and type_num in ("I", "II", "III", "IV", "V"):
                    poisons.append({
                        "name": f"Ingested Poison Type {type_num}",
                        "type": "ingested",
                        "tier": type_num,
                        "cost": cost,
                        "save_modifier": save_mod,
                        "detection_chance": detection,
                        "onset_time": onset,
                        "effect_on_save": effect_success,
                        "effect_on_fail": effect_fail,
                        "category": "poison",
                    })
            i += 1

    return poisons, i


def extract_animals(lines: list[str], start_idx: int) -> tuple[list[dict], int]:
    """Extract animals of burden table."""
    animals = []
    i = start_idx

    # Find animals of burden table
    while i < len(lines) and "Animal" not in lines[i]:
        i += 1

    # Look for the actual data table with Camel, Horse, etc.
    while i < len(lines):
        if "Camel" in lines[i] and "|" in lines[i]:
            break
        i += 1

    if i >= len(lines):
        return animals, start_idx

    # Parse animal rows
    while i < len(lines):
        line = lines[i].strip()
        if not line.startswith("|") or is_table_separator(line):
            break

        cells = parse_table_row(line)
        if cells and len(cells) >= 8:
            name = cells[0].strip()
            cost = parse_cost(cells[1])

            if name and name not in ("Animal", ""):
                animals.append({
                    "name": name,
                    "cost": cost,
                    "unencumbered": {
                        "miles_per_day": cells[2].strip(),
                        "movement_rate": cells[3].strip(),
                        "max_load_coins": cells[4].strip().replace(",", ""),
                    },
                    "encumbered": {
                        "miles_per_day": cells[5].strip(),
                        "movement_rate": cells[6].strip(),
                        "max_load_coins": cells[7].strip().replace(",", ""),
                    },
                    "category": "mount",
                })
        i += 1

    return animals, i


def extract_dogs(lines: list[str], start_idx: int) -> tuple[list[dict], int]:
    """Extract dogs table."""
    dogs = []
    i = start_idx

    # Find dogs table
    while i < len(lines) and "Dogs Dog Type" not in lines[i]:
        i += 1

    if i >= len(lines):
        return dogs, start_idx

    i += 1
    if i < len(lines) and is_table_separator(lines[i]):
        i += 1

    while i < len(lines):
        line = lines[i].strip()
        if not line.startswith("|") or is_table_separator(line):
            break

        cells = parse_table_row(line)
        if cells and len(cells) >= 4:
            name = cells[0].strip()
            cost = parse_cost(cells[1])
            miles = cells[2].strip()
            movement = cells[3].strip()

            if name and name not in ("Dog Type", ""):
                dogs.append({
                    "name": f"{name} dog",
                    "cost": cost,
                    "miles_per_day": miles,
                    "movement_rate": movement,
                    "category": "animal",
                })
        i += 1

    return dogs, i


def extract_tack_harness(lines: list[str], start_idx: int) -> tuple[list[dict], int]:
    """Extract tack and harness table."""
    items = []
    i = start_idx

    # Find tack and harness section
    while i < len(lines) and "Tack and Harness" not in lines[i]:
        i += 1

    if i >= len(lines):
        return items, start_idx

    # Find Item/Cost row
    while i < len(lines) and "Item" not in lines[i]:
        i += 1

    i += 1
    while i < len(lines):
        line = lines[i].strip()
        if not line.startswith("|") or is_table_separator(line):
            break

        cells = parse_table_row(line)
        if cells and len(cells) >= 2:
            name = cells[0].strip()
            cost = parse_cost(cells[1])

            if name and name not in ("Item", ""):
                items.append({
                    "name": name,
                    "cost": cost,
                    "category": "tack",
                })
        i += 1

    return items, i


def extract_land_vehicles(lines: list[str], start_idx: int) -> tuple[list[dict], int]:
    """Extract land vehicles table."""
    vehicles = []
    i = start_idx

    # Find land vehicles table
    while i < len(lines) and "## Land Vehicles" not in lines[i]:
        i += 1

    if i >= len(lines):
        return vehicles, start_idx

    # Find the actual data table
    while i < len(lines) and ("Cart" not in lines[i] or "|" not in lines[i]):
        i += 1

    if i >= len(lines):
        return vehicles, start_idx

    # Parse vehicle rows
    while i < len(lines):
        line = lines[i].strip()
        if not line.startswith("|") or is_table_separator(line):
            break

        cells = parse_table_row(line)
        if cells and len(cells) >= 6:
            name = cells[0].strip()
            cost = parse_cost(cells[1])
            miles = cells[2].strip()
            movement = cells[3].strip()
            min_animals = cells[4].strip()
            min_load = cells[5].strip().replace(",", "")

            if name and name not in ("Vehicle", ""):
                vehicle = {
                    "name": name,
                    "cost": cost,
                    "miles_per_day": miles,
                    "movement_rate": movement,
                    "minimum_animals": min_animals,
                    "min_load_coins": min_load,
                    "category": "land_vehicle",
                }
                # Check for extra columns
                if len(cells) >= 8:
                    vehicle["extra_animals"] = cells[6].strip()
                    vehicle["max_load_coins"] = cells[7].strip().replace(",", "")
                vehicles.append(vehicle)
        i += 1

    return vehicles, i


def extract_water_vessels(lines: list[str], start_idx: int) -> tuple[list[dict], int]:
    """Extract water vessel tables."""
    vessels = []
    i = start_idx

    # Find seaworthy vessels
    while i < len(lines) and "## Seaworthy Vessels" not in lines[i]:
        i += 1

    if i >= len(lines):
        return vessels, start_idx

    # Find the data table
    while i < len(lines) and ("Lifeboat" not in lines[i] or "|" not in lines[i]):
        i += 1

    # Parse seaworthy vessels
    while i < len(lines):
        line = lines[i].strip()
        if not line.startswith("|") or is_table_separator(line) or "## " in line:
            break

        cells = parse_table_row(line)
        if cells and len(cells) >= 7:
            name = cells[0].strip()
            cost = parse_cost(cells[1])
            cargo = cells[2].strip().replace(",", "")
            usage = cells[3].strip()
            length = cells[4].strip()
            beam = cells[5].strip()
            draft = cells[6].strip()

            if name and name not in ("Vessel", ""):
                vessels.append({
                    "name": name,
                    "cost": cost,
                    "cargo_capacity_coins": cargo,
                    "usage": usage,
                    "length": length,
                    "beam": beam,
                    "draft": draft,
                    "seaworthy": True,
                    "category": "water_vessel",
                })
        i += 1

    # Find unseaworthy vessels
    while i < len(lines) and "## Unseaworthy Vessels" not in lines[i]:
        i += 1

    if i < len(lines):
        # Find the data table
        while i < len(lines) and ("Boat" not in lines[i] or "|" not in lines[i]):
            i += 1

        while i < len(lines):
            line = lines[i].strip()
            if not line.startswith("|") or is_table_separator(line) or "## " in line:
                break

            cells = parse_table_row(line)
            if cells and len(cells) >= 7:
                name = cells[0].strip()
                cost = parse_cost(cells[1])
                cargo = cells[2].strip().replace(",", "")
                usage = cells[3].strip()
                length = cells[4].strip()
                beam = cells[5].strip()
                draft = cells[6].strip()

                if name and name not in ("Vessel", ""):
                    vessels.append({
                        "name": name,
                        "cost": cost,
                        "cargo_capacity_coins": cargo,
                        "usage": usage,
                        "length": length,
                        "beam": beam,
                        "draft": draft,
                        "seaworthy": False,
                        "category": "water_vessel",
                    })
            i += 1

    return vessels, i


def extract_ship_weaponry(lines: list[str], start_idx: int) -> tuple[list[dict], int]:
    """Extract ship weaponry table."""
    items = []
    i = start_idx

    # Find ship weaponry table
    while i < len(lines) and "Ship Weaponry Item" not in lines[i]:
        i += 1

    if i >= len(lines):
        return items, start_idx

    i += 1
    if i < len(lines) and is_table_separator(lines[i]):
        i += 1

    while i < len(lines):
        line = lines[i].strip()
        if not line.startswith("|") or is_table_separator(line):
            break

        cells = parse_table_row(line)
        if cells and len(cells) >= 2:
            name = cells[0].strip()
            cost = parse_cost(cells[1])

            if name and name not in ("Item", ""):
                items.append({
                    "name": name,
                    "cost": cost,
                    "category": "ship_weapon",
                })
        i += 1

    return items, i


def extract_equipment(markdown_path: Path) -> dict:
    """Extract all equipment from markdown file."""
    text = markdown_path.read_text()
    lines = text.split("\n")

    # Find equipment section
    start_idx = 0
    for i, line in enumerate(lines):
        if line.strip() == "## Equipment":
            start_idx = i
            break

    gear, idx = extract_adventuring_gear(lines, start_idx)
    weapons, ammo, idx = extract_weapons(lines, idx)
    armour, idx = extract_armour(lines, start_idx)
    poisons, idx = extract_poisons(lines, idx)
    animals, idx = extract_animals(lines, idx)
    dogs, idx = extract_dogs(lines, idx)
    tack, idx = extract_tack_harness(lines, idx)
    land_vehicles, idx = extract_land_vehicles(lines, idx)
    water_vessels, idx = extract_water_vessels(lines, idx)
    ship_weapons, idx = extract_ship_weaponry(lines, idx)

    return {
        "gear": gear,
        "weapons": weapons,
        "ammunition": ammo,
        "armour": armour,
        "poisons": poisons,
        "mounts": animals,
        "dogs": dogs,
        "tack": tack,
        "land_vehicles": land_vehicles,
        "water_vessels": water_vessels,
        "ship_weapons": ship_weapons,
    }


def main():
    # Default paths
    default_input = Path.home() / ".osr_data/extracted/Advanced_Fantasy_Players_Tome_v1-3.md"

    input_path = Path(sys.argv[1]) if len(sys.argv) > 1 else default_input
    output_path = Path(sys.argv[2]) if len(sys.argv) > 2 else None

    if not input_path.exists():
        print(f"Error: Input file not found: {input_path}", file=sys.stderr)
        print(f"Run extract.sh first to generate the markdown file.", file=sys.stderr)
        sys.exit(1)

    equipment = extract_equipment(input_path)

    # Collect all items into a flat list with categories
    all_items = []
    for category, items in equipment.items():
        all_items.extend(items)

    result = {
        "source": input_path.name,
        "counts": {cat: len(items) for cat, items in equipment.items()},
        "total": len(all_items),
        "equipment": equipment,
    }

    output = json.dumps(result, indent=2)

    if output_path:
        output_path.write_text(output)
        print(f"Wrote {len(all_items)} equipment items to {output_path}", file=sys.stderr)
        for cat, items in equipment.items():
            if items:
                print(f"  {cat}: {len(items)}", file=sys.stderr)
    else:
        print(output)


if __name__ == "__main__":
    main()
