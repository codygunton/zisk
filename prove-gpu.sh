#!/bin/bash

# Prove with GPU using ZisK test vectors
# Usage: ./prove-gpu.sh [ELF_FILE] [INPUT_FILE] [OUTPUT_DIR] [WITNESS_FILE]
#
# Uses pessimistic-proof test vectors by default, or specify custom ELF/input.
# Optional WITNESS_FILE for oracle-based programs like ZKsyncOS.
#
# Prerequisites:
# - Build with GPU support: cargo build --release --features gpu
# - Download proving key (see book/building-for-gpu.md)
# - Configure memlock limits in /etc/security/limits.conf (or use sudo)
#
# Note: There's a known cleanup segfault after proof completion. The script
# checks for successful proof generation and returns success despite the crash.

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$REPO_ROOT"

# Default test vectors
ELF_FILE="${1:-zisk-testvectors/pessimistic-proof/elf/pp-keccakf.elf}"
INPUT_FILE="${2:-zisk-testvectors/pessimistic-proof/inputs/pp_input_1_1.bin}"
OUTPUT_DIR="${3:-tmp}"

# Optional witness file for oracle-based programs
WITNESS_FILE="${4:-}"
WITNESS_ARG=""
if [[ -n "$WITNESS_FILE" ]]; then
    if [[ ! -f "$WITNESS_FILE" ]]; then
        echo "ERROR: Witness file not found: $WITNESS_FILE"
        exit 1
    fi
    WITNESS_ARG="--witness-file $WITNESS_FILE"
    echo "Witness: $WITNESS_FILE"
fi

# Rust dylib requires std library in LD_LIBRARY_PATH
RUST_STD_PATH="$(rustc --print sysroot)/lib/rustlib/x86_64-unknown-linux-gnu/lib"
export LD_LIBRARY_PATH="$RUST_STD_PATH:${LD_LIBRARY_PATH:-}"

# Intel OneAPI library path (required for proofman)
export LIBRARY_PATH="/opt/intel/oneapi/compiler/2025.0/lib:${LIBRARY_PATH:-}"

echo "=== ZisK GPU Proving ==="
echo "ELF: $ELF_FILE"
echo "Input: $INPUT_FILE"
echo "Output: $OUTPUT_DIR"
echo ""

# Run prover and capture output (there's a known cleanup segfault after success)
OUTPUT=$(./target/release/cargo-zisk prove \
    --elf "$ELF_FILE" \
    --input "$INPUT_FILE" \
    $WITNESS_ARG \
    --witness-lib ./target/release/libzisk_witness.so \
    --proving-key ./provingKey \
    --output-dir "$OUTPUT_DIR" \
    --emulator \
    -r \
    -t 4 \
    -v 2>&1) || true

echo "$OUTPUT"

# Check if proof generation succeeded (before cleanup crash)
if echo "$OUTPUT" | grep -q "All proofs were successfully generated"; then
    echo ""
    echo "=== Proof generation SUCCESSFUL ==="
    echo "Output files in: $OUTPUT_DIR/"
    ls -la "$OUTPUT_DIR/" 2>/dev/null || true
    exit 0
else
    echo ""
    echo "=== Proof generation FAILED ==="
    exit 1
fi
