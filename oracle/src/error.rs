//! Oracle error types.

use thiserror::Error;

/// Errors that can occur during oracle operations.
#[derive(Error, Debug)]
pub enum OracleError {
    /// Unknown query ID was received.
    #[error("Unknown query ID: 0x{0:08x}")]
    UnknownQueryId(u32),

    /// Query buffer received more data than expected.
    #[error("Query buffer overflow: received {received} words, expected {expected}")]
    BufferOverflow { received: usize, expected: usize },

    /// No response is available for reading.
    #[error("No response available")]
    NoResponse,

    /// A processor encountered an error.
    #[error("Processor error: {0}")]
    ProcessorError(String),
}
