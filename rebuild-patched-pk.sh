#!/usr/bin/env bash
#
# Rebuild the ZisK proving key from the patched PIL.
#
# state-machines/arith/pil/arith.pil on this branch carries the np-sign
# constraint. This script recompiles zisk.pilout and regenerates the proving
# key so the patched Arith AIR is what the verifier actually enforces. Output
# goes to ~/.zisk/provingKey-patched; the stock ~/.zisk/provingKey is left
# untouched.
#
# Only the basic per-AIR setup is generated, not the recursive/aggregation
# setup (`-r`). That is enough for GPU-backed `verify-constraints`, which is
# what the repro uses for the patched circuit. The proposed fix compiles as a
# degree-5 Arith constraint, and the recursive proof path is intentionally left
# for the ZisK team to review rather than treated as part of this repro.
#
# Mirrors the steps of tools/test-env/build_setup.sh. Versions of the pil2
# toolchain are pinned in Dockerfile.repro-arith-mul (tools/test-env/.env).
#
set -euo pipefail

zisk=/workspace/zisk
std_pil=/workspace/pil2-proofman/pil2-components/lib/std/pil
pil_compiler=/workspace/pil2-compiler/src/pil.js
proofman_js=/workspace/pil2-proofman-js/src/main_setup.js
patched_pk="${HOME}/.zisk/provingKey-patched"

cd "$zisk"

echo "[rebuild] generating fixed data..."
cargo run --release --bin arith_frops_fixed_gen
cargo run --release --bin binary_basic_frops_fixed_gen
cargo run --release --bin binary_extension_frops_fixed_gen

echo "[rebuild] compiling pil/zisk.pil -> pil/zisk.pilout (patched Arith AIR)..."
mkdir -p tmp/fixed
node --max-old-space-size=16384 "$pil_compiler" pil/zisk.pil \
    -I pil,"$std_pil",state-machines,precompiles \
    -o pil/zisk.pilout -u tmp/fixed -O fixed-to-file

echo "[rebuild] generating basic setup (no recursive setup)..."
rm -rf build/provingKey
node --max-old-space-size=16384 --stack-size=8192 "$proofman_js" \
    -a ./pil/zisk.pilout -b build \
    -u tmp/fixed \
    -s state-machines/starkstructs.json

echo "[rebuild] installing patched proving key -> ${patched_pk}"
rm -rf "$patched_pk" "${patched_pk}.tmp"
cp -R build/provingKey "${patched_pk}.tmp"
mv "${patched_pk}.tmp" "$patched_pk"

echo "[rebuild] done."
