//! 256-bit addition and subtraction operations.

/// Compute 256-bit addition with optional carry input.
///
/// Returns (result, overflow) where:
/// - result: 256-bit sum as 8 × u32 limbs (little-endian)
/// - overflow: true if addition overflowed (carry out of MSB)
#[inline]
pub fn u256_add(a: &[u32; 8], b: &[u32; 8], carry_in: bool) -> ([u32; 8], bool) {
    let mut result = [0u32; 8];
    let mut carry = carry_in as u64;

    for i in 0..8 {
        let sum = a[i] as u64 + b[i] as u64 + carry;
        result[i] = sum as u32;
        carry = sum >> 32;
    }

    (result, carry != 0)
}

/// Compute 256-bit subtraction with optional borrow input.
///
/// Computes A - B - borrow_in.
///
/// Returns (result, underflow) where:
/// - result: 256-bit difference as 8 × u32 limbs (little-endian)
/// - underflow: true if subtraction underflowed (borrow out of MSB)
#[inline]
pub fn u256_sub(a: &[u32; 8], b: &[u32; 8], borrow_in: bool) -> ([u32; 8], bool) {
    let mut result = [0u32; 8];
    let mut borrow = borrow_in as u64;

    for i in 0..8 {
        let a_val = a[i] as u64;
        let b_val = b[i] as u64 + borrow;

        if a_val >= b_val {
            result[i] = (a_val - b_val) as u32;
            borrow = 0;
        } else {
            // Need to borrow from next limb
            result[i] = ((1u64 << 32) + a_val - b_val) as u32;
            borrow = 1;
        }
    }

    (result, borrow != 0)
}

/// Compute B - A with optional borrow input (negate subtraction).
///
/// This is equivalent to negating the result of A - B.
///
/// Returns (result, underflow) where:
/// - result: 256-bit difference as 8 × u32 limbs (little-endian)
/// - underflow: true if subtraction underflowed
#[inline]
pub fn u256_sub_negate(a: &[u32; 8], b: &[u32; 8], borrow_in: bool) -> ([u32; 8], bool) {
    // SUB_NEGATE computes B - A instead of A - B
    u256_sub(b, a, borrow_in)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_no_carry() {
        let a = [100, 200, 300, 400, 500, 600, 700, 800];
        let b = [1, 2, 3, 4, 5, 6, 7, 8];
        let (result, overflow) = u256_add(&a, &b, false);
        assert_eq!(result, [101, 202, 303, 404, 505, 606, 707, 808]);
        assert!(!overflow);
    }

    #[test]
    fn test_add_with_carry_in() {
        let a = [0xFFFFFFFE, 0, 0, 0, 0, 0, 0, 0];
        let b = [1, 0, 0, 0, 0, 0, 0, 0];
        let (result, overflow) = u256_add(&a, &b, true); // carry_in = 1
        assert_eq!(result, [0, 1, 0, 0, 0, 0, 0, 0]);
        assert!(!overflow);
    }

    #[test]
    fn test_add_max_values() {
        let max = [0xFFFFFFFF; 8];
        let one = [1, 0, 0, 0, 0, 0, 0, 0];
        let (result, overflow) = u256_add(&max, &one, false);
        assert_eq!(result, [0, 0, 0, 0, 0, 0, 0, 0]);
        assert!(overflow);
    }

    #[test]
    fn test_sub_no_borrow() {
        let a = [100, 200, 300, 400, 500, 600, 700, 800];
        let b = [1, 2, 3, 4, 5, 6, 7, 8];
        let (result, underflow) = u256_sub(&a, &b, false);
        assert_eq!(result, [99, 198, 297, 396, 495, 594, 693, 792]);
        assert!(!underflow);
    }

    #[test]
    fn test_sub_with_borrow_propagation() {
        let a = [0, 0, 1, 0, 0, 0, 0, 0]; // 2^64
        let b = [1, 0, 0, 0, 0, 0, 0, 0]; // 1
        let (result, underflow) = u256_sub(&a, &b, false);
        // 2^64 - 1 = 0xFFFFFFFF_FFFFFFFF
        assert_eq!(result, [0xFFFFFFFF, 0xFFFFFFFF, 0, 0, 0, 0, 0, 0]);
        assert!(!underflow);
    }

    #[test]
    fn test_sub_underflow() {
        let a = [0, 0, 0, 0, 0, 0, 0, 0]; // 0
        let b = [1, 0, 0, 0, 0, 0, 0, 0]; // 1
        let (result, underflow) = u256_sub(&a, &b, false);
        // 0 - 1 = -1 (wraps to max u256)
        assert_eq!(result, [0xFFFFFFFF; 8]);
        assert!(underflow);
    }

    #[test]
    fn test_sub_negate() {
        let a = [1, 0, 0, 0, 0, 0, 0, 0];
        let b = [5, 0, 0, 0, 0, 0, 0, 0];
        let (result, underflow) = u256_sub_negate(&a, &b, false);
        // B - A = 5 - 1 = 4
        assert_eq!(result, [4, 0, 0, 0, 0, 0, 0, 0]);
        assert!(!underflow);
    }
}
