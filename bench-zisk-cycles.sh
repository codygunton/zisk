#!/bin/bash
set -e

# Benchmark RISC-V cycles for Zisk (64-bit) execution
#
# Usage: ./bench-zisk-cycles.sh [BLOCK_NUMBER]
#
# Environment variables:
#   SETUP=1           - Run ./setup.sh first (default: 1)
#   SKIP_SIMULATION=1 - Skip ZisK emulation, run bootloader only (faster, no cycle counts)

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$REPO_ROOT"

SETUP="${SETUP:-1}"
# BLOCK_NUMBER="${1:-22244135}"
# BLOCK_NUMBER="${1:-19299001}"  # Flat storage block (no witness.json)
BLOCK_NUMBER="${1:-24198369}"  # Ethereum block with 426 txs, ~45M gas
ZKSYNCOS_DIR="$REPO_ROOT/zksync-os"
ETH_RUNNER_DIR="$ZKSYNCOS_DIR/tests/instances/eth_runner"
BLOCK_DIR="$ETH_RUNNER_DIR/blocks/$BLOCK_NUMBER"

echo "=== Zisk (RV64IM) Cycle Benchmark ==="
echo "Block: $BLOCK_NUMBER"
echo ""

# Clear log file
> /tmp/zisk-bench.log

# Optionally run setup first
if [[ "$SETUP" == "1" ]]; then
    echo "Running setup..."
    ./setup.sh >> /tmp/zisk-bench.log 2>&1
fi

# Force rebuild of eth_runner to pick up any changes
touch "$ETH_RUNNER_DIR/src/main.rs"

# Create symlinks so eth_runner can find the binaries
ln -sf zksync_os_zisk.bin "$ZKSYNCOS_DIR/zksync_os/for_tests.bin"
ln -sf zksync_os_zisk.elf "$ZKSYNCOS_DIR/zksync_os/for_tests.elf"
ln -sf zksync_os_zisk.bin "$ZKSYNCOS_DIR/zksync_os/evm_replay.bin"
ln -sf zksync_os_zisk.elf "$ZKSYNCOS_DIR/zksync_os/evm_replay.elf"
# Default app name used by single_eth_run
ln -sf zksync_os_zisk.bin "$ZKSYNCOS_DIR/zksync_os/app.bin"
ln -sf zksync_os_zisk.elf "$ZKSYNCOS_DIR/zksync_os/app.elf"

# Run eth_runner in forward mode with benchmarking
echo "Running eth_runner..."
cd "$ETH_RUNNER_DIR"
export OVERRIDE_ZKSYNC_OS_PATH="$ZKSYNCOS_DIR/zksync_os"
export LIBRARY_PATH="/opt/intel/oneapi/compiler/2025.0/lib:$LIBRARY_PATH"
export VERBOSE_ORACLE=1 # Uncomment to see detailed oracle query logs
# export ZISK_QUIET=1  # Uncomment to suppress zisk output

# SKIP_SIMULATION=1 to skip ZisK emulation (faster, but no cycle counts)
SKIP_SIM_FLAG=""
if [[ "${SKIP_SIMULATION:-0}" == "1" ]]; then
    echo "SKIP_SIMULATION=1: Skipping ZisK emulation (bootloader only)" >> /tmp/zisk-bench.log
    SKIP_SIM_FLAG="--skip-witness"
fi

# Use single-eth-run which uses Ethereum storage with Keccak MPT (matches ZisK build)
# pectra feature enables type 3 (blob) and type 4 (EIP-7702) transaction support
RUSTFLAGS="-Awarnings" RUST_LOG=eth_runner=info,rig=info cargo run --release \
    --features "pectra,rig/zisk-witness,rig/no_print,rig/unlimited_native" \
    -- single-eth-run --block-dir "$BLOCK_DIR" $SKIP_SIM_FLAG >> /tmp/zisk-bench.log 2>&1
cd "$REPO_ROOT"

echo ""
echo "Log: /tmp/zisk-bench.log"
echo ""

# Extract key metrics from log
grep -E "(Running block:|Block gas used:|process_rom\(\) steps|Expected block hash:|Forward storage diff hash:|Proof output hash:|\[GUEST\].*G2 deserialization|\[GUEST\].*Withdrawals root|\[GUEST\].*Finished processing|\[GUEST\].*Using ZisK Keccak|All good)" /tmp/zisk-bench.log || true
