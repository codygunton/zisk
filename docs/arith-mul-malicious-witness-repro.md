# Arith MUL Malicious-Witness: Repro and Fix

## TL;DR

One can prove `-1 * 1 = 1` for the standard 64-bit unsigned MUL. This branch demonstrates the bug and fixes it. The fix is one constraint in `state-machines/arith/pil/arith.pil`

Running the container walks through:

1. **stock circuit, malicious witness** — `prove --verify-proofs` produces a
   proof of `MUL(-1,1) = 1` that verifies (the bug);
2. **rebuild** the proving key from the patched PIL (warning, very slow)
3. **patched circuit** — the same malicious witness is now rejected, while the
   honest `MUL(-1,1) = -1` still proves and verifies.

## Changes in this branch

- `state-machines/arith/pil/arith.pil` — **the fix**: one new constraint forcing the product sign for signed MUL.
- `core/src/zisk_ops.rs` — env-gated malicious Main: `op_mul` returns `1` for `(-1)·1`.
- `state-machines/arith/src/arith_full.rs` — env-gated malicious Arith generator: emits the matching `c=1, d=0, np=0` row.
- `elf-regressions/arith_bad_mul/test.s` — minimal RV64 ELF input: `li t0,-1; li t1,1; mul t2,t0,t1`.
- `Dockerfile.repro-arith-mul`, `repro-arith-mul.sh`, `rebuild-patched-pk.sh` — container, three-phase driver, patched-pk rebuilder.

## The bug and its exploit

The Arith circuit row for a `MUL` receives fixed inputs from the operation bus
— `a`, `b`, `op` — and should *uniquely* determine the output columns: `c`
(low 64 bits of the product), `d` (high 64 bits), and `np` (the product's sign
bit). For `a = -1, b = 1` it does not. Both of these assignments satisfy every
constraint in the stock Arith AIR:

| witness   | `c`           | `d`           | `np` | encodes                |
|-----------|---------------|---------------|------|------------------------|
| honest    | `0xFFFF…FFFF` | `0xFFFF…FFFF` | `1`  | product `-1` (correct) |
| malicious | `1`           | `0`           | `0`  | product `+1` (wrong)   |

Main reads `c` as the MUL result, so the same instruction `mul t2,t0,t1` can be
proven to yield `-1` or `1`.

**Why both pass.** The Arith chunk equations reduce, for this case, to a single
identity of the form `|a · b| = |result|` — they check the *magnitude* of the
claimed result against the magnitude of the true product, with `np` acting as a
sign selector. `np` is only ever cross-checked against the sign of `d`, and `d`
is itself a free witness column. Nothing forces `np` to match the sign implied
by the *operands*. Since `|-1| = |+1| = 1`, both signs pass.

**Exploiting it.** A real attacker controls the witness generator; we simulate
one with an env gate. Under `ZISK_REPRO_BAD_ARITH_MUL=1`:

- `core/src/zisk_ops.rs` — `op_mul` returns `1` for `(-1) * 1`, so Main stores
  the malicious result and asserts it on the operation bus.
- `state-machines/arith/src/arith_full.rs` — Arith emits the matching malformed
  row (`c=1, d=0, np=0, …`).

Both sides are patched because Main and Arith are linked by the operation bus,
and the global permutation check forces them to agree: together they form one
self-consistent malicious witness, the minimum needed to pass the global bus
check. Normal execution is unchanged unless the env var is set. The input ELF
`elf-regressions/arith_bad_mul/test.s` is a stock RV64 program — `li t0,-1; li
t1,1; mul t2,t0,t1` — followed by two `sw` writes that commit `t2`'s two
halves to ZisK's public-output region (`OUTPUT_ADDR = 0xa001_0000`, the same
slots `ziskos::set_output` uses). That puts the value the proof attests to
into the publics, so the prove summary prints it. No special opcodes or
instrumentation; the writes are ordinary stores to a memory-mapped region.

## The fix

`state-machines/arith/pil/arith.pil` gains one constraint:

```
signed * (1 - div) * (np - (na + nb - 2 * na * nb))
                   * (c[0] + c[1] + c[2] + c[3] + d[0] + d[1] + d[2] + d[3]) === 0;
```

