.section .text
.globl _start
.option norvc

_start:
    li t0, -256
    li t1, 256
    div t2, t0, t1
    rem t3, t0, t1

    li a0, 0
    li a7, 93
    ecall
