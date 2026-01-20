//! U256 Delegation Instance
//!
//! Manages the witness computation for a single U256 delegation air instance.

use std::{
    any::Any,
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use fields::PrimeField64;
use proofman_common::{AirInstance, ProofCtx, ProofmanResult, SetupCtx};
use zisk_common::{
    BusDevice, BusId, CheckPoint, ChunkId, CollectSkipper, Instance, InstanceCtx, InstanceType,
    MemCollectorInfo, PayloadType, OPERATION_BUS_ID, OP_TYPE,
};
use zisk_core::ZiskOperationType;

use crate::{U256DelegationInput, U256DelegationSM};

/// Instance for U256 delegation witness computation.
pub struct U256DelegationInstance<F: PrimeField64> {
    /// U256 delegation state machine
    u256_sm: Arc<U256DelegationSM<F>>,

    /// Collect info for each chunk ID
    collect_info: HashMap<ChunkId, (u64, CollectSkipper)>,

    /// Instance context
    ictx: InstanceCtx,
}

impl<F: PrimeField64> U256DelegationInstance<F> {
    /// Creates a new instance.
    pub fn new(u256_sm: Arc<U256DelegationSM<F>>, mut ictx: InstanceCtx) -> Self {
        let meta = ictx.plan.meta.take().expect("Expected metadata in ictx.plan.meta");

        let collect_info = *meta
            .downcast::<HashMap<ChunkId, (u64, CollectSkipper)>>()
            .expect("Failed to downcast ictx.plan.meta to expected type");

        Self { u256_sm, collect_info, ictx }
    }
}

impl<F: PrimeField64> Instance<F> for U256DelegationInstance<F> {
    fn compute_witness(
        &self,
        _pctx: &ProofCtx<F>,
        _sctx: &SetupCtx<F>,
        collectors: Vec<(usize, Box<dyn BusDevice<PayloadType>>)>,
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<Option<AirInstance<F>>> {
        // Extract inputs from all collectors
        let inputs: Vec<Vec<U256DelegationInput>> = collectors
            .into_iter()
            .map(|(_, collector)| {
                collector
                    .as_any()
                    .downcast::<U256DelegationCollector>()
                    .expect("Expected U256DelegationCollector")
                    .inputs
            })
            .collect();

        // Count total operations
        let total_ops: usize = inputs.iter().map(|v| v.len()).sum();

        if total_ops == 0 {
            tracing::debug!("U256DelegationInstance: No operations to witness");
            return Ok(None);
        }

        tracing::info!("U256DelegationInstance: Computing witness for {} operations", total_ops);

        // Compute witness using state machine
        let witness = self.u256_sm.compute_witness(&inputs, trace_buffer)?;

        Ok(Some(witness))
    }

    fn check_point(&self) -> &CheckPoint {
        &self.ictx.plan.check_point
    }

    fn instance_type(&self) -> InstanceType {
        InstanceType::Instance
    }

    fn build_inputs_collector(&self, chunk_id: ChunkId) -> Option<Box<dyn BusDevice<PayloadType>>> {
        let (num_ops, collect_skipper) = self.collect_info[&chunk_id];
        Some(Box::new(U256DelegationCollector::new(num_ops, collect_skipper)))
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Collector for U256 delegation operation data.
pub struct U256DelegationCollector {
    /// Collected inputs
    pub inputs: Vec<U256DelegationInput>,

    /// Number of operations to collect
    num_operations: u64,

    /// Helper to skip instructions based on plan configuration
    collect_skipper: CollectSkipper,
}

impl U256DelegationCollector {
    /// Creates a new collector.
    pub fn new(num_operations: u64, collect_skipper: CollectSkipper) -> Self {
        Self {
            inputs: Vec::with_capacity(num_operations as usize),
            num_operations,
            collect_skipper,
        }
    }
}

impl BusDevice<PayloadType> for U256DelegationCollector {
    #[inline(always)]
    fn process_data(
        &mut self,
        bus_id: &BusId,
        data: &[PayloadType],
        _pending: &mut VecDeque<(BusId, Vec<PayloadType>)>,
        _mem_collector_info: Option<&[MemCollectorInfo]>,
    ) -> bool {
        debug_assert!(*bus_id == OPERATION_BUS_ID);

        if self.inputs.len() == self.num_operations as usize {
            return false;
        }

        if data[OP_TYPE] as u32 != ZiskOperationType::U256Delegation as u32 {
            return true;
        }

        if self.collect_skipper.should_skip() {
            return true;
        }

        // Extract U256DelegationInput from bus data
        let input = U256DelegationInput::from_bus_data(data);
        self.inputs.push(input);

        self.inputs.len() < self.num_operations as usize
    }

    fn bus_id(&self) -> Vec<BusId> {
        vec![OPERATION_BUS_ID]
    }

    fn as_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}
