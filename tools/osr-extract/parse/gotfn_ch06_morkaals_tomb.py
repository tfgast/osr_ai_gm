#!/usr/bin/env python3
"""
Parse Morkaal's Tomb (GotFN V1, Chapter 6) from docling-extracted markdown
into module.json format.

Usage:
    python gotfn_ch06_morkaals_tomb.py [input.md] [output_dir]

    Defaults:
      input:  ~/.osr_data/extracted/GotFN_V1.md
      output: ~/.osr_data/modules/gotfn_morkaals_tomb/

Produces:
    module.json   - ModuleDef for the game engine
    monsters.json - Module-specific monster stat blocks
    raw.md        - Chapter 6 text extract for AI GM fallback
"""

import json
import re
import sys
from pathlib import Path


# ---------------------------------------------------------------------------
# Stat block regex (GotFN variant: ATK not Att, comma-separated saves)
# ---------------------------------------------------------------------------

STAT_LINE_RE = re.compile(
    r"AC\s+(?P<ac>-?\d+)\s*\[(?P<ac_asc>\d+)\],\s*"
    r"HD\s+(?P<hd>[^,(]+?)\s*(?:\((?P<hp>[^)]+)\))?,\s*"
    r"ATK\s+(?P<attacks>[^,]+(?:,\s*\d+\s*×[^,]+)*),\s*"
    r"THAC0\s+(?P<thac0>\d+)\s*\[(?P<thac0_bonus>[+-]?\d+)\],\s*"
    r"MV\s+(?P<movement>[^,]+),\s*"
    r"SV\s+(?P<saves>D\d+,\s*W\d+,\s*P\d+,\s*B\d+,\s*S\d+\s*\([^)]+\)),\s*"
    r"ML\s+(?P<morale>\d+),\s*"
    r"AL\s+(?P<alignment>[^,]+),\s*"
    r"XP\s+(?P<xp>[0-9,]+)"
    r"(?:\s*\([^)]+\))?"  # optional parenthetical after XP
    r"(?:,\s*NA\s+(?P<num_appearing>[^,]+),\s*TT\s+(?P<treasure>[A-Za-z0-9,() .+-]+))?"
)


# ---------------------------------------------------------------------------
# Utility functions (shared with WD parser pattern)
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
    # Match fly speed: number before ' eventually followed by fly/flying
    fly_match = re.search(r"(\d+)'[^/]*fly", mv_str, re.IGNORECASE)
    if fly_match:
        result["fly"] = int(fly_match.group(1))
    base_match = re.match(r"(\d+)'", mv_str)
    if base_match:
        result["base"] = int(base_match.group(1))
    return result


def is_stat_block_line(line: str) -> bool:
    """Check if a line looks like a stat block."""
    return bool(re.search(r"AC\s+-?\d+\s*\[\d+\]", line))


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
            monster["treasure_type"] = m.group("treasure").strip().rstrip(".")
        monsters.append(monster)
    return monsters


# ---------------------------------------------------------------------------
# Monster definitions (curated special abilities)
# ---------------------------------------------------------------------------

MONSTER_DEFS = {
    "Ectoplasmic Echo Skeleton": {
        "special_abilities": [
            "Ectoplasmic Organs: When animated, bony husk fills with glowing "
            "reddish lungs and attached vocal system.",
            "Ectoplasmic Screech: 1/day, emits an earth-shattering scream. "
            "Glowing organs burst into reddish sludge soaking creatures within "
            "10'. Save vs. death or suffer curse: evil spirits gain +4 to hit "
            "victim, victim at -4 on saves vs. their special abilities. Fail "
            "by 6+: instant teleportation to Great Defile. Organs reform in "
            "1 day.",
            "Undead: Makes no noise until it attacks. Immune to poison, charm, "
            "hold, sleep.",
        ],
    },
    "Colossus of Morkaal": {
        "special_abilities": [
            "Amulet-Dependent: If the Eye of J'karaa is taken from the "
            "statue's base or a full day passes, the colossus returns to base "
            "and de-animates.",
            "Damage Reduction: Half damage from magical stabbing or cutting "
            "weapons. Only 1 damage from magical projectiles.",
            "Immunity: Immune to charm, cold, fire, hold, lightning, and sleep.",
            "Mundane Damage Immunity: Harmed only by magical attacks.",
            "Swat: 1/round, if a smaller foe runs past within reach, kick: "
            "save vs. death or 2d6 damage and hurled 1d4x5' prone.",
            "Tremor Step: Anyone within 200' can hear the colossus's stomping "
            "feet.",
        ],
    },
    "Wane Wraith": {
        "special_abilities": [
            "Damage Reduction: Half damage from non-magical, non-silver "
            "weapons.",
            "Lesser Energy Drain: Touch victim saves vs. death or -1 to hit "
            "and saves; after 24 hours, second save vs. death or permanently "
            "lose 1 level. If drained of all levels, rises as wane wraith "
            "that night. Bless or remove curse ends the effect.",
            "Mundane Damage Immunity: Harmed only by silver weapons or "
            "magical attacks.",
            "Undead: Makes no noise until it attacks. Immune to poison, charm, "
            "hold, sleep.",
        ],
    },
}

