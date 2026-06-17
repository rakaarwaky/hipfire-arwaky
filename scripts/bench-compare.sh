#!/bin/bash
# hipfire vs hipfire-arwaky benchmark comparison
# Runs alternating benchmarks to compare performance
# Usage: bash scripts/bench-compare.sh [model] [runs] [prompt]
set -uo pipefail

MODEL="${1:-carnice-9b.mq4}"
RUNS="${2:-3}"
PROMPT="${3:-What is the capital of France?}"

HIPFIRE_DIR="$HOME/.hipfire"
ARWAKY_DIR="$HOME/.hipfire-arwaky"

HIPFIRE_MODEL="$HIPFIRE_DIR/models/$MODEL"
ARWAKY_MODEL="$ARWAKY_DIR/models/$MODEL"

if [ ! -f "$HIPFIRE_MODEL" ]; then
    echo "ERROR: Model not found: $HIPFIRE_MODEL"
    exit 1
fi
if [ ! -f "$ARWAKY_MODEL" ]; then
    echo "ERROR: Model not found: $ARWAKY_MODEL"
    exit 1
fi

echo "=========================================="
echo " hipfire vs hipfire-arwaky benchmark"
echo "=========================================="
echo "Model:    $MODEL"
echo "Runs:     $RUNS per engine"
echo "Prompt:   $PROMPT"
echo ""

RESULTS_H=()
RESULTS_A=()

for i in $(seq 1 $RUNS); do
    echo "=== Round $i/$RUNS ==="

    # --- hipfire ---
    echo -n "  hipfire:   "
    OUTPUT_H=$(echo "$PROMPT" | timeout 60 "$HIPFIRE_DIR/bin/hipfire" run "$HIPFIRE_MODEL" --temp 0.0 --max-seq 512 2>&1)
    TOKS_H=$(echo "$OUTPUT_H" | grep -oP '[\d.]+(?= tok/s)' | tail -1)
    if [ -n "$TOKS_H" ]; then
        echo "${TOKS_H} tok/s"
        RESULTS_H+=("$TOKS_H")
    else
        echo "FAILED"
        RESULTS_H+=("0")
    fi

    sleep 2

    # --- hipfire-arwaky ---
    echo -n "  arwaky:    "
    OUTPUT_A=$(echo "$PROMPT" | timeout 60 "$ARWAKY_DIR/bin/hipfire-arwaky-run" "$ARWAKY_MODEL" --temp 0.0 --max-seq 512 2>&1)
    TOKS_A=$(echo "$OUTPUT_A" | grep -oP '[\d.]+(?= tok/s)' | tail -1)
    if [ -n "$TOKS_A" ]; then
        echo "${TOKS_A} tok/s"
        RESULTS_A+=("$TOKS_A")
    else
        echo "FAILED"
        RESULTS_A+=("0")
    fi

    sleep 2
    echo ""
done

echo "=========================================="
echo " Results"
echo "=========================================="

# Calculate average
avg() {
    local sum=0 n=0
    for v in "$@"; do
        if [ "$v" != "0" ] && [ -n "$v" ]; then
            sum=$(echo "$sum + $v" | bc 2>/dev/null || echo "$sum")
            n=$((n + 1))
        fi
    done
    [ $n -gt 0 ] && echo "scale=1; $sum / $n" | bc 2>/dev/null || echo "N/A"
}

AVG_H=$(avg "${RESULTS_H[@]}")
AVG_A=$(avg "${RESULTS_A[@]}")

echo "  hipfire:        ${RESULTS_H[*]}"
echo "  hipfire-arwaky: ${RESULTS_A[*]}"
echo ""
echo "  Average hipfire:        $AVG_H tok/s"
echo "  Average hipfire-arwaky: $AVG_A tok/s"

if [ "$AVG_H" != "N/A" ] && [ "$AVG_A" != "N/A" ]; then
    DIFF=$(echo "scale=1; ($AVG_A - $AVG_H) / $AVG_H * 100" | bc 2>/dev/null)
    if [ -n "$DIFF" ]; then
        if (( $(echo "$DIFF > 0" | bc -l 2>/dev/null || echo 0) )); then
            echo "  -> arwaky is +${DIFF}% faster"
        else
            echo "  -> hipfire is +${DIFF#-}% faster"
        fi
    fi
fi

echo ""
echo "=========================================="
