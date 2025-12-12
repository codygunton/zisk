#!/bin/bash
set -e

# Build zksync-os for QEMU and start debugger
#
# Usage: ./debug_qemu.sh [--rebuild]
#
# Output: zksync-os/zksync_os/zksync_os_qemu.elf
#
# Then connect GDB from another terminal

ZISK_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ZKSYNCOS_DIR="$ZISK_DIR/zksync-os/zksync_os"
OUTPUT_ELF="$ZKSYNCOS_DIR/zksync_os_qemu.elf"

# Check if rebuild needed
REBUILD=false
if [[ ! -f "$OUTPUT_ELF" ]]; then
    REBUILD=true
elif [[ "$1" == "--rebuild" ]]; then
    REBUILD=true
fi

if $REBUILD; then
    # Build with --clean to ensure fresh build with correct memory layout
    "$ZKSYNCOS_DIR/build.sh" qemu --clean
fi

# Kill any existing QEMU
pkill -9 qemu-system-risc 2>/dev/null || true
sleep 1

echo ""
echo "=== Starting QEMU ==="
echo "ELF: $OUTPUT_ELF"
echo ""
echo "Memory layout (memory-qemu.x):"
echo "  ROM: 0x80000000 (128M) - code"
echo "  RAM: 0x88000000 (512M) - stack/heap"
echo ""
echo "Debug symbols: function names only (no line numbers due to DWARF overflow)"
echo ""
echo "Note: QEMU cannot provide oracle data via CSR 0x7c0"
echo "      Program will fail when it queries the oracle"
echo "      Use this for inspecting startup, memory layout, registers"
echo ""

# Start QEMU waiting for GDB
qemu-system-riscv64 -machine virt -m 4G -nographic -bios none \
    -kernel "$OUTPUT_ELF" \
    -s -S &
QEMU_PID=$!
sleep 1

echo "QEMU started (PID: $QEMU_PID), waiting for GDB on port 1234"
echo ""
echo "=== Connect GDB in another terminal ==="
echo ""
echo "  riscv64-elf-gdb $OUTPUT_ELF -ex 'target remote :1234'"
echo ""
echo "=== Useful GDB commands ==="
echo ""
echo "  # Breakpoints"
echo "  break _start              # entry point"
echo "  break run_proving         # main proving function"
echo "  break unwrap_failed       # panic handler"
echo ""
echo "  # Execution"
echo "  continue                  # run until breakpoint"
echo "  si                        # step one instruction"
echo "  ni                        # step over function calls"
echo "  finish                    # run until function returns"
echo ""
echo "  # Inspection"
echo "  bt                        # backtrace"
echo "  info reg                  # show all registers"
echo "  info reg pc ra sp         # show specific registers"
echo "  x/10i \$pc                 # disassemble 10 instructions at PC"
echo "  x/10x \$sp                 # examine 10 words at stack pointer"
echo "  p/x \$a0                   # print register a0 in hex"
echo ""
echo "Press Ctrl+C to stop QEMU"
wait $QEMU_PID
