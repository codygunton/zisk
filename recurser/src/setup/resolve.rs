//! Re-exports of pil2-proofman's path-resolution helpers.
//!
//! We deliberately do NOT keep local copies — pil2-proofman bakes
//! `CARGO_MANIFEST_DIR` at compile time inside its own crate so the fallback
//! resolves correctly even when pil2-proofman is consumed as a git dependency
//! (which is the zisk case). A local copy here would bake zisk's manifest
//! dir instead, breaking the fallback.

pub(super) use pil2_stark_setup::commands::recursive_setup::{
    resolve_circom_exec, resolve_path_env,
};
