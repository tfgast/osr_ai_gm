You are extracting room and area information from a tabletop RPG module's text. This information will be used to guide a vision model that analyzes the dungeon map image in a later step.

## Module Text

{module_text}

## Instructions

Extract every keyed area (room, corridor, junction, etc.) mentioned in the text. For each one, provide:

1. **label**: The room number/label as it would appear on the map (e.g., "1", "3A", "4A")
2. **name**: The area's name from the module text
3. **summary**: A 1-2 sentence summary focusing on what a GM would see on the map — physical layout, notable features visible from the room (doors, stairs, pits, water, statues, sarcophagi). Do NOT include monster stats, treasure values, or detailed trap mechanics.
4. **spatial_clues**: Any mentions of cardinal directions, relative positions, or connections to other rooms (e.g., "north of area 4", "corridor branches east and west", "river crossing to area 8")
5. **expected_symbols**: What map symbols you'd expect to see at or near this room — traps (T), secret doors (S), pit traps (PP), stairs, doors, etc.
6. **on_map**: Whether this room is expected to appear on the dungeon level map. Set to false for rooms explicitly described as being off-map (e.g., reached only via teleporter).

Also extract any general dungeon information:
- Default construction (stone type, ceiling heights)
- River/water features and their general flow
- Scale information
- Wandering monster policy

## Output Format

```json
{
  "areas": {
    "<label>": {
      "label": "<map label>",
      "name": "<area name>",
      "summary": "<1-2 sentence physical description>",
      "spatial_clues": ["<clue 1>", "<clue 2>"],
      "expected_symbols": ["<symbol type>"],
      "on_map": true
    }
  },
  "dungeon_info": {
    "name": "<dungeon name>",
    "construction": "<default wall/floor/ceiling materials and heights>",
    "water_features": "<description of rivers, pools, etc.>",
    "scale": "<map scale if mentioned>",
    "wandering_monsters": "<policy>"
  },
  "observations": "<any additional notes about layout or structure>"
}
```

## Important

- Use room labels exactly as they appear in the text (e.g., "3A" not "3a")
- Focus on VISUAL and SPATIAL information useful for map analysis
- Keep summaries brief — the goal is context for a vision model, not a full description
- Note connections between areas, especially non-obvious ones (secret doors, river crossings, teleporters)
- If the text references rooms by internal key names, use the numbered label instead
