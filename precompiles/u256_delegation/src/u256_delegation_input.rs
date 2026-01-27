// ⚠️  WARNING: This module was AI-generated and has NOT undergone thorough human review.

//! Input data structure for U256 delegation operations.

/// Control mask bits for U256 operations (matches Airbender protocol)
pub const U256_OP_ADD: u32 = 0x01;
pub const U256_OP_SUB: u32 = 0x02;
pub const U256_OP_SUB_NEGATE: u32 = 0x04;
pub const U256_OP_MUL_LOW: u32 = 0x08;
pub const U256_OP_MUL_HIGH: u32 = 0x10;
pub const U256_OP_EQ: u32 = 0x20;
pub const U256_OP_MEMCPY: u32 = 0x80;
pub const U256_OP_CARRY_BIT: u32 = 0x40;

/// Mask to extract operation type (excludes carry bit)
pub const U256_OP_MASK: u32 = 0xBF;

/// U256 operation types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum U256Operation {
    Add = 0,
    Sub = 1,
    SubNegate = 2,
    MulLow = 3,
    MulHigh = 4,
    Eq = 5,
    MemCpy = 6,
}

impl U256Operation {
    /// Parse operation from control mask
    pub fn from_control(control: u32) -> Self {
        match control & U256_OP_MASK {
            U256_OP_ADD => Self::Add,
            U256_OP_SUB => Self::Sub,
            U256_OP_SUB_NEGATE => Self::SubNegate,
            U256_OP_MUL_LOW => Self::MulLow,
            U256_OP_MUL_HIGH => Self::MulHigh,
            U256_OP_EQ => Self::Eq,
            U256_OP_MEMCPY => Self::MemCpy,
            other => panic!("Invalid U256 control mask: 0x{:02x}", other),
        }
    }
}

/// Input data for a single U256 delegation operation.
///
/// This structure captures all information needed to verify a U256 delegation
/// operation in the proof system.
#[derive(Debug, Clone, Default)]
pub struct U256DelegationInput {
    /// CPU step number when this operation was executed
    pub step_main: u64,
    /// Memory address of operand A (x10 register value)
    pub addr_a: u64,
    /// Memory address of operand B (x11 register value)
    pub addr_b: u64,
    /// Control mask from x12 register (operation selector + carry flag)
    pub control: u32,
    /// Operand A value (8 × u32 limbs, little-endian)
    pub a: [u32; 8],
    /// Operand B value (8 × u32 limbs, little-endian)
    pub b: [u32; 8],
    /// Result value (8 × u32 limbs, little-endian)
    pub result: [u32; 8],
    /// Overflow/carry flag output (written to x12)
    pub overflow: bool,
}

impl U256DelegationInput {
    /// Create a new U256DelegationInput
    pub fn new(
        step_main: u64,
        addr_a: u64,
        addr_b: u64,
        control: u32,
        a: [u32; 8],
        b: [u32; 8],
        result: [u32; 8],
        overflow: bool,
    ) -> Self {
        Self { step_main, addr_a, addr_b, control, a, b, result, overflow }
    }

    /// Get the operation type
    pub fn operation(&self) -> U256Operation {
        U256Operation::from_control(self.control)
    }

    /// Check if carry/borrow input is set
    pub fn has_carry_in(&self) -> bool {
        (self.control & U256_OP_CARRY_BIT) != 0
    }

    /// Reconstruct operand A as a 256-bit value (for verification)
    pub fn a_as_u256(&self) -> [u64; 4] {
        [
            self.a[0] as u64 | ((self.a[1] as u64) << 32),
            self.a[2] as u64 | ((self.a[3] as u64) << 32),
            self.a[4] as u64 | ((self.a[5] as u64) << 32),
            self.a[6] as u64 | ((self.a[7] as u64) << 32),
        ]
    }

