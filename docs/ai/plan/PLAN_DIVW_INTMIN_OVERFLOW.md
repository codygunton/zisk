# DIVW Intmin Overflow

## Goal

Make a branch with a minimal reproducer for issue #7, following the structure of `cg/auipc-rv64-overflow`.

## Checklist

- [x] Inspect issue #7 and the `cg/auipc-rv64-overflow` example branch.
- [x] Create an isolated worktree and branch from `origin/main`.
- [x] Add a focused `elf-regressions/divw_intmin_overflow` assembly fixture and README.
- [x] Add or reuse helper scripts matching the example branch.
- [x] Add a scoped diagnostic comment near the relevant operation-bus path.
- [x] Build the repro ELF and run focused verification where available.
- [x] Review diff, commit the repro branch, and push it to `origin`.
- [x] Rebase the PR branch onto `cg/repro-base-v1.0.0-alpha`.
- [x] Validate both DIVW fixtures against the official `cargo-zisk 1.0.0-alpha` bundle.

## Current Notes

The bug is a DIVW signed-overflow mismatch: `core/src/zisk_ops.rs::op_div_w` widens operands to `i64`, producing positive `+2^31` and committing `0x0000000080000000`, while RISC-V DIVW and the Arith/PIL path expect 32-bit wrapping followed by sign-extension to `0xFFFFFFFF80000000`.

Original v0.18 verification results:

- `./build-elf elf-regressions/divw_intmin_overflow/test.s` succeeded.
- `./build-elf elf-regressions/divw_intmin_safe/test.s` succeeded.
- `./prove-elf elf-regressions/divw_intmin_overflow/test.elf` failed as expected at `VerifyGlobalConstraints`.
- `./prove-elf elf-regressions/divw_intmin_safe/test.elf` succeeded and verified the final proof.

Alpha rebase note: branch was rebased onto `cg/repro-base-v1.0.0-alpha` for ZisK team review, and `prove-elf` now accepts both the old `--verify-proofs`/`--emulator` CLI and the 1.0-alpha `--verify-proof` default-emulator CLI. With `cargo-zisk 1.0.0-alpha [cpu] (4b9f758 2026-06-19T11:52:52.465302597Z)`, both `divw_intmin_overflow` and `divw_intmin_safe` build and generate verified proofs. The alpha source has explicit `op_div_w` handling for `0x80000000 / -1`, so issue #7 appears fixed on this base and this branch is now a regression check.
