# Handoff: Raw Witness Bundle Repro

This branch currently demonstrates the Arith signed-MUL bug by env-gating two
normal witness-generation paths:

- `core/src/zisk_ops.rs::op_mul` makes Main claim
  `MUL(0xffffffffffffffff, 1) = 1`.
- `state-machines/arith/src/arith_full.rs` emits a matching malformed Arith row.

That is enough to show that `verify-constraints --emulator` and
`prove --verify-proofs` accept a globally balanced malicious trace, but it is
not the cleanest disclosure artifact. The current repro can be misread as an
executor bug because the proof goes through the ELF entrypoint and patches
executor-side semantics.

## Goal

Build a raw-witness-bundle repro path that bypasses the ELF executor/witness
generator and feeds hand-authored or serialized `AirInstance` traces directly
to proofman.

The target disclosure statement should be:

> This witness bundle is accepted by the ZisK proving system, but its Main trace
> witnesses `MUL(-1, 1) = 1` and its Arith trace locally satisfies the Arith
> constraints.

## Relevant internal boundary

The right in-memory boundary already exists:

- ZisK witness generation constructs `proofman_common::AirInstance<F>` values.
- The executor/witness pipeline registers those values with
  `ProofCtx::add_air_instance(air_instance, global_id)`.
- Proofman then verifies constraints or generates proofs from the registered
  instances.

Useful code locations:

- `prover-backend/src/prover/backend.rs`
  - `verify_constraints`
  - `prove`
- `executor/src/executor.rs`
  - `calculate_witness`
- `executor/src/witness_orchestrator.rs`
  - `compute_witness_for_instance`
- `executor/src/witness_generator.rs`
  - `compute_main_witness`
  - `compute_secn_witness`
- pil2-proofman dependency:
  - `common/src/air_instance.rs`
  - `common/src/proof_ctx.rs::add_air_instance`
  - `proofman/src/proofman.rs::{verify_proof_constraints_from_lib,generate_proof_from_lib}`

## Suggested implementation

Add a narrow repro-only command or binary, for example:

```text
cargo-zisk verify-witness-bundle --bundle repro-bundle.json -k /path/to/provingKey
cargo-zisk prove-witness-bundle --bundle repro-bundle.json -k /path/to/provingKey --verify-proofs
```

The bundle format can be JSON for readability. It only needs to support this
small repro initially:

```json
{
  "instances": [
    {
      "global_id": 0,
      "airgroup_id": 0,
      "air_id": 12,
      "num_rows": 8,
      "n_cols_trace": 123,
      "trace": ["0", "1", "..."],
      "airvalues": [],
      "airgroup_values": [],
      "custom_commits_fixed": []
    }
  ]
}
```

Do not rely on the exact numbers above; inspect the initialized proving key and
the existing `AirInstance` values from the current repro run.

## Practical approach

1. Add a temporary dump mode around `ProofCtx::add_air_instance` or the ZisK
   witness generator to serialize the accepted malicious `AirInstance`s.
2. Check in a minimized bundle or a generator for the bundle.
3. Add the replay command that initializes proofman from the proving key,
   deserializes each instance, calls `pctx.add_air_instance`, and runs the same
   constraint/proof path.
4. Verify both:
   - constraint acceptance
   - `prove --verify-proofs` equivalent acceptance

## Acceptance criteria

- The final repro command does not patch `core/src/zisk_ops.rs::op_mul`.
- The final repro command does not run the ELF executor to compute the bad row.
- The bad Main/Arith rows are visible in the bundle or generated from a small
  explicit witness-construction program.
- The docs clearly distinguish:
  - constraint acceptance
  - proof generation
  - proof verification
- The command exits successfully and logs final proof verification.

## Current baseline

Current branch head before this handoff:

```text
c36a02e27 repro: add dockerized arith mul flow
cfdd005d0 repro: document arith mul malicious witness
790f9e28a v0.18.0
```

The Docker repro already confirms:

- `verify-constraints --emulator` accepts the malicious trace.
- `prove --verify-proofs` accepts and logs `Vadcop Final proof was verified`.
