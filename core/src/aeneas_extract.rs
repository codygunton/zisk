use std::marker::PhantomData;

#[path = "../../riscv/src/fence_decode.rs"]
mod fence_decode;
#[path = "../../riscv/src/rv64im_decode.rs"]
mod rv64im_decode;

use fence_decode::{decode_fence_raw, FenceDecodeKind};
use riscv::RiscvInstruction;
use rv64im_decode::{decode_32_core, DecodedRv64im, RiscvFormat, RiscvOpcode};

use crate::{Riscv2ZiskContext, Rv64imLoweringInput, Rv64imSingleRowOpcode, ZiskInst};

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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rv64imDecodeExtract {
    pub supported: bool,
    pub opcode_id: u32,
    pub format_id: u32,
    pub funct3: u32,
    pub funct7: u32,
    pub rd: u32,
    pub rs1: u32,
    pub rs2: u32,
    pub imm: i32,
    pub pred: u32,
    pub succ: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rv64imTranspileExtract {
    pub accepted: bool,
    pub decode: Rv64imDecodeExtract,
    pub row: ZiskInstExtract,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rv64imTranspileRowsExtract {
    pub accepted: bool,
    pub decode: Rv64imDecodeExtract,
    pub row_count: u32,
    pub first_row: ZiskInstExtract,
    pub last_row: ZiskInstExtract,
}

fn opcode_id(opcode: RiscvOpcode) -> u32 {
    match opcode {
        RiscvOpcode::Lui => 1,
        RiscvOpcode::Auipc => 2,
        RiscvOpcode::Jal => 3,
        RiscvOpcode::Jalr => 4,
        RiscvOpcode::Fence => 5,
        RiscvOpcode::Add => 6,
        RiscvOpcode::Sub => 7,
        RiscvOpcode::Sll => 8,
        RiscvOpcode::Slt => 9,
        RiscvOpcode::Sltu => 10,
        RiscvOpcode::Xor => 11,
        RiscvOpcode::Srl => 12,
        RiscvOpcode::Sra => 13,
        RiscvOpcode::Or => 14,
        RiscvOpcode::And => 15,
        RiscvOpcode::Addw => 16,
        RiscvOpcode::Subw => 17,
        RiscvOpcode::Sllw => 18,
        RiscvOpcode::Srlw => 19,
        RiscvOpcode::Sraw => 20,
        RiscvOpcode::Mul => 21,
        RiscvOpcode::Mulh => 22,
        RiscvOpcode::Mulhsu => 23,
        RiscvOpcode::Mulhu => 24,
        RiscvOpcode::Mulw => 25,
        RiscvOpcode::Div => 26,
        RiscvOpcode::Divu => 27,
        RiscvOpcode::Divw => 28,
        RiscvOpcode::Divuw => 29,
        RiscvOpcode::Rem => 30,
        RiscvOpcode::Remu => 31,
        RiscvOpcode::Remw => 32,
        RiscvOpcode::Remuw => 33,
        RiscvOpcode::Addi => 34,
        RiscvOpcode::Slli => 35,
        RiscvOpcode::Slti => 36,
        RiscvOpcode::Sltiu => 37,
        RiscvOpcode::Xori => 38,
        RiscvOpcode::Srli => 39,
        RiscvOpcode::Srai => 40,
        RiscvOpcode::Ori => 41,
        RiscvOpcode::Andi => 42,
        RiscvOpcode::Addiw => 43,
        RiscvOpcode::Slliw => 44,
        RiscvOpcode::Srliw => 45,
        RiscvOpcode::Sraiw => 46,
        RiscvOpcode::Beq => 47,
        RiscvOpcode::Bne => 48,
        RiscvOpcode::Blt => 49,
        RiscvOpcode::Bge => 50,
        RiscvOpcode::Bltu => 51,
        RiscvOpcode::Bgeu => 52,
        RiscvOpcode::Lb => 53,
        RiscvOpcode::Lbu => 54,
        RiscvOpcode::Lh => 55,
        RiscvOpcode::Lhu => 56,
        RiscvOpcode::Lw => 57,
        RiscvOpcode::Lwu => 58,
        RiscvOpcode::Ld => 59,
        RiscvOpcode::Sb => 60,
        RiscvOpcode::Sh => 61,
        RiscvOpcode::Sw => 62,
        RiscvOpcode::Sd => 63,
        RiscvOpcode::Reserved => 1000,
        RiscvOpcode::Unsupported => 1001,
    }
}

fn format_id(format: RiscvFormat) -> u32 {
    match format {
        RiscvFormat::I => 1,
        RiscvFormat::R => 2,
        RiscvFormat::S => 3,
        RiscvFormat::B => 4,
        RiscvFormat::U => 5,
        RiscvFormat::J => 6,
        RiscvFormat::F => 7,
        RiscvFormat::Invalid => 1000,
        RiscvFormat::Unsupported => 1001,
    }
}

fn decode_extract_from_decoded(decoded: DecodedRv64im) -> Rv64imDecodeExtract {
    Rv64imDecodeExtract {
        supported: decoded.is_supported_rv64im(),
        opcode_id: opcode_id(decoded.opcode),
        format_id: format_id(decoded.format),
        funct3: decoded.funct3,
        funct7: decoded.funct7,
        rd: decoded.rd,
        rs1: decoded.rs1,
        rs2: decoded.rs2,
        imm: decoded.imm,
        pred: decoded.pred,
        succ: decoded.succ,
    }
}

fn lowering_opcode(opcode: RiscvOpcode) -> Option<Rv64imSingleRowOpcode> {
    match opcode {
        RiscvOpcode::Lui => Some(Rv64imSingleRowOpcode::Lui),
        RiscvOpcode::Auipc => Some(Rv64imSingleRowOpcode::Auipc),
        RiscvOpcode::Jal => Some(Rv64imSingleRowOpcode::Jal),
        RiscvOpcode::Jalr => Some(Rv64imSingleRowOpcode::Jalr),
        RiscvOpcode::Fence => Some(Rv64imSingleRowOpcode::Fence),
        RiscvOpcode::Add => Some(Rv64imSingleRowOpcode::Add),
        RiscvOpcode::Sub => Some(Rv64imSingleRowOpcode::Sub),
        RiscvOpcode::Sll => Some(Rv64imSingleRowOpcode::Sll),
        RiscvOpcode::Slt => Some(Rv64imSingleRowOpcode::Slt),
        RiscvOpcode::Sltu => Some(Rv64imSingleRowOpcode::Sltu),
        RiscvOpcode::Xor => Some(Rv64imSingleRowOpcode::Xor),
        RiscvOpcode::Srl => Some(Rv64imSingleRowOpcode::Srl),
        RiscvOpcode::Sra => Some(Rv64imSingleRowOpcode::Sra),
        RiscvOpcode::Or => Some(Rv64imSingleRowOpcode::Or),
        RiscvOpcode::And => Some(Rv64imSingleRowOpcode::And),
        RiscvOpcode::Addw => Some(Rv64imSingleRowOpcode::Addw),
        RiscvOpcode::Subw => Some(Rv64imSingleRowOpcode::Subw),
        RiscvOpcode::Sllw => Some(Rv64imSingleRowOpcode::Sllw),
        RiscvOpcode::Srlw => Some(Rv64imSingleRowOpcode::Srlw),
        RiscvOpcode::Sraw => Some(Rv64imSingleRowOpcode::Sraw),
        RiscvOpcode::Mul => Some(Rv64imSingleRowOpcode::Mul),
        RiscvOpcode::Mulh => Some(Rv64imSingleRowOpcode::Mulh),
        RiscvOpcode::Mulhsu => Some(Rv64imSingleRowOpcode::Mulhsu),
        RiscvOpcode::Mulhu => Some(Rv64imSingleRowOpcode::Mulhu),
        RiscvOpcode::Mulw => Some(Rv64imSingleRowOpcode::Mulw),
        RiscvOpcode::Div => Some(Rv64imSingleRowOpcode::Div),
        RiscvOpcode::Divu => Some(Rv64imSingleRowOpcode::Divu),
        RiscvOpcode::Divw => Some(Rv64imSingleRowOpcode::Divw),
        RiscvOpcode::Divuw => Some(Rv64imSingleRowOpcode::Divuw),
        RiscvOpcode::Rem => Some(Rv64imSingleRowOpcode::Rem),
        RiscvOpcode::Remu => Some(Rv64imSingleRowOpcode::Remu),
        RiscvOpcode::Remw => Some(Rv64imSingleRowOpcode::Remw),
        RiscvOpcode::Remuw => Some(Rv64imSingleRowOpcode::Remuw),
        RiscvOpcode::Addi => Some(Rv64imSingleRowOpcode::Addi),
        RiscvOpcode::Slli => Some(Rv64imSingleRowOpcode::Slli),
        RiscvOpcode::Slti => Some(Rv64imSingleRowOpcode::Slti),
        RiscvOpcode::Sltiu => Some(Rv64imSingleRowOpcode::Sltiu),
        RiscvOpcode::Xori => Some(Rv64imSingleRowOpcode::Xori),
        RiscvOpcode::Srli => Some(Rv64imSingleRowOpcode::Srli),
        RiscvOpcode::Srai => Some(Rv64imSingleRowOpcode::Srai),
        RiscvOpcode::Ori => Some(Rv64imSingleRowOpcode::Ori),
        RiscvOpcode::Andi => Some(Rv64imSingleRowOpcode::Andi),
        RiscvOpcode::Addiw => Some(Rv64imSingleRowOpcode::Addiw),
        RiscvOpcode::Slliw => Some(Rv64imSingleRowOpcode::Slliw),
        RiscvOpcode::Srliw => Some(Rv64imSingleRowOpcode::Srliw),
        RiscvOpcode::Sraiw => Some(Rv64imSingleRowOpcode::Sraiw),
        RiscvOpcode::Beq => Some(Rv64imSingleRowOpcode::Beq),
        RiscvOpcode::Bne => Some(Rv64imSingleRowOpcode::Bne),
        RiscvOpcode::Blt => Some(Rv64imSingleRowOpcode::Blt),
        RiscvOpcode::Bge => Some(Rv64imSingleRowOpcode::Bge),
        RiscvOpcode::Bltu => Some(Rv64imSingleRowOpcode::Bltu),
        RiscvOpcode::Bgeu => Some(Rv64imSingleRowOpcode::Bgeu),
        RiscvOpcode::Lb => Some(Rv64imSingleRowOpcode::Lb),
        RiscvOpcode::Lbu => Some(Rv64imSingleRowOpcode::Lbu),
        RiscvOpcode::Lh => Some(Rv64imSingleRowOpcode::Lh),
        RiscvOpcode::Lhu => Some(Rv64imSingleRowOpcode::Lhu),
        RiscvOpcode::Lw => Some(Rv64imSingleRowOpcode::Lw),
        RiscvOpcode::Lwu => Some(Rv64imSingleRowOpcode::Lwu),
        RiscvOpcode::Ld => Some(Rv64imSingleRowOpcode::Ld),
        RiscvOpcode::Sb => Some(Rv64imSingleRowOpcode::Sb),
        RiscvOpcode::Sh => Some(Rv64imSingleRowOpcode::Sh),
        RiscvOpcode::Sw => Some(Rv64imSingleRowOpcode::Sw),
        RiscvOpcode::Sd => Some(Rv64imSingleRowOpcode::Sd),
        RiscvOpcode::Reserved | RiscvOpcode::Unsupported => None,
    }
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

#[cfg(test)]
fn make_riscv_instruction(
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
            extract_first_inst: None,
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

pub fn extract_lui_from_inst(i: &RiscvInstruction) -> ZiskInstExtract {
    with_context!(i, ctx, {
        ctx.lower_rv64im_single_row(i, Rv64imSingleRowOpcode::Lui, &[]);
    })
}

pub fn extract_auipc_from_inst(i: &RiscvInstruction) -> ZiskInstExtract {
    with_context!(i, ctx, {
        ctx.lower_rv64im_single_row(i, Rv64imSingleRowOpcode::Auipc, &[]);
    })
}

pub fn extract_jal_from_inst(i: &RiscvInstruction) -> ZiskInstExtract {
    with_context!(i, ctx, {
        ctx.lower_rv64im_single_row(i, Rv64imSingleRowOpcode::Jal, &[]);
    })
}

pub fn extract_jalr_from_inst(i: &RiscvInstruction) -> ZiskInstExtract {
    with_context!(i, ctx, {
        ctx.lower_rv64im_single_row(i, Rv64imSingleRowOpcode::Jalr, &[]);
    })
}

pub fn extract_fence_from_inst(i: &RiscvInstruction) -> ZiskInstExtract {
    with_context!(i, ctx, {
        ctx.lower_rv64im_single_row(i, Rv64imSingleRowOpcode::Fence, &[]);
    })
}

pub fn extract_fence_accepts_raw_inst(raw: u32) -> bool {
    let fence = decode_fence_raw(raw);
    fence.kind == FenceDecodeKind::Fence
}

pub fn extract_decode_rv64im_raw(raw: u32) -> Rv64imDecodeExtract {
    decode_extract_from_decoded(decode_32_core(raw))
}

pub fn extract_rv64im_opcode_supported(raw: u32) -> bool {
    decode_32_core(raw).is_supported_rv64im()
}

pub fn extract_transpile_rv64im_raw(raw: u32) -> Rv64imTranspileExtract {
    let decoded = decode_32_core(raw);
    let decode = decode_extract_from_decoded(decoded);
    match lowering_opcode(decoded.opcode) {
        Some(opcode) => {
            let input =
                Rv64imLoweringInput::new(0, decoded.rd, decoded.rs1, decoded.rs2, decoded.imm);
            let row = with_context!(input, ctx, {
                ctx.lower_rv64im_single_row_input(&input, opcode, false);
            });
            Rv64imTranspileExtract { accepted: true, decode, row }
        }
        None => Rv64imTranspileExtract { accepted: false, decode, row: ZiskInstExtract::default() },
    }
}

pub fn extract_transpile_rv64im_rows_raw(raw: u32) -> Rv64imTranspileRowsExtract {
    let terminal = extract_transpile_rv64im_raw(raw);
    if raw & 0x7f == 0x67 && terminal.accepted && terminal.decode.imm % 4 != 0 {
        let input = Rv64imLoweringInput::new(
            0,
            terminal.decode.rd,
            terminal.decode.rs1,
            terminal.decode.rs2,
            terminal.decode.imm,
        );
        let mut ctx = Riscv2ZiskContext {
            extract_inst: None,
            extract_first_inst: None,
            extract_marker: PhantomData,
            input_precompile: None,
            output_precompile: None,
            input_precompile_reg: None,
            output_precompile_reg: None,
        };
        ctx.lower_rv64im_single_row_input(&input, Rv64imSingleRowOpcode::Jalr, false);
        return Rv64imTranspileRowsExtract {
            accepted: true,
            decode: terminal.decode,
            row_count: 2,
            first_row: ctx.extract_first_inst.unwrap(),
            last_row: terminal.row,
        };
    }
    Rv64imTranspileRowsExtract {
        accepted: terminal.accepted,
        decode: terminal.decode,
        row_count: 1,
        first_row: terminal.row,
        last_row: terminal.row,
    }
}

pub fn extract_transpile_rv64im_accepted_raw(raw: u32) -> bool {
    let decoded = decode_32_core(raw);
    lowering_opcode(decoded.opcode).is_some()
}

pub fn extract_transpile_rv64im_materializes_raw(raw: u32) -> bool {
    extract_transpile_rv64im_accepted_raw(raw)
}

macro_rules! register_extract {
    ($name:ident, $opcode:expr) => {
        pub fn $name(i: &RiscvInstruction) -> ZiskInstExtract {
            with_context!(i, ctx, {
                ctx.lower_rv64im_single_row(i, $opcode, &[]);
            })
        }
    };
}

macro_rules! immediate_extract {
    ($name:ident, $opcode:expr) => {
        pub fn $name(i: &RiscvInstruction) -> ZiskInstExtract {
            with_context!(i, ctx, {
                ctx.lower_rv64im_single_row(i, $opcode, &[]);
            })
        }
    };
}

macro_rules! immediate_or_x0_copyb_extract {
    ($name:ident, $opcode:expr) => {
        pub fn $name(i: &RiscvInstruction) -> ZiskInstExtract {
            with_context!(i, ctx, {
                ctx.lower_rv64im_single_row(i, $opcode, &[]);
            })
        }
    };
}

macro_rules! branch_extract {
    ($name:ident, $opcode:expr) => {
        pub fn $name(i: &RiscvInstruction) -> ZiskInstExtract {
            with_context!(i, ctx, {
                ctx.lower_rv64im_single_row(i, $opcode, &[]);
            })
        }
    };
}

macro_rules! load_extract {
    ($name:ident, $opcode:expr) => {
        pub fn $name(i: &RiscvInstruction) -> ZiskInstExtract {
            with_context!(i, ctx, {
                ctx.lower_rv64im_single_row(i, $opcode, &[]);
            })
        }
    };
}

macro_rules! store_extract {
    ($name:ident, $opcode:expr) => {
        pub fn $name(i: &RiscvInstruction) -> ZiskInstExtract {
            with_context!(i, ctx, {
                ctx.lower_rv64im_single_row(i, $opcode, &[]);
            })
        }
    };
}

register_extract!(extract_add_from_inst, Rv64imSingleRowOpcode::Add);
register_extract!(extract_sub_from_inst, Rv64imSingleRowOpcode::Sub);
register_extract!(extract_sll_from_inst, Rv64imSingleRowOpcode::Sll);
register_extract!(extract_slt_from_inst, Rv64imSingleRowOpcode::Slt);
register_extract!(extract_sltu_from_inst, Rv64imSingleRowOpcode::Sltu);
register_extract!(extract_xor_from_inst, Rv64imSingleRowOpcode::Xor);
register_extract!(extract_srl_from_inst, Rv64imSingleRowOpcode::Srl);
register_extract!(extract_sra_from_inst, Rv64imSingleRowOpcode::Sra);
register_extract!(extract_or_from_inst, Rv64imSingleRowOpcode::Or);
register_extract!(extract_and_from_inst, Rv64imSingleRowOpcode::And);
register_extract!(extract_addw_from_inst, Rv64imSingleRowOpcode::Addw);
register_extract!(extract_subw_from_inst, Rv64imSingleRowOpcode::Subw);
register_extract!(extract_sllw_from_inst, Rv64imSingleRowOpcode::Sllw);
register_extract!(extract_srlw_from_inst, Rv64imSingleRowOpcode::Srlw);
register_extract!(extract_sraw_from_inst, Rv64imSingleRowOpcode::Sraw);
register_extract!(extract_mul_from_inst, Rv64imSingleRowOpcode::Mul);
register_extract!(extract_mulh_from_inst, Rv64imSingleRowOpcode::Mulh);
register_extract!(extract_mulhsu_from_inst, Rv64imSingleRowOpcode::Mulhsu);
register_extract!(extract_mulhu_from_inst, Rv64imSingleRowOpcode::Mulhu);
register_extract!(extract_mulw_from_inst, Rv64imSingleRowOpcode::Mulw);
register_extract!(extract_div_from_inst, Rv64imSingleRowOpcode::Div);
register_extract!(extract_divu_from_inst, Rv64imSingleRowOpcode::Divu);
register_extract!(extract_divw_from_inst, Rv64imSingleRowOpcode::Divw);
register_extract!(extract_divuw_from_inst, Rv64imSingleRowOpcode::Divuw);
register_extract!(extract_rem_from_inst, Rv64imSingleRowOpcode::Rem);
register_extract!(extract_remu_from_inst, Rv64imSingleRowOpcode::Remu);
register_extract!(extract_remw_from_inst, Rv64imSingleRowOpcode::Remw);
register_extract!(extract_remuw_from_inst, Rv64imSingleRowOpcode::Remuw);

immediate_or_x0_copyb_extract!(extract_addi_from_inst, Rv64imSingleRowOpcode::Addi);
immediate_extract!(extract_slli_from_inst, Rv64imSingleRowOpcode::Slli);
immediate_extract!(extract_slti_from_inst, Rv64imSingleRowOpcode::Slti);
immediate_extract!(extract_sltiu_from_inst, Rv64imSingleRowOpcode::Sltiu);
immediate_or_x0_copyb_extract!(extract_xori_from_inst, Rv64imSingleRowOpcode::Xori);
immediate_extract!(extract_srli_from_inst, Rv64imSingleRowOpcode::Srli);
immediate_extract!(extract_srai_from_inst, Rv64imSingleRowOpcode::Srai);
immediate_or_x0_copyb_extract!(extract_ori_from_inst, Rv64imSingleRowOpcode::Ori);
immediate_extract!(extract_andi_from_inst, Rv64imSingleRowOpcode::Andi);
immediate_extract!(extract_addiw_from_inst, Rv64imSingleRowOpcode::Addiw);
immediate_extract!(extract_slliw_from_inst, Rv64imSingleRowOpcode::Slliw);
immediate_extract!(extract_srliw_from_inst, Rv64imSingleRowOpcode::Srliw);
immediate_extract!(extract_sraiw_from_inst, Rv64imSingleRowOpcode::Sraiw);

branch_extract!(extract_beq_from_inst, Rv64imSingleRowOpcode::Beq);
branch_extract!(extract_bne_from_inst, Rv64imSingleRowOpcode::Bne);
branch_extract!(extract_blt_from_inst, Rv64imSingleRowOpcode::Blt);
branch_extract!(extract_bge_from_inst, Rv64imSingleRowOpcode::Bge);
branch_extract!(extract_bltu_from_inst, Rv64imSingleRowOpcode::Bltu);
branch_extract!(extract_bgeu_from_inst, Rv64imSingleRowOpcode::Bgeu);

load_extract!(extract_lb_from_inst, Rv64imSingleRowOpcode::Lb);
load_extract!(extract_lbu_from_inst, Rv64imSingleRowOpcode::Lbu);
load_extract!(extract_lh_from_inst, Rv64imSingleRowOpcode::Lh);
load_extract!(extract_lhu_from_inst, Rv64imSingleRowOpcode::Lhu);
load_extract!(extract_lw_from_inst, Rv64imSingleRowOpcode::Lw);
load_extract!(extract_lwu_from_inst, Rv64imSingleRowOpcode::Lwu);
load_extract!(extract_ld_from_inst, Rv64imSingleRowOpcode::Ld);

store_extract!(extract_sb_from_inst, Rv64imSingleRowOpcode::Sb);
store_extract!(extract_sh_from_inst, Rv64imSingleRowOpcode::Sh);
store_extract!(extract_sw_from_inst, Rv64imSingleRowOpcode::Sw);
store_extract!(extract_sd_from_inst, Rv64imSingleRowOpcode::Sd);

#[cfg(test)]
mod tests {
    use super::*;

    type ExtractStart = fn(&RiscvInstruction) -> ZiskInstExtract;

    fn sample_inst(mnemonic: &str) -> RiscvInstruction {
        let mut i = make_riscv_instruction(16, 3, 5, 7, 4096);
        i.inst = mnemonic.to_owned();
        i
    }

    fn convert_single_row(i: &RiscvInstruction) -> ZiskInstExtract {
        let mut ctx = Riscv2ZiskContext {
            extract_inst: None,
            extract_first_inst: None,
            extract_marker: PhantomData,
            input_precompile: None,
            output_precompile: None,
            input_precompile_reg: None,
            output_precompile_reg: None,
        };
        ctx.convert(i, &[]);
        ZiskInstExtract::from_inst(&ctx.extract_inst.unwrap().i)
    }

    #[test]
    fn raw_fence_extraction_uses_production_decoder_gate() {
        assert!(extract_fence_accepts_raw_inst(0x0000000F));
        assert!(!extract_fence_accepts_raw_inst(0x1000000F)); // fm = 1
        assert!(!extract_fence_accepts_raw_inst(0x0000800F)); // rs1 = x1
        assert!(!extract_fence_accepts_raw_inst(0x0000008F)); // rd = x1
        assert!(!extract_fence_accepts_raw_inst(0x0000100F)); // fence.i
        assert!(!extract_fence_accepts_raw_inst(0x00000013)); // addi/nop
    }

    #[test]
    fn raw_rv64im_extraction_uses_general_decoder_gate() {
        let add = extract_decode_rv64im_raw(0x00b50533);
        assert!(add.supported);
        assert_eq!(add.opcode_id, 6);
        assert!(extract_rv64im_opcode_supported(0x00b50533));
        let add_transpile = extract_transpile_rv64im_raw(0x00b50533);
        assert!(add_transpile.accepted);
        assert!(extract_transpile_rv64im_accepted_raw(0x00b50533));
        assert!(extract_transpile_rv64im_materializes_raw(0x00b50533));
        assert_eq!(add_transpile.row.b_src, crate::SRC_REG);
        assert_eq!(add_transpile.row.b_offset_imm0, 11);

        let fence_with_fm = extract_decode_rv64im_raw(0x1000000F);
        assert!(!fence_with_fm.supported);
        assert_eq!(fence_with_fm.opcode_id, 1000);
        assert!(!extract_rv64im_opcode_supported(0x1000000F));
        assert!(!extract_transpile_rv64im_raw(0x1000000F).accepted);
        assert!(!extract_transpile_rv64im_accepted_raw(0x1000000F));
        assert!(!extract_transpile_rv64im_materializes_raw(0x1000000F));
    }

    #[test]
    fn raw_jalr_extraction_exposes_one_or_two_production_rows() {
        // jalr x2, 4(x1): aligned immediate, one terminal AND row.
        let aligned = extract_transpile_rv64im_rows_raw(0x00408167);
        assert!(aligned.accepted);
        assert_eq!(aligned.row_count, 1);
        assert_eq!(aligned.first_row, aligned.last_row);
        assert_eq!(aligned.last_row.op, crate::zisk_ops::ZiskOp::And.code());

        // jalr x2, 2(x1): unaligned immediate, intermediate ADD then terminal AND.
        let unaligned = extract_transpile_rv64im_rows_raw(0x00208167);
        assert!(unaligned.accepted);
        assert_eq!(unaligned.row_count, 2);
        assert_eq!(unaligned.first_row.paddr, 0);
        assert_eq!(unaligned.last_row.paddr, 1);
        assert_eq!(unaligned.first_row.op, crate::zisk_ops::ZiskOp::Add.code());
        assert_eq!(unaligned.last_row.op, crate::zisk_ops::ZiskOp::And.code());
    }

    #[test]
    fn extraction_starts_match_production_convert_for_single_row_opcodes() {
        let cases: &[(&str, ExtractStart)] = &[
            ("lui", extract_lui_from_inst),
            ("auipc", extract_auipc_from_inst),
            ("jal", extract_jal_from_inst),
            ("jalr", extract_jalr_from_inst),
            ("fence", extract_fence_from_inst),
            ("add", extract_add_from_inst),
            ("sub", extract_sub_from_inst),
            ("sll", extract_sll_from_inst),
            ("slt", extract_slt_from_inst),
            ("sltu", extract_sltu_from_inst),
            ("xor", extract_xor_from_inst),
            ("srl", extract_srl_from_inst),
            ("sra", extract_sra_from_inst),
            ("or", extract_or_from_inst),
            ("and", extract_and_from_inst),
            ("addw", extract_addw_from_inst),
            ("subw", extract_subw_from_inst),
            ("sllw", extract_sllw_from_inst),
            ("srlw", extract_srlw_from_inst),
            ("sraw", extract_sraw_from_inst),
            ("mul", extract_mul_from_inst),
            ("mulh", extract_mulh_from_inst),
            ("mulhsu", extract_mulhsu_from_inst),
            ("mulhu", extract_mulhu_from_inst),
            ("mulw", extract_mulw_from_inst),
            ("div", extract_div_from_inst),
            ("divu", extract_divu_from_inst),
            ("divw", extract_divw_from_inst),
            ("divuw", extract_divuw_from_inst),
            ("rem", extract_rem_from_inst),
            ("remu", extract_remu_from_inst),
            ("remw", extract_remw_from_inst),
            ("remuw", extract_remuw_from_inst),
            ("addi", extract_addi_from_inst),
            ("slli", extract_slli_from_inst),
            ("slti", extract_slti_from_inst),
            ("sltiu", extract_sltiu_from_inst),
            ("xori", extract_xori_from_inst),
            ("srli", extract_srli_from_inst),
            ("srai", extract_srai_from_inst),
            ("ori", extract_ori_from_inst),
            ("andi", extract_andi_from_inst),
            ("addiw", extract_addiw_from_inst),
            ("slliw", extract_slliw_from_inst),
            ("srliw", extract_srliw_from_inst),
            ("sraiw", extract_sraiw_from_inst),
            ("beq", extract_beq_from_inst),
            ("bne", extract_bne_from_inst),
            ("blt", extract_blt_from_inst),
            ("bge", extract_bge_from_inst),
            ("bltu", extract_bltu_from_inst),
            ("bgeu", extract_bgeu_from_inst),
            ("lb", extract_lb_from_inst),
            ("lbu", extract_lbu_from_inst),
            ("lh", extract_lh_from_inst),
            ("lhu", extract_lhu_from_inst),
            ("lw", extract_lw_from_inst),
            ("lwu", extract_lwu_from_inst),
            ("ld", extract_ld_from_inst),
            ("sb", extract_sb_from_inst),
            ("sh", extract_sh_from_inst),
            ("sw", extract_sw_from_inst),
            ("sd", extract_sd_from_inst),
        ];

        for (mnemonic, extract) in cases {
            let i = sample_inst(mnemonic);
            assert_eq!(extract(&i), convert_single_row(&i), "{mnemonic}");
        }
    }
}
