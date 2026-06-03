use std::marker::PhantomData;

use riscv::RiscvInstruction;

use crate::{zisk_ops::ZiskOp, Riscv2ZiskContext, ZiskInst};

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

pub fn extract_fence_from_inst(i: &RiscvInstruction) -> ZiskInstExtract {
    with_context!(i, ctx, {
        ctx.nop(i, 4);
    })
}

macro_rules! register_extract {
    ($name:ident, $op:expr) => {
        pub fn $name(i: &RiscvInstruction) -> ZiskInstExtract {
            with_context!(i, ctx, {
                ctx.create_register_op_zisk(i, $op, 4);
            })
        }
    };
}

macro_rules! immediate_extract {
    ($name:ident, $op:expr) => {
        pub fn $name(i: &RiscvInstruction) -> ZiskInstExtract {
            with_context!(i, ctx, {
                ctx.immediate_op_zisk(i, $op, 4);
            })
        }
    };
}

macro_rules! immediate_or_x0_copyb_extract {
    ($name:ident, $op:expr) => {
        pub fn $name(i: &RiscvInstruction) -> ZiskInstExtract {
            with_context!(i, ctx, {
                ctx.immediate_op_or_x0_copyb_zisk(i, $op, 4);
            })
        }
    };
}

macro_rules! branch_extract {
    ($name:ident, $op:expr, $neg:expr) => {
        pub fn $name(i: &RiscvInstruction) -> ZiskInstExtract {
            with_context!(i, ctx, {
                ctx.create_branch_op_zisk(i, $op, $neg, 4);
            })
        }
    };
}

macro_rules! load_extract {
    ($name:ident, $op:expr, $width:expr) => {
        pub fn $name(i: &RiscvInstruction) -> ZiskInstExtract {
            with_context!(i, ctx, {
                ctx.load_op_zisk(i, $op, $width, 4);
            })
        }
    };
}

macro_rules! store_extract {
    ($name:ident, $width:expr) => {
        pub fn $name(i: &RiscvInstruction) -> ZiskInstExtract {
            with_context!(i, ctx, {
                ctx.store_op_zisk(i, ZiskOp::CopyB, $width, 4);
            })
        }
    };
}

register_extract!(extract_add_from_inst, ZiskOp::Add);
register_extract!(extract_sub_from_inst, ZiskOp::Sub);
register_extract!(extract_sll_from_inst, ZiskOp::Sll);
register_extract!(extract_slt_from_inst, ZiskOp::Lt);
register_extract!(extract_sltu_from_inst, ZiskOp::Ltu);
register_extract!(extract_xor_from_inst, ZiskOp::Xor);
register_extract!(extract_srl_from_inst, ZiskOp::Srl);
register_extract!(extract_sra_from_inst, ZiskOp::Sra);
register_extract!(extract_or_from_inst, ZiskOp::Or);
register_extract!(extract_and_from_inst, ZiskOp::And);
register_extract!(extract_addw_from_inst, ZiskOp::AddW);
register_extract!(extract_subw_from_inst, ZiskOp::SubW);
register_extract!(extract_sllw_from_inst, ZiskOp::SllW);
register_extract!(extract_srlw_from_inst, ZiskOp::SrlW);
register_extract!(extract_sraw_from_inst, ZiskOp::SraW);

immediate_or_x0_copyb_extract!(extract_addi_from_inst, ZiskOp::Add);
immediate_extract!(extract_slli_from_inst, ZiskOp::Sll);
immediate_extract!(extract_slti_from_inst, ZiskOp::Lt);
immediate_extract!(extract_sltiu_from_inst, ZiskOp::Ltu);
immediate_or_x0_copyb_extract!(extract_xori_from_inst, ZiskOp::Xor);
immediate_extract!(extract_srli_from_inst, ZiskOp::Srl);
immediate_extract!(extract_srai_from_inst, ZiskOp::Sra);
immediate_or_x0_copyb_extract!(extract_ori_from_inst, ZiskOp::Or);
immediate_extract!(extract_andi_from_inst, ZiskOp::And);
immediate_extract!(extract_addiw_from_inst, ZiskOp::AddW);
immediate_extract!(extract_slliw_from_inst, ZiskOp::SllW);
immediate_extract!(extract_srliw_from_inst, ZiskOp::SrlW);
immediate_extract!(extract_sraiw_from_inst, ZiskOp::SraW);

branch_extract!(extract_beq_from_inst, ZiskOp::Eq, false);
branch_extract!(extract_bne_from_inst, ZiskOp::Eq, true);
branch_extract!(extract_blt_from_inst, ZiskOp::Lt, false);
branch_extract!(extract_bge_from_inst, ZiskOp::Lt, true);
branch_extract!(extract_bltu_from_inst, ZiskOp::Ltu, false);
branch_extract!(extract_bgeu_from_inst, ZiskOp::Ltu, true);

load_extract!(extract_lb_from_inst, ZiskOp::SignExtendB, 1);
load_extract!(extract_lbu_from_inst, ZiskOp::CopyB, 1);
load_extract!(extract_lh_from_inst, ZiskOp::SignExtendH, 2);
load_extract!(extract_lhu_from_inst, ZiskOp::CopyB, 2);
load_extract!(extract_lw_from_inst, ZiskOp::SignExtendW, 4);
load_extract!(extract_lwu_from_inst, ZiskOp::CopyB, 4);
load_extract!(extract_ld_from_inst, ZiskOp::CopyB, 8);

store_extract!(extract_sb_from_inst, 1);
store_extract!(extract_sh_from_inst, 2);
store_extract!(extract_sw_from_inst, 4);
store_extract!(extract_sd_from_inst, 8);
