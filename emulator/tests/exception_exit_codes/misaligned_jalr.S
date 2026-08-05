.option norvc
.section .text.init
.globl rvtest_entry_point
rvtest_entry_point:
  auipc t0, 0
  addi t0, t0, 2
  jalr x0, 0(t0)
