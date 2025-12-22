#!/bin/bash
set -e

# Benchmark RISC-V cycles for Airbender (32-bit) execution
#
# This script builds the Airbender binary and runs eth_runner to measure
# cycle counts. Uses Airbender's built-in simulator (not Zisk).
#
# Usage: ./bench-airbender-cycles.sh [BLOCK_NUMBER]

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$REPO_ROOT"

BLOCK_NUMBER="${1:-22244135}"
ZKSYNCOS_DIR="$REPO_ROOT/zksync-os"
ETH_RUNNER_DIR="$ZKSYNCOS_DIR/tests/instances/eth_runner"
BLOCK_DIR="$ETH_RUNNER_DIR/blocks/$BLOCK_NUMBER"

echo "=== Airbender (RV32IM) Cycle Benchmark ==="
echo "Block: $BLOCK_NUMBER"
echo ""

# Clear log file
> /tmp/airbender-bench.log

# Build the Airbender binary (output to log)
# Enable print_debug_info to see UART output from the guest
echo "Building Airbender binary..."
{
    cd "$ZKSYNCOS_DIR/zksync_os"
    FEATURES="proving,eth_runner,print_debug_info" ./build.sh --machine airbender
    cd "$REPO_ROOT"
} >> /tmp/airbender-bench.log 2>&1

# Verify binary exists
if [[ ! -f "$ZKSYNCOS_DIR/zksync_os/zksync_os_airbender.bin" ]]; then
    echo "ERROR: Airbender binary not found at $ZKSYNCOS_DIR/zksync_os/zksync_os_airbender.bin"
    exit 1
fi

# Force rebuild of eth_runner to pick up any changes
touch "$ETH_RUNNER_DIR/src/main.rs"

# Create symlinks so eth_runner can find the binaries
ln -sf zksync_os_airbender.bin "$ZKSYNCOS_DIR/zksync_os/for_tests.bin"
ln -sf zksync_os_airbender.elf "$ZKSYNCOS_DIR/zksync_os/for_tests.elf"
ln -sf zksync_os_airbender.bin "$ZKSYNCOS_DIR/zksync_os/evm_replay.bin"
ln -sf zksync_os_airbender.elf "$ZKSYNCOS_DIR/zksync_os/evm_replay.elf"

# Run eth_runner with Airbender simulator
# NO zisk-witness feature - uses Airbender's built-in simulator
echo "Running eth_runner..."
cd "$ETH_RUNNER_DIR"
export OVERRIDE_ZKSYNC_OS_PATH="$ZKSYNCOS_DIR/zksync_os"
# export VERBOSE_ORACLE=1  # Uncomment to see detailed oracle query logs
RUSTFLAGS="-Awarnings" RUST_LOG=eth_runner=info,rig=info cargo run --release \
    --features "rig/no_print,rig/unlimited_native" \
    -- single-run --block-dir "$BLOCK_DIR" >> /tmp/airbender-bench.log 2>&1
cd "$REPO_ROOT"

echo ""
echo "Log: /tmp/airbender-bench.log"
echo ""

# Extract key metrics from log
grep -E "(\[GUEST\]|(\[ORACLE\] (Query breakdown|Total queries|Transactions processed|Queries by transaction)|cycles to finish|Native used|Effective cycles))" /tmp/airbender-bench.log || true
