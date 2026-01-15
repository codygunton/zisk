//! Oracle integration helpers for the emulator.

use std::sync::{Arc, Mutex};
use zisk_core::{OracleCallback, OracleOp, ZiskMemoryReader};
use zisk_oracle::processors::{ProtocolAwareReplayOracle, Replay64Oracle, ReplayOracle};
use zisk_oracle::ZiskOracle;

// Q?: This oracle callback stuff is extermely spaghetti. Is there a light-touch refactor that
// would improve on this?
// A: The complexity comes from needing to support multiple oracle types (ZiskOracle, ReplayOracle,
// Replay64Oracle, ProtocolAwareReplayOracle) through a single OracleCallback type. A light-touch
// improvement: define an `Oracle` trait with `read(&self) -> u64` and `write(&mut self, u64)`,
// then have Mem hold `Option<Box<dyn Oracle>>` instead of the callback. This removes Arc<Mutex>
// wrapping from user code and makes the interface cleaner.
/// Creates an `OracleCallback` that wraps a `ZiskOracle`.
///
/// The callback routes reads and writes to the appropriate oracle methods.
///
/// # Example
///
/// ```ignore
/// use zisk_oracle::{ZiskOracle, processors::{UartProcessor, BlockMetadataProcessor}};
/// use ziskemu::create_oracle_callback;
///
/// let mut oracle = ZiskOracle::new();
/// oracle.add_processor(UartProcessor::new());
/// oracle.add_processor(BlockMetadataProcessor::new_for_test());
///
/// let callback = create_oracle_callback(oracle);
/// emu.ctx.inst_ctx.mem.set_oracle_callback(callback);
/// ```
pub fn create_oracle_callback(oracle: ZiskOracle) -> OracleCallback {
    let oracle = Arc::new(Mutex::new(oracle));

    Arc::new(Mutex::new(move |op: OracleOp, _mem_reader: &dyn ZiskMemoryReader| -> u64 {
        let mut oracle_guard = oracle.lock().expect("oracle lock poisoned");
        match op {
            OracleOp::Read => oracle_guard.read() as u64,
            OracleOp::Write(val) => {
                oracle_guard.write(val as u32);
                0
            }
        }
    }))
}

/// Creates an `OracleCallback` that wraps a `ReplayOracle`.
///
/// The replay oracle returns pre-recorded u32 values from witness generation.
/// This is the oracle to use when executing with a pre-computed witness file.
///
/// # Example
///
/// ```ignore
/// use zisk_oracle::processors::ReplayOracle;
/// use ziskemu::create_replay_oracle_callback;
/// use std::fs;
///
/// // Load witness/inputs file
/// let data = fs::read("inputs.bin").expect("read inputs");
/// let replay_oracle = ReplayOracle::from_bytes(&data);
///
/// let callback = create_replay_oracle_callback(replay_oracle);
/// emu.ctx.inst_ctx.mem.set_oracle_callback(callback);
/// ```
pub fn create_replay_oracle_callback(oracle: ReplayOracle) -> OracleCallback {
    let oracle = Arc::new(oracle);

    Arc::new(Mutex::new(move |op: OracleOp, _mem_reader: &dyn ZiskMemoryReader| -> u64 {
        match op {
            OracleOp::Read => oracle.read() as u64,
            OracleOp::Write(val) => {
                oracle.write(val as u32);
                0
            }
        }
    }))
}