    /// Reconstruct operand B as a 256-bit value (for verification)
    pub fn b_as_u256(&self) -> [u64; 4] {
        [
            self.b[0] as u64 | ((self.b[1] as u64) << 32),
            self.b[2] as u64 | ((self.b[3] as u64) << 32),
            self.b[4] as u64 | ((self.b[5] as u64) << 32),
            self.b[6] as u64 | ((self.b[7] as u64) << 32),
        ]
    }

    /// Reconstruct result as a 256-bit value (for verification)
    pub fn result_as_u256(&self) -> [u64; 4] {
        [
            self.result[0] as u64 | ((self.result[1] as u64) << 32),
            self.result[2] as u64 | ((self.result[3] as u64) << 32),
            self.result[4] as u64 | ((self.result[5] as u64) << 32),
            self.result[6] as u64 | ((self.result[7] as u64) << 32),
        ]
    }

    /// Parse U256DelegationInput from operation bus data.
    ///
    /// Bus payload layout:
    /// - [0]: op (U256_OP = 0xca)
    /// - [1]: op_type (U256Delegation = 11)
    /// - [2]: step
    /// - [3]: control
    /// - [4]: addr_a
    /// - [5]: addr_b
    /// - [6..10]: a_limbs (4 u64, each containing 2 u32 limbs)
    /// - [10..14]: b_limbs (4 u64)
    /// - [14..18]: result_limbs (4 u64)
    /// - [18]: overflow
    pub fn from_bus_data(data: &[u64]) -> Self {
        use zisk_common::{
            U256_A_LIMBS_START, U256_ADDR_A, U256_ADDR_B, U256_B_LIMBS_START, U256_CONTROL,
            U256_OVERFLOW, U256_RESULT_LIMBS_START, U256_STEP,
        };

        // Unpack u64 into pairs of u32 limbs
        let unpack_limbs = |packed: &[u64]| -> [u32; 8] {
            [
                (packed[0] & 0xFFFF_FFFF) as u32,
                ((packed[0] >> 32) & 0xFFFF_FFFF) as u32,
                (packed[1] & 0xFFFF_FFFF) as u32,
                ((packed[1] >> 32) & 0xFFFF_FFFF) as u32,
                (packed[2] & 0xFFFF_FFFF) as u32,
                ((packed[2] >> 32) & 0xFFFF_FFFF) as u32,
                (packed[3] & 0xFFFF_FFFF) as u32,
                ((packed[3] >> 32) & 0xFFFF_FFFF) as u32,
            ]
        };

        Self {
            step_main: data[U256_STEP],
            addr_a: data[U256_ADDR_A],
            addr_b: data[U256_ADDR_B],
            control: data[U256_CONTROL] as u32,
            a: unpack_limbs(&data[U256_A_LIMBS_START..U256_A_LIMBS_START + 4]),
            b: unpack_limbs(&data[U256_B_LIMBS_START..U256_B_LIMBS_START + 4]),
            result: unpack_limbs(&data[U256_RESULT_LIMBS_START..U256_RESULT_LIMBS_START + 4]),
            overflow: data[U256_OVERFLOW] != 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operation_from_control() {
        assert_eq!(U256Operation::from_control(0x01), U256Operation::Add);
        assert_eq!(U256Operation::from_control(0x02), U256Operation::Sub);
        assert_eq!(U256Operation::from_control(0x04), U256Operation::SubNegate);
        assert_eq!(U256Operation::from_control(0x08), U256Operation::MulLow);
        assert_eq!(U256Operation::from_control(0x10), U256Operation::MulHigh);
        assert_eq!(U256Operation::from_control(0x20), U256Operation::Eq);
        assert_eq!(U256Operation::from_control(0x80), U256Operation::MemCpy);

        // With carry bit set
        assert_eq!(U256Operation::from_control(0x41), U256Operation::Add);
        assert_eq!(U256Operation::from_control(0x42), U256Operation::Sub);
    }

    #[test]
    fn test_has_carry_in() {
        let mut input = U256DelegationInput::default();
        input.control = 0x01; // ADD without carry
        assert!(!input.has_carry_in());

        input.control = 0x41; // ADD with carry
        assert!(input.has_carry_in());
    }
}
