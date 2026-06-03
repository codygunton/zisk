use std::marker::PhantomData;

use riscv::RiscvInstruction;

use crate::{Riscv2ZiskContext, ZiskInst};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ZiskInstExtract {
    pub paddr: u64,
    pub store_pc: bool,
    pub store_use_sp: bool,
    pub store: u64,
    pub store_offset: i64,
    pub set_pc: bool,
    pub is_precompiled: bool,
    pub ind_width: u64,
    pub end: bool,
    pub a_src: u64,
    pub a_use_sp_imm1: u64,
    pub a_offset_imm0: u64,
    pub b_src: u64,
    pub b_use_sp_imm1: u64,
    pub b_offset_imm0: u64,
    pub jmp_offset1: i64,
    pub jmp_offset2: i64,
    pub is_external_op: bool,
    pub op: u8,
    pub op_type_id: u32,
    pub m32: bool,
    pub input_size: u64,
    pub sorted_pc_list_index: usize,
}

impl ZiskInstExtract {
    pub fn from_inst(i: &ZiskInst) -> Self {
        Self {
            paddr: i.paddr,
            store_pc: i.store_pc,
            store_use_sp: i.store_use_sp,
            store: i.store,
            store_offset: i.store_offset,
            set_pc: i.set_pc,
            is_precompiled: i.is_precompiled,
            ind_width: i.ind_width,
            end: i.end,
            a_src: i.a_src,
            a_use_sp_imm1: i.a_use_sp_imm1,
            a_offset_imm0: i.a_offset_imm0,
            b_src: i.b_src,
            b_use_sp_imm1: i.b_use_sp_imm1,
            b_offset_imm0: i.b_offset_imm0,
            jmp_offset1: i.jmp_offset1,
            jmp_offset2: i.jmp_offset2,
            is_external_op: i.is_external_op,
            op: i.op,
            op_type_id: i.op_type as u32,
            m32: i.m32,
            input_size: i.input_size,
            sorted_pc_list_index: i.sorted_pc_list_index,
        }
    }
}

pub fn make_riscv_instruction(
    rom_address: u64,
    rd: u32,
    rs1: u32,
    rs2: u32,
    imm: i32,
) -> RiscvInstruction {
    RiscvInstruction {
        rom_address,
        rvinst: 0,
        t: String::new(),
        funct2: 0,
        funct3: 0,
        funct5: 0,
        funct7: 0,
        rd,
        rs1,
        rs2,
        rs3: 0,
        imm,
        imme: 0,
        inst: String::new(),
        aq: 0,
        rl: 0,
        csr: 0,
        pred: 0,
        succ: 0,
    }
}

macro_rules! with_context {
    ($i:ident, $ctx:ident, $body:block) => {{
        let mut $ctx = Riscv2ZiskContext {
            extract_inst: None,
            extract_marker: PhantomData,
            input_precompile: None,
            output_precompile: None,
            input_precompile_reg: None,
            output_precompile_reg: None,
        };
        $body
        ZiskInstExtract::from_inst(&$ctx.extract_inst.unwrap().i)
    }};
}

pub fn extract_lui(rom_address: u64, rd: u32, imm: i32) -> ZiskInstExtract {
    let i = make_riscv_instruction(rom_address, rd, 0, 0, imm);
    extract_lui_from_inst(&i)
}

pub fn extract_auipc(rom_address: u64, rd: u32, imm: i32) -> ZiskInstExtract {
    let i = make_riscv_instruction(rom_address, rd, 0, 0, imm);
    extract_auipc_from_inst(&i)
}

pub fn extract_jal(rom_address: u64, rd: u32, imm: i32) -> ZiskInstExtract {
    let i = make_riscv_instruction(rom_address, rd, 0, 0, imm);
    extract_jal_from_inst(&i)
}

pub fn extract_jalr(rom_address: u64, rd: u32, rs1: u32, imm: i32) -> ZiskInstExtract {
    let i = make_riscv_instruction(rom_address, rd, rs1, 0, imm);
    extract_jalr_from_inst(&i)
}

pub fn extract_lui_from_inst(i: &RiscvInstruction) -> ZiskInstExtract {
    with_context!(i, ctx, {
        ctx.lui(i, 4);
    })
}

pub fn extract_auipc_from_inst(i: &RiscvInstruction) -> ZiskInstExtract {
    with_context!(i, ctx, {
        ctx.auipc(i);
    })
}

pub fn extract_jal_from_inst(i: &RiscvInstruction) -> ZiskInstExtract {
    with_context!(i, ctx, {
        ctx.jal(i, 4);
    })
}

pub fn extract_jalr_from_inst(i: &RiscvInstruction) -> ZiskInstExtract {
    with_context!(i, ctx, {
        ctx.jalr(i, 4);
    })
}

pub fn extract_register_op(
    rom_address: u64,
    rd: u32,
    rs1: u32,
    rs2: u32,
    op: &str,
) -> ZiskInstExtract {
    let i = make_riscv_instruction(rom_address, rd, rs1, rs2, 0);
    with_context!(i, ctx, {
        ctx.create_register_op(&i, op, 4);
    })
}

pub fn extract_immediate_op(
    rom_address: u64,
    rd: u32,
    rs1: u32,
    imm: i32,
    op: &str,
) -> ZiskInstExtract {
    let i = make_riscv_instruction(rom_address, rd, rs1, 0, imm);
    with_context!(i, ctx, {
        ctx.immediate_op(&i, op, 4);
    })
}

pub fn extract_branch_op(
    rom_address: u64,
    rs1: u32,
    rs2: u32,
    imm: i32,
    op: &str,
    neg: bool,
) -> ZiskInstExtract {
    let i = make_riscv_instruction(rom_address, 0, rs1, rs2, imm);
    with_context!(i, ctx, {
        ctx.create_branch_op(&i, op, neg, 4);
    })
}
