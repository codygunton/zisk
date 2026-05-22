# Arith MUL Malicious-Witness Repro

This branch demonstrates that a malicious witness generator can make ZisK
accept an RV64 `MUL` row where `-1 * 1` produces `1` instead of the correct
low 64-bit result `0xffffffffffffffff`.

The repro is intentionally env-gated. Normal execution is unchanged unless
`ZISK_REPRO_BAD_ARITH_MUL=1` is set.

This is a reproduction branch, not a proposed fix. The env-gated code mutates
both the Main result and the Arith witness so the ordinary
`verify-constraints --emulator` entrypoint can inspect the same kind of
globally balanced malicious trace that a hostile witness generator could try to
submit.

## What the patch changes

- `core/src/zisk_ops.rs`
  - Under `ZISK_REPRO_BAD_ARITH_MUL=1`, Main computes
    `op_mul(0xffffffffffffffff, 1) = 1`.
  - This makes Main store the malicious result and assume that result on the
    operation bus.
- `state-machines/arith/src/arith_full.rs`
  - Under the same env var, Arith emits a malformed row for `(MUL, -1, 1)`:
    - `c = 1`
    - `d = 0`
    - `na = 1`
    - `nb = 0`
    - `np = 0`
    - `nr = 0`
    - `range_ab = 7`
    - `range_cd = 1`
    - `carry = [-1, -1, -1, -1, 0, 0, 0]`
- `elf-regressions/arith_bad_mul/test.s`
  - Minimal RV64 assembly input that executes `li t0, -1; li t1, 1; mul t2, t0, t1`.

Main and Arith agree on the same bad operation-bus result, so the global bus
permutation balances. The issue is that the Arith constraints do not force the
signed multiplication result to be correct for this sign pattern.

## Build

On Ubuntu 24.04 with ZisK's usual native dependencies:

```bash
cargo build --release --bin cargo-zisk --features cpu-only
```

On a host whose MPI/Clang libraries do not match ZisK's dependencies, this
Docker command matches the environment used by the test monitor:

```bash
docker run --rm \
  -v "$PWD:/workspace/zisk" \
  -v "$HOME/.cargo/registry:/root/.cargo/registry" \
  -v "$HOME/.cargo/git:/root/.cargo/git" \
  -w /workspace/zisk \
  -e CARGO_NET_GIT_FETCH_WITH_CLI=true \
  -e CARGO_TARGET_DIR=/workspace/zisk/target-docker \
  ubuntu:24.04 bash -lc '
    set -euo pipefail
    export DEBIAN_FRONTEND=noninteractive
    apt-get update >/dev/null
    apt-get install -y \
      curl git ca-certificates build-essential clang libclang-dev pkg-config cmake \
      jq xz-utils libomp-dev libgmp-dev nlohmann-json3-dev protobuf-compiler \
      uuid-dev libgrpc++-dev libsecp256k1-dev libsodium-dev libpqxx-dev nasm \
      libopenmpi-dev openmpi-bin openmpi-common gcc-riscv64-unknown-elf >/dev/null
    if [ ! -x /root/.cargo/bin/cargo ]; then
      curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y >/dev/null
    fi
    . /root/.cargo/env
    cargo build --release --bin cargo-zisk --features cpu-only
  '
```

## Build the tiny ELF

```bash
mkdir -p /tmp/zisk-mul-edge
riscv64-unknown-elf-gcc \
  -march=rv64imac -mabi=lp64 \
  -nostdlib -nostartfiles -static -Ttext=0x80000000 \
  elf-regressions/arith_bad_mul/test.s \
  -o /tmp/zisk-mul-edge/arith_bad_mul.elf
```

## Control run

```bash
LD_LIBRARY_PATH=/path/to/zisk-lib \
  ./target-docker/release/cargo-zisk \
  verify-constraints \
  --elf /tmp/zisk-mul-edge/arith_bad_mul.elf \
  --emulator \
  -k "$HOME/.zisk/provingKey"
```

Expected result: all local and global constraints pass.

If you built directly on the host instead of using the Docker command, replace
`./target-docker/release/cargo-zisk` with `./target/release/cargo-zisk`.

## Malicious run

```bash
ZISK_REPRO_BAD_ARITH_MUL=1 \
RUST_LOG=info \
LD_LIBRARY_PATH=/path/to/zisk-lib \
  ./target-docker/release/cargo-zisk \
  verify-constraints \
  --elf /tmp/zisk-mul-edge/arith_bad_mul.elf \
  --emulator \
  -k "$HOME/.zisk/provingKey"
```

Expected result: all local and global constraints still pass, and the log
contains:

```text
WARN: injecting bad Arith MUL repro row: op=MUL a=0xffffffffffffffff b=1 c=1 d=0
```

In the original investigation, patching only Arith caused the Arith instance
to pass but global constraint #0 to fail. This branch patches Main as well,
which demonstrates that the global operation-bus check links Main and Arith
but does not make the Arith signed multiplication relation sound.
