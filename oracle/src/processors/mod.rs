//! Oracle query processors.

mod block_metadata;
mod replay;
mod uart;

pub use block_metadata::BlockMetadataProcessor;
pub use replay::{Replay64Oracle, ReplayOracle};
pub use uart::{UartBuilder, UartConfig, UartProcessor};
