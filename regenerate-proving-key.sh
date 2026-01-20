#!/bin/bash
set -e

# Regenerate ZisK Proving Key
#
# This script regenerates the proving key when new AIRs are added to the PIL.
# It requires the pil2 toolchain to be installed in ~/workspace/
#
# Prerequisites:
# - pil2-compiler at ~/workspace/pil2-compiler (npm install done)
# - pil2-proofman-js at ~/workspace/pil2-proofman-js (npm install done)
# - pil2-proofman at ~/workspace/pil2-proofman
#
# Output:
# - ./provingKey/ directory with regenerated proving key

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$REPO_ROOT"

# Source Intel OneAPI environment (required for linking)
if [[ -f /opt/intel/oneapi/setvars.sh ]]; then
    source /opt/intel/oneapi/setvars.sh > /dev/null 2>&1
fi

# Set up library paths for Intel OpenMP and Rust std
RUST_STD_PATH="$(rustc --print sysroot)/lib/rustlib/x86_64-unknown-linux-gnu/lib"
export LD_LIBRARY_PATH="$RUST_STD_PATH:/opt/intel/oneapi/compiler/2025.0/lib:${LD_LIBRARY_PATH:-}"
export LIBRARY_PATH="/opt/intel/oneapi/compiler/2025.0/lib:${LIBRARY_PATH:-}"

WORKSPACE_DIR="${HOME}/workspace"
PIL2_COMPILER="${WORKSPACE_DIR}/pil2-compiler"
PIL2_PROOFMAN_JS="${WORKSPACE_DIR}/pil2-proofman-js"

# Find pil2-proofman in cargo cache (matches version in Cargo.toml)
PIL2_PROOFMAN=$(find ~/.cargo/git/checkouts/pil2-proofman-* -maxdepth 2 -name "pil2-components" -type d 2>/dev/null | head -1 | xargs dirname)

echo "=== ZisK Proving Key Regeneration ==="
echo "Repository: $REPO_ROOT"
echo ""

# Check prerequisites
check_prereq() {
    local name="$1"
    local path="$2"
    if [[ ! -d "$path" ]]; then
        echo "ERROR: $name not found at $path"
        echo "Please clone it first or run tools/test-env/build_setup.sh"
        exit 1
    fi
    echo "Found $name at $path"
}

check_prereq "pil2-compiler" "$PIL2_COMPILER"
check_prereq "pil2-proofman (cargo cache)" "$PIL2_PROOFMAN"
check_prereq "pil2-proofman-js" "$PIL2_PROOFMAN_JS"
echo ""

# Step 1: Generate fixed data
echo "=== Step 1/5: Generating fixed data ==="
cargo run --release --bin keccakf_fixed_gen
cargo run --release --bin arith_frops_fixed_gen
cargo run --release --bin binary_basic_frops_fixed_gen
cargo run --release --bin binary_extension_frops_fixed_gen
echo ""

# Step 2: Compile PIL
echo "=== Step 2/5: Compiling ZisK PIL ==="
mkdir -p tmp/fixed
node "${PIL2_COMPILER}/src/pil.js" pil/zisk.pil \
    -I pil,"${PIL2_PROOFMAN}/pil2-components/lib/std/pil",state-machines,precompiles \
    -o pil/zisk.pilout \
    -u tmp/fixed \
    -O fixed-to-file
echo ""

# Step 3: Generate setup (proving key)
echo "=== Step 3/5: Generating proving key (this may take a while) ==="
rm -rf build/provingKey

# Non-recursive setup (recursive has a bug in pil2-proofman-js v0.14.0)
# To try recursive setup, set ENABLE_RECURSIVE_SETUP=1
if [[ "${ENABLE_RECURSIVE_SETUP}" == "1" ]]; then
    echo "Running recursive setup..."
    node "${PIL2_PROOFMAN_JS}/src/main_setup.js" \
        -a ./pil/zisk.pilout \
        -b build \
        -u tmp/fixed \
        -t "${PIL2_PROOFMAN}/pil2-components/lib/std/pil" \
        -r
else
    echo "Running non-recursive setup..."
    node "${PIL2_PROOFMAN_JS}/src/main_setup.js" \
        -a ./pil/zisk.pilout \
        -b build \
        -u tmp/fixed
fi
echo ""

# Step 4: Copy to provingKey directory
echo "=== Step 4/5: Installing proving key ==="
rm -rf ./provingKey
cp -R build/provingKey ./provingKey
echo ""

# Step 5: Generate constant tree files for GPU proving
echo "=== Step 5/5: Generating constant tree files (for GPU proving) ==="
echo "Building cargo-zisk with GPU support..."
cargo build --release --features gpu -p cargo-zisk 2>&1 | tail -5
echo "Running check-setup..."
./target/release/cargo-zisk check-setup --proving-key ./provingKey
echo ""

# Show the AIRs in the new proving key
echo "=== Proving Key Generated ==="
echo "Location: ./provingKey/"
echo ""
echo "AIRs in the new proving key:"
if [[ -f "./provingKey/pilout.globalInfo.json" ]]; then
    node -e "
        const info = require('./provingKey/pilout.globalInfo.json');
        info.airs[0].forEach((air, idx) => {
            console.log('  ' + idx + ': ' + air.name + ' (' + air.num_rows + ' rows)');
        });
    "
fi
echo ""

echo "=== Done ==="
echo ""
if [[ "${ENABLE_RECURSIVE_SETUP}" != "1" ]]; then
    echo "NOTE: Non-recursive setup was used (recursive has a bug in pil2-proofman-js v0.14.0)."
fi
echo ""
echo "Next steps:"
echo "1. Run ROM setup: ./target/release/cargo-zisk rom-setup --elf <ELF> --proving-key ./provingKey"
echo "2. Rebuild and test: ./prove-block-gpu.sh"
