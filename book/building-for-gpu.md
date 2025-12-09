Need gcc version < 15 because:
 The issue is CUDA 12.9 incompatibility with GCC 15's C++23 features.

  Specifically, GCC 15 introduced __is_pointer as a built-in type trait (part of C++23),
  which conflicts with CUDA's type_traits header. When nvcc compiles with GCC 15 as the
  host compiler, you get errors like:

  /usr/include/c++/15.2.1/type_traits(554): error: type name is not allowed

  The __is_pointer identifier is now reserved by GCC 15, but CUDA's headers still try to
  use it as a template name.

  GCC 13 doesn't have this conflict because it predates the C++23 type trait builtins.

Solution (in my case):
  One-time system setup:
  sudo ln -s /opt/cuda /usr/local/cuda
  sudo pacman -S intel-oneapi-openmp

  Build command:
  MAKEFLAGS="CXX=/usr/bin/g++-13" \
  LIBRARY_PATH="/opt/intel/oneapi/compiler/2025.0/lib:$LIBRARY_PATH" \
  PROOFMAN_HOST_COMPILER_BIN=/usr/bin/g++-13 \
  cargo build --release --features gpu


Get the proving key:
  curl -L -o zisk-provingkey.tar.gz \
  "https://storage.googleapis.com/zisk-setup/zisk-provingkey-pre-0.14.0.tar.gz"
  mkdir -p .proving_key
  tar -xf zisk-provingkey.tar.gz -C .

  ./target/release/cargo-zisk prove \
      --elf emulator/benches/data/my.elf \
      --input emulator/benches/data/input_one_segment.bin \
      --witness-lib ./target/release/libzisk_witness.so \
      --proving-key ./provingKey \
      -r \
      -t 4 \
      -v \
      -u 


## Issues?
If Error: Error executing Prove command

Caused by:
    Path does not exist: "/home/cody/.zisk/cache/my-308a56d739014f3f5ea304702c0f2a49e73068a
196843791b9bce637a4aa8142-mt.bin"

You need to regenerate the cache/constant files after switching versions. Run:

  ./target/release/cargo-zisk check-setup -a

This generates the constant tree files (the -mt.bin files it's looking for). It takes a few minutes.

Or maybe it was 

  sudo rm -rf ~/.zisk/zisk/emulator-asm/build

  Then run rom-setup again:

  ./target/release/cargo-zisk rom-setup \
      --elf emulator/benches/data/my.elf \
      --proving-key ~/.zisk/provingKey \
      -v

rm -rf ~/.zisk/cache/*
./target/release/cargo-zisk rom-setup \
      --elf emulator/benches/data/my.elf \
      --proving-key ~/.zisk/provingKey \
      -v


  # Delete the old GPU library
  rm -rf ~/.cargo/git/checkouts/pil2-proofman-*/*/pil2-stark/lib-gpu
  rm -rf ~/.cargo/git/checkouts/pil2-proofman-*/*/pil2-stark/build-gpu

To skip the AOT step:
 sudo -E HOME=/home/cody ./target/release/cargo-zisk prove -e
  ./elf-regressions/prebuilt-elfs/go-program-hello-world.elf --witness-lib
  ./target/release/libzisk_witness.so --proving-key ./provingKey -l -t 4 -v

Good working command:
sudo -E HOME=/home/cody ./target/release/cargo-zisk prove -e  zisk-testvectors/pessimistic-proof/elf/pp-keccakf.elf -i  zisk-testvectors/pessimistic-proof/inputs/pp_input_1_1.bin --witness-lib  ./target/release/libzisk_witness.so --proving-key ./provingKey -t 4 -v

  To summarize what was needed for GPU proving with ASM microservices on your RTX 5090:

  1. Build with CUDA sm_120: CUDA_ARCH=sm_120 (already done)
  2. Memory locking: Run with sudo (or set ulimit -l unlimited properly in
  /etc/security/limits.conf)
  3. HOME path: Use sudo -E HOME=/home/cody to preserve the ~/.zisk/cache path
  4. Real test program: Simple test ELFs don't generate memory metrics - use
  zisk-testvectors programs with actual inputs

  The working command:
sudo -E HOME=/home/cody ./target/release/cargo-zisk prove \
    -e zisk-testvectors/pessimistic-proof/elf/pp-keccakf.elf \
    -i zisk-testvectors/pessimistic-proof/inputs/pp_input_1_1.bin \
    --witness-lib ./target/release/libzisk_witness.so \
    --proving-key ./provingKey \
    -t 4 -v

  To avoid needing sudo every time, add to /etc/security/limits.conf:
  cody hard memlock unlimited or some number (unit is kilobytes)
  cody soft memlock unlimited or some number (unit is kilobytes)
  Then start a new terminal session.

So the final working command (since I have upped my limits) is 
./target/release/cargo-zisk prove \
    -e zisk-testvectors/pessimistic-proof/elf/pp-keccakf.elf \
    -i zisk-testvectors/pessimistic-proof/inputs/pp_input_1_1.bin \
    --witness-lib ./target/release/libzisk_witness.so \
    --proving-key ./provingKey \
    -t 4 -v

