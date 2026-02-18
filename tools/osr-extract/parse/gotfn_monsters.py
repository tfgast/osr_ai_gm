#!/usr/bin/env python3
"""
Parse GotFN V2 and V3 monster appendices from docling-extracted markdown
into monster JSON databases.

Usage:
    python gotfn_monsters.py [--v2 path] [--v3 path] [--output dir]

    Defaults:
      --v2:     ~/.osr_data/extracted/GotFN_V2.md
      --v3:     ~/.osr_data/extracted/GotFN_V3.md
      --output: ~/.osr_data/modules/gotfn/

Produces:
    monsters_v2.json - Monster stat blocks from V2 Appendix A
    monsters_v3.json - Monster stat blocks from V3 Appendix A
"""

import argparse
import json
import re
import sys
from pathlib import Path


# ---------------------------------------------------------------------------
# Stat block regex (GotFN variant: ATK not Att, comma-separated saves)
# Extended from gotfn_ch06_morkaals_tomb.py with optional Gear capture.
# ---------------------------------------------------------------------------

STAT_LINE_RE = re.compile(
    r"AC\s+(?P<ac>-?\d+)\s*\[(?P<ac_asc>\d+)\](?:[^,]*)?,\s*"
    r"HD\s+(?P<hd>[^,(]+?)\s*(?:\((?P<hp>[^)]+)\))?,\s*"
    r"ATK\s+(?P<attacks>.+?),\s*THAC0"
    r"\s+(?P<thac0>\d+)\s*\[(?P<thac0_bonus>[+-]?\d+)\](?:[^,]*)?,\s*"
    r"MV\s+(?P<movement>.+?),\s*SV\s+"
    r"(?P<saves>D\d+,\s*W\d+,\s*P\d+,\s*B\d+,\s*S\d+\s*\([^)]+\)|[^,]+),\s*"
    r"ML\s+(?P<morale>\d+)(?:\s*\([^)]*\))?,\s*"
    r"AL\s+(?P<alignment>[^,]+),\s*"
    r"XP\s+(?P<xp>[0-9,]+)"
    r"(?:\s*\([^)]+\))?"  # optional parenthetical after XP
    r"(?:,\s*NA\s+(?P<num_appearing>[^,]+),\s*TT\s+(?P<treasure>[A-Za-z0-9,() .+-]+?))?"
    r"(?:,\s*Gear\s+(?P<gear>.+?))?"
    r"\s*\.?\s*$",
    re.MULTILINE,
)

# Appendix boundary markers
APPENDIX_A_START = re.compile(r"^## APPENDIX A: MONSTERS", re.MULTILINE)
APPENDIX_B_START = re.compile(r"^## APPENDIX B:", re.MULTILINE)


# ---------------------------------------------------------------------------
# Utility functions (from gotfn_ch06_morkaals_tomb.py)
# ---------------------------------------------------------------------------

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
    fly_match = re.search(r"(\d+)'[^/]*fly", mv_str, re.IGNORECASE)
    if fly_match:
        result["fly"] = int(fly_match.group(1))
    swim_match = re.search(r"(\d+)'[^/]*swim", mv_str, re.IGNORECASE)
    if swim_match:
        result["swim"] = int(swim_match.group(1))
    burrow_match = re.search(r"(\d+)'[^/]*burrow", mv_str, re.IGNORECASE)
    if burrow_match:
        result["burrow"] = int(burrow_match.group(1))
    climb_match = re.search(r"(\d+)'[^/]*climb", mv_str, re.IGNORECASE)
    if climb_match:
        result["climb"] = int(climb_match.group(1))
    base_match = re.match(r"(\d+)'", mv_str)
    if base_match:
        result["base"] = int(base_match.group(1))
    return result


# ---------------------------------------------------------------------------
# Core parsing
# ---------------------------------------------------------------------------

def extract_appendix(text: str) -> str:
    """Extract Appendix A monster content from a GotFN volume.

    Uses the LAST occurrence of each marker to skip the table of contents.
    """
    # Find all occurrences, take the last one (skip TOC)
    all_starts = list(APPENDIX_A_START.finditer(text))
    if not all_starts:
        return ""
    start_match = all_starts[-1]
    content_start = text.find("\n", start_match.start()) + 1

    all_ends = list(APPENDIX_B_START.finditer(text, content_start))
    if all_ends:
        return text[content_start:all_ends[0].start()]
    return text[content_start:]


def collect_abilities(text: str, stat_end: int, next_boundary: int) -> list[str]:
    """Collect special abilities from bullet points after a stat block.

    Stops at next ## heading, next stat block, or next_boundary.
    """
    region = text[stat_end:next_boundary]
    abilities = []
    for line in region.split("\n"):
        stripped = line.strip()
        if stripped.startswith("##"):
            break
        if re.search(r"AC\s+-?\d+\s*\[\d+\]", stripped):
            break
        if (stripped.startswith("-  ") or stripped.startswith("- ")) and ":" in stripped[:60]:
            ability = stripped.lstrip("- ").strip()
            # Remove control characters from docling extraction
            ability = re.sub(r'[\x00-\x08\x0b\x0c\x0e-\x1f\x7f-\x9f]', '', ability).strip()
            abilities.append(ability)
        elif stripped and not stripped.startswith("-") and abilities:
            # Continuation line — append to last ability
            abilities[-1] += " " + stripped
    return abilities


