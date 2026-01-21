#!/bin/bash
# Regenerate traces.rs from PIL files
#
# This script:
# 1. Compiles PIL to pilout (requires ~180GB RAM)
# 2. Generates traces.rs from pilout
#
# Usage: ./regenerate-traces.sh

set -e

cd "$(dirname "$0")"

echo "=== Regenerating traces.rs from PIL ==="

# Find pil2-proofman components path
PIL2_PROOFMAN=$(find ~/.cargo/git/checkouts/pil2-proofman-* -maxdepth 2 -name "pil2-components" -type d | head -1 | xargs dirname)
if [ -z "$PIL2_PROOFMAN" ]; then
    echo "ERROR: Could not find pil2-proofman checkout"
    exit 1
fi
echo "PIL2_PROOFMAN=$PIL2_PROOFMAN"

# Find proofman-cli
PROOFMAN_CLI=$(find ~/.cargo/git/checkouts/pil2-proofman-* -name "proofman-cli" -type f -executable | head -1)
if [ -z "$PROOFMAN_CLI" ]; then
    echo "ERROR: Could not find proofman-cli binary"
    exit 1
fi
echo "PROOFMAN_CLI=$PROOFMAN_CLI"

# Step 1: Compile PIL to pilout
echo ""
echo "Step 1: Compiling PIL to pilout (this requires ~180GB RAM)..."
NODE_OPTIONS="--max-old-space-size=204800" node ~/workspace/pil2-compiler/src/pil.js pil/zisk.pil \
    -I pil,"${PIL2_PROOFMAN}/pil2-components/lib/std/pil",state-machines,precompiles \
    -o pil/zisk.pilout

echo ""
echo "Step 2: Backing up current traces.rs..."
BACKUP_FILE="/tmp/traces.rs.backup.$(date +%Y%m%d_%H%M%S)"
cp pil/src/pil_helpers/traces.rs "$BACKUP_FILE"
echo "  Backup: $BACKUP_FILE"

echo ""
echo "Step 3: Removing old pil_helpers directory..."
rm -rf pil/src/pil_helpers

echo ""
echo "Step 4: Regenerating pil_helpers from pilout..."
# Note: proofman-cli creates a pil_helpers/ subdirectory inside --path
"$PROOFMAN_CLI" \
    pil-helpers \
    --pilout /home/cody/zisk/pil/zisk.pilout \
    --path /home/cody/zisk/pil/src

echo ""
echo "=== Done ==="
echo "Generated: pil/src/pil_helpers/traces.rs"
echo ""
echo "Next steps:"
echo "  1. cargo build -p zisk-pil --release"
echo "  2. cargo build -p precomp-u256-delegation --release"
echo "  3. VERBOSE=1 ./prove-block-zisk.sh 24198369"
