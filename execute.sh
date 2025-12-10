#!/bin/bash

./target/release/ziskemu \
    -e zksync-os/zksync_os/zksync_os_for_zisk.elf \
    -i /tmp/witness/22244135_witness.bin \
    -v
