//! Oracle types and constants for CSR 0x7c0 (NON_DETERMINISM_CSR).
//!
//! This module contains the zksync-os oracle interface definitions used
//! for non-deterministic data queries during execution.

use std::sync::{Arc, Mutex};

/// CSR 0x7c0 (NON_DETERMINISM_CSR) memory-mapped address
pub const ORACLE_CSR_ADDR: u64 = 0xa000be00;

/// CSR 0x7ca (U256_OPS_WITH_CONTROL) memory-mapped address
/// Used for U256 arithmetic delegation in zksync-os
/// Calculated as: 0xa0008000 + 0x7ca * 8 = 0xa000be50
pub const U256_CSR_ADDR: u64 = 0xa000be50;

/// ZKsyncOS Oracle operation type for CSR 0x7c0 reads/writes.
#[derive(Debug, Clone, Copy)]
pub enum OracleOp {
    /// Read from the oracle CSR.
    Read,
    /// Write to the oracle CSR.
    Write(u64),
}

/// Trait for reading memory from the Zisk emulator.
/// This allows oracle callbacks to read guest memory for operations like modexp.
pub trait ZiskMemoryReader: Send + Sync {
    /// Read a value from memory at the given address with the given width (1, 2, 4, or 8 bytes).
    fn read_mem(&self, addr: u64, width: u64) -> u64;
}

/// Callback for oracle CSR reads/writes.
/// The callback receives the operation and a memory reader for accessing guest memory.
pub type OracleCallback = Arc<Mutex<dyn FnMut(OracleOp, &dyn ZiskMemoryReader) -> u64 + Send>>;
