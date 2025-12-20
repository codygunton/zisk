#!/bin/bash
set -e

# Extract and diff oracle queries between Airbender and Zisk benchmark logs
#
# Usage: ./get-query-diff.sh [airbender_log] [zisk_log]

AIR_LOG="${1:-/tmp/airbender-bench.log}"
ZISK_LOG="${2:-/tmp/zisk-bench.log}"

if [[ ! -f "$AIR_LOG" ]]; then
    echo "Error: Airbender log not found: $AIR_LOG"
    echo "Run ./bench-airbender-cycles.sh first"
    exit 1
fi

if [[ ! -f "$ZISK_LOG" ]]; then
    echo "Error: Zisk log not found: $ZISK_LOG"
    echo "Run ./bench-zisk-cycles.sh first"
    exit 1
fi

grep "\[oracle\] query" "$AIR_LOG" > /tmp/airbender-queries
grep "\[oracle\] query" "$ZISK_LOG" > /tmp/zisk-queries
diff -y /tmp/airbender-queries /tmp/zisk-queries > /tmp/query-diff || true

echo "Airbender queries: $(wc -l < /tmp/airbender-queries)"
echo "Zisk queries: $(wc -l < /tmp/zisk-queries)"
echo ""
echo "Diff saved to: /tmp/query-diff"
