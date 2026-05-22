# Arith MUL Malicious-Witness Repro

## TL;DR

**Stock ZisK accepts signed-MUL-family malicious witnesses.** This branch
demonstrates low `MUL(-1, 1) = 1` and high-half
`MULH(-1, 1) = 0` / `MULHSU(-1, 1) = 0`.

We do **not** change any circuit, constraint, or verifier code. The AIR
constraints and the proof verifier are exactly stock ZisK. What we change is
the *witness* — the trace values fed to the prover — because the witness is
precisely what a malicious prover controls.

ZisK's witness is built by a witness generator. An attacker forging a proof
runs their own witness generator, so this repro simulates one: under env
flags it emits traces claiming `MUL(-1, 1) = 1`, `MULH(-1, 1) = 0`, or
`MULHSU(-1, 1) = 0`. Stock ZisK's constraints then accept those traces —
locally and globally — and `prove --verify-proofs` produces proofs that
verify.

That acceptance is the bug, and it lives in the Arith circuit, not in our
patch: the signed-multiplication constraints do not pin the result for this
sign pattern. Because the repro makes no circuit changes, the verifier is the
fixed reference point here — this branch only shows that the *unmodified*
verifier will accept `-1 * 1 = 1`. The fix, when it comes, is also a circuit
change; nothing in this repro needs to be undone in the verifier.

### Why two files are patched

A malicious prover cannot change one number in isolation. ZisK's Main and
Arith state machines are linked by the operation bus, and the global
permutation check forces the two sides to agree. A real attacker's witness
generator must therefore produce a *self-consistent* malicious trace on both
sides. The two env-gated patches — `core/src/zisk_ops.rs` (Main) and
`state-machines/arith/src/arith_full.rs` (Arith) — are not two independent
hacks; together they are one self-consistent malicious witness, the minimum
needed to pass the global bus check. Patching only the Arith side fails global
constraint #0 — that is the system working as intended.

The repro is intentionally env-gated: normal execution is unchanged unless
`ZISK_REPRO_BAD_ARITH_MUL=1` is set. This is a reproduction branch, not a
proposed fix.

## What the patch changes

- `core/src/zisk_ops.rs`
  - Under `ZISK_REPRO_BAD_ARITH_MUL=1`, Main computes
    `op_mul(0xffffffffffffffff, 1) = 1`,
    `op_mulh(0xffffffffffffffff, 1) = 0`, or
    `op_mulsuh(0xffffffffffffffff, 1) = 0`, selected by
    `ZISK_REPRO_BAD_ARITH_MUL_KIND`.
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
- `elf-regressions/arith_bad_{mul,mulh,mulhsu}/test.s`
  - Minimal RV64 assembly inputs that execute `li t0, -1; li t1, 1`
    followed by `mul`, `mulh`, or `mulhsu`.

Main and Arith agree on the same bad operation-bus result, so the global bus
permutation balances. The issue is that the Arith constraints do not force the
signed multiplication result to be correct for this sign pattern.

## Build and run

The reproduction runs entirely through the branch Dockerfile, which builds a
GPU-capable `cargo-zisk` and tiny `MUL`, `MULH`, and `MULHSU` ELFs. See
`Dockerfile.repro-arith-mul` for the exact build steps and dependencies.

It requires an NVIDIA GPU and the NVIDIA Container Toolkit. The build targets
compute capability `sm_120` (RTX 5090); change `CUDA_ARCH` in the Dockerfile
for a different GPU.

```bash
docker build -f Dockerfile.repro-arith-mul -t zisk-arith-mul-repro .

docker run --rm --gpus all \
  -v "$HOME/.zisk:/root/.zisk" \
  zisk-arith-mul-repro
```

The `~/.zisk` mount is the canonical ZisK directory. If it does not already
contain a `provingKey`, the container installs the stock v0.18.0 proving key
into it with `ziskup`; if the key is already present, that step is skipped, so
the (large) key is downloaded at most once and reused across runs.

The default container command runs `repro-arith-mul.sh`, which exercises the
malicious witness through two clearly labelled steps:

- **Step 1 — `verify-constraints`** (quick check): builds the execution trace
  and checks every AIR and global constraint. No proof is produced.
- **Step 2 — `prove --verify-proofs`** (full proof): generates a STARK proof
  and runs the stock verifier on that proof.

Both steps run with `ZISK_REPRO_BAD_ARITH_MUL=1`, so both operate on the
malicious witness. `ZISK_REPRO_BAD_ARITH_MUL_KIND` selects the opcode under
test, and each case prints a bad-row warning:

```text
WARN: injecting bad Arith MULH repro row: a=0xffffffffffffffff b=1 c=1 d=0
```

The script ends with an explicit summary:

```text
Step 1  verify-constraints    : ACCEPTED  -- malicious trace passed all constraints
Step 2  prove --verify-proofs  : VERIFIED  -- malicious proof was generated and verified

RESULT MULH: BUG REPRODUCED -- stock ZisK accepted and proved MULH(-1, 1) = 0.

RESULT: ALL BUGS REPRODUCED -- stock ZisK accepted and verified all malicious signed-MUL witnesses.
```
