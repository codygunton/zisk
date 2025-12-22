#!/bin/bash
set -e

# Benchmark RISC-V cycles for Zisk (64-bit) execution
#
# Usage: ./bench-zisk-cycles.sh [BLOCK_NUMBER]
#
# Environment variables:
#   SETUP=1  - Run ./setup.sh first (default: 1)

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$REPO_ROOT"

SETUP="${SETUP:-1}"
BLOCK_NUMBER="${1:-22244135}"
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

# Run eth_runner in forward mode with benchmarking
echo "Running eth_runner..."
cd "$ETH_RUNNER_DIR"
export OVERRIDE_ZKSYNC_OS_PATH="$ZKSYNCOS_DIR/zksync_os"
export LIBRARY_PATH="/opt/intel/oneapi/compiler/2025.0/lib:$LIBRARY_PATH"
# export VERBOSE_ORACLE=1  # Uncomment to see detailed oracle query logs
export ZISK_QUIET=1
RUSTFLAGS="-Awarnings" RUST_LOG=eth_runner=info,rig=info cargo run --release \
    --features "rig/zisk-witness,rig/no_print,rig/unlimited_native" \
    -- single-run --block-dir "$BLOCK_DIR" --randomized >> /tmp/zisk-bench.log 2>&1
cd "$REPO_ROOT"

echo ""
echo "Log: /tmp/zisk-bench.log"
echo ""

# Extract key metrics from log
grep -E "(\[GUEST\]|(\[ORACLE\] (Query breakdown|Total queries|Transactions processed|Queries by transaction)|cycles to finish|Native used|Effective cycles))" /tmp/zisk-bench.log || true
