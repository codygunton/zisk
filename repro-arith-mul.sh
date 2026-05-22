#!/usr/bin/env bash
#
# Arith MUL malicious-witness repro driver.
#
# Demonstrates that stock ZisK accepts a witness claiming MUL(-1, 1) = 1,
# through two independent checks on the SAME malicious witness:
#
#   Step 1  verify-constraints    -- quick: checks AIR + global constraints,
#                                    produces no proof
#   Step 2  prove --verify-proofs -- full: generates a STARK proof and runs
#                                    the stock verifier on that proof
#
# Both steps run with ZISK_REPRO_BAD_ARITH_MUL=1, i.e. on the malicious
# witness, and with --gpu. No circuit, constraint, or verifier code is
# modified.
#
set -uo pipefail

elf=/tmp/zisk-mul-edge/arith_bad_mul.elf
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

# --- Step 1: quick constraint check ---------------------------------------
banner "STEP 1/2  verify-constraints on the MALICIOUS witness (quick check)"
echo "Builds the execution trace and checks every AIR and global constraint."
echo "No proof is produced. If stock ZisK were sound this would be REJECTED."
echo
ZISK_REPRO_BAD_ARITH_MUL=1 RUST_LOG=info \
    "$cargo_zisk" verify-constraints --elf "$elf" --emulator -k "$pk" --gpu
step1_rc=$?

# --- Step 2: full proof + verification ------------------------------------
banner "STEP 2/2  prove --verify-proofs on the MALICIOUS witness (full proof)"
echo "Generates a STARK proof, then runs the stock verifier on that proof."
echo "If stock ZisK were sound, proving or verification would FAIL."
echo
ZISK_REPRO_BAD_ARITH_MUL=1 RUST_LOG=info \
    "$cargo_zisk" prove --elf "$elf" --emulator -k "$pk" --gpu \
    --verify-proofs -o /tmp/arith_bad_mul.proof
step2_rc=$?

# --- Summary --------------------------------------------------------------
banner "SUMMARY"
if [ "$step1_rc" -eq 0 ]; then
    echo "Step 1  verify-constraints    : ACCEPTED  -- malicious trace passed all constraints"
else
    echo "Step 1  verify-constraints    : REJECTED  -- constraints caught the malicious witness (rc=$step1_rc)"
fi
if [ "$step2_rc" -eq 0 ]; then
    echo "Step 2  prove --verify-proofs  : VERIFIED  -- a proof of MUL(-1,1)=1 was generated and verified"
else
    echo "Step 2  prove --verify-proofs  : FAILED    -- proof generation or verification did not succeed (rc=$step2_rc)"
fi
echo
if [ "$step1_rc" -eq 0 ] && [ "$step2_rc" -eq 0 ]; then
    echo "RESULT: BUG REPRODUCED -- stock ZisK accepted and proved MUL(-1, 1) = 1."
    exit 0
else
    echo "RESULT: bug did NOT reproduce -- at least one check rejected the malicious witness."
    exit 1
fi