# Name normalization: map stat block heading names to canonical monster names.
# GotFN stat blocks have the name on the same line before ": AC".
NAME_MAP = {
    "Ectoplasmic Echo Skeletons (11)": "Ectoplasmic Echo Skeleton",
    "Ectoplasmic Echo Skeleton": "Ectoplasmic Echo Skeleton",
    "Colossus of Morkaal": "Colossus of Morkaal",
    "Wane Wraiths (2)": "Wane Wraith",
    "Wane Wraiths (5)": "Wane Wraith",
    "Wane Wraith": "Wane Wraith",
    # Post-chapter monsters — exclude from module monsters.json
    "Shuun Slavers (8)": None,
    "Shuun Slavers (5)": None,
    "Shuun Slavers (24)": None,
    "Shuun Slavers": None,
}


# ---------------------------------------------------------------------------
# Room definitions — 19 keyed areas
# ---------------------------------------------------------------------------

ROOM_DEFS = {
    "main_entry_landing": {
        "number": "1",
        "name": "Main Entry Landing",
        "exits": [
            {"to": "western_burial_tunnels", "door": "open",
             "connection_type": "custom",
             "description": "Cliff-face tunnel openings to the west"},
            {"to": "great_northern_hall", "door": "locked",
             "connection_type": "door",
             "description": "18' high sealed stone double doors"},
        ],
        "monsters": [{"name": "Colossus of Morkaal", "count": 1}],
        "treasure": [],
        "trap": "20' deep pit trap in the platform center. Save vs. breath "
                "weapon or fall in. Resets after 1 turn, locking victims inside.",
        "trap_trigger": "entry",
        "features": [
            {"name": "Statue of Morkaal", "kind": "mechanism",
             "description": "15' tall pale white stone statue with amulet-shaped depression in base.",
             "interaction": "The Eye of J'karaa fits in the depression. Turning it triggers the colossus animation and doom gas (see area 4A)."},
            {"name": "Door Inscription", "kind": "inscription",
             "description": "Poem in Inilgaan script hinting at area 14 vault puzzle: 'Seven letters, eight names, nine entwined...'"},
            {"name": "Underground River", "kind": "hazard",
             "description": "East wall collapsed into raging river pouring into ocean 50' below. Swimming north sweeps victim over cliffs: 1d4 d6 damage (save vs. death for half), high drowning risk."},
        ],
        "tags": ["outdoor", "windy"],
        "gm_notes": "Ceiling 20'. Colossus starts as statue — only animates when Eye of J'karaa teleports into base (see area 4A). Once animated, patrols areas 1 and 4.",
    },
    "western_burial_tunnels": {
        "number": "2",
        "name": "Western Burial Tunnels",
        "exits": [
            {"to": "main_entry_landing", "door": "open",
             "connection_type": "custom",
             "description": "Cliff-face passage back to main landing"},
            {"to": "south_junction_archway", "door": "open",
             "connection_type": "custom",
             "description": "Ice-coated tunnel merging south"},
            {"to": "arched_stairwell_junction_b", "door": "open",
             "connection_type": "custom",
             "description": "Ice-coated tunnel merging north/east"},
        ],
        "monsters": [],
        "treasure": [
            {"item": "Ancient Inilgaan coins (11 urns, 2d6gp each)"},
        ],
        "features": [
            {"name": "Slippery Ice", "kind": "hazard",
             "description": "Combined passage coated with slippery ice from ~20' in. Continues into areas 4 and 10."},
            {"name": "Undead Alcoves", "kind": "description",
             "description": "Eleven alcoves hold skeletal corpses on burial slabs glittering with hoarfrost. Dormant undead that animate when glyphs in areas 3A-3C are triggered."},
        ],
        "tags": ["cold", "icy", "dark"],
        "gm_notes": "Ceiling 7-8'. Track which corpses heroes damage — they won't animate. First area to fill with doom gas (wave 1, 3 turns).",
    },
    "south_junction_archway": {
        "number": "3A",
        "name": "South Junction Archway",
        "exits": [
            {"to": "western_burial_tunnels", "door": "open",
             "connection_type": "custom",
             "description": "Tunnel back to burial tunnels"},
            {"to": "great_northern_hall", "door": "open",
             "connection_type": "stairs",
             "description": "Rough-hewn stairs descending northward"},
        ],
        "monsters": [{"name": "Ectoplasmic Echo Skeleton", "count": 4}],
        "treasure": [],
        "trap": "Glyph of Unlife: wiping grime off archway activates hidden "
                "glyph, animating 4 echo skeletons in south tunnel. Death "
                "curse: anyone killed here rises as skeleton that night.",
        "trap_trigger": "action",
        "features": [
            {"name": "Glyph of Unlife", "kind": "mechanism",
             "description": "Hidden glyph carved into archway under webs and dust.",
             "interaction": "Wiping the grime triggers the glyph, animating 4 skeletons."},
        ],
        "tags": ["dark", "trapped", "undead"],
    },
    "arched_stairwell_junction_b": {
        "number": "3B",
        "name": "Arched Stairwell Junction",
        "exits": [
            {"to": "western_burial_tunnels", "door": "open",
             "connection_type": "custom",
             "description": "Tunnel back to burial tunnels"},
            {"to": "arched_stairwell_junction_c", "door": "open",
             "connection_type": "custom",
             "description": "Tunnel continuing north"},
        ],
        "monsters": [{"name": "Ectoplasmic Echo Skeleton", "count": 3}],
        "treasure": [],
        "trap": "Glyph of Unlife: as area 3A but animates 3 echo skeletons "
                "in east tunnel.",
        "trap_trigger": "action",
        "tags": ["dark", "trapped", "undead"],
    },
    "arched_stairwell_junction_c": {
        "number": "3C",
        "name": "Arched Stairwell Junction",
        "exits": [
            {"to": "arched_stairwell_junction_b", "door": "open",
             "connection_type": "custom",
             "description": "Tunnel continuing south"},
            {"to": "great_northern_hall", "door": "open",
             "connection_type": "stairs",
             "description": "Passage descending to great hall"},
        ],
        "monsters": [{"name": "Ectoplasmic Echo Skeleton", "count": 4}],
        "treasure": [],
        "trap": "Glyph of Unlife: as area 3A but animates 4 echo skeletons "
                "in north tunnel.",
        "trap_trigger": "action",
        "tags": ["dark", "trapped", "undead"],
    },
    "great_northern_hall": {
        "number": "4",
        "name": "Great Northern Hall",
        "exits": [
            {"to": "main_entry_landing", "door": "locked",
             "connection_type": "door",
             "description": "Massive stone double doors (20 STR to push open)"},
            {"to": "south_junction_archway", "door": "open",
             "connection_type": "stairs",
             "description": "Powdery stairwell ascending to SW corner"},
            {"to": "arched_stairwell_junction_c", "door": "open",
             "connection_type": "stairs",
             "description": "Passage ascending to NE tunnels"},
            {"to": "collapsed_burial_chamber", "door": "locked",
             "connection_type": "door",
             "description": "Locked stone door on east wall"},
            {"to": "sealed_eastern_door", "door": "locked",
             "connection_type": "door",
             "description": "Wizard-locked stone door on east wall"},
            {"to": "western_corridor", "door": "open",
             "connection_type": "custom",
             "description": "Corridor branching west"},
            {"to": "dead_end_river_tunnel", "door": "open",
             "connection_type": "custom",
             "description": "Corridor branching east at north end"},
            {"to": "great_bronze_doors", "door": "open",
             "connection_type": "door",
             "description": "Imposing bronze double doors at north end"},
        ],
        "monsters": [],
        "treasure": [],
        "trap": "20' deep pit trap on the hallway's left side. Save vs. "
                "breath weapon or fall in. Resets after 1 turn.",
        "trap_trigger": "entry",
        "features": [
            {"name": "West Mural", "kind": "description",
             "description": "Petroglyph: giant white figure with black mask in eight black rings, reaching for floating green cube. 8,000 years old."},
            {"name": "Weak Ceiling", "kind": "hazard",
             "description": "Unstable ceiling. Area-affecting spells/explosions have 4:6 chance of cave-in: 1d8 damage (save vs. breath for half).",
             "interaction": "Sky-shaker bombs can collapse the ceiling on the Colossus."},
        ],
        "tags": ["dark", "trapped", "unstable"],
        "gm_notes": "Ceiling 20'. Colossus patrols this area and area 1 when animated. Safe from doom gas (one of three safe areas with 1 and 16).",
    },
    "great_bronze_doors": {
        "number": "4A",
        "name": "Great Bronze Doors",
        "exits": [
            {"to": "great_northern_hall", "door": "open",
             "connection_type": "door",
             "description": "Back through to the great hall"},
            {"to": "riven_hall", "door": "locked",
             "connection_type": "door",
             "description": "18' bronze doors, wizard locked (11th level) with anti-magic shell"},
        ],
        "monsters": [],
        "treasure": [],
        "features": [
            {"name": "Rotting Hand", "kind": "mechanism",
             "description": "Rotting arm protrudes from fleshy mass on left door. Beckons for the Eye of J'karaa.",
             "interaction": "Give the Eye to the hand to open bronze doors to area 13. Arm has 18 STR, regenerates in 1 turn. After doors open, grabbing amulet requires surprise check and AC 3 [16] attack."},
            {"name": "Archway Inscription", "kind": "inscription",
             "description": "Inilgaan script: 'A jewel in hand is a pair of entombed tomes.'"},
        ],
        "tags": ["dark", "magical"],
        "gm_notes": "CRITICAL: If the Eye vanishes with the head, it teleports to area 1 statue base. This triggers: (1) Colossus animates, (2) Doom gas trap. Bronze doors close forever in 1 turn. Gas fills outer rooms in 3 turns, inner in 6. Only areas 1, 4, 16 remain safe.",
    },
    "collapsed_burial_chamber": {
        "number": "5",
        "name": "Collapsed Burial Chamber",
        "exits": [
            {"to": "great_northern_hall", "door": "locked",
             "connection_type": "door",
             "description": "Locked stone door back to great hall"},
            {"to": "river_ledge_forgotten_crypt", "door": "open",
             "connection_type": "custom",
             "description": "River crossing via fallen rocks (15' deep, strong current)"},
        ],
        "monsters": [],
        "treasure": [
            {"item": "Gleaming orange octahedral gemstone (permanent torchlight)", "value_gp": 400},
            {"gp": 29},
            {"item": "Jade ring set with large peridot", "value_gp": 220},
        ],
        "features": [
            {"name": "Sarcophagus of Kalaborm", "kind": "mechanism",
             "description": "Cracked lid bearing name 'Kalaborm'. Letter B detects as magical. Contains skeleton with orange gemstone (400gp) and 29gp.",
             "interaction": "Remove rubble to open lid (1d2 turns)."},
            {"name": "Sarcophagus of Ralomaka", "kind": "mechanism",
             "description": "Partially submerged in river. Name 'Ralomaka'. Letter A detects as magical. Contains skeleton with jade ring (220gp)."},
        ],
        "tags": ["dark", "collapsed", "wet"],
    },
    "sealed_eastern_door": {
        "number": "6",
        "name": "Sealed Eastern Door",
        "exits": [
            {"to": "great_northern_hall", "door": "locked",
             "connection_type": "door",
             "description": "Wizard-locked stone door back to great hall"},
            {"to": "river_ledge_forgotten_crypt", "door": "open",
             "connection_type": "custom",
             "description": "Corridor beyond sealed door to river ledge"},
        ],
        "monsters": [],
        "treasure": [],
        "tags": ["sealed", "magical"],
        "gm_notes": "Sealed by wizard lock (6th level). Rushing water audible behind it. Corridor beyond leads to area 8.",
    },
    "western_corridor": {
        "number": "7",
        "name": "Western Corridor",
        "exits": [
            {"to": "great_northern_hall", "door": "open",
             "connection_type": "custom",
             "description": "Corridor running east back to great hall"},
            {"to": "northern_corridor", "door": "open",
             "connection_type": "custom",
             "description": "Passage branching northward"},
        ],
        "monsters": [],
        "treasure": [],
        "trap": "Two hidden traps: (1) 20' pit trap, save vs. breath or fall, "
                "resets after 1 turn. (2) Pressure plate west of pit fires 3 "
                "poison darts eastward 60'. Each dart: save vs. breath or 1d4 "
                "damage + 8 poison damage (save vs. poison for half).",
        "trap_trigger": "entry",
        "tags": ["dark", "trapped"],
    },
    "river_ledge_forgotten_crypt": {
        "number": "8",
        "name": "River Ledge & Forgotten Crypt",
        "exits": [
            {"to": "collapsed_burial_chamber", "door": "open",
             "connection_type": "custom",
             "description": "River crossing back via fallen rocks"},
            {"to": "sealed_eastern_door", "door": "open",
             "connection_type": "custom",
             "description": "Corridor back to sealed door"},
        ],
        "monsters": [],
        "treasure": [
            {"item": "+1 staff"},
            {"item": "Obsidian coin box", "value_gp": 100},
            {"gp": 413},
            {"item": "+1 long bow"},
            {"item": "Emerald-encrusted jade necklace", "value_gp": 800},
        ],
        "features": [
            {"name": "Sarcophagus of Corlamak", "kind": "description",
             "description": "Partially submerged. Letter C magical. Contains +1 staff, obsidian coin box (100gp), 413gp."},
            {"name": "Sarcophagus of Rak'kalom", "kind": "description",
             "description": "On burial platform. Letter K magical. Contains +1 long bow."},
            {"name": "Sarcophagus of Marmokal", "kind": "description",
             "description": "On burial platform. Letter M magical. Contains jade necklace (800gp)."},
            {"name": "Sarcophagus of Nol'karam", "kind": "description",
             "description": "On burial platform. Letter N magical. Contains only bones."},
        ],
        "tags": ["dark", "wet", "collapsed"],
    },
    "northern_corridor": {
        "number": "9",
        "name": "Northern Corridor",
        "exits": [
            {"to": "western_corridor", "door": "open",
             "connection_type": "custom",
             "description": "Corridor running south"},
            {"to": "riven_hall", "door": "secret",
             "connection_type": "door",
             "description": "Secret door in eastern alcove (opened by buttons in areas 10 and 12)"},
            {"to": "hanging_north_alcove", "door": "open",
             "connection_type": "custom",
             "description": "14' leap across river to hanging alcove"},
        ],
        "monsters": [],
        "treasure": [
            {"item": "Kikituk (magic item)"},
        ],
        "trap": "Boulder trap: pressure plate opens ceiling trapdoor. Huge "
                "boulder fills corridor, rolls north into river. Anyone struck "
                "dies instantly (no save). Single use, never resets.",
        "trap_trigger": "action",
        "features": [
            {"name": "Secret Door", "kind": "mechanism",
             "description": "Outline visible if discovered, but cannot be opened manually. Both mural buttons (areas 10 and 12) must be pressed within 1 turn.",
             "interaction": "Press both buttons in areas 10 and 12 to open."},
        ],
        "tags": ["dark", "trapped", "wet"],
    },
    "dead_end_river_tunnel": {
        "number": "10",
        "name": "Dead-End River Tunnel",
        "exits": [
            {"to": "great_northern_hall", "door": "open",
             "connection_type": "custom",
             "description": "Corridor running west back to great hall"},
            {"to": "hanging_south_alcove", "door": "open",
             "connection_type": "custom",
             "description": "8' leap across river to dead-end ledge"},
        ],
        "monsters": [],
        "treasure": [],
        "trap": "Blade-saw trap: pressing the hidden button in mural extends "
                "blade-saw from west wall seam. Save vs. death or decapitated.",
        "trap_trigger": "action",
        "features": [
            {"name": "Hidden Button (Mural)", "kind": "mechanism",
             "description": "Hidden button in the shining cube of the south mural. Stays active 1 turn.",
             "interaction": "Push to help open secret door in area 9. WARNING: also triggers blade-saw trap."},
        ],
        "tags": ["dark", "trapped"],
    },
    "hanging_north_alcove": {
        "number": "11",
        "name": "Hanging North Alcove",
        "exits": [
            {"to": "northern_corridor", "door": "open",
             "connection_type": "custom",
             "description": "14' leap back across river to area 9"},
            {"to": "hanging_south_alcove", "door": "open",
             "connection_type": "custom",
             "description": "9' gap to hanging south alcove"},
        ],
        "monsters": [],
        "treasure": [],
        "tags": ["dark", "precarious"],
    },
    "hanging_south_alcove": {
        "number": "12",
        "name": "Hanging South Alcove",
        "exits": [
            {"to": "hanging_north_alcove", "door": "open",
             "connection_type": "custom",
             "description": "9' gap to hanging north alcove"},
            {"to": "dead_end_river_tunnel", "door": "open",
             "connection_type": "custom",
             "description": "8' leap back across river to area 10"},
        ],
        "monsters": [],
        "treasure": [],
        "trap": "Blade-saw trap: pressing hidden button extends blade-saw "
                "from east wall seam. Save vs. death or decapitated.",
        "trap_trigger": "action",
        "features": [
            {"name": "Hidden Button (Mural)", "kind": "mechanism",
             "description": "Hidden button in unruined building of cityscape mural. Stays active 1 turn.",
             "interaction": "Push to help open secret door in area 9. WARNING: also triggers blade-saw trap."},
        ],
        "tags": ["dark", "trapped", "precarious"],
    },
    "riven_hall": {
        "number": "13",
        "name": "The Riven Hall",
        "exits": [
            {"to": "great_bronze_doors", "door": "locked",
             "connection_type": "door",
             "description": "Bronze doors south, wizard locked (11th level)"},
            {"to": "the_far_side", "door": "open",
             "connection_type": "custom",
             "description": "12' leap across river to north platform"},
            {"to": "northern_corridor", "door": "secret",
             "connection_type": "door",
             "description": "Secret door in western alcove"},
        ],
        "monsters": [{"name": "Wane Wraith", "count": 2}],
        "treasure": [],
        "tags": ["dark", "undead", "wet"],
    },
    "the_far_side": {
        "number": "14",
        "name": "The Far Side",
        "exits": [
            {"to": "riven_hall", "door": "open",
             "connection_type": "custom",
             "description": "12' leap back across river to south platform"},
            {"to": "penultimate_crypt", "door": "open",
             "connection_type": "stairs",
             "description": "Stairwells ascending into darkness"},
            {"to": "morkaals_tomb", "door": "open",
             "connection_type": "teleporter",
             "description": "Bronze vault door — solve BLACK MOON puzzle to open portal"},
        ],
        "monsters": [],
        "treasure": [],
        "features": [
            {"name": "Archway Inscription", "kind": "inscription",
             "description": "Inilgaan script: 'MORKAAL First to Stride the Black Moon Path Last to Elude the Black God's Wrath'"},
            {"name": "Morkaal's Vault Door", "kind": "mechanism",
             "description": "Bronze doors with nine turnable wheels (5 top, 4 bottom). Each wheel has 8 faces: A, B, C, K, L, M, N, O.",
             "interaction": "Spell BLACK MOON on the wheels. Each servant's name is Morkaal's letters + one extra (B-L-A-C-K-M-O-N). Larokoma's two skulls with silver O hint at the double-O."},
        ],
        "tags": ["dark", "puzzle"],
        "gm_notes": "Vault door puzzle solution: BLACK MOON. The nine letters come from the eight servants' extra letters plus the double-O from Larokoma's two skulls.",
    },
    "penultimate_crypt": {
        "number": "15",
        "name": "Penultimate Crypt",
        "exits": [
            {"to": "the_far_side", "door": "open",
             "connection_type": "stairs",
             "description": "Stairs descending back to the far side"},
        ],
        "monsters": [{"name": "Wane Wraith", "count": 5}],
        "treasure": [
            {"item": "Obsidian coin box", "value_gp": 100},
            {"gp": 203},
            {"item": "Jade figurine of Morkaal", "value_gp": 250},
            {"item": "4 sky-shaker bombs (magic item)"},
        ],
        "features": [
            {"name": "Sarcophagus of Al'morlak", "kind": "description",
             "description": "Letter L magical. Contains obsidian coin box (100gp) and 203gp."},
            {"name": "Sarcophagus of Larokoma", "kind": "description",
             "description": "Letter O magical. Two skeletons side-by-side, each skull has silver eye ringed with silver (also magical). Two O's hint at BLACK MOON puzzle."},
        ],
        "tags": ["dark", "undead", "wet"],
    },
    "morkaals_tomb": {
        "number": "16",
        "name": "Morkaal's Tomb",
        "exits": [
            {"to": "the_far_side", "door": "open",
             "connection_type": "teleporter",
             "description": "Magic archway teleports back to portal in area 14"},
        ],
        "monsters": [],
        "treasure": [
            {"item": "Black book of Morkaal (magic item)"},
            {"item": "Mask of the black moon (magic item)"},
            {"item": "Ornate jade jewelry box", "value_gp": 600},
            {"item": "511 shiny peridots (7gp each)", "value_gp": 3577},
        ],
        "trap": "Curse trap: swiping dust from sarcophagus triggers magical "
                "curse. All creatures save vs. spells. Those who fail rise as "
                "ectoplasmic echo skeletons 1d4 rounds after death.",
        "trap_trigger": "action",
        "features": [
            {"name": "Sarcophagus of Morkaal", "kind": "mechanism",
             "description": "Huge ornate sarcophagus on dais. Lid forced to floor, cracked in two. Dusty mold of a 15' tall humanoid inside.",
             "interaction": "Swiping dust triggers curse. Secret compartment in base holds black book, mask, and jade jewelry box with 511 peridots."},
        ],
        "tags": ["dark", "cursed", "magical"],
        "gm_notes": "Not on level map. Only reachable via teleporter in area 14. Safe from doom gas. The mold outline is of a 15' tall humanoid (proto-Trow).",
    },
}


