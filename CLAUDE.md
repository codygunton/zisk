ZKsyncOS (in zksync-os/) is a "guest program" for proving EVM execution using Airbender (in zksync-airbender/), a ZKVM targeting RV32IM + a limited set of csrrw instructions used to interact with external oracles. We are adapting that guest program to run in ZisK, which targets RV64IMAFDC.

We have already achieved a primary goal using ZisK to execute real Ethereum blocks in ZKsyncOS as built for RV64IMAC. We did this on the branches

% git rev-parse HEAD         ~/zisk zksyncos
e11018e4af2176a4599ed9ba91f028743c91e782 
% git rev-parse HEAD         ~/zisk/zksync-os zisk-integration
11c3a4f5f8ab24c90964134568096bcf260cb332
% git rev-parse HEAD         ~/zisk/zksync-airbender zisk-integration
c55b84d1840194f29953a4e37495e9372080f28c


The cycle benchign/testing scripts
./bench-zisk-cycles.sh 
and
./bench-airbender-cycles.sh
which produce logs 
/tmp/airbender-bench.log
and
/tmp/zisk-bench.log
confirm this.

However, on these branches, the storage model uses a blake2s tree, whereas for real Ethereum state transitions we should use the real MPT. Our goal is to change that. We have leared that  using repositories as in this table:
  | Repository       | Branch                       |
  |------------------|------------------------------|
  | zksync-os        | popzxc/more-ethproofs-fusaka |
  | zksync-airbender | dev                          |

we _could_ have built our ZisK integration to use the Keccak MPT of Ethereum. The only real different _should_ be that we implement Keccak delegation rather than Blake2s delegation. This we will have to fix after rebasing.

The goal of our rebase are: 
  1) execute bench-airbender-cycles.sh successfully
  2) execute bench-zisk-cycles.sh at least as far as hitting some failing instruction, which will likely be a Keccak delegation instruction. This is
  csrrw x0, 0x7CB, x0 where:
  - CSR address 0x7CB = NON_DETERMINISM_CSR (0x7C0) + 11
  - The state pointer is passed in register x11
  - The control value is tracked in register x10
