//! Protocol-aware replay processor for pre-recorded oracle responses.
//!
//! Unlike `ReplayOracle` which simply returns sequential u32 values, this processor
//! understands the CSR 0x7c0 oracle protocol and injects response lengths appropriately.
//!
//! ## Protocol
//!
//! The oracle protocol works as follows:
//! 1. Guest writes query_type (e.g., 0x40070000 for BLOCK_METADATA)
//! 2. Guest writes input_len (number of input words)
//! 3. Guest writes input_data[0..input_len]
//! 4. Guest reads response_len (this is what we inject!)
//! 5. Guest reads response_data[0..response_len]
//!
//! The inputs.bin file contains only the response DATA, not the response lengths.
//! This processor tracks writes to determine the query type, then injects the
//! correct response length on the first read after a query completes.

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

/// Query response sizes (in u32 words) for known query types
const BLOCK_METADATA_RESPONSE_SIZE: u32 = 1048; // From BlockMetadataProcessor
const UART_RESPONSE_SIZE: u32 = 0; // UART has no response
const DISCONNECT_RESPONSE_SIZE: u32 = 0; // Disconnect has no response
const NEXT_TX_SIZE_RESPONSE_SIZE: u32 = 1; // Returns 1 u32 (the tx size or 0)

/// Query IDs
const BLOCK_METADATA_QUERY_ID: u32 = 0x40070000;
const UART_QUERY_ID: u32 = 0xFFFFFFFF;
const DISCONNECT_QUERY_ID: u32 = 0xFFFFFFFE;
const NEXT_TX_SIZE_QUERY_ID: u32 = 0x40070001;

/// State of the protocol state machine
#[derive(Debug, Clone, Copy, PartialEq)]
enum ReplayState {
    /// Waiting for query_type write
    Idle,
    /// Received query_type, waiting for input_len
    ReceivedQueryType { query_type: u32 },
    /// Receiving input data
    ReceivingInput { query_type: u32, input_len: u32, received: u32 },
    /// Query complete, next read returns length
    QueryComplete { query_type: u32 },
    /// Reading response data
    ReadingResponse { remaining: u32 },
}

/// A protocol-aware replay oracle that injects response lengths.
///
/// This oracle understands the CSR 0x7c0 query protocol:
/// - Tracks writes to detect query type and input completion
/// - On first read after query, returns the known response length
/// - On subsequent reads, returns data from the pre-recorded buffer
pub struct ProtocolAwareReplayOracle {
    /// Pre-recorded response data (u32 words)
    data: Vec<u32>,
    /// Current read position in data
    position: AtomicUsize,
    /// Protocol state machine
    state: Mutex<ReplayState>,
    /// Query response sizes
    query_sizes: HashMap<u32, u32>,
}

impl ProtocolAwareReplayOracle {
    /// Create a new protocol-aware replay oracle.
    ///
    /// # Arguments
    /// * `data` - The Vec<u32> of pre-recorded oracle response data (without lengths)
    pub fn new(data: Vec<u32>) -> Self {
        let mut query_sizes = HashMap::new();
        query_sizes.insert(BLOCK_METADATA_QUERY_ID, BLOCK_METADATA_RESPONSE_SIZE);
        query_sizes.insert(UART_QUERY_ID, UART_RESPONSE_SIZE);
        query_sizes.insert(DISCONNECT_QUERY_ID, DISCONNECT_RESPONSE_SIZE);
        query_sizes.insert(NEXT_TX_SIZE_QUERY_ID, NEXT_TX_SIZE_RESPONSE_SIZE);

        Self {
            data,
            position: AtomicUsize::new(0),
            state: Mutex::new(ReplayState::Idle),
            query_sizes,
        }
    }

    /// Create a protocol-aware replay oracle from raw bytes (little-endian u32 values).
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

