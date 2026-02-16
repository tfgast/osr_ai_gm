You are analyzing a dungeon map image from a tabletop RPG module. Your task is to identify all numbered rooms and map the physical connections (exits/passages) between them.

## Instructions

1. Read the dungeon map image carefully
2. Identify every numbered room/area on the map
3. For each room, identify all visible connections (doors, passages, stairs, tunnels) to other numbered rooms
4. Note the type of each connection based on what you see on the map

## Connection Types

Classify each connection as one of:
- "door" — a doorway (may show as a gap in the wall with door marks)
- "open" — an open passage/archway/tunnel with no door
- "stairs" — stairs connecting areas (usually shown with parallel lines)
- "secret" — a secret door (usually shown with an "S" mark)
- "pit" — a vertical connection via pit
- "river_crossing" — connection across water/river requiring a jump or swim

## Map Symbols to Watch For

- "T" markers typically indicate traps
- "S" markers typically indicate secret doors
- "PP" markers indicate pit traps
- Numbers (1, 2, 3A, 3B, etc.) identify rooms/areas
- Rivers/water shown as flowing blue/grey lines
- Dotted or dashed lines may indicate hidden passages

## Output Format

Return a JSON object with this exact structure:

{
  "rooms": {
    "<room_number>": {
      "name_guess": "<brief description of what the room looks like on the map>",
      "exits": [
        {
          "to": "<destination room number>",
          "connection_type": "<door|open|stairs|secret|river_crossing|pit>",
          "notes": "<any observations about this connection>"
        }
      ]
    }
  },
  "observations": "<any general notes about the map layout, things you're uncertain about>"
}

Use the room numbers exactly as they appear on the map (e.g., "1", "2", "3A", "3B", "4", "4A", etc.).

Be thorough — every physical passage between rooms should be listed. If you're uncertain about a connection, include it with a note explaining your uncertainty.
