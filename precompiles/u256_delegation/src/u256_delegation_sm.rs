//! U256 Delegation State Machine
//!
//! Computes witness traces for U256 arithmetic operations.

use std::sync::Arc;

use fields::PrimeField64;
use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, ProofmanResult};
use proofman_util::{timer_start_trace, timer_stop_and_log_trace};
use rayon::prelude::*;

#[cfg(not(feature = "packed"))]
use zisk_pil::{U256DelegationTrace, U256DelegationTraceRow};

#[cfg(not(feature = "packed"))]
type U256DelegationTraceRowType<F> = U256DelegationTraceRow<F>;
#[cfg(not(feature = "packed"))]
type U256DelegationTraceType<F> = U256DelegationTrace<F>;

// Packed types not yet available for U256Delegation
#[cfg(feature = "packed")]
type U256DelegationTraceRowType<F> = U256DelegationTraceRow<F>;
#[cfg(feature = "packed")]
type U256DelegationTraceType<F> = U256DelegationTrace<F>;
#[cfg(feature = "packed")]
use zisk_pil::{U256DelegationTrace, U256DelegationTraceRow};

use crate::{U256DelegationInput, U256Operation};

/// State machine for U256 delegation witness computation.
pub struct U256DelegationSM<F: PrimeField64> {
    /// Reference to the PIL2 standard library.
    pub std: Arc<Std<F>>,

    /// Number of available operations in the trace.
    pub num_availables: usize,

    /// Range check ID for 16-bit values
    range_id: usize,
}

impl<F: PrimeField64> U256DelegationSM<F> {
    /// Creates a new U256 Delegation State Machine instance.
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        let num_availables = U256DelegationTraceType::<F>::NUM_ROWS;
        let range_id = std.get_range_id(0, (1 << 16) - 1, None).unwrap();

