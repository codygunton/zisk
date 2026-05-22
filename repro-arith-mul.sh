#!/usr/bin/env bash
#
# Arith MUL malicious-witness repro + fix demonstration.
#
#   Phase 1  stock circuit   + malicious witness -> proof VERIFIES  (the bug)
#   Phase 2  rebuild the proving key from the patched arith.pil
#   Phase 3a patched circuit + malicious witness -> proof REJECTED  (the fix)
#   Phase 3b patched circuit + honest witness    -> proof VERIFIES  (still correct)
#
# Every phase runs `prove --verify-proofs --gpu`: it generates a STARK proof
# and runs the stock verifier on it. The malicious witness (claiming
# MUL(-1,1) = 1) is injected under ZISK_REPRO_BAD_ARITH_MUL=1.
#
# The patch (state-machines/arith/pil/arith.pil) adds one constraint --
#   signed * (1 - div) * (np - (na + nb - 2*na*nb)) === 0
# -- forcing the product's sign bit to follow the operand signs. It adds no
# trace columns, so the same cargo-zisk binary drives both proving keys.
#
set -uo pipefail

elf=/tmp/zisk-mul-edge/arith_bad_mul.elf
cargo_zisk=/workspace/zisk/target-docker/release/cargo-zisk
stock_pk="${HOME}/.zisk/provingKey"
patched_pk="${HOME}/.zisk/provingKey-patched"

banner() {
    printf '\n================================================================\n'
    printf '  %s\n' "$1"
    printf '================================================================\n'
}

# prove <proving-key> <malicious:0|1> <output-proof>
prove() {
    local pk=$1 malicious=$2 out=$3
    if [ "$malicious" = "1" ]; then
        ZISK_REPRO_BAD_ARITH_MUL=1 RUST_LOG=info \
            "$cargo_zisk" prove --elf "$elf" --emulator -k "$pk" --gpu \
            --verify-proofs -o "$out"
    else
        RUST_LOG=info \
            "$cargo_zisk" prove --elf "$elf" --emulator -k "$pk" --gpu \
            --verify-proofs -o "$out"
    fi
}

# --- stock proving key ----------------------------------------------------
if [ ! -d "$stock_pk" ]; then
    banner "Installing stock v0.18.0 proving key via ziskup"
    ziskup --version 0.18.0 --provingkey --gpu -y \
        || { echo "ERROR: ziskup failed to install the proving key" >&2; exit 1; }
fi

# --- PHASE 1: stock circuit, malicious witness ----------------------------
banner "PHASE 1/3  stock circuit + MALICIOUS witness"
echo "The stock Arith AIR has no constraint pinning the product's sign."
echo "Expectation (the bug): a proof of MUL(-1,1) = 1 is generated and VERIFIES."
echo
prove "$stock_pk" 1 /tmp/arith_bad_mul.stock.proof
p1=$?

# --- PHASE 2: rebuild proving key from the patched PIL --------------------
banner "PHASE 2/3  rebuild proving key from patched arith.pil"
if [ -d "$patched_pk" ]; then
    echo "Patched proving key already present at ${patched_pk} -- skipping rebuild."
else
    echo "Recompiling zisk.pilout and regenerating the proving key."
    echo "This is slow: PIL compile + setup generation + GPU constant trees."
    bash /workspace/zisk/rebuild-patched-pk.sh \
        || { echo "ERROR: patched proving key rebuild failed" >&2; exit 1; }
fi

# --- PHASE 3a: patched circuit, malicious witness -------------------------
banner "PHASE 3a/3  patched circuit + MALICIOUS witness"
echo "The patched Arith AIR forces np = na XOR nb."
echo "Expectation (the fix): the malicious row violates it -> proof REJECTED."
echo
prove "$patched_pk" 1 /tmp/arith_bad_mul.patched.proof
p3a=$?

# --- PHASE 3b: patched circuit, honest witness ----------------------------
banner "PHASE 3b/3  patched circuit + HONEST witness"
echo "Same ELF, no ZISK_REPRO_BAD_ARITH_MUL -> the real MUL(-1,1) = -1 trace."
echo "Expectation: the fix does not break correctness -> proof VERIFIES."
echo
prove "$patched_pk" 0 /tmp/arith_honest_mul.patched.proof
p3b=$?

# --- summary --------------------------------------------------------------
banner "SUMMARY"
verdict() { [ "$1" -eq 0 ] && echo "VERIFIED" || echo "REJECTED (rc=$1)"; }
echo "Phase 1   stock   circuit + malicious witness : $(verdict $p1)"
echo "Phase 3a  patched circuit + malicious witness : $(verdict $p3a)"
echo "Phase 3b  patched circuit + honest witness    : $(verdict $p3b)"
echo
if [ "$p1" -eq 0 ] && [ "$p3a" -ne 0 ] && [ "$p3b" -eq 0 ]; then
    echo "RESULT: as expected -- the stock circuit proves MUL(-1,1) = 1, the"
    echo "        patched circuit rejects that malicious witness, and the honest"
    echo "        multiplication still proves and verifies."
    exit 0
else
    echo "RESULT: unexpected outcome -- see the phase results above."
    exit 1
fi
