#!/bin/bash

# Prove zksync-os execution with GPU
# Usage: ./prove-gpu.sh [BLOCK_NUMBER]
# Default block: 22244135

BLOCK_NUMBER="${1:-22244135}"
INPUTS_BIN="/tmp/inputs/${BLOCK_NUMBER}_inputs.bin"

./target/release/cargo-zisk prove \
    -e zksync-os/zksync_os/zksync_os_for_zisk.elf \
    -i "$INPUTS_BIN" \
    --witness-lib ./target/release/libzisk_witness.so \
    --proving-key ./provingKey \
    -t 4 \
    -vvv
