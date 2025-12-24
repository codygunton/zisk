# RISC-V Compiler Optimization Bugs in ZKsyncOS

This document summarizes compiler optimization issues encountered when porting ZKsyncOS from RV32IM (Airbender) to RV64IMAC (ZisK).

## Overview

Multiple locations in the codebase required workarounds for compiler optimization bugs that produced incorrect results on RISC-V targets. The consistent fix is using `volatile` reads/writes to prevent the compiler from reordering, combining, or eliding memory operations.

## Likely Root Cause: LLVM LoopIdiomRecognize + RISC-V Backend

**Build Configuration:**
- `lto = true` (Link-Time Optimization)
- `opt-level = 3`
- `codegen-units = 1`
- Target: `riscv64imac-unknown-none-elf`
- Target features: `+m,+a,+c,-unaligned-scalar-mem,+relax`
- LLVM version: 20.1.5
- rustc: 1.89.0-nightly (2eef47813 2025-05-22)

**Hypothesis:** The bugs are caused by LLVM's **LoopIdiomRecognize pass** transforming byte-copy loops into `memcpy` intrinsics, combined with incorrect code generation for the resulting memory operations on RISC-V 64-bit when unaligned access is disabled (`-unaligned-scalar-mem`).

**Evidence:**
1. All three bugs involve sequential byte operations that LLVM could transform into bulk memory operations
2. The generated binary contains custom `memcpy`/`memset` implementations (visible in disassembly at `0x80000124` and `0x8000002c`)
3. Known LLVM issues exist for [RISC-V memcpy IR generation](https://github.com/llvm/llvm-project/issues/89866) and [store merging with alignment constraints](https://github.com/llvm/llvm-project/issues/56110)
4. Using `volatile` prevents loop-to-memcpy transformation, which is why it consistently fixes the issue

**Why This Affects ZisK (RV64) But Not Airbender (RV32):**
- Different code paths in LLVM's RISC-V backend for 32-bit vs 64-bit targets
- 64-bit targets use 8-byte stores where 32-bit uses 4-byte stores, potentially triggering different optimization paths
- The `memcpy` implementation differs between targets due to register width differences

## Affected Areas

### 1. Input Buffer Copying in ecrecover

**Commit:** `820e339e` ("magical fix (compiler bug??)")

**File:** `basic_system/src/system_functions/ecrecover.rs`

**Problem:** Iterator-based byte copying produced incorrect buffer contents.

```rust
// BROKEN - compiler optimizes incorrectly
for (dst, src) in buffer.iter_mut().zip(src.iter()) {
    *dst = *src;
}

// FIXED - volatile writes force correct behavior
for byte in src.iter() {
    unsafe {
        core::ptr::write_volatile(&mut buffer[idx], *byte);
    }
    idx += 1;
}
```

### 2. Field Element Serialization in secp256k1

**Commit:** `28e3e821` ("push through to first tx passing; 39 don't revert")

**Files:**
- `crypto/src/secp256k1/field/field_10x26.rs`
- `crypto/src/secp256k1/field/field_5x52.rs`

**Problem:** Direct byte assignment in `to_bytes()` produced incorrect serialization.

```rust
// BROKEN - compiler optimizes incorrectly
r[0] = (self.0[9] >> 14) as u8;
r[1] = (self.0[9] >> 6) as u8;
// ...

// FIXED - volatile writes force correct behavior
unsafe {
    core::ptr::write_volatile(&mut r[0], (self.0[9] >> 14) as u8);
    core::ptr::write_volatile(&mut r[1], (self.0[9] >> 6) as u8);
    // ...
}
```

### 3. Slice Passing to Keccak256 Hasher (EIP-2718)

**Commit:** `5470c8ab` ("64 don't revert")

**File:** `basic_bootloader/src/bootloader/transaction/rlp_encoded/eip_2718_tx_envelope.rs`

**Problem:** Passing a slice reference from parsed RLP data directly to `hasher.update()` produced incorrect hashes on 64-bit ZisK.

```rust
// BROKEN - direct slice update fails on ZisK
hasher.update(inner_slice);

// FIXED - byte-by-byte update works correctly
for i in 0..inner_slice.len() {
    hasher.update(&[inner_slice[i]]);
}
```

**Investigation findings:**
- `hash_local` (copy to stack buffer, single update) = correct
- `hash_bytewise` (byte-by-byte updates) = correct
- `hash_direct` (direct `update(inner_slice)`) = wrong on ZisK

The issue appears to be in how the `block-buffer` crate's `split_blocks()` handles slice pointers, or a ZisK emulator issue with memory access patterns for slices pointing to input buffer regions vs stack memory.

### 4. Legacy Transaction Hashing

**File:** `basic_bootloader/src/bootloader/transaction/rlp_encoded/transaction_types/legacy_tx.rs`

**Problem:** Same issue as EIP-2718 - passing `inner_slice` directly to `hasher.update()` produced incorrect sig_hash for legacy transactions (e.g., TX#47).

```rust
// BROKEN - direct slice update fails on ZisK
hasher.update(inner_slice);

// FIXED - volatile copy + byte-by-byte updates
let mut local_buf = [0u8; 2048];
for i in 0..inner_slice.len() {
    unsafe {
        let byte = core::ptr::read_volatile(&inner_slice[i]);
        core::ptr::write_volatile(&mut local_buf[i], byte);
    }
}
let local_slice = &local_buf[..inner_slice.len()];
for i in 0..local_slice.len() {
    hasher.update(&[local_slice[i]]);
}
```

**Key insight:** Both volatile reads AND writes are required, plus byte-by-byte hasher updates. Just copying to a local buffer and passing the slice wasn't sufficient.

### 5. EVM Memory Copy Operations (CALLDATACOPY, CODECOPY, etc.)

**File:** `zk_ee/src/utils/convenience/memcopy.rs`

**Problem:** The `copy_and_zeropad_nonoverlapping` function used `copy_from_slice` which corrupted data when the source was from EVM heap/calldata memory regions.

```rust
// BROKEN - copy_from_slice corrupts data on ZisK
dst[..to_copy].copy_from_slice(&src[..to_copy]);

// FIXED - volatile reads and writes
for i in 0..to_copy {
    unsafe {
        let byte = core::ptr::read_volatile(&src[i]);
        core::ptr::write_volatile(&mut dst[i], byte);
    }
}
```

**Impact:** This function is used by CALLDATACOPY, CODECOPY, RETURNDATACOPY, and EXTCODECOPY EVM opcodes. Fixing it resolved several transactions where ecrecover input data was being corrupted.

### 6. Keccak256 MiniDigest Implementation

**File:** `crypto/src/sha3/mod.rs`

**Problem:** The `MiniDigest::digest()` and `MiniDigest::update()` functions passed slice references directly to the underlying Keccak256 hasher, causing incorrect hashes when the input slice came from certain memory regions.

```rust
// BROKEN - direct slice passing corrupts hash
<Keccak256 as Digest>::update(&mut hasher, input);

// FIXED - volatile reads + byte-by-byte updates
let slice = input.as_ref();
for i in 0..slice.len() {
    let byte = unsafe { core::ptr::read_volatile(&slice[i]) };
    <Keccak256 as Digest>::update(&mut hasher, &[byte]);
}
```

**Impact:** This affected address computation from ecrecover output (pk_bytes → Keccak256 → address). Even when pk_bytes were identical between platforms, the resulting addresses differed due to this bug.

## Root Cause Hypotheses

1. **LLVM backend bug for RISC-V:** The optimizer may be making incorrect assumptions about memory ordering or aliasing.

2. **Undefined behavior exposure:** Code that works on x86/ARM may rely on undefined behavior that manifests differently on RISC-V.

3. **Memory region handling:** The ZisK emulator may handle memory reads differently depending on whether the pointer targets input buffer memory vs stack memory.

## Recommendations

### Current Workarounds (What We're Doing)

1. **Use volatile for cross-boundary data:** When copying data between memory regions (especially from input buffers to local buffers), use `core::ptr::read_volatile` and `core::ptr::write_volatile`.

2. **Prefer byte-by-byte operations for hashing:** When hashing data from parsed/external sources, consider byte-by-byte updates rather than passing slice references directly.

3. **Test on both targets:** Always verify correctness on both Airbender (RV32) and ZisK (RV64) as bugs may manifest differently.

### Potential Proper Fixes (Investigated)

#### Tested and FAILED:

1. **Disable memcpy/memmove idiom recognition:** Tested with `-C llvm-args=--disable-memcpy-idiom -C llvm-args=--disable-memmove-idiom`. These flags prevent LLVM from transforming loops into memcpy/memmove intrinsics. **Result: No effect.** The `hash_direct` trace still shows incorrect results, proving the bug is NOT caused by loop-to-memcpy transformations.

2. **Disable LTO:** Tested `lto = false`. **Result: Linker error** - `rust-lld: error: relocation R_RISCV_32_PCREL out of range`. The binary is too large for 32-bit PC-relative relocations without LTO optimization reducing code size.

3. **Reduce optimization level:** Previously tested with `-O0`. **Result: Same bugs persisted**, ruling out pure optimization level issues.

#### Not Yet Tested:

1. **Use `#[inline(never)]` barriers:** Add `#[inline(never)]` to functions that process slices to prevent LTO from inlining and exposing the bug.

2. **Report upstream:** File a bug report with LLVM/rustc with a minimal reproducer. The pattern is:
   - RISC-V 64-bit target with `-unaligned-scalar-mem`
   - LTO enabled
   - Passing slice references to functions that process data in blocks
   - The data is corrupted when accessed through the original slice pointer

### Refined Hypothesis

The bug is likely NOT related to memcpy/memmove transformations. Given that:
- Disabling memcpy/memmove idiom detection had no effect
- The issue only manifests with slices pointing to certain memory regions (input buffers) vs others (stack)
- Copying data to a local stack buffer and then processing it works correctly

The most likely root causes are:
1. **ZisK emulator memory handling:** The emulator may handle memory reads differently depending on address ranges (input buffer regions vs normal RAM)
2. **Pointer aliasing issues:** LLVM may be making incorrect aliasing assumptions about slice pointers in 64-bit mode
3. **Memory layout/alignment differences:** 64-bit targets may have different alignment requirements that trigger incorrect code generation

### Why Volatile Works

The `volatile` keyword tells LLVM:
- Do not reorder this memory operation
- Do not eliminate this memory operation
- Do not combine this with adjacent operations
- Do not transform this loop into memcpy

This prevents all the transformations that trigger the bug.

## Progress Tracking

| Commit | Description | Non-Reverting TXs |
|--------|-------------|-------------------|
| `820e339e` | Volatile writes in ecrecover input copy | (initial fix) |
| `28e3e821` | Volatile writes in field element serialization | 39 |
| `5470c8ab` | Byte-by-byte hashing for EIP-2718 transactions | 64 |
| (session) | Volatile copy + byte-by-byte hashing for legacy transactions | 95 |
| (session) | Volatile reads/writes in copy_and_zeropad_nonoverlapping | 99 |
| (session) | Volatile reads + byte-by-byte updates in Keccak256 MiniDigest | 103 |
| (session) | Volatile reads/writes in Blake2s256 (delegated + naive) | 103 (state changed) |

**Current status:**
- ZisK: 103/126 transactions pass (23 revert)
- Airbender: 104/126 transactions pass (22 revert)
- **Only TX#76 diverges** between platforms (reverts on ZisK but not Airbender)

The remaining TX#76 divergence occurs during a 1inch/Uniswap V3 swap:
- Both platforms execute identically until getting pool fee (0xbb8 = 3000)
- On ZisK: 36 instructions executed, then `EvmError(Revert)` with error `b2c02722`
- On Airbender: 72 instructions executed, external call made successfully
- The ecrecover operations during TX#76 produce correct, matching results
- **Oracle queries are identical** between both platforms for TX#76
- Investigation ongoing to find the remaining data corruption source

### 7. Blake2s256 Implementation (delegated_extended and naive)

**Files:**
- `crypto/src/blake2s/delegated_extended.rs` - `spec_memcopy` function
- `crypto/src/blake2s/naive.rs` - MiniDigest impl

**Problem:** Same pattern as other fixes - `copy_from_slice` or direct slice passing corrupts data.

```rust
// delegated_extended.rs - spec_memcopy
// BROKEN
dst.copy_from_slice(src);

// FIXED
for i in 0..src.len() {
    let byte = core::ptr::read_volatile(&src[i]);
    core::ptr::write_volatile(&mut dst[i], byte);
}

// naive.rs - MiniDigest::update
// BROKEN
self.inner.update(input);

// FIXED
let slice = input.as_ref();
for i in 0..slice.len() {
    let byte = unsafe { core::ptr::read_volatile(&slice[i]) };
    self.inner.update(&[byte]);
}
```

**Note:** Blake2s256 is used for flat storage key derivation. The fix changed the state computation but did not resolve TX#76.
