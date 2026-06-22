# Arith DIV/REM malicious-witness repro

This branch is a v0.18.0-based reproduction of a candidate signed DIV/REM
soundness bug. It follows the same container stages as `cg/patch-mul`.

The malicious witness targets:

```text
DIV(-256, 256) = 0       # honest result: -1
REM(-256, 256) = -256    # honest result: 0
```

The assignment preserves the division identity:

```text
-256 = 256 * 0 + (-256)
```

but violates the RISC-V remainder bound because `|-256| < |256|` is false.
The current circuit delegates that bound to Binary `OP_LT_ABS_NP` when the
remainder is negative and the divisor is non-negative. That bytewise table
computes the two's-complement `+1` only in the first byte, so the `-256` case
can be accepted as if `abs(-256) < 256`.

## Branch contents

- `elf-regressions/arith_bad_div_rem/test.s`: tiny RV64 program that executes
  `DIV` and `REM` on `-256, 256`.
- `core/src/zisk_ops.rs`: env-gated Main result injection under
  `ZISK_REPRO_BAD_ARITH_DIV_REM`.
- `state-machines/arith/src/arith_full.rs`: env-gated Arith witness injection,
  including the matching pending Binary lookup input.
- `state-machines/arith/pil/arith.pil`: diagnostic patched-key guard that
  rejects this malicious negative-remainder witness. This is not presented as
  ZisK's final efficient fix.
- `Dockerfile.repro-arith-div-rem`, `repro-arith-div-rem.sh`,
  `rebuild-patched-pk.sh`: one-command Docker repro and patched proving-key
  rebuild flow.

## Run

```bash
docker build -f Dockerfile.repro-arith-div-rem -t zisk-repro-arith-div-rem .
docker run --rm --gpus all zisk-repro-arith-div-rem
```

The ELF is assembled as RV64IM with RVC disabled so the interpreter sees only
complete 32-bit instructions.

Expected summary:

```text
Phase 1   stock   circuit + malicious  (prove)             : VERIFIED
Phase 3a  patched circuit + malicious  (verify-constraints): REJECTED (...)
Phase 3b  patched circuit + honest     (verify-constraints): PASSED
```

Phase 1 uses the stock v0.18.0 proving key from `ziskup`. Phase 2 rebuilds a
basic patched proving key from this branch's PIL at
`~/.zisk/provingKey-patched-div-rem`. Phase 3 uses `verify-constraints` rather
than recursive proving for the patched circuit, as in the MUL demo.
