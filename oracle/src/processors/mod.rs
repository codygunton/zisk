//! Oracle query processors.

mod block_metadata;
mod protocol_replay;
mod replay;
mod uart;

pub use block_metadata::BlockMetadataProcessor;
pub use protocol_replay::ProtocolAwareReplayOracle;
pub use replay::ReplayOracle;
pub use uart::UartProcessor;
