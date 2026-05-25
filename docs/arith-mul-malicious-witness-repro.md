# Arith MUL Malicious-Witness Repro

## Summary

This branch demonstrates a soundness bug in ZisK's 64-bit `mul` operation: the
stock circuit can prove the same program with `MUL(-1, 1) = 1` instead of the
correct result `-1`.

The branch also carries a minimal proposed fix in
`state-machines/arith/pil/arith.pil`. The fix adds no trace columns, but the
new constraint compiles as degree 5 in the Arith AIR, which I think is unacceptable.

The containerized repro runs:

1. stock circuit + malicious witness -> `MUL(-1,1) = 1` verifies;
2. rebuild proving key from the patched PIL;
3. patched circuit + malicious witness -> rejected;
4. patched circuit + honest witness -> verifies.

## Branch Contents

- `state-machines/arith/pil/arith.pil` - proposed constraint tying the signed
  product sign to operand signs.
- `core/src/zisk_ops.rs` - env-gated malicious Main witness: `op_mul(-1, 1)`
  returns `1`.
- `state-machines/arith/src/arith_full.rs` - env-gated malicious Arith witness:
  emits the matching `c=1, d=0, np=0` row.
- `elf-regressions/arith_bad_mul/test.s` - tiny RV64 program:
  `li t0,-1; li t1,1; mul t2,t0,t1`, then stores `t2` into public outputs.
- `Dockerfile.repro-arith-mul`, `repro-arith-mul.sh`,
  `rebuild-patched-pk.sh` - GPU repro environment, driver, and patched proving
  key builder.

The env-gated code is repro scaffolding. It simulates a malicious prover
choosing an invalid witness while leaving normal execution unchanged unless
`ZISK_REPRO_BAD_ARITH_MUL` is set.

## Bug

For an Arith `MUL` row, the operation bus fixes `op`, `a`, and `b`; the row
should then uniquely determine:

- `c`: low 64 bits of the product;
- `d`: high 64 bits of the product;
- `np`: sign bit used by the Arith equations for the product.

For `a = -1`, `b = 1`, the stock Arith AIR accepts both rows:

| witness   | `c`           | `d`           | `np` | product encoded |
|-----------|---------------|---------------|------|-----------------|
| honest    | `0xffff..ffff` | `0xffff..ffff` | `1`  | `-1`            |
| malicious | `1`           | `0`           | `0`  | `+1`            |

The chunk equations effectively check `|a * b| = |result|`, with `np` selecting
the sign of the claimed product. `np` is cross-checked against the sign of `d`,
but `d` is itself witness-controlled. Nothing in the stock AIR forces `np` to
match the sign implied by the operands.

Main reads `c` as the `mul` result. The malicious witness therefore makes the
same instruction write `1` instead of `0xffff_ffff_ffff_ffff`.

Both Main and Arith are patched in the repro because the operation bus requires
them to agree. Main emits `c=1` on the bus, and Arith proves the matching bad
row. That is the minimum self-consistent malicious witness needed to pass the
global bus check.

The ELF commits the result to public outputs with ordinary stores to
`OUTPUT_ADDR = 0xa001_0000`, the same region used by `ziskos::set_output`.
Successful prove logs print the first public slots, so the proved value is
visible.

## Proposed Fix

`state-machines/arith/pil/arith.pil` adds:

```pil
signed * (1 - div) * (np - (na + nb - 2 * na * nb))
                   * (c[0] + c[1] + c[2] + c[3] + d[0] + d[1] + d[2] + d[3]) === 0;
```

`na + nb - 2 * na * nb` is `na XOR nb`. For signed multiplication, a nonzero
product is negative exactly when one operand is negative, so the constraint
forces `np = na XOR nb`.

The selector `signed * (1 - div)` limits the check to signed multiplication
rows (`mul`, `mulh`, `mulsuh`) and excludes division rows.

The limb-sum factor makes the constraint vacuous for zero products. That case
must be exempt because, for example, `0 * (-5)` has `na XOR nb = 1` but product
sign `np = 0`. Since all `c[i]` and `d[i]` limbs are range-checked as 16-bit
values, their sum is zero in Goldilocks iff every limb is zero: the integer sum
is at most `8 * (2^16 - 1)`, far below the field modulus. For zero products,
the existing Arith table/range checks still pin `np` consistently with `d = 0`.

For the malicious row, `signed=1`, `div=0`, `na=1`, `nb=0`, `np=0`, and the
limb sum is `1`, so the new constraint rejects it. The honest row has `np=1`
and passes.

Compilation note: using `pil2-compiler v0.9.0`, this line remains an Arith
every-row constraint of degree 5. It is not rewritten into lower-degree
constraints and it adds no witness columns.

## Run

Requires an NVIDIA GPU and NVIDIA Container Toolkit. The Dockerfile defaults to
`CUDA_ARCH=sm_120` for RTX 5090; change it for other GPUs.

```bash
docker build -f Dockerfile.repro-arith-mul -t zisk-arith-mul-repro .

docker run --rm --gpus all \
  -v "$HOME/.zisk:/root/.zisk" \
  zisk-arith-mul-repro
```

The script installs the stock v0.18.0 proving key if needed, rebuilds the
patched proving key into `~/.zisk/provingKey-patched`, and caches both across
runs. The patched setup is slow because it recompiles `zisk.pilout`, regenerates
the proving key, builds recursive/aggregation setup, and generates GPU constant
trees.

Expected summary:

```text
Phase 1   stock   circuit + malicious witness : VERIFIED
Phase 3a  patched circuit + malicious witness : REJECTED (rc=...)
Phase 3b  patched circuit + honest witness    : VERIFIED
```

Expected public outputs:

```text
# Phase 1: stock circuit + malicious witness
Public outputs[0..8]: 0x00000001 0x00000000 ...    # t2 = 1

# Phase 3b: patched circuit + honest witness
Public outputs[0..8]: 0xffffffff 0xffffffff ...    # t2 = -1
```

The stock circuit verifies the wrong public output. The patched circuit rejects
that bad witness while preserving the honest one.