# ---------------------------------------------------------------------------
# Section definitions (non-room narrative content)
# ---------------------------------------------------------------------------

SECTION_DEFS = [
    {"key": "traveling_to_tomb",
     "start": "## TRAVELING TO THE TOMB",
     "stop": "## ENTERING THE TOMB"},
    {"key": "entering_the_tomb",
     "start": "## ENTERING THE TOMB",
     "stop": "## BUILDER OF THE TOMB"},
    {"key": "builder_of_tomb",
     "start": "## BUILDER OF THE TOMB",
     "stop": "## MORKAAL THE FIRST"},
    {"key": "morkaal_the_first",
     "start": "## MORKAAL THE FIRST",
     "stop": "## LEGACY OF J"},
    {"key": "legacy_of_jkaraa",
     "start": "## LEGACY OF J",
     "stop": "## CLIFFS OF MORKAAL"},
    {"key": "cliffs_of_morkaal",
     "start": "## CLIFFS OF MORKAAL",
     "stop": "## 1\ufffd"},
    {"key": "conclusion",
     "start": "## CONCLUSION",
     "stop": "## GROWING TROUBLES"},
    {"key": "growing_troubles",
     "start": "## GROWING TROUBLES",
     "stop": "## BLACK MARKET DEALINGS"},
    {"key": "black_market_dealings",
     "start": "## BLACK MARKET DEALINGS",
     "stop": "## REVENGE OF THE SHAHN"},
    {"key": "adventure_rewards",
     "start": "## ADVENTURE REWARDS",
     "stop": "## MONSTERS:"},
]


