use riscv::RiscvInstruction;
#[cfg(not(feature = "aeneas_extract"))]
use zisk_definitions::{SYSCALL_DMA_MEMCMP_ID, SYSCALL_DMA_MEMCPY_ID};

use crate::{riscv2zisk_context::Riscv2ZiskContext, zisk_ops::ZiskOp};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rv64imLoweringInput {
    pub rom_address: u64,
    pub rd: u32,
    pub rs1: u32,
    pub rs2: u32,
    pub imm: i32,
}

impl Rv64imLoweringInput {
    pub fn from_riscv(i: &RiscvInstruction) -> Self {
        Self { rom_address: i.rom_address, rd: i.rd, rs1: i.rs1, rs2: i.rs2, imm: i.imm }
    }

    pub fn new(rom_address: u64, rd: u32, rs1: u32, rs2: u32, imm: i32) -> Self {
        Self { rom_address, rd, rs1, rs2, imm }
    }
}

#[cfg(not(feature = "aeneas_extract"))]
const CSR_DMA_MEMCPY_ADDR: u32 = SYSCALL_DMA_MEMCPY_ID as u32;
#[cfg(feature = "aeneas_extract")]
const CSR_DMA_MEMCPY_ADDR: u32 = 0x813;
#[cfg(not(feature = "aeneas_extract"))]
const CSR_DMA_MEMCMP_ADDR: u32 = SYSCALL_DMA_MEMCMP_ID as u32;
#[cfg(feature = "aeneas_extract")]
const CSR_DMA_MEMCMP_ADDR: u32 = 0x814;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rv64imSingleRowOpcode {
    Lui,
    Auipc,
    Jal,
    Jalr,
    Fence,
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
}

impl Rv64imSingleRowOpcode {
    pub fn from_inst_name(inst: &str) -> Option<Self> {
        match inst {
            "lui" => Some(Self::Lui),
            "auipc" => Some(Self::Auipc),
            "jal" => Some(Self::Jal),
            "jalr" => Some(Self::Jalr),
            "fence" | "fence.i" => Some(Self::Fence),
            "add" => Some(Self::Add),
            "sub" => Some(Self::Sub),
            "sll" => Some(Self::Sll),
            "slt" => Some(Self::Slt),
            "sltu" => Some(Self::Sltu),
            "xor" => Some(Self::Xor),
            "srl" => Some(Self::Srl),
            "sra" => Some(Self::Sra),
            "or" => Some(Self::Or),
            "and" => Some(Self::And),
            "addw" => Some(Self::Addw),
            "subw" => Some(Self::Subw),
            "sllw" => Some(Self::Sllw),
            "srlw" => Some(Self::Srlw),
            "sraw" => Some(Self::Sraw),
            "mul" => Some(Self::Mul),
            "mulh" => Some(Self::Mulh),
            "mulhsu" => Some(Self::Mulhsu),
            "mulhu" => Some(Self::Mulhu),
            "mulw" => Some(Self::Mulw),
            "div" => Some(Self::Div),
            "divu" => Some(Self::Divu),
            "divw" => Some(Self::Divw),
            "divuw" => Some(Self::Divuw),
            "rem" => Some(Self::Rem),
            "remu" => Some(Self::Remu),
            "remw" => Some(Self::Remw),
            "remuw" => Some(Self::Remuw),
            "addi" => Some(Self::Addi),
            "slli" => Some(Self::Slli),
            "slti" => Some(Self::Slti),
            "sltiu" => Some(Self::Sltiu),
            "xori" => Some(Self::Xori),
            "srli" => Some(Self::Srli),
            "srai" => Some(Self::Srai),
            "ori" => Some(Self::Ori),
            "andi" => Some(Self::Andi),
            "addiw" => Some(Self::Addiw),
            "slliw" => Some(Self::Slliw),
            "srliw" => Some(Self::Srliw),
            "sraiw" => Some(Self::Sraiw),
            "beq" => Some(Self::Beq),
            "bne" => Some(Self::Bne),
            "blt" => Some(Self::Blt),
            "bge" => Some(Self::Bge),
            "bltu" => Some(Self::Bltu),
            "bgeu" => Some(Self::Bgeu),
            "lb" => Some(Self::Lb),
            "lbu" => Some(Self::Lbu),
            "lh" => Some(Self::Lh),
            "lhu" => Some(Self::Lhu),
            "lw" => Some(Self::Lw),
            "lwu" => Some(Self::Lwu),
            "ld" => Some(Self::Ld),
            "sb" => Some(Self::Sb),
            "sh" => Some(Self::Sh),
            "sw" => Some(Self::Sw),
            "sd" => Some(Self::Sd),
            _ => None,
        }
    }
}

