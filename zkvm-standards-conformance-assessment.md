# ZisK + ZKsyncOS ZKVM Standards Conformance Assessment

## Executive Summary

| Standard | Status | Summary |
|----------|--------|---------|
| **RISC-V Target** | **CONFORMANT** | Exceeds minimum (RV64IMAC vs required RV64IM) |
| **IO Interface** | **PARTIAL** | CSR-based custom protocol; Option 2 principles but different API |
| **Crypto Accelerators** | **17/22** | Core EVM precompiles covered; KZG mock-only, some ECDSA gaps |

### Key Findings
- ZisK implements a superset of the required RV64IM instruction set (adds A, C, F, D extensions)
- IO uses a CSR-based oracle protocol that shares zero-copy principles with Option 2 but has incompatible function signatures
- All standard EVM precompiles (0x01-0x11) are implemented except KZG point evaluation (0x0a) which is mock-only
- BLS12-381 (EIP-2537) has full coverage; secp256k1 verification (non-recovery) is missing

### Critical Gaps
1. **KZG Point Evaluation (0x0a)** - Mock implementation only, not production-ready
2. **IO Interface** - Not directly compatible with either Option 1 or Option 2 function signatures
3. **secp256k1_verify** - Not implemented (only ecrecover available)

---

## 1. RISC-V Target Standard Conformance

### Status: CONFORMANT (Exceeds Minimum)

The standard defines a **minimal** target `riscv64im-unknown-none-elf`. ZisK + ZKsyncOS implements **RV64IMAC** which is a superset.

### Conformance Matrix

| Requirement | Status | Evidence |
|-------------|--------|----------|
| **RV64I base** | ✅ Yes | Custom JSON target, objdump confirms rv64 architecture |
| **M extension** | ✅ Yes | RUSTFLAGS: `-C target-feature=+m`, mul/div/rem instructions in `riscv2zisk_context.rs:189-203` |
| **LP64 ABI** | ✅ Yes | `riscv64im-unknown-none-elf` target, soft-float ABI (ELF header confirms) |
| **Little-endian** | ✅ Yes | ELF header: "2's complement, little-endian" |
| **ELF format** | ✅ Yes | ELF64-littleriscv confirmed with file/readelf |
| **Static linking** | ✅ Yes | "statically linked" in file output |
| **Flat memory** | ✅ Yes | ROM: 0x80000000-0x88000000, RAM: 0xa0000000-0xc0000000 (no MMU) |
| **Machine mode** | ✅ Yes | No user/supervisor mode instructions |

### Additional Extensions (Beyond Minimum)

| Extension | Status | Evidence |
|-----------|--------|----------|
| A (Atomic) | ✅ Supported | lr/sc, amo* instructions (`riscv2zisk_context.rs:165-238`) |
| C (Compressed) | ✅ Supported | c.* instructions (`riscv2zisk_context.rs:240-309`) |
| F (Single-Precision FP) | ✅ Supported | Delegated to float library (`riscv2zisk_context.rs:311-342`) |
| D (Double-Precision FP) | ✅ Supported | Delegated to float library (`riscv2zisk_context.rs:344-359`) |

### Build Configuration
- **Target Triple**: `riscv64im-unknown-none-elf.json` (custom JSON, actual binary is riscv64imac)
- **Toolchain**: `nightly-2025-09-04`
- **Memory Layout**: ROM at 0x80000000 (128M), RAM at 0xa0000000 (512M)
- **Linker Scripts**: `memory-zisk.x`, `link-512m.x`

### Recommendation
Consider renaming the JSON target file to `riscv64imac-unknown-none-elf.json` for clarity, as the actual target includes A and C extensions.

---

## 2. IO Interface Standard Conformance

### Status: PARTIAL (Custom CSR-based Protocol)

The implementation uses **CSR 0x7c0 (NON_DETERMINISM_CSR)** with query-response semantics. This shares principles with Option 2 but has incompatible function signatures.

### Option 1 (POSIX) Assessment

| Requirement | Status | Notes |
|-------------|--------|-------|
| `read(fd=0, ...)` | ❌ No | Uses query IDs, not file descriptors |
| `write(fd=1, ...)` | ❌ No | Outputs not written through oracle |
| errno/EBADF errors | ❌ No | Uses Rust Result types |

