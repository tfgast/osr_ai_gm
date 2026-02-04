# OSE Data Extraction Tools

Extract game data from Old-School Essentials PDFs into JSON format.

## Directory Structure

```
~/.osr_data/                    # Central data location (not in git)
  pdfs/                         # Source PDFs (copyrighted, never commit)
  extracted/                    # docling output (intermediate files)

tools/osr-extract/              # This directory
  extract.sh                    # docling wrapper
  parse/                        # Parser scripts
    monsters.py                 # Monster stat block parser
    spells.py                   # Spell definition parser
    items.py                    # Magic item parser
  fixes/                        # Manual corrections (JSONL)
  schemas/                      # JSON schemas for validation

data/                           # Output directory (committed)
  core/                         # Official OSE data
    monsters.json
    spells.json
    magic_items.json
  modules/                      # Third-party module data
```

## Prerequisites

```bash
pip install docling
```

## Usage

### 1. Extract PDFs to text

```bash
# Place OSE PDFs in ~/.osr_data/pdfs/
./extract.sh                    # Extract all
./extract.sh Advanced_Fantasy_Referees_Tome_v1-3.pdf  # Extract one
```

### 2. Parse extracted text

```bash
python parse/monsters.py        # Parse monster stat blocks
python parse/spells.py          # Parse spells
```

### 3. Apply manual fixes

Fixes are stored as JSONL in `fixes/`:
```json
{"id": "goblin", "field": "xp_value", "value": 5, "reason": "OSE errata"}
```

```bash
python apply_fixes.py monsters  # Apply monster fixes
```

## Output Format

See `schemas/` for JSON schemas. Example monster:

```json
{
  "name": "Goblin",
  "hit_dice": "1-1",
  "armor_class": 6,
  "attacks": ["weapon"],
  "damage": "1d6 or weapon",
  "movement": 60,
  "morale": 7,
  "xp_value": 5,
  "num_appearing": "2d4",
  "special": "Infravision 90', -1 to hit in daylight"
}
```

## Adding Manual Fixes

When the parser gets something wrong:

1. Add a line to the appropriate `fixes/*.jsonl` file
2. Run `apply_fixes.py` to regenerate the JSON
3. Commit the fix file (not the intermediate extracted text)
