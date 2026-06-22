# divw_intmin_overflow - DIVW INT_MIN / -1 mismatch

Two tests exercise `divw` on the low 32-bit value `0x80000000`.

```
divw_intmin_safe/      divw a2, a0, 1    -> 0xFFFFFFFF80000000  (emulator and Arith agree)
divw_intmin_overflow/  divw a2, a0, -1   -> emulator/Arith mismatch, proving fails
```

The failing case is issue #7: ZisK's fast emulator computes the signed DIVW overflow case at `i64` width, while RV64 DIVW wraps at 32 bits and then sign-extends the 32-bit quotient.

## The arithmetic

```
low32(a0) = 0x80000000 = INT_MIN_32 = -2147483648
low32(a1) = 0xFFFFFFFF = -1

RV64 DIVW:
  INT_MIN_32 / -1 overflows at 32 bits
  quotient wraps to 0x80000000
  DIVW sign-extends to XLEN
  result = 0xFFFFFFFF80000000

Current op_div_w:
  ((a as i32) as i64) / ((b as i32) as i64)
  (-2147483648_i64) / (-1_i64) = +2147483648
  as u64 = 0x0000000080000000
```

## Why proving fails

The Main state machine sends the emulator's `c` value to the operation bus. For this instruction it sends:

```
c[0] = 0x80000000
c[1] = 0x00000000
```

The Arith state machine recomputes the operation from `(op, a, b)`. In the real PIL path, a signed 32-bit DIVW result with `sext = 1` proves:

```
bus_res1 = 0xFFFFFFFF
```

The operation-bus tuple includes `c`, so the Main-side consumed tuple and Arith-side proved tuple differ on `c[1]`. The lookup cannot balance, making any valid program that executes this instruction unprovable.

## Reproducer

```
lui  a0, 0x80000        # low32(a0) = 0x80000000
li   a1, -1             # low32(a1) = 0xFFFFFFFF
divw a2, a0, a1         # expected 0xFFFFFFFF80000000, emulator writes 0x0000000080000000
```

Build and prove:

```
./build-elf elf-regressions/divw_intmin_overflow/test.s
./prove-elf elf-regressions/divw_intmin_overflow/test.elf
```

The proof is expected to fail on the buggy code. `elf-regressions/divw_intmin_safe/test.s` is a nearby non-overflow contrast case.
