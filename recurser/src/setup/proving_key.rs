//! Setup pipeline for the recurser-aggregator stage.
//!
//! Pipeline:
//!   1. Generate the inner `vadcop_final` stark verifier circom.
//!   2. Build `define_stark_inputs` / `assign_stark_inputs` blocks for A and B.
//!   3. Generate the recurser-aggregator circom via [`crate::gen_aggregator`].
//!   4. Compile circom → r1cs + c++.
//!   5. Build witness library.
//!   6. plonk2pil → `.pil`, `.fixed.bin`, `.exec`.

use std::fs;
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use serde_json::Value;

use pil2_stark_setup::io::fixed_cols;
use pil2_stark_setup::output::witness_gen::WitnessTracker;
use stark_recurser::plonk2pil::r1cs_types::PlonkOptions;
use stark_recurser::plonk2pil::{self, PlonkResult};
use stark_recurser::stark2circom::stark_inputs::{
    assign_stark_inputs, define_stark_inputs, EnableInput, StarkInputOptions,
};
use stark_recurser::stark2circom::{gen_stark_verifier, StarkVerifierOptions};

use crate::templates::StarkInputBlocks;
use crate::{gen_aggregator, CircomTemplates};

/// Configuration for the recurser-aggregator setup.
pub struct RecurserAggregatorConfig<'a> {
    pub build_dir: &'a str,
    pub name: &'a str,

    // Inner (leaf) verifier inputs (read from `provingKey/<name>/vadcop_final/`).
    /// Inner verifier's verkey row 0 — the 4-element rootC used to verify leaf
    /// proofs. Passed both to the stark verifier generator and baked into the
    /// aggregator template as `rootCVadcopFinalZisk`.
    pub vadcop_final_zisk_vk_row0: &'a [String; 4],
    pub stark_info: &'a Value,
    pub verifier_info: &'a Value,

    // Recurser-side inputs.
    /// Side-input count threaded into the user's `PreparePublics` (free parameter).
    pub n_private_inputs: usize,
    pub program_vks: &'a [[String; 4]],

    /// Required Circom bodies for the three publics-handling sub-templates.
    pub circom_templates: &'a CircomTemplates,

    // Tool paths.
    pub circom_exec: &'a str,
    pub circuits_gl_path: &'a str,
    pub recurser_circuits_path: &'a str,
    pub circom_helpers_dir: &'a str,
}

