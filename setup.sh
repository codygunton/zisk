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

# Require GCC 14 for pil2-proofman compatibility
# GCC 15+ has stricter include requirements that pil2-proofman doesn't satisfy yet
# See: https://github.com/0xPolygonHermez/pil2-proofman/issues/382
GCC14="${GCC14:-/usr/bin/gcc-14}"
GXX14="${GXX14:-/usr/bin/g++-14}"

if [[ ! -x "$GCC14" ]] || [[ ! -x "$GXX14" ]]; then
    echo "ERROR: GCC 14 is required but not found at expected locations:"
    echo "  GCC14=$GCC14"
    echo "  GXX14=$GXX14"
    echo ""
    echo "The pil2-proofman dependency has missing #include <cstdint> in some headers,"
    echo "which causes compilation failures with GCC 15+."
    echo ""
    echo "To fix, either:"
    echo "  1. Install GCC 14 (e.g., 'sudo pacman -S gcc14' on Arch)"
    echo "  2. Set GCC14 and GXX14 environment variables to your GCC 14 installation"
    echo "  3. Patch the cargo cache (see REPRODUCIBILITY_NOTES.md)"
    exit 1
fi

export CC="$GCC14"
export CXX="$GXX14"
echo "Using GCC 14: $CC / $CXX"

# Set library path for Intel OpenMP (required for linking)
export LIBRARY_PATH="/opt/intel/oneapi/compiler/2025.0/lib:$LIBRARY_PATH"

# Fetch dependencies first (needed to get pil2-proofman checkout)
echo ""
echo "=== Fetching cargo dependencies ==="
cargo fetch

# Pre-build pil2-stark library with GCC 14
# The build.rs in proofman-starks-lib-c doesn't pass CXX to make,
# so we need to build it manually first
PIL2_STARK_DIR=$(find ~/.cargo/git/checkouts/pil2-proofman-* -maxdepth 2 -name "pil2-stark" -type d 2>/dev/null | head -1)
if [[ -n "$PIL2_STARK_DIR" && -d "$PIL2_STARK_DIR" ]]; then
    LIBSTARKS="$PIL2_STARK_DIR/lib/libstarks.a"
    if [[ ! -f "$LIBSTARKS" ]]; then
        echo ""
        echo "=== Pre-building pil2-stark with GCC 14 ==="
        (cd "$PIL2_STARK_DIR" && make clean && make CXX="$GXX14" -j starks_lib)
    else
        echo "pil2-stark library already built: $LIBSTARKS"
    fi
else
    echo "WARNING: Could not find pil2-stark directory in cargo cache"
    echo "The build may fail if GCC 15+ is used"
fi

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