# ---------------------------------------------------------------------------
# Module rules (custom mechanics)
# ---------------------------------------------------------------------------

MODULE_RULES = {
    "slippery_ice": {
        "name": "Slippery Ice",
        "description": "Ice-coated tunnels in areas 2, 3A-3C, 4, and 10. "
                       "Undead echo skeletons ignore ice (toe claws grip surface).",
    },
    "doom_gas": {
        "name": "Final Doom Gas Trap",
        "description": "Triggered when Eye of J'karaa activates the colossus. "
                       "1 damage per round (no save). Wave 1 (3 turns): areas "
                       "2-3, 8, 10-12, 14-15. Wave 2 (3 more turns): areas "
                       "5-7, 9, 13. Only areas 1, 4, 16 remain safe. Lasts "
                       "3 hours.",
    },
    "river_crossing": {
        "name": "Underground River Crossings",
        "description": "Several rooms require crossing a raging underground "
                       "river by jumping to precarious ledges. River is 15' "
                       "deep with strong current. Swimming risks drowning.",
    },
    "colossus_patrol": {
        "name": "Colossus Patrol",
        "description": "When animated, the Colossus of Morkaal patrols areas "
                       "1 and 4. Cannot fit through other passages. Defeated "
                       "by: sneaking past, removing Eye from statue base, "
                       "destroying it, or waiting 24 hours.",
    },
    "inilgaan_script": {
        "name": "Inilgaan Script",
        "description": "Ancient language found throughout the tomb. "
                       "Decipherable by characters with appropriate language "
                       "skills or magic.",
    },
}


