// ⚠️  WARNING: This module was AI-generated and has NOT undergone thorough human review.

//! 256-bit multiplication operations.

/// Compute 512-bit multiplication and return the low 256 bits.
///
/// Returns (low_256_bits, overflow) where:
/// - low_256_bits: Lower 256 bits of A * B as 8 × u32 limbs
/// - overflow: true if result exceeds 256 bits (high part is non-zero)
#[inline]
pub fn u256_mul_low(a: &[u32; 8], b: &[u32; 8]) -> ([u32; 8], bool) {
    let (low, high) = u256_mul_full(a, b);
    let overflow = high.iter().any(|&x| x != 0);
    (low, overflow)
}

/// Compute 512-bit multiplication and return the high 256 bits.
///
/// Returns (high_256_bits, false) where:
/// - high_256_bits: Upper 256 bits of A * B as 8 × u32 limbs
/// - The overflow flag is always false (high cannot overflow)
#[inline]
pub fn u256_mul_high(a: &[u32; 8], b: &[u32; 8]) -> ([u32; 8], bool) {
    let (_low, high) = u256_mul_full(a, b);
    (high, false)
}

/// Compute full 512-bit multiplication.
///
/// Uses schoolbook multiplication algorithm with 32-bit limbs.
/// Returns (low_256_bits, high_256_bits).
fn u256_mul_full(a: &[u32; 8], b: &[u32; 8]) -> ([u32; 8], [u32; 8]) {
    // Accumulate in 64-bit intermediates to handle carries
    let mut product = [0u64; 16];

    // Schoolbook multiplication: product[i+j] += a[i] * b[j]
    for i in 0..8 {
        for j in 0..8 {
            let p = a[i] as u64 * b[j] as u64;
            product[i + j] += p;
        }
    }

    // Carry propagation
    for i in 0..15 {
        product[i + 1] += product[i] >> 32;
        product[i] &= 0xFFFFFFFF;
    }

    // Extract low and high parts
    let mut low = [0u32; 8];
    let mut high = [0u32; 8];

    for i in 0..8 {
        low[i] = product[i] as u32;
        high[i] = product[i + 8] as u32;
    }

    (low, high)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mul_small_numbers() {
        let a = [7, 0, 0, 0, 0, 0, 0, 0];
        let b = [9, 0, 0, 0, 0, 0, 0, 0];
        let (low, overflow) = u256_mul_low(&a, &b);
        assert_eq!(low, [63, 0, 0, 0, 0, 0, 0, 0]);
        assert!(!overflow);
    }

    #[test]
    fn test_mul_with_carry() {
        // 0x100000000 * 0x100000000 = 0x10000000000000000
        let a = [0, 1, 0, 0, 0, 0, 0, 0]; // 2^32
        let b = [0, 1, 0, 0, 0, 0, 0, 0]; // 2^32
        let (low, overflow) = u256_mul_low(&a, &b);
        // 2^64 = low[2] = 1
        assert_eq!(low, [0, 0, 1, 0, 0, 0, 0, 0]);
        assert!(!overflow);
    }

    #[test]
    fn test_mul_overflow() {
        // Large numbers that overflow 256 bits
        let a = [0, 0, 0, 0, 1, 0, 0, 0]; // 2^128
        let b = [0, 0, 0, 0, 1, 0, 0, 0]; // 2^128
        let (low, overflow) = u256_mul_low(&a, &b);
        // 2^256 -> low = 0, high = 1
        assert_eq!(low, [0, 0, 0, 0, 0, 0, 0, 0]);
        assert!(overflow);

        let (high, _) = u256_mul_high(&a, &b);
        assert_eq!(high, [1, 0, 0, 0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_mul_max_single_limb() {
        let a = [0xFFFFFFFF, 0, 0, 0, 0, 0, 0, 0];
        let b = [0xFFFFFFFF, 0, 0, 0, 0, 0, 0, 0];
        let (low, overflow) = u256_mul_low(&a, &b);
        // (2^32 - 1)^2 = 2^64 - 2*2^32 + 1 = 0xFFFFFFFE00000001
        assert_eq!(low[0], 1);
        assert_eq!(low[1], 0xFFFFFFFE);
        assert!(!overflow);
    }

    #[test]
    fn test_mul_high_extraction() {
        // Compute (2^128 - 1) * (2^128 - 1)
        // = 2^256 - 2^129 + 1
        // Low = 1, High = 2^256 - 2^129 = -2^129 mod 2^256
        let a = [0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0, 0, 0, 0];
        let b = [0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0xFFFFFFFF, 0, 0, 0, 0];
        let (low, high) = u256_mul_full(&a, &b);

        // Verify by manual calculation
        // (2^128 - 1)^2 = 2^256 - 2*2^128 + 1
        // Low 256 bits: 1 + (2^256 - 2*2^128) mod 2^256
        //             = 1 + (2^256 mod 2^256) - 2*2^128 mod 2^256
        //             = 1 - 2*2^128 mod 2^256 (wrapping)
        // This equals 0x00000001_00000000_00000000_00000000_FFFFFFFE_FFFFFFFF_FFFFFFFF_FFFFFFFF + 1
        assert_eq!(low[0], 1);
        assert_eq!(low[1], 0);
        assert_eq!(low[2], 0);
        assert_eq!(low[3], 0);
        assert_eq!(low[4], 0xFFFFFFFE);
        assert_eq!(low[5], 0xFFFFFFFF);
        assert_eq!(low[6], 0xFFFFFFFF);
        assert_eq!(low[7], 0xFFFFFFFF);

        // High part should be 0xFFFFFFFF_FFFFFFFF_FFFFFFFF_FFFFFFFE_00000000_00000000_00000000_00000000
        assert_eq!(high[0], 0);
        assert_eq!(high[1], 0);
        assert_eq!(high[2], 0);
        assert_eq!(high[3], 0);
        assert_eq!(high[4], 0xFFFFFFFE);
        assert_eq!(high[5], 0xFFFFFFFF);
        assert_eq!(high[6], 0xFFFFFFFF);
        assert_eq!(high[7], 0xFFFFFFFF);
    }
}
