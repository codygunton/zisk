//! Zisk Oracle System
//!
//! Handles CSR 0x7c0 (NON_DETERMINISM_CSR) reads/writes for oracle queries.
//!
//! # Protocol
//!
//! The guest communicates with the oracle via CSR 0x7c0:
//!
//! **Write phase** (guest writes query):
//! ```text
//! csrrw x0, 0x7c0, query_type   # e.g., 0x40070000 (BLOCK_METADATA_QUERY_ID)
//! csrrw x0, 0x7c0, input_len    # number of input u32 words
//! csrrw x0, 0x7c0, input[0]     # input data (if any)
//! csrrw x0, 0x7c0, input[1]
//! ...
//! ```
//!
//! **Read phase** (guest reads response):
//! ```text
//! csrrw response_len, 0x7c0, x0  # first read returns response length in u32 words
//! csrrw response[0], 0x7c0, x0   # subsequent reads return data words
//! csrrw response[1], 0x7c0, x0
//! ...
//! ```

mod error;
mod query_buffer;
pub mod processors;
pub mod query_ids;

pub use error::OracleError;
use query_buffer::QueryBuffer;
use query_ids::{DISCONNECT_ORACLE_QUERY_ID, UART_QUERY_ID};
use std::collections::BTreeMap;

/// Trait for oracle query processors.
///
/// Implement this trait to handle specific query types.
pub trait OracleProcessor: Send {
    /// Returns the list of query IDs this processor handles.
    fn supported_query_ids(&self) -> Vec<u32>;

    /// Process a buffered query and return response iterator.
    ///
    /// # Arguments
    /// * `query_id` - The query type identifier
    /// * `query` - The input data words (usize values)
    ///
    /// # Returns
    /// Iterator yielding response words (usize values)
    fn process_query(
        &mut self,
        query_id: u32,
        query: Vec<usize>,
    ) -> Result<Box<dyn ExactSizeIterator<Item = usize> + Send + 'static>, OracleError>;
}

/// Oracle state machine for CSR 0x7c0 protocol.
///
/// This struct buffers incoming queries, dispatches them to registered
/// processors, and yields responses back to the guest.
pub struct ZiskOracle {
    query_buffer: Option<QueryBuffer>,
    current_iterator: Option<Box<dyn ExactSizeIterator<Item = usize> + Send + 'static>>,
    iterator_len_to_indicate: Option<u32>,
    high_half: Option<u32>,
    is_connected: bool,
    processors: Vec<Box<dyn OracleProcessor>>,
    query_id_to_processor: BTreeMap<u32, usize>,
}

impl Default for ZiskOracle {
    fn default() -> Self {
        Self::new()
    }
}

impl ZiskOracle {
    /// Create a new oracle with no processors.
    pub fn new() -> Self {
        Self {
            query_buffer: None,
            current_iterator: None,
            iterator_len_to_indicate: None,
            high_half: None,
            is_connected: true,
            processors: Vec::new(),
            query_id_to_processor: BTreeMap::new(),
        }
    }

    /// Register a query processor.
    ///
    /// # Panics
    /// Panics if a processor for the same query ID is already registered.
    pub fn add_processor<P: OracleProcessor + 'static>(&mut self, processor: P) {
        let query_ids = processor.supported_query_ids();
        let processor_idx = self.processors.len();

        for id in query_ids {
            if self.query_id_to_processor.contains_key(&id) {
                panic!("Duplicate processor for query ID 0x{:08x}", id);
            }
            self.query_id_to_processor.insert(id, processor_idx);
        }

