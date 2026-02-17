#!/bin/bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PIPELINE_DIR="$(dirname "$(dirname "$SCRIPT_DIR")")"
VENV="$PIPELINE_DIR/.venv"

echo "=== Watershed Connection Detection Experiment ==="

# Ensure dependencies
"$VENV/bin/pip" install -q opencv-python-headless numpy scipy

# Run watershed
"$VENV/bin/python" "$SCRIPT_DIR/step3_watershed.py" "$@"

# Error analysis
echo ""
echo "=== Error Analysis ==="
"$VENV/bin/python" "$SCRIPT_DIR/analyze_errors.py"
