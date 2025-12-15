# Plan: Use Fixed-Width U32 Serialization for Oracle I/O

## Problem

The zksync-os oracle I/O currently uses `UsizeSerializable` which serializes data differently based on target architecture:
- On rv32: Bytes32 becomes 8 x u32 values
- On rv64: Bytes32 becomes 4 x u64 values

This creates a mismatch when:
1. Witness is generated on airbender (rv32) with u32 values
2. Witness is consumed on Zisk (rv64) expecting u64 values

Current workaround involves format conversion in `Replay64Oracle` and changing `csr_read_word()` return type - both are fragile.

## Root Cause

`UsizeSerializable` is used in the oracle query path:

```
zksync-os/proof_running_system/src/io_oracle/mod.rs:
  - raw_query() uses UsizeSerializable::iter(input)
  - Writes/reads usize values via csr_write_impl/csr_read_impl

zksync-os/zk_ee/src/utils/bytes32.rs:
  - impl UsizeSerializable for Bytes32
  - Returns different representations based on target_pointer_width
```

## Proposed Solution

Replace `UsizeSerializable` with fixed-width `U32Serializable` for oracle I/O:

1. Create `U32Serializable` trait (or similar) that always uses u32 regardless of architecture
2. Update `raw_query()` to use fixed-width serialization
3. Update `Bytes32` and other types to implement fixed-width oracle serialization
4. Keep `csr_read_word() -> u32` unchanged in airbender

## Benefits

- Witness format is architecture-independent
- rv64 guest can consume rv32-generated witness directly
- No format conversion needed in replay oracle
- No changes to airbender's csr_read_word() needed
- Simpler debugging - exact parity with airbender behavior
- Oracle returns raw u32 stream on both architectures

## Files to Modify (in zksync-os)

1. `zk_ee/src/oracle/usize_serialization.rs` - Add U32Serializable trait
2. `zk_ee/src/utils/bytes32.rs` - Implement U32Serializable for Bytes32
3. `proof_running_system/src/io_oracle/mod.rs` - Use U32Serializable in raw_query
4. Other types implementing UsizeSerializable for oracle use

## Alternative Considered

Modify Zisk's replay oracle to convert u32 witness to u64 format and change airbender's `csr_read_word()` to return `usize`. This works but:
- Requires modifying airbender code
- Adds format conversion complexity
- Creates divergence between airbender and Zisk behavior
- Harder to debug due to format differences
