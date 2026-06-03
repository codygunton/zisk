#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RiscvFormat {
    I,
    R,
    S,
    B,
    U,
    J,
    F,
    Invalid,
    Unsupported,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RiscvOpcode {
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
    Reserved,
    Unsupported,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecodedRv64im {
    pub opcode: RiscvOpcode,
    pub format: RiscvFormat,
    pub funct3: u32,
    pub funct7: u32,
    pub rd: u32,
    pub rs1: u32,
    pub rs2: u32,
    pub imm: i32,
    pub pred: u32,
    pub succ: u32,
}

impl DecodedRv64im {
    fn new(opcode: RiscvOpcode, format: RiscvFormat) -> Self {
        Self {
            opcode,
            format,
            funct3: 0,
            funct7: 0,
            rd: 0,
            rs1: 0,
            rs2: 0,
            imm: 0,
            pred: 0,
            succ: 0,
        }
    }

    fn reserved() -> Self {
        Self::new(RiscvOpcode::Reserved, RiscvFormat::Invalid)
    }

    fn unsupported() -> Self {
        Self::new(RiscvOpcode::Unsupported, RiscvFormat::Unsupported)
    }

    pub fn is_supported_rv64im(self) -> bool {
        match self.opcode {
            RiscvOpcode::Reserved | RiscvOpcode::Unsupported => false,
            _ => true,
        }
    }
}

fn signext(v: u32, size: u32) -> i32 {
    let sign_bit: u32 = 1u32 << (size - 1);
    let max_value: u32 = 1u32 << size;
    if (sign_bit & v) != 0 {
        v as i32 - max_value as i32
    } else {
        v as i32
    }
}

fn decode_i(inst: u32, opcode: RiscvOpcode, shift_level: bool) -> DecodedRv64im {
    let mut d = DecodedRv64im::new(opcode, RiscvFormat::I);
    d.funct3 = (inst & 0x7000) >> 12;
    d.rd = (inst & 0xF80) >> 7;
    d.rs1 = (inst & 0xF8000) >> 15;
    d.imm = signext((inst & 0xFFF00000) >> 20, 12);
    if shift_level {
        d.imm &= 0x3F;
        d.funct7 = (inst & 0xFC000000) >> 26;
    }
    d
}

fn decode_r(inst: u32, opcode: RiscvOpcode) -> DecodedRv64im {
    let mut d = DecodedRv64im::new(opcode, RiscvFormat::R);
    d.funct3 = (inst & 0x7000) >> 12;
    d.rd = (inst & 0xF80) >> 7;
    d.rs1 = (inst & 0xF8000) >> 15;
    d.rs2 = (inst & 0x1F00000) >> 20;
    d.funct7 = (inst & 0xFE000000) >> 25;
    d
}

fn decode_s(inst: u32, opcode: RiscvOpcode) -> DecodedRv64im {
    let mut d = DecodedRv64im::new(opcode, RiscvFormat::S);
    d.funct3 = (inst & 0x7000) >> 12;
    let imm4_0 = (inst & 0xF80) >> 7;
    d.rs1 = (inst & 0xF8000) >> 15;
    d.rs2 = (inst & 0x1F00000) >> 20;
    let imm11_5 = (inst & 0xFE000000) >> 25;
    d.imm = signext((imm11_5 << 5) | imm4_0, 12);
    d
}

fn decode_b(inst: u32, opcode: RiscvOpcode) -> DecodedRv64im {
    let mut d = DecodedRv64im::new(opcode, RiscvFormat::B);
    d.funct3 = (inst & 0x7000) >> 12;
    let imm11 = (inst & 0x080) >> 7;
    let imm4_1 = (inst & 0xF00) >> 8;
    d.rs1 = (inst & 0xF8000) >> 15;
    d.rs2 = (inst & 0x1F00000) >> 20;
    let imm10_5 = (inst & 0x7E000000) >> 25;
    let imm12 = (inst & 0x80000000) >> 31;
    d.imm = signext((imm12 << 12) | (imm11 << 11) | (imm10_5 << 5) | (imm4_1 << 1), 13);
    d
}

fn decode_u(inst: u32, opcode: RiscvOpcode) -> DecodedRv64im {
    let mut d = DecodedRv64im::new(opcode, RiscvFormat::U);
    d.rd = (inst & 0xF80) >> 7;
    d.imm = (((inst & 0xFFFFF000) >> 12) << 12) as i32;
    d
}

fn decode_j(inst: u32, opcode: RiscvOpcode) -> DecodedRv64im {
    let mut d = DecodedRv64im::new(opcode, RiscvFormat::J);
    d.rd = (inst & 0xF80) >> 7;
    let imm20 = (inst & 0x80000000) >> 31;
    let imm10_1 = (inst & 0x7FE00000) >> 21;
    let imm11 = (inst & 0x100000) >> 20;
    let imm19_12 = (inst & 0xFF000) >> 12;
    d.imm = signext((imm20 << 20) | (imm19_12 << 12) | (imm11 << 11) | (imm10_1 << 1), 21);
    d
}

fn decode_fence(inst: u32) -> DecodedRv64im {
    let mut d = DecodedRv64im::new(RiscvOpcode::Fence, RiscvFormat::F);
    d.funct3 = (inst & 0x7000) >> 12;
    d.rd = (inst & 0xF80) >> 7;
    d.rs1 = (inst & 0xF8000) >> 15;
    d.pred = (inst >> 24) & 0xF;
    d.succ = (inst >> 20) & 0xF;
    match d.funct3 {
        0 => {
            if (inst & 0xF00F8F80) != 0 {
                DecodedRv64im::reserved()
            } else {
                d
            }
        }
        1 => DecodedRv64im::unsupported(),
        _ => DecodedRv64im::reserved(),
    }
}

pub fn decode_32_core(inst: u32) -> DecodedRv64im {
    match inst & 0x7F {
        0x03 => match (inst >> 12) & 0x7 {
            0 => decode_i(inst, RiscvOpcode::Lb, false),
            1 => decode_i(inst, RiscvOpcode::Lh, false),
            2 => decode_i(inst, RiscvOpcode::Lw, false),
            3 => decode_i(inst, RiscvOpcode::Ld, false),
            4 => decode_i(inst, RiscvOpcode::Lbu, false),
            5 => decode_i(inst, RiscvOpcode::Lhu, false),
            6 => decode_i(inst, RiscvOpcode::Lwu, false),
            _ => DecodedRv64im::reserved(),
        },
        0x0F => decode_fence(inst),
        0x13 => match (inst >> 12) & 0x7 {
            0 => decode_i(inst, RiscvOpcode::Addi, false),
            1 => {
                if ((inst >> 26) & 0x3F) == 0 {
                    decode_i(inst, RiscvOpcode::Slli, true)
                } else {
                    DecodedRv64im::reserved()
                }
            }
            2 => decode_i(inst, RiscvOpcode::Slti, false),
            3 => decode_i(inst, RiscvOpcode::Sltiu, false),
            4 => decode_i(inst, RiscvOpcode::Xori, false),
            5 => match (inst >> 26) & 0x3F {
                0 => decode_i(inst, RiscvOpcode::Srli, true),
                16 => decode_i(inst, RiscvOpcode::Srai, true),
                _ => DecodedRv64im::reserved(),
            },
            6 => decode_i(inst, RiscvOpcode::Ori, false),
            7 => decode_i(inst, RiscvOpcode::Andi, false),
            _ => DecodedRv64im::reserved(),
        },
        0x17 => decode_u(inst, RiscvOpcode::Auipc),
        0x1B => match (inst >> 12) & 0x7 {
            0 => decode_i(inst, RiscvOpcode::Addiw, false),
            1 => {
                if ((inst >> 25) & 0x7F) == 0 {
                    decode_i(inst, RiscvOpcode::Slliw, true)
                } else {
                    DecodedRv64im::reserved()
                }
            }
            5 => match (inst >> 25) & 0x7F {
                0 => decode_i(inst, RiscvOpcode::Srliw, true),
                32 => decode_i(inst, RiscvOpcode::Sraiw, true),
                _ => DecodedRv64im::reserved(),
            },
            _ => DecodedRv64im::reserved(),
        },
        0x23 => match (inst >> 12) & 0x7 {
            0 => decode_s(inst, RiscvOpcode::Sb),
            1 => decode_s(inst, RiscvOpcode::Sh),
            2 => decode_s(inst, RiscvOpcode::Sw),
            3 => decode_s(inst, RiscvOpcode::Sd),
            _ => DecodedRv64im::reserved(),
        },
        0x33 => match (inst >> 12) & 0x7 {
            0 => match (inst >> 25) & 0x7F {
                0 => decode_r(inst, RiscvOpcode::Add),
                1 => decode_r(inst, RiscvOpcode::Mul),
                32 => decode_r(inst, RiscvOpcode::Sub),
                _ => DecodedRv64im::reserved(),
            },
            1 => match (inst >> 25) & 0x7F {
                0 => decode_r(inst, RiscvOpcode::Sll),
                1 => decode_r(inst, RiscvOpcode::Mulh),
                _ => DecodedRv64im::reserved(),
            },
            2 => match (inst >> 25) & 0x7F {
                0 => decode_r(inst, RiscvOpcode::Slt),
                1 => decode_r(inst, RiscvOpcode::Mulhsu),
                _ => DecodedRv64im::reserved(),
            },
            3 => match (inst >> 25) & 0x7F {
                0 => decode_r(inst, RiscvOpcode::Sltu),
                1 => decode_r(inst, RiscvOpcode::Mulhu),
                _ => DecodedRv64im::reserved(),
            },
            4 => match (inst >> 25) & 0x7F {
                0 => decode_r(inst, RiscvOpcode::Xor),
                1 => decode_r(inst, RiscvOpcode::Div),
                _ => DecodedRv64im::reserved(),
            },
            5 => match (inst >> 25) & 0x7F {
                0 => decode_r(inst, RiscvOpcode::Srl),
                1 => decode_r(inst, RiscvOpcode::Divu),
                32 => decode_r(inst, RiscvOpcode::Sra),
                _ => DecodedRv64im::reserved(),
            },
            6 => match (inst >> 25) & 0x7F {
                0 => decode_r(inst, RiscvOpcode::Or),
                1 => decode_r(inst, RiscvOpcode::Rem),
                _ => DecodedRv64im::reserved(),
            },
            7 => match (inst >> 25) & 0x7F {
                0 => decode_r(inst, RiscvOpcode::And),
                1 => decode_r(inst, RiscvOpcode::Remu),
                _ => DecodedRv64im::reserved(),
            },
            _ => DecodedRv64im::reserved(),
        },
        0x37 => decode_u(inst, RiscvOpcode::Lui),
        0x3B => match (inst >> 12) & 0x7 {
            0 => match (inst >> 25) & 0x7F {
                0 => decode_r(inst, RiscvOpcode::Addw),
                1 => decode_r(inst, RiscvOpcode::Mulw),
                32 => decode_r(inst, RiscvOpcode::Subw),
                _ => DecodedRv64im::reserved(),
            },
            1 => {
                if ((inst >> 25) & 0x7F) == 0 {
                    decode_r(inst, RiscvOpcode::Sllw)
                } else {
                    DecodedRv64im::reserved()
                }
            }
            4 => {
                if ((inst >> 25) & 0x7F) == 1 {
                    decode_r(inst, RiscvOpcode::Divw)
                } else {
                    DecodedRv64im::reserved()
                }
            }
            5 => match (inst >> 25) & 0x7F {
                0 => decode_r(inst, RiscvOpcode::Srlw),
                1 => decode_r(inst, RiscvOpcode::Divuw),
                32 => decode_r(inst, RiscvOpcode::Sraw),
                _ => DecodedRv64im::reserved(),
            },
            6 => {
                if ((inst >> 25) & 0x7F) == 1 {
                    decode_r(inst, RiscvOpcode::Remw)
                } else {
                    DecodedRv64im::reserved()
                }
            }
            7 => {
                if ((inst >> 25) & 0x7F) == 1 {
                    decode_r(inst, RiscvOpcode::Remuw)
                } else {
                    DecodedRv64im::reserved()
                }
            }
            _ => DecodedRv64im::reserved(),
        },
        0x63 => match (inst >> 12) & 0x7 {
            0 => decode_b(inst, RiscvOpcode::Beq),
            1 => decode_b(inst, RiscvOpcode::Bne),
            4 => decode_b(inst, RiscvOpcode::Blt),
            5 => decode_b(inst, RiscvOpcode::Bge),
            6 => decode_b(inst, RiscvOpcode::Bltu),
            7 => decode_b(inst, RiscvOpcode::Bgeu),
            _ => DecodedRv64im::reserved(),
        },
        0x67 => decode_i(inst, RiscvOpcode::Jalr, false),
        0x6F => decode_j(inst, RiscvOpcode::Jal),
        _ => DecodedRv64im::unsupported(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(funct7: u32, rs2: u32, rs1: u32, funct3: u32, rd: u32, opcode: u32) -> u32 {
        (funct7 << 25) | (rs2 << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | opcode
    }

    fn i(imm: u32, rs1: u32, funct3: u32, rd: u32, opcode: u32) -> u32 {
        (imm << 20) | (rs1 << 15) | (funct3 << 12) | (rd << 7) | opcode
    }

    fn s(imm: u32, rs2: u32, rs1: u32, funct3: u32) -> u32 {
        ((imm >> 5) << 25) | (rs2 << 20) | (rs1 << 15) | (funct3 << 12) | ((imm & 0x1F) << 7) | 0x23
    }

    fn b(imm: u32, rs2: u32, rs1: u32, funct3: u32) -> u32 {
        (((imm >> 12) & 1) << 31)
            | (((imm >> 5) & 0x3F) << 25)
            | (rs2 << 20)
            | (rs1 << 15)
            | (funct3 << 12)
            | (((imm >> 1) & 0xF) << 8)
            | (((imm >> 11) & 1) << 7)
            | 0x63
    }

    fn u(imm: u32, rd: u32, opcode: u32) -> u32 {
        (imm & 0xFFFFF000) | (rd << 7) | opcode
    }

    fn j(imm: u32, rd: u32) -> u32 {
        (((imm >> 20) & 1) << 31)
            | (((imm >> 1) & 0x3FF) << 21)
            | (((imm >> 11) & 1) << 20)
            | (((imm >> 12) & 0xFF) << 12)
            | (rd << 7)
            | 0x6F
    }

    #[test]
    fn decode_core_covers_current_rv64im_opcode_surface() {
        let cases = [
            (u(0x12345000, 3, 0x37), RiscvOpcode::Lui),
            (u(0x12345000, 3, 0x17), RiscvOpcode::Auipc),
            (j(0x100, 3), RiscvOpcode::Jal),
            (i(0x100, 5, 0, 3, 0x67), RiscvOpcode::Jalr),
            (0x0000000F, RiscvOpcode::Fence),
            (r(0, 7, 5, 0, 3, 0x33), RiscvOpcode::Add),
            (r(32, 7, 5, 0, 3, 0x33), RiscvOpcode::Sub),
            (r(0, 7, 5, 1, 3, 0x33), RiscvOpcode::Sll),
            (r(0, 7, 5, 2, 3, 0x33), RiscvOpcode::Slt),
            (r(0, 7, 5, 3, 3, 0x33), RiscvOpcode::Sltu),
            (r(0, 7, 5, 4, 3, 0x33), RiscvOpcode::Xor),
            (r(0, 7, 5, 5, 3, 0x33), RiscvOpcode::Srl),
            (r(32, 7, 5, 5, 3, 0x33), RiscvOpcode::Sra),
            (r(0, 7, 5, 6, 3, 0x33), RiscvOpcode::Or),
            (r(0, 7, 5, 7, 3, 0x33), RiscvOpcode::And),
            (r(0, 7, 5, 0, 3, 0x3B), RiscvOpcode::Addw),
            (r(32, 7, 5, 0, 3, 0x3B), RiscvOpcode::Subw),
            (r(0, 7, 5, 1, 3, 0x3B), RiscvOpcode::Sllw),
            (r(0, 7, 5, 5, 3, 0x3B), RiscvOpcode::Srlw),
            (r(32, 7, 5, 5, 3, 0x3B), RiscvOpcode::Sraw),
            (r(1, 7, 5, 0, 3, 0x33), RiscvOpcode::Mul),
            (r(1, 7, 5, 1, 3, 0x33), RiscvOpcode::Mulh),
            (r(1, 7, 5, 2, 3, 0x33), RiscvOpcode::Mulhsu),
            (r(1, 7, 5, 3, 3, 0x33), RiscvOpcode::Mulhu),
            (r(1, 7, 5, 0, 3, 0x3B), RiscvOpcode::Mulw),
            (r(1, 7, 5, 4, 3, 0x33), RiscvOpcode::Div),
            (r(1, 7, 5, 5, 3, 0x33), RiscvOpcode::Divu),
            (r(1, 7, 5, 4, 3, 0x3B), RiscvOpcode::Divw),
            (r(1, 7, 5, 5, 3, 0x3B), RiscvOpcode::Divuw),
            (r(1, 7, 5, 6, 3, 0x33), RiscvOpcode::Rem),
            (r(1, 7, 5, 7, 3, 0x33), RiscvOpcode::Remu),
            (r(1, 7, 5, 6, 3, 0x3B), RiscvOpcode::Remw),
            (r(1, 7, 5, 7, 3, 0x3B), RiscvOpcode::Remuw),
            (i(0x123, 5, 0, 3, 0x13), RiscvOpcode::Addi),
            (i(7, 5, 1, 3, 0x13), RiscvOpcode::Slli),
            (i(0x123, 5, 2, 3, 0x13), RiscvOpcode::Slti),
            (i(0x123, 5, 3, 3, 0x13), RiscvOpcode::Sltiu),
            (i(0x123, 5, 4, 3, 0x13), RiscvOpcode::Xori),
            (i(7, 5, 5, 3, 0x13), RiscvOpcode::Srli),
            (i(0x407, 5, 5, 3, 0x13), RiscvOpcode::Srai),
            (i(0x123, 5, 6, 3, 0x13), RiscvOpcode::Ori),
            (i(0x123, 5, 7, 3, 0x13), RiscvOpcode::Andi),
            (i(0x123, 5, 0, 3, 0x1B), RiscvOpcode::Addiw),
            (i(7, 5, 1, 3, 0x1B), RiscvOpcode::Slliw),
            (i(7, 5, 5, 3, 0x1B), RiscvOpcode::Srliw),
            (i(0x407, 5, 5, 3, 0x1B), RiscvOpcode::Sraiw),
            (b(0x100, 7, 5, 0), RiscvOpcode::Beq),
            (b(0x100, 7, 5, 1), RiscvOpcode::Bne),
            (b(0x100, 7, 5, 4), RiscvOpcode::Blt),
            (b(0x100, 7, 5, 5), RiscvOpcode::Bge),
            (b(0x100, 7, 5, 6), RiscvOpcode::Bltu),
            (b(0x100, 7, 5, 7), RiscvOpcode::Bgeu),
            (i(0x20, 5, 0, 3, 0x03), RiscvOpcode::Lb),
            (i(0x20, 5, 4, 3, 0x03), RiscvOpcode::Lbu),
            (i(0x20, 5, 1, 3, 0x03), RiscvOpcode::Lh),
            (i(0x20, 5, 5, 3, 0x03), RiscvOpcode::Lhu),
            (i(0x20, 5, 2, 3, 0x03), RiscvOpcode::Lw),
            (i(0x20, 5, 6, 3, 0x03), RiscvOpcode::Lwu),
            (i(0x20, 5, 3, 3, 0x03), RiscvOpcode::Ld),
            (s(0x20, 7, 5, 0), RiscvOpcode::Sb),
            (s(0x20, 7, 5, 1), RiscvOpcode::Sh),
            (s(0x20, 7, 5, 2), RiscvOpcode::Sw),
            (s(0x20, 7, 5, 3), RiscvOpcode::Sd),
        ];

        assert_eq!(cases.len(), 63);
        for (raw, expected) in cases {
            let decoded = decode_32_core(raw);
            assert_eq!(decoded.opcode, expected, "raw=0x{raw:08x}");
            assert!(decoded.is_supported_rv64im(), "raw=0x{raw:08x}");
        }
    }

    #[test]
    fn decode_core_keeps_known_restrictions_visible() {
        assert_eq!(decode_32_core(0x1000000F).opcode, RiscvOpcode::Reserved);
        assert_eq!(decode_32_core(0x0000800F).opcode, RiscvOpcode::Reserved);
        assert_eq!(decode_32_core(0x0000008F).opcode, RiscvOpcode::Reserved);
        assert_eq!(decode_32_core(0x0000100F).opcode, RiscvOpcode::Unsupported);
    }
}
