//! UART debug output processor.

use crate::query_ids::UART_QUERY_ID;
use crate::{OracleError, OracleProcessor};

/// Processor for UART_QUERY_ID (0xffffffff) - debug output.
///
/// Prints bytes received in the query to stdout.
/// Returns an empty response.
pub struct UartProcessor;

impl UartProcessor {
    pub fn new() -> Self {
        Self
    }
}

impl Default for UartProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl OracleProcessor for UartProcessor {
    fn supported_query_ids(&self) -> Vec<u32> {
        vec![UART_QUERY_ID]
    }

    fn process_query(
        &mut self,
        _query_id: u32,
        query: Vec<usize>,
    ) -> Result<Box<dyn ExactSizeIterator<Item = usize> + Send + 'static>, OracleError> {
        // UART query contains bytes to print
        for word in query {
            let bytes = word.to_le_bytes();
            for &b in &bytes {
                if b != 0 {
                    print!("{}", b as char);
                }
            }
        }

        // UART queries return empty response
        Ok(Box::new(std::iter::empty()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_uart_processor_supported_ids() {
        let processor = UartProcessor::new();
        let ids = processor.supported_query_ids();
        assert_eq!(ids, vec![UART_QUERY_ID]);
    }

    #[test]
    fn test_uart_processor_returns_empty() {
        let mut processor = UartProcessor::new();

        let result = processor.process_query(UART_QUERY_ID, vec![0x41424344]); // "DCBA" in LE
        assert!(result.is_ok());

        let iter = result.expect("should succeed");
        assert_eq!(iter.len(), 0, "UART should return empty response");
    }
}
