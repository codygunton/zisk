#!/bin/bash
set -e

# Build zksync-os for Zisk and generate witness data
#
# Usage: ./setup_zksyncos.sh [BLOCK_NUMBER]
#
# Outputs:
#   zksync-os/zksync_os/zksync_os_zisk.elf  - ELF for Zisk
#   /tmp/inputs/<BLOCK>_witness              - Witness file

ZISK_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BLOCK_NUMBER="${1:-22244135}"
ZKSYNCOS_DIR="$ZISK_DIR/zksync-os/zksync_os"
OUTPUT_ELF="$ZKSYNCOS_DIR/zksync_os_zisk.elf"

LIBRARY_PATH="/opt/intel/oneapi/compiler/2025.0/lib:$LIBRARY_PATH" \
    cargo build --bin ziskemu --release

# Build zksync-os for Zisk
"$ZKSYNCOS_DIR/build.sh" --machine zisk

# ROM setup
echo ""
echo "=== Running ROM setup ==="
cd "$ZISK_DIR"
PROVING_KEY="${PROVING_KEY:-$ZISK_DIR/provingKey}"
./target/release/cargo-zisk rom-setup --elf "$OUTPUT_ELF" --proving-key "$PROVING_KEY" -v

# Generate witness data using Zisk (64-bit)
echo ""
echo "=== Generating witness data (Zisk 64-bit) ==="

INPUTS_DIR="/tmp/inputs"
WITNESS_HEX="$INPUTS_DIR/${BLOCK_NUMBER}_witness"
mkdir -p "$INPUTS_DIR"

ETH_RUNNER_DIR="$ZISK_DIR/zksync-os/tests/instances/eth_runner"
cd "$ETH_RUNNER_DIR"

echo "Running eth_runner with Zisk witness generation for block $BLOCK_NUMBER..."
# Use zisk-witness feature for 64-bit witness generation (not airbender 32-bit)
# LIBRARY_PATH needed for Intel oneAPI liomp5 dependency
LIBRARY_PATH="/opt/intel/oneapi/compiler/2025.0/lib:${LIBRARY_PATH:-}" \
    RUSTFLAGS="-Awarnings" RUST_LOG=eth_runner=info,rig=info cargo run --release \
    --features rig/no_print,rig/unlimited_native,rig/zisk-witness \
    -- single-run \
    --block-dir "$ETH_RUNNER_DIR/blocks/$BLOCK_NUMBER" \
    --randomized \
    --witness-output-dir "$INPUTS_DIR"

echo ""
echo "=== Setup complete ==="
echo "ELF:     $OUTPUT_ELF"
echo "Witness: $WITNESS_HEX"
echo ""
echo "To run with Zisk emulator:"
echo "  ./execute.sh $BLOCK_NUMBER"
