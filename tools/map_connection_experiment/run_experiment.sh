#!/usr/bin/env bash
# Room Connection Mapping Experiment
# Uses Gemini vision models to extract room connections from dungeon maps
#
# Usage: ./run_experiment.sh [model] [map_variant]
#   model:       gemini-3-pro-preview (default) | gemini-2.5-pro | gemini-2.5-flash
#   map_variant: labeled (default) | unlabeled | both

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
RESULTS_DIR="$SCRIPT_DIR/results"
mkdir -p "$RESULTS_DIR"

MODEL="${1:-gemini-2.5-pro}"
VARIANT="${2:-labeled}"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)

LABELED_MAP="$PROJECT_DIR/morkaal_map_labeled.png"
UNLABELED_MAP="$PROJECT_DIR/morkaal_map_unlabeled.png"

# The extraction prompt - asking for structured JSON output
PROMPT_FILE="$SCRIPT_DIR/extraction_prompt.md"

run_extraction() {
    local map_file="$1"
    local variant_name="$2"
    local output_file="$RESULTS_DIR/${MODEL//\//_}_${variant_name}_${TIMESTAMP}.json"
    local raw_file="$RESULTS_DIR/${MODEL//\//_}_${variant_name}_${TIMESTAMP}_raw.txt"

    echo "=== Running: model=$MODEL variant=$variant_name ==="
    echo "    Map: $map_file"
    echo "    Output: $output_file"

    # Read the prompt template
    local prompt
    prompt=$(cat "$PROMPT_FILE")

    # Call gemini CLI in headless mode with yolo (auto-approve file reads)
    # The prompt references the map file path for the agent to read
    local full_prompt="$prompt

The dungeon map image to analyze is at: $map_file

Output ONLY the JSON object, no markdown fences, no commentary."

    echo "$full_prompt" | timeout 300 gemini -m "$MODEL" -o text -y -p "" 2>/dev/null > "$raw_file" || {
        echo "    ERROR: gemini call failed (exit $?)"
        return 1
    }

    echo "    Raw output saved: $raw_file"

    # Try to extract JSON from the raw output
    python3 "$SCRIPT_DIR/extract_json.py" "$raw_file" "$output_file"

    if [ -f "$output_file" ]; then
        local room_count
        room_count=$(python3 -c "import json; d=json.load(open('$output_file')); print(len(d.get('rooms',{})))" 2>/dev/null || echo "?")
        echo "    Extracted $room_count rooms"
    fi
}

echo "Room Connection Mapping Experiment"
echo "Model: $MODEL"
echo "Variant: $VARIANT"
echo "Timestamp: $TIMESTAMP"
echo ""

case "$VARIANT" in
    labeled)
        run_extraction "$LABELED_MAP" "labeled"
        ;;
    unlabeled)
        run_extraction "$UNLABELED_MAP" "unlabeled"
        ;;
    both)
        run_extraction "$LABELED_MAP" "labeled"
        echo ""
        run_extraction "$UNLABELED_MAP" "unlabeled"
        ;;
esac

echo ""
echo "=== Experiment complete ==="
echo "Results in: $RESULTS_DIR"

# Run comparison if ground truth exists
if [ -f "$SCRIPT_DIR/ground_truth.json" ]; then
    echo ""
    echo "=== Comparing against ground truth ==="
    for f in "$RESULTS_DIR"/*_${TIMESTAMP}.json; do
        [ -f "$f" ] || continue
        echo "--- $(basename "$f") ---"
        python3 "$SCRIPT_DIR/compare.py" "$SCRIPT_DIR/ground_truth.json" "$f"
    done
fi
