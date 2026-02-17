#!/usr/bin/env python3
"""Deep error analysis comparing step3 outputs against ground truth.

Categorizes errors by:
- Map region (which areas are problematic)
- Connection type (which types get confused)
- Node type (rooms vs junctions vs river waypoints)
- Error class (missing, hallucinated, type mismatch)
- Spatial proximity (are hallucinated connections between nearby/distant rooms)
"""

import json
import sys
from collections import defaultdict
from pathlib import Path


def load(path):
    with open(path) as f:
        return json.load(f)


def norm(s):
    return s.strip().upper()


def conn_set(rooms):
    """Undirected connection set as frozensets."""
    conns = set()
    for rid, r in rooms.items():
        for e in r.get("exits", []):
            conns.add(frozenset((norm(rid), norm(e["to"]))))
    return conns


def conn_types(rooms):
    """Dict of frozenset -> connection_type (picks first seen direction)."""
    types = {}
    for rid, r in rooms.items():
        for e in r.get("exits", []):
            key = frozenset((norm(rid), norm(e["to"])))
            if key not in types:
                types[key] = (e.get("connection_type") or e.get("type", "unknown")).lower()
    return types


def conn_notes(rooms):
    """Dict of frozenset -> notes from first direction seen."""
    notes = {}
    for rid, r in rooms.items():
        for e in r.get("exits", []):
            key = frozenset((norm(rid), norm(e["to"])))
            if key not in notes:
                notes[key] = e.get("notes", "")
    return notes


def get_node_type(loc_id, locations):
    """Get location type from step2 data."""
    loc = locations.get(loc_id) or locations.get(loc_id.lower())
    if loc:
        return loc.get("type", "room")
    return "unknown"


def pixel_dist(a_id, b_id, locations):
    """Euclidean pixel distance between two locations."""
    a = locations.get(a_id) or locations.get(a_id.lower())
    b = locations.get(b_id) or locations.get(b_id.lower())
    if not a or not b:
        return None
    ax, ay = a.get("pixel_x", 0), a.get("pixel_y", 0)
    bx, by = b.get("pixel_x", 0), b.get("pixel_y", 0)
    return ((ax - bx)**2 + (ay - by)**2) ** 0.5


