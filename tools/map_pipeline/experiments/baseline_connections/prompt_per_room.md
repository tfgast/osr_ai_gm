You are analyzing a dungeon map to identify physical connections for ONE specific location. The map has been annotated with colored labels.

## Label Color Key

- **Blue circles**: Original labeled rooms
- **Green circles**: Hallway junctions (navigation points)
- **Red circles**: River waypoints (water crossings)
- **Orange circles**: Split rooms (large rooms divided into areas)

## Your Target Location

- **ID**: {target_id}
- **Name**: {target_name}
- **Type**: {target_type}
- **Circle color**: {target_color}

## Candidate Neighbors

These are the locations closest to your target. Examine EACH one and determine whether it connects to {target_id}:

{candidates_list}

## Connection Types

- **"door"** — a doorway (gap in wall with door marks or arc)
- **"open"** — an open passage, archway, or tunnel with no door
- **"stairs"** — stairs connecting areas (parallel lines on map)
- **"secret"** — a secret door (marked with "S")
- **"pit"** — a vertical connection via pit or cliff
- **"river_crossing"** — connection across water requiring swimming, wading, or boating

## Instructions

1. Find location **{target_id}** ({target_name}) on the map using its {target_color} circle
2. For EACH candidate neighbor listed above:
   a. Find the candidate on the map
   b. Trace walls and passages between {target_id} and the candidate
   c. Determine: is there a VISIBLE passage, door, stairs, or river crossing connecting them?
   d. Adjacent rooms sharing a wall with NO visible opening are NOT connected
3. Report your findings for every candidate — both connections AND non-connections

## Critical Rules

- It is BETTER TO MISS a real connection than to HALLUCINATE a false one
- Only report a connection if you can trace a clear passage, doorway, stairway, or river path
- Proximity alone does NOT mean connection — walls block movement
- Junctions connect to corridors/rooms they sit between via visible passages
- River waypoints connect to rooms on both banks at that point via water
- If two locations share a wall but have NO visible opening, they are NOT connected

## Output Format

```json
{{
  "target": "{target_id}",
  "connections": [
    {{
      "to": "<candidate id>",
      "connection_type": "<door|open|stairs|secret|pit|river_crossing>",
      "confidence": "<high|medium|low>",
      "notes": "<what you see on the map that confirms this connection>"
    }}
  ],
  "not_connected": [
    {{
      "to": "<candidate id>",
      "reason": "<why there is no connection — e.g., solid wall, no visible passage>"
    }}
  ]
}}
```

Every candidate MUST appear in either `connections` or `not_connected`. Do not skip any.
