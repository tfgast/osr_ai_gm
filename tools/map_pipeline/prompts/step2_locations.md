You are analyzing a dungeon map to identify locations that are NOT labeled on the map but are important for player navigation and decision-making.

## Context

A previous step identified the labeled rooms and features on this map. That verified data is provided below. Your task is to suggest NEW locations that should be added to make the dungeon's navigation graph complete and unambiguous.

## Types of Locations to Add

### Hallway Junctions
Where corridors branch and players must choose a direction. If a hallway splits into two or more paths, the junction point is a location.

### River Waypoints
If the map has a river or waterway, rooms are typically on land. Without locations IN or ON the river, it is ambiguous which rooms can connect to which via water. Add waypoints along the river where crossings are possible or where the river passes between rooms.

### Room Splits
Large rooms that span both sides of an obstacle (river, chasm, wall) or that have functionally distinct areas may need to be split into separate locations. For example, a room that spans both banks of a river should become two locations (one per bank).

## Verified Features from Step 1

{step1_json}

## Instructions

1. Study the map image alongside the verified feature list
2. Identify locations where player decisions happen but no label exists
3. For each suggested location, explain WHY it matters for navigation
4. Assign new IDs: J1, J2... for junctions, R1, R2... for river waypoints, use suffixes like 8N/8S for split rooms
5. Estimate pixel positions for each new location

## Output Format

Return a JSON object containing ALL locations (original + new):

{
  "locations": {
    "<existing locations from step 1 — include them unchanged>": "...",
    "J1": {
      "label": "J1",
      "name_guess": "<what this junction looks like>",
      "type": "junction",
      "pixel_x": <x>, "pixel_y": <y>,
      "rationale": "<why this location matters for player navigation>"
    },
    "R1": {
      "label": "R1",
      "name_guess": "<description of this river point>",
      "type": "river_waypoint",
      "pixel_x": <x>, "pixel_y": <y>,
      "rationale": "<why this waypoint is needed>"
    },
    "8N": {
      "label": "8N",
      "name_guess": "<description of split portion>",
      "type": "room_split",
      "split_from": "8",
      "pixel_x": <x>, "pixel_y": <y>,
      "rationale": "<why this room should be split>"
    }
  },
  "symbols": [<unchanged from step 1>],
  "added_locations": ["J1", "R1", "8N", "8S"],
  "observations": "<notes about the new locations and why they matter>"
}

## Important

- Include ALL original locations unchanged — this output replaces the step 1 data
- Only add locations that serve a clear navigation purpose
- River waypoints should be placed where crossing is physically plausible on the map
- Provide a rationale for every new location — the human reviewer needs to understand your reasoning
- Do NOT remove or rename any original labeled rooms