def analyze(label, result_data, truth_data, locations):
    print(f"\n{'='*60}")
    print(f"  {label}")
    print(f"{'='*60}")

    truth_conns = conn_set(truth_data["rooms"])
    result_conns = conn_set(result_data["rooms"])
    truth_types = conn_types(truth_data["rooms"])
    result_types = conn_types(result_data["rooms"])
    result_notes = conn_notes(result_data["rooms"])

    correct = truth_conns & result_conns
    missed = truth_conns - result_conns
    extra = result_conns - truth_conns

    # --- Per-node error counts ---
    print(f"\n--- Per-Node Error Breakdown ---")
    node_missed = defaultdict(list)
    node_extra = defaultdict(list)
    for pair in missed:
        a, b = sorted(pair)
        node_missed[a].append(b)
        node_missed[b].append(a)
    for pair in extra:
        a, b = sorted(pair)
        node_extra[a].append(b)
        node_extra[b].append(a)

    all_nodes = sorted(set(
        list(node_missed.keys()) + list(node_extra.keys())
    ))

    # Count correct connections per node too
    node_correct = defaultdict(int)
    for pair in correct:
        for n in pair:
            node_correct[n] += 1

    # Count truth connections per node
    node_truth_count = defaultdict(int)
    for pair in truth_conns:
        for n in pair:
            node_truth_count[n] += 1

    print(f"  {'Node':<6} {'Type':<16} {'OK':>3} {'Miss':>5} {'Extra':>5} {'Truth':>6}  Details")
    print(f"  {'-'*6} {'-'*16} {'-'*3} {'-'*5} {'-'*5} {'-'*6}  {'-'*30}")
    for n in sorted(all_nodes):
        ntype = get_node_type(n, locations)
        m = len(node_missed.get(n, []))
        e = len(node_extra.get(n, []))
        c = node_correct[n]
        t = node_truth_count[n]
        details = []
        if node_missed.get(n):
            details.append(f"miss: {','.join(sorted(node_missed[n]))}")
        if node_extra.get(n):
            details.append(f"extra: {','.join(sorted(node_extra[n]))}")
        print(f"  {n:<6} {ntype:<16} {c:>3} {m:>5} {e:>5} {t:>6}  {'; '.join(details)}")

    # --- By connection type ---
    print(f"\n--- Errors by Ground Truth Connection Type ---")
    type_stats = defaultdict(lambda: {"correct": 0, "missed": 0})
    for pair in correct:
        ct = truth_types.get(pair, "unknown")
        type_stats[ct]["correct"] += 1
    for pair in missed:
        ct = truth_types.get(pair, "unknown")
        type_stats[ct]["missed"] += 1

    print(f"  {'Type':<16} {'Correct':>8} {'Missed':>8} {'Recall':>8}")
    print(f"  {'-'*16} {'-'*8} {'-'*8} {'-'*8}")
    for ct in sorted(type_stats.keys()):
        s = type_stats[ct]
        total = s["correct"] + s["missed"]
        recall = s["correct"] / total if total else 0
        print(f"  {ct:<16} {s['correct']:>8} {s['missed']:>8} {recall:>7.0%}")

    # --- Hallucinated connections by type ---
    print(f"\n--- Hallucinated Connections by Claimed Type ---")
    hallu_type = defaultdict(list)
    for pair in extra:
        ct = result_types.get(pair, "unknown")
        a, b = sorted(pair)
        hallu_type[ct].append(f"{a}<->{b}")
    for ct in sorted(hallu_type.keys()):
        print(f"  {ct}: {', '.join(sorted(hallu_type[ct]))}")

    # --- Type mismatches ---
    print(f"\n--- Type Mismatches (correct connection, wrong type) ---")
    mismatches = []
    for pair in correct:
        tt = truth_types.get(pair, "unknown")
        rt = result_types.get(pair, "unknown")
        if tt != rt:
            a, b = sorted(pair)
            mismatches.append((a, b, tt, rt))
    if mismatches:
        for a, b, tt, rt in sorted(mismatches):
            print(f"  {a}<->{b}: truth={tt}, got={rt}")
    else:
        print(f"  (none)")

    # --- Spatial analysis of hallucinated connections ---
    print(f"\n--- Spatial Analysis: Hallucinated Connections ---")
    dists = []
    for pair in extra:
        a, b = sorted(pair)
        d = pixel_dist(a, b, locations)
        note = result_notes.get(pair, "")
        if d is not None:
            dists.append((d, a, b, note))
    dists.sort()
    if dists:
        print(f"  {'Dist':>6}  {'Connection':<16} Notes")
        print(f"  {'-'*6}  {'-'*16} {'-'*40}")
        for d, a, b, note in dists:
            print(f"  {d:>6.0f}  {a}<->{b:<10} {note[:60]}")
        avg_d = sum(d for d, _, _, _ in dists) / len(dists)
        print(f"\n  Average hallucination distance: {avg_d:.0f}px")

    # --- Spatial analysis of missed connections ---
    print(f"\n--- Spatial Analysis: Missed Connections ---")
    dists_m = []
    for pair in missed:
        a, b = sorted(pair)
        d = pixel_dist(a, b, locations)
        ct = truth_types.get(pair, "unknown")
        if d is not None:
            dists_m.append((d, a, b, ct))
    dists_m.sort()
    if dists_m:
        print(f"  {'Dist':>6}  {'Connection':<16} Type")
        print(f"  {'-'*6}  {'-'*16} {'-'*16}")
        for d, a, b, ct in dists_m:
            print(f"  {d:>6.0f}  {a}<->{b:<10} {ct}")
        avg_d = sum(d for d, _, _, _ in dists_m) / len(dists_m)
        print(f"\n  Average missed distance: {avg_d:.0f}px")

    # --- By node type ---
    print(f"\n--- Errors by Node Type ---")
    ntype_stats = defaultdict(lambda: {"missed": 0, "extra": 0, "correct": 0, "truth": 0})
    for pair in truth_conns:
        for n in pair:
            nt = get_node_type(n, locations)
            ntype_stats[nt]["truth"] += 1
    for pair in correct:
        for n in pair:
            nt = get_node_type(n, locations)
            ntype_stats[nt]["correct"] += 1
    for pair in missed:
        for n in pair:
            nt = get_node_type(n, locations)
            ntype_stats[nt]["missed"] += 1
    for pair in extra:
        for n in pair:
            nt = get_node_type(n, locations)
            ntype_stats[nt]["extra"] += 1

    # Halve counts since each connection touches 2 nodes
    print(f"  {'Node Type':<16} {'Correct':>8} {'Missed':>8} {'Extra':>8} {'Recall':>8} {'Precision':>10}")
    print(f"  {'-'*16} {'-'*8} {'-'*8} {'-'*8} {'-'*8} {'-'*10}")
    for nt in sorted(ntype_stats.keys()):
        s = ntype_stats[nt]
        recall = s["correct"] / s["truth"] if s["truth"] else 0
        precision = s["correct"] / (s["correct"] + s["extra"]) if (s["correct"] + s["extra"]) else 0
        print(f"  {nt:<16} {s['correct']:>8} {s['missed']:>8} {s['extra']:>8} {recall:>7.0%} {precision:>9.0%}")


def main():
    pipeline_dir = Path(__file__).parent.parent.parent
    output_dir = pipeline_dir / "pipeline_output"
    exp_dir = Path(__file__).parent

    truth = load(output_dir / "step3_output_reviewed.json")
    pipeline_result = load(output_dir / "step3_output.json")
    baseline_result = load(exp_dir / "step3_baseline.json")

    step2 = load(output_dir / "step2_output_reviewed.json")
    locations = step2.get("locations", {})
    # Normalize location keys to uppercase for matching
    locations = {k.upper(): v for k, v in locations.items()}

    analyze("PIPELINE (labeled map + full annotations)", pipeline_result, truth, locations)
    analyze("BASELINE (unlabeled map + locations only)", baseline_result, truth, locations)

    # Per-room experiment variants (if available)
    per_room_unanimous = exp_dir / "step3_per_room_unanimous.json"
    per_room_any = exp_dir / "step3_per_room_any.json"
    if per_room_unanimous.exists():
        analyze("PER-ROOM UNANIMOUS (both directions)", load(per_room_unanimous), truth, locations)
    if per_room_any.exists():
        analyze("PER-ROOM ANY (at least one direction)", load(per_room_any), truth, locations)

    # Watershed experiment (if available)
    watershed_path = exp_dir / "step3_watershed.json"
    if watershed_path.exists():
        analyze("WATERSHED (CV segmentation)", load(watershed_path), truth, locations)


if __name__ == "__main__":
    main()
