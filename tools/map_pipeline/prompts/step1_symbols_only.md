You are analyzing a dungeon map image from a tabletop RPG module. Room positions have already been identified — your task is to find **symbols and map features only**.

## Module Text Context

{step0_context}

## Known Room Positions

The following room labels and their pixel positions have already been verified:

{known_labels}

You do NOT need to find or locate these rooms. They are provided for spatial reference only — use them to describe where symbols are relative to rooms.

## Instructions

1. Find every symbol marker on the map (traps, secret doors, pit traps, stairs, doors)
2. Identify major map features (rivers, elevation changes, scale)
3. For each symbol, estimate its pixel position where x=0 is the left edge and y=0 is the top edge of the image

## What to Look For

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
  "symbols": [
    {
      "type": "<trap|secret_door|pit_trap|stairs|door>",
      "label": "<text shown on map, e.g. T, S, PP>",
      "pixel_x": <x coordinate>,
      "pixel_y": <y coordinate>,
      "near_location": "<nearest room label from the known list>",
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

## Symbol Reporting Rules

**CRITICAL: Only report symbols you can VISUALLY SEE drawn on the map image.**

- A "T" symbol means you can see the letter T printed on the map
- An "S" symbol means you can see the letter S printed on the map
- A "PP" symbol means you can see the letters PP printed on the map
- Stairs means you can see parallel lines or a staircase drawn on the map
- Doors means you can see door markers (arcs, gaps, or blocks) drawn on walls

**Do NOT infer or hallucinate symbols from the module text.** If the text says a room has a trap but you cannot see a "T" on the map, do NOT add a trap symbol. If the text says there is a secret door but you cannot see an "S" on the map, do NOT add a secret door symbol. The module text tells you what to look for, but you must confirm each symbol visually on the map before reporting it.

In your notes field, indicate whether a symbol was visually confirmed (e.g., "Visible T symbol on map") or if you are uncertain (e.g., "Possible T symbol, hard to read").
