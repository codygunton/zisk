# MemAlign narrow-load value-lane soundness repro (v1.0.0-alpha)

A prover can forge the **high 32 bits** of a narrow unsigned load. The stock circuit accepts an
`rd` value that was never present in memory — both `verify-constraints` and a full recursive STARK
proof (`prove --verify-proofs`).

## Bug

On the general `MemAlign` "prove" row, the returned value's high chunk is reconstructed as
(`state-machines/mem/pil/mem_align.pil`):

```
value[1] = Σ_offset sel[offset] · Σ_ichunk reg[(offset + 4 + ichunk) % 8] · 256^ichunk
```

For a width-4 offset-0 load (`sel[0]=1`) that is `value[1] = reg[4] + reg[5]·256 + reg[6]·256² +
reg[7]·256³`. Those `reg[4..7]` lie **outside the access window** and are constrained only by the
8-bit range check: the cross-row continuity gates (`sel_up_to_down`/`sel_down_to_up`) tie only the
in-window lanes `reg[0..3]`, and the `MemAlignRom` lookup pins the row structure but no data lane. So
nothing forces `value[1] = 0`. Since `lwu -> copyb` and `op_copyb(_a,b)=(b,false)` copy the loaded
value to `rd` unmodified (Main AIR `c[i]=b[i]`), `reg[4]=1 ⇒ value[1]=1 ⇒ rd = 0x0000_0001_0000_0000`.

**`lbu` is NOT exploitable here.** 1-byte reads use the separate `MemAlignByte` machine
(`mem_align_byte.pil`), whose prove row is `[LOAD, addr, step, 1, byte_value, 0]` — the high chunk is
a literal `0` and `byte_value` a `byte(8)` column, so byte loads are pinned. The collector
(`mem_align_collector.rs`) routes width 1 → MemAlignByte (safe) and width 2/4 + unaligned width 8 →
general MemAlign (this gap). So the fixture uses `lwu`; `lhu` is equivalent.

## The malicious witness (two coordinated injections, env `ZISK_REPRO_BAD_MEM_ALIGN_LWU`)

Main SM and MemAlign SM are tied by the `MEMORY_ID` permutation, which includes the value, so a
single forged load has two halves that must agree:

- `core/src/mem.rs` (`get_single_not_aligned_data`) — forge the Main/load result: `word | (1<<32)`.
- `state-machines/mem/src/mem_align_sm.rs` (`prove_mem_align_op`, one-read branch) — forge the
  MemAlign prove row: `reg[4]=1 ⇒ value[1]=1`.

Both are gated on the env var and the exact target access (`0xa0030000`, width 4); honest runs are
untouched. Every other lane, the byte range check, the ROM lookup and the read-row continuity stay
honestly satisfied — that is what makes the forged row an *accepted* witness.

## Reproduce

Build `cargo-zisk-dev` from this branch (the injections are compiled in), then:

```bash
riscv64-elf-as -march=rv64ima elf-regressions/mem_align_bad_lwu/test.s -o /tmp/t.o
riscv64-elf-ld  -Ttext=0x80000000 /tmp/t.o -o /tmp/lwu.elf

# The bug: the forged narrow-load witness is ACCEPTED by the stock circuit.
ZISK_REPRO_BAD_MEM_ALIGN_LWU=1 cargo-zisk-dev verify-constraints --elf /tmp/lwu.elf -k ~/.zisk/provingKey
# and the full proof verifies:
ZISK_REPRO_BAD_MEM_ALIGN_LWU=1 cargo-zisk-dev prove --elf /tmp/lwu.elf -k ~/.zisk/provingKey --gpu --verify-proofs

# Honest contrast (no env var), same ELF: rd = 0x00000000, also passes.
cargo-zisk-dev verify-constraints --elf /tmp/lwu.elf -k ~/.zisk/provingKey
```

## Verified (stock v1.0.0-alpha proving key)

- `verify-constraints` + env var: PASSES — MemAlign AIR + all global constraints accept `value[1]=1`.
- `prove --verify-proofs --gpu` + env var: the recursive **Vadcop Final proof verifies**.
- honest (no env var): PASSES.

## Diagnostic guard

The one-line guard added to `mem_align.pil`, `sel_prove * (width - 8) * value[RC-1] === 0`, pins
`value[1]=0` for narrow accesses (inert on honest traces; width-8 vanishes the factor). Rebuilding
the proving key from the patched PIL and re-running `verify-constraints` **rejects** the malicious
witness (MemAlign AIR constraint) and **passes** the honest trace — verified. Diagnostic only, not a
proposed final fix (the real fix pins all out-of-window reconstruction lanes; ZisK's design choice).

## Scope

Proves acceptance of a forged witness (`rd` corruption), nothing beyond that. Distinct from the
adjacent, already-fixed MemAlign issues (PR #1116, #1142).
