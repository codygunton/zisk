ZKsyncOS (in zksync-os/) is a "guest program" for proving EVM execution using Airbender (in zksync-airbender/), a ZKVM targeting RV32IM + a limited set of csrrw instructions used to interact with external oracles. We are adapting that guest program to run in ZisK, which targets RV64IMAFDC.

We have achieved the primary goal of using ZisK to execute real Ethereum blocks in ZKsyncOS built for RV64IMAC with Keccak MPT support.

The cycle benchmarking/testing scripts:
- ./bench-zisk-cycles.sh - produces logs at /tmp/zisk-bench.log
- ./bench-airbender-cycles.sh - produces logs at /tmp/airbender-bench.log

For GPU proving development:
- ./prove-block-gpu.sh - produces logs at /tmp/prove-block.log

You will NOT use needlessly long timeouts when running these. 90s is sufficient for each.

## Development Workflow

Use `prove-block-gpu.sh` for proving development. This script:
1. Uses pre-built binaries from `./target/release/`
2. Sources Intel OneAPI environment automatically
3. Logs verbose output to /tmp/prove-block.log (terminal shows minimal output)

Do NOT attempt to rebuild with `cargo build --features gpu` during development iterations - the build environment issues with Intel libraries are already handled by the script.

## Current State

ZKsyncOS is built with the `zisk_keccak` feature, which uses ZisK's native Keccak precompile (CSR 0x800) for the Ethereum MPT. This is different from Airbender's Keccak delegation protocol (CSR 0x7CB).

The bench-zisk-cycles.sh script completes successfully, processing real Ethereum blocks with Keccak MPT state proofs.

## Notes

- Blake2s delegation (CSR 0x7C7) support was removed from ZisK as it is not needed for Ethereum execution. The earlier zisk-integration branches used Blake2s for the storage tree, but the current implementation uses Keccak.
- U256 delegation (CSR 0x7CA) is still supported for BigInt arithmetic operations.

# Tools
gcc binutils for risc-v are installed as  riscv64-elf-foo (e.g.,  riscv64-elf-objdump).