`na + nb - 2·na·nb` is `na XOR nb`. For a signed multiply the product is
negative exactly when one operand is negative, so this forces
`np = na XOR nb`. The selector `signed * (1 - div)` restricts it to signed
64-bit multiplication (`mul`, `mulh`, `mulsuh`).

The trailing factor `c[0]+…+d[3]` is the sum of the eight 16-bit limbs of the
result, and is zero in the Goldilocks field iff every limb is zero iff the
true product is zero. The "iff" needs justifying because field arithmetic can
wrap: but each limb is range-checked to a 16-bit unsigned value (`[0, 2¹⁶)`),
so the integer sum is at most `8 · (2¹⁶ − 1) < 2¹⁹`, far below the Goldilocks
modulus `p ≈ 2⁶⁴`. No wraparound — the sum as a field element equals the sum
as a non-negative integer, so it vanishes only when every limb does.

Multiplying by that sum makes the constraint vacuous on zero products, where
forcing `np = na XOR nb` would be wrong (e.g. `0 * (-5)`: `na=0, nb=1,
na XOR nb = 1`, but the true product is `0` whose sign bit is `0`). For zero
products, the existing `arith_table` lookup already pins `np = sign(d3) = 0`,
so `np` remains correctly fixed.

It adds no trace columns, so the same `cargo-zisk` binary drives both the
stock and patched proving keys.

With the constraint in place the malicious row is rejected (`na=1, nb=0` and a
nonzero limb sum force `np=1`, but the malicious witness has `np=0`), while
the honest row (`np=1`) passes.

## Build and run

The demonstration runs entirely through the branch Dockerfile. It requires an
NVIDIA GPU and the NVIDIA Container Toolkit; the build targets compute
capability `sm_120` (RTX 5090) — change `CUDA_ARCH` in the Dockerfile for a
different GPU.

```bash
docker build -f Dockerfile.repro-arith-mul -t zisk-arith-mul-repro .

docker run --rm --gpus all \
  -v "$HOME/.zisk:/root/.zisk" \
  zisk-arith-mul-repro
```

The container builds a GPU-capable `cargo-zisk`, the tiny `MUL` ELF, and the
pil2 toolchain needed to recompile the circuit. `repro-arith-mul.sh` then runs
three phases:

- **Phase 1** — stock proving key + malicious witness → `prove --verify-proofs`
  generates a full STARK proof of `MUL(-1,1) = 1` that **verifies**.
- **Phase 2** — recompile `zisk.pilout` from the patched `arith.pil` and
  regenerate the full proving key into `~/.zisk/provingKey-patched`. Slow
  (PIL compile + basic setup + recursive/aggregation setup + GPU constant
  trees, ~2 hours), and cached -- runs at most once per host.
- **Phase 3** — patched proving key + malicious witness → `prove --verify-proofs`
  **rejects** it; patched proving key + honest witness → `prove --verify-proofs`
  **verifies**.

Every phase exercises the full proving pipeline (basic per-AIR proofs
aggregated up to a vadcop final proof). The patched Arith constraint is a
local AIR constraint, so a malicious Arith row makes basic proof generation
fail; the run reports a non-zero exit, which the script records as REJECTED.

The `~/.zisk` mount is the canonical ZisK directory. The stock v0.18.0 proving
key is installed there by `ziskup` if absent, and the rebuilt patched key is
cached alongside it; both are large, so the run reuses them across invocations.

Each successful prove prints the proof's public outputs, so the committed
value of `t2` is visible directly in the log:

```text
# Phase 1 (stock, malicious):
Public outputs[0..8]: 0x00000001 0x00000000 ...    # t2 = 1   (wrong, but verifies)

# Phase 3b (patched, honest):
Public outputs[0..8]: 0xffffffff 0xffffffff ...    # t2 = -1  (correct, verifies)
```

Same ELF, same verifier, two distinct verifying proofs publicly committing to
two different values of `t2`. That contradiction is the soundness break.

The script ends with an explicit summary:

```text
Phase 1   stock   circuit + malicious witness : VERIFIED
Phase 3a  patched circuit + malicious witness : REJECTED (rc=...)
Phase 3b  patched circuit + honest witness    : VERIFIED

RESULT: as expected -- the stock circuit proves MUL(-1,1) = 1, the
        patched circuit rejects that malicious witness, and the honest
        multiplication still proves and verifies.
```
