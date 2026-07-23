# MemAlign narrow-load value-lane soundness repro (see README.md).
#
# Store 0 to an 8-aligned address in free RAM, then `lwu` the low word. Honest
# rd = 0x00000000. Under ZISK_REPRO_BAD_MEM_ALIGN_LWU the two injections forge
# the load's high 32 bits, so the trace carries rd = 0x0000_0001_0000_0000 -- a
# value never present in memory -- and the stock circuit accepts it.
# RVC disabled so the interpreter sees only complete 32-bit instructions.

.section .text
.globl _start
.option norvc

_start:
    li    t0, 0xa0030000     # 8-aligned free RAM (AVAILABLE_MEM_ADDR)
    slli  t0, t0, 32
    srli  t0, t0, 32         # zero-extend: t0 = 0x0000_0000_a003_0000
    sd    x0, 0(t0)          # low word = 0x00000000 (honest lwu result)
    lwu   t1, 0(t0)          # honest t1 = 0 ; forged t1 = 0x0000_0001_0000_0000
    li    a0, 0
    li    a7, 93             # exit
    ecall
