# Room Connection Mapping Experiment — Morkaal's Tomb

**Date**: 2026-02-16
**Bead**: oag-n5ci4
**Map**: GotFN V1 Map Pack → "Morkaal Tomb Labeled.png" (1550×1518, B&W)

## Summary

Fed the Morkaal's Tomb dungeon map to Gemini vision models via the `gemini` CLI
to test automated extraction of room numbers and inter-room connections.

## Models Tested

| Model | Map | Rooms | Connections | Notes |
|-------|-----|-------|-------------|-------|
| gemini-2.5-pro | Labeled | 17/17 | 18 | Full run, clean JSON |
| gemini-3-pro-preview | Labeled | 17/17 | 18 | 100% agreement with 2.5 Pro |
| gemini-2.5-pro | Unlabeled | 17/17 | 19 | Diverges on 7 connections |
| gemini-2.5-flash | Labeled | FAIL | — | Cannot do vision via CLI agent |

## Key Findings

### Room Detection: Perfect (17/17)

All vision-capable runs correctly identified:
`1, 2, 3A, 3B, 3C, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15`

### 4A Omission

All runs folded area 4A ("Great Bronze Doors") into adjacent rooms rather than
treating it as a separate area. This is arguably correct visually — 4A is a
transitional zone between rooms 4 and 13, not a distinct room on the map.

### Connection Agreement

Gemini 2.5 Pro and 3 Pro produced **identical** results on the labeled map:
- Same 18 unique connections
- Same connection types for all 18

### 18 Agreed Connections (labeled map)

```
1 <-> 3C   open           10 <-> 11  river_crossing
1 <-> 4    open           10 <-> 12  open
2 <-> 3A   open           10 <-> 9   open
3A <-> 3B  open           11 <-> 14  open
3B <-> 3C  open           12 <-> 13  stairs
4 <-> 5    door           13 <-> 7   open
4 <-> 7    open           14 <-> 15  door
5 <-> 6    open            6 <-> 7   door
6 <-> 8    open            7 <-> 9   secret
```

### Unlabeled Map Divergence

7 connections found ONLY by unlabeled run (not in labeled):
```
11 <-> 12   (hanging alcoves — may be correct)
12 <-> 14   (alternative path reading)
13 <-> 4    (direct connection reading)
13 <-> 9    (alternative secret door placement)
3A <-> 3C   (direct cave link)
3C <-> 4    (cave to hall)
4 <-> 6     (direct east door)
```

6 connections found ONLY by labeled runs (not in unlabeled):
```
1 <-> 3C    (entrance to cave)
10 <-> 12   (dead-end to hanging alcove)
11 <-> 14   (alcove to far side)
13 <-> 7    (riven hall to corridor)
3B <-> 3C   (cave link)
6 <-> 7     (door between rooms)
```

## Methodology

1. Map images extracted from V1 Map Pack zip
2. Gemini CLI (`gemini` v0.28.2) in headless mode (`-p`, `-y`, `-o text`)
3. Prompt: structured extraction instructions piped via stdin, map file path in `-p` flag
4. Agent reads map via `read_file` tool → model applies vision to the image
5. JSON extracted from response, compared programmatically

### Important: CLI Agent vs Direct API

The `gemini` CLI is an agent that uses tools. Vision works when the agent's
`read_file` tool reads a PNG and the underlying model has vision capability.
This worked for gemini-2.5-pro and gemini-3-pro-preview, but NOT for
gemini-2.5-flash (which reported it cannot analyze images visually).

### Reliability Issue

Non-deterministic: running the same model twice can produce different results
if the agent takes different tool paths. The `-p "$(cat prompt)"` approach
(no stdin pipe) produced wildly hallucinated output (52 rooms) because the
agent likely didn't read the image file at all.

## Next Steps

1. **Ground truth**: Need manual verification against GotFN V1 Chapter 6 text
   - Template at: `ground_truth_template.json`
   - Existing `ROOM_DEFS` in parser was Opus-generated, not verified
2. **Accuracy scoring**: Once ground truth is available, `compare.py` will compute
   recall/precision/F1 for connections and connection type accuracy
3. **Gemini 3 Pro retry**: Was capacity-limited initially, got one clean run later
4. **Multiple runs**: Run each model 3-5 times to measure consistency
5. **Hybrid approach**: AI draft → human correction workflow evaluation
