#!/bin/bash

TAIL_N="${TAIL_N:-100000}" # Default to last 100k jumps
DEBUG_ELF="zksync-os/zksync_os/zksync_os_zisk_debug.elf"
PROD_ELF="zksync-os/zksync_os/zksync_os_zisk.elf"
DUMP_FILE=/tmp/zisk.dump

# Rebuild debug ELF if requested (takes ~3 minutes)
# Usage: REBUILD=1 ./qnd-stack-trace.sh
if [ -n "$REBUILD" ]; then
    echo "Rebuilding debug ELF (this takes ~3 minutes)..."
    (cd zksync-os/zksync_os && ./build.sh --machine zisk --debug)
fi

# Check debug ELF exists if SYMBOLS mode requested
if [ -n "$SYMBOLS" ] && [ ! -f "$DEBUG_ELF" ]; then
    echo "Error: Debug ELF not found at $DEBUG_ELF"
    echo "Run with REBUILD=1 to create it (takes ~3 minutes)"
    exit 1
fi

# Use /tmp for intermediate files
TRACE_FILE=/tmp/zisk-trace
DIFF_FILE=/tmp/pc-diffs
JUMPS_FILE=/tmp/jumps

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

if [ -n "$SYMBOLS" ]; then
    # Use demangled objdump for function names + addr2line.sh for source locations
    riscv64-elf-objdump --demangle -d "$PROD_ELF" > "$DUMP_FILE"

    # Build symbol tables for address translation
    PROD_SYMBOLS=/tmp/prod_symbols
    DEBUG_SYMBOLS=/tmp/debug_symbols
    riscv64-elf-nm -n "$PROD_ELF" > "$PROD_SYMBOLS"
    riscv64-elf-nm "zksync-os/zksync_os/zksync_os_zisk_debug.elf" > "$DEBUG_SYMBOLS"

    # Process jumps with symbol-relative address translation
    awk -v prod_sym="$PROD_SYMBOLS" -v debug_sym="$DEBUG_SYMBOLS" -v debug_elf="zksync-os/zksync_os/zksync_os_zisk_debug.elf" '
    BEGIN {
        # Load production symbols (sorted by address)
        while ((getline line < prod_sym) > 0) {
            split(line, parts, " ")
            addr = strtonum("0x" parts[1])
            name = parts[3]
            if (name != "") {
                prod_addrs[prod_n] = addr
                prod_names[prod_n] = name
                prod_n++
            }
        }
        # Load debug symbols into lookup table
        while ((getline line < debug_sym) > 0) {
            split(line, parts, " ")
            addr = strtonum("0x" parts[1])
            name = parts[3]
            if (name != "") debug_addr[name] = addr
        }
    }
    NR==FNR && /^[0-9a-f]+ </ && !/\.LBB/ && !/\.L[^a-z]/ {
        addr = strtonum("0x" $1)
        dump_addrs[dump_n] = addr
        dump_labels[addr] = $0
        dump_n++
        next
    }
    NR!=FNR && !/^EOF$/ {
        target = strtonum($1)

        # Find containing function from objdump labels
        best = -1
        for (i = 0; i < dump_n; i++) {
            if (dump_addrs[i] <= target && dump_addrs[i] > best) best = dump_addrs[i]
        }

        if (best >= 0) {
            current = dump_labels[best]
            if (current != prev) {
                if (skipped > 0) print "  ..."
                printf "%x\n  in %s\n", target, current

                # Find containing function from nm symbols for addr2line
                func_addr = -1
                func_name = ""
                for (i = 0; i < prod_n; i++) {
                    if (prod_addrs[i] <= target) {
                        func_addr = prod_addrs[i]
                        func_name = prod_names[i]
                    } else break
                }

                if (func_name != "" && func_name in debug_addr) {
                    offset = target - func_addr
                    debug_target = debug_addr[func_name] + offset
                    # Use -i flag to show inlined function frames
                    cmd = "addr2line -e \"" debug_elf "\" -C -i " sprintf("0x%x", debug_target) " 2>/dev/null"
                    # Read all lines from addr2line (multiple frames with -i)
                    first_frame = 1
                    while ((cmd | getline src_loc) > 0) {
                        if (src_loc !~ /\?\?:/ && src_loc !~ /:0$/) {
                            if (first_frame) {
                                print "     at " src_loc
                                first_frame = 0
                            } else {
                                print "        inlined from " src_loc
                            }
                        }
                    }
                    close(cmd)
                }
                skipped = 0
            } else {
                skipped++
            }
            prev = current
        }
    }
    END { if (skipped > 0) print "  ..." }' "$DUMP_FILE" "$JUMPS_FILE"
else
    # Fast mode: use objdump labels (no debug symbols needed)
    riscv64-elf-objdump --demangle -d "$PROD_ELF" > "$DUMP_FILE"

    # Load dump labels into sorted array, find nearest preceding label for each jump
    # Filter out .LBB labels, deduplicate consecutive identical labels
    awk '
    NR==FNR && /^[0-9a-f]+ </ && !/\.LBB/ {
        addr = strtonum("0x" $1)
        addrs[n] = addr
        labels[addr] = $0
        n++
        next
    }
    NR!=FNR && !/^EOF$/ {
        target = strtonum($1)
        # Find largest label addr <= target
        best = -1
        for (i = 0; i < n; i++) {
            if (addrs[i] <= target && addrs[i] > best) best = addrs[i]
        }
        if (best >= 0) {
            current = labels[best]
            if (current != prev) {
                if (skipped > 0) print "  ..."
                printf "%x\n  in %s\n", target, current
                skipped = 0
            } else {
                skipped++
            }
            prev = current
        }
    }
    END { if (skipped > 0) print "  ..." }' "$DUMP_FILE" "$JUMPS_FILE"
fi
