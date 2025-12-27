//! U256/BigInt delegation support for zksync-os
//!
//! This module implements U256 arithmetic operations used by CSR 0x7ca delegation
//! in zksync-os. When the guest writes to CSR 0x7ca, the emulator performs U256
//! operations based on control bits.
//!
//! # Protocol
//!
//! The guest sets up registers x10-x12 before triggering the delegation:
//! - x10: Pointer to operand A (read-write, 32 bytes = 8 u32 words)
//! - x11: Pointer to operand B (read-only, 32 bytes = 8 u32 words)
//! - x12: Control mask (input) / overflow flag (output)
//!
//! # Control Bits
//!
//! Exactly one operation bit (0-5, 7) must be set. Bit 6 (CARRY) can be combined.
//! - Bit 0: ADD_OP (a + b [+ 1 if carry])
//! - Bit 1: SUB_OP (a - b [- 1 if carry])
//! - Bit 2: SUB_AND_NEGATE (b - a [- 1 if carry])
//! - Bit 3: MUL_LOW ((a * b) & MASK256)
//! - Bit 4: MUL_HIGH ((a * b) >> 256)
//! - Bit 5: EQ_OP (result = a, overflow = a != b)
//! - Bit 6: CARRY (modifier for other ops)
//! - Bit 7: MEMCOPY (result = b [+ 1 if carry])

use ruint::aliases::{U256, U512};

/// Control bit indices (matching airbender exactly)
pub const ADD_OP_BIT: u8 = 0;
pub const SUB_OP_BIT: u8 = 1;
pub const SUB_AND_NEGATE_OP_BIT: u8 = 2;
pub const MUL_LOW_OP_BIT: u8 = 3;
pub const MUL_HIGH_OP_BIT: u8 = 4;
pub const EQ_OP_BIT: u8 = 5;
pub const CARRY_BIT: u8 = 6;
pub const MEMCOPY_BIT: u8 = 7;

/// Convert 8 u32 words (little-endian limbs) to U256.
///
/// The limbs are arranged as: [w0, w1, w2, w3, w4, w5, w6, w7]
/// where w0 is the least significant and w7 is the most significant.
#[inline]
pub fn u256_from_limbs(limbs: &[u32; 8]) -> U256 {
    let mut bytes = [0u8; 32];
    for (i, limb) in limbs.iter().enumerate() {
        bytes[i * 4..(i + 1) * 4].copy_from_slice(&limb.to_le_bytes());
    }
    U256::from_le_bytes(bytes)
}

/// Convert U256 to 8 u32 words (little-endian limbs).
///
/// The limbs are arranged as: [w0, w1, w2, w3, w4, w5, w6, w7]
/// where w0 is the least significant and w7 is the most significant.
#[inline]
pub fn u256_to_limbs(value: U256) -> [u32; 8] {
    let bytes = value.to_le_bytes::<32>();
    let mut limbs = [0u32; 8];
    for i in 0..8 {
        limbs[i] = u32::from_le_bytes(bytes[i * 4..(i + 1) * 4].try_into().unwrap());
    }
    limbs
}

