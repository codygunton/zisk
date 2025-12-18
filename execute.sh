#!/bin/bash

# Execute zksync-os with block inputs (Zisk 64-bit witness)
# Usage: ./execute.sh [BLOCK_NUMBER]
# Default block: 22244135

BLOCK_NUMBER="${1:-22244135}"
WITNESS_HEX="/tmp/inputs/${BLOCK_NUMBER}_witness"

LIBRARY_PATH="/opt/intel/oneapi/compiler/2025.0/lib:$LIBRARY_PATH" \
    cargo build --bin ziskemu --release

# Only run setup if ELF or witness doesn't exist
ELF_FILE="zksync-os/zksync_os/zksync_os_zisk.elf"
if [[ ! -f "$ELF_FILE" ]] || [[ ! -f "$WITNESS_HEX" ]]; then
    echo "Running setup (ELF or witness missing)..."
    ./setup_zksyncos.sh "$BLOCK_NUMBER"
fi

if [[ ! -f "$WITNESS_HEX" ]]; then
    echo "Error: No witness file found at $WITNESS_HEX"
    exit 1
fi

# Witness file is hex text - ziskemu parses it directly
ZISK_QUIET=1 ./target/release/ziskemu \
    -e zksync-os/zksync_os/zksync_os_zisk.elf \
    -i "$WITNESS_HEX" \
    --oracle \
    -v
