//! Extractable RV64IM 32-bit decode and static transpiler core.
//!
//! This module is intentionally free of `HashMap`, string dispatch, formatting,
//! and builder mutation in its decode/lower core so it can be targeted by
//! Charon/Aeneas. The adapter at the bottom converts static rows back into
//! production `ZiskInst`s.

use riscv::RiscvInstruction;

use crate::zisk_ops::ZiskOp;
use crate::{ZiskInst, SRC_C, SRC_IMM, SRC_IND, SRC_REG, STORE_IND, STORE_NONE, STORE_REG};

const OP_FLAG: u64 = 0x00;
const OP_COPYB: u64 = 0x01;
const OP_LTU: u64 = 0x06;
const OP_LT: u64 = 0x07;
const OP_EQ: u64 = 0x09;
const OP_ADD: u64 = 0x0a;
const OP_SUB: u64 = 0x0b;
const OP_AND: u64 = 0x0e;
const OP_OR: u64 = 0x0f;
const OP_XOR: u64 = 0x10;
const OP_ADD_W: u64 = 0x1a;
const OP_SUB_W: u64 = 0x1b;
const OP_SLL: u64 = 0x21;
const OP_SRL: u64 = 0x22;
const OP_SRA: u64 = 0x23;
const OP_SLL_W: u64 = 0x24;
const OP_SRL_W: u64 = 0x25;
const OP_SRA_W: u64 = 0x26;
const OP_SIGNEXT_B: u64 = 0x27;
const OP_SIGNEXT_H: u64 = 0x28;
const OP_SIGNEXT_W: u64 = 0x29;
const OP_MULHU: u64 = 0xb1;
const OP_MULHSU: u64 = 0xb3;
const OP_MUL: u64 = 0xb4;
const OP_MULH: u64 = 0xb5;
const OP_MUL_W: u64 = 0xb6;
const OP_DIVU: u64 = 0xb8;
const OP_REMU: u64 = 0xb9;
const OP_DIV: u64 = 0xba;
const OP_REM: u64 = 0xbb;
const OP_DIVU_W: u64 = 0xbc;
const OP_REMU_W: u64 = 0xbd;
const OP_DIV_W: u64 = 0xbe;
const OP_REM_W: u64 = 0xbf;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rv64imOp {
    Add,
    Sub,
    Sll,
    Slt,
    Sltu,
    Xor,
    Srl,
    Sra,
    Or,
    And,
    Addw,
    Subw,
    Sllw,
    Srlw,
    Sraw,
    Addi,
    Slli,
    Slti,
    Sltiu,
    Xori,
    Srli,
    Srai,
    Ori,
    Andi,
    Addiw,
    Slliw,
    Srliw,
    Sraiw,
    Beq,
    Bne,
    Blt,
    Bge,
    Bltu,
    Bgeu,
    Lb,
    Lbu,
    Lh,
    Lhu,
    Lw,
    Lwu,
    Ld,
    Sb,
    Sh,
    Sw,
    Sd,
    Lui,
    Auipc,
    Jal,
    Jalr,
    Fence,
    Mul,
    Mulh,
    Mulhsu,
    Mulhu,
    Mulw,
    Div,
    Divu,
    Divw,
    Divuw,
    Rem,
    Remu,
    Remw,
    Remuw,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rv64imInst {
    pub paddr: u64,
    pub op: Rv64imOp,
    pub rd: u32,
    pub rs1: u32,
    pub rs2: u32,
    pub imm: i32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StaticRow {
    pub paddr: u64,
    pub op: u64,
    pub a_src: u64,
    pub a_use_sp_imm1: u64,
    pub a_offset_imm0: u64,
    pub b_src: u64,
    pub b_use_sp_imm1: u64,
    pub b_offset_imm0: u64,
    pub store: u64,
    pub store_offset: i64,
    pub store_pc: bool,
    pub set_pc: bool,
    pub ind_width: u64,
    pub jmp_offset1: i64,
    pub jmp_offset2: i64,
    pub is_external_op: bool,
    pub m32: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StaticRows {
    pub rows: [StaticRow; 2],
    pub len: usize,
}

impl StaticRows {
    fn one(row: StaticRow) -> Self {
        Self { rows: [row, StaticRow::default()], len: 1 }
    }

    fn two(first: StaticRow, second: StaticRow) -> Self {
        Self { rows: [first, second], len: 2 }
    }

    pub fn as_slice(&self) -> &[StaticRow] {
        &self.rows[..self.len]
    }
}

pub fn decode_rv64im32(word: u32, paddr: u64) -> Option<Rv64imInst> {
    let opcode = word & 0x7f;
    let funct3 = (word >> 12) & 0x7;
    let funct7 = (word >> 25) & 0x7f;
    let rd = (word >> 7) & 0x1f;
    let rs1 = (word >> 15) & 0x1f;
    let rs2 = (word >> 20) & 0x1f;
    let op = match opcode {
        0x03 => match funct3 {
            0 => Rv64imOp::Lb,
            1 => Rv64imOp::Lh,
            2 => Rv64imOp::Lw,
            3 => Rv64imOp::Ld,
            4 => Rv64imOp::Lbu,
            5 => Rv64imOp::Lhu,
            6 => Rv64imOp::Lwu,
            _ => return None,
        },
        0x0f => match funct3 {
            0 => Rv64imOp::Fence,
            _ => return None,
        },
        0x13 => match funct3 {
            0 => Rv64imOp::Addi,
            1 if (word >> 26) & 0x3f == 0 => Rv64imOp::Slli,
            2 => Rv64imOp::Slti,
            3 => Rv64imOp::Sltiu,
            4 => Rv64imOp::Xori,
            5 if (word >> 26) & 0x3f == 0 => Rv64imOp::Srli,
            5 if (word >> 26) & 0x3f == 16 => Rv64imOp::Srai,
            6 => Rv64imOp::Ori,
            7 => Rv64imOp::Andi,
            _ => return None,
        },
        0x17 => Rv64imOp::Auipc,
        0x1b => match funct3 {
            0 => Rv64imOp::Addiw,
            1 if funct7 == 0 => Rv64imOp::Slliw,
            5 if funct7 == 0 => Rv64imOp::Srliw,
            5 if funct7 == 32 => Rv64imOp::Sraiw,
            _ => return None,
        },
        0x23 => match funct3 {
            0 => Rv64imOp::Sb,
            1 => Rv64imOp::Sh,
            2 => Rv64imOp::Sw,
            3 => Rv64imOp::Sd,
            _ => return None,
        },
        0x33 => match (funct3, funct7) {
            (0, 0) => Rv64imOp::Add,
            (0, 1) => Rv64imOp::Mul,
            (0, 32) => Rv64imOp::Sub,
            (1, 0) => Rv64imOp::Sll,
            (1, 1) => Rv64imOp::Mulh,
            (2, 0) => Rv64imOp::Slt,
            (2, 1) => Rv64imOp::Mulhsu,
            (3, 0) => Rv64imOp::Sltu,
            (3, 1) => Rv64imOp::Mulhu,
            (4, 0) => Rv64imOp::Xor,
            (4, 1) => Rv64imOp::Div,
            (5, 0) => Rv64imOp::Srl,
            (5, 1) => Rv64imOp::Divu,
            (5, 32) => Rv64imOp::Sra,
            (6, 0) => Rv64imOp::Or,
            (6, 1) => Rv64imOp::Rem,
            (7, 0) => Rv64imOp::And,
            (7, 1) => Rv64imOp::Remu,
            _ => return None,
        },
        0x37 => Rv64imOp::Lui,
        0x3b => match (funct3, funct7) {
            (0, 0) => Rv64imOp::Addw,
            (0, 1) => Rv64imOp::Mulw,
            (0, 32) => Rv64imOp::Subw,
            (1, 0) => Rv64imOp::Sllw,
            (4, 1) => Rv64imOp::Divw,
            (5, 0) => Rv64imOp::Srlw,
            (5, 1) => Rv64imOp::Divuw,
            (5, 32) => Rv64imOp::Sraw,
            (6, 1) => Rv64imOp::Remw,
            (7, 1) => Rv64imOp::Remuw,
            _ => return None,
        },
        0x63 => match funct3 {
            0 => Rv64imOp::Beq,
            1 => Rv64imOp::Bne,
            4 => Rv64imOp::Blt,
            5 => Rv64imOp::Bge,
            6 => Rv64imOp::Bltu,
            7 => Rv64imOp::Bgeu,
            _ => return None,
        },
        0x67 if funct3 == 0 => Rv64imOp::Jalr,
        0x6f => Rv64imOp::Jal,
        _ => return None,
    };
    let (rd, rs1, rs2) = decoded_registers(op, rd, rs1, rs2);
    Some(Rv64imInst { paddr, op, rd, rs1, rs2, imm: decode_imm(word, op) })
}

pub fn lower_rv64im32(inst: Rv64imInst) -> StaticRows {
    match inst.op {
        Rv64imOp::Add => {
            if inst.rs1 == 0 {
                copyb_from_reg(inst, inst.rs2)
            } else if inst.rs2 == 0 {
                copyb_from_reg(inst, inst.rs1)
            } else {
                register_op(inst, OP_ADD)
            }
        }
        Rv64imOp::Or => {
            if inst.rs1 == 0 {
                copyb_from_reg(inst, inst.rs2)
            } else if inst.rs2 == 0 {
                copyb_from_reg(inst, inst.rs1)
            } else {
                register_op(inst, OP_OR)
            }
        }
        Rv64imOp::Sub => register_op(inst, OP_SUB),
        Rv64imOp::Sll => register_op(inst, OP_SLL),
        Rv64imOp::Slt => register_op(inst, OP_LT),
        Rv64imOp::Sltu => register_op(inst, OP_LTU),
        Rv64imOp::Xor => register_op(inst, OP_XOR),
        Rv64imOp::Srl => register_op(inst, OP_SRL),
        Rv64imOp::Sra => register_op(inst, OP_SRA),
        Rv64imOp::And => register_op(inst, OP_AND),
        Rv64imOp::Addw => register_op(inst, OP_ADD_W),
        Rv64imOp::Subw => register_op(inst, OP_SUB_W),
        Rv64imOp::Sllw => register_op(inst, OP_SLL_W),
        Rv64imOp::Srlw => register_op(inst, OP_SRL_W),
        Rv64imOp::Sraw => register_op(inst, OP_SRA_W),
        Rv64imOp::Mul => register_op(inst, OP_MUL),
        Rv64imOp::Mulh => register_op(inst, OP_MULH),
        Rv64imOp::Mulhsu => register_op(inst, OP_MULHSU),
        Rv64imOp::Mulhu => register_op(inst, OP_MULHU),
        Rv64imOp::Mulw => register_op(inst, OP_MUL_W),
        Rv64imOp::Div => register_op(inst, OP_DIV),
        Rv64imOp::Divu => register_op(inst, OP_DIVU),
        Rv64imOp::Divw => register_op(inst, OP_DIV_W),
        Rv64imOp::Divuw => register_op(inst, OP_DIVU_W),
        Rv64imOp::Rem => register_op(inst, OP_REM),
        Rv64imOp::Remu => register_op(inst, OP_REMU),
        Rv64imOp::Remw => register_op(inst, OP_REM_W),
        Rv64imOp::Remuw => register_op(inst, OP_REMU_W),
        Rv64imOp::Addi => {
            if inst.rd == 0 {
                if inst.rs1 == 0 {
                    nop(inst)
                } else {
                    hint(inst)
                }
            } else if inst.imm == 0 && inst.rs1 != 0 {
                copyb_from_reg(inst, inst.rs1)
            } else {
                immediate_op_or_x0_copyb(inst, OP_ADD)
            }
        }
        Rv64imOp::Xori => immediate_op_or_x0_copyb(inst, OP_XOR),
        Rv64imOp::Ori => immediate_op_or_x0_copyb(inst, OP_OR),
        Rv64imOp::Slti => immediate_op(inst, OP_LT),
        Rv64imOp::Sltiu => immediate_op(inst, OP_LTU),
        Rv64imOp::Andi => immediate_op(inst, OP_AND),
        Rv64imOp::Slli => immediate_op(inst, OP_SLL),
        Rv64imOp::Srli => immediate_op(inst, OP_SRL),
        Rv64imOp::Srai => immediate_op(inst, OP_SRA),
        Rv64imOp::Addiw => {
            if inst.rd == 0 && inst.rs1 == 0 && inst.imm == 0 {
                nop(inst)
            } else {
                immediate_op(inst, OP_ADD_W)
            }
        }
        Rv64imOp::Slliw => immediate_op(inst, OP_SLL_W),
        Rv64imOp::Srliw => immediate_op(inst, OP_SRL_W),
        Rv64imOp::Sraiw => immediate_op(inst, OP_SRA_W),
        Rv64imOp::Beq => branch_op(inst, OP_EQ, false),
        Rv64imOp::Bne => branch_op(inst, OP_EQ, true),
        Rv64imOp::Blt => branch_op(inst, OP_LT, false),
        Rv64imOp::Bge => branch_op(inst, OP_LT, true),
        Rv64imOp::Bltu => branch_op(inst, OP_LTU, false),
        Rv64imOp::Bgeu => branch_op(inst, OP_LTU, true),
        Rv64imOp::Lb => load_op(inst, OP_SIGNEXT_B, 1),
        Rv64imOp::Lbu => load_op(inst, OP_COPYB, 1),
        Rv64imOp::Lh => load_op(inst, OP_SIGNEXT_H, 2),
        Rv64imOp::Lhu => load_op(inst, OP_COPYB, 2),
        Rv64imOp::Lw => load_op(inst, OP_SIGNEXT_W, 4),
        Rv64imOp::Lwu => load_op(inst, OP_COPYB, 4),
        Rv64imOp::Ld => load_op(inst, OP_COPYB, 8),
        Rv64imOp::Sb => store_op(inst, 1),
        Rv64imOp::Sh => store_op(inst, 2),
        Rv64imOp::Sw => store_op(inst, 4),
        Rv64imOp::Sd => store_op(inst, 8),
        Rv64imOp::Lui => lui(inst),
        Rv64imOp::Auipc => auipc(inst),
        Rv64imOp::Jal => jal(inst),
        Rv64imOp::Jalr => jalr(inst),
        Rv64imOp::Fence => nop(inst),
    }
}

pub fn decode_and_lower_rv64im32(word: u32, paddr: u64) -> Option<StaticRows> {
    match decode_rv64im32(word, paddr) {
        Some(inst) => Some(lower_rv64im32(inst)),
        None => None,
    }
}

pub fn decode_and_lower_matching_riscv_instruction(i: &RiscvInstruction) -> Option<StaticRows> {
    let decoded = decode_rv64im32(i.rvinst, i.rom_address)?;
    if Some(decoded.op) == op_from_mnemonic(i.inst.as_str()) {
        Some(lower_rv64im32(decoded))
    } else {
        None
    }
}

pub fn lower_riscv_instruction(i: &RiscvInstruction) -> Option<StaticRows> {
    let op = op_from_mnemonic(i.inst.as_str())?;
    Some(lower_rv64im32(Rv64imInst {
        paddr: i.rom_address,
        op,
        rd: i.rd,
        rs1: i.rs1,
        rs2: i.rs2,
        imm: i.imm,
    }))
}

pub fn static_row_to_zisk_inst(row: StaticRow) -> ZiskInst {
    let mut i = ZiskInst {
        paddr: row.paddr,
        store_pc: row.store_pc,
        store: row.store,
        store_offset: row.store_offset,
        set_pc: row.set_pc,
        ind_width: row.ind_width,
        a_src: row.a_src,
        a_use_sp_imm1: row.a_use_sp_imm1,
        a_offset_imm0: row.a_offset_imm0,
        b_src: row.b_src,
        b_use_sp_imm1: row.b_use_sp_imm1,
        b_offset_imm0: row.b_offset_imm0,
        jmp_offset1: row.jmp_offset1,
        jmp_offset2: row.jmp_offset2,
        is_external_op: row.is_external_op,
        op: row.op as u8,
        m32: row.m32,
        ..Default::default()
    };
    let op = ZiskOp::try_from_code(i.op).expect("RV64IM static row uses a valid ZisK opcode");
    i.func = op.get_call_function();
    i.op_str = op.name();
    i.op_type = op.op_type().into();
    i.input_size = op.input_size();
    i.is_precompiled = op.input_size() > 0;
    i
}

fn op_from_mnemonic(mnemonic: &str) -> Option<Rv64imOp> {
    match mnemonic {
        "add" => Some(Rv64imOp::Add),
        "sub" => Some(Rv64imOp::Sub),
        "sll" => Some(Rv64imOp::Sll),
        "slt" => Some(Rv64imOp::Slt),
        "sltu" => Some(Rv64imOp::Sltu),
        "xor" => Some(Rv64imOp::Xor),
        "srl" => Some(Rv64imOp::Srl),
        "sra" => Some(Rv64imOp::Sra),
        "or" => Some(Rv64imOp::Or),
        "and" => Some(Rv64imOp::And),
        "addw" => Some(Rv64imOp::Addw),
        "subw" => Some(Rv64imOp::Subw),
        "sllw" => Some(Rv64imOp::Sllw),
        "srlw" => Some(Rv64imOp::Srlw),
        "sraw" => Some(Rv64imOp::Sraw),
        "addi" => Some(Rv64imOp::Addi),
        "slli" => Some(Rv64imOp::Slli),
        "slti" => Some(Rv64imOp::Slti),
        "sltiu" => Some(Rv64imOp::Sltiu),
        "xori" => Some(Rv64imOp::Xori),
        "srli" => Some(Rv64imOp::Srli),
        "srai" => Some(Rv64imOp::Srai),
        "ori" => Some(Rv64imOp::Ori),
        "andi" => Some(Rv64imOp::Andi),
        "addiw" => Some(Rv64imOp::Addiw),
        "slliw" => Some(Rv64imOp::Slliw),
        "srliw" => Some(Rv64imOp::Srliw),
        "sraiw" => Some(Rv64imOp::Sraiw),
        "beq" => Some(Rv64imOp::Beq),
        "bne" => Some(Rv64imOp::Bne),
        "blt" => Some(Rv64imOp::Blt),
        "bge" => Some(Rv64imOp::Bge),
        "bltu" => Some(Rv64imOp::Bltu),
        "bgeu" => Some(Rv64imOp::Bgeu),
        "lb" => Some(Rv64imOp::Lb),
        "lbu" => Some(Rv64imOp::Lbu),
        "lh" => Some(Rv64imOp::Lh),
        "lhu" => Some(Rv64imOp::Lhu),
        "lw" => Some(Rv64imOp::Lw),
        "lwu" => Some(Rv64imOp::Lwu),
        "ld" => Some(Rv64imOp::Ld),
        "sb" => Some(Rv64imOp::Sb),
        "sh" => Some(Rv64imOp::Sh),
        "sw" => Some(Rv64imOp::Sw),
        "sd" => Some(Rv64imOp::Sd),
        "lui" => Some(Rv64imOp::Lui),
        "auipc" => Some(Rv64imOp::Auipc),
        "jal" => Some(Rv64imOp::Jal),
        "jalr" => Some(Rv64imOp::Jalr),
        "fence" => Some(Rv64imOp::Fence),
        "mul" => Some(Rv64imOp::Mul),
        "mulh" => Some(Rv64imOp::Mulh),
        "mulhsu" => Some(Rv64imOp::Mulhsu),
        "mulhu" => Some(Rv64imOp::Mulhu),
        "mulw" => Some(Rv64imOp::Mulw),
        "div" => Some(Rv64imOp::Div),
        "divu" => Some(Rv64imOp::Divu),
        "divw" => Some(Rv64imOp::Divw),
        "divuw" => Some(Rv64imOp::Divuw),
        "rem" => Some(Rv64imOp::Rem),
        "remu" => Some(Rv64imOp::Remu),
        "remw" => Some(Rv64imOp::Remw),
        "remuw" => Some(Rv64imOp::Remuw),
        _ => None,
    }
}

fn decode_imm(word: u32, op: Rv64imOp) -> i32 {
    match op {
        Rv64imOp::Lb
        | Rv64imOp::Lbu
        | Rv64imOp::Lh
        | Rv64imOp::Lhu
        | Rv64imOp::Lw
        | Rv64imOp::Lwu
        | Rv64imOp::Ld
        | Rv64imOp::Addi
        | Rv64imOp::Slti
        | Rv64imOp::Sltiu
        | Rv64imOp::Xori
        | Rv64imOp::Ori
        | Rv64imOp::Andi
        | Rv64imOp::Addiw
        | Rv64imOp::Jalr => signext((word >> 20) & 0xfff, 12),
        Rv64imOp::Slli | Rv64imOp::Srli | Rv64imOp::Srai => ((word >> 20) & 0x3f) as i32,
        Rv64imOp::Slliw | Rv64imOp::Srliw | Rv64imOp::Sraiw => ((word >> 20) & 0x1f) as i32,
        Rv64imOp::Sb | Rv64imOp::Sh | Rv64imOp::Sw | Rv64imOp::Sd => {
            let imm4_0 = (word >> 7) & 0x1f;
            let imm11_5 = (word >> 25) & 0x7f;
            signext((imm11_5 << 5) | imm4_0, 12)
        }
        Rv64imOp::Beq
        | Rv64imOp::Bne
        | Rv64imOp::Blt
        | Rv64imOp::Bge
        | Rv64imOp::Bltu
        | Rv64imOp::Bgeu => {
            let imm11 = (word >> 7) & 0x1;
            let imm4_1 = (word >> 8) & 0xf;
            let imm10_5 = (word >> 25) & 0x3f;
            let imm12 = (word >> 31) & 0x1;
            signext((imm12 << 12) | (imm11 << 11) | (imm10_5 << 5) | (imm4_1 << 1), 13)
        }
        Rv64imOp::Lui | Rv64imOp::Auipc => (word & 0xffff_f000) as i32,
        Rv64imOp::Jal => {
            let imm20 = (word >> 31) & 0x1;
            let imm10_1 = (word >> 21) & 0x3ff;
            let imm11 = (word >> 20) & 0x1;
            let imm19_12 = (word >> 12) & 0xff;
            signext((imm20 << 20) | (imm19_12 << 12) | (imm11 << 11) | (imm10_1 << 1), 21)
        }
        _ => 0,
    }
}

fn decoded_registers(op: Rv64imOp, rd: u32, rs1: u32, rs2: u32) -> (u32, u32, u32) {
    match op {
        Rv64imOp::Sb
        | Rv64imOp::Sh
        | Rv64imOp::Sw
        | Rv64imOp::Sd
        | Rv64imOp::Beq
        | Rv64imOp::Bne
        | Rv64imOp::Blt
        | Rv64imOp::Bge
        | Rv64imOp::Bltu
        | Rv64imOp::Bgeu => (0, rs1, rs2),
        Rv64imOp::Lui | Rv64imOp::Auipc | Rv64imOp::Jal => (rd, 0, 0),
        Rv64imOp::Lb
        | Rv64imOp::Lbu
        | Rv64imOp::Lh
        | Rv64imOp::Lhu
        | Rv64imOp::Lw
        | Rv64imOp::Lwu
        | Rv64imOp::Ld
        | Rv64imOp::Addi
        | Rv64imOp::Slli
        | Rv64imOp::Slti
        | Rv64imOp::Sltiu
        | Rv64imOp::Xori
        | Rv64imOp::Srli
        | Rv64imOp::Srai
        | Rv64imOp::Ori
        | Rv64imOp::Andi
        | Rv64imOp::Addiw
        | Rv64imOp::Slliw
        | Rv64imOp::Srliw
        | Rv64imOp::Sraiw
        | Rv64imOp::Jalr => (rd, rs1, 0),
        Rv64imOp::Fence => (0, 0, 0),
        _ => (rd, rs1, rs2),
    }
}

fn signext(v: u32, size: u32) -> i32 {
    let sign_bit = 1u32 << (size - 1);
    let max_value = 1u32 << size;
    if (sign_bit & v) != 0 {
        v as i32 - max_value as i32
    } else {
        v as i32
    }
}

fn row(paddr: u64, op: u64) -> StaticRow {
    StaticRow {
        paddr,
        op,
        is_external_op: op != OP_FLAG && op != OP_COPYB,
        m32: matches!(
            op,
            OP_ADD_W
                | OP_SUB_W
                | OP_SLL_W
                | OP_SRL_W
                | OP_SRA_W
                | OP_SIGNEXT_W
                | OP_MUL_W
                | OP_DIVU_W
                | OP_REMU_W
                | OP_DIV_W
                | OP_REM_W
        ),
        ..Default::default()
    }
}

fn source_reg(r: u32) -> (u64, u64, u64) {
    if r == 0 {
        (SRC_IMM, 0, 0)
    } else {
        (SRC_REG, 0, r as u64)
    }
}

fn source_imm(v: u64) -> (u64, u64, u64) {
    (SRC_IMM, (v >> 32) & 0xffff_ffff, v & 0xffff_ffff)
}

fn source_ind(offset: i32) -> (u64, u64, u64) {
    (SRC_IND, 0, offset as u64)
}

fn store_reg(r: u32, store_pc: bool) -> (u64, i64, bool) {
    if r == 0 {
        (STORE_NONE, 0, false)
    } else {
        (STORE_REG, r as i64, store_pc)
    }
}

fn set_a(mut r: StaticRow, source: (u64, u64, u64)) -> StaticRow {
    r.a_src = source.0;
    r.a_use_sp_imm1 = source.1;
    r.a_offset_imm0 = source.2;
    r
}

fn set_b(mut r: StaticRow, source: (u64, u64, u64)) -> StaticRow {
    r.b_src = source.0;
    r.b_use_sp_imm1 = source.1;
    r.b_offset_imm0 = source.2;
    r
}

fn set_store(mut r: StaticRow, store: (u64, i64, bool)) -> StaticRow {
    r.store = store.0;
    r.store_offset = store.1;
    r.store_pc = store.2;
    r
}

fn set_j(mut r: StaticRow, j1: i64, j2: i64) -> StaticRow {
    r.jmp_offset1 = j1;
    r.jmp_offset2 = j2;
    r
}

fn register_op(inst: Rv64imInst, op: u64) -> StaticRows {
    let r = set_j(
        set_store(
            set_b(set_a(row(inst.paddr, op), source_reg(inst.rs1)), source_reg(inst.rs2)),
            store_reg(inst.rd, false),
        ),
        4,
        4,
    );
    StaticRows::one(r)
}

fn immediate_op(inst: Rv64imInst, op: u64) -> StaticRows {
    let r = set_j(
        set_store(
            set_b(set_a(row(inst.paddr, op), source_reg(inst.rs1)), source_imm(inst.imm as u64)),
            store_reg(inst.rd, false),
        ),
        4,
        4,
    );
    StaticRows::one(r)
}

fn immediate_op_or_x0_copyb(inst: Rv64imInst, op: u64) -> StaticRows {
    if inst.rs1 == 0 {
        let r = set_j(
            set_store(
                set_b(
                    set_a(row(inst.paddr, OP_COPYB), source_reg(inst.rs1)),
                    source_imm(inst.imm as u64),
                ),
                store_reg(inst.rd, false),
            ),
            4,
            4,
        );
        StaticRows::one(r)
    } else {
        immediate_op(inst, op)
    }
}

fn copyb_from_reg(inst: Rv64imInst, rs: u32) -> StaticRows {
    let r = set_j(
        set_store(
            set_b(set_a(row(inst.paddr, OP_COPYB), source_imm(0)), source_reg(rs)),
            store_reg(inst.rd, false),
        ),
        4,
        4,
    );
    StaticRows::one(r)
}

fn branch_op(inst: Rv64imInst, op: u64, neg: bool) -> StaticRows {
    let (j1, j2) = if neg { (4, inst.imm as i64) } else { (inst.imm as i64, 4) };
    let r = set_j(
        set_b(set_a(row(inst.paddr, op), source_reg(inst.rs1)), source_reg(inst.rs2)),
        j1,
        j2,
    );
    StaticRows::one(r)
}

fn load_op(inst: Rv64imInst, op: u64, width: u64) -> StaticRows {
    let mut r = set_j(
        set_store(
            set_b(set_a(row(inst.paddr, op), source_reg(inst.rs1)), source_ind(inst.imm)),
            store_reg(inst.rd, false),
        ),
        4,
        4,
    );
    r.ind_width = width;
    StaticRows::one(r)
}

fn store_op(inst: Rv64imInst, width: u64) -> StaticRows {
    let mut r = set_j(
        set_store(
            set_b(set_a(row(inst.paddr, OP_COPYB), source_reg(inst.rs1)), source_reg(inst.rs2)),
            (STORE_IND, inst.imm as i64, false),
        ),
        4,
        4,
    );
    r.ind_width = width;
    StaticRows::one(r)
}

fn lui(inst: Rv64imInst) -> StaticRows {
    let r = set_j(
        set_store(
            set_b(set_a(row(inst.paddr, OP_COPYB), source_imm(0)), source_imm(inst.imm as u64)),
            store_reg(inst.rd, false),
        ),
        4,
        4,
    );
    StaticRows::one(r)
}

fn auipc(inst: Rv64imInst) -> StaticRows {
    let r = set_j(
        set_store(
            set_b(set_a(row(inst.paddr, OP_FLAG), source_imm(0)), source_imm(0)),
            store_reg(inst.rd, true),
        ),
        4,
        inst.imm as i64,
    );
    StaticRows::one(r)
}

fn jal(inst: Rv64imInst) -> StaticRows {
    let r = set_j(
        set_store(
            set_b(set_a(row(inst.paddr, OP_FLAG), source_imm(0)), source_imm(0)),
            store_reg(inst.rd, true),
        ),
        inst.imm as i64,
        4,
    );
    StaticRows::one(r)
}

fn jalr(inst: Rv64imInst) -> StaticRows {
    const JALR_MASK: u64 = 0xffff_ffff_ffff_fffe;
    if inst.imm % 4 == 0 {
        let mut r = set_j(
            set_store(
                set_b(set_a(row(inst.paddr, OP_AND), source_imm(JALR_MASK)), source_reg(inst.rs1)),
                store_reg(inst.rd, true),
            ),
            inst.imm as i64,
            4,
        );
        r.set_pc = true;
        StaticRows::one(r)
    } else {
        let first = set_j(
            set_b(
                set_a(row(inst.paddr, OP_ADD), source_imm(inst.imm as u64)),
                source_reg(inst.rs1),
            ),
            1,
            1,
        );
        let mut second = set_j(
            set_store(
                set_b(set_a(row(inst.paddr + 1, OP_AND), source_imm(JALR_MASK)), (SRC_C, 0, 0)),
                store_reg(inst.rd, true),
            ),
            0,
            3,
        );
        second.set_pc = true;
        StaticRows::two(first, second)
    }
}

fn hint(inst: Rv64imInst) -> StaticRows {
    let r = set_j(
        set_b(set_a(row(inst.paddr, OP_FLAG), source_reg(inst.rs1)), source_imm(inst.imm as u64)),
        4,
        4,
    );
    StaticRows::one(r)
}

fn nop(inst: Rv64imInst) -> StaticRows {
    let r = set_j(set_b(set_a(row(inst.paddr, OP_FLAG), source_imm(0)), source_imm(0)), 4, 4);
    StaticRows::one(r)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn enc_i(imm: i32, rs1: u32, funct3: u32, rd: u32, opcode: u32) -> u32 {
        ((imm as u32 & 0xfff) << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | opcode
    }

    fn enc_s(imm: i32, rs2: u32, rs1: u32, funct3: u32) -> u32 {
        let imm = imm as u32 & 0xfff;
        ((imm >> 5) << 25) | (rs2 << 20) | (rs1 << 15) | (funct3 << 12) | ((imm & 0x1f) << 7) | 0x23
    }

    fn enc_b(imm: i32, rs2: u32, rs1: u32, funct3: u32) -> u32 {
        let imm = imm as u32 & 0x1fff;
        (((imm >> 12) & 0x1) << 31)
            | (((imm >> 5) & 0x3f) << 25)
            | (rs2 << 20)
            | (rs1 << 15)
            | (funct3 << 12)
            | (((imm >> 1) & 0xf) << 8)
            | (((imm >> 11) & 0x1) << 7)
            | 0x63
    }

    fn enc_r(funct7: u32, rs2: u32, rs1: u32, funct3: u32, rd: u32, opcode: u32) -> u32 {
        (funct7 << 25) | (rs2 << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | opcode
    }

    fn enc_u(imm: i32, rd: u32, opcode: u32) -> u32 {
        (imm as u32 & 0xffff_f000) | (rd << 7) | opcode
    }

    fn enc_j(imm: i32, rd: u32) -> u32 {
        let imm = imm as u32 & 0x1f_ffff;
        (((imm >> 20) & 0x1) << 31)
            | (((imm >> 1) & 0x3ff) << 21)
            | (((imm >> 11) & 0x1) << 20)
            | (((imm >> 12) & 0xff) << 12)
            | (rd << 7)
            | 0x6f
    }

    #[test]
    fn decodes_and_lowers_add() {
        let inst = decode_rv64im32(enc_r(0, 3, 2, 0, 1, 0x33), 0x1000).unwrap();
        assert_eq!(inst.op, Rv64imOp::Add);
        let rows = lower_rv64im32(inst);
        assert_eq!(rows.len, 1);
        assert_eq!(rows.rows[0].op, OP_ADD);
        assert_eq!(rows.rows[0].a_src, SRC_REG);
        assert_eq!(rows.rows[0].a_offset_imm0, 2);
        assert_eq!(rows.rows[0].b_offset_imm0, 3);
        assert_eq!(rows.rows[0].store, STORE_REG);
        assert_eq!(rows.rows[0].store_offset, 1);
    }

    #[test]
    fn decodes_signed_addi_immediate_chunks() {
        let inst = decode_rv64im32(enc_i(-2048, 1, 0, 2, 0x13), 0x1000).unwrap();
        assert_eq!(inst.imm, -2048);
        let rows = lower_rv64im32(inst);
        assert_eq!(rows.rows[0].op, OP_ADD);
        assert_eq!(rows.rows[0].b_use_sp_imm1, 0xffff_ffff);
        assert_eq!(rows.rows[0].b_offset_imm0, 0xffff_f800);
    }

    #[test]
    fn decodes_jalr_unaligned_as_two_rows() {
        let inst = decode_rv64im32(enc_i(3, 5, 0, 1, 0x67), 0x1000).unwrap();
        let rows = lower_rv64im32(inst);
        assert_eq!(rows.len, 2);
        assert_eq!(rows.rows[0].op, OP_ADD);
        assert_eq!(rows.rows[1].op, OP_AND);
        assert_eq!(rows.rows[1].paddr, 0x1001);
        assert!(rows.rows[1].set_pc);
        assert!(rows.rows[1].store_pc);
    }

    #[test]
    fn decodes_representative_rv64im_formats() {
        let cases = [
            (enc_r(32, 3, 2, 0, 1, 0x33), Rv64imOp::Sub, 1, 2, 3, 0),
            (enc_i(-7, 2, 2, 1, 0x03), Rv64imOp::Lw, 1, 2, 0, -7),
            (enc_s(-16, 3, 2, 3), Rv64imOp::Sd, 0, 2, 3, -16),
            (enc_b(-4, 3, 2, 1), Rv64imOp::Bne, 0, 2, 3, -4),
            (enc_u(0x12345000, 7, 0x37), Rv64imOp::Lui, 7, 0, 0, 0x12345000),
            (enc_j(-8, 1), Rv64imOp::Jal, 1, 0, 0, -8),
        ];
        for (word, op, rd, rs1, rs2, imm) in cases {
            let inst = decode_rv64im32(word, 0x2000).unwrap();
            assert_eq!(inst.op, op);
            assert_eq!(inst.rd, rd);
            assert_eq!(inst.rs1, rs1);
            assert_eq!(inst.rs2, rs2);
            assert_eq!(inst.imm, imm);
        }
    }

    #[test]
    fn raw_decode_helper_requires_legacy_mnemonic_match() {
        let word = enc_r(0, 3, 2, 0, 1, 0x33);
        let matching = RiscvInstruction {
            rom_address: 0x3000,
            rvinst: word,
            inst: "add".to_string(),
            ..Default::default()
        };
        assert!(decode_and_lower_matching_riscv_instruction(&matching).is_some());

        let mismatching = RiscvInstruction { inst: "sub".to_string(), ..matching };
        assert!(decode_and_lower_matching_riscv_instruction(&mismatching).is_none());
    }
}