**Verdict**: Not POSIX-conformant

### Option 2 (Custom) Assessment

| Requirement | Status | Notes |
|-------------|--------|-------|
| `read_input()` signature | ❌ Differs | Uses `raw_query<I>() -> Result<RawIterator>` |
| `write_output()` signature | ❌ Missing | Outputs use Rust callbacks, not oracle |
| Zero-copy semantics | ✅ Yes | Iterator-based streaming |
| Multiple calls | ✅ Yes | Via DISCONNECT_ORACLE_QUERY_ID |
| Idempotent | ⚠️ Partial | Requires explicit disconnect |

**Verdict**: Shares Option 2 principles but different API

### Implementation Details

**Input Mechanism** (`/home/cody/zisk/oracle/src/lib.rs`):
1. Guest writes query_id via `csrrw` to CSR 0x7c0
2. Guest writes input_len (in u32 words)
3. Guest writes input data words
4. Guest reads response_len, then response data

**Output Mechanism**:
- Outputs NOT written through oracle
- Accumulated in `ForwardRunningResultKeeper`
- Returned via Rust API callbacks

### Query IDs Used
- `0x40070002` - BLOCK_METADATA_QUERY_ID
- `0x40060000` - NEXT_TX_SIZE_QUERY_ID
- `0x40060001` - TX_DATA_WORDS_QUERY_ID
- `0x40020000+` - PREIMAGE_SUBSPACE_MASK
- `0x40000001` - DISCONNECT_ORACLE_QUERY_ID
- `0xffffffff` - UART_QUERY_ID (debug only)

### Recommendations
1. Document the CSR-based interface as a "ZisK-specific Option 2 variant"
2. Consider adding adapter functions if strict Option 2 compatibility is needed
3. Define output writing through oracle if needed for non-determinism replay

---

## 3. Cryptographic Accelerators Standard Conformance

### Status: 17/22 Functions Implemented

### Implementation Mechanism
- **Hash Functions**: CSR syscalls (0x800 Keccak, 0x805 SHA256)
- **EC Operations**: CSR syscalls (0x801-0x810)
- **Precompiles**: System hooks via `PurePrecompileInvocation` trait
- **BLS12-381**: Pure Rust with modular arithmetic delegation

### Function Compliance Matrix

#### Hash Functions (4/4)

| Function | Status | Location | Notes |
|----------|--------|----------|-------|
| zkvm_keccak256 | ✅ | `crypto/src/sha3/delegated/zisk_precompile.rs:54-76` | CSR 0x800, state-based |
| zkvm_sha256 | ✅ | `ziskos/entrypoint/src/syscalls/sha256f.rs:30-35` | CSR 0x805, compress function |
| zkvm_ripemd160 | ✅ | `system_hooks/src/lib.rs:259` | Precompile 0x03 |
| zkvm_blake2f | ✅ | `system_hooks/src/eip_152/mod.rs:17-25` | Precompile 0x09, pure Rust |

#### ECDSA Functions (2/3)

| Function | Status | Location | Notes |
|----------|--------|----------|-------|
| zkvm_secp256k1_verify | ❌ | N/A | Not implemented |
| zkvm_secp256k1_ecrecover | ✅ | `crypto/src/secp256k1/recover.rs:20-78` | Precompile 0x01 |
| zkvm_secp256r1_verify | ⚠️ | `crypto/src/secp256r1/verify.rs:12-36` | Function exists, precompile gated |

#### Math Functions (1/1)

| Function | Status | Location | Notes |
|----------|--------|----------|-------|
| zkvm_modexp | ✅ | `system_hooks/src/lib.rs:263-264` | Precompile 0x05 |

#### BN254 Functions (3/3)

| Function | Status | Location | Notes |
|----------|--------|----------|-------|
| zkvm_bn254_g1_add | ✅ | `system_hooks/src/lib.rs:266-267` | Precompile 0x06 |
| zkvm_bn254_g1_mul | ✅ | `system_hooks/src/lib.rs:269-270` | Precompile 0x07 |
| zkvm_bn254_pairing | ✅ | `system_hooks/src/lib.rs:272-273` | Precompile 0x08 |