        Arc::new(Self { std, num_availables, range_id })
    }

    /// Processes a single U256 operation, updating the trace row.
    #[inline(always)]
    pub fn process_slice(
        &self,
        input: &U256DelegationInput,
        trace: &mut U256DelegationTraceRowType<F>,
        multiplicities: &mut [u32],
    ) {
        // Set operand A limbs
        for i in 0..8 {
            trace.set_a(i, input.a[i]);
        }

        // Set operand B limbs
        for i in 0..8 {
            trace.set_b(i, input.b[i]);
        }

        // Set result chunks (split each 32-bit limb into low/high 16-bit)
        for i in 0..8 {
            let r = input.result[i];
            let lo = (r & 0xFFFF) as u16;
            let hi = ((r >> 16) & 0xFFFF) as u16;
            trace.set_r_lo(i, lo);
            trace.set_r_hi(i, hi);

            // Track multiplicities for range checks
            multiplicities[lo as usize] += 1;
            multiplicities[hi as usize] += 1;
        }

        // Set addresses and control
        trace.set_addr_a(input.addr_a as u32);
        trace.set_addr_b(input.addr_b as u32);
        trace.set_control(input.control as u8);
        trace.set_step(input.step_main);

        // Set operation selectors
        let op = input.operation();
        trace.set_is_add(op == U256Operation::Add);
        trace.set_is_sub(op == U256Operation::Sub);
        trace.set_is_sub_neg(op == U256Operation::SubNegate);
        trace.set_is_mul_low(op == U256Operation::MulLow);
        trace.set_is_mul_high(op == U256Operation::MulHigh);
        trace.set_is_eq(op == U256Operation::Eq);
        trace.set_is_memcpy(op == U256Operation::MemCpy);

        // Set carry input
        let has_carry = input.has_carry_in();
        trace.set_carry_in(has_carry);

        // Compute and set carry chain
        let carries = self.compute_carries(input);
        for i in 0..9 {
            trace.set_carry(i, carries[i]);
        }

        // Set overflow
        trace.set_overflow(input.overflow);

        // Compute MUL columns if this is a MUL operation
        if op == U256Operation::MulLow || op == U256Operation::MulHigh {
            self.compute_mul_columns(input, trace);
        }

        // Set selector (active row)
        trace.set_sel(true);
    }

    /// Computes the MUL auxiliary columns for schoolbook multiplication.
    fn compute_mul_columns(
        &self,
        input: &U256DelegationInput,
        trace: &mut U256DelegationTraceRowType<F>,
    ) {
        // Schoolbook multiplication: compute 512-bit product of two 256-bit numbers
        // P[k] = sum_{i+j=k} A[i] * B[j] + carry_in[k]

        // First, compute partial products for the low half (positions 0-7)
        let mut mul_pp_lo = [0u64; 8];
        let mut mul_carry_lo = [0u64; 9];

        // Position by position, accumulate partial products
        for k in 0..8 {
            let mut sum: u128 = mul_carry_lo[k] as u128;
            for i in 0..=k {
                let j = k - i;
                if i < 8 && j < 8 {
                    sum += (input.a[i] as u128) * (input.b[j] as u128);
                }
            }
            // Extract the 32-bit result and carry
            mul_pp_lo[k] = (sum & 0xFFFF_FFFF_FFFF_FFFF) as u64;  // This is partial product sum
            mul_carry_lo[k + 1] = (sum >> 64) as u64;

            // Actually, we need to split into result limb and carry
            // The constraint is: pp_sum + carry_in = result[k] + carry_out * 2^32
            // So we need to recompute more carefully

            // Recalculate: just the partial products sum (no carry yet)
            let mut pp_sum: u128 = 0;
            for i in 0..=k {
                let j = k - i;
                if i < 8 && j < 8 {
                    pp_sum += (input.a[i] as u128) * (input.b[j] as u128);
                }
            }
            mul_pp_lo[k] = pp_sum as u64;  // Store the full partial product sum
        }

        // Recompute carries properly by propagating through the result
        mul_carry_lo[0] = 0;  // No initial carry
        for k in 0..8 {
            // pp_sum + carry_in = result[k] + carry_out * 2^32
            // Therefore: carry_out = (pp_sum + carry_in - result[k]) / 2^32
            let pp_sum = mul_pp_lo[k] as u128;
            let total = pp_sum + mul_carry_lo[k] as u128;
            // result[k] should equal total mod 2^32
            mul_carry_lo[k + 1] = (total >> 32) as u64;
        }

        // Now compute partial products for the high half (positions 8-15)
        let mut mul_pp_hi = [0u64; 8];
        let mut mul_carry_hi = [0u64; 9];

        // High half carry starts from where low half ended
        mul_carry_hi[0] = mul_carry_lo[8];

        for k in 0..8 {
            let pos = k + 8;  // Position 8..15
            let mut pp_sum: u128 = 0;
            for i in 0..8 {
                let j = pos as i32 - i as i32;
                if j >= 0 && j < 8 {
                    pp_sum += (input.a[i] as u128) * (input.b[j as usize] as u128);
                }
            }
            mul_pp_hi[k] = pp_sum as u64;

            // Compute carry propagation for high half
            let total = pp_sum + mul_carry_hi[k] as u128;
            mul_carry_hi[k + 1] = (total >> 32) as u64;
        }

        // Set the MUL columns in the trace
        for i in 0..8 {
            trace.set_mul_pp_lo(i, mul_pp_lo[i]);
            trace.set_mul_pp_hi(i, mul_pp_hi[i]);
        }
        for i in 0..9 {
            trace.set_mul_carry_lo(i, mul_carry_lo[i]);
            trace.set_mul_carry_hi(i, mul_carry_hi[i]);
        }

        // mul_has_high_bits: for MUL_LOW, overflow is set if any high bit is non-zero
        let has_high_bits = input.overflow;
        trace.set_mul_has_high_bits(has_high_bits);
    }

    /// Computes the carry chain for the given input.
    fn compute_carries(&self, input: &U256DelegationInput) -> [bool; 9] {
        let mut carries = [false; 9];
        let op = input.operation();

        // carry[0] = carry_in
        carries[0] = input.has_carry_in();

        match op {
            U256Operation::Add => {
                // a[i] + b[i] + carry[i] = result[i] + carry[i+1] * 2^32
                let mut carry = if input.has_carry_in() { 1u64 } else { 0 };
                for i in 0..8 {
                    let sum = input.a[i] as u64 + input.b[i] as u64 + carry;
                    carry = sum >> 32;
                    carries[i + 1] = carry != 0;
                }
            }
            U256Operation::Sub => {
                // a[i] - b[i] - borrow[i] = result[i] - borrow[i+1] * 2^32
                let mut borrow: i64 = if input.has_carry_in() { 1 } else { 0 };
                for i in 0..8 {
                    let diff = input.a[i] as i64 - input.b[i] as i64 - borrow;
                    borrow = if diff < 0 { 1 } else { 0 };
                    carries[i + 1] = borrow != 0;
                }
            }
            U256Operation::SubNegate => {
                // b[i] - a[i] - borrow[i] = result[i] - borrow[i+1] * 2^32
                let mut borrow: i64 = if input.has_carry_in() { 1 } else { 0 };
                for i in 0..8 {
                    let diff = input.b[i] as i64 - input.a[i] as i64 - borrow;
                    borrow = if diff < 0 { 1 } else { 0 };
                    carries[i + 1] = borrow != 0;
                }
            }
            U256Operation::MemCpy => {
                // When carry_in=1: result = b + 1 with carry propagation
                if input.has_carry_in() {
                    let mut carry = 1u64;
                    for i in 0..8 {
                        let sum = input.b[i] as u64 + carry;
                        carry = sum >> 32;
                        carries[i + 1] = carry != 0;
                    }
                }
                // When carry_in=0: no carries needed
            }
            U256Operation::Eq | U256Operation::MulLow | U256Operation::MulHigh => {
                // EQ: no arithmetic carries
                // MUL: carries computed differently (partial products)
                // For MUL, we would need extended logic - see Task #6
            }
        }

        carries
    }

    /// Computes the witness for a series of inputs and produces an `AirInstance`.
    pub fn compute_witness(
        &self,
        inputs: &[Vec<U256DelegationInput>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut trace = U256DelegationTraceType::<F>::new_from_vec(trace_buffer)?;

        let num_rows = trace.num_rows();

        let total_inputs: usize = inputs.iter().map(|c| c.len()).sum();
        assert!(total_inputs <= num_rows);

        tracing::debug!(
            "··· Creating U256Delegation instance [{} / {} rows filled {:.2}%]",
            total_inputs,
            num_rows,
            total_inputs as f64 / num_rows as f64 * 100.0
        );

        timer_start_trace!(U256_DELEGATION_TRACE);

        // Flatten inputs
        let flat_inputs: Vec<_> = inputs.iter().flatten().collect();
        let trace_rows = trace.buffer.as_mut_slice();

        // Determine optimal chunk size
        let num_threads = rayon::current_num_threads();
        let chunk_size = std::cmp::max(1, flat_inputs.len() / num_threads);

        // Process in chunks to share local multiplicity arrays
        let local_multiplicities_vec: Vec<Vec<u32>> = flat_inputs
            .par_chunks(chunk_size)
            .zip(trace_rows.par_chunks_mut(chunk_size))
            .map(|(input_chunk, trace_chunk)| {
                let mut local_multiplicities = vec![0u32; 1 << 16];

                for (input, trace_row) in input_chunk.iter().zip(trace_chunk.iter_mut()) {
                    self.process_slice(input, trace_row, &mut local_multiplicities);
                }

                local_multiplicities
            })
            .collect();

        // Sum all local arrays into global
        let mut global_multiplicities = vec![0u32; 1 << 16];
        for local_multiplicities in local_multiplicities_vec {
            for (i, count) in local_multiplicities.iter().enumerate() {
                global_multiplicities[i] += count;
            }
        }

        // Send multiplicities to std for range checks
        self.std.range_checks(self.range_id, global_multiplicities);

        timer_stop_and_log_trace!(U256_DELEGATION_TRACE);

        // Pad remaining rows
        let padding_row = U256DelegationTraceRowType::<F>::default();
        trace.buffer[total_inputs..num_rows].par_iter_mut().for_each(|slot| *slot = padding_row);

        Ok(AirInstance::<F>::new_from_trace(FromTrace::new(&mut trace)))
    }
}