        self.processors.push(Box::new(processor));
    }

    /// Handle a CSR write (guest writing to oracle).
    ///
    /// Called when guest executes: `csrrw x0, 0x7c0, rs1`
    pub fn write(&mut self, value: u32) {
        // Clear any pending read state on new write
        if self.current_iterator.is_some() {
            self.current_iterator = None;
        }
        self.iterator_len_to_indicate = None;
        self.high_half = None;

        if let Some(buffer) = self.query_buffer.as_mut() {
            if buffer.write(value) {
                // Query complete, dispatch to processor
                self.dispatch_query();
            }
        } else {
            // First write is the query type
            if !self.is_connected && value != UART_QUERY_ID {
                return; // Ignore non-UART queries when disconnected
            }
            self.query_buffer = Some(QueryBuffer::new(value));
        }
    }

    /// Handle a CSR read (guest reading from oracle).
    ///
    /// Called when guest executes: `csrrw rd, 0x7c0, x0`
    pub fn read(&mut self) -> u32 {
        if !self.is_connected {
            return 0;
        }

        // First read after query returns response length
        if let Some(len) = self.iterator_len_to_indicate.take() {
            return len;
        }

        // Return cached high half from previous 64-bit read
        if let Some(high) = self.high_half.take() {
            return high;
        }

        // Read next word from response iterator
        let Some(iter) = self.current_iterator.as_mut() else {
            return 0; // No response available
        };

        let Some(next) = iter.next() else {
            self.current_iterator = None;
            return 0;
        };

        if iter.len() == 0 {
            self.current_iterator = None;
        }

        // Split 64-bit usize into two 32-bit halves
        self.high_half = Some((next >> 32) as u32);
        next as u32
    }

    fn dispatch_query(&mut self) {
        let buffer = self.query_buffer.take().expect("buffer must exist");
        let query_id = buffer.query_type();

        if query_id == DISCONNECT_ORACLE_QUERY_ID {
            self.is_connected = false;
            return;
        }

        let Some(&processor_idx) = self.query_id_to_processor.get(&query_id) else {
            eprintln!("Oracle: Unknown query ID 0x{:08x}", query_id);
            return;
        };

        let processor = &mut self.processors[processor_idx];
        match processor.process_query(query_id, buffer.into_data()) {
            Ok(iter) => {
                let len = iter.len() * 2; // 2 u32s per usize
                self.iterator_len_to_indicate = Some(len as u32);
                if len > 0 {
                    self.current_iterator = Some(iter);
                }
            }
            Err(e) => {
                eprintln!("Oracle processor error: {}", e);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestProcessor {
        response: Vec<usize>,
    }

    impl OracleProcessor for TestProcessor {
        fn supported_query_ids(&self) -> Vec<u32> {
            vec![0x12345678]
        }

        fn process_query(
            &mut self,
            _query_id: u32,
            _query: Vec<usize>,
        ) -> Result<Box<dyn ExactSizeIterator<Item = usize> + Send + 'static>, OracleError> {
            Ok(Box::new(self.response.clone().into_iter()))
        }
    }

    #[test]
    fn test_oracle_write_read_protocol() {
        let mut oracle = ZiskOracle::new();
        oracle.add_processor(TestProcessor { response: vec![0x0000010E_00000001] });

        // Write query type
        oracle.write(0x12345678);

        // Write input length (0 = no input)
        oracle.write(0);

        // Read response length (2 u32s = 1 usize)
        let len = oracle.read();
        assert_eq!(len, 2);

        // Read first u32 (low half of first usize)
        let low = oracle.read();
        assert_eq!(low, 0x00000001);

        // Read second u32 (high half of first usize)
        let high = oracle.read();
        assert_eq!(high, 0x0000010E);
    }

    #[test]
    fn test_oracle_disconnect() {
        let mut oracle = ZiskOracle::new();

        // Write disconnect query
        oracle.write(DISCONNECT_ORACLE_QUERY_ID);
        oracle.write(0); // No input

        // After disconnect, reads return 0
        let value = oracle.read();
        assert_eq!(value, 0);
    }

    #[test]
    #[should_panic(expected = "Duplicate processor")]
    fn test_duplicate_processor_panics() {
        let mut oracle = ZiskOracle::new();

        oracle.add_processor(TestProcessor { response: vec![] });
        oracle.add_processor(TestProcessor { response: vec![] }); // Should panic
    }
}