#### BLS12-381 Functions (7/7)

| Function | Status | Location | Notes |
|----------|--------|----------|-------|
| zkvm_bls12_g1_add | ✅ | `system_hooks/src/eip_2537/addition.rs` | Precompile 0x0b |
| zkvm_bls12_g1_msm | ✅ | `system_hooks/src/eip_2537/msm.rs` | Precompile 0x0c |
| zkvm_bls12_g2_add | ✅ | `system_hooks/src/eip_2537/addition.rs` | Precompile 0x0d |
| zkvm_bls12_g2_msm | ✅ | `system_hooks/src/eip_2537/msm.rs` | Precompile 0x0e |
| zkvm_bls12_pairing | ✅ | `system_hooks/src/eip_2537/pairing.rs` | Precompile 0x0f |
| zkvm_bls12_map_fp_to_g1 | ✅ | `system_hooks/src/eip_2537/mappings.rs` | Precompile 0x10 |
| zkvm_bls12_map_fp2_to_g2 | ✅ | `system_hooks/src/eip_2537/mappings.rs` | Precompile 0x11 |

#### KZG Functions (0/1)

| Function | Status | Location | Notes |
|----------|--------|----------|-------|
| zkvm_kzg_point_eval | ❌ MOCK | `system_hooks/src/mock_precompiles.rs:28-41` | Input validation only, no computation |

### Type Conformance
- **zkvm_status enum**: Not used; Rust `Result<T, E>` pattern instead
- **zkvm_bytes_* types**: Not used; native Rust `[u8; N]` arrays instead
- **64-bit alignment**: ✅ Enforced for syscall operations

### ZisK Native Syscall IDs
```
0x800: Keccak-f1600 permutation
0x801: 256-bit arithmetic
0x802: 256-bit modular arithmetic
0x803: Secp256k1 point addition
0x804: Secp256k1 point doubling
0x805: SHA256 compress function
0x806-0x807: BN254 curve operations
0x808-0x80A: BN254 complex field operations
0x80B: 384-bit modular arithmetic
0x80C-0x810: BLS12-381 operations
```

### Gaps and Recommendations

| Gap | Impact | Recommendation |
|-----|--------|----------------|
| **KZG point eval (0x0a)** | HIGH | Implement full KZG computation for EIP-4844 support |
| **secp256k1_verify** | MEDIUM | Implement if ECDSA verification without recovery needed |
| **secp256r1 precompile** | LOW | Enable `p256_precompile` feature flag if needed |

---

## 4. Appendix: Evidence References

### Key Files

**RISC-V Target**:
- `/home/cody/zisk/zksync-os/zksync_os/.cargo/config.toml` - Target configuration
- `/home/cody/zisk/zksync-os/zksync_os/src/lds/memory-zisk.x` - Memory layout
- `/home/cody/zisk/core/src/riscv2zisk_context.rs` - ISA support documentation

**IO Interface**:
- `/home/cody/zisk/oracle/src/lib.rs` - CSR protocol implementation
- `/home/cody/zisk/zksync-os/zk_ee/src/system_io_oracle/mod.rs` - IOOracle trait
- `/home/cody/zisk/zksync-os/oracle_provider/src/lib.rs` - Query processor

**Crypto Accelerators**:
- `/home/cody/zisk/zksync-os/system_hooks/src/lib.rs` - Precompile registration
- `/home/cody/zisk/zksync-os/system_hooks/src/eip_2537/` - BLS12-381 implementations
- `/home/cody/zisk/ziskos/entrypoint/src/syscalls/syscall.rs` - Syscall IDs

---

## Implementation Tasks

### Progress

- [x] **Task #1**: Assess RISC-V Target Standard Conformance
- [x] **Task #2**: Assess IO Interface Standard Conformance
- [x] **Task #3**: Assess Cryptographic Accelerators Standard Conformance
- [x] **Task #4**: Compile Final Conformance Report

---

## Notes

- The standards are marked as "tentative and under active development"
- This assessment reflects the current state as of 2026-01-28
- ZisK targets RV64IMAC - this is conformant since the standard defines a *minimum* (RV64IM), not a maximum
