  Working ELF (pp-keccakf.elf):
  - .rodata VirtAddr = 0x8003bbd0, PhysAddr = 0x8003bbd0 (SAME)
  - Everything in ROM region uses same VirtAddr and PhysAddr

  zksync-os ELF:
  - .rodata VirtAddr = 0xa1000000, PhysAddr = 0x800c02a0 (DIFFERENT)
  - VirtAddr is in RAM, PhysAddr is in ROM

  zksync-os uses a split VMA/LMA layout where rodata is stored in ROM (PhysAddr) but accessed at a different RAM address (VirtAddr) at runtime.
  This requires a bootloader to copy data from ROM to RAM - which Zisk doesn't do.u


# Note: using 32-bit witnesses
> Should a rv64im compilation of the guest be able to execute a witness that's all u32s I mean

● Yes, it should work. The witness is logically a stream of u32 values regardless of XLEN.

  The issue is the CSR read/write functions. On RV64, a CSR read returns 64 bits (XLEN), but the oracle protocol deals in 32-bit words. Looking at
  the code we saw earlier:

  // riscv_common crate
  fn csr_read_word() -> u32  // Returns u32, not usize

  And in zksync-os:

  fn csr_read_impl() -> usize {
      csr_read_word() as usize  // Cast u32 → usize (zero-extends on RV64)
  }

  So even on RV64:
  - The oracle still produces/consumes u32 values
  - The CSR instruction returns 64 bits, but only the low 32 bits matter
  - The witness format stays Vec<u32>

  The witness doesn't need to change. The only concern is whether riscv_common's inline assembly works correctly on RV64 - it should, since CSR
  operations are XLEN-agnostic and the function signature forces truncation to u32.


● So INPUT_ADDR = 0x90000000. The inputs are placed there for standard ZisK programs. But zksync-os doesn't read from INPUT_ADDR - it uses CSR 0x7c0
   for oracle queries!

  The problem is that zksync-os uses a completely different I/O mechanism than standard ZisK programs. ZisK's standard input system places data at
  0x90000000, but zksync-os uses CSR-based oracle queries at 0x7c0.

  Looking back at the disassembly at 0x800579ac-0x800579c8:
  1. It writes to CSR 0x7c0 (the oracle query type 0x40070000)
  2. It writes 0 to CSR 0x7c0 (additional query data)
  3. It reads from CSR 0x7c0 (expects the response)
  4. If response is 0, it errors out

  The inputs.bin file we provide contains witness data, but there's no code path in ziskemu to interpret CSR 0x7c0 reads as oracle responses from
  that data.

  This is the root cause: ziskemu doesn't have the oracle protocol implementation that zksync-os expects. The witness library (libzisk_witness.so)
  is supposed to provide this, but we couldn't run it due to missing proving key setup.

  Let me summarize the findings:

  Summary

  Root Cause Identified: The crash at step 183021 occurs because:

  1. zksync-os uses CSR 0x7c0 for oracle I/O - it writes query types (like 0x40070000 = BLOCK_METADATA_QUERY_ID) to CSR 0x7c0 and expects responses
  back
  2. ziskemu translates CSR to memory operations - CSR 0x7c0 becomes memory at 0xa0008000 + (0x7c0 * 8) = 0xa000c000
  3. No oracle handler exists - The memory location is initialized to 0, so when zksync-os reads the oracle response, it gets 0
  4. First oracle query fails - At PC 0x800579bc, zksync-os queries BLOCK_METADATA and gets 0, causing it to jump to error path at 0x80060ab0
  5. Error code 30 is loaded and unwrap_failed is called

  The fix required: To run zksync-os on Zisk, you need to use the proper zk-sync-os-run command with the witness library, which implements the
  oracle protocol. However, this requires:
  1. The hex-encoded witness file (not binary) - ✓ We have this now
  2. A properly set up proving key with .consttree file - ✗ Only GPU version exists
  3. The witness library at the expected path or provided via --witness-lib

  Next steps to make this work:
  1. Run the setup command to generate the CPU constant tree: cargo-zisk setup --proving-key provingKey
  2. Or copy the witness library to ~/.zisk/bin/libzisk_witness.so
  3. Then run: cargo-zisk zk-sync-os-run -e <elf> -w <witness> -k <proving-key> -l



● The BIOS is initializing memory - specifically zeroing a massive heap:

  .heap  0x1b800000 (461MB) at 0xa1200000

  It's zeroing 461MB one 8-byte word at a time = ~57 million store operations.

  Current progress:
  - Current PC: 0x0cb50fcc (213M)
  - Max BIOS PC: 0x0e41a9cc (239M)
  - ~89% through BIOS init

  The BIOS alone requires ~60 million Zisk instructions before the actual program even starts. This is a consequence of the 64-bit build with its large memory regions.

  Options:
  1. Wait - it's almost done with BIOS, but then the actual program runs
  2. Reduce heap size in the linker script (zksync-os/zksync_os/src/lds/memory-zisk.x)
  3. Use 32-bit airbender build - much smaller memory footprint

  The 64-bit build is fundamentally very slow for Zisk due to the instruction expansion and large memory initialization.

> /cody:QO is running… where is this zero-initialization defined? in zisk? zksyncos? show me the code

● Search(pattern: "Init Data|zero.*init|bss.*init|heap.*init", path: "core/src", output_mode: "content")
  ⎿  Found 9 lines (ctrl+o to expand)

● Read(core/src/riscv2zisk_context.rs)
  ⎿  Read 150 lines

● Now let me find where this function is called from - specifically what data is being initialized:

● Search(pattern: "add_zisk_init_data", path: "core/src", output_mode: "content")
  ⎿  Found 6 lines (ctrl+o to expand)

● Read(core/src/elf2rom.rs)
  ⎿  Read 40 lines

● Now let me check what sections are being extracted from the ELF:

● Read(core/src/elf_extraction.rs)
  ⎿  Read 50 lines

● The zero-initialization is defined in Zisk (/home/cody/zisk/core/src):

  Where the initialization happens:

  1. ELF Section Extraction (elf_extraction.rs:89-95)

  } else if sh.sh_type == SHT_NOBITS {
      // BSS sections - uninitialized data, should be zero-filled
      // Create a zero-filled vector of the appropriate size
      let size = sh.sh_size as usize;
      let aligned_size = (size + 3) & !3;
      vec![0u8; aligned_size]  // <-- Creates zeros for BSS/.heap sections
  }

  2. ROM Generation (elf2rom.rs:35-37)

  // 2. Add read-write data sections (will be copied to RAM)
  for section in &payload.rw {
      add_zisk_init_data(&mut rom, section.addr, &section.data, true);
  }

  3. Instruction Generation (riscv2zisk_context.rs:1701-1726)

  pub fn add_zisk_init_data(rom: &mut ZiskRom, addr: u64, data: &[u8], force_aligned: bool) {
      // Read 64-bit input data chunks and store them in rom
      let nd = data.len() / 8;
      for i in 0..nd {
          let v = u64::from_le_bytes(data[i * 8..i * 8 + 8].try_into().unwrap());
          // Creates one Zisk instruction per 8 bytes
          zib.verbose(&format!("Init Data {o:08x}: {v:08x}"));
          ...
      }
  }

  The problem:

  Your ELF has a .heap section of 461MB (0x1b800000 bytes) defined as SHT_NOBITS (BSS-like). Zisk creates a zero-filled vector and then generates ~57 million Zisk instructions to store all those zeros in memory.

  The huge heap comes from zksync-os linker script, not Zisk itself. Check zksync-os/zksync_os/src/lds/memory-zisk.x or similar.
