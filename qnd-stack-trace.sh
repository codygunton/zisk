#!/bin/bash

TAIL_N="${TAIL_N:-64}" # Default to last 64 jumps

# Use /tmp for intermediate files
TRACE_FILE=/tmp/zisk-trace
DIFF_FILE=/tmp/pc-diffs
JUMPS_FILE=/tmp/jumps
DUMP_FILE=/tmp/zisk.dump

./execute.sh 2>&1 | grep "ctx.pc=" | cut -d = -f3 > "$TRACE_FILE"

python -c "
with open('$TRACE_FILE') as f:
    pcs = [int(x, 16) for x in f]
    for pc0, pc1 in zip(pcs, pcs[1:]):
        print(pc1-pc0)
" > "$DIFF_FILE"

# Prints address lines where corresponding diff != 4
paste "$TRACE_FILE" "$DIFF_FILE" \
    | awk '{
  # $1 = address, $2 = diff (supports decimal or 0x... hex)
  d = $2
  if (d ~ /^0[xX]/) d = strtonum(d)
  if (prev_d != 4 && NR > 1) print $1
  prev = $1; prev_d = d
} END { if (prev_d != 4) print "EOF" }' > "$JUMPS_FILE"

# Use TAIL_N to only look at last N jumps (default: all)
if [ -n "$TAIL_N" ]; then
    tail -n "$TAIL_N" "$JUMPS_FILE" > "${JUMPS_FILE}.tmp" && mv "${JUMPS_FILE}.tmp" "$JUMPS_FILE"
fi

riscv64-elf-objdump -d zksync-os/zksync_os/zksync_os_zisk.elf > "$DUMP_FILE"

# Load dump labels into awk hash, then lookup each jump (single pass through dump)
awk 'NR==FNR && /^[0-9a-f]+ </ { labels[$1]=$0; next }
     NR!=FNR && !/^EOF$/ { a=sprintf("%016x",strtonum($1)); if(a in labels) print labels[a] }' \
    "$DUMP_FILE" "$JUMPS_FILE"