    /// Handle a write to CSR 0x7c0.
    ///
    /// Updates the state machine based on the protocol:
    /// - First write: query_type
    /// - Second write: input_len
    /// - Subsequent writes: input data
    pub fn write(&self, value: u32) {
        let mut state = self.state.lock().expect("state lock");
        *state = match *state {
            ReplayState::Idle => {
                // First write is query_type
                ReplayState::ReceivedQueryType { query_type: value }
            }
            ReplayState::ReceivedQueryType { query_type } => {
                // Second write is input_len
                if value == 0 {
                    ReplayState::QueryComplete { query_type }
                } else {
                    ReplayState::ReceivingInput { query_type, input_len: value, received: 0 }
                }
            }
            ReplayState::ReceivingInput { query_type, input_len, received } => {
                // Receiving input data words
                let new_received = received + 1;
                if new_received >= input_len {
                    ReplayState::QueryComplete { query_type }
                } else {
                    ReplayState::ReceivingInput { query_type, input_len, received: new_received }
                }
            }
            _ => {
                // Unexpected write - reset to idle and treat as new query
                ReplayState::ReceivedQueryType { query_type: value }
            }
        };
    }

    /// Handle a read from CSR 0x7c0.
    ///
    /// Based on the protocol state:
    /// - After query complete: returns the response length (injected)
    /// - During response reading: returns data from the buffer
    pub fn read(&self) -> u32 {
        let mut state = self.state.lock().expect("state lock");

        match *state {
            ReplayState::QueryComplete { query_type } => {
                // First read after query - return response length
                let response_size = self.query_sizes.get(&query_type).copied().unwrap_or(0);

                if response_size == 0 {
                    // No response data, go back to idle
                    *state = ReplayState::Idle;
                } else {
                    *state = ReplayState::ReadingResponse { remaining: response_size };
                }

                response_size
            }
            ReplayState::ReadingResponse { remaining } => {
                // Return next data word from buffer
                let pos = self.position.fetch_add(1, Ordering::SeqCst);
                let value = if pos < self.data.len() { self.data[pos] } else { 0 };

                let new_remaining = remaining.saturating_sub(1);
                if new_remaining == 0 {
                    *state = ReplayState::Idle;
                } else {
                    *state = ReplayState::ReadingResponse { remaining: new_remaining };
                }

                value
            }
            _ => {
                // Unexpected read (no query pending) - return 0
                0
            }
        }
    }

    /// Returns the number of data values remaining to be read.
    pub fn remaining(&self) -> usize {
        let pos = self.position.load(Ordering::SeqCst);
        self.data.len().saturating_sub(pos)
    }

    /// Returns the total number of data values in the replay buffer.
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
    fn test_protocol_replay_block_metadata_query() {
        // Simulate data that would be returned for BLOCK_METADATA query
        // (first few words of response)
        let data: Vec<u32> = (0..1048).collect();
        let oracle = ProtocolAwareReplayOracle::new(data);

        // Send BLOCK_METADATA query with 0 input words
        oracle.write(BLOCK_METADATA_QUERY_ID); // query_type
        oracle.write(0); // input_len = 0

        // First read should return the response length (1048)
        assert_eq!(oracle.read(), 1048);

        // Subsequent reads return data
        assert_eq!(oracle.read(), 0);
        assert_eq!(oracle.read(), 1);
        assert_eq!(oracle.read(), 2);
    }

    #[test]
    fn test_protocol_replay_uart_query() {
        let oracle = ProtocolAwareReplayOracle::new(vec![]);

        // Send UART query with 1 input word
        oracle.write(UART_QUERY_ID); // query_type
        oracle.write(1); // input_len = 1
        oracle.write(b'A' as u32); // input data

        // UART has 0 response size
        assert_eq!(oracle.read(), 0);

        // Should be back to idle, another read returns 0
        assert_eq!(oracle.read(), 0);
    }

    #[test]
    fn test_protocol_replay_next_tx_size_query() {
        let data = vec![42]; // TX size value
        let oracle = ProtocolAwareReplayOracle::new(data);

        // Send NEXT_TX_SIZE query
        oracle.write(NEXT_TX_SIZE_QUERY_ID);
        oracle.write(0); // no input

        // Returns response length (1)
        assert_eq!(oracle.read(), 1);

        // Returns the TX size
        assert_eq!(oracle.read(), 42);
    }

    #[test]
    fn test_protocol_replay_from_bytes() {
        let bytes = [0x04, 0x03, 0x02, 0x01, 0x08, 0x07, 0x06, 0x05];
        let oracle = ProtocolAwareReplayOracle::from_bytes(&bytes);

        assert_eq!(oracle.len(), 2);

        // Query to trigger response
        oracle.write(NEXT_TX_SIZE_QUERY_ID);
        oracle.write(0);

        // Length
        assert_eq!(oracle.read(), 1);
        // First data word
        assert_eq!(oracle.read(), 0x01020304);
    }
}
