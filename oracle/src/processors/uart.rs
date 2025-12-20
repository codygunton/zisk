//! UART debug output processor.
//!
//! Handles UART_QUERY_ID (0xffffffff) for guest program debug output.
//!
//! # Example
//!
//! ```
//! use zisk_oracle::processors::UartProcessor;
//! use zisk_oracle::ZiskOracle;
//!
//! // Simple usage - prints to stderr with "[GUEST] " prefix
//! let mut oracle = ZiskOracle::new();
//! oracle.add_processor(UartProcessor::default());
//!
//! // Custom configuration
//! let uart = UartProcessor::builder()
//!     .prefix("[VM] ")
//!     .line_buffered(true)
//!     .build();
//! ```

use crate::query_ids::UART_QUERY_ID;
use crate::{OracleError, OracleProcessor};
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

/// Configuration for UART output behavior.
#[derive(Clone)]
pub struct UartConfig {
    /// Prefix to prepend to each line of output.
    pub prefix: String,
    /// Whether to buffer output until a newline is received.
    pub line_buffered: bool,
    /// Whether UART output is enabled.
    pub enabled: bool,
}

impl Default for UartConfig {
    fn default() -> Self {
        Self { prefix: "[GUEST] ".to_string(), line_buffered: true, enabled: true }
    }
}

/// Builder for configuring a UartProcessor.
pub struct UartBuilder {
    config: UartConfig,
    writer: Option<Arc<Mutex<dyn Write + Send>>>,
}

impl UartBuilder {
    /// Create a new builder with default settings.
    pub fn new() -> Self {
        Self { config: UartConfig::default(), writer: None }
    }

    /// Set the prefix prepended to each line.
    pub fn prefix(mut self, prefix: impl Into<String>) -> Self {
        self.config.prefix = prefix.into();
        self
    }

    /// Set whether to buffer output until newlines.
    ///
    /// When enabled, output is collected until a newline character is seen,
    /// then the entire line is printed with the prefix. This produces cleaner
    /// output when the guest writes character-by-character.
    pub fn line_buffered(mut self, buffered: bool) -> Self {
        self.config.line_buffered = buffered;
        self
    }

    /// Enable or disable UART output entirely.
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.config.enabled = enabled;
        self
    }

    /// Set a custom writer for UART output.
    ///
    /// By default, output goes to stderr. Use this to capture output
    /// for testing or logging purposes.
    pub fn writer<W: Write + Send + 'static>(mut self, writer: W) -> Self {
        self.writer = Some(Arc::new(Mutex::new(writer)));
        self
    }

    /// Build the configured UartProcessor.
    pub fn build(self) -> UartProcessor {
        UartProcessor {
            config: self.config,
            writer: self.writer,
            line_buffer: String::new(),
        }
    }
}

impl Default for UartBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Processor for UART_QUERY_ID (0xffffffff) - debug output from guest programs.
///
/// Receives bytes from the guest and outputs them with optional line buffering
/// and prefixing for easier identification in logs.
pub struct UartProcessor {
    config: UartConfig,
    writer: Option<Arc<Mutex<dyn Write + Send>>>,
    line_buffer: String,
}

impl UartProcessor {
    /// Create a new UART processor with default settings.
    ///
    /// Default configuration:
    /// - Prefix: "[GUEST] "
    /// - Line buffered: true
    /// - Output: stderr
    pub fn new() -> Self {
        Self::builder().build()
    }

    /// Create a builder for custom configuration.
    pub fn builder() -> UartBuilder {
        UartBuilder::new()
    }

    /// Create a UART processor that captures output to a shared buffer.
    ///
    /// Useful for testing or capturing guest output programmatically.
    pub fn capturing() -> (Self, Arc<Mutex<Vec<u8>>>) {
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let writer = Arc::clone(&buffer);
        let processor =
            Self::builder().writer(CaptureWriter { buffer: Arc::clone(&buffer) }).build();
        (processor, writer)
    }

    /// Create a silent UART processor that discards all output.
    pub fn silent() -> Self {
        Self::builder().enabled(false).build()
    }

    fn write_output(&mut self, data: &str) {
        if !self.config.enabled {
            return;
        }

        if let Some(ref writer) = self.writer {
            if let Ok(mut w) = writer.lock() {
                let _ = w.write_all(data.as_bytes());
                let _ = w.flush();
            }
        } else {
            eprint!("{}", data);
        }
    }

