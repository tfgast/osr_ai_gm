#!/bin/bash
# Experiment: Run step3 connections with unlabeled map + locations-only annotations
# Goal: Establish baseline without extra symbol annotations (traps, stairs, pits, doors)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PIPELINE_DIR="$(dirname "$(dirname "$SCRIPT_DIR")")"
TOOLS_DIR="$(dirname "$PIPELINE_DIR")"
PROJECT_DIR="$(dirname "$TOOLS_DIR")"
OUTPUT_DIR="$SCRIPT_DIR"

UNLABELED_MAP="$PROJECT_DIR/morkaal_map_unlabeled.png"
STEP2_REVIEWED="$PIPELINE_DIR/pipeline_output/step2_output_reviewed.json"
GROUND_TRUTH="$PIPELINE_DIR/pipeline_output/step3_output_reviewed.json"
COMPARE_SCRIPT="$TOOLS_DIR/map_connection_experiment/compare.py"

MODEL="${1:-opus}"

echo "=== Baseline Connections Experiment ==="
echo "  Model: $MODEL"
echo "  Base map: unlabeled (no original labels)"
echo "  Annotations: locations only (no symbols)"
echo ""

# Step 1: Create locations-only annotated map on unlabeled base
echo "--- Step 1: Create locations-only annotated map ---"
python "$SCRIPT_DIR/make_locations_only_map.py" \
    "$UNLABELED_MAP" \
    "$STEP2_REVIEWED" \
    "$OUTPUT_DIR/annotated_locations_only.png"

# Step 2: Run step3 connections
echo ""
echo "--- Step 2: Run step3 connections ---"
python "$PIPELINE_DIR/step3_connections.py" \
    "$OUTPUT_DIR/annotated_locations_only.png" \
    "$STEP2_REVIEWED" \
    "$OUTPUT_DIR/step3_baseline.json" \
    --model "$MODEL"

# Step 3: Compare against ground truth
echo ""
echo "--- Step 3: Compare against ground truth ---"
python "$COMPARE_SCRIPT" "$GROUND_TRUTH" "$OUTPUT_DIR/step3_baseline.json"
