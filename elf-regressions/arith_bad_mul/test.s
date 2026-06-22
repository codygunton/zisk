## Minimal RV64 program that exercises the signed MUL bug.
#
# Computes t2 = mul(t0, t1) with t0 = -1 and t1 = 1. An honest execution
# yields t2 = -1 (= 0xffff_ffff_ffff_ffff). Under ZISK_REPRO_BAD_ARITH_MUL=1
# the env-gated malicious witness generator instead emits t2 = 1.

.section .text.init
.global _start

_start:
    # Inputs.
    li   t0, -1                 # t0 = 0xffff_ffff_ffff_ffff (signed -1)
    li   t1, 1                  # t1 = 1

    # The instruction under test: signed 64-bit multiply, low 64 bits.
    mul  t2, t0, t1

    # Exit via Linux/RISC-V exit syscall (a7 = 93).
    li   a0, 0
    li   a7, 93
    ecall

    # Safety: spin if the syscall ever returns (it shouldn't).
1:  j 1b
