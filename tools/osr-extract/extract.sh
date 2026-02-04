#!/bin/bash
# Extract text from OSE PDFs using docling
#
# Prerequisites:
#   pip install docling
#
# Usage:
#   ./extract.sh                    # Extract all PDFs
#   ./extract.sh <pdf-name>         # Extract specific PDF

set -euo pipefail

OSR_DATA="${HOME}/.osr_data"
PDF_DIR="${OSR_DATA}/pdfs"
OUT_DIR="${OSR_DATA}/extracted"
VENV="${OSR_DATA}/venv"
DOCLING="${VENV}/bin/docling"

if [[ ! -f "$DOCLING" ]]; then
    echo "Error: docling not found. Install with:"
    echo "  python -m venv $VENV"
    echo "  $VENV/bin/pip install docling"
    exit 1
fi

if [[ ! -d "$PDF_DIR" ]]; then
    echo "Error: PDF directory not found: $PDF_DIR"
    echo "Please place OSE PDFs in $PDF_DIR"
    exit 1
fi

mkdir -p "$OUT_DIR"

extract_pdf() {
    local pdf="$1"
    local basename=$(basename "$pdf" .pdf)
    local outfile="${OUT_DIR}/${basename}.md"

    if [[ -f "$outfile" ]]; then
        echo "Skipping $basename (already extracted)"
        return 0
    fi

    echo "Extracting: $basename"
    "$DOCLING" --image-export-mode referenced "$pdf" --output "$OUT_DIR"
    echo "Done: $outfile"
}

if [[ $# -gt 0 ]]; then
    # Extract specific PDF
    pdf="${PDF_DIR}/$1"
    if [[ ! -f "$pdf" ]]; then
        echo "Error: PDF not found: $pdf"
        exit 1
    fi
    extract_pdf "$pdf"
else
    # Extract all PDFs
    for pdf in "$PDF_DIR"/*.pdf; do
        [[ -f "$pdf" ]] || continue
        extract_pdf "$pdf"
    done
fi

echo ""
echo "Extracted files in: $OUT_DIR"
ls -la "$OUT_DIR"/*.md 2>/dev/null || echo "(no markdown files yet)"
