use crate::ux::print_banner;
use anyhow::{Context, Result};

use colored::Colorize;
use proofman_common::ParamsGPU;
use std::path::PathBuf;
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_common::io::ZiskStdin;
#[cfg(feature = "stats")]
use zisk_common::ExecutorStatsEvent;
use zisk_sdk::{ProverClient, ZiskProveResult};

// Structure representing the 'prove' subcommand of cargo.
#[derive(clap::Args)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
#[command(propagate_version = true)]
#[command(group(
    clap::ArgGroup::new("input_mode")
        .args(["asm", "emulator"])
        .multiple(false)
        .required(false)
))]
pub struct ZiskProve {
    /// Witness computation dynamic library path
    #[clap(short = 'w', long)]
    pub witness_lib: Option<PathBuf>,

    /// ELF file path
    /// This is the path to the ROM file that the witness computation dynamic library will use
    /// to generate the witness.
    #[clap(short = 'e', long)]
    pub elf: PathBuf,

    /// ASM file path
    /// Optional, mutually exclusive with `--emulator`
    #[clap(short = 's', long)]
    pub asm: Option<PathBuf>,

    /// Use prebuilt emulator (mutually exclusive with `--asm`)
    #[clap(short = 'l', long, action = clap::ArgAction::SetTrue)]
    pub emulator: bool,

    /// Input path
    #[clap(short = 'i', long)]
    pub input: Option<PathBuf>,

    /// ZKsyncOS witness file path (hex-encoded Vec<u32> from eth_runner).
    /// When provided, creates a Replay64Oracle for oracle CSR operations.
    /// This is required for proving programs that use oracle callbacks (like ZKsyncOS).
    #[clap(long)]
    pub witness_file: Option<PathBuf>,

    /// Setup folder path
    #[clap(short = 'k', long)]
    pub proving_key: Option<PathBuf>,

    /// Output dir path
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

    /// Base port for Assembly microservices (default: 23115).
    /// A single execution will use 3 consecutive ports, from this port to port + 2.
    /// If you are running multiple instances of ZisK using mpi on the same machine,
    /// it will use from this base port to base port + 2 * number_of_instances.
    /// For example, if you run 2 mpi instances of ZisK, it will use ports from 23115 to 23117
    /// for the first instance, and from 23118 to 23120 for the second instance.
    #[clap(short = 'p', long, conflicts_with = "emulator")]
    pub port: Option<u16>,

    /// Map unlocked flag
    /// This is used to unlock the memory map for the ROM file.
    /// If you are running ZisK on a machine with limited memory, you may want to enable this option.
    /// This option is mutually exclusive with `--emulator`.
    #[clap(short = 'u', long, conflicts_with = "emulator")]
    pub unlock_mapped_memory: bool,

    /// Verbosity (-v, -vv)
    #[arg(short ='v', long, action = clap::ArgAction::Count, help = "Increase verbosity level")]
    pub verbose: u8, // Using u8 to hold the number of `-v`

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

impl ZiskProve {
    pub fn run(&mut self) -> Result<()> {
        print_banner();

        let mut gpu_params = ParamsGPU::new(self.preallocate);

        if self.max_streams.is_some() {
            gpu_params.with_max_number_streams(self.max_streams.unwrap());
        }
        if self.number_threads_witness.is_some() {
            gpu_params.with_number_threads_pools_witness(self.number_threads_witness.unwrap());
        }
        if self.max_witness_stored.is_some() {
            gpu_params.with_max_witness_stored(self.max_witness_stored.unwrap());
        }

        let stdin = self.create_stdin()?;
        let oracle_bytes = self.load_oracle_bytes()?;

        let emulator = if cfg!(target_os = "macos") { true } else { self.emulator };

        let (result, world_rank) = if emulator {
            self.run_emu(stdin, gpu_params, oracle_bytes)?
        } else {
            self.run_asm(stdin, gpu_params, oracle_bytes)?
        };

        if world_rank == 0 {
            let elapsed = result.duration.as_secs_f64();
            tracing::info!("");
            tracing::info!(
                "{}",
                "--- PROVE SUMMARY ------------------------".bright_green().bold()
            );
            if let Some(proof_id) = result.proof.id {
                tracing::info!("      Proof ID: {}", proof_id);
            }
            tracing::info!("    ► Statistics");
            tracing::info!(
                "      time: {} seconds, steps: {}",
                elapsed,
                result.execution.executed_steps
            );
        }

        Ok(())
    }

    fn create_stdin(&mut self) -> Result<ZiskStdin> {
        let stdin = if let Some(input) = &self.input {
            if !input.exists() {
                return Err(anyhow::anyhow!("Input file not found at {:?}", input.display()));
            }
            ZiskStdin::from_file(input)?
        } else {
            ZiskStdin::null()
        };
        Ok(stdin)
    }

    fn load_oracle_bytes(&self) -> Result<Option<Vec<u8>>> {
        if let Some(witness_path) = &self.witness_file {
            if !witness_path.exists() {
                return Err(anyhow::anyhow!(
                    "Witness file not found at {:?}",
                    witness_path.display()
                ));
            }

            let witness_hex = std::fs::read_to_string(witness_path)
                .with_context(|| format!("Failed to read witness file: {}", witness_path.display()))?;
            let witness_bytes = hex::decode(witness_hex.trim())
                .context("Failed to decode hex witness data")?;

            eprintln!("[CLI] Loaded witness file with {} bytes of oracle data", witness_bytes.len());
            tracing::info!("Loaded witness file with {} bytes of oracle data", witness_bytes.len());

            Ok(Some(witness_bytes))
        } else {
            Ok(None)
        }
    }

    pub fn run_emu(
        &mut self,
        stdin: ZiskStdin,
        gpu_params: ParamsGPU,
        oracle_bytes: Option<Vec<u8>>,
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

        if let Some(bytes) = oracle_bytes {
            eprintln!("[CLI] Setting oracle bytes on prover: {} bytes", bytes.len());
            prover.set_oracle_bytes(bytes);
        } else {
            eprintln!("[CLI] No oracle bytes to set on prover");
        }

        let result = prover.prove(stdin)?;
        let world_rank = prover.world_rank();

        Ok((result, world_rank))
    }

    pub fn run_asm(
        &mut self,
        stdin: ZiskStdin,
        gpu_params: ParamsGPU,
        oracle_bytes: Option<Vec<u8>>,
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

        if let Some(bytes) = oracle_bytes {
            prover.set_oracle_bytes(bytes);
        }

        let result = prover.prove(stdin)?;
        let world_rank = prover.world_rank();

        Ok((result, world_rank))
    }
}
