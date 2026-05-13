//! Small path-resolution helpers — local re-implementation of the
//! `pub(crate)` helpers from pil2-stark-setup's `recursive_setup.rs`.
//!
//! Kept private to the `setup` module.

use std::path::Path;

/// Resolve the circom executable: prefer `<circom_helpers_dir>/circom[_mac]`,
/// fall back to a bare `circom` (PATH lookup).
pub(super) fn resolve_circom_exec(circom_helpers_dir: &str) -> String {
    let bin_name = if cfg!(target_os = "macos") { "circom_mac" } else { "circom" };
    let in_helpers = Path::new(circom_helpers_dir).join(bin_name);
    if in_helpers.is_file() {
        if let Ok(abs) = in_helpers.canonicalize() {
            return abs.to_string_lossy().to_string();
        }
        return in_helpers.to_string_lossy().to_string();
    }
    bin_name.to_string()
}

/// Resolve a path from an environment variable; if not set, returns `fallback`
/// as-is. (Simpler than pil2-stark-setup's version: no compile-time anchoring
/// to the proofman repo root since we're outside that repo.)
pub(super) fn resolve_path_env(env_var: &str, fallback: &str) -> String {
    match std::env::var(env_var) {
        Ok(v) if !v.is_empty() => v,
        _ => fallback.to_string(),
    }
}
