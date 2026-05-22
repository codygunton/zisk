#!/usr/bin/env bash
#
# Arith MUL malicious-witness repro driver.
#
# Demonstrates that stock ZisK accepts witnesses claiming bad signed-MUL-family
# results (`MUL(-1, 1) = 1`, `MULH(-1, 1) = 0`, and
# `MULHSU(-1, 1) = 0`) through two independent checks on each malicious
# witness:
#
#   Step 1  verify-constraints    -- quick: checks AIR + global constraints,
#                                    produces no proof
#   Step 2  prove --verify-proofs -- full: generates a STARK proof and runs
#                                    the stock verifier on that proof
#
# Both steps run with ZISK_REPRO_BAD_ARITH_MUL=1, i.e. on the malicious
# witness, and with --gpu. ZISK_REPRO_BAD_ARITH_MUL_KIND selects the opcode.
# No circuit, constraint, or verifier code is modified.
#
set -uo pipefail

cargo_zisk=./target-docker/release/cargo-zisk
pk="$HOME/.zisk/provingKey"

banner() {
    printf '\n'
    printf '================================================================\n'
    printf '  %s\n' "$1"
    printf '================================================================\n'
}

# --- Proving key ----------------------------------------------------------
if [ ! -d "$pk" ]; then
    banner "Installing stock v0.18.0 proving key via ziskup"
    ziskup --version 0.18.0 --provingkey --gpu -y \
        || { echo "ERROR: ziskup failed to install the proving key" >&2; exit 1; }
else
    echo "Proving key already present at $pk -- skipping ziskup."
fi

run_case() {
    local kind="$1"
    local elf="$2"
    local claim="$3"
    local proof="/tmp/arith_bad_${kind,,}.proof"

    banner "${kind}: STEP 1/2  verify-constraints on the MALICIOUS witness"
    echo "Builds the execution trace and checks every AIR and global constraint."
    echo "Claim under test: ${claim}"
    echo
    ZISK_REPRO_BAD_ARITH_MUL=1 ZISK_REPRO_BAD_ARITH_MUL_KIND="$kind" RUST_LOG=info \
        "$cargo_zisk" verify-constraints --elf "$elf" --emulator -k "$pk" --gpu
    local step1_rc=$?

    banner "${kind}: STEP 2/2  prove --verify-proofs on the MALICIOUS witness"
    echo "Generates a STARK proof, then runs the stock verifier on that proof."
    echo "Claim under test: ${claim}"
    echo
    ZISK_REPRO_BAD_ARITH_MUL=1 ZISK_REPRO_BAD_ARITH_MUL_KIND="$kind" RUST_LOG=info \
        "$cargo_zisk" prove --elf "$elf" --emulator -k "$pk" --gpu \
        --verify-proofs -o "$proof"
    local step2_rc=$?

    banner "${kind}: SUMMARY"
    if [ "$step1_rc" -eq 0 ]; then
        echo "Step 1  verify-constraints    : ACCEPTED  -- malicious trace passed all constraints"
    else
        echo "Step 1  verify-constraints    : REJECTED  -- constraints caught the malicious witness (rc=$step1_rc)"
    fi
    if [ "$step2_rc" -eq 0 ]; then
        echo "Step 2  prove --verify-proofs  : VERIFIED  -- malicious proof was generated and verified"
    else
        echo "Step 2  prove --verify-proofs  : FAILED    -- proof generation or verification did not succeed (rc=$step2_rc)"
    fi
    echo
    if [ "$step1_rc" -eq 0 ] && [ "$step2_rc" -eq 0 ]; then
        echo "RESULT ${kind}: BUG REPRODUCED -- stock ZisK accepted and proved ${claim}."
        return 0
    else
        echo "RESULT ${kind}: bug did NOT reproduce -- at least one check rejected the malicious witness."
        return 1
    fi
}

overall=0
run_case "MUL" "/tmp/zisk-mul-edge/arith_bad_mul.elf" "MUL(-1, 1) = 1" || overall=1
run_case "MULH" "/tmp/zisk-mul-edge/arith_bad_mulh.elf" "MULH(-1, 1) = 0" || overall=1
run_case "MULHSU" "/tmp/zisk-mul-edge/arith_bad_mulhsu.elf" "MULHSU(-1, 1) = 0" || overall=1

banner "FINAL SUMMARY"
if [ "$overall" -eq 0 ]; then
    echo "RESULT: ALL BUGS REPRODUCED -- stock ZisK accepted and verified all malicious signed-MUL witnesses."
else
    echo "RESULT: PARTIAL/NO REPRO -- at least one malicious witness was rejected."
fi
exit "$overall"
