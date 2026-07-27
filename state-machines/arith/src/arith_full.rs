//! The `ArithFullSM` module implements the Arithmetic Full State Machine.
//!
//! This state machine manages the computation of arithmetic operations and their associated
//! trace generation. It coordinates with `ArithTableSM` and `ArithRangeTableSM` to handle
//! state transitions and multiplicity updates.

use std::collections::VecDeque;
use std::sync::Arc;

use crate::{
    ArithOperation, ArithRangeTableInputs, ArithRangeTableSM, ArithTableInputs, ArithTableSM,
};
use fields::PrimeField64;
use pil_std_lib::Std;
use proofman_common::{AirInstance, FromTrace, ProofmanResult};
use rayon::prelude::*;
use sm_binary::{GT_OP, LTU_OP, LT_ABS_NP_OP, LT_ABS_PN_OP};
use zisk_common::{BusId, ExtOperationData, OperationBusData, OperationData};
use zisk_core::{zisk_ops::ZiskOp, ZiskOperationType};
use zisk_pil::{ArithTrace, ArithTraceRowOps};

const CHUNK_SIZE: u64 = 0x10000;
const EXTENSION: u64 = 0xFFFFFFFF;

// Arith signed-DIV quotient-sign malicious-witness repro (env-gated). Target DIV(1, -1).
const REPRO_BAD_ARITH_DIV_SIGN_ENV: &str = "ZISK_REPRO_BAD_ARITH_DIV_SIGN";
const REPRO_DIV_SIGN_A: u64 = 1;
const REPRO_DIV_SIGN_B: u64 = 0xFFFF_FFFF_FFFF_FFFF;

/// The `ArithFullSM` struct represents the Arithmetic Full State Machine.
///
/// This state machine coordinates the computation of arithmetic operations and updates
/// the `ArithTableSM` and `ArithRangeTableSM` components based on operation traces.
pub struct ArithFullSM<F: PrimeField64> {
    /// Reference to the PIL2 standard library.
    std: Arc<Std<F>>,

    /// The table ID for the Table State Machine
    table_id: usize,

    /// The table ID for the Range Table State Machine
    range_table_id: usize,
}

impl<F: PrimeField64> ArithFullSM<F> {
    /// Creates a new `ArithFullSM` instance.
    ///
    /// # Arguments
    /// * `std` - An `Arc`-wrapped reference to the PIL2 standard library.
    ///
    /// # Returns
    /// An `Arc`-wrapped instance of `ArithFullSM`.
    pub fn new(std: Arc<Std<F>>) -> Arc<Self> {
        // Get the Arithmetic table ID
        let table_id =
            std.get_virtual_table_id(ArithTableSM::TABLE_ID).expect("Failed to get table ID");

        // Get the Arithmetic Range table ID
        let range_table_id = std
            .get_virtual_table_id(ArithRangeTableSM::TABLE_ID)
            .expect("Failed to get range table ID");

        Arc::new(Self { std, table_id, range_table_id })
    }

    /// Computes the witness for arithmetic operations and updates associated tables.
    ///
    /// # Arguments
    /// * `inputs` - A slice of `OperationData` representing the arithmetic inputs.
    ///
    /// # Returns
    /// An `AirInstance` containing the computed arithmetic trace.
    pub fn compute_witness<R: ArithTraceRowOps<F>>(
        &self,
        inputs: &[Vec<OperationData<u64>>],
        trace_buffer: Vec<F>,
    ) -> ProofmanResult<AirInstance<F>> {
        let mut arith_trace = ArithTrace::<R>::new_from_vec(trace_buffer)?;

        let num_rows = arith_trace.num_rows();

        let total_inputs: usize = inputs.iter().map(|c| c.len()).sum();
        assert!(total_inputs <= num_rows);

        let mut range_table_inputs = ArithRangeTableInputs::new();
        let mut table_inputs = ArithTableInputs::new();

        tracing::debug!(
            "··· Creating Arith instance [{} / {} rows filled {:.2}%]",
            total_inputs,
            num_rows,
            total_inputs as f64 / num_rows as f64 * 100.0
        );

        // Split the arith_trace.buffer into slices matching each inner vector’s length.
        if total_inputs > 0 {
            let flat_inputs: Vec<_> = inputs.iter().flatten().collect(); // Vec<&OperationData<u64>>
            let flat_buffer = arith_trace.buffer.as_mut_slice();
            let chunk_size = total_inputs.div_ceil(rayon::current_num_threads());

            flat_buffer
                .par_chunks_mut(chunk_size)
                .zip(flat_inputs.par_chunks(chunk_size))
                .for_each(|(trace_slice, input_slice)| {
                    let mut aop = ArithOperation::new();
                    let mut range_table = ArithRangeTableInputs::new();
                    let mut table = ArithTableInputs::new();

                    trace_slice.iter_mut().zip(input_slice.iter()).for_each(
                        |(trace_row, input)| {
                            *trace_row = Self::process_slice::<R>(
                                &mut range_table,
                                &mut table,
                                &mut aop,
                                input,
                            );
                        },
                    );

                    for (row, multiplicity) in &table {
                        self.std.inc_virtual_row(self.table_id, row as u64, multiplicity);
                    }

                    for (row, multiplicity) in &range_table {
                        self.std.inc_virtual_row(self.range_table_id, row as u64, multiplicity);
                    }
                });
        }

        let padding_offset = total_inputs;
        let padding_rows: usize = num_rows.saturating_sub(padding_offset);

        if padding_rows > 0 {
            let mut row = R::default();
            let padding_opcode = ZiskOp::Muluh.code();
            row.set_op(padding_opcode);

            arith_trace.buffer[padding_offset..num_rows]
                .par_iter_mut()
                .for_each(|elem| *elem = row);

            range_table_inputs.multi_use_chunk_range_check(padding_rows * 10, 0, 0);
            range_table_inputs.multi_use_chunk_range_check(padding_rows * 2, 26, 0);
            range_table_inputs.multi_use_chunk_range_check(padding_rows * 2, 17, 0);
            range_table_inputs.multi_use_chunk_range_check(padding_rows * 2, 9, 0);
            range_table_inputs.multi_use_carry_range_check(padding_rows * 7, 0);
            table_inputs.multi_add_use(
                padding_rows,
                padding_opcode,
                false,
                false,
                false,
                false,
                false,
                false,
                false,
            );
        }

        // TODO: We should compare against cache-then-increase version instead of increase each time...

        for (row, multiplicity) in &table_inputs {
            self.std.inc_virtual_row(self.table_id, row as u64, multiplicity);
        }

        for (row, multiplicity) in &range_table_inputs {
            self.std.inc_virtual_row(self.range_table_id, row as u64, multiplicity);
        }

        Ok(AirInstance::new_from_trace(FromTrace::new(&mut arith_trace)))
    }

