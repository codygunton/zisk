#!/bin/bash
set -e

# Build zksync-os and run ROM setup for Zisk proving
# Usage: ./setup_zksyncos.sh

ZISK_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# ZISK_DIR already set above
ZKSYNCOS_DIR="$ZISK_DIR/zksync-os/zksync_os"

# Configuration
FEATURES="${FEATURES:-proving,eth_runner}"
OUTPUT_ELF="$ZKSYNCOS_DIR/zksync_os_for_zisk.elf"
PROVING_KEY="${PROVING_KEY:-$ZISK_DIR/provingKey}"

echo "=== Building zksync-os ==="
cd "$ZKSYNCOS_DIR"
cargo build --features "$FEATURES" --release

echo ""
echo "=== Creating Zisk-compatible ELF ==="
cargo objcopy --features "$FEATURES" --release -- \
    -R .heap -R .stack \
    "$OUTPUT_ELF"

ls -lh "$OUTPUT_ELF"

echo ""
echo "=== Running ROM setup ==="
cd "$ZISK_DIR"
./target/release/cargo-zisk rom-setup \
    --elf "$OUTPUT_ELF" \
    --proving-key "$PROVING_KEY" \
    -v

echo ""
echo "=== Done ==="
echo "ELF: $OUTPUT_ELF"
echo "ROM setup complete"
