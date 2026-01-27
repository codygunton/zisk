//--BIG WARNING HERE TOO and on every file in this new precompile
//! U256 Delegation Precompile
//!
//! This precompile handles 256-bit arithmetic operations delegated via CSR 0x7ca.
//! It follows the Airbender U256 delegation protocol where:
//! - x10 (a0) = pointer to operand A (also destination for result)
//! - x11 (a1) = pointer to operand B
//! - x12 (a2) = control mask (operation selector + carry flag)
//!
//! Supported operations (selected by control mask):
//! - ADD (0x01): A + B with optional carry
//! - SUB (0x02): A - B with optional borrow
//! - SUB_NEGATE (0x04): B - A with optional borrow
//! - MUL_LOW (0x08): Low 256 bits of A * B
//! - MUL_HIGH (0x10): High 256 bits of A * B
//! - EQ (0x20): Compare A == B
//! - MEMCPY (0x80): Copy B to A with optional +1

mod operations;
mod u256_delegation_bus_device;
mod u256_delegation_input;
mod u256_delegation_instance;
mod u256_delegation_manager;
mod u256_delegation_planner;
mod u256_delegation_sm;

pub use operations::*;
pub use u256_delegation_bus_device::*;
pub use u256_delegation_input::*;
pub use u256_delegation_instance::*;
pub use u256_delegation_manager::*;
pub use u256_delegation_planner::*;
pub use u256_delegation_sm::*;
