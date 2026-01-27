//! Common utilities for zksync-os commands

use anyhow::{Context, Result};
use std::path::Path;

/// Load witness file in airbender format (hex-encoded Vec<u32>).
///
/// The witness format is:
/// - Vec<u32> where each u32 is encoded as big-endian bytes
/// - The bytes are then hex-encoded as a string
///
/// This function reverses that process: hex decode → chunks of 4 bytes → big-endian u32
pub fn load_zksyncos_witness(path: &Path) -> Result<Vec<u32>> {
    let hex_string = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read witness file: {}", path.display()))?;

    let hex_string = hex_string.trim();

    if hex_string.is_empty() {
        return Ok(Vec::new());
    }

    // Each u32 is 4 bytes = 8 hex characters
    if hex_string.len() % 8 != 0 {
        anyhow::bail!(
            "Invalid witness file: hex string length {} is not a multiple of 8",
            hex_string.len()
        );
    }

    let bytes = hex::decode(hex_string)
        .with_context(|| format!("Failed to decode hex witness from: {}", path.display()))?;

    // Convert bytes to Vec<u32> (big-endian)
    let witness: Vec<u32> = bytes
        .chunks_exact(4)
        .map(|chunk| u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]))
        .collect();

    Ok(witness)
}

/// Convert Vec<u32> witness to raw bytes (little-endian for Zisk input format).
///
/// Zisk expects input as raw bytes, so we convert the u32 array to bytes.
/// Note: Using little-endian here as that's typical for RISC-V targets.
pub fn witness_to_bytes(witness: &[u32]) -> Vec<u8> {
    witness.iter().flat_map(|w| w.to_le_bytes()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_witness_roundtrip() {
        // Simulate what zksync-os does: Vec<u32> -> big-endian bytes -> hex
        let original: Vec<u32> = vec![0x12345678, 0xDEADBEEF, 0x00000001];

        // Encode like zksync-os does
        let bytes: Vec<u8> = original.iter().flat_map(|x| x.to_be_bytes()).collect();
        let hex_str = hex::encode(&bytes);

        // Now decode using our function
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_witness.hex");
        std::fs::write(&temp_file, &hex_str).unwrap();

        let decoded = load_zksyncos_witness(&temp_file).unwrap();
        assert_eq!(decoded, original);

        std::fs::remove_file(&temp_file).ok();
    }

    #[test]
    fn test_empty_witness() {
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_empty_witness.hex");
        std::fs::write(&temp_file, "").unwrap();

        let decoded = load_zksyncos_witness(&temp_file).unwrap();
        assert!(decoded.is_empty());

        std::fs::remove_file(&temp_file).ok();
    }

    #[test]
    fn test_invalid_hex_length() {
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("test_invalid_witness.hex");
        std::fs::write(&temp_file, "1234567").unwrap(); // 7 chars, not multiple of 8

        let result = load_zksyncos_witness(&temp_file);
        assert!(result.is_err());

        std::fs::remove_file(&temp_file).ok();
    }
}
