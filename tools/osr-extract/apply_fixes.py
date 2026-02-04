#!/usr/bin/env python3
"""
Apply manual fixes to extracted JSON data.

Usage:
    python apply_fixes.py monsters    # Apply monster fixes
    python apply_fixes.py spells      # Apply spell fixes
"""

import json
import sys
from pathlib import Path

SCRIPT_DIR = Path(__file__).parent
PROJECT_ROOT = SCRIPT_DIR.parent.parent  # tools/osr-extract -> tools -> project root
DATA_DIR = PROJECT_ROOT / "data" / "core"
FIXES_DIR = SCRIPT_DIR / "fixes"


def load_fixes(entity_type: str) -> list[dict]:
    """Load fixes from JSONL file."""
    fixes_file = FIXES_DIR / f"{entity_type}.jsonl"
    if not fixes_file.exists():
        return []

    fixes = []
    for line in fixes_file.read_text().splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        try:
            fixes.append(json.loads(line))
        except json.JSONDecodeError as e:
            print(f"WARNING: Invalid JSON in fixes file: {e}", file=sys.stderr)
    return fixes


def apply_fixes(entity_type: str):
    """Apply fixes to the entity JSON file."""
    json_file = DATA_DIR / f"{entity_type}.json"
    if not json_file.exists():
        print(f"Error: {json_file} not found", file=sys.stderr)
        sys.exit(1)

    data = json.loads(json_file.read_text())
    entities = data.get(entity_type, [])
    fixes = load_fixes(entity_type)

    if not fixes:
        print(f"No fixes found for {entity_type}")
        return

    # Index entities by name for quick lookup
    entity_index = {e["name"]: e for e in entities}

    added = 0
    updated = 0

    for fix in fixes:
        name = fix.get("name")
        if not name:
            continue

        # Remove metadata fields
        fix_data = {k: v for k, v in fix.items() if k not in ("reason",)}

        if name in entity_index:
            # Update existing entity
            entity_index[name].update(fix_data)
            updated += 1
        else:
            # Add new entity
            entities.append(fix_data)
            entity_index[name] = fix_data
            added += 1

    # Update count
    data["count"] = len(entities)
    data[entity_type] = entities

    # Write back
    json_file.write_text(json.dumps(data, indent=2))
    print(f"Applied {updated} updates, {added} additions to {entity_type}")
    print(f"Total {entity_type}: {len(entities)}")


def main():
    if len(sys.argv) < 2:
        print("Usage: python apply_fixes.py <entity_type>")
        print("  e.g.: python apply_fixes.py monsters")
        sys.exit(1)

    entity_type = sys.argv[1]
    apply_fixes(entity_type)


if __name__ == "__main__":
    main()
