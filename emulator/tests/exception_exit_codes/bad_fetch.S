.option norvc
.section .text.init
.globl rvtest_entry_point
rvtest_entry_point:
  li t0, 0x80000002
  # Keep a real compressed instruction at an address ending in 2 so the ROM has a non-aligned
  # lookup table. The target below is then a missing entry in that table, not an OOB lookup.
  .2byte 0x0001
  .2byte 0x0001
  jalr x0, 0(t0)
