//! Query buffer for accumulating CSR writes.

/// Buffers incoming query words until the query is complete.
///
/// The protocol is:
/// 1. First write: query type (u32)
/// 2. Second write: input length in u32 words
/// 3. Subsequent writes: input data (u32 words packed into usize)
pub struct QueryBuffer {
    query_type: u32,
    expected_len: Option<usize>,
    buffer: Vec<usize>,
    write_low: bool,
}

impl QueryBuffer {
    /// Create a new buffer for the given query type.
    pub fn new(query_type: u32) -> Self {
        Self { query_type, expected_len: None, buffer: Vec::new(), write_low: true }
    }

    /// Write a 32-bit word to the buffer.
    ///
    /// Returns `true` when the query is complete.
    pub fn write(&mut self, value: u32) -> bool {
        if let Some(expected) = self.expected_len.as_mut() {
            // Accumulating data words (32-bit to 64-bit packing)
            if self.write_low {
                self.buffer.push(value as usize);
                self.write_low = false;
            } else {
                let last = self.buffer.last_mut().expect("buffer not empty");
                *last |= (value as usize) << 32;
                self.write_low = true;
            }
            *expected -= 1;
            *expected == 0
        } else {
            // Second write is the input length
            self.expected_len = Some(value as usize);
            value == 0 // Complete if no input words expected
        }
    }

    /// Get the query type.
    pub fn query_type(&self) -> u32 {
        self.query_type
    }

    /// Consume the buffer and return the accumulated data.
    pub fn into_data(self) -> Vec<usize> {
        self.buffer
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_buffer_empty_input() {
        let mut buffer = QueryBuffer::new(0x40070000);

        // Write input length = 0
        let complete = buffer.write(0);
        assert!(complete);
        assert_eq!(buffer.query_type(), 0x40070000);
        assert!(buffer.into_data().is_empty());
    }

    #[test]
    fn test_query_buffer_with_data() {
        let mut buffer = QueryBuffer::new(0x40070000);

        // Write input length = 2 (2 u32 words = 1 usize)
        assert!(!buffer.write(2));

        // Write first u32 (low half)
        assert!(!buffer.write(0xDEADBEEF));

        // Write second u32 (high half) - completes the query
        assert!(buffer.write(0xCAFEBABE));

        let data = buffer.into_data();
        assert_eq!(data.len(), 1);
        assert_eq!(data[0], 0xCAFEBABE_DEADBEEF_usize);
    }
}
