# U256 Delegation Benchmark

Block: 24198369 (426 txs, 45.1M gas)
ELF: zksync_os_zisk.elf (ZKsyncOS with zisk_keccak feature)

## Summary

| Metric | Delegation ON | Delegation OFF | Delta |
|--------|---------------|----------------|-------|
| Proving Steps | 1,637,862,070 | 1,963,588,297 | +20% |
| Proving Time | 248.58s | 297.11s | +20% |
| ELF .text size | 929 KB | 1,013 KB | +9% |

**Conclusion**: U256 CSR delegation saves 20% of proving steps/time by offloading 256-bit arithmetic to hardware-accelerated precompiles instead of software implementation.

---

## Run 1: U256 Delegation ENABLED

- **Date**: 2026-01-20
- **Proving Steps**: 1,637,862,070
- **Proving Time**: 248.58 seconds
- **ELF .text Size**: 929,116 bytes (0x000e2d5c)
- **AIRs in proving key**: 32 (includes U256Delegation)

Uses CSR 0x7ca for U256 operations (add, sub, mul_low, mul_high, eq, memcpy).

---

## Run 2: U256 Delegation DISABLED

- **Date**: 2026-01-20
- **Proving Steps**: 1,963,588,297
- **Proving Time**: 297.11 seconds
- **ELF .text Size**: 1,013,468 bytes (0x000f76dc)
- **AIRs in proving key**: 21 (U256Delegation removed)

Uses ruint software fallback for U256 operations.

---

## Methodology

1. Built ZKsyncOS without `delegation` feature and with `bigint_ops` removed from `proving` feature in crypto crate
2. Regenerated proving key without U256Delegation AIR using `regenerate-proving-key.sh`
3. Generated new witness file with the non-delegation ELF
4. Ran GPU prover with the new proving key

Key changes to disable delegation:
- pil/zisk.pil: Commented out U256Delegation
- executor/src/static_data_bus*.rs: Commented out U256 collectors
- executor/src/sm_static_bundle.rs: Commented out U256DelegationManager
- witness-computation/src/zisk_lib.rs: Commented out U256 state machine
- zksync-os/crypto/Cargo.toml: Removed `bigint_ops` from `proving` feature
- setup.sh: Removed `delegation` from FEATURES

---

## Notes

- Proving steps measured from GPU prover trace (`time: X seconds, steps: Y`)
- ELF without delegation is larger because it includes software U256 implementation
- The 20% overhead without delegation is consistent across both steps and time