# ---------------------------------------------------------------------------
# Core parsing functions
# ---------------------------------------------------------------------------

def extract_sections(text: str) -> dict[str, str]:
    """Extract non-room sections from the markdown text."""
    sections = {}
    for sdef in SECTION_DEFS:
        start_idx = text.find(sdef["start"])
        if start_idx == -1:
            continue
        content_start = text.find("\n", start_idx) + 1

        stop_idx = text.find(sdef["stop"], content_start)
        if stop_idx == -1:
            content = text[content_start:]
        else:
            content = text[content_start:stop_idx]

        lines = []
        for line in content.split("\n"):
            stripped = line.strip()
            if stripped.startswith("!["):
                continue
            lines.append(line)

        cleaned = "\n".join(lines).strip()
        if cleaned:
            sections[sdef["key"]] = cleaned

    return sections


def extract_room_description(text: str, number: str, name: str) -> str:
    """Extract a room's description from the GotFN Chapter 6 markdown."""
    # GotFN room headings use: ## N\ufffd Name
    pattern = rf"##\s+{re.escape(number)}\ufffd\s+[^\n]+\n"
    m = re.search(pattern, text)
    if not m:
        return ""

    start = m.end()

    # Find next room heading (with \ufffd separator) or major section heading
    section_end = re.search(
        r"\n##\s+(?:\d+[A-Z]?\ufffd"
        r"|CONCLUSION|GROWING|BLACK MARKET|REVENGE|FUTURE|YALTO"
        r"|ADVENTURE|MONSTERS:|TREASURE:|MAGIC ITEMS|CHAPTER)",
        text[start:],
    )
    if section_end:
        section = text[start:start + section_end.start()]
    else:
        section = text[start:start + 2000]

    desc_lines = []
    for line in section.split("\n"):
        stripped = line.strip()
        if not stripped:
            continue
        if stripped.startswith("##"):
            continue
        if stripped.startswith("!["):
            continue
        if is_stat_block_line(stripped):
            continue
        if stripped.startswith("-  ") or stripped.startswith("- "):
            continue
        if re.match(r"^\|", stripped):
            continue
        desc_lines.append(stripped)
        if len(desc_lines) >= 8:
            break

    return " ".join(desc_lines) if desc_lines else ""


