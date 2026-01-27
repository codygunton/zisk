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

    // Create oracle callback if zksyncos_oracle flag is set
    // Uses ReplayOracle to replay u32 witness data directly.
    //--say more about what this replay model is and why it to used nz in the new guest program
    //well that reviewers of this pull requests can better understand what's going on
    //--let's also get rid of this logging here and any other auxiliary logging of this or for
    //clarity of the pr  the but we can keep conditional logging  especially uart logging
    // The witness file from zksync-os contains u32 values in big-endian format.
    // The 64-bit guest handles combining u32 pairs into u64 via io_oracle.
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