pub fn gen_recurser_aggregator_setup(
    config: &RecurserAggregatorConfig<'_>,
    witness_tracker: &WitnessTracker,
) -> Result<()> {
    let template = "recurser_aggregator";
    let verifier_name = "vadcop_final_stark.verifier.circom";
    let build_dir = PathBuf::from(config.build_dir);

    let files_dir = build_dir.join("provingKey").join(config.name).join(template);
    fs::create_dir_all(&files_dir)?;

    let circom_dir = build_dir.join("circom");
    let build_path = build_dir.join("build");
    let pil_dir = build_dir.join("pil");
    fs::create_dir_all(&circom_dir)?;
    fs::create_dir_all(&build_path)?;
    fs::create_dir_all(&pil_dir)?;

    // 1. Inner verifier circom — verkey_input=true so rootC is a signal driven by the aggregator mux.
    {
        let rust_opts = StarkVerifierOptions {
            skip_main: true,
            verkey_input: true,
            enable_input: false,
            input_challenges: false,
            fri_queries_batch_size: None,
            multi_fri: false,
        };
        let circom_src = gen_stark_verifier(
            Some(config.vadcop_final_zisk_vk_row0),
            config.stark_info,
            config.verifier_info,
            &rust_opts,
        )
        .context("gen_stark_verifier failed in recurser_aggregator setup")?;
        fs::write(circom_dir.join(verifier_name), &circom_src).context("Failed to write inner verifier circom")?;
    }

    // 2. Build the StarkInputBlocks for sides A and B.
    let io_opts = StarkInputOptions { add_publics: true, is_final: false, parallel: false };
    let define_a = define_stark_inputs(config.stark_info, "a_sv", &io_opts);
    let define_b = define_stark_inputs(config.stark_info, "b_sv", &io_opts);
    let assign_a = assign_stark_inputs("vA", "a_sv", config.stark_info, &io_opts, &EnableInput::None);
    let assign_b = assign_stark_inputs("vB", "b_sv", config.stark_info, &io_opts, &EnableInput::None);

    let stark_inputs = StarkInputBlocks {
        define_a: define_a.as_str(),
        define_b: define_b.as_str(),
        assign_a: assign_a.as_str(),
        assign_b: assign_b.as_str(),
    };

    // 3. Generate the recurser-aggregator circom.
    let circom_out = circom_dir.join(format!("{}.circom", template));
    {
        let circom_src = gen_aggregator(
            config.n_private_inputs,
            verifier_name,
            &config.vadcop_final_zisk_vk_row0[..],
            config.program_vks,
            &stark_inputs,
            config.circom_templates,
        )
        .map_err(|e| anyhow::anyhow!("gen_aggregator failed: {e}"))?;
        fs::write(&circom_out, &circom_src).context("Failed to write recurser_aggregator circom")?;
    }

    // 4. Compile circom → r1cs + c++.
    tracing::info!("Compiling {}...", template);
    let compile_output = std::process::Command::new(config.circom_exec)
        .args([
            "--O1",
            "--r1cs",
            "--prime",
            "goldilocks",
            "--c",
            "--verbose",
            "-l",
            config.recurser_circuits_path,
            "-l",
            config.circuits_gl_path,
        ])
        .arg(circom_out.to_str().unwrap())
        .arg("-o")
        .arg(build_path.to_str().unwrap())
        .output()
        .context("Failed to execute circom for recurser_aggregator setup")?;

    if !compile_output.status.success() {
        let stderr = String::from_utf8_lossy(&compile_output.stderr);
        bail!("Circom compilation failed for {}: {}", template, stderr);
    }

    // Copy .dat
    tracing::info!("Copying circom files...");
    let dat_src = build_path.join(format!("{}_cpp", template)).join(format!("{}.dat", template));
    let dat_dst = files_dir.join(format!("{}.dat", template));
    if dat_src.exists() {
        fs::copy(&dat_src, &dat_dst)?;
    }

    // 5. Witness library
    witness_tracker.run_witness_library_generation(
        config.build_dir,
        files_dir.to_str().unwrap_or(""),
        template,
        template,
        config.circom_helpers_dir,
    );

    // 6. plonk2pil → .pil, .fixed.bin, .exec
    let r1cs_path = build_path.join(format!("{}.r1cs", template));
    let r1cs_data = fs::read(&r1cs_path).with_context(|| format!("Failed to read R1CS: {}", r1cs_path.display()))?;

    let plonk_opts =
        PlonkOptions { airgroup_name: Some("RecurserAggregator".to_string()), max_constraint_degree: None };
    let plonk_result: PlonkResult = plonk2pil::plonk2pil(&r1cs_data, "aggregation", &plonk_opts)
        .context("plonk2pil failed in recurser_aggregator setup")?;

    let fixed_bin_path = build_path.join(format!("{}.fixed.bin", template));
    let fixed_info: Vec<(String, Vec<u32>, Vec<u64>)> =
        plonk_result.fixed_pols.iter().map(|fp| (fp.name.clone(), vec![fp.index as u32], fp.values.clone())).collect();
    fixed_cols::write_fixed_pols_bin(
        fixed_bin_path.to_str().unwrap(),
        &plonk_result.airgroup_name,
        &plonk_result.air_name,
        1u64 << plonk_result.n_bits,
        &fixed_info,
    )?;

    let pil_path = pil_dir.join(format!("{}.pil", template));
    fs::write(&pil_path, &plonk_result.pil_str)?;

    let exec_path = files_dir.join(format!("{}.exec", template));
    let exec_bytes: Vec<u8> = plonk_result.exec.iter().flat_map(|v| v.to_le_bytes()).collect();
    fs::write(&exec_path, &exec_bytes)?;

    witness_tracker.await_all()?;
    Ok(())
}
