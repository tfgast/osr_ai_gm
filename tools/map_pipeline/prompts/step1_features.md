You are analyzing a dungeon map image from a tabletop RPG module. Your task is to identify and locate every important feature on the map.

## Module Text Context

{step0_context}

## Instructions

1. Find every numbered room/area label on the map
2. Find every symbol marker (traps, secret doors, pit traps, etc.)
3. Identify major map features (rivers, elevation changes, scale)
4. For each feature, estimate its pixel position where x=0 is the left edge and y=0 is the top edge of the image

## What to Look For

- **Room labels**: Numbers like 1, 2, 3A, 3B, 4A, etc. printed on the map
- **Trap markers**: "T" symbols indicating traps
- **Secret doors**: "S" symbols indicating secret doors
- **Pit traps**: "PP" symbols indicating pit traps
- **Stairs**: Parallel lines indicating level changes
- **Doors**: Gaps in walls with door marks or arcs
- **Rivers/water**: Flowing water features through the dungeon
- **Scale indicators**: Grid scale or measurement references

## Output Format

Return a JSON object with this exact structure:

{
  "locations": {
    "<label>": {
      "label": "<label as printed on map>",
      "name_guess": "<brief description of what this area looks like>",
      "type": "room",
      "pixel_x": <approximate x coordinate of the label>,
      "pixel_y": <approximate y coordinate of the label>
    }
  },
  "symbols": [
    {
      "type": "<trap|secret_door|pit_trap|stairs|door>",
      "label": "<text shown on map, e.g. T, S, PP>",
      "pixel_x": <x coordinate>,
      "pixel_y": <y coordinate>,
      "near_location": "<nearest room label>",
      "notes": "<any observations>"
    }
  ],
  "map_features": {
    "river_path": "<description of how the river flows through the dungeon, if present>",
    "scale": "<grid scale if visible, e.g. '1 square = 10 feet'>",
    "dimensions": {"width": <image width>, "height": <image height>}
  },
  "observations": "<general notes about the map layout>"
}

## Important

- Use room labels EXACTLY as printed on the map (e.g., "3A" not "3a")
- Do NOT invent rooms that don't exist on the map
- Pixel coordinates are approximate — within ~50 pixels is fine
- Report EVERY labeled area you can see, even if partially obscured
- Note any areas where labels are unclear or hard to read
- If module text context is provided, use the area names from it instead of guessing. Match map labels to the text's room labels.
- If the text mentions rooms you cannot find on the map, note them in observations but do NOT add them to locations

## Symbol Reporting Rules

**CRITICAL: Only report symbols you can VISUALLY SEE drawn on the map image.**

- A "T" symbol means you can see the letter T printed on the map
- An "S" symbol means you can see the letter S printed on the map
- A "PP" symbol means you can see the letters PP printed on the map
- Stairs means you can see parallel lines or a staircase drawn on the map
- Doors means you can see door markers (arcs, gaps, or blocks) drawn on walls

**Do NOT infer or hallucinate symbols from the module text.** If the text says a room has a trap but you cannot see a "T" on the map, do NOT add a trap symbol. If the text says there is a secret door but you cannot see an "S" on the map, do NOT add a secret door symbol. The module text tells you what to look for, but you must confirm each symbol visually on the map before reporting it.

In your notes field, indicate whether a symbol was visually confirmed (e.g., "Visible T symbol on map") or if you are uncertain (e.g., "Possible T symbol, hard to read").