def assign_stat_block_names(text: str) -> dict[str, dict]:
    """Match each stat block to a monster name from the same line.

    GotFN format: 'Monster Name : AC X [Y], ...'
    The name appears before ' : AC' on the same line.
    """
    result = {}

    for match in STAT_LINE_RE.finditer(text):
        # Find the start of the line containing this stat block
        line_start = text.rfind("\n", 0, match.start()) + 1
        preceding = text[line_start:match.start()].strip()
        # Strip trailing colon/spaces
        name = preceding.rstrip(": ").strip()
        if not name:
            continue

        canonical = NAME_MAP.get(name, name)
        if canonical is None:
            continue

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

        if match.group("num_appearing"):
            monster["num_appearing"] = match.group("num_appearing").strip()
        if match.group("treasure"):
            monster["treasure_type"] = match.group("treasure").strip().rstrip(".")

        # Collect special abilities from bullet points after the stat block
        after_text = text[match.end():match.end() + 1500]
        abilities = []
        for line in after_text.split("\n"):
            line = line.strip()
            if (line.startswith("-  ") or line.startswith("- ")) and ":" in line[:50]:
                abilities.append(line.lstrip("- ").strip())
            elif line.startswith("##") or (line and not line.startswith("-") and abilities):
                break
        monster["special_abilities"] = abilities

        # Deduplicate: keep first occurrence
        if canonical not in result:
            result[canonical] = monster

    return result


