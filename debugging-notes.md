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



> Can you tell from this why we need to incrase the ROM size so much?


Memory sections:
  0 .text.dummy   00000000  0000000080000000  0000000080000000  00001000  2**0
  1 .text         000c0288  0000000080000000  0000000080000000  00001000  2**12
  2 .stack        01000000  00000000a0000000  00000000a0000000  000c1288  2**12
  3 .rodata       00032af0  00000000800c02a0  00000000800c02a0  000c12a0  2**5
  4 .note.gnu.build-id 00000024  00000000800f2d90  00000000800f2d90  000f3d90  2**2
  5 .data         00000000  00000000a1000000  00000000a1000000  00201000  2**12
  6 .bss          00000220  00000000a1000000  00000000a1000000  00200000  2**12
  7 .heap         1b800000  00000000a1200000  00000000a1200000  00200000  2**21
  8 .eh_frame     0000002c  0000000000000000  0000000000000000  00201000  2**3
  9 .comment      000000e7  0000000000000000  0000000000000000  0020102c  2**0

● Yes! Look at the .heap section:

  .heap  1b800000  (461 MB)

  The .heap is 0x1b800000 = 461 MB and it's SHT_NOBITS (like BSS), so Zisk generates instructions to zero-initialize it:

  461 MB / 8 bytes per store = ~57.6 million instructions

  That's where ~60M instructions come from - almost entirely from zeroing the 461MB heap.




● Airbender doesn't have this problem because it's architecturally different:

  Airbender: Direct RISC-V simulator
  - Loads the binary into memory
  - Allocates heap/stack as memory regions
  - Memory starts as zero (or undefined) - no explicit initialization needed
  - Just executes RISC-V instructions directly

  Zisk: RISC-V → Zisk transpiler
  - Converts RISC-V ELF to Zisk ROM
  - For NOBITS sections (heap, stack, bss) with WRITE flag, it generates explicit copyb instructions to zero-initialize every 8 bytes
  - This is what causes the instruction explosion

  The issue is Zisk's elf_extraction.rs treating NOBITS sections as data that needs initialization code, rather than just reserving address space.

  Potential fixes in Zisk (not your code):
  1. Skip NOBITS sections entirely (they're uninitialized by definition)
  2. Or only initialize small NOBITS sections (e.g., < 1MB)

  Your workaround: Strip .heap and .stack from the ELF before passing to Zisk, which you're already doing.


● Good question. Yes, it works because:

  1. Stripping sections from ELF only removes metadata - it doesn't change the compiled code or memory addresses


  ● Good question. Yes, it works because:

  1. Stripping sections from ELF only removes metadata - it doesn't change the compiled code or memory addresses
  2. The code still references those addresses - the linker script baked in:
    - Stack at 0xa0000000
    - Heap at higher addresses in RAM
  3. Zisk's emulator has RAM mapped at 0xa0000000 - when the program accesses those addresses, Zisk handles them
  4. We're only skipping Zisk's ROM zero-initialization - the memory regions still exist and are accessible
  5. In practice:
    - Stack: written before read (push then pop), doesn't need pre-zeroing
    - Heap: allocator (talc) manages it, initializes as needed
    - BSS: we're NOT stripping .bss (only 544 bytes), so static variables are initialized

  The only risk would be if code assumes uninitialized heap memory is zero. But that's generally not a safe assumption anyway.
  2. The code still references those addresses - the linker script baked in:
    - Stack at 0xa0000000
    - Heap at higher addresses in RAM
  3. Zisk's emulator has RAM mapped at 0xa0000000 - when the program accesses those addresses, Zisk handles them
  4. We're only skipping Zisk's ROM zero-initialization - the memory regions still exist and are accessible
  5. In practice:
    - Stack: written before read (push then pop), doesn't need pre-zeroing
    - Heap: allocator (talc) manages it, initializes as needed
    - BSS: we're NOT stripping .bss (only 544 bytes), so static variables are initialized

  The only risk would be if code assumes uninitialized heap memory is zero. But that's generally not a safe assumption anyway.



 … +33 lines (ctrl+o to expand)

● We've made significant progress. The oracle is working - 6 queries complete successfully and execution runs 11% further (step 225331 vs 202696). The crash seems to be after the oracle data processing.

  Let me summarize what we discovered and implemented:

  Summary of Findings

  Key Discovery: Endianness Mismatch

  The inputs.bin file is big-endian, not little-endian as initially assumed:
  - First u32 in file: 0x00000830 = 2096 (response_len for BlockMetadata)

  Key Discovery: 64-bit Value Splitting

  The oracle_provider splits 64-bit usize values into pairs of 32-bit reads:
  - response_len is doubled: 1048 usize → 2096 u32 reads
  - Each 64-bit value is stored as: low 32 bits first, then high 32 bits

  Implementation: Replay64Oracle

  Created a stateful oracle that:
  1. Returns response_len halved (2096 → 1048)
  2. Combines pairs of u32 into u64: ((high << 32) | low)
  3. Tracks state to handle the response_len/data pattern

  Results

  - 32-bit oracle (big-endian): Crash at step 202696
  - 64-bit oracle: Crash at step 225331 (11% further)
  - 6 queries processed successfully before crash

  The remaining crash appears to be in materialize_element in the account cache - likely a data interpretation issue that requires deeper investigation of the zksync-os code.

  Do you want me to continue debugging the remaining crash, or is this enough progress for now?



● This is not scope creep - it's a bug fix for 64-bit targets. Let me explain:

  The Problem

  The csr_read_word() function performs a CSR read instruction that returns a full register-width value (64 bits on RV64, 32 bits on RV32). However, the function was declared

  pub fn csr_read_word() -> u32 {
      let mut output;
      unsafe {
          core::arch::asm!(
              "csrrw {rd}, 0x7c0, x0",
              rd = out(reg) output,  // This is 64 bits on RV64!
              options(...)
          );
      }
      output  // Compiler truncates to u32
  }

  On RV64, the assembly reads 64 bits into output, but the -> u32 return type causes the compiler to generate truncation code:

  csrrw a1, 0x7c0, x0   ; Read 64-bit value
  slli  a1, a1, 0x20    ; Shift left 32
  srli  a1, a1, 0x20    ; Shift right 32 = CLEAR UPPER 32 BITS

  This destroys the upper 32 bits of every oracle value.

  Why This Matters

  The oracle returns 64-bit values (like 0xe9afe5fae99e198a), but due to truncation, the guest only sees 0x00000000e99e198a. When these truncated values are used for hash verification (Bytes32), they don't match the expected hashes, causing the execution to fail.

  The I/O Format Does NOT Change

  The witness file format remains unchanged - it's still big-endian u32 values. The change is purely in how the guest code reads CSR values:

  - Before: CSR read → truncate to 32 bits → return
  - After: CSR read → return full register width

  The Replay64Oracle already handles the u32-to-u64 conversion correctly. The bug was that even after the oracle returned correct 64-bit values, the guest's csr_read_word() function was throwing away the upper 32 bits.

  This Is Standard Practice

  The csr_write_word() function in the same file already takes usize:
  pub fn csr_write_word(word: usize) {  // Already correct!

  The fix makes csr_read_word() consistent with csr_write_word() - both now use the native register width.
