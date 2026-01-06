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
FEATURES="proving,eth_runner,print_debug_info" ./build.sh --machine zisk
cd "$REPO_ROOT"

# ROM setup
echo ""
echo "=== Running ROM setup ==="
PROVING_KEY="${PROVING_KEY:-$REPO_ROOT/provingKey}"
./target/release/cargo-zisk rom-setup --elf "$OUTPUT_ELF" --proving-key "$PROVING_KEY" -v

echo ""
echo "=== Setup complete ==="
echo "ELF: $OUTPUT_ELF"
