#!/bin/bash
set -e

# Run zksync-os in forward mode (live oracle) and generate witness
#
# Usage: ./forward.sh [BLOCK_NUMBER]
#
# Environment variables:
#   SETUP=1  - Run ./setup.sh first (default: 1)
#
# Outputs:
#   /tmp/inputs/<BLOCK>_witness  - Witness file for replay mode (name set by eth_runner)

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$REPO_ROOT"

SETUP="${SETUP:-1}"
BLOCK_NUMBER="${1:-22244135}"
ZKSYNCOS_DIR="$REPO_ROOT/zksync-os"

# Optionally run setup first
if [[ "$SETUP" == "1" ]]; then
    ./setup.sh
fi

# Generate witness data using ZisK in forward mode
echo ""
echo "=== Running forward mode (Zisk 64-bit) ==="

INPUTS_DIR="/tmp/inputs"
WITNESS_HEX="$INPUTS_DIR/${BLOCK_NUMBER}_witness"
mkdir -p "$INPUTS_DIR"

ETH_RUNNER_DIR="$ZKSYNCOS_DIR/tests/instances/eth_runner"

# Create symlinks so eth_runner can find the binaries.
# eth_runner looks for generic names (for_tests.*, evm_replay.*) rather than
# the architecture-specific names (zksync_os_zisk.*).
ln -sf zksync_os_zisk.bin "$ZKSYNCOS_DIR/zksync_os/for_tests.bin"
ln -sf zksync_os_zisk.elf "$ZKSYNCOS_DIR/zksync_os/for_tests.elf"
ln -sf zksync_os_zisk.bin "$ZKSYNCOS_DIR/zksync_os/evm_replay.bin"
ln -sf zksync_os_zisk.elf "$ZKSYNCOS_DIR/zksync_os/evm_replay.elf"

echo "Running eth_runner with Zisk witness generation for block $BLOCK_NUMBER..."
# Per-step logging controlled by RUST_LOG (trace level is very verbose)
cd "$ETH_RUNNER_DIR"
export OVERRIDE_ZKSYNC_OS_PATH="$ZKSYNCOS_DIR/zksync_os"
LIBRARY_PATH="/opt/intel/oneapi/compiler/2025.0/lib:${LIBRARY_PATH:-}" \
    RUSTFLAGS="-Awarnings" RUST_LOG=eth_runner=info,rig=info cargo run --release \
    --features rig/no_print,rig/unlimited_native,rig/zisk-witness \
    -- single-run \
    --block-dir "$ETH_RUNNER_DIR/blocks/$BLOCK_NUMBER" \
    --randomized \
    --witness-output-dir "$INPUTS_DIR"
cd "$REPO_ROOT"

echo ""
echo "=== Forward mode complete ==="
echo "Witness: $WITNESS_HEX"
echo ""
echo "To replay with Zisk emulator:"
echo "  ./execute.sh $BLOCK_NUMBER"
