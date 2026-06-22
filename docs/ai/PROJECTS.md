# DIVW Intmin Overflow

Create a small repro branch for issue #7 showing the historical `DIVW(INT_MIN_32, -1)` emulator/PIL mismatch, following the `cg/auipc-rv64-overflow` branch structure. The branch has now been rebased onto `cg/repro-base-v1.0.0-alpha` for ZisK team review. On `v1.0.0-alpha`, both the overflow and safe fixtures build and prove successfully, so this repro branch now acts as a regression check for an apparent alpha fix.
