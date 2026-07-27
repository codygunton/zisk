# Arith signed-DIV quotient-sign malicious-witness repro (v1.0.0-alpha)

A prover can forge the **sign of a signed-division quotient**. The stock circuit accepts a `DIV`
result that is the negation of the true quotient -- both `verify-constraints` and a full recursive
STARK proof (`prove --verify-proofs`). Demonstrated on `DIV(1, -1)`: honest quotient `-1`, forged
quotient `+1`.

## Bug

The Arith AIR (`state-machines/arith/pil/arith.pil`) proves signed div/rem with the identity
`quotient*divisor + remainder = dividend` (`eq[]` + carries) plus a lookup that fixes the flag tuple
`(na, nb, np, nr)` = signs of `(quotient, divisor, dividend, remainder)`. Nothing pins the **quotient
sign** `na` to the operand signs: for a nonzero quotient the correct relation is `na = np ^ nb`, but
the circuit never enforces it.

So for `DIV(1, -1)` a prover can set the quotient to `+1` (`na=0`) instead of `-1` (`na=1`), keeping
`dividend=1 (np=0)`, `divisor=-1 (nb=1)`, `remainder=0 (nr=0)`:

- The eq identity still balances -- carries are `[-1, -1, -1, -1, 0, 0, 0]`.
- The flag row `(na=0, nb=1, np=0, nr=0)` is a **legal** `arith_table` entry: it is honestly realized
  whenever the quotient truncates to `0` (e.g. `DIV(1, -2) = 0` rem `1`), so the table cannot exclude
  it. (`range_ab=5, range_cd=4`.)
- The remainder bound `|remainder| < |divisor|` is `|0| < |-1|`, which holds.

All three gates pass, so `rd = +1` is accepted for `1 / -1`.

## Relationship to prior findings

- **Distinct from the div/rem remainder bug** (remainder *magnitude* via Binary `OP_LT_ABS_NP`). That
  is a different constraint; here the remainder is `0` and genuinely in range.
- **Same family as the signed-MUL product-sign forgery, but a separate instance.** That fix pins
  `np = na ^ nb` for multiplication and is gated `signed * (1 - div)` -- **mul only**, so it does not
  cover this. Upstream's `rz` / `div_overflow_mul_rz` rework likewise only closed the MUL zero-product
  sign case (`rz = div_overflow_mul_rz * (1 - div)`, mul-gated); the division quotient-sign gap
  survives it. `arith.pil` is byte-identical across `v1.0.0-alpha`, `1.1.0-alpha`, `1.2.0-alpha`, and
  `develop`, so this reproduces unchanged on the current tip.

## The malicious witness (two coordinated injections, env `ZISK_REPRO_BAD_ARITH_DIV_SIGN`)

Main SM and Arith SM are tied by the operation bus (which carries the result), so a single forged
`DIV` has two halves that must agree:

- `core/src/ops_core.rs` (`op_div`) -- forge the Main/result: return `+1` for `DIV(1, -1)`.
- `state-machines/arith/src/arith_full.rs` (`maybe_inject_bad_div_sign_repro`) -- forge the Arith row:
  quotient `a = [1,0,0,0]`, `na = 0`, carries `[-1,-1,-1,-1,0,0,0]`, `range_ab = 5`, `range_cd = 4`.

Both are gated on the env var and the exact target op (`DIV(1, -1)`); honest runs are untouched. Every
other lane, the range checks, the arith_table lookup and the remainder-bound lookup stay honestly
satisfied -- that is what makes the forged row an *accepted* witness.

## Reproduce

Build `cargo-zisk` from this branch (the injections are compiled in), then:

```bash
riscv64-unknown-elf-gcc -march=rv64im -mabi=lp64 -nostdlib -nostartfiles -static \
    -Ttext=0x80000000 elf-regressions/arith_bad_div_sign/test.s -o /tmp/div.elf

# The bug: the forged sign-flipped quotient is ACCEPTED by the stock circuit.
ZISK_REPRO_BAD_ARITH_DIV_SIGN=1 cargo-zisk verify-constraints --elf /tmp/div.elf --emulator -k ~/.zisk/provingKey
# and the full proof verifies:
ZISK_REPRO_BAD_ARITH_DIV_SIGN=1 cargo-zisk prove --elf /tmp/div.elf --emulator -k ~/.zisk/provingKey --gpu --verify-proofs

# Honest contrast (no env var), same ELF: rd = -1, also passes.
cargo-zisk verify-constraints --elf /tmp/div.elf --emulator -k ~/.zisk/provingKey
```

## Verified (stock v1.0.0-alpha proving key)

End-to-end GPU run against the stock v1.0.0-alpha proving key:

- `prove --verify-proofs --gpu` + env var: the recursive **Vadcop Final proof verifies** -- the stock
  circuit accepts `DIV(1, -1) = +1`, a result that is the negation of the true quotient.
- Patched key (guard added), `verify-constraints` + env var: **REJECTED** -- only the guard fires
  (`arith.pil` constraint `na = np ^ nb`; every other AIR and all global constraints still pass).
- Patched key, honest (no env var): **PASSED** -- the real `DIV(1, -1) = -1` trace still satisfies
  every constraint.

## Diagnostic guard

The guard added to `arith.pil` is the div analogue of the MUL np-sign fix -- for a nonzero quotient
it pins `na = np ^ nb`:

```
div * (1 - div_overflow_mul_rz) * (a[0] + a[1] + a[2] + a[3]) * (na - (np + nb - 2 * np * nb)) === 0;
```

Inert on honest traces (the `a`-limb sum is `0` for a zero quotient; `(1 - div_overflow_mul_rz)`
exempts the `MIN/-1` overflow case; unsigned div has `na=np=nb=0`). Rebuilding the proving key from the
patched PIL **rejects** the malicious witness and **passes** the honest trace.

## Scope

Proves acceptance of a forged witness (sign-flipped `DIV` result), nothing beyond that.