    fn flush_line(&mut self) {
        if !self.line_buffer.is_empty() {
            let line = std::mem::take(&mut self.line_buffer);
            self.write_output(&format!("{}{}\n", self.config.prefix, line));
        }
    }

    fn process_char(&mut self, c: char) {
        if self.config.line_buffered {
            if c == '\n' {
                self.flush_line();
            } else if c != '\r' {
                // Skip carriage returns, buffer everything else
                self.line_buffer.push(c);
            }
        } else {
            // Immediate output mode
            if c == '\n' {
                self.write_output(&format!("{}\n", self.config.prefix));
            } else {
                self.write_output(&c.to_string());
            }
        }
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
        if !self.config.enabled {
            return Ok(Box::new(std::iter::empty()));
        }

        // Process each word as bytes
        for word in query {
            let bytes = word.to_le_bytes();
            for &b in &bytes {
                if b != 0 {
                    self.process_char(b as char);
                }
            }
        }

        // UART queries return empty response
        Ok(Box::new(std::iter::empty()))
    }
}

/// Helper struct for capturing output to a Vec<u8>.
struct CaptureWriter {
    buffer: Arc<Mutex<Vec<u8>>>,
}

impl Write for CaptureWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if let Ok(mut buffer) = self.buffer.lock() {
            buffer.extend_from_slice(buf);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
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
        let mut processor = UartProcessor::silent();

        let result = processor.process_query(UART_QUERY_ID, vec![0x41424344]); // "DCBA" in LE
        assert!(result.is_ok());

        let iter = result.expect("should succeed");
        assert_eq!(iter.len(), 0, "UART should return empty response");
    }

    #[test]
    fn test_uart_capturing() {
        let (mut processor, buffer) = UartProcessor::capturing();

        // Send "Hello\n" as bytes packed into usize words
        // 'H' = 0x48, 'e' = 0x65, 'l' = 0x6c, 'l' = 0x6c, 'o' = 0x6f, '\n' = 0x0a
        let hello: usize = 0x6c6c6548; // "lleH" in LE (first 4 bytes)
        let world: usize = 0x000a6f; // "\no" in LE

        processor.process_query(UART_QUERY_ID, vec![hello, world]).expect("should succeed");

        let output = buffer.lock().expect("lock");
        let output_str = String::from_utf8_lossy(&output);
        assert!(output_str.contains("[GUEST]"), "Should have prefix");
        assert!(output_str.contains("Hello"), "Should contain Hello");
    }

    #[test]
    fn test_uart_line_buffering() {
        let (mut processor, buffer) = UartProcessor::capturing();

        // Send characters one at a time
        let chars = ['H', 'i', '\n'];
        for c in chars {
            let word = c as usize;
            processor.process_query(UART_QUERY_ID, vec![word]).expect("should succeed");
        }

        let output = buffer.lock().expect("lock");
        let output_str = String::from_utf8_lossy(&output);
        // Line buffered: should only output after newline
        assert_eq!(output_str.matches("[GUEST]").count(), 1);
        assert!(output_str.contains("Hi"));
    }

    #[test]
    fn test_uart_custom_prefix() {
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let mut processor = UartProcessor::builder()
            .prefix("[VM] ")
            .writer(CaptureWriter { buffer: Arc::clone(&buffer) })
            .build();

        // Send "X\n"
        let word = (('\n' as usize) << 8) | ('X' as usize);
        processor.process_query(UART_QUERY_ID, vec![word]).expect("should succeed");

        let output = buffer.lock().expect("lock");
        let output_str = String::from_utf8_lossy(&output);
        assert!(output_str.contains("[VM] X"));
    }

    #[test]
    fn test_uart_disabled() {
        let buffer = Arc::new(Mutex::new(Vec::new()));
        let mut processor = UartProcessor::builder()
            .enabled(false)
            .writer(CaptureWriter { buffer: Arc::clone(&buffer) })
            .build();

        processor.process_query(UART_QUERY_ID, vec![0x0a48656c]).expect("should succeed");

        let output = buffer.lock().expect("lock");
        assert!(output.is_empty(), "Disabled UART should produce no output");
    }
}
