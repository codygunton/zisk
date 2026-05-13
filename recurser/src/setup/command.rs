//! Top-level command entry: loads inputs from disk, builds the config, and
//! drives [`gen_recurser_aggregator_setup`](super::gen_recurser_aggregator_setup).
//!
//! Suitable for direct use from a CLI binary (e.g. zisk's `cargo-zisk`) — see
//! [`SetupRecurserAggregatorOptions`].

use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use serde_json::Value;

use pil2_stark_setup::output::witness_gen::WitnessTracker;

use super::proving_key::{gen_recurser_aggregator_setup, RecurserAggregatorConfig};
use super::resolve::{resolve_circom_exec, resolve_path_env};
use crate::CircomTemplates;

pub struct SetupRecurserAggregatorOptions {
    /// Build directory containing `provingKey/<name>/vadcop_final/`.
    pub build_dir: String,
    /// Path to a JSON file `[["a","b","c","d"], ...]` listing registered program VKs.
    pub program_vks: String,
    /// Number of side inputs threaded into the user's `PreparePublics`.
    pub n_private_inputs: usize,
    /// Path to a user-supplied `PreparePublics` Circom body. Optional — when
    /// `None`, the built-in identity-passthrough body is used.
    pub prepare_publics_template: Option<String>,
    /// Path to a user-supplied `CheckPublics` Circom body. Optional — when
    /// `None`, the built-in no-op body is used.
    pub check_publics_template: Option<String>,
    /// Path to the user-supplied `AggregatePublics` Circom body (required).
    pub aggregate_publics_template: String,
}

pub fn run_setup_recurser_aggregator(opts: &SetupRecurserAggregatorOptions) -> Result<()> {
    let build_dir = &opts.build_dir;

    let global_info_path = PathBuf::from(build_dir).join("provingKey").join("pilout.globalInfo.json");
    if !global_info_path.exists() {
        bail!("Global info file not found: {:?}. Run `setup --recursive` first.", global_info_path);
    }
    let global_info: Value = serde_json::from_str(&fs::read_to_string(&global_info_path)?)?;
    let name = global_info.get("name").and_then(|v| v.as_str()).unwrap_or("pilout").to_string();

    // vadcop_final artifacts (the inner verifier this stage will include).
    let vadcop_final_dir =
        PathBuf::from(build_dir).join("provingKey").join(&name).join("vadcop_final");
    let verkey_path = vadcop_final_dir.join("vadcop_final.verkey.json");
    let starkinfo_path = vadcop_final_dir.join("vadcop_final.starkinfo.json");
    let verifier_info_path = vadcop_final_dir.join("vadcop_final.verifierinfo.json");
    for p in [&verkey_path, &starkinfo_path, &verifier_info_path] {
        if !p.exists() {
            bail!("Required file not found: {:?}. Run `setup-final` first.", p);
        }
    }

    let vadcop_final_zisk_vk_row0 =
        parse_verkey_4(&verkey_path).context("Failed to parse vadcop_final.verkey.json")?;
    let stark_info: Value = serde_json::from_str(&fs::read_to_string(&starkinfo_path)?)?;
    let verifier_info: Value = serde_json::from_str(&fs::read_to_string(&verifier_info_path)?)?;

    let program_vks_str = fs::read_to_string(&opts.program_vks)
        .with_context(|| format!("Failed to read program_vks: {}", opts.program_vks))?;
    let program_vks: Vec<[String; 4]> = serde_json::from_str(&program_vks_str)
        .with_context(|| format!("Failed to parse program_vks: {}", opts.program_vks))?;
    if program_vks.is_empty() {
        bail!("program_vks must contain at least one entry");
    }

    let load_optional = |opt: &Option<String>, name: &str| -> Result<Option<String>> {
        match opt {
            Some(path) => Ok(Some(
                fs::read_to_string(path).with_context(|| format!("Failed to read {}: {}", name, path))?,
            )),
            None => Ok(None),
        }
    };
    let circom_templates = CircomTemplates {
        prepare_publics: load_optional(&opts.prepare_publics_template, "prepare_publics_template")?,
        check_publics: load_optional(&opts.check_publics_template, "check_publics_template")?,
        aggregate_publics: fs::read_to_string(&opts.aggregate_publics_template).with_context(|| {
            format!("Failed to read aggregate_publics_template: {}", opts.aggregate_publics_template)
        })?,
    };

    // Tool paths (env-overridable). The fallbacks point at pil2-proofman-relative
    // layouts; set the corresponding env vars when running zisk against a custom
    // proofman checkout.
    let circuits_gl_path =
        resolve_path_env("CIRCUITS_GL_PATH", "setup/stark-recurser/stark2circom/circom_verifier/circuits.gl");
    let recurser_circuits_path = resolve_path_env(
        "RECURSER_CIRCUITS_COMPRESSED_FINAL_PATH",
        "setup/stark-recurser/stark2circom/circom_verifier/helper_circuits",
    );
    let circom_helpers_dir = resolve_path_env("CIRCOM_HELPERS_DIR", "setup/circom");
    let goldilocks_src_dir = resolve_path_env("GOLDILOCKS_SRC_DIR", "pil2-stark/src/goldilocks/src");
    let circom_exec = resolve_circom_exec(&circom_helpers_dir);
    let witness_tracker = WitnessTracker::with_goldilocks_src(&goldilocks_src_dir);

    let config = RecurserAggregatorConfig {
        build_dir,
        name: &name,
        vadcop_final_zisk_vk_row0: &vadcop_final_zisk_vk_row0,
        stark_info: &stark_info,
        verifier_info: &verifier_info,
        n_private_inputs: opts.n_private_inputs,
        program_vks: &program_vks,
        circom_templates: &circom_templates,
        circom_exec: &circom_exec,
        circuits_gl_path: &circuits_gl_path,
        recurser_circuits_path: &recurser_circuits_path,
        circom_helpers_dir: &circom_helpers_dir,
    };

    tracing::info!("Running recurser-aggregator setup (standalone) for '{}'", name);
    gen_recurser_aggregator_setup(&config, &witness_tracker).context("Recurser-aggregator setup failed")?;
    witness_tracker.await_all()?;
    tracing::info!("Recurser-aggregator setup complete");
    Ok(())
}

fn parse_verkey_4(path: &std::path::Path) -> Result<[String; 4]> {
    let s = fs::read_to_string(path).with_context(|| format!("Failed to read verkey: {:?}", path))?;
    let v: Vec<Value> = serde_json::from_str(&s)
        .with_context(|| format!("Failed to parse verkey JSON: {:?}", path))?;
    if v.len() != 4 {
        bail!("verkey.json has {} elements, expected 4", v.len());
    }
    let to_str = |i: usize, e: &Value| -> Result<String> {
        if let Some(s) = e.as_str() {
            Ok(s.to_string())
        } else if let Some(n) = e.as_u64() {
            Ok(n.to_string())
        } else {
            bail!("verkey.json element {} is not a number or string: {}", i, e)
        }
    };
    Ok([to_str(0, &v[0])?, to_str(1, &v[1])?, to_str(2, &v[2])?, to_str(3, &v[3])?])
}
