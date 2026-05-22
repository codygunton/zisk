.section .text.init
.global _start

_start:
    li t0, -1
    li t1, 1
    mulhsu t2, t0, t1

    li a0, 0
    li a7, 93
    ecall

1:
    j 1b
