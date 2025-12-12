#!/bin/bash
set -e

# Build zksync-os for Zisk and generate witness data
#
# Usage: ./setup_zksyncos.sh [BLOCK_NUMBER]
#
# Outputs:
#   zksync-os/zksync_os/zksync_os_for_zisk.elf  - ELF for Zisk
#   /tmp/inputs/<BLOCK>_witness                  - Witness file

ZISK_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BLOCK_NUMBER="${1:-22244135}"
ZKSYNCOS_DIR="$ZISK_DIR/zksync-os/zksync_os"
OUTPUT_ELF="$ZKSYNCOS_DIR/zksync_os_for_zisk.elf"

# Build zksync-os for Zisk
"$ZKSYNCOS_DIR/build.sh" zisk

# ROM setup
echo ""
echo "=== Running ROM setup ==="
cd "$ZISK_DIR"
PROVING_KEY="${PROVING_KEY:-$ZISK_DIR/provingKey}"
./target/release/cargo-zisk rom-setup --elf "$OUTPUT_ELF" --proving-key "$PROVING_KEY" -v

# Generate witness data
echo ""
echo "=== Generating witness data ==="

# Download 32-bit evm_replay.bin for airbender (our build is 64-bit for Zisk)
ZKSYNC_OS_VERSION="${ZKSYNC_OS_VERSION:-v0.2.5}"
if [[ ! -f "$ZKSYNCOS_DIR/evm_replay.bin" ]] || [[ $(stat -c%s "$ZKSYNCOS_DIR/evm_replay.bin") -lt 1000000 ]]; then
    echo "Downloading 32-bit evm_replay.bin for airbender..."
    curl -fsSL -o "$ZKSYNCOS_DIR/evm_replay.bin" \
        "https://github.com/matter-labs/zksync-os/releases/download/$ZKSYNC_OS_VERSION/evm_replay.bin"
fi

INPUTS_DIR="/tmp/inputs"
WITNESS_HEX="$INPUTS_DIR/${BLOCK_NUMBER}_witness"
mkdir -p "$INPUTS_DIR"

ETH_RUNNER_DIR="$ZISK_DIR/zksync-os/tests/instances/eth_runner"
cd "$ETH_RUNNER_DIR"

echo "Running eth_runner for block $BLOCK_NUMBER..."
RUSTFLAGS="-Awarnings" RUST_LOG=eth_runner=info cargo run --release \
    --features rig/no_print,rig/unlimited_native \
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