/// Execute U256 operation based on control mask.
///
/// Returns (result, overflow_flag).
///
/// # Panics
///
/// Panics if not exactly one operation bit is set (excluding CARRY bit).
#[inline]
pub fn execute_u256_op(a: U256, b: U256, control: u8) -> (U256, bool) {
    let carry_set = (control & (1 << CARRY_BIT)) != 0;
    let ops_mask = control & !(1 << CARRY_BIT);

    // Validate: exactly one operation bit must be set
    if ops_mask.count_ones() != 1 {
        panic!(
            "U256 operation requires exactly one operation bit set, got control={:#04x} (ops_mask={:#04x}, count={})",
            control, ops_mask, ops_mask.count_ones()
        );
    }

    match ops_mask {
        x if x == (1 << ADD_OP_BIT) => {
            // ADD: result = a + b [+ 1 if carry]
            let carry_val = if carry_set { U256::from(1) } else { U256::ZERO };
            let (sum1, overflow1) = a.overflowing_add(b);
            let (result, overflow2) = sum1.overflowing_add(carry_val);
            (result, overflow1 || overflow2)
        }
        x if x == (1 << SUB_OP_BIT) => {
            // SUB: result = a - b [- 1 if carry]
            let carry_val = if carry_set { U256::from(1) } else { U256::ZERO };
            let (diff1, underflow1) = a.overflowing_sub(b);
            let (result, underflow2) = diff1.overflowing_sub(carry_val);
            (result, underflow1 || underflow2)
        }
        x if x == (1 << SUB_AND_NEGATE_OP_BIT) => {
            // SUB_NEGATE: result = b - a [- 1 if carry]
            let carry_val = if carry_set { U256::from(1) } else { U256::ZERO };
            let (diff1, underflow1) = b.overflowing_sub(a);
            let (result, underflow2) = diff1.overflowing_sub(carry_val);
            (result, underflow1 || underflow2)
        }
        x if x == (1 << MUL_LOW_OP_BIT) => {
            // MUL_LOW: result = (a * b) & ((1 << 256) - 1)
            // overflow = high part != 0
            let a_wide = U512::from(a);
            let b_wide = U512::from(b);
            let product = a_wide * b_wide;

            // Extract low 256 bits
            let low_bytes: [u8; 64] = product.to_le_bytes();
            let result = U256::from_le_slice(&low_bytes[..32]);

            // Check if high 256 bits are non-zero (overflow)
            let high_part = U256::from_le_slice(&low_bytes[32..64]);
            let overflow = high_part != U256::ZERO;

            (result, overflow)
        }
        x if x == (1 << MUL_HIGH_OP_BIT) => {
            // MUL_HIGH: result = (a * b) >> 256
            // overflow = false (always)
            let a_wide = U512::from(a);
            let b_wide = U512::from(b);
            let product = a_wide * b_wide;

            // Extract high 256 bits
            let bytes: [u8; 64] = product.to_le_bytes();
            let result = U256::from_le_slice(&bytes[32..64]);

            (result, false)
        }
        x if x == (1 << EQ_OP_BIT) => {
            // EQ: result = a, overflow = (a == b)
            // Returns 1 if equal, 0 if not equal
            (a, a == b)
        }
        x if x == (1 << MEMCOPY_BIT) => {
            // MEMCOPY: result = b [+ 1 if carry]
            if carry_set {
                let (result, overflow) = b.overflowing_add(U256::from(1));
                (result, overflow)
            } else {
                (b, false)
            }
        }
        _ => {
            panic!("U256 operation: invalid ops_mask={:#04x}", ops_mask);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_u256_from_to_limbs_roundtrip() {
        // Test zero
        let zero_limbs = [0u32; 8];
        let zero = u256_from_limbs(&zero_limbs);
        assert_eq!(zero, U256::ZERO);
        assert_eq!(u256_to_limbs(zero), zero_limbs);

        // Test one
        let one_limbs = [1, 0, 0, 0, 0, 0, 0, 0];
        let one = u256_from_limbs(&one_limbs);
        assert_eq!(one, U256::from(1));
        assert_eq!(u256_to_limbs(one), one_limbs);

        // Test max
        let max_limbs = [u32::MAX; 8];
        let max = u256_from_limbs(&max_limbs);
        assert_eq!(max, U256::MAX);
        assert_eq!(u256_to_limbs(max), max_limbs);

        // Test specific value
        let specific_limbs = [0x12345678, 0x9ABCDEF0, 0, 0, 0, 0, 0, 0];
        let specific = u256_from_limbs(&specific_limbs);
        assert_eq!(u256_to_limbs(specific), specific_limbs);
    }

    #[test]
    fn test_add_no_overflow() {
        let a = U256::from(100u64);
        let b = U256::from(200u64);
        let control = 1 << ADD_OP_BIT;

        let (result, overflow) = execute_u256_op(a, b, control);
        assert_eq!(result, U256::from(300u64));
        assert!(!overflow);
    }

    #[test]
    fn test_add_with_overflow() {
        let a = U256::MAX;
        let b = U256::from(1u64);
        let control = 1 << ADD_OP_BIT;

        let (result, overflow) = execute_u256_op(a, b, control);
        assert_eq!(result, U256::ZERO);
        assert!(overflow);
    }

    #[test]
    fn test_add_with_carry_bit() {
        let a = U256::from(100u64);
        let b = U256::from(200u64);
        let control = (1 << ADD_OP_BIT) | (1 << CARRY_BIT);

        let (result, overflow) = execute_u256_op(a, b, control);
        assert_eq!(result, U256::from(301u64)); // 100 + 200 + 1
        assert!(!overflow);
    }

    #[test]
    fn test_add_with_carry_overflow() {
        let a = U256::MAX;
        let b = U256::ZERO;
        let control = (1 << ADD_OP_BIT) | (1 << CARRY_BIT);

        let (result, overflow) = execute_u256_op(a, b, control);
        assert_eq!(result, U256::ZERO); // MAX + 0 + 1 = overflow
        assert!(overflow);
    }

    #[test]
    fn test_sub_no_underflow() {
        let a = U256::from(300u64);
        let b = U256::from(100u64);
        let control = 1 << SUB_OP_BIT;

        let (result, overflow) = execute_u256_op(a, b, control);
        assert_eq!(result, U256::from(200u64));
        assert!(!overflow);
    }

    #[test]
    fn test_sub_with_underflow() {
        let a = U256::from(100u64);
        let b = U256::from(200u64);
        let control = 1 << SUB_OP_BIT;

        let (result, overflow) = execute_u256_op(a, b, control);
        // Wraps around
        assert!(overflow);
    }

    #[test]
    fn test_sub_with_carry_bit() {
        let a = U256::from(300u64);
        let b = U256::from(100u64);
        let control = (1 << SUB_OP_BIT) | (1 << CARRY_BIT);

        let (result, overflow) = execute_u256_op(a, b, control);
        assert_eq!(result, U256::from(199u64)); // 300 - 100 - 1
        assert!(!overflow);
    }

    #[test]
    fn test_sub_with_carry_underflow() {
        let a = U256::ZERO;
        let b = U256::ZERO;
        let control = (1 << SUB_OP_BIT) | (1 << CARRY_BIT);

        let (result, overflow) = execute_u256_op(a, b, control);
        assert_eq!(result, U256::MAX); // 0 - 0 - 1 = underflow
        assert!(overflow);
    }

    #[test]
    fn test_sub_and_negate() {
        let a = U256::from(100u64);
        let b = U256::from(300u64);
        let control = 1 << SUB_AND_NEGATE_OP_BIT;

        let (result, overflow) = execute_u256_op(a, b, control);
        assert_eq!(result, U256::from(200u64)); // b - a = 300 - 100
        assert!(!overflow);
    }

    #[test]
    fn test_sub_and_negate_with_carry() {
        let a = U256::from(100u64);
        let b = U256::from(300u64);
        let control = (1 << SUB_AND_NEGATE_OP_BIT) | (1 << CARRY_BIT);

        let (result, overflow) = execute_u256_op(a, b, control);
        assert_eq!(result, U256::from(199u64)); // b - a - 1 = 300 - 100 - 1
        assert!(!overflow);
    }

    #[test]
    fn test_mul_low_small_values() {
        let a = U256::from(100u64);
        let b = U256::from(200u64);
        let control = 1 << MUL_LOW_OP_BIT;

        let (result, overflow) = execute_u256_op(a, b, control);
        assert_eq!(result, U256::from(20000u64));
        assert!(!overflow);
    }

    #[test]
    fn test_mul_low_overflow_detection() {
        // Use large values that will overflow when multiplied
        let a = U256::MAX;
        let b = U256::from(2u64);
        let control = 1 << MUL_LOW_OP_BIT;

        let (result, overflow) = execute_u256_op(a, b, control);
        // MAX * 2 = 2*MAX which overflows
        // Low part: (MAX * 2) & MASK256 = MAX - 1 (since MAX * 2 = 2^257 - 2)
        assert_eq!(result, U256::MAX - U256::from(1));
        assert!(overflow);
    }

    #[test]
    fn test_mul_high() {
        // Use values that produce a non-zero high part
        let a = U256::MAX;
        let b = U256::from(2u64);
        let control = 1 << MUL_HIGH_OP_BIT;

        let (result, overflow) = execute_u256_op(a, b, control);
        // MAX * 2 >> 256 = 1
        assert_eq!(result, U256::from(1u64));
        assert!(!overflow); // MUL_HIGH never sets overflow
    }

    #[test]
    fn test_mul_high_small_values() {
        let a = U256::from(100u64);
        let b = U256::from(200u64);
        let control = 1 << MUL_HIGH_OP_BIT;

        let (result, overflow) = execute_u256_op(a, b, control);
        assert_eq!(result, U256::ZERO); // Small values don't overflow into high bits
        assert!(!overflow);
    }

    #[test]
    fn test_eq_equal_values() {
        let a = U256::from(12345u64);
        let b = U256::from(12345u64);
        let control = 1 << EQ_OP_BIT;

        let (result, overflow) = execute_u256_op(a, b, control);
        assert_eq!(result, a);
        assert!(overflow); // Equal values => overflow = true (returns 1)
    }

    #[test]
    fn test_eq_unequal_values() {
        let a = U256::from(12345u64);
        let b = U256::from(54321u64);
        let control = 1 << EQ_OP_BIT;

        let (result, overflow) = execute_u256_op(a, b, control);
        assert_eq!(result, a);
        assert!(!overflow); // Unequal values => overflow = false (returns 0)
    }

    #[test]
    fn test_memcopy_without_carry() {
        let a = U256::from(111u64);
        let b = U256::from(222u64);
        let control = 1 << MEMCOPY_BIT;

        let (result, overflow) = execute_u256_op(a, b, control);
        assert_eq!(result, b);
        assert!(!overflow);
    }

    #[test]
    fn test_memcopy_with_carry() {
        let a = U256::from(111u64);
        let b = U256::from(222u64);
        let control = (1 << MEMCOPY_BIT) | (1 << CARRY_BIT);

        let (result, overflow) = execute_u256_op(a, b, control);
        assert_eq!(result, U256::from(223u64)); // b + 1
        assert!(!overflow);
    }

    #[test]
    fn test_memcopy_with_carry_overflow() {
        let a = U256::from(111u64);
        let b = U256::MAX;
        let control = (1 << MEMCOPY_BIT) | (1 << CARRY_BIT);

        let (result, overflow) = execute_u256_op(a, b, control);
        assert_eq!(result, U256::ZERO); // MAX + 1 = overflow
        assert!(overflow);
    }

    #[test]
    #[should_panic(expected = "exactly one operation bit set")]
    fn test_invalid_control_no_op() {
        let a = U256::from(1u64);
        let b = U256::from(2u64);
        let control = 0; // No operation bit set

        execute_u256_op(a, b, control);
    }

    #[test]
    #[should_panic(expected = "exactly one operation bit set")]
    fn test_invalid_control_multiple_ops() {
        let a = U256::from(1u64);
        let b = U256::from(2u64);
        let control = (1 << ADD_OP_BIT) | (1 << SUB_OP_BIT); // Two operation bits

        execute_u256_op(a, b, control);
    }

    #[test]
    fn test_carry_bit_alone_is_invalid() {
        // CARRY bit alone (without an operation) should panic
        let a = U256::from(1u64);
        let b = U256::from(2u64);
        let control = 1 << CARRY_BIT; // Only carry bit

        let result = std::panic::catch_unwind(|| execute_u256_op(a, b, control));
        assert!(result.is_err());
    }
}
