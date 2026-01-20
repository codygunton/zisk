//! Equality comparison and memory copy operations.

/// Compare two 256-bit values for equality.
///
/// Returns true if A == B, false otherwise.
#[inline]
pub fn u256_eq(a: &[u32; 8], b: &[u32; 8]) -> bool {
    a == b
}

/// Memory copy operation with optional increment.
///
/// If carry_in is false: result = B (simple copy)
/// If carry_in is true: result = B + 1 (copy with increment)
///
/// Returns (result, overflow) where:
/// - result: Copy of B, optionally incremented
/// - overflow: true if increment caused overflow (only possible when carry_in=true)
#[inline]
pub fn u256_memcpy(b: &[u32; 8], carry_in: bool) -> ([u32; 8], bool) {
    if !carry_in {
        (*b, false)
    } else {
        // Add 1 to b
        let mut result = *b;
        let mut carry = 1u64;

        for limb in &mut result {
            let sum = *limb as u64 + carry;
            *limb = sum as u32;
            carry = sum >> 32;
            if carry == 0 {
                break;
            }
        }

        (result, carry != 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eq_equal_values() {
        let a = [1, 2, 3, 4, 5, 6, 7, 8];
        let b = [1, 2, 3, 4, 5, 6, 7, 8];
        assert!(u256_eq(&a, &b));
    }

    #[test]
    fn test_eq_different_values() {
        let a = [1, 2, 3, 4, 5, 6, 7, 8];
        let b = [1, 2, 3, 4, 5, 6, 7, 9];
        assert!(!u256_eq(&a, &b));
    }

    #[test]
    fn test_eq_first_limb_different() {
        let a = [0, 0, 0, 0, 0, 0, 0, 0];
        let b = [1, 0, 0, 0, 0, 0, 0, 0];
        assert!(!u256_eq(&a, &b));
    }

    #[test]
    fn test_eq_last_limb_different() {
        let a = [0, 0, 0, 0, 0, 0, 0, 0];
        let b = [0, 0, 0, 0, 0, 0, 0, 1];
        assert!(!u256_eq(&a, &b));
    }

    #[test]
    fn test_memcpy_simple() {
        let b = [1, 2, 3, 4, 5, 6, 7, 8];
        let (result, overflow) = u256_memcpy(&b, false);
        assert_eq!(result, b);
        assert!(!overflow);
    }

    #[test]
    fn test_memcpy_with_increment() {
        let b = [1, 2, 3, 4, 5, 6, 7, 8];
        let (result, overflow) = u256_memcpy(&b, true);
        assert_eq!(result, [2, 2, 3, 4, 5, 6, 7, 8]);
        assert!(!overflow);
    }

    #[test]
    fn test_memcpy_increment_with_carry() {
        let b = [0xFFFFFFFF, 0, 0, 0, 0, 0, 0, 0];
        let (result, overflow) = u256_memcpy(&b, true);
        assert_eq!(result, [0, 1, 0, 0, 0, 0, 0, 0]);
        assert!(!overflow);
    }

    #[test]
    fn test_memcpy_increment_overflow() {
        let b = [0xFFFFFFFF; 8];
        let (result, overflow) = u256_memcpy(&b, true);
        assert_eq!(result, [0, 0, 0, 0, 0, 0, 0, 0]);
        assert!(overflow);
    }
}
