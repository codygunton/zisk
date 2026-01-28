# ZisK/ZKsyncOS Docker Image
# NVIDIA CUDA base with GCC 14 for pil2-proofman compatibility

FROM nvidia/cuda:12.6.3-devel-ubuntu24.04

ENV DEBIAN_FRONTEND=noninteractive

# Install system dependencies including GCC 14
# GCC 14 is required because pil2-proofman has missing #include <cstdint> headers
# that cause compilation failures with GCC 15+
RUN apt-get update && apt-get install -y \
    build-essential \
    git \
    curl \
    wget \
    gcc-14 \
    g++-14 \
    cmake \
    make \
    libssl-dev \
    pkg-config \
    libffi-dev \
    libgmp-dev \
    nlohmann-json3-dev \
    libomp-dev \
    nasm \
    libsodium-dev \
    libopenmpi-dev \
    libclang-dev \
    clang \
    protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

# Install RISC-V toolchain
RUN apt-get update && apt-get install -y \
    gcc-riscv64-unknown-elf \
    binutils-riscv64-unknown-elf \
    && rm -rf /var/lib/apt/lists/*

# Set up environment variables for GCC 14
ENV CC=/usr/bin/gcc-14
ENV CXX=/usr/bin/g++-14
ENV GCC14=/usr/bin/gcc-14
ENV GXX14=/usr/bin/g++-14

# CUDA environment
ENV PATH="/usr/local/cuda/bin:${PATH}"
ENV LD_LIBRARY_PATH="/usr/local/cuda/lib64:${LD_LIBRARY_PATH}"

# Install Rust via rustup with nightly toolchain
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
    --default-toolchain nightly-2026-01-19

# Add Rust to PATH
ENV PATH="/root/.cargo/bin:${PATH}"

# Add components and RISC-V targets
RUN rustup component add rust-src llvm-tools-preview && \
    rustup target add riscv32i-unknown-none-elf riscv64imac-unknown-none-elf

# Install toolchain used by zksync-os with rust-src and llvm-tools
RUN rustup toolchain install nightly-2025-09-04 && \
    rustup component add rust-src llvm-tools-preview --toolchain nightly-2025-09-04 && \
    rustup target add riscv64imac-unknown-none-elf --toolchain nightly-2025-09-04

# Install cargo-binutils
RUN cargo install cargo-binutils

# Set working directory
WORKDIR /workspace

# Clone repository from remote
ARG REPO_URL=https://github.com/codygunton/zisk
ARG BRANCH=zksyncos
RUN git clone --recursive --branch ${BRANCH} ${REPO_URL} . && \
    git submodule update --init --recursive

# Pre-fetch cargo dependencies
RUN cargo fetch

# Build release binaries using setup script
RUN ./setup.sh

# Copy entrypoint script to final location
RUN cp docker-entrypoint.sh /usr/local/bin/ && \
    chmod +x /usr/local/bin/docker-entrypoint.sh

ENTRYPOINT ["/usr/local/bin/docker-entrypoint.sh"]
CMD ["--help"]