    /// Generates binary inputs for operations requiring additional validation (e.g., division).
    #[inline(always)]
    pub fn generate_inputs(
        input: &OperationData<u64>,
        pending: &mut VecDeque<(BusId, Vec<u64>, Vec<u64>)>,
    ) {
        let mut aop = ArithOperation::new();

        let input_data = ExtOperationData::OperationData(*input);

        let opcode = OperationBusData::get_op(&input_data);
        let a = OperationBusData::get_a(&input_data);
        let b = OperationBusData::get_b(&input_data);

        aop.calculate(opcode, a, b);
        Self::maybe_inject_bad_div_sign_repro(&mut aop, opcode, a, b);

        // If the operation is a division, then use the binary component
        // to check that the remainer is lower than the divisor
        if aop.div && !aop.div_by_zero {
            let opcode = match (aop.nr, aop.nb) {
                (false, false) => LTU_OP,
                (false, true) => LT_ABS_PN_OP,
                (true, false) => LT_ABS_NP_OP,
                (true, true) => GT_OP,
            };

            let extension = match (aop.m32, aop.nr, aop.nb) {
                (false, _, _) => (0, 0),
                (true, false, false) => (0, 0),
                (true, false, true) => (0, EXTENSION),
                (true, true, false) => (EXTENSION, 0),
                (true, true, true) => (EXTENSION, EXTENSION),
            };

            // TODO: We dont need to "glue" the d,b chunks back, we can use the aop API to do this!
            OperationBusData::from_values(
                opcode,
                ZiskOperationType::Binary as u64,
                aop.d[0] as u64
                    + CHUNK_SIZE * aop.d[1] as u64
                    + CHUNK_SIZE.pow(2) * (aop.d[2] as u64 + extension.0)
                    + CHUNK_SIZE.pow(3) * aop.d[3] as u64,
                aop.b[0] as u64
                    + CHUNK_SIZE * aop.b[1] as u64
                    + CHUNK_SIZE.pow(2) * (aop.b[2] as u64 + extension.1)
                    + CHUNK_SIZE.pow(3) * aop.b[3] as u64,
                pending,
            );
        }
    }

