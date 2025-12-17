#!/bin/bash

# Execute zksync-os with block inputs (Zisk 64-bit witness)
# Usage: ./execute.sh [BLOCK_NUMBER]
# Default block: 22244135

BLOCK_NUMBER="${1:-22244135}"
WITNESS_HEX="/tmp/inputs/${BLOCK_NUMBER}_witness"
WITNESS_BIN="/tmp/inputs/${BLOCK_NUMBER}_witness.bin"

LIBRARY_PATH="/opt/intel/oneapi/compiler/2025.0/lib:$LIBRARY_PATH" \
    cargo build --bin ziskemu --release

# Only run setup if ELF or witness doesn't exist
ELF_FILE="zksync-os/zksync_os/zksync_os_zisk.elf"
if [[ ! -f "$ELF_FILE" ]] || [[ ! -f "$WITNESS_HEX" && ! -f "$WITNESS_BIN" ]]; then
    echo "Running setup (ELF or witness missing)..."
    ./setup_zksyncos.sh "$BLOCK_NUMBER"
fi

# Convert hex witness to binary if needed
if [[ -f "$WITNESS_HEX" ]]; then
    echo "Converting hex witness to binary..."
    xxd -r -p < "$WITNESS_HEX" > "$WITNESS_BIN"
elif [[ ! -f "$WITNESS_BIN" ]]; then
    echo "Error: No witness file found at $WITNESS_HEX or $WITNESS_BIN"
    echo "Generate witness first with: cd zksync-os/zksync_os && ./build_witness.sh blocks/$BLOCK_NUMBER /tmp/inputs --zisk"
    exit 1
fi

./target/release/ziskemu \
    -e zksync-os/zksync_os/zksync_os_zisk.elf \
    -i "$WITNESS_BIN" \
    --oracle \
    -v