impl Riscv2ZiskContext<'_> {
    pub fn lower_rv64im_single_row(
        &mut self,
        riscv_instruction: &RiscvInstruction,
        opcode: Rv64imSingleRowOpcode,
        next_instructions: &[RiscvInstruction],
    ) {
        let input = Rv64imLoweringInput::from_riscv(riscv_instruction);
        self.lower_rv64im_single_row_input(&input, opcode, next_instructions.len() != 0);
    }

    pub fn lower_rv64im_single_row_input(
        &mut self,
        riscv_instruction: &Rv64imLoweringInput,
        opcode: Rv64imSingleRowOpcode,
        has_next_instruction: bool,
    ) {
        match opcode {
            Rv64imSingleRowOpcode::Lui => self.lui(riscv_instruction, 4),
            Rv64imSingleRowOpcode::Auipc => self.auipc(riscv_instruction),
            Rv64imSingleRowOpcode::Jal => self.jal(riscv_instruction, 4),
            Rv64imSingleRowOpcode::Jalr => self.jalr(riscv_instruction, 4),
            Rv64imSingleRowOpcode::Fence => self.nop(riscv_instruction, 4),
            Rv64imSingleRowOpcode::Add => {
                let input_precompile = match self.input_precompile {
                    Some(precompile) => precompile,
                    None => 0,
                };
                if riscv_instruction.rd == 0 && input_precompile == CSR_DMA_MEMCPY_ADDR {
                    let input_precompile_reg = self.input_precompile_reg.unwrap();
                    self.create_precompiled_op_typed(
                        riscv_instruction,
                        ZiskOp::DmaMemCpy,
                        riscv_instruction.rs1,
                        input_precompile_reg,
                        4,
                    );
                } else if input_precompile == CSR_DMA_MEMCMP_ADDR {
                    let input_precompile_reg = self.input_precompile_reg.unwrap();
                    self.create_precompiled_op_typed(
                        riscv_instruction,
                        ZiskOp::DmaMemCmp,
                        riscv_instruction.rs1,
                        input_precompile_reg,
                        4,
                    );
                } else if riscv_instruction.rs1 == 0 {
                    if has_next_instruction {
                        self.copyb(riscv_instruction, 4, 2);
                    } else {
                        self.copyb(riscv_instruction, 4, 2);
                    }
                } else if riscv_instruction.rs2 == 0 {
                    self.copyb(riscv_instruction, 4, 1);
                } else {
                    self.create_register_op_typed(riscv_instruction, ZiskOp::Add, 4);
                }
            }
            Rv64imSingleRowOpcode::Sub => {
                self.create_register_op_typed(riscv_instruction, ZiskOp::Sub, 4)
            }
            Rv64imSingleRowOpcode::Sll => {
                self.create_register_op_typed(riscv_instruction, ZiskOp::Sll, 4)
            }
            Rv64imSingleRowOpcode::Slt => {
                self.create_register_op_typed(riscv_instruction, ZiskOp::Lt, 4)
            }
            Rv64imSingleRowOpcode::Sltu => {
                self.create_register_op_typed(riscv_instruction, ZiskOp::Ltu, 4)
            }
            Rv64imSingleRowOpcode::Xor => {
                self.create_register_op_typed(riscv_instruction, ZiskOp::Xor, 4)
            }
            Rv64imSingleRowOpcode::Srl => {
                self.create_register_op_typed(riscv_instruction, ZiskOp::Srl, 4)
            }
            Rv64imSingleRowOpcode::Sra => {
                self.create_register_op_typed(riscv_instruction, ZiskOp::Sra, 4)
            }
            Rv64imSingleRowOpcode::Or => {
                if riscv_instruction.rs1 == 0 {
                    self.copyb(riscv_instruction, 4, 2);
                } else if riscv_instruction.rs2 == 0 {
                    self.copyb(riscv_instruction, 4, 1);
                } else {
                    self.create_register_op_typed(riscv_instruction, ZiskOp::Or, 4);
                }
            }
            Rv64imSingleRowOpcode::And => {
                self.create_register_op_typed(riscv_instruction, ZiskOp::And, 4)
            }
            Rv64imSingleRowOpcode::Addw => {
                self.create_register_op_typed(riscv_instruction, ZiskOp::AddW, 4)
            }
            Rv64imSingleRowOpcode::Subw => {
                self.create_register_op_typed(riscv_instruction, ZiskOp::SubW, 4)
            }
            Rv64imSingleRowOpcode::Sllw => {
                self.create_register_op_typed(riscv_instruction, ZiskOp::SllW, 4)
            }
            Rv64imSingleRowOpcode::Srlw => {
                self.create_register_op_typed(riscv_instruction, ZiskOp::SrlW, 4)
            }
            Rv64imSingleRowOpcode::Sraw => {
                self.create_register_op_typed(riscv_instruction, ZiskOp::SraW, 4)
            }
            Rv64imSingleRowOpcode::Mul => {
                self.create_register_op_typed(riscv_instruction, ZiskOp::Mul, 4)
            }
            Rv64imSingleRowOpcode::Mulh => {
                self.create_register_op_typed(riscv_instruction, ZiskOp::Mulh, 4)
            }
            Rv64imSingleRowOpcode::Mulhsu => {
                self.create_register_op_typed(riscv_instruction, ZiskOp::Mulsuh, 4)
            }
            Rv64imSingleRowOpcode::Mulhu => {
                self.create_register_op_typed(riscv_instruction, ZiskOp::Muluh, 4)
            }
            Rv64imSingleRowOpcode::Mulw => {
                self.create_register_op_typed(riscv_instruction, ZiskOp::MulW, 4)
            }
            Rv64imSingleRowOpcode::Div => {
                self.create_register_op_typed(riscv_instruction, ZiskOp::Div, 4)
            }
            Rv64imSingleRowOpcode::Divu => {
                self.create_register_op_typed(riscv_instruction, ZiskOp::Divu, 4)
            }
            Rv64imSingleRowOpcode::Divw => {
                self.create_register_op_typed(riscv_instruction, ZiskOp::DivW, 4)
            }
            Rv64imSingleRowOpcode::Divuw => {
                self.create_register_op_typed(riscv_instruction, ZiskOp::DivuW, 4)
            }
            Rv64imSingleRowOpcode::Rem => {
                self.create_register_op_typed(riscv_instruction, ZiskOp::Rem, 4)
            }
            Rv64imSingleRowOpcode::Remu => {
                self.create_register_op_typed(riscv_instruction, ZiskOp::Remu, 4)
            }
            Rv64imSingleRowOpcode::Remw => {
                self.create_register_op_typed(riscv_instruction, ZiskOp::RemW, 4)
            }
            Rv64imSingleRowOpcode::Remuw => {
                self.create_register_op_typed(riscv_instruction, ZiskOp::RemuW, 4)
            }
            Rv64imSingleRowOpcode::Addi => {
                if riscv_instruction.rd == 0 {
                    if riscv_instruction.rs1 == 0 && riscv_instruction.rs2 == 0 {
                        self.nop(riscv_instruction, 4);
                    } else {
                        self.hint(riscv_instruction, 4);
                    }
                } else if riscv_instruction.imm == 0 && riscv_instruction.rs1 != 0 {
                    self.copyb(riscv_instruction, 4, 1);
                } else {
                    self.immediate_op_or_x0_copyb_typed(riscv_instruction, ZiskOp::Add, 4);
                }
            }
            Rv64imSingleRowOpcode::Slli => {
                self.immediate_op_typed(riscv_instruction, ZiskOp::Sll, 4)
            }
            Rv64imSingleRowOpcode::Slti => {
                self.immediate_op_typed(riscv_instruction, ZiskOp::Lt, 4)
            }
            Rv64imSingleRowOpcode::Sltiu => {
                self.immediate_op_typed(riscv_instruction, ZiskOp::Ltu, 4)
            }
            Rv64imSingleRowOpcode::Xori => {
                self.immediate_op_or_x0_copyb_typed(riscv_instruction, ZiskOp::Xor, 4)
            }
            Rv64imSingleRowOpcode::Srli => {
                self.immediate_op_typed(riscv_instruction, ZiskOp::Srl, 4)
            }
            Rv64imSingleRowOpcode::Srai => {
                self.immediate_op_typed(riscv_instruction, ZiskOp::Sra, 4)
            }
            Rv64imSingleRowOpcode::Ori => {
                self.immediate_op_or_x0_copyb_typed(riscv_instruction, ZiskOp::Or, 4)
            }
            Rv64imSingleRowOpcode::Andi => {
                self.immediate_op_typed(riscv_instruction, ZiskOp::And, 4)
            }
            Rv64imSingleRowOpcode::Addiw => {
                if riscv_instruction.rd == 0
                    && riscv_instruction.rs1 == 0
                    && riscv_instruction.imm == 0
                {
                    self.nop(riscv_instruction, 4);
                } else {
                    self.immediate_op_typed(riscv_instruction, ZiskOp::AddW, 4);
                }
            }
            Rv64imSingleRowOpcode::Slliw => {
                self.immediate_op_typed(riscv_instruction, ZiskOp::SllW, 4)
            }
            Rv64imSingleRowOpcode::Srliw => {
                self.immediate_op_typed(riscv_instruction, ZiskOp::SrlW, 4)
            }
            Rv64imSingleRowOpcode::Sraiw => {
                self.immediate_op_typed(riscv_instruction, ZiskOp::SraW, 4)
            }
            Rv64imSingleRowOpcode::Beq => {
                self.create_branch_op_typed(riscv_instruction, ZiskOp::Eq, false, 4)
            }
            Rv64imSingleRowOpcode::Bne => {
                self.create_branch_op_typed(riscv_instruction, ZiskOp::Eq, true, 4)
            }
            Rv64imSingleRowOpcode::Blt => {
                self.create_branch_op_typed(riscv_instruction, ZiskOp::Lt, false, 4)
            }
            Rv64imSingleRowOpcode::Bge => {
                self.create_branch_op_typed(riscv_instruction, ZiskOp::Lt, true, 4)
            }
            Rv64imSingleRowOpcode::Bltu => {
                self.create_branch_op_typed(riscv_instruction, ZiskOp::Ltu, false, 4)
            }
            Rv64imSingleRowOpcode::Bgeu => {
                self.create_branch_op_typed(riscv_instruction, ZiskOp::Ltu, true, 4)
            }
            Rv64imSingleRowOpcode::Lb => {
                self.load_op_typed(riscv_instruction, ZiskOp::SignExtendB, 1, 4)
            }
            Rv64imSingleRowOpcode::Lbu => {
                self.load_op_typed(riscv_instruction, ZiskOp::CopyB, 1, 4)
            }
            Rv64imSingleRowOpcode::Lh => {
                self.load_op_typed(riscv_instruction, ZiskOp::SignExtendH, 2, 4)
            }
            Rv64imSingleRowOpcode::Lhu => {
                self.load_op_typed(riscv_instruction, ZiskOp::CopyB, 2, 4)
            }
            Rv64imSingleRowOpcode::Lw => {
                self.load_op_typed(riscv_instruction, ZiskOp::SignExtendW, 4, 4)
            }
            Rv64imSingleRowOpcode::Lwu => {
                self.load_op_typed(riscv_instruction, ZiskOp::CopyB, 4, 4)
            }
            Rv64imSingleRowOpcode::Ld => self.load_op_typed(riscv_instruction, ZiskOp::CopyB, 8, 4),
            Rv64imSingleRowOpcode::Sb => {
                self.store_op_typed(riscv_instruction, ZiskOp::CopyB, 1, 4)
            }
            Rv64imSingleRowOpcode::Sh => {
                self.store_op_typed(riscv_instruction, ZiskOp::CopyB, 2, 4)
            }
            Rv64imSingleRowOpcode::Sw => {
                self.store_op_typed(riscv_instruction, ZiskOp::CopyB, 4, 4)
            }
            Rv64imSingleRowOpcode::Sd => {
                self.store_op_typed(riscv_instruction, ZiskOp::CopyB, 8, 4)
            }
        }
    }
}