    fn process_slice<R: ArithTraceRowOps<F>>(
        range_table_inputs: &mut ArithRangeTableInputs,
        table_inputs: &mut ArithTableInputs,
        aop: &mut ArithOperation,
        input: &[u64; 4],
    ) -> R {
        let input_data = ExtOperationData::OperationData(*input);

        let opcode = OperationBusData::get_op(&input_data);
        let a = OperationBusData::get_a(&input_data);
        let b = OperationBusData::get_b(&input_data);

        aop.calculate(opcode, a, b);
        Self::maybe_inject_bad_div_sign_repro(aop, opcode, a, b);
        let mut row = R::default();
        for i in [0, 2] {
            range_table_inputs.use_chunk_range_check(0, aop.a[i] as u64);
            range_table_inputs.use_chunk_range_check(0, aop.b[i] as u64);
            range_table_inputs.use_chunk_range_check(0, aop.c[i] as u64);
            range_table_inputs.use_chunk_range_check(0, aop.d[i] as u64);
        }
        row.set_all_a(&aop.a);
        row.set_all_b(&aop.b);
        row.set_all_c(&aop.c);
        row.set_all_d(&aop.d);
        range_table_inputs.use_chunk_range_check(aop.range_ab, aop.a[3] as u64);
        range_table_inputs.use_chunk_range_check(aop.range_ab + 26, aop.a[1] as u64);
        range_table_inputs.use_chunk_range_check(aop.range_ab + 17, aop.b[3] as u64);
        range_table_inputs.use_chunk_range_check(aop.range_ab + 9, aop.b[1] as u64);

        range_table_inputs.use_chunk_range_check(aop.range_cd, aop.c[3] as u64);
        range_table_inputs.use_chunk_range_check(aop.range_cd + 26, aop.c[1] as u64);
        range_table_inputs.use_chunk_range_check(aop.range_cd + 17, aop.d[3] as u64);
        range_table_inputs.use_chunk_range_check(aop.range_cd + 9, aop.d[1] as u64);

        let mut carry_values = [0u64; 7];
        for (i, carry_value) in carry_values.iter_mut().enumerate() {
            let carry = if aop.carry[i] >= 0 {
                aop.carry[i] as u64
            } else {
                (aop.carry[i] + F::ORDER_U64 as i64) as u64
            };
            *carry_value = carry;
            range_table_inputs.use_carry_range_check(aop.carry[i]);
        }
        row.set_all_carry(&carry_values);

        row.set_op(aop.op);
        row.set_m32(aop.m32);
        row.set_div(aop.div);
        row.set_na(aop.na);
        row.set_nb(aop.nb);
        row.set_np(aop.np);
        row.set_nr(aop.nr);
        row.set_signed(aop.signed);
        row.set_main_mul(aop.main_mul);
        row.set_main_div(aop.main_div);
        row.set_sext(aop.sext);
        row.set_multiplicity(true);
        row.set_range_ab(aop.range_ab);
        row.set_range_cd(aop.range_cd);
        row.set_div_by_zero(aop.div_by_zero);
        row.set_div_overflow_mul_rz(aop.div_overflow_mul_rz);

        let inv_sum_all_bs = if aop.div && !aop.div_by_zero {
            F::from_u64(aop.b[0] as u64 + aop.b[1] as u64 + aop.b[2] as u64 + aop.b[3] as u64)
                .inverse()
                .as_canonical_u64()
        } else {
            0
        };
        row.set_inv_sum_all_bs(inv_sum_all_bs);

        table_inputs.add_use(
            aop.op,
            aop.na,
            aop.nb,
            aop.np,
            aop.nr,
            aop.sext,
            aop.div_by_zero,
            aop.div_overflow_mul_rz,
        );

        row
    }

    /// Repro (env-gated): forge the Arith witness row for `DIV(1, -1)` so the quotient
    /// sign is positive (claimed quotient +1; the honest quotient is -1). The Main-side
    /// result forgery is in `core/src/ops_core.rs::op_div`. Honest runs (env unset) are
    /// untouched. See elf-regressions/arith_bad_div_sign/README.md.
    fn maybe_inject_bad_div_sign_repro(aop: &mut ArithOperation, opcode: u8, a: u64, b: u64) {
        if opcode != ZiskOp::Div.code()
            || a != REPRO_DIV_SIGN_A
            || b != REPRO_DIV_SIGN_B
            || std::env::var_os(REPRO_BAD_ARITH_DIV_SIGN_ENV).is_none()
        {
            return;
        }

        tracing::warn!("injecting bad Arith DIV sign repro row: DIV(1, -1) quotient forged +1 (honest -1)");

        // THE LIE is the quotient sign. Dividend=1, divisor=-1, remainder=0 stay honest;
        // only the quotient value a[] and its sign na change (and the carries / range_ab
        // that depend on them). The row still satisfies the Arith AIR: carries
        // [-1,-1,-1,-1,0,0,0] balance the eq identity, the flag tuple (na=0,nb=1,np=0,nr=0)
        // is a legal arith_table row (realized honestly at quotient 0, e.g. DIV(1,-2)=0),
        // and the remainder bound |0| < |-1| holds. So the stock circuit accepts rd=+1.
        aop.a = [1, 0, 0, 0]; // quotient = +1   (honest: -1 = [0xFFFF; 4])
        aop.b = [0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF]; // divisor = -1
        aop.c = [1, 0, 0, 0]; // dividend = 1
        aop.d = [0, 0, 0, 0]; // remainder = 0
        aop.carry = [-1, -1, -1, -1, 0, 0, 0];
        aop.m32 = false;
        aop.div = true;
        aop.na = false; // THE LIE: quotient sign positive (honest: true)
        aop.nb = true; // divisor negative
        aop.np = false; // dividend positive
        aop.nr = false; // remainder non-negative
        aop.sext = false;
        aop.main_mul = false;
        aop.main_div = true;
        aop.signed = true;
        aop.range_ab = 5; // na=0 (a3 '+'), nb=1 (b3 '-')  -> rid 5
        aop.range_cd = 4; // np=0 (c3 '+'), nr=0 (d3 '+')  -> rid 4
        aop.div_by_zero = false;
        aop.div_overflow_mul_rz = false;
    }
}
