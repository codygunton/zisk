# DIVW Intmin Overflow

Create a small repro branch for issue #7 showing that `DIVW(INT_MIN_32, -1)` makes the emulator commit `0x0000000080000000` while the Arith state machine expects the sign-extended `0xFFFFFFFF80000000`. The branch should mirror `cg/auipc-rv64-overflow`: assembly fixture, README analysis, helper scripts if needed, and a scoped diagnostic comment near the relevant circuit path. Verification should prove that the fixture builds and that the branch diff is limited to the repro material.
