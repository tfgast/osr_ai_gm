#!/usr/bin/env python3
"""Compare AI-extracted room connections against ground truth.

Metrics:
- Room detection: did the AI find all rooms?
- Connection recall: what % of ground truth connections did the AI find?
- Connection precision: what % of AI connections are correct?
- Connection type accuracy: of correct connections, how many have the right type?
"""

import json
import sys
from collections import defaultdict


def normalize_room_id(room_id: str) -> str:
    """Normalize room IDs for comparison."""
    return room_id.strip().upper()


def connection_set(rooms: dict) -> set[tuple[str, str]]:
    """Extract set of (from, to) room connections, normalized."""
    conns = set()
    for room_id, room in rooms.items():
        src = normalize_room_id(room_id)
        for exit_def in room.get("exits", []):
            dst = normalize_room_id(exit_def["to"])
            conns.add((src, dst))
    return conns


def connection_types(rooms: dict) -> dict[tuple[str, str], str]:
    """Extract dict of (from, to) -> connection_type."""
    types = {}
    for room_id, room in rooms.items():
        src = normalize_room_id(room_id)
        for exit_def in room.get("exits", []):
            dst = normalize_room_id(exit_def["to"])
            ctype = exit_def.get("connection_type", "unknown")
            types[(src, dst)] = ctype
    return types


def undirected_set(conns: set[tuple[str, str]]) -> set[frozenset]:
    """Convert directed connections to undirected pairs."""
    return {frozenset((a, b)) for a, b in conns}


def compare(truth_path: str, result_path: str):
    with open(truth_path) as f:
        truth = json.load(f)
    with open(result_path) as f:
        result = json.load(f)

    truth_rooms = set(normalize_room_id(r) for r in truth["rooms"])
    result_rooms = set(normalize_room_id(r) for r in result["rooms"])

    # Room detection
    found = truth_rooms & result_rooms
    missed = truth_rooms - result_rooms
    extra = result_rooms - truth_rooms

    print(f"  Rooms: {len(found)}/{len(truth_rooms)} detected", end="")
    if missed:
        print(f", missed: {sorted(missed)}", end="")
    if extra:
        print(f", extra: {sorted(extra)}", end="")
    print()

    # Connection analysis (undirected - a connection A->B matches B->A)
    truth_conns = connection_set(truth["rooms"])
    result_conns = connection_set(result["rooms"])

    truth_undirected = undirected_set(truth_conns)
    result_undirected = undirected_set(result_conns)

    correct = truth_undirected & result_undirected
    missed_conns = truth_undirected - result_undirected
    extra_conns = result_undirected - truth_undirected

    total_truth = len(truth_undirected)
    total_result = len(result_undirected)
    total_correct = len(correct)

    recall = total_correct / total_truth if total_truth else 0
    precision = total_correct / total_result if total_result else 0
    f1 = 2 * precision * recall / (precision + recall) if (precision + recall) else 0

    print(f"  Connections: {total_correct}/{total_truth} found (recall={recall:.0%}), "
          f"precision={precision:.0%}, F1={f1:.0%}")

    if missed_conns:
        print(f"  Missed connections:")
        for pair in sorted(missed_conns, key=lambda p: sorted(p)):
            a, b = sorted(pair)
            print(f"    {a} <-> {b}")

    if extra_conns:
        print(f"  Extra connections (hallucinated or unlisted):")
        for pair in sorted(extra_conns, key=lambda p: sorted(p)):
            a, b = sorted(pair)
            print(f"    {a} <-> {b}")

    # Connection type accuracy (for correctly identified connections)
    truth_types = connection_types(truth["rooms"])
    result_types = connection_types(result["rooms"])

    type_correct = 0
    type_total = 0
    type_mismatches = []
    for pair in correct:
        a, b = tuple(pair)
        # Check both directions for type match
        truth_type = truth_types.get((a, b)) or truth_types.get((b, a))
        result_type = result_types.get((a, b)) or result_types.get((b, a))
        if truth_type and result_type:
            type_total += 1
            if truth_type.lower() == result_type.lower():
                type_correct += 1
            else:
                type_mismatches.append((a, b, truth_type, result_type))

    if type_total:
        type_acc = type_correct / type_total
        print(f"  Connection types: {type_correct}/{type_total} correct ({type_acc:.0%})")
        for a, b, tt, rt in type_mismatches:
            print(f"    {a}<->{b}: truth={tt}, got={rt}")

    return {
        "rooms_found": len(found),
        "rooms_total": len(truth_rooms),
        "connections_correct": total_correct,
        "connections_total": total_truth,
        "recall": recall,
        "precision": precision,
        "f1": f1,
    }


def main():
    if len(sys.argv) < 3:
        print("Usage: compare.py <ground_truth.json> <result.json>", file=sys.stderr)
        sys.exit(1)
    compare(sys.argv[1], sys.argv[2])


if __name__ == "__main__":
    main()
