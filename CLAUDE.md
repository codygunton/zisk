ZKsyncOS (in zksync-os/) is a "guest program" for proving EVM execution using Airbender (in zksync-airbender/), a ZKVM targeting RV32IM + a limited set of csrrw instructions used to interact with external oracles. We are adaupting that guest program to run in ZisK, which targets RV64IMAFDC.


We care about reproducibility and maintainability of our code. We have bash scripts that we use for building and running the software. Our main debugging tool at the moment is to run
./bench-zisk-cycles.sh 
and/or
./bench-airbender-cycles.sh
and then inspect the logs
/tmp/airbender-bench.log
and/or
/tmp/zisk-bench.log
and sometimes
zksync-os/zksync_os/zksync_os_zisk.dump

We can track progress by running list-txs.sh with flags to count the number of transactions that don't revert.

You are running on a capable machine and you should make use of parallelism freely.

