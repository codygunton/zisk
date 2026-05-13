pub mod error;
pub mod templates;

#[cfg(feature = "setup")]
pub mod setup;

pub use error::{RecurserError, Result};
pub use templates::{gen_aggregator, CircomTemplates, StarkInputBlocks};
