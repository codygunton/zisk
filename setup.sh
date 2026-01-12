#!/bin/bash
set -e

# Build zksync-os for Zisk and run ROM setup
#
# Usage: ./setup.sh
#
# Outputs:
#   zksync-os/zksync_os/zksync_os_zisk.elf  - ELF for Zisk
#   ~/.zisk/cache/*                          - ROM setup artifacts

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$REPO_ROOT"

# Set library path for Intel OpenMP (required for linking)
export LIBRARY_PATH="/opt/intel/oneapi/compiler/2025.0/lib:$LIBRARY_PATH"

ZKSYNCOS_DIR="$REPO_ROOT/zksync-os"
# Output path is determined by zksync-os's build.sh
OUTPUT_ELF="$ZKSYNCOS_DIR/zksync_os/zksync_os_zisk.elf"

# Build ziskemu, cargo-zisk, and ziskclib
echo "=== Building ziskemu, cargo-zisk, and ziskclib ==="
cargo build -p ziskemu -p cargo-zisk -p ziskclib --release

# Build zksync-os for ZisK
# Enable print_debug_info to see UART output from the guest
echo ""
echo "=== Building zksync-os for Zisk ==="
cd "$ZKSYNCOS_DIR/zksync_os"
FEATURES="proving,print_debug_info,delegation,global-alloc,pectra,evm_refunds,unlimited_native,prevrandao,disable_system_contracts,zisk_keccak" ./build.sh --machine zisk
cd "$REPO_ROOT"

# ROM setup (skip if no proving key available)
PROVING_KEY="${PROVING_KEY:-$REPO_ROOT/provingKey}"
if [[ ! -d "$PROVING_KEY" ]]; then
    echo ""
    echo "=== Skipping ROM setup (no proving key at $PROVING_KEY) ==="
else
    echo ""
    echo "=== Running ROM setup ==="
    ./target/release/cargo-zisk rom-setup --elf "$OUTPUT_ELF" --proving-key "$PROVING_KEY" -v
fi

echo ""
echo "=== Setup complete ==="
echo "ELF: $OUTPUT_ELF"
