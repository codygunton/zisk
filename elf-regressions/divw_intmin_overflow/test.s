# divw_intmin_overflow: DIVW INT_MIN_32 / -1.
# Expected RV64 DIVW result: 0xFFFFFFFF80000000.
# Current op_div_w commits:  0x0000000080000000, so the operation-bus lookup fails.
# See README.md for the circuit analysis.

.section .text.init
.global _start
_start:
    lui  a0, 0x80000    # low32(a0) = 0x80000000 (INT_MIN_32)
    li   a1, -1         # low32(a1) = 0xFFFFFFFF (-1)
    divw a2, a0, a1     # signed 32-bit overflow: should wrap then sign-extend
    li   a7, 93
    ecall
1:  j 1b
