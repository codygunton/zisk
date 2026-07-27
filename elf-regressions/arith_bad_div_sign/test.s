# Arith signed-DIV quotient-sign malicious-witness repro (see README.md).
#
# DIV(1, -1). Honest quotient is -1. Under ZISK_REPRO_BAD_ARITH_DIV_SIGN the two
# injections forge the quotient sign, so the trace carries t2 = +1 -- a division
# result that is off by sign -- and the stock circuit accepts it.
# RVC disabled so the interpreter sees only complete 32-bit instructions.

.section .text
.globl _start
.option norvc

_start:
    li    t0, 1
    li    t1, -1
    div   t2, t0, t1        # honest t2 = -1 ; forged t2 = +1

    li    a0, 0
    li    a7, 93            # exit
    ecall
