use clap::Parser;
use std::{fmt::Write, fs, process};
use zisk_common::EmuTrace;
use zisk_oracle::processors::ReplayOracle;
use ziskemu::{create_replay_oracle_callback, EmuOptions, Emulator, ZiskEmulator};

fn main() {
    // Create a emulator options instance based on arguments or default values
    let options: EmuOptions = EmuOptions::parse();

    //println! {"options={}", options};

    // Log the emulator options if requested
    if options.verbose {
        println!("ziskemu converts an ELF RISCV file into a ZISK rom or loads a ZISK rom file, emulates it with the provided input, and copies the output to console or a file");
    }

    // Create oracle callback if zksyncos_oracle flag is set.
    //
    // Why replay mode? ZKsyncOS queries external oracles (CSR 0x7c0) for Ethereum
    // state data (block headers, transactions, MPT proofs). These queries are
    // non-deterministic, but ZK proofs require deterministic execution. The solution:
    //
    // 1. Forward pass: Execute with live oracle, capture responses as Vec<u32> witness
    // 2. Replay pass: Re-execute using captured witness data (no network needed)
    //
    // This is required by ZKsyncOS's architecture regardless of host VM (ZisK, Airbender,
    // QEMU) - any program that queries external oracles needs replay for proving.
    //
    // Note: The 64-bit guest reads u32 pairs and combines them into u64 via io_oracle.
    let oracle_callback = if options.zksyncos_oracle {
        if let Some(ref inputs_path) = options.inputs {
            if options.verbose {
                println!("Oracle support enabled - loading witness data from inputs file");
            }

            // Load the inputs file as oracle replay data
            let witness_data =
                fs::read(inputs_path).expect("Could not read inputs file for oracle");

            if options.verbose {
                println!(
                    "Loaded {} bytes ({} u32 values) of witness data",
                    witness_data.len(),
                    witness_data.len() / 4
                );
            }

            // Create replay oracle from the witness data
            // The zksync-os witness is hex text (8 ASCII chars per u32 value)
            // Returns u32 values directly; 64-bit guest reads pairs and combines them
            let replay_oracle = ReplayOracle::from_hex_bytes(&witness_data);
            Some(create_replay_oracle_callback(replay_oracle))
        } else {
            eprintln!("Warning: --oracle flag set but no inputs file provided");
            None
        }
    } else {
        None
    };

    // Call emulate with these options
    let emulator = ZiskEmulator;
    let result = emulator.emulate(&options, None::<Box<dyn Fn(EmuTrace)>>, oracle_callback);

    match result {
        Ok(result) => {
            // println!("Emulation completed successfully");
            result.iter().fold(String::new(), |mut acc, byte| {
                write!(&mut acc, "{byte:02x}").unwrap();
                acc
            });
            // print!("Result: 0x{}", hex_string);
        }
        Err(e) => {
            eprintln!("Error during emulation: {e:?}");
            process::exit(1);
        }
    }
}
