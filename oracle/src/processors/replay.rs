//! Replay processor for pre-recorded oracle responses.
//!
//! This processor replays u32 values that were recorded during witness generation.
//! It's used when executing with a pre-computed witness rather than live oracle data.

use std::sync::atomic::{AtomicUsize, Ordering};

/// A replay oracle that returns pre-recorded u32 values sequentially.
///
/// During witness generation, all oracle reads are recorded to a `Vec<u32>`.
/// This struct replays those values in the same order.
pub struct ReplayOracle {
    /// Pre-recorded u32 values from witness generation
    data: Vec<u32>,
    /// Current read position
    position: AtomicUsize,
}

impl ReplayOracle {
    /// Create a new replay oracle from pre-recorded data.
    ///
    /// # Arguments
    /// * `data` - The Vec<u32> of pre-recorded oracle reads
    pub fn new(data: Vec<u32>) -> Self {
        Self { data, position: AtomicUsize::new(0) }
    }

    /// Create a replay oracle from raw bytes (little-endian u32 values).
    ///
    /// # Arguments
    /// * `bytes` - Raw bytes containing little-endian u32 values
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let data: Vec<u32> = bytes
            .chunks_exact(4)
            .map(|chunk| u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
            .collect();
        Self::new(data)
    }

    /// Read the next u32 value from the replay buffer.
    ///
    /// Returns 0 if all values have been consumed.
    pub fn read(&self) -> u32 {
        let pos = self.position.fetch_add(1, Ordering::SeqCst);
        if pos < self.data.len() {
            self.data[pos]
        } else {
            // All data consumed - return 0
            0
        }
    }

    /// Write is a no-op for replay oracle (writes were processed during witness generation).
    pub fn write(&self, _value: u32) {
        // Intentionally empty - writes don't affect replay
    }

    /// Returns the number of values remaining to be read.
    pub fn remaining(&self) -> usize {
        let pos = self.position.load(Ordering::SeqCst);
        self.data.len().saturating_sub(pos)
    }

    /// Returns the total number of values in the replay buffer.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Returns true if the replay buffer is empty.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_replay_oracle_reads_sequentially() {
        let data = vec![0x12345678, 0xDEADBEEF, 0xCAFEBABE];
        let oracle = ReplayOracle::new(data);

        assert_eq!(oracle.read(), 0x12345678);
        assert_eq!(oracle.read(), 0xDEADBEEF);
        assert_eq!(oracle.read(), 0xCAFEBABE);
        // After exhaustion, returns 0
        assert_eq!(oracle.read(), 0);
    }

    #[test]
    fn test_replay_oracle_from_bytes() {
        // Little-endian bytes for [0x01020304, 0x05060708]
        let bytes = [0x04, 0x03, 0x02, 0x01, 0x08, 0x07, 0x06, 0x05];
        let oracle = ReplayOracle::from_bytes(&bytes);

        assert_eq!(oracle.len(), 2);
        assert_eq!(oracle.read(), 0x01020304);
        assert_eq!(oracle.read(), 0x05060708);
    }

    #[test]
    fn test_replay_oracle_remaining() {
        let data = vec![1, 2, 3];
        let oracle = ReplayOracle::new(data);

        assert_eq!(oracle.remaining(), 3);
        oracle.read();
        assert_eq!(oracle.remaining(), 2);
        oracle.read();
        oracle.read();
        assert_eq!(oracle.remaining(), 0);
    }

    #[test]
    fn test_replay_oracle_write_is_noop() {
        let data = vec![42];
        let oracle = ReplayOracle::new(data);

        // Write should not affect reads
        oracle.write(999);
        assert_eq!(oracle.read(), 42);
    }
}
