#!/bin/bash
# Experiment: Per-room connection queries for step 3
# Sends one LLM call per location with K=10 nearest candidates,
# then aggregates via bidirectional voting.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PIPELINE_DIR="$(dirname "$(dirname "$SCRIPT_DIR")")"
TOOLS_DIR="$(dirname "$PIPELINE_DIR")"
PROJECT_DIR="$(dirname "$TOOLS_DIR")"

UNLABELED_MAP="$PROJECT_DIR/morkaal_map_unlabeled.png"
STEP2_REVIEWED="$PIPELINE_DIR/pipeline_output/step2_output_reviewed.json"
GROUND_TRUTH="$PIPELINE_DIR/pipeline_output/step3_output_reviewed.json"
COMPARE_SCRIPT="$TOOLS_DIR/map_connection_experiment/compare.py"

MODEL="${1:-opus}"

echo "=== Per-Room Connections Experiment ==="
echo "  Model: $MODEL"
echo ""

# Step 1: Ensure locations-only annotated map exists
if [ ! -f "$SCRIPT_DIR/annotated_locations_only.png" ]; then
    echo "--- Creating locations-only annotated map ---"
    python "$SCRIPT_DIR/make_locations_only_map.py" \
        "$UNLABELED_MAP" \
        "$STEP2_REVIEWED" \
        "$SCRIPT_DIR/annotated_locations_only.png"
    echo ""
fi

# Step 2: Run per-room queries + aggregation
echo "--- Running per-room queries ---"
python "$SCRIPT_DIR/step3_per_room.py" --model "$MODEL"

# Step 3: Detailed error analysis
echo ""
echo "--- Error Analysis ---"
python "$SCRIPT_DIR/analyze_errors.py"