def parse_monsters(appendix_text: str) -> list[dict]:
    """Parse all monster stat blocks from appendix text."""
    monsters = []
    seen_names: set[str] = set()
    matches = list(STAT_LINE_RE.finditer(appendix_text))

    for i, match in enumerate(matches):
        # Extract monster name from text before ": AC" on the same line
        line_start = appendix_text.rfind("\n", 0, match.start()) + 1
        preceding = appendix_text[line_start:match.start()].strip()
        name = preceding.rstrip(": ").strip()
        if not name:
            continue

        # Deduplicate by name (keep first occurrence)
        if name in seen_names:
            continue
        seen_names.add(name)

        # Find description — text between ## heading and stat block
        heading_pos = appendix_text.rfind("\n##", 0, line_start)
        if heading_pos >= 0:
            desc_start = appendix_text.find("\n", heading_pos + 1) + 1
            desc_text = appendix_text[desc_start:line_start].strip()
            # Filter out cross-reference-only descriptions and image refs
            desc_lines = []
            for dline in desc_text.split("\n"):
                dline = dline.strip()
                if dline.startswith("!["):
                    continue
                if re.search(r"AC\s+-?\d+\s*\[\d+\]", dline):
                    continue
                if dline.startswith("-  ") or dline.startswith("- "):
                    continue
                if dline:
                    desc_lines.append(dline)
            description = " ".join(desc_lines)
            # Skip if this is just a cross-reference with no stat block text
            if description.startswith("See appendix") or description.startswith("See '"):
                description = ""
        else:
            description = ""

        # Collect special abilities
        next_boundary = matches[i + 1].start() if i + 1 < len(matches) else len(appendix_text)
        abilities = collect_abilities(appendix_text, match.end(), next_boundary)

        # Build monster dict
        hp_raw = match.group("hp")
        monster: dict = {
            "name": name,
            "armor_class": int(match.group("ac")),
            "armor_class_ascending": int(match.group("ac_asc")),
            "hit_dice": match.group("hd").strip(),
            "attacks": parse_attacks(match.group("attacks")),
            "thac0": int(match.group("thac0")),
            "thac0_bonus": int(match.group("thac0_bonus")),
            "movement": parse_movement(match.group("movement")),
            "saves": match.group("saves").strip(),
            "morale": int(match.group("morale")),
            "alignment": match.group("alignment").strip(),
            "xp_value": int(match.group("xp").replace(",", "")),
        }
        if hp_raw:
            monster["hp_typical"] = hp_raw.strip()
        if description:
            monster["description"] = description
        if match.group("num_appearing"):
            monster["num_appearing"] = match.group("num_appearing").strip()
        if match.group("treasure"):
            monster["treasure_type"] = match.group("treasure").strip().rstrip(".")
        if abilities:
            monster["special_abilities"] = abilities

        monsters.append(monster)

    return monsters


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def process_volume(md_path: Path, vol_label: str) -> list[dict]:
    """Process one GotFN volume and return parsed monsters."""
    print(f"\nProcessing {vol_label}: {md_path}")
    text = md_path.read_text(encoding="utf-8")

    appendix = extract_appendix(text)
    if not appendix:
        print(f"  WARNING: Appendix A not found in {md_path.name}", file=sys.stderr)
        return []

    print(f"  Appendix A: {len(appendix)} chars, {appendix.count(chr(10))} lines")
    monsters = parse_monsters(appendix)
    print(f"  Parsed: {len(monsters)} monsters")

    # Summary of first/last few
    for m in monsters[:3]:
        print(f"    {m['name']}: HD {m['hit_dice']}, AC {m['armor_class']}, XP {m['xp_value']}")
    if len(monsters) > 6:
        print(f"    ...")
    for m in monsters[-3:]:
        print(f"    {m['name']}: HD {m['hit_dice']}, AC {m['armor_class']}, XP {m['xp_value']}")

    return monsters


def main():
    osr_data = Path.home() / ".osr_data"

    parser = argparse.ArgumentParser(description="Parse GotFN monster appendices")
    parser.add_argument("--v2", type=Path, default=osr_data / "extracted" / "GotFN_V2.md")
    parser.add_argument("--v3", type=Path, default=osr_data / "extracted" / "GotFN_V3.md")
    parser.add_argument("--output", type=Path, default=osr_data / "modules" / "gotfn")
    args = parser.parse_args()

    args.output.mkdir(parents=True, exist_ok=True)

    for vol_path, vol_label, out_name in [
        (args.v2, "V2", "monsters_v2.json"),
        (args.v3, "V3", "monsters_v3.json"),
    ]:
        if not vol_path.exists():
            print(f"Skipping {vol_label}: {vol_path} not found")
            continue

        monsters = process_volume(vol_path, vol_label)
        if not monsters:
            continue

        out_path = args.output / out_name
        output = {
            "source": vol_path.name,
            "count": len(monsters),
            "monsters": monsters,
        }
        with open(out_path, "w") as f:
            json.dump(output, f, indent=2, ensure_ascii=False)
        print(f"  Wrote: {out_path}")

    print("\nDone.")


if __name__ == "__main__":
    main()
