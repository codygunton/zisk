## Minimal RV64 program that exercises the signed MUL bug visibly.
#
# Computes t2 = mul(t0, t1) with t0 = -1 and t1 = 1. An honest execution
# yields t2 = -1 (= 0xffff_ffff_ffff_ffff). Under ZISK_REPRO_BAD_ARITH_MUL=1
# the env-gated malicious witness generator instead emits t2 = 1.
#
# The two halves of t2 are written to ZisK's public-output region at
# OUTPUT_ADDR = 0xa001_0000, which is the same memory-mapped slot that
# `ziskos::set_output(id, value)` writes to from a Rust guest. This makes
# the committed value of t2 part of the proof's public outputs and visible
# in the prove summary -- so a malicious vs honest run is distinguishable
# in the log even though they prove the same program.

.section .text.init
.global _start

_start:
    # Inputs.
    li   t0, -1                 # t0 = 0xffff_ffff_ffff_ffff (signed -1)
    li   t1, 1                  # t1 = 1

    # The instruction under test: signed 64-bit multiply, low 64 bits.
    mul  t2, t0, t1

    # Commit t2 to public outputs.
    # ZisK's OUTPUT_ADDR region holds u32 slots; set_output(id, v) writes
    # v at OUTPUT_ADDR + 4 * id. Here id=0 holds the low half, id=1 the high.
    li   t4, 0xa0010000         # OUTPUT_ADDR
    sw   t2, 0(t4)              # public_output[0] = bits  0..31 of t2
    srli t5, t2, 32
    sw   t5, 4(t4)              # public_output[1] = bits 32..63 of t2

    # Exit via Linux/RISC-V exit syscall (a7 = 93).
    li   a0, 0
    li   a7, 93
    ecall

    # Safety: spin if the syscall ever returns (it shouldn't).
1:  j 1b
