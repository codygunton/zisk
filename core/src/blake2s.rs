//! Blake2s round function delegation support for zksync-os
//!
//! This module implements the Blake2s mixing function used by CSR 0x7C7 delegation
//! in zksync-os. When the guest writes to CSR 0x7C7, the emulator performs one round
//! of the Blake2s mixing function.
//!
//! # Protocol
//!
//! The guest sets up registers x10-x13 before triggering the delegation:
//! - x10: Pointer to state (8 u32) + extended_state (16 u32) = 24 u32 words, 128-byte aligned
//! - x11: Pointer to input buffer (16 u32 words), 4-byte aligned
//! - x12: Round bitmask (power of 2, indicates which round to execute)
//! - x13: Control flags:
//!   - bit 0: last_round (output flag)
//!   - bit 1: is_right (for compression mode node ordering)
//!   - bit 2: compression mode

pub const BLAKE2S_BLOCK_SIZE_BYTES: usize = 64;
pub const BLAKE2S_BLOCK_SIZE_U32_WORDS: usize = 16;
pub const BLAKE2S_STATE_WIDTH_IN_U32_WORDS: usize = 8;
pub const BLAKE2S_EXTENDED_STATE_WIDTH_IN_U32_WORDS: usize = 16;

/// Blake2s initialization vector
pub const IV: [u32; 8] = [
    0x6A09E667, 0xBB67AE85, 0x3C6EF372, 0xA54FF53A, 0x510E527F, 0x9B05688C, 0x1F83D9AB, 0x5BE0CD19,
];

/// IV[0] XORed with configuration (no tree, 32-byte digest)
pub const IV_0_TWIST: u32 = 0x6A09E667 ^ 0x01010000 ^ 32;

/// Configured IV with IV_0_TWIST applied
pub const CONFIGURED_IV: [u32; 8] = [
    IV_0_TWIST,
    IV[1],
    IV[2],
    IV[3],
    IV[4],
    IV[5],
    IV[6],
    IV[7],
];

/// Round permutation schedules (sigma)
pub const SIGMAS: [[usize; 16]; 10] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    [14, 10, 4, 8, 9, 15, 13, 6, 1, 12, 0, 2, 11, 7, 5, 3],
    [11, 8, 12, 0, 5, 2, 15, 13, 10, 14, 3, 6, 7, 1, 9, 4],
    [7, 9, 3, 1, 13, 12, 11, 14, 2, 6, 5, 10, 4, 0, 15, 8],
    [9, 0, 5, 7, 2, 4, 10, 15, 14, 1, 11, 12, 6, 8, 3, 13],
    [2, 12, 6, 10, 0, 11, 8, 3, 4, 13, 7, 5, 15, 14, 1, 9],
    [12, 5, 1, 15, 14, 13, 4, 10, 0, 7, 6, 3, 9, 2, 8, 11],
    [13, 11, 7, 14, 12, 1, 3, 9, 5, 0, 15, 4, 8, 6, 2, 10],
    [6, 15, 14, 9, 11, 3, 0, 8, 12, 2, 13, 7, 1, 4, 10, 5],
    [10, 2, 8, 4, 7, 6, 1, 5, 15, 11, 9, 14, 3, 12, 13, 0],
];

/// Control flag masks
pub const TEST_IF_LAST_ROUND_MASK: u32 = 1 << 0;
pub const TEST_IF_INPUT_IS_RIGHT_NODE_MASK: u32 = 1 << 1;
pub const TEST_IF_COMPRESSION_MODE_MASK: u32 = 1 << 2;

/// Rotate right by N bits
#[inline(always)]
fn rotate_right<const N: u32>(x: u32) -> u32 {
    x.rotate_right(N)
}

/// Blake2s G function - the core mixing operation
#[inline(always)]
fn g_function(
    v: &mut [u32; BLAKE2S_BLOCK_SIZE_U32_WORDS],
    a: usize,
    b: usize,
    c: usize,
    d: usize,
    x: u32,
    y: u32,
) {
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(x);
    v[d] = rotate_right::<16>(v[d] ^ v[a]);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = rotate_right::<12>(v[b] ^ v[c]);
    v[a] = v[a].wrapping_add(v[b]).wrapping_add(y);
    v[d] = rotate_right::<8>(v[d] ^ v[a]);
    v[c] = v[c].wrapping_add(v[d]);
    v[b] = rotate_right::<7>(v[b] ^ v[c]);
}

/// Blake2s mixing function - applies G functions according to the sigma permutation
#[inline(always)]
pub fn mixing_function(
    state: &mut [u32; BLAKE2S_EXTENDED_STATE_WIDTH_IN_U32_WORDS],
    message_block: &[u32; BLAKE2S_BLOCK_SIZE_U32_WORDS],
    sigma: &[usize; 16],
) {
    // Mix columns
    g_function(
        state,
        0,
        4,
        8,
        12,
        message_block[sigma[0]],
        message_block[sigma[1]],
    );
    g_function(
        state,
        1,
        5,
        9,
        13,
        message_block[sigma[2]],
        message_block[sigma[3]],
    );
    g_function(
        state,
        2,
        6,
        10,
        14,
        message_block[sigma[4]],
        message_block[sigma[5]],
    );
    g_function(
        state,
        3,
        7,
        11,
        15,
        message_block[sigma[6]],
        message_block[sigma[7]],
    );

    // Mix diagonals
    g_function(
        state,
        0,
        5,
        10,
        15,
        message_block[sigma[8]],
        message_block[sigma[9]],
    );
    g_function(
        state,
        1,
        6,
        11,
        12,
        message_block[sigma[10]],
        message_block[sigma[11]],
    );
    g_function(
        state,
        2,
        7,
        8,
        13,
        message_block[sigma[12]],
        message_block[sigma[13]],
    );
    g_function(
        state,
        3,
        4,
        9,
        14,
        message_block[sigma[14]],
        message_block[sigma[15]],
    );
}
