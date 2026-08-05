.option norvc
.section .text.init
.globl rvtest_entry_point
rvtest_entry_point:
  beq x0, x0, . + 6
  nop
