//! Oracle query ID constants.
//!
//! These values match the zk_ee oracle query ID definitions.

/// Top bit (0x80_00_00_00) reserved
pub const RESERVED_SUBSPACE_MASK: u32 = 0x80_00_00_00;

/// Second bit (0x40_00_00_00) for basic oracle functionality
pub const BASIC_SUBSPACE_MASK: u32 = 0x40_00_00_00;

/// System-level queries (e.g. disconnect oracle)
pub const SYSTEM_SUBSPACE_MASK: u32 = BASIC_SUBSPACE_MASK | 0x00_00_00_00; // 0x40000000

/// Transaction-related queries
pub const TRANSACTION_SUBSPACE_MASK: u32 = BASIC_SUBSPACE_MASK | 0x00_06_00_00; // 0x40060000

/// Block- (and batch-) related queries
pub const BLOCK_SUBSPACE_MASK: u32 = BASIC_SUBSPACE_MASK | 0x00_07_00_00; // 0x40070000

/// Special case: UART output query ID (for debugging purposes)
pub const UART_QUERY_ID: u32 = 0xff_ff_ff_ff;

/// Signal to disconnect from external oracle and switch to autonomous execution mode
pub const DISCONNECT_ORACLE_QUERY_ID: u32 = SYSTEM_SUBSPACE_MASK | 0; // 0x40000000

/// Query to get the size (in bytes) of the next transaction to be processed
pub const NEXT_TX_SIZE_QUERY_ID: u32 = TRANSACTION_SUBSPACE_MASK | 0; // 0x40060000

/// Query to get transaction data words for the current transaction being processed
pub const TX_DATA_WORDS_QUERY_ID: u32 = TRANSACTION_SUBSPACE_MASK | 1; // 0x40060001

/// Query to retrieve block metadata (timestamp, number, etc.) from the oracle
pub const BLOCK_METADATA_QUERY_ID: u32 = BLOCK_SUBSPACE_MASK | 0; // 0x40070000

/// Query to get the data required for state correctness proving
pub const ZK_PROOF_DATA_INIT_QUERY_ID: u32 = BLOCK_SUBSPACE_MASK | 1; // 0x40070001
