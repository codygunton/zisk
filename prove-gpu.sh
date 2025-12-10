#!/bin/bash

./target/release/cargo-zisk prove \
    -e zksync-os/zksync_os/zksync_os_for_zisk.elf \
    -i /tmp/witness/22244135_witness.bin \
    --witness-lib ./target/release/libzisk_witness.so \
    --proving-key ./provingKey \
    -t 4 \
    -v
