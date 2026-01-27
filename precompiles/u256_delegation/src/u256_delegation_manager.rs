// ⚠️  WARNING: This module was AI-generated and has NOT undergone thorough human review.

//! U256 Delegation Manager
//!
//! Manages the U256 delegation precompile, providing:
//! - Counter building for operation counting
//! - Planner building for execution planning
//! - Instance building for witness computation

use std::sync::Arc;

use fields::PrimeField64;
use pil_std_lib::Std;
use zisk_common::{
    BusDevice, BusDeviceMetrics, BusDeviceMode, ComponentBuilder, Instance, InstanceCtx,
    InstanceInfo, PayloadType, Planner,
};
use zisk_core::ZiskOperationType;
use zisk_pil::{U256DelegationTrace, U256_DELEGATION_AIR_IDS, ZISK_AIRGROUP_ID};

use crate::{
    U256DelegationCounterInputGen, U256DelegationInstance, U256DelegationPlanner, U256DelegationSM,
};

/// Manager for U256 delegation precompile.
pub struct U256DelegationManager<F: PrimeField64> {
    /// U256 delegation state machine
    u256_sm: Arc<U256DelegationSM<F>>,
}

impl<F: PrimeField64> U256DelegationManager<F> {
    /// Creates a new manager.
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        let u256_sm = U256DelegationSM::new(std);
        Arc::new(Self { u256_sm })
    }

    /// Builds a counter for U256 delegation operations.
    pub fn build_u256_counter(&self) -> U256DelegationCounterInputGen {
        U256DelegationCounterInputGen::new(BusDeviceMode::Counter)
    }

    /// Builds an input generator for U256 delegation operations.
    pub fn build_u256_input_generator(&self) -> U256DelegationCounterInputGen {
        U256DelegationCounterInputGen::new(BusDeviceMode::InputGenerator)
    }
}

impl<F: PrimeField64> ComponentBuilder<F> for U256DelegationManager<F> {
    fn build_counter(&self) -> Option<Box<dyn BusDeviceMetrics>> {
        Some(Box::new(U256DelegationCounterInputGen::new(BusDeviceMode::Counter)))
    }

    fn build_planner(&self) -> Box<dyn Planner> {
        let num_availables = self.u256_sm.num_availables;

        Box::new(U256DelegationPlanner::new().add_instance(InstanceInfo::new(
            ZISK_AIRGROUP_ID,
            U256_DELEGATION_AIR_IDS[0],
            num_availables,
            ZiskOperationType::U256Delegation,
        )))
    }

    fn build_instance(&self, ictx: InstanceCtx) -> Box<dyn Instance<F>> {
        match ictx.plan.air_id {
            id if id == U256DelegationTrace::<F>::AIR_ID => {
                Box::new(U256DelegationInstance::new(self.u256_sm.clone(), ictx))
            }
            _ => {
                panic!(
                    "U256DelegationManager::build_instance() Unsupported air_id: {:?}",
                    ictx.plan.air_id
                )
            }
        }
    }

    fn build_inputs_generator(&self) -> Option<Box<dyn BusDevice<PayloadType>>> {
        Some(Box::new(U256DelegationCounterInputGen::new(BusDeviceMode::InputGenerator)))
    }
}
