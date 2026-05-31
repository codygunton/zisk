#!/usr/bin/env bash
#
# Arith DIV/REM malicious-witness repro + diagnostic patched-key demonstration.
#
#   Phase 1  stock circuit   + malicious witness -> proof VERIFIES  (the bug)
#   Phase 2  rebuild the proving key from the patched arith.pil
#   Phase 3a patched circuit + malicious witness -> constraints REJECT
#   Phase 3b patched circuit + honest witness    -> constraints PASS
#
# Phase 1 runs `prove --verify-proofs --gpu`: it generates a full STARK proof
# and runs the stock verifier on it. Phase 3 runs `verify-constraints --gpu`,
# which evaluates every AIR's local constraints and all global constraints
# directly on the trace. The malicious witness is injected under
# ZISK_REPRO_BAD_ARITH_DIV_REM=1.
#
# The bad witness claims:
#   DIV(-256, 256) = 0
#   REM(-256, 256) = -256
# It preserves dividend = divisor * quotient + remainder, but violates
# |remainder| < |divisor|. The vulnerable path is the Binary OP_LT_ABS_NP
# lookup used for a negative remainder and non-negative divisor.
#
set -uo pipefail

elf=/tmp/zisk-div-rem-edge/arith_bad_div_rem.elf
cargo_zisk=/workspace/zisk/target-docker/release/cargo-zisk
stock_pk="${HOME}/.zisk/provingKey"
patched_pk="${HOME}/.zisk/provingKey-patched-div-rem"

banner() {
    printf '\n================================================================\n'
    printf '  %s\n' "$1"
    printf '================================================================\n'
}

# prove <proving-key> <malicious:0|1> <output-proof>
prove() {
    local pk=$1 malicious=$2 out=$3
    local env_args=()
    [ "$malicious" = "1" ] && env_args=(ZISK_REPRO_BAD_ARITH_DIV_REM=1)
    env "${env_args[@]}" RUST_LOG=info \
        "$cargo_zisk" prove --elf "$elf" --emulator -k "$pk" --gpu \
        --verify-proofs -o "$out"
}

# verify_constraints <proving-key> <malicious:0|1>
verify_constraints() {
    local pk=$1 malicious=$2
    local env_args=()
    [ "$malicious" = "1" ] && env_args=(ZISK_REPRO_BAD_ARITH_DIV_REM=1)
    env "${env_args[@]}" RUST_LOG=info \
        "$cargo_zisk" verify-constraints --elf "$elf" --emulator -k "$pk" --gpu
}

if [ ! -d "$stock_pk" ]; then
    banner "Installing stock v0.18.0 proving key via ziskup"
    ziskup --version 0.18.0 --provingkey --gpu -y \
        || { echo "ERROR: ziskup failed to install the proving key" >&2; exit 1; }
fi

banner "PHASE 1/3  stock circuit + MALICIOUS witness  (prove --verify-proofs)"
echo "Expectation (the bug): a proof of DIV(-256,256)=0 and REM(-256,256)=-256 is generated and VERIFIES."
echo
prove "$stock_pk" 1 /tmp/arith_bad_div_rem.stock.proof
p1=$?

banner "PHASE 2/3  rebuild proving key from patched arith.pil"
if [ -d "$patched_pk" ]; then
    echo "Patched proving key already present at ${patched_pk} -- skipping rebuild."
else
    echo "Recompiling zisk.pilout and regenerating the basic setup."
    bash /workspace/zisk/rebuild-patched-pk.sh \
        || { echo "ERROR: patched proving key rebuild failed" >&2; exit 1; }
fi

banner "PHASE 3a/3  patched circuit + MALICIOUS witness  (verify-constraints)"
echo "Expectation: the diagnostic patched Arith AIR rejects the malicious remainder-bound witness."
echo
verify_constraints "$patched_pk" 1
p3a=$?

banner "PHASE 3b/3  patched circuit + HONEST witness  (verify-constraints)"
echo "Same ELF, no ZISK_REPRO_BAD_ARITH_DIV_REM -> the real DIV=-1 and REM=0 trace."
echo "Expectation: the honest trace still satisfies all constraints."
echo
verify_constraints "$patched_pk" 0
p3b=$?

banner "SUMMARY"
proof_verdict() { [ "$1" -eq 0 ] && echo "VERIFIED" || echo "FAILED (rc=$1)"; }
constraint_verdict() { [ "$1" -eq 0 ] && echo "PASSED" || echo "REJECTED (rc=$1)"; }
echo "Phase 1   stock   circuit + malicious  (prove)             : $(proof_verdict $p1)"
echo "Phase 3a  patched circuit + malicious  (verify-constraints): $(constraint_verdict $p3a)"
echo "Phase 3b  patched circuit + honest     (verify-constraints): $(constraint_verdict $p3b)"
echo
if [ "$p1" -eq 0 ] && [ "$p3a" -ne 0 ] && [ "$p3b" -eq 0 ]; then
    echo "RESULT: as expected -- the stock circuit proves the bad DIV/REM witness,"
    echo "        the patched circuit rejects it, and the honest trace still passes."
    exit 0
else
    echo "RESULT: unexpected outcome -- see the phase results above."
    exit 1
fi
