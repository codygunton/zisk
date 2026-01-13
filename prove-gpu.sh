#!/bin/bash

# Prove with GPU using ZisK test vectors
# Usage: ./prove-gpu.sh

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$REPO_ROOT"

./target/release/cargo-zisk prove \
    -e zisk-testvectors/pessimistic-proof/elf/pp-keccakf.elf \
    -i zisk-testvectors/pessimistic-proof/inputs/pp_input_1_1.bin \
    --witness-lib ./target/release/libzisk_witness.so \
    --proving-key ./provingKey \
    -t 4 \
    -v
