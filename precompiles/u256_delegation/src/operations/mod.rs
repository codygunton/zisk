// ⚠️  WARNING: This module was AI-generated and has NOT undergone thorough human review.

//! U256 arithmetic operations for delegation precompile.
//!
//! These functions implement the core 256-bit arithmetic operations that are
//! verified by the PIL constraints. They match the Airbender bigint_with_control
//! circuit semantics exactly.

mod add;
mod mul;
mod eq;

pub use add::{u256_add, u256_sub, u256_sub_negate};
pub use mul::{u256_mul_low, u256_mul_high};
pub use eq::{u256_eq, u256_memcpy};

use crate::{U256DelegationInput, U256Operation};

/// Execute a U256 operation and verify the result matches the input.
///
/// This is used during witness computation to validate that the emulator
/// computed the operation correctly.
pub fn verify_operation(input: &U256DelegationInput) -> bool {
    let carry_in = input.has_carry_in();

    let (expected_result, expected_overflow) = match input.operation() {
        U256Operation::Add => u256_add(&input.a, &input.b, carry_in),
        U256Operation::Sub => u256_sub(&input.a, &input.b, carry_in),
        U256Operation::SubNegate => u256_sub_negate(&input.a, &input.b, carry_in),
        U256Operation::MulLow => u256_mul_low(&input.a, &input.b),
        U256Operation::MulHigh => u256_mul_high(&input.a, &input.b),
        U256Operation::Eq => {
            let is_eq = u256_eq(&input.a, &input.b);
            // For EQ, result is unchanged (operand A), overflow indicates equality
            return input.result == input.a && input.overflow == is_eq;
        }
        U256Operation::MemCpy => u256_memcpy(&input.b, carry_in),
    };

    input.result == expected_result && input.overflow == expected_overflow
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_simple() {
        let a = [1, 0, 0, 0, 0, 0, 0, 0];
        let b = [2, 0, 0, 0, 0, 0, 0, 0];
        let (result, overflow) = u256_add(&a, &b, false);
        assert_eq!(result, [3, 0, 0, 0, 0, 0, 0, 0]);
        assert!(!overflow);
    }

    #[test]
    fn test_add_with_carry_propagation() {
        let a = [0xFFFFFFFF, 0, 0, 0, 0, 0, 0, 0];
        let b = [1, 0, 0, 0, 0, 0, 0, 0];
        let (result, overflow) = u256_add(&a, &b, false);
        assert_eq!(result, [0, 1, 0, 0, 0, 0, 0, 0]);
        assert!(!overflow);
    }

    #[test]
    fn test_add_overflow() {
        let a = [0xFFFFFFFF; 8];
        let b = [1, 0, 0, 0, 0, 0, 0, 0];
        let (result, overflow) = u256_add(&a, &b, false);
        assert_eq!(result, [0, 0, 0, 0, 0, 0, 0, 0]);
        assert!(overflow);
    }

    #[test]
    fn test_sub_simple() {
        let a = [5, 0, 0, 0, 0, 0, 0, 0];
        let b = [3, 0, 0, 0, 0, 0, 0, 0];
        let (result, overflow) = u256_sub(&a, &b, false);
        assert_eq!(result, [2, 0, 0, 0, 0, 0, 0, 0]);
        assert!(!overflow);
    }

    #[test]
    fn test_sub_with_borrow() {
        let a = [0, 1, 0, 0, 0, 0, 0, 0];
        let b = [1, 0, 0, 0, 0, 0, 0, 0];
        let (result, overflow) = u256_sub(&a, &b, false);
        assert_eq!(result, [0xFFFFFFFF, 0, 0, 0, 0, 0, 0, 0]);
        assert!(!overflow);
    }

    #[test]
    fn test_mul_low() {
        let a = [2, 0, 0, 0, 0, 0, 0, 0];
        let b = [3, 0, 0, 0, 0, 0, 0, 0];
        let (result, overflow) = u256_mul_low(&a, &b);
        assert_eq!(result, [6, 0, 0, 0, 0, 0, 0, 0]);
        assert!(!overflow);
    }

    #[test]
    fn test_eq_equal() {
        let a = [1, 2, 3, 4, 5, 6, 7, 8];
        let b = [1, 2, 3, 4, 5, 6, 7, 8];
        assert!(u256_eq(&a, &b));
    }

    #[test]
    fn test_eq_not_equal() {
        let a = [1, 2, 3, 4, 5, 6, 7, 8];
        let b = [1, 2, 3, 4, 5, 6, 7, 9];
        assert!(!u256_eq(&a, &b));
    }
}