def build_module(text: str, sections: dict[str, str]) -> dict:
    """Build the ModuleDef JSON structure."""
    rooms = {}
    for key, room_def in ROOM_DEFS.items():
        room: dict = {"name": room_def["name"]}

        desc = extract_room_description(text, room_def["number"], room_def["name"])
        room["description"] = desc

        if room_def.get("monsters"):
            room["monsters"] = room_def["monsters"]
        if room_def.get("treasure"):
            room["treasure"] = room_def["treasure"]
        if room_def.get("trap"):
            room["trap"] = room_def["trap"]
        if room_def.get("trap_trigger"):
            room["trap_trigger"] = room_def["trap_trigger"]

        room["exits"] = room_def["exits"]

        if room_def.get("features"):
            room["features"] = room_def["features"]
        if room_def.get("tags"):
            room["tags"] = room_def["tags"]
        if room_def.get("read_aloud"):
            room["read_aloud"] = room_def["read_aloud"]
        if room_def.get("gm_notes"):
            room["gm_notes"] = room_def["gm_notes"]

        rooms[key] = room

    return {
        "name": "Morkaal's Tomb",
        "level_range": [1, 1],
        "entry_room": "main_entry_landing",
        "sections": sections,
        "rooms": rooms,
        "rules": MODULE_RULES,
    }


