#!/bin/bash

TAIL_N="${TAIL_N:-64}" # Default to mixed mode with custom mutators

./execute.sh 2>&1 | grep "ctx.pc=" | cut -d = -f3 > zisk-trace

python -c '
with open("zisk-trace") as f:
    pcs = [int(x, 16) for x in f]
    for pc0, pc1 in zip(pcs, pcs[1:]):
        print(pc1-pc0)
' > pc-diffs

addr_file=./zisk-trace
diff_file=./pc-diffs

# Prints address lines where corresponding diff != 4
paste "$addr_file" "$diff_file" \
    | awk '{
  # $1 = address, $2 = diff (supports decimal or 0x... hex)
  d = $2
  if (d ~ /^0[xX]/) d = strtonum(d)
  if (prev_d != 4 && NR > 1) print $1
  prev = $1; prev_d = d
} END { if (prev_d != 4) print "EOF" }' > jumps

# Use TAIL_N to only look at last N jumps (default: all)
if [ -n "$TAIL_N" ]; then
    tail -n "$TAIL_N" jumps > jumps.tmp && mv jumps.tmp jumps
fi

riscv64-elf-objdump -d zksync-os/zksync_os/zksync_os_zisk.elf > zisk.dump

# Load dump labels into awk hash, then lookup each jump (single pass through dump)
awk 'NR==FNR && /^[0-9a-f]+ </ { labels[$1]=$0; next }
     NR!=FNR && !/^EOF$/ { a=sprintf("%016x",strtonum($1)); if(a in labels) print labels[a] }' \
    zisk.dump jumps
