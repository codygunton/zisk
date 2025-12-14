#!/bin/bash

# Execute zksync-os with block inputs
# Usage: ./execute.sh [BLOCK_NUMBER]
# Default block: 22244135

BLOCK_NUMBER="${1:-22244135}"
INPUTS_BIN="/tmp/inputs/${BLOCK_NUMBER}_inputs.bin"

LIBRARY_PATH="/opt/intel/oneapi/compiler/2025.0/lib:$LIBRARY_PATH" \
    cargo build --bin ziskemu --release

./target/release/ziskemu \
    -e zksync-os/zksync_os/zksync_os_zisk.elf \
    -i "$INPUTS_BIN" \
    --oracle \
    -v
