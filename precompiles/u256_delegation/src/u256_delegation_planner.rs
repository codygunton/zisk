//! U256 Delegation Planner
//!
//! Plans execution instances for U256 delegation operations based on
//! operation counts collected during the counting phase.

use std::any::Any;

use crate::U256DelegationCounterInputGen;
use zisk_common::{
    plan, BusDeviceMetrics, ChunkId, InstCount, InstanceInfo, InstanceType, Metrics, Plan, Planner,
};

/// Planner for U256 delegation instances.
#[derive(Default)]
pub struct U256DelegationPlanner {
    /// Instance info for planning
    instances_info: Vec<InstanceInfo>,
}

impl U256DelegationPlanner {
    /// Creates a new planner.
    pub fn new() -> Self {
        Self { instances_info: Vec::new() }
    }

    /// Adds an instance info to the planner.
    pub fn add_instance(mut self, instance_info: InstanceInfo) -> Self {
        self.instances_info.push(instance_info);
        self
    }
}

impl Planner for U256DelegationPlanner {
    fn plan(&self, counters: Vec<(ChunkId, Box<dyn BusDeviceMetrics>)>) -> Vec<Plan> {
        // Prepare counts for each instance type
        let mut count: Vec<Vec<InstCount>> = Vec::with_capacity(self.instances_info.len());

        for _ in 0..self.instances_info.len() {
            count.push(Vec::new());
        }

        // Process counters from each chunk
        counters.iter().for_each(|(chunk_id, counter)| {
            let u256_counter =
                Metrics::as_any(&**counter).downcast_ref::<U256DelegationCounterInputGen>().unwrap();

            for (index, instance_info) in self.instances_info.iter().enumerate() {
                let inst_count = InstCount::new(
                    *chunk_id,
                    u256_counter.inst_count(instance_info.op_type).unwrap_or(0),
                );
                count[index].push(inst_count);
            }
        });

        // Generate plans for each instance type
        let mut plan_result = Vec::new();

        for (idx, instance) in self.instances_info.iter().enumerate() {
            let plans: Vec<_> = plan(&count[idx], instance.num_ops as u64)
                .into_iter()
                .map(|(check_point, collect_info)| {
                    let converted: Box<dyn Any> = Box::new(collect_info);
                    Plan::new(
                        instance.airgroup_id,
                        instance.air_id,
                        None,
                        InstanceType::Instance,
                        check_point,
                        Some(converted),
                    )
                })
                .collect();

            plan_result.extend(plans);
        }

        plan_result
    }
}
