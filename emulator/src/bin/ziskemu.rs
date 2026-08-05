use clap::Parser;
use std::{fmt::Write, panic, process};
use zisk_common::EmuTrace;
use ziskemu::{EmuOptions, Emulator, ZiskEmulator, ZiskEmulatorErr};

fn main() {
    install_access_fault_panic_hook();

    // Create a emulator options instance based on arguments or default values
    let options: EmuOptions = EmuOptions::parse();

    //println! {"options={}", options};

    // Log the emulator options if requested
    if options.verbose {
        println!("ziskemu converts an ELF RISCV file into a ZISK rom or loads a ZISK rom file, emulates it with the provided input, and copies the output to console or a file");
    }

    // Call emulate, with these options
    let emulator = ZiskEmulator;
    let result = emulator.emulate(&options, None::<Box<dyn Fn(EmuTrace)>>);

    match result {
        Ok(result) => {
            // println!("Emulation completed successfully");
            result.iter().fold(String::new(), |mut acc, byte| {
                write!(&mut acc, "{byte:02x}").unwrap();
                acc
            });
            // print!("Result: 0x{}", hex_string);
        }
        Err(ZiskEmulatorErr::Exception(cause)) => {
            eprintln!("Emulation stopped with RISC-V exception cause {cause}");
            process::exit(32 + cause as i32);
        }
        Err(error) => {
            eprintln!("Error during emulation: {error:?}");
            process::exit(1);
        }
    }
}

fn install_access_fault_panic_hook() {
    let default_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let message = panic_info
            .payload()
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| panic_info.payload().downcast_ref::<String>().map(String::as_str));

        let exit_code = match message {
            Some(message) if message.contains("Mem::read()") => Some(37),
            Some(message) if message.contains("Mem::write_silent()") => Some(39),
            _ => None,
        };

        if let Some(exit_code) = exit_code {
            eprintln!("Emulation stopped with RISC-V exception exit code {exit_code}");
            process::exit(exit_code);
        }

        default_hook(panic_info);
    }));
}
