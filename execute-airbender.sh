#!/bin/bash
set -e

# Execute Ethereum blocks using Airbender (RV32IM) simulator
#
# This script builds the Airbender binary and runs eth_runner to execute
# blocks. Uses Airbender's built-in simulator (not ZisK).
#
# Usage: ./execute-airbender.sh [BLOCK_NUMBER]

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$REPO_ROOT"
# Default to block with Keccak MPT witness (requires block_hashes.json)
# Block 22244135 has 155 txs (~12.8M gas) but missing block_hashes.json
# Block 23598300 is empty (0 txs) but has all required files for Keccak MPT
BLOCK_NUMBER="${1:-24198369}" # Ethereum block with 426 txs, ~45M gas
# BLOCK_NUMBER="${1:-22244135}"
# BLOCK_NUMBER="${1:-19299001}"
ZKSYNCOS_DIR="$REPO_ROOT/zksync-os"
ETH_RUNNER_DIR="$ZKSYNCOS_DIR/tests/instances/eth_runner"
BLOCK_DIR="$ETH_RUNNER_DIR/blocks/$BLOCK_NUMBER"

echo "=== Airbender (RV32IM) Cycle Benchmark ==="
echo "Block: $BLOCK_NUMBER"
echo ""

# Clear log file
> /tmp/airbender-execute.log

# Build the Airbender binary (output to log)
# Enable print_debug_info to see UART output from the guest
echo "Building Airbender binary..."
{
    cd "$ZKSYNCOS_DIR/zksync_os"
    FEATURES="proving,unlimited_native,disable_system_contracts,prevrandao,evm_refunds,print_debug_info,global-alloc,pectra" ./build.sh --machine airbender
    cd "$REPO_ROOT"
} >> /tmp/airbender-execute.log 2>&1

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

# SKIP_SIMULATION=1 to skip RISC-V simulation (faster, but no cycle counts)
SKIP_SIM_FLAG=""
if [[ "${SKIP_SIMULATION:-0}" == "1" ]]; then
    echo "SKIP_SIMULATION=1: Skipping RISC-V simulation (bootloader only)" >> /tmp/airbender-execute.log
fi

# Detect which command to use based on available files
if [[ -f "$BLOCK_DIR/witness.json" ]]; then
    echo "Using single-eth-run (Keccak MPT model with witness.json)" >> /tmp/airbender-execute.log
    if [[ "${SKIP_SIMULATION:-0}" == "1" ]]; then
        SKIP_SIM_FLAG="--skip-witness"
    fi
    # pectra feature enables type 3 (blob) and type 4 (EIP-7702) transaction support
    # cycle_marker enables cycle count output from the simulator
    RUSTFLAGS="-Awarnings" RUST_LOG=eth_runner=info,rig=info cargo run --release \
        --features "pectra,rig/unlimited_native" \
        -- single-eth-run --block-dir "$BLOCK_DIR" $SKIP_SIM_FLAG >> /tmp/airbender-execute.log 2>&1
elif [[ -f "$BLOCK_DIR/prestatetrace.json" ]]; then
    echo "Using single-run (flat storage model with prestatetrace.json)" >> /tmp/airbender-execute.log
    # cycle_marker enables cycle count output from the simulator
    RUSTFLAGS="-Awarnings" RUST_LOG=eth_runner=info,rig=info cargo run --release \
        --features "rig/unlimited_native,cycle_marker" \
        -- single-run --block-dir "$BLOCK_DIR" >> /tmp/airbender-execute.log 2>&1
else
    echo "ERROR: Block directory doesn't have required files (witness.json or prestatetrace.json)" >> /tmp/airbender-execute.log
    exit 1
fi
cd "$REPO_ROOT"

echo ""
echo "Log: /tmp/airbender-execute.log"
echo ""

# Extract key metrics from log (matching zisk output format)
grep -E "(Running block:|Block gas used:|Expected block hash:|Took.*cycles to finish|\[GUEST\].*(Using|Withdrawals root|Finished processing)|Proof output hash:|All good|panicked|GasMismatch)" /tmp/airbender-execute.log || true
