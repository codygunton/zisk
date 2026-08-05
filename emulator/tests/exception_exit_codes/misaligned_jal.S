.option norvc
.section .text.init
.globl rvtest_entry_point
rvtest_entry_point:
  jal x0, . + 6
  nop
