  Working ELF (pp-keccakf.elf):
  - .rodata VirtAddr = 0x8003bbd0, PhysAddr = 0x8003bbd0 (SAME)
  - Everything in ROM region uses same VirtAddr and PhysAddr

  zksync-os ELF:
  - .rodata VirtAddr = 0xa1000000, PhysAddr = 0x800c02a0 (DIFFERENT)
  - VirtAddr is in RAM, PhysAddr is in ROM

  zksync-os uses a split VMA/LMA layout where rodata is stored in ROM (PhysAddr) but accessed at a different RAM address (VirtAddr) at runtime.
  This requires a bootloader to copy data from ROM to RAM - which Zisk doesn't do.u
