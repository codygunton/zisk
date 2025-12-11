#!/bin/bash
set -e

ZISK_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
BLOCK_NUMBER="${1:-22244135}"
FEATURES="${FEATURES:-proving,eth_runner}"

# Build guest program
ZKSYNCOS_DIR="$ZISK_DIR/zksync-os/zksync_os"
cd "$ZKSYNCOS_DIR"
cargo build --features "$FEATURES" --release
OUTPUT_ELF="$ZKSYNCOS_DIR/zksync_os_for_zisk.elf"
# Remove .heap and .stack NOBITS sections to avoid generating ~58M zero-init instructions
cargo objcopy --features "$FEATURES" --release -- -R .heap -R .stack "$OUTPUT_ELF"

# ROM setup
cd "$ZISK_DIR"
PROVING_KEY="${PROVING_KEY:-$ZISK_DIR/provingKey}"
./target/release/cargo-zisk rom-setup --elf "$OUTPUT_ELF" --proving-key "$PROVING_KEY" -v

# Generate inputs to guest program (n.b. uses airbender 32-bit simulator)
# Download 32-bit evm_replay.bin for airbender (our build is 64-bit for Zisk)
ZKSYNC_OS_VERSION="${ZKSYNC_OS_VERSION:-v0.2.5}"
if [[ ! -f "$ZKSYNCOS_DIR/evm_replay.bin" ]] || [[ $(stat -c%s "$ZKSYNCOS_DIR/evm_replay.bin") -lt 1000000 ]]; then
    curl -fsSL -o "$ZKSYNCOS_DIR/evm_replay.bin" \
        "https://github.com/matter-labs/zksync-os/releases/download/$ZKSYNC_OS_VERSION/evm_replay.bin"
fi
INPUTS_DIR="/tmp/inputs"
INPUTS_BIN="$INPUTS_DIR/${BLOCK_NUMBER}_inputs.bin"
mkdir -p "$INPUTS_DIR"
ETH_RUNNER_DIR="$ZISK_DIR/zksync-os/tests/instances/eth_runner"
cd "$ETH_RUNNER_DIR"
RUSTFLAGS="-Awarnings" RUST_LOG=eth_runner=info cargo run --release \
    --features rig/no_print,rig/unlimited_native \
    -- single-run \
    --block-dir "$ETH_RUNNER_DIR/blocks/$BLOCK_NUMBER" \
    --randomized \
    --witness-output-dir "$INPUTS_DIR"
xxd -r -p "$INPUTS_DIR/${BLOCK_NUMBER}_witness" > "$INPUTS_BIN"
rm "$INPUTS_DIR/${BLOCK_NUMBER}_witness"
