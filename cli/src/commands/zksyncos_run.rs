//! ZkSyncOS run command - execute zksync-os ELF with witness data (no proving)

use crate::{
    commands::{cli_fail_if_gpu_mode, load_zksyncos_witness, witness_to_bytes},
    ux::print_banner,
};
use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;
use tracing::info;
use zisk_build::ZISK_VERSION_MESSAGE;
use zisk_common::io::ZiskStdin;
use zisk_sdk::{ProverClient, ZiskExecuteResult};

/// Execute zksync-os ELF with witness data (no proving).
///
/// This command loads a zksync-os witness file (hex-encoded Vec<u32> from eth_runner)
/// and executes the ELF without generating proofs.
#[derive(Parser)]
#[command(author, about, long_about = None, version = ZISK_VERSION_MESSAGE)]
#[command(propagate_version = true)]
#[command(group(
    clap::ArgGroup::new("input_mode")
        .args(["asm", "emulator"])
        .multiple(false)
        .required(false)
))]
pub struct ZiskZkSyncOsRun {
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

    /// Base port for Assembly microservices (default: 23115)
    #[clap(short = 'p', long, conflicts_with = "emulator")]
    pub port: Option<u16>,

    /// Map unlocked flag for memory-limited machines
    #[clap(short = 'u', long, conflicts_with = "emulator")]
    pub unlock_mapped_memory: bool,

    /// Verbosity (-v, -vv)
    #[arg(short = 'v', long, action = clap::ArgAction::Count, help = "Increase verbosity level")]
    pub verbose: u8,

    #[clap(short = 'j', long, default_value_t = false)]
    pub shared_tables: bool,
}

impl ZiskZkSyncOsRun {
    pub fn run(&mut self) -> Result<()> {
        cli_fail_if_gpu_mode()?;

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

        let emulator = if cfg!(target_os = "macos") { true } else { self.emulator };
        let result = if emulator { self.run_emu(stdin)? } else { self.run_asm(stdin)? };

        info!(
            "Execution completed in {:.2?}, executed steps: {}",
            result.duration, result.execution.executed_steps
        );

        Ok(())
    }

    pub fn run_emu(&mut self, stdin: ZiskStdin) -> Result<ZiskExecuteResult> {
        let prover = ProverClient::builder()
            .emu()
            .witness()
            .witness_lib_path_opt(self.witness_lib.clone())
            .proving_key_path_opt(self.proving_key.clone())
            .elf_path(self.elf.clone())
            .verbose(self.verbose)
            .shared_tables(self.shared_tables)
            .print_command_info()
            .build()?;

        prover.execute(stdin)
    }

    pub fn run_asm(&mut self, stdin: ZiskStdin) -> Result<ZiskExecuteResult> {
        let prover = ProverClient::builder()
            .asm()
            .verify_constraints()
            .witness_lib_path_opt(self.witness_lib.clone())
            .proving_key_path_opt(self.proving_key.clone())
            .elf_path(self.elf.clone())
            .verbose(self.verbose)
            .shared_tables(self.shared_tables)
            .asm_path_opt(self.asm.clone())
            .base_port_opt(self.port)
            .unlock_mapped_memory(self.unlock_mapped_memory)
            .print_command_info()
            .build()?;

        prover.execute(stdin)
    }
}
