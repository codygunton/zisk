# ZisK Changes Since Forkbase

This document summarizes the major changes made to the ZisK repository since the `forkbase` tag. Changes are grouped by semantic area.

## Docker Infrastructure

Complete Docker-based proving environment for standalone use.

| Files | Description |
|-------|-------------|
| `Dockerfile` | CUDA-based image with RISC-V toolchain, GCC 14, and full proving key generation |
| `docker-entrypoint.sh` | Entrypoint script for execute/prove/shell modes |
| `drun` | Host-side wrapper script for Docker commands |

## CLI Commands for ZKsyncOS

New cargo-zisk subcommands for running and proving ZKsyncOS Ethereum blocks.

| Files | Description |
|-------|-------------|
| `cli/src/commands/zksyncos_common.rs` | Shared utilities for ZKsyncOS commands |
| `cli/src/commands/zksyncos_prove.rs` | `zksyncos-prove` command implementation |
| `cli/src/commands/zksyncos_run.rs` | `zksyncos-run` command implementation |
| `cli/src/commands/mod.rs` | Command registration |
| `cli/src/commands/prove.rs` | Extended prove command with `--witness-file` support |
| `cli/src/bin/cargo-zisk.rs` | Binary entry point |

## U256 Delegation Precompile (New)

Complete state machine for 256-bit integer delegation operations (ADD, SUB, MUL_LOW, MUL_HIGH, EQ, MEMCPY). This is a new precompile that handles BigInt operations from ZKsyncOS.

| Files | Description |
|-------|-------------|
| `precompiles/u256_delegation/pil/u256_delegation.pil` | PIL constraints for U256 operations |
| `precompiles/u256_delegation/src/lib.rs` | Crate entry point |
| `precompiles/u256_delegation/src/operations/*.rs` | Individual operation implementations (add, eq, mul) |
| `precompiles/u256_delegation/src/u256_delegation_sm.rs` | Main state machine |
| `precompiles/u256_delegation/src/u256_delegation_*.rs` | Bus, input, instance, manager, planner modules |

## Oracle Processing (New)

New `oracle` crate for handling ZKsyncOS I/O oracle queries during emulation and proving.

| Files | Description |
|-------|-------------|
| `oracle/src/lib.rs` | Oracle trait and main processing logic |
| `oracle/src/processors/replay.rs` | Replay processor for witness playback |
| `oracle/src/processors/uart.rs` | UART output processor |
| `oracle/src/processors/block_metadata.rs` | Block metadata processor |
| `oracle/src/query_buffer.rs` | Query buffer management |
| `oracle/src/query_ids.rs` | Query ID definitions |
| `oracle/src/error.rs` | Error types |

## Emulator Oracle & Witness Support

Extended emulator to support oracle-based I/O and witness generation/replay.

| Files | Description |
|-------|-------------|
| `emulator/src/oracle_helper.rs` | New: Oracle callback helper |
| `emulator/src/witness.rs` | New: Witness capture and replay |
| `emulator/src/emu.rs` | Oracle integration, U256 delegation handling |
| `emulator/src/emulator.rs` | Witness mode support |
| `emulator/src/emu_options.rs` | New CLI flags (`--oracle`, `--witness-file`) |
| `emulator/src/bin/ziskemu.rs` | CLI argument handling |

## Core U256 & Oracle Support

Core data structures for U256 operations and oracle interfacing.

| Files | Description |
|-------|-------------|
| `core/src/u256.rs` | New: U256 delegation operation types and data structures |
| `core/src/oracle.rs` | New: Oracle types for external I/O |
| `core/src/mem.rs` | Extended memory operations for 256-bit data |
| `core/src/inst_context.rs` | Instruction context for U256 operations |
| `core/src/elf_extraction.rs` | ELF loading improvements |

## Scripts

Shell scripts for build, execution, and proving workflows.

| Files | Description |
|-------|-------------|
| `setup.sh` | Build ZKsyncOS and run ROM setup |
| `prove-block-zisk.sh` | GPU proving workflow for Ethereum blocks |
| `execute-zisk.sh` | Execute blocks with ZisK emulator |
| `execute-airbender.sh` | Execute blocks with Airbender |
| `execute.sh` | Simple execution wrapper |
| `forward.sh` | Forward-mode witness generation |
| `list-txs.sh` | List transactions in a block |
| `regenerate-proving-key.sh` | Regenerate proving key artifacts |
| `regenerate-traces.sh` | Regenerate PIL traces |

## PIL Changes

Updates to the constraint system for U256 delegation and bus operations.

| Files | Description |
|-------|-------------|
| `pil/zisk.pil` | Include u256_delegation precompile |
| `pil/operations.pil` | New operations for delegation |
| `pil/src/pil_helpers/traces.rs` | Generated trace structures |

## Executor & State Machine Integration

Integration of U256 delegation into the proving pipeline.

| Files | Description |
|-------|-------------|
| `executor/src/executor.rs` | Register U256 delegation SM |
| `executor/src/sm_static_bundle.rs` | Static bundle with U256 |
| `executor/src/static_data_bus.rs` | Data bus integration |
| `executor/src/static_data_bus_collect.rs` | Data collection |
| `state-machines/main/src/main_sm.rs` | Main SM coordination |

## SDK Prover Extensions

Backend-agnostic prover abstraction for witness-based proving.

| Files | Description |
|-------|-------------|
| `sdk/src/prover/mod.rs` | Prover module reorganization |
| `sdk/src/prover/emu.rs` | Emulator-based prover |
| `sdk/src/prover/asm.rs` | Assembly-based prover |
| `sdk/src/prover/backend.rs` | Backend trait |

## Witness Computation

Witness library for external witness generation.

| Files | Description |
|-------|-------------|
| `witness-computation/src/zisk_lib.rs` | Witness computation interface |
| `witness-computation/Cargo.toml` | Dependencies |

## Other Changes

| Files | Description |
|-------|-------------|
| `Cargo.toml` | Workspace configuration, new crate dependencies |
| `.gitmodules` | zksync-os submodule reference |
| `common/src/bus/data_bus_operation.rs` | Bus operation types |
| `common/src/zisk_lib_init.rs` | Library initialization |
| `zkvm-standards-conformance-assessment.md` | Standards compliance documentation |
