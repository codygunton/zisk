## ⚠️ Disclaimer: Software Under Development ⚠️

This software is currently under **active development** and has not been audited for security or correctness.

Please be aware of the following:
* The software is **not fully tested**.
* **Do not use it in production environments** until a stable production release is available. 🚧
* Additional functionalities and optimizations **are planned for future releases**.
* Future updates may introduce breaking **backwards compatible changes** as development progresses.
* Mac is currently not supported.  We are working to support it soon.

If you encounter any errors or unexpected behavior, please report them. Your feedback is highly appreciated in improving the software.

# ZisK

ZisK is an innovative and high-performance zkVM (Zero-Knowledge Virtual Machine) developed by Polygon that enables trustless, verifiable computation, allowing developers to generate and verify proofs for arbitrary program execution efficiently.

ZisK aims to provide a flexible and developer-friendly zkVM, with Rust as its primary language for writing provable programs, with planned support for other languages in the future. By abstracting complex zero-knowledge proof generation, ZisK simplifies the integration of ZK technology into scalable, private, and secure applications across blockchain ecosystems and beyond.

## Getting Started

To start using ZisK, follow the [Quickstart](https://0xpolygonhermez.github.io/zisk/getting_started/quickstart.html) guide.

📚 Complete Documentation: [ZisK Docs](https://0xpolygonhermez.github.io/zisk/)

## Docker-based Proving (drun)

The `drun` script provides a Docker-based workflow for executing and proving Ethereum blocks with ZisK. This is useful for standalone proving without a full local development setup.

### Prerequisites

- Docker
- NVIDIA Container Toolkit (for GPU proving)

### Commands

```bash
# Build the Docker image (clones from remote, includes proving key generation)
./drun build

# Execute an Ethereum block with ZisK emulator
./drun execute

# Execute with Airbender instead
./drun execute -a

# Run GPU proving (requires NVIDIA GPU)
./drun prove

# Interactive shell for debugging
./drun shell
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `BLOCK` | `24198369` | Block number to execute/prove |
| `VERBOSE` | `0` | Enable verbose output |
| `REPO_URL` | `https://github.com/codygunton/zisk` | Git repository URL for build |
| `BRANCH` | `zksyncos` | Git branch to clone |

### Examples

```bash
# Execute a specific block
BLOCK=22244135 ./drun execute

# Prove with verbose output
BLOCK=24198369 VERBOSE=1 ./drun prove

# Build from a different branch
BRANCH=main ./drun build
```

### Notes

- The proving key is built into the Docker image during `./drun build`
- To override the built-in proving key, place your key in `./provingKey/` (it will be mounted read-only)
- GPU proving requires the NVIDIA Container Toolkit to be installed and configured

## License

All crates in this monorepo are licensed under one of the following options:

- The Apache License, Version 2.0 (see LICENSE-APACHE or http://www.apache.org/licenses/LICENSE-2.0)

- The MIT License (see LICENSE-MIT or http://opensource.org/licenses/MIT)

You may choose either license at your discretion.

## Acknowledgements

ZisK is a collaborative effort made possible by the contributions of researchers, engineers, and developers dedicated to advancing zero-knowledge technology.

We extend our gratitude to the [Polygon zkEVM](https://github.com/0xpolygonhermez) and [Plonky3](https://github.com/Plonky3/Plonky3) teams for their foundational work in zero-knowledge proving systems, as well as to the [RISC-V](https://github.com/riscv) community for providing a robust architecture that enables the zkVM model.

Additionally, we acknowledge the efforts of the open-source cryptography and ZK research communities, whose insights and contributions continue to shape the evolution of efficient and scalable zero-knowledge technologies.

🚀 Special thanks to all contributors who have helped develop, refine, and improve ZisK!