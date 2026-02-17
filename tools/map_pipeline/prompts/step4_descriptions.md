You are generating descriptions for new and split locations in a dungeon.
Original room descriptions have already been preserved verbatim — you are
only handling locations that need new or redistributed text.

## Context

{general_notes}

### Rules
{rules}

## Connection Map (for locations being described)

{connections_subset}

## Split Rooms

For each split room, the original parent room text is provided. Distribute
the relevant parts to each half. Preserve original wording where possible.
Keep the structured format (features, trap, gm_notes, tags, monsters).
Assign each feature/trap/monster to whichever half it physically belongs to.

{split_rooms_data}

## Generated Locations

Create descriptions for these new locations (junctions, river waypoints).
Match the tone of the original module. 2-4 sentences for description.
For junctions: describe the branching paths and any distinguishing features.
For river waypoints: describe the water conditions (depth, current, temperature).

{generated_rooms_data}

## Output Format

Return a JSON object with one key per location ID:

{
  "<location_id>": {
    "key": "<snake_case_slug>",
    "name": "<name>",
    "description": "<prose description>",
    "features": [{"name": "...", "description": "..."}],
    "trap": "<trap description or omit>",
    "gm_notes": "<GM-only notes or null>",
    "tags": ["..."],
    "monsters": [{"name": "...", "count": N}]
  }
}

Only include fields that apply — omit features/trap/monsters/tags if none.
Do NOT include a "read_aloud" field.
Do NOT include a "source" field (it will be added automatically).

## Style Guide

- Match the tone of the original module descriptions
- Keep descriptions concise — 2-4 sentences for new locations
- Emphasize information relevant to player decisions
- Note hazards, environmental conditions, and sensory cues
- Do not include game mechanics (saving throws, damage, etc.) in descriptions — put those in gm_notes
