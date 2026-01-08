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
# Default to block with actual transactions (155 txs, ~12.8M gas)
# Block 23598300 is empty (0 txs) but has Keccak MPT witness
BLOCK_NUMBER="${1:-19299001}"
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
    FEATURES="proving,unlimited_native,disable_system_contracts,prevrandao,evm_refunds,print_debug_info" ./build.sh --machine airbender
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
# single-run uses "evm_replay" binary name
# single-eth-run uses "app" binary name (default)
ln -sf zksync_os_airbender.bin "$ZKSYNCOS_DIR/zksync_os/for_tests.bin"
ln -sf zksync_os_airbender.elf "$ZKSYNCOS_DIR/zksync_os/for_tests.elf"
ln -sf zksync_os_airbender.bin "$ZKSYNCOS_DIR/zksync_os/evm_replay.bin"
ln -sf zksync_os_airbender.elf "$ZKSYNCOS_DIR/zksync_os/evm_replay.elf"
ln -sf zksync_os_airbender.bin "$ZKSYNCOS_DIR/zksync_os/app.bin"
ln -sf zksync_os_airbender.elf "$ZKSYNCOS_DIR/zksync_os/app.elf"

# Run eth_runner with Airbender simulator
# NO zisk-witness feature - uses Airbender's built-in simulator
# Detects which command to use based on available files:
# - single-eth-run: for Ethereum Keccak MPT model (requires witness.json)
# - single-run: for flat storage model (requires prestatetrace.json, difftrace.json)
# Note: removed rig/no_print to enable guest logging
echo "Running eth_runner..."
cd "$ETH_RUNNER_DIR"
export OVERRIDE_ZKSYNC_OS_PATH="$ZKSYNCOS_DIR/zksync_os"
# export VERBOSE_ORACLE=1  # Uncomment to see detailed oracle query logs

# Detect which command to use based on available files
if [[ -f "$BLOCK_DIR/witness.json" ]]; then
    echo "Using single-eth-run (Keccak MPT model with witness.json)" >> /tmp/airbender-bench.log
    RUSTFLAGS="-Awarnings" RUST_LOG=eth_runner=info,rig=info cargo run --release \
        --features "rig/unlimited_native" \
        -- single-eth-run --block-dir "$BLOCK_DIR" >> /tmp/airbender-bench.log 2>&1
elif [[ -f "$BLOCK_DIR/prestatetrace.json" ]]; then
    echo "Using single-run (flat storage model with prestatetrace.json)" >> /tmp/airbender-bench.log
    RUSTFLAGS="-Awarnings" RUST_LOG=eth_runner=info,rig=info cargo run --release \
        --features "rig/unlimited_native" \
        -- single-run --block-dir "$BLOCK_DIR" >> /tmp/airbender-bench.log 2>&1
else
    echo "ERROR: Block directory doesn't have required files (witness.json or prestatetrace.json)" >> /tmp/airbender-bench.log
    exit 1
fi
cd "$REPO_ROOT"

echo ""
echo "Log: /tmp/airbender-bench.log"
echo ""

# Extract key metrics from log
grep -E "(Running block:|Block gas used:|Simulator.*executed|Native used|Effective cycles|cycles to finish|Took.*cycles|\[GUEST\]|GasMismatch|panicked)" /tmp/airbender-bench.log || true
