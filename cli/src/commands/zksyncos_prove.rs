//! ZkSyncOS prove command - prove zksync-os execution with witness data

use crate::{
    commands::{load_zksyncos_witness, witness_to_bytes},
    ux::print_banner,
};
use anyhow::Result;
use clap::Parser;
use colored::Colorize;
use proofman_common::ParamsGPU;
use std::path::PathBuf;
use tracing::info;
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_common::io::ZiskStdin;
use zisk_sdk::{ProverClient, ZiskProveResult};

/// Prove zksync-os execution with witness data.
///
/// This command loads a zksync-os witness file (hex-encoded Vec<u32> from eth_runner)
/// and generates proofs for the execution.
#[derive(Parser)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
#[command(propagate_version = true)]
#[command(group(
    clap::ArgGroup::new("input_mode")
        .args(["asm", "emulator"])
        .multiple(false)
        .required(false)
))]
pub struct ZiskZkSyncOsProve {
    /// Path to zksync-os ELF file (e.g., evm_replay.elf)
    #[clap(short = 'e', long)]
    pub elf: PathBuf,

    /// Path to witness file (hex-encoded Vec<u32> from eth_runner)
    #[clap(short = 'w', long)]
    pub witness: PathBuf,

    /// Witness computation dynamic library path (optional, uses default if not specified)
    #[clap(long)]
    pub witness_lib: Option<PathBuf>,

    /// ASM file path (optional, mutually exclusive with `--emulator`)
    #[clap(short = 's', long)]
    pub asm: Option<PathBuf>,

    /// Use prebuilt emulator (mutually exclusive with `--asm`)
    #[clap(short = 'l', long, action = clap::ArgAction::SetTrue)]
    pub emulator: bool,

    /// Setup folder path (proving key directory)
    #[clap(short = 'k', long)]
    pub proving_key: Option<PathBuf>,

    /// Output directory for proofs
    #[clap(short = 'o', long, default_value = "tmp")]
    pub output_dir: PathBuf,

    #[clap(short = 'a', long, default_value_t = false)]
    pub aggregation: bool,

    #[clap(short = 'f', long, default_value_t = false)]
    pub final_snark: bool,

    #[clap(short = 'y', long, default_value_t = false)]
    pub verify_proofs: bool,

    #[clap(short = 'z', long, default_value_t = false)]
    pub preallocate: bool,

    /// Base port for Assembly microservices (default: 23115)
    #[clap(short = 'p', long, conflicts_with = "emulator")]
    pub port: Option<u16>,

    /// Map unlocked flag for memory-limited machines
    #[clap(short = 'u', long, conflicts_with = "emulator")]
    pub unlock_mapped_memory: bool,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count, help = "Increase verbosity level")]
    pub verbose: u8,

    #[clap(short = 't', long)]
    pub max_streams: Option<usize>,

    #[clap(short = 'n', long)]
    pub number_threads_witness: Option<usize>,

    #[clap(short = 'x', long)]
    pub max_witness_stored: Option<usize>,

    #[clap(short = 'b', long, default_value_t = false)]
    pub save_proofs: bool,

    #[clap(short = 'm', long, default_value_t = false)]
    pub minimal_memory: bool,

    #[clap(short = 'j', long, default_value_t = false)]
    pub shared_tables: bool,

    #[clap(short = 'r', long, default_value_t = false)]
    pub rma: bool,
}

impl ZiskZkSyncOsProve {
    pub fn run(&mut self) -> Result<()> {
        print_banner();

        // Validate witness file exists
        if !self.witness.exists() {
            return Err(anyhow::anyhow!("Witness file not found at {:?}", self.witness.display()));
        }

        // Load and convert witness
        info!("Loading zksync-os witness from {:?}", self.witness.display());
        let witness_data = load_zksyncos_witness(&self.witness)?;
        info!("Loaded {} u32 values from witness", witness_data.len());

        // Log CSR 0x7c* detection info
        info!(
            "Note: CSR 0x7c0 (ORACLE_IO), 0x7c7 (BLAKE2_DELEG), 0x7ca (BIGINT_DELEG) \
             instructions will be logged when encountered during execution"
        );

        // Convert witness to bytes and create stdin
        let witness_bytes = witness_to_bytes(&witness_data);
        info!("Converted witness to {} bytes", witness_bytes.len());

        let stdin = ZiskStdin::from_vec(witness_bytes);

        let mut gpu_params = ParamsGPU::new(self.preallocate);

        if let Some(max_streams) = self.max_streams {
            gpu_params.with_max_number_streams(max_streams);
        }
        if let Some(threads) = self.number_threads_witness {
            gpu_params.with_number_threads_pools_witness(threads);
        }
        if let Some(max_stored) = self.max_witness_stored {
            gpu_params.with_max_witness_stored(max_stored);
        }

        let emulator = if cfg!(target_os = "macos") { true } else { self.emulator };

        let (result, world_rank) = if emulator {
            self.run_emu(stdin, gpu_params)?
        } else {
            self.run_asm(stdin, gpu_params)?
        };

        if world_rank == 0 {
            let elapsed = result.duration.as_secs_f64();
            info!("");
            info!("{}", "--- ZKSYNCOS PROVE SUMMARY ---------------".bright_green().bold());
            if let Some(proof_id) = result.proof.id {
                info!("      Proof ID: {}", proof_id);
            }
            info!("    ► Statistics");
            info!("      time: {} seconds, steps: {}", elapsed, result.execution.executed_steps);
        }

        Ok(())
    }

    pub fn run_emu(
        &mut self,
        stdin: ZiskStdin,
        gpu_params: ParamsGPU,
    ) -> Result<(ZiskProveResult, i32)> {
        let prover = ProverClient::builder()
            .emu()
            .prove()
            .aggregation(self.aggregation)
            .rma(self.rma)
            .witness_lib_path_opt(self.witness_lib.clone())
            .proving_key_path_opt(self.proving_key.clone())
            .elf_path(self.elf.clone())
            .verbose(self.verbose)
            .shared_tables(self.shared_tables)
            .save_proofs(self.save_proofs)
            .output_dir(self.output_dir.clone())
            .verify_proofs(self.verify_proofs)
            .minimal_memory(self.minimal_memory)
            .gpu(gpu_params)
            .print_command_info()
            .build()?;

        let result = prover.prove(stdin)?;
        let world_rank = prover.world_rank();

        Ok((result, world_rank))
    }

    pub fn run_asm(
        &mut self,
        stdin: ZiskStdin,
        gpu_params: ParamsGPU,
    ) -> Result<(ZiskProveResult, i32)> {
        let prover = ProverClient::builder()
            .asm()
            .prove()
            .aggregation(self.aggregation)
            .rma(self.rma)
            .witness_lib_path_opt(self.witness_lib.clone())
            .proving_key_path_opt(self.proving_key.clone())
            .elf_path(self.elf.clone())
            .verbose(self.verbose)
            .shared_tables(self.shared_tables)
            .asm_path_opt(self.asm.clone())
            .base_port_opt(self.port)
            .unlock_mapped_memory(self.unlock_mapped_memory)
            .save_proofs(self.save_proofs)
            .output_dir(self.output_dir.clone())
            .verify_proofs(self.verify_proofs)
            .minimal_memory(self.minimal_memory)
            .gpu(gpu_params)
            .print_command_info()
            .build()?;

        let result = prover.prove(stdin)?;
        let world_rank = prover.world_rank();

        Ok((result, world_rank))
    }
}
