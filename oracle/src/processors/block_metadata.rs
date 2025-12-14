//! Block metadata query processor.

use crate::query_ids::BLOCK_METADATA_QUERY_ID;
use crate::{OracleError, OracleProcessor};

/// Processor for BLOCK_METADATA_QUERY_ID (0x40070000).
///
/// Returns block-level metadata. The response data should be pre-serialized
/// in the zk_ee `UsizeSerializable` format.
pub struct BlockMetadataProcessor {
    /// Pre-serialized response data (Vec<usize> matching zk_ee format)
    response_data: Vec<usize>,
}

impl BlockMetadataProcessor {
    /// Create a new processor with pre-serialized response data.
    ///
    /// The `response_data` should be the serialized `BlockMetadataFromOracle`
    /// in zk_ee `UsizeSerializable` format.
    pub fn new(response_data: Vec<usize>) -> Self {
        Self { response_data }
    }

    /// Create a processor with minimal test metadata.
    ///
    /// Returns a response with:
    /// - chain_id: 270
    /// - block_number: 1
    /// - timestamp: 1700000000
    /// - Other fields zeroed
    pub fn new_for_test() -> Self {
        // Minimal serialization matching BlockMetadataFromOracle format:
        // eip1559_basefee (4 usize), pubdata_price (4), native_price (4),
        // block_number (1), timestamp (1), chain_id (1), gas_limit (1),
        // pubdata_limit (1), coinbase (3), block_hashes (256*4), mix_hash (4)
        // Total: 4+4+4+1+1+1+1+1+3+1024+4 = 1048 usize values

        let mut data = Vec::with_capacity(1048);

        // eip1559_basefee (U256 = 4 usize LE)
        data.extend_from_slice(&[1000_usize, 0, 0, 0]);
        // pubdata_price (U256)
        data.extend_from_slice(&[0_usize, 0, 0, 0]);
        // native_price (U256)
        data.extend_from_slice(&[10_usize, 0, 0, 0]);
        // block_number (u64)
        data.push(1_usize);
        // timestamp (u64)
        data.push(1700000000_usize);
        // chain_id (u64)
        data.push(270_usize);
        // gas_limit (u64)
        data.push(u64::MAX as usize / 256);
        // pubdata_limit (u64)
        data.push(u64::MAX as usize);
        // coinbase (B160 = 3 usize LE)
        data.extend_from_slice(&[0_usize, 0, 0]);
        // block_hashes (256 * U256 = 256 * 4 usize)
        for _ in 0..(256 * 4) {
            data.push(0_usize);
        }
        // mix_hash (U256)
        data.extend_from_slice(&[1_usize, 0, 0, 0]);

        Self { response_data: data }
    }
}

impl OracleProcessor for BlockMetadataProcessor {
    fn supported_query_ids(&self) -> Vec<u32> {
        vec![BLOCK_METADATA_QUERY_ID]
    }

    fn process_query(
        &mut self,
        query_id: u32,
        _query: Vec<usize>,
    ) -> Result<Box<dyn ExactSizeIterator<Item = usize> + Send + 'static>, OracleError> {
        debug_assert_eq!(query_id, BLOCK_METADATA_QUERY_ID);

        Ok(Box::new(self.response_data.clone().into_iter()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_metadata_processor_returns_data() {
        let mut processor = BlockMetadataProcessor::new_for_test();

        let result = processor.process_query(BLOCK_METADATA_QUERY_ID, vec![]);
        assert!(result.is_ok());

        let iter = result.expect("should succeed");
        assert!(iter.len() > 0, "response should not be empty");
    }

    #[test]
    fn test_block_metadata_processor_supported_ids() {
        let processor = BlockMetadataProcessor::new_for_test();

        let ids = processor.supported_query_ids();
        assert_eq!(ids, vec![BLOCK_METADATA_QUERY_ID]);
    }

    #[test]
    fn test_block_metadata_custom_data() {
        let custom_data = vec![1_usize, 2, 3, 4, 5];
        let mut processor = BlockMetadataProcessor::new(custom_data.clone());

        let result = processor.process_query(BLOCK_METADATA_QUERY_ID, vec![]);
        let data: Vec<usize> = result.expect("should succeed").collect();

        assert_eq!(data, custom_data);
    }
}