/// Creates an `OracleCallback` that wraps a `ProtocolAwareReplayOracle`.
///
/// This oracle understands the CSR 0x7c0 protocol and injects response
/// lengths correctly. Use this for replaying witness data from inputs.bin.
///
/// Unlike `create_replay_oracle_callback`, this version:
/// - Tracks writes to understand which query is being made
/// - Injects the correct response length on first read after query completion
/// - Returns data from the buffer on subsequent reads
///
/// # Example
///
/// ```ignore
/// use zisk_oracle::processors::ProtocolAwareReplayOracle;
/// use ziskemu::create_protocol_replay_callback;
/// use std::fs;
///
/// // Load witness/inputs file
/// let data = fs::read("inputs.bin").expect("read inputs");
/// let replay_oracle = ProtocolAwareReplayOracle::from_bytes(&data);
///
/// let callback = create_protocol_replay_callback(replay_oracle);
/// emu.ctx.inst_ctx.mem.set_oracle_callback(callback);
/// ```
pub fn create_protocol_replay_callback(oracle: ProtocolAwareReplayOracle) -> OracleCallback {
    let oracle = Arc::new(oracle);

    Arc::new(Mutex::new(move |op: OracleOp, _mem_reader: &dyn ZiskMemoryReader| -> u64 {
        match op {
            OracleOp::Read => oracle.read() as u64,
            OracleOp::Write(val) => {
                oracle.write(val as u32);
                0
            }
        }
    }))
}

/// Creates an `OracleCallback` that wraps a `Replay64Oracle`.
///
/// This oracle combines pairs of u32 values from the witness file into 64-bit values.
/// Use this for 64-bit RISC-V targets where the guest expects full 64-bit CSR reads.
///
/// The zksync-os oracle_provider splits 64-bit usize values into pairs of 32-bit reads.
/// This oracle recombines them: `result = ((high << 32) | low)`.
///
/// # Example
///
/// ```ignore
/// use zisk_oracle::processors::Replay64Oracle;
/// use ziskemu::create_replay64_oracle_callback;
/// use std::fs;
///
/// // Load witness/inputs file (big-endian format from zksync-os)
/// let data = fs::read("inputs.bin").expect("read inputs");
/// let replay_oracle = Replay64Oracle::from_bytes_be(&data);
///
/// let callback = create_replay64_oracle_callback(replay_oracle);
/// emu.ctx.inst_ctx.mem.set_oracle_callback(callback);
/// ```
pub fn create_replay64_oracle_callback(oracle: Replay64Oracle) -> OracleCallback {
    let oracle = Arc::new(oracle);

    Arc::new(Mutex::new(move |op: OracleOp, _mem_reader: &dyn ZiskMemoryReader| -> u64 {
        match op {
            OracleOp::Read => oracle.read(),
            OracleOp::Write(val) => {
                oracle.write(val);
                0
            }
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use zisk_oracle::processors::UartProcessor;

    /// Dummy memory reader for testing - panics if actually used
    struct DummyMemReader;

    impl ZiskMemoryReader for DummyMemReader {
        fn read_mem(&self, addr: u64, width: u64) -> u64 {
            panic!("DummyMemReader::read_mem() called with addr={addr:x} width={width}");
        }
    }

    #[test]
    fn test_create_oracle_callback() {
        let mut oracle = ZiskOracle::new();
        oracle.add_processor(UartProcessor::new());

        let callback = create_oracle_callback(oracle);
        let dummy_mem = DummyMemReader;

        // Test write
        let mut cb = callback.lock().expect("lock");
        let _ = cb(OracleOp::Write(0xFFFFFFFF), &dummy_mem); // UART query ID

        // Test read (should return 0 since no response after UART)
        let value = cb(OracleOp::Read, &dummy_mem);
        // UART doesn't return length, so this should be 0 initially
        assert_eq!(value, 0);
    }

    #[test]
    fn test_create_replay_oracle_callback() {
        let data = vec![0x12345678, 0xDEADBEEF];
        let replay_oracle = ReplayOracle::new(data);
        let callback = create_replay_oracle_callback(replay_oracle);
        let dummy_mem = DummyMemReader;

        let mut cb = callback.lock().expect("lock");

        // Reads return the pre-recorded values
        assert_eq!(cb(OracleOp::Read, &dummy_mem), 0x12345678);
        assert_eq!(cb(OracleOp::Read, &dummy_mem), 0xDEADBEEF);
        assert_eq!(cb(OracleOp::Read, &dummy_mem), 0); // Exhausted

        // Writes are no-ops
        assert_eq!(cb(OracleOp::Write(999), &dummy_mem), 0);
    }
}
