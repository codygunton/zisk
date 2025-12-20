#!/bin/bash

# Execute zksync-os with block inputs (Zisk 64-bit witness replay)
# Usage: ./execute.sh [BLOCK_NUMBER]
# Default block: 22244135

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$REPO_ROOT"

BLOCK_NUMBER="${1:-22244135}"
WITNESS_HEX="/tmp/inputs/${BLOCK_NUMBER}_witness"
ELF_FILE="$REPO_ROOT/zksync-os/zksync_os/zksync_os_zisk.elf"

# Build ziskemu
LIBRARY_PATH="/opt/intel/oneapi/compiler/2025.0/lib:$LIBRARY_PATH" \
    cargo build --bin ziskemu --release

# Only run forward mode if ELF or witness doesn't exist
if [[ ! -f "$ELF_FILE" ]] || [[ ! -f "$WITNESS_HEX" ]]; then
    echo "Running forward mode (ELF or witness missing)..."
    ./forward.sh "$BLOCK_NUMBER"
fi

if [[ ! -f "$WITNESS_HEX" ]]; then
    echo "Error: No witness file found at $WITNESS_HEX"
    exit 1
fi

# Witness file is hex text - ziskemu parses it directly
# Use --uart stderr for clear guest output with [GUEST] prefix
echo ""
echo "=== Running replay mode ==="
ZISK_QUIET="${ZISK_QUIET:-1}" ./target/release/ziskemu \
    -e "$ELF_FILE" \
    -i "$WITNESS_HEX" \
    --oracle \
    --uart stderr \
    -v
