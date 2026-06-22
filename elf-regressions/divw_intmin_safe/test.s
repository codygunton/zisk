# divw_intmin_safe: same INT_MIN_32 dividend without the / -1 overflow case.
# DIVW INT_MIN_32 / 1 = INT_MIN_32, sign-extended to 0xFFFFFFFF80000000.
# The current emulator and Arith state machine agree for this case.

.section .text.init
.global _start
_start:
    lui  a0, 0x80000    # low32(a0) = 0x80000000 (INT_MIN_32)
    li   a1, 1
    divw a2, a0, a1
    li   a7, 93
    ecall
1:  j 1b