def build_monsters(named_stats: dict[str, dict]) -> list[dict]:
    """Build the monsters.json list, merging extracted stats with manual metadata."""
    monsters = []

    for raw_name, stats in named_stats.items():
        canonical = NAME_MAP.get(raw_name, raw_name)
        if canonical is None:
            continue

        manual = MONSTER_DEFS.get(canonical, {})
        if manual.get("special_abilities"):
            stats["special_abilities"] = manual["special_abilities"]

        stats["name"] = canonical
        if not any(m["name"] == canonical for m in monsters):
            monsters.append(stats)

    return monsters


def parse_module(md_path: Path) -> tuple[dict, list[dict], str]:
    """Parse Morkaal's Tomb from GotFN V1 markdown.

    Returns (module_dict, monsters_list, chapter_text).
    """
    text = md_path.read_text(encoding="utf-8")

    # Extract only Chapter 6
    ch6_start = text.find("## CHAPTER 6: MORKAAL")
    if ch6_start == -1:
        raise ValueError("Chapter 6 not found in source")
    ch7_start = text.find("## CHAPTER 7:", ch6_start)
    chapter_text = text[ch6_start:ch7_start] if ch7_start != -1 else text[ch6_start:]

    named_stats = assign_stat_block_names(chapter_text)
    sections = extract_sections(chapter_text)
    module = build_module(chapter_text, sections)
    monsters = build_monsters(named_stats)

    return module, monsters, chapter_text


# ---------------------------------------------------------------------------
# Room sort key for display
# ---------------------------------------------------------------------------

def room_sort_key(key: str) -> tuple:
    num = ROOM_DEFS[key]["number"]
    m = re.match(r"(\d+)([A-Z]?)", num)
    if m:
        return (int(m.group(1)), m.group(2))
    return (999, num)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    osr_data = Path.home() / ".osr_data"
    default_input = osr_data / "extracted" / "GotFN_V1.md"
    default_output = osr_data / "modules" / "gotfn_morkaals_tomb"

    md_path = Path(sys.argv[1]) if len(sys.argv) > 1 else default_input
    out_dir = Path(sys.argv[2]) if len(sys.argv) > 2 else default_output

    if not md_path.exists():
        print(f"Error: input file not found: {md_path}", file=sys.stderr)
        sys.exit(1)

    out_dir.mkdir(parents=True, exist_ok=True)

    print(f"Parsing: {md_path}")
    module, monsters, chapter_text = parse_module(md_path)

    # Validate room exits
    room_keys = set(module["rooms"].keys())
    for key, room in module["rooms"].items():
        for exit_def in room.get("exits", []):
            if exit_def["to"] not in room_keys:
                print(f"WARNING: room '{key}' has exit to unknown room '{exit_def['to']}'")

    # Validate bidirectional door states
    for key, room in module["rooms"].items():
        for exit_def in room.get("exits", []):
            other = module["rooms"].get(exit_def["to"])
            if not other:
                continue
            reverse = next(
                (e for e in other.get("exits", []) if e["to"] == key), None
            )
            if reverse and exit_def["door"] != reverse["door"]:
                print(
                    f"WARNING: mismatched door states {key} -> {exit_def['to']}: "
                    f"{exit_def['door']} vs {reverse['door']}"
                )

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

    # Write chapter text as raw.md (not the full book)
    raw_path = out_dir / "raw.md"
    with open(raw_path, "w") as f:
        f.write(chapter_text)
    print(f"Wrote: {raw_path} ({len(chapter_text) // 1024}KB)")

    # Summary
    print(f"\nModule: {module['name']}")
    print(f"Level range: {module['level_range']}")
    print(f"Entry room: {module['entry_room']}")
    print(f"Rooms: {len(module['rooms'])}")
    print(f"Sections: {len(module.get('sections', {}))}")
    for key, text in module.get("sections", {}).items():
        print(f"  {key}: {len(text)} chars")
    print(f"Rules: {len(module.get('rules', {}))}")
    print(f"Monsters defined: {len(monsters)}")

    # List rooms with their monsters/treasure
    for key in sorted(module["rooms"], key=room_sort_key):
        room = module["rooms"][key]
        parts = [f"  {ROOM_DEFS[key]['number']:>3s}. {room['name']}"]
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
        if room.get("features"):
            parts.append(f"  feat:{len(room['features'])}")
        print("".join(parts))


if __name__ == "__main__":
    main()
