use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use fields::Goldilocks;
use proofman::ProofMan;
use proofman_common::{ProofmanOptions, VerboseMode};
use proofman_util::VadcopFinalProof;
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_prover_backend::setup_logger;

use crate::common::get_proving_key;

#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
/// Fold two `vadcop_final`-shape proofs into one recurser-aggregator proof,
/// using a setup previously produced by `cargo-zisk setup-recurser-aggregator`.
/// See `recurser/docs/aggregator-flow.md`.
pub struct ZiskProveRecurserAggregator {
    /// Directory the recurser setup wrote its artifacts to (matches
    /// `setup-recurser-aggregator`'s `--output-dir`). Defaults to `./build`.
    #[arg(short = 'o', long = "output-dir", default_value = "build")]
    pub output_dir: String,

    /// `<recurser-id>` segment under `<output-dir>/provingKey/recurser/`. The
    /// setup CLI logs this at startup.
    #[arg(long = "recurser-id")]
    pub recurser_id: String,

    /// First input proof (`vadcop_final_proof.bin` from `cargo-zisk prove`,
    /// or a prior recurser-aggregator output).
    #[arg(short = 'a', long = "proof-a")]
    pub proof_a: PathBuf,

    /// Second input proof.
    #[arg(short = 'b', long = "proof-b")]
    pub proof_b: PathBuf,

    /// Where to write the resulting `VadcopFinalProof`. Defaults to
    /// `recurser_aggregator_proof.bin`.
    #[arg(long = "output", default_value = "recurser_aggregator_proof.bin")]
    pub output: PathBuf,

    /// `rootCRecurserAgg` as 4 comma-separated decimal Goldilocks limbs (e.g.
    /// `--root-c-recurser-agg 1,2,3,4`). Omit to read the recurser's own
    /// `recurser_aggregator.verkey.bin` — the natural default per §10.
    #[arg(long = "root-c-recurser-agg")]
    pub root_c_recurser_agg: Option<String>,

    /// `privateInputs` as comma-separated decimal `u64`s. Length must match
    /// the `--n-private-inputs` baked into the setup.
    #[arg(long = "private-inputs")]
    pub private_inputs: Option<String>,

    /// Use the GPU prover path.
    #[arg(long, default_value_t = false)]
    pub gpu: bool,

    /// Path to a precomputed proving key. ProofMan loads it to size the
    /// shared memory-handler / aux-trace buffers the recurser proof reuses.
    /// Defaults to the standard ZisK location.
    #[arg(short = 'k', long = "proving-key")]
    pub proving_key: Option<PathBuf>,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: u8,
}

impl ZiskProveRecurserAggregator {
    pub fn run(&self) -> Result<()> {
        setup_logger(self.verbose.into());

        rayon::ThreadPoolBuilder::new().stack_size(64 * 1024 * 1024).build_global().ok();

        let setup_stem = recurser_setup_stem(&self.output_dir, &self.recurser_id);
        if !sibling_exists(&setup_stem, ".starkinfo.json") {
            bail!(
                "Recurser setup not found at {:?}. Run `cargo-zisk setup-recurser-aggregator` \
                 and the follow-up proofman setup to fill in .starkinfo.json / .bin / .verkey.bin / .const \
                 before proving.",
                setup_stem
            );
        }

        let proof_a = VadcopFinalProof::load(&self.proof_a)
            .map_err(|e| anyhow::anyhow!(e.to_string()))
            .with_context(|| format!("Failed to load proof_a: {}", self.proof_a.display()))?;
        let proof_b = VadcopFinalProof::load(&self.proof_b)
            .map_err(|e| anyhow::anyhow!(e.to_string()))
            .with_context(|| format!("Failed to load proof_b: {}", self.proof_b.display()))?;

        let private_inputs = match &self.private_inputs {
            Some(s) if !s.trim().is_empty() => s
                .split(',')
                .map(|t| {
                    t.trim()
                        .parse::<u64>()
                        .with_context(|| format!("private input '{t}' is not a valid u64"))
                })
                .collect::<Result<Vec<_>>>()?,
            _ => Vec::new(),
        };

        let root_c = match &self.root_c_recurser_agg {
            Some(s) => parse_root_c(s)?,
            None => read_verkey_4(&setup_stem)
                .context("Failed to default rootCRecurserAgg from recurser's verkey.bin")?,
        };

        let proving_key = get_proving_key(self.proving_key.as_ref())?;
        let verbose_mode: VerboseMode = self.verbose.into();
        let mut pm_options = ProofmanOptions::new();
        pm_options.verbose_mode = verbose_mode;
        pm_options.gpu = self.gpu;

        tracing::info!("Initializing ProofMan against {}", proving_key.display());
        let proofman = ProofMan::<Goldilocks>::new(proving_key, pm_options)
            .map_err(|e| anyhow::anyhow!("ProofMan::new failed: {e}"))?;

        proofman
            .register_recurser_setup(&self.recurser_id, &setup_stem)
            .map_err(|e| anyhow::anyhow!("register_recurser_setup failed: {e}"))?;

        tracing::info!("Proving recurser-aggregator '{}'", self.recurser_id);
        let out = proofman
            .prove_recurser_aggregator(&self.recurser_id, &proof_a, &proof_b, &private_inputs, &root_c)
            .map_err(|e| anyhow::anyhow!("prove_recurser_aggregator failed: {e}"))?;

        if let Some(parent) = self.output.parent() {
            fs::create_dir_all(parent).with_context(|| format!("Failed to mkdir {}", parent.display()))?;
        }
        out.save(&self.output)
            .map_err(|e| anyhow::anyhow!(e.to_string()))
            .with_context(|| format!("Failed to save output proof to {}", self.output.display()))?;
        tracing::info!("Recurser-aggregator proof written to {}", self.output.display());
        Ok(())
    }
}

fn recurser_setup_stem(output_dir: &str, recurser_id: &str) -> PathBuf {
    PathBuf::from(output_dir)
        .join("provingKey")
        .join("recurser")
        .join(recurser_id)
        .join("recurser_aggregator")
}

fn sibling_exists(stem: &Path, ext: &str) -> bool {
    let mut p = stem.as_os_str().to_owned();
    p.push(ext);
    Path::new(&p).is_file()
}

fn parse_root_c(s: &str) -> Result<[u64; 4]> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 4 {
        bail!("--root-c-recurser-agg needs 4 comma-separated limbs, got {}", parts.len());
    }
    let mut limbs = [0u64; 4];
    for (i, p) in parts.iter().enumerate() {
        limbs[i] = p
            .trim()
            .parse::<u64>()
            .with_context(|| format!("rootC limb #{} ('{}') is not a valid u64", i, p))?;
    }
    Ok(limbs)
}

/// Read 32 bytes (4 × u64 LE) from `<stem>.verkey.bin`. Same on-disk format
/// rom-setup uses for program VKs elsewhere in the codebase.
fn read_verkey_4(stem: &Path) -> Result<[u64; 4]> {
    let mut path = stem.as_os_str().to_owned();
    path.push(".verkey.bin");
    let mut file = fs::File::open(Path::new(&path))
        .with_context(|| format!("Failed to open {:?}", path))?;
    let mut bytes = [0u8; 32];
    file.read_exact(&mut bytes)
        .with_context(|| format!("Failed to read 32 bytes from {:?}", path))?;
    let mut limbs = [0u64; 4];
    for i in 0..4 {
        let chunk: [u8; 8] = bytes[i * 8..(i + 1) * 8].try_into().unwrap();
        limbs[i] = u64::from_le_bytes(chunk);
    }
    Ok(limbs)
}
