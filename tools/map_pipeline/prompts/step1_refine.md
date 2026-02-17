You are reviewing an annotated dungeon map where colored markers show the positions estimated in a previous analysis pass. Your task is to check each marker's position and correct any that are wrong.

The markers are:
- **Blue circles with white text**: Room/area labels (original numbered rooms)
- **Red/yellow/grey diamonds**: Symbol markers (traps, secret doors, doors, stairs)

## Previous Analysis Result

{previous_json}

## Instructions

1. For each location marker, check if the colored circle is placed on or very near the actual room label text on the underlying map. If it's offset, provide corrected pixel coordinates.
2. For each symbol marker, check if the diamond is placed on or very near the actual symbol on the map. Correct any that are offset.
3. Remove any symbols that you cannot visually confirm exist on the map — they may have been hallucinated from the module text.
4. Add any symbols you can now see that were missed in the first pass.
5. The pixel coordinates should point to where the label/symbol is actually drawn on the map, not to the center of the room.

## Output Format

Return the COMPLETE corrected JSON in the same format as the previous analysis (locations, symbols, map_features, observations). Include ALL locations and symbols, not just the ones you changed. Update the observations to note what you corrected.

## Important

- Only adjust coordinates that are visibly wrong — don't shift things that are already close
- **Only report symbols you can VISUALLY SEE on the map** — remove any that were inferred from text but not visible
- In notes, indicate confidence: "Confirmed visible" or "Uncertain, may be artifact"
- Keep map_features.dimensions from the previous pass
