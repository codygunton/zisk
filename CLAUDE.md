ZKsyncOS (in zksync-os/) is a "guest program" for proving EVM execution using Airbender (in zksync-airbender/), a ZKVM targeting RV32IM + a limited set of csrrw instructions used to interact with external oracles. We are adapting that guest program to run in ZisK, which targets RV64IMAFDC.

We are currently debugging **block 19299001**. The benchmark scripts should be configured to run this block.


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

DO NOT FORGET: We have had trouble with memory corruption, even during simple copies, due to what seem like compiler misoptimizations. These are fixed using volatile reads and writes. We are tracking these issues in @ai_plans/riscv-compiler-bugs.md.

DEBUGGING CHECKLIST (STOP if you skip a step):
1. Add logging to suspected area
2. Run bench-airbender-cycles.sh (~55s)
3. Run bench-zisk-cycles.sh (~55s)
4. DIFF the logs - find FIRST divergence point
5. Repeat 1-4 until isolated to specific function/line
6. ONLY THEN consider volatile workaround

NEVER add volatile read/write workarounds until you have:
- Logged output from BOTH Airbender AND Zisk
- Identified the EXACT line where they first diverge
- Documented the evidence in ai_plans/riscv-compiler-bugs.md

ALSO: running the zisk benching script only takes about 55s, so you should not use long 3-5 min timeouts and waiting periods needlessly.

You are running on a capable machine and you should make use of parallelism freely.
