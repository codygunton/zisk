// ⚠️  WARNING: This module was AI-generated and has NOT undergone thorough human review.

//! U256 Delegation Counter and Input Generator
//!
//! This module provides a bus device that:
//! - In Counter mode: counts U256 delegation operations for planning
//! - In InputGenerator mode: generates memory inputs for U256 operations

use std::{collections::VecDeque, ops::Add};

use zisk_common::{
    BusDevice, BusDeviceMode, BusId, Counter, MemCollectorInfo, Metrics, OPERATION_BUS_ID, OP_TYPE,
};
use zisk_core::ZiskOperationType;

/// Counter and input generator for U256 delegation operations.
pub struct U256DelegationCounterInputGen {
    /// Operation counter
    counter: Counter,

    /// Bus device mode (counter or input generator)
    mode: BusDeviceMode,
}

impl U256DelegationCounterInputGen {
    /// Creates a new instance.
    ///
    /// # Arguments
    /// * `mode` - Whether to act as counter or input generator
    pub fn new(mode: BusDeviceMode) -> Self {
        Self { counter: Counter::default(), mode }
    }

    /// Gets the instruction count for a specific operation type.
    pub fn inst_count(&self, op_type: ZiskOperationType) -> Option<u64> {
        (op_type == ZiskOperationType::U256Delegation).then_some(self.counter.inst_count)
    }
}

impl Metrics for U256DelegationCounterInputGen {
    #[inline(always)]
    fn measure(&mut self, _data: &[u64]) {
        self.counter.update(1);
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl Add for U256DelegationCounterInputGen {
    type Output = U256DelegationCounterInputGen;

    fn add(self, other: Self) -> U256DelegationCounterInputGen {
        U256DelegationCounterInputGen { counter: &self.counter + &other.counter, mode: self.mode }
    }
}

impl BusDevice<u64> for U256DelegationCounterInputGen {
    #[inline(always)]
    fn process_data(
        &mut self,
        bus_id: &BusId,
        data: &[u64],
        _pending: &mut VecDeque<(BusId, Vec<u64>)>,
        _mem_collector_info: Option<&[MemCollectorInfo]>,
    ) -> bool {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        // Only process U256 delegation operations
        if data[OP_TYPE] as u32 != ZiskOperationType::U256Delegation as u32 {
            return true;
        }

        // In counter mode, just count the operation
        if self.mode == BusDeviceMode::Counter {
            self.measure(data);
        }

        // Note: Unlike Keccak, U256 delegation doesn't need to generate
        // additional memory inputs because the memory reads/writes are
        // already handled by the delegation mechanism in the emulator.
        // The precompile will constrain the operation results directly.

        true
    }

    fn bus_id(&self) -> Vec<BusId> {
        vec![OPERATION_BUS_ID]
    }

    fn as_any(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}
