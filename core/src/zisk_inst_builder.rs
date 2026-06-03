//! Builds a Zisk instruction.
//! The ZiskInstBuilder structure contains one ZiskInst structure, and provides a set of helper
//! methods to modify its attributes

use crate::{
    riscv2zisk_single_row::Rv64imLoweringInput,
    zisk_ops::{InvalidNameError, OpType, ZiskOp},
    ZiskInst, REGS_IN_MAIN_FROM, REGS_IN_MAIN_TO, REG_FIRST, SRC_C, SRC_IMM, SRC_IND, SRC_MEM,
    SRC_REG, STORE_IND, STORE_MEM, STORE_NONE, STORE_REG,
};
use riscv::RiscvInstruction;

// #[cfg(feature = "sp")]
// use crate::SRC_SP;

/// Helps building a Zisk instruction during the transpilation process
#[derive(Debug, Clone, Default)]
pub struct ZiskInstBuilder {
    /// Zisk instruction
    pub i: ZiskInst,
}

impl ZiskInstBuilder {
    /// Constructor setting the initial pc address
    #[inline(always)]
    pub fn new(paddr: u64) -> ZiskInstBuilder {
        let mut zib = ZiskInstBuilder::default();
        zib.i.paddr = paddr;
        zib
    }
    /// Constructor setting the initial pc address and original RISC-V instruction
    #[inline(always)]
    #[cfg(not(feature = "aeneas_extract"))]
    pub fn new_from_riscv(paddr: u64, riscv_inst: String) -> ZiskInstBuilder {
        let mut zib = ZiskInstBuilder::default();
        zib.i.paddr = paddr;
        zib.i.riscv_inst = Some(riscv_inst);
        zib
    }

    #[inline(always)]
    #[cfg(feature = "aeneas_extract")]
    pub fn new_from_riscv(paddr: u64, riscv_inst: String) -> ZiskInstBuilder {
        let _ = riscv_inst;
        Self::new(paddr)
    }

    #[inline(always)]
    #[cfg(not(feature = "aeneas_extract"))]
    pub fn new_for_riscv(i: &RiscvInstruction) -> ZiskInstBuilder {
        Self::new_from_riscv(i.rom_address, i.inst.clone())
    }

    #[inline(always)]
    #[cfg(feature = "aeneas_extract")]
    pub fn new_for_riscv(i: &RiscvInstruction) -> ZiskInstBuilder {
        Self::new(i.rom_address)
    }

    #[inline(always)]
    #[cfg(not(feature = "aeneas_extract"))]
    pub fn new_for_rv64im_lowering(i: &Rv64imLoweringInput) -> ZiskInstBuilder {
        Self::new(i.rom_address)
    }

    #[inline(always)]
    #[cfg(feature = "aeneas_extract")]
    pub fn new_for_rv64im_lowering(i: &Rv64imLoweringInput) -> ZiskInstBuilder {
        Self::new(i.rom_address)
    }

    /// Converts a string to an a source value
    fn a_src(&self, src: &str) -> u64 {
        match src {
            "reg" => SRC_REG,
            "mem" => SRC_MEM,
            "imm" => SRC_IMM,
            "lastc" => SRC_C,
            // #[cfg(feature = "sp")]
            // "sp" => SRC_SP,
            // "step" => SRC_STEP,
            _ => panic!("ZiskInstBuilder::a_src() called with invalid src={src}"),
        }
    }

    /// Converts a string to a b source value
    fn b_src(&self, src: &str) -> u64 {
        match src {
            "reg" => SRC_REG,
            "mem" => SRC_MEM,
            "imm" => SRC_IMM,
            "lastc" => SRC_C,
            "ind" => SRC_IND,
            _ => panic!("ZiskInstBuilder::b_src() called with invalid src={src}"),
        }
    }

    /// Converts a string to a c store value
    fn c_store(&self, store: &str) -> u64 {
        match store {
            "none" => STORE_NONE,
            "mem" => STORE_MEM,
            "reg" => STORE_REG,
            "ind" => STORE_IND,
            _ => panic!("ZiskInstBuilder::c_store() called with invalid store={store}"),
        }
    }

    /// Splits a 128 bits into 2 32-bits chunks
    pub fn nto32s(n: i128) -> (u32, u32) {
        let mut a = n;
        if a >= (1_i128 << 64) {
            panic!("ZiskInstBuilder::nto32s() n={a} is too big");
        }
        if a < 0 {
            a += 1_i128 << 64;
            if a < (1_i128 << 63) {
                panic!("ZiskInstBuilder::nto32s() n={a} is too small");
            }
        }
        ((a & 0xFFFFFFFF) as u32, (a >> 32) as u32)
    }

    /// Sets the a source instruction sttributes
    pub fn src_a(&mut self, src_input: &str, offset_imm_reg_input: u64, use_sp: bool) {
        let mut src = src_input;
        let mut offset_imm_reg = offset_imm_reg_input;
        if src == "reg" {
            if offset_imm_reg == 0 {
                src = "imm";
                offset_imm_reg = 0;
            } else if offset_imm_reg < REGS_IN_MAIN_FROM as u64
                || offset_imm_reg > REGS_IN_MAIN_TO as u64
            {
                src = "mem";
                offset_imm_reg = REG_FIRST + offset_imm_reg * 8;
            }
        }
        // assert!(src != "mem" || offset_imm_reg != 0);

        self.i.a_src = self.a_src(src);

        if self.i.a_src == SRC_REG || self.i.a_src == SRC_MEM {
            if use_sp {
                self.i.a_use_sp_imm1 = 1;
            } else {
                self.i.a_use_sp_imm1 = 0;
            }
            self.i.a_offset_imm0 = offset_imm_reg;
        } else if self.i.a_src == SRC_IMM {
            let (v0, v1) = Self::nto32s(offset_imm_reg as i128);
            self.i.a_use_sp_imm1 = v1 as u64;
            self.i.a_offset_imm0 = v0 as u64;
        } else {
            self.i.a_use_sp_imm1 = 0;
            self.i.a_offset_imm0 = 0;
        }
    }

    /// Sets the b source instruction sttributes
    pub fn src_b(&mut self, src_input: &str, offset_imm_reg_input: u64, use_sp: bool) {
        let mut src = src_input;
        let mut offset_imm_reg = offset_imm_reg_input;
        if src == "reg" {
            if offset_imm_reg == 0 {
                src = "imm";
                offset_imm_reg = 0;
            } else if offset_imm_reg < REGS_IN_MAIN_FROM as u64
                || offset_imm_reg > REGS_IN_MAIN_TO as u64
            {
                src = "mem";
                offset_imm_reg = REG_FIRST + offset_imm_reg * 8;
            }
        }
        self.i.b_src = self.b_src(src);

        if self.i.b_src == SRC_REG || self.i.b_src == SRC_MEM || self.i.b_src == SRC_IND {
            if use_sp {
                self.i.b_use_sp_imm1 = 1;
            } else {
                self.i.b_use_sp_imm1 = 0;
            }
            self.i.b_offset_imm0 = offset_imm_reg;
        } else if self.i.b_src == SRC_IMM {
            let (v0, v1) = Self::nto32s(offset_imm_reg as i128);
            self.i.b_use_sp_imm1 = v1 as u64;
            self.i.b_offset_imm0 = v0 as u64;
        } else {
            self.i.b_use_sp_imm1 = 0;
            self.i.b_offset_imm0 = 0;
        }
    }

    #[cfg(not(feature = "aeneas_extract"))]
    pub fn src_a_imm(&mut self, value: u64) {
        let (v0, v1) = Self::nto32s(value as i128);
        self.i.a_src = SRC_IMM;
        self.i.a_use_sp_imm1 = v1 as u64;
        self.i.a_offset_imm0 = v0 as u64;
    }

    #[cfg(feature = "aeneas_extract")]
    pub fn src_a_imm(&mut self, value: u64) {
        self.i.a_src = SRC_IMM;
        self.i.a_use_sp_imm1 = value >> 32;
        self.i.a_offset_imm0 = value & 0xffffffff;
    }

    pub fn src_a_reg(&mut self, reg: u64, use_sp: bool) {
        if reg == 0 {
            self.src_a_imm(0);
        } else if reg < REGS_IN_MAIN_FROM as u64 || reg > REGS_IN_MAIN_TO as u64 {
            self.i.a_src = SRC_MEM;
            self.i.a_use_sp_imm1 = if use_sp { 1 } else { 0 };
            self.i.a_offset_imm0 = REG_FIRST + reg * 8;
        } else {
            self.i.a_src = SRC_REG;
            self.i.a_use_sp_imm1 = if use_sp { 1 } else { 0 };
            self.i.a_offset_imm0 = reg;
        }
    }

    #[cfg(not(feature = "aeneas_extract"))]
    pub fn src_b_imm(&mut self, value: u64) {
        let (v0, v1) = Self::nto32s(value as i128);
        self.i.b_src = SRC_IMM;
        self.i.b_use_sp_imm1 = v1 as u64;
        self.i.b_offset_imm0 = v0 as u64;
    }

    #[cfg(feature = "aeneas_extract")]
    pub fn src_b_imm(&mut self, value: u64) {
        self.i.b_src = SRC_IMM;
        self.i.b_use_sp_imm1 = value >> 32;
        self.i.b_offset_imm0 = value & 0xffffffff;
    }

    pub fn src_b_reg(&mut self, reg: u64, use_sp: bool) {
        if reg == 0 {
            self.src_b_imm(0);
        } else if reg < REGS_IN_MAIN_FROM as u64 || reg > REGS_IN_MAIN_TO as u64 {
            self.i.b_src = SRC_MEM;
            self.i.b_use_sp_imm1 = if use_sp { 1 } else { 0 };
            self.i.b_offset_imm0 = REG_FIRST + reg * 8;
        } else {
            self.i.b_src = SRC_REG;
            self.i.b_use_sp_imm1 = if use_sp { 1 } else { 0 };
            self.i.b_offset_imm0 = reg;
        }
    }

    pub fn src_b_lastc(&mut self) {
        self.i.b_src = SRC_C;
        self.i.b_use_sp_imm1 = 0;
        self.i.b_offset_imm0 = 0;
    }

    pub fn src_b_ind(&mut self, offset: u64, use_sp: bool) {
        self.i.b_src = SRC_IND;
        self.i.b_use_sp_imm1 = if use_sp { 1 } else { 0 };
        self.i.b_offset_imm0 = offset;
    }

    /// Sets the c store instruction attributes
    pub fn store(&mut self, dst_input: &str, offset_input: i64, use_sp: bool, store_pc: bool) {
        let mut dst = dst_input;
        let mut offset = offset_input;
        if dst == "reg" {
            if offset == 0 {
                return;
            } else if offset < REGS_IN_MAIN_FROM as i64 || offset > REGS_IN_MAIN_TO as i64 {
                dst = "mem";
                offset = REG_FIRST as i64 + offset * 8;
            }
        }

        self.i.store_pc = store_pc;
        self.i.store = self.c_store(dst);

        if self.i.store == STORE_REG || self.i.store == STORE_MEM || self.i.store == STORE_IND {
            self.i.store_use_sp = use_sp;
            self.i.store_offset = offset;
        } else {
            self.i.store_use_sp = false;
            self.i.store_offset = 0;
        }
    }

    pub fn store_reg(&mut self, offset: i64, use_sp: bool, store_pc: bool) {
        if offset == 0 {
            return;
        } else if offset < REGS_IN_MAIN_FROM as i64 || offset > REGS_IN_MAIN_TO as i64 {
            self.i.store_pc = store_pc;
            self.i.store = STORE_MEM;
            self.i.store_use_sp = use_sp;
            self.i.store_offset = REG_FIRST as i64 + offset * 8;
        } else {
            self.i.store_pc = store_pc;
            self.i.store = STORE_REG;
            self.i.store_use_sp = use_sp;
            self.i.store_offset = offset;
        }
    }

    /// Set the store as a store ra
    pub fn store_pc(&mut self, dst: &str, offset: i64, use_sp: bool) {
        self.store(dst, offset, use_sp, true);
    }

    pub fn store_pc_reg(&mut self, offset: i64, use_sp: bool) {
        self.store_reg(offset, use_sp, true);
    }

    pub fn store_ind(&mut self, offset: i64, use_sp: bool) {
        self.i.store_pc = false;
        self.i.store = STORE_IND;
        self.i.store_use_sp = use_sp;
        self.i.store_offset = offset;
    }

    /// Sets the set pc flag to true
    pub fn set_pc(&mut self) {
        self.i.set_pc = true;
    }

    // #[cfg(feature = "sp")]
    // pub fn set_sp(&mut self) {
    //     self.i.set_sp = true;
    // }

    /// Sets the opcode, and other instruction attributes that depend on it
    pub fn op(&mut self, optxt: &str) -> Result<(), InvalidNameError> {
        let op = ZiskOp::try_from_name(optxt)?;
        self.op_zisk(op);
        Ok(())
    }

    pub fn op_zisk(&mut self, op: ZiskOp) {
        let op_type = op.op_type();
        self.i.is_external_op = match op_type {
            OpType::Internal | OpType::Fcall => false,
            _ => true,
        };
        self.i.op = op.code();
        self.set_runtime_op_fields(op);
        self.i.op_type = op_type.into();
        self.i.input_size = op.input_size();
        self.i.is_precompiled = op.input_size() > 0;
    }

    fn set_runtime_op_fields(&mut self, op: ZiskOp) {
        #[cfg(not(feature = "aeneas_extract"))]
        {
            self.i.op_str = op.name();
            self.i.func = op.get_call_function();
        }
        self.i.m32 = op.is_m32();
    }

    /// Sets jump offsets.  The first offset is added to the pc when a set pc or a flag happens,
    /// and the second offset is the default one.
    pub fn j(&mut self, j1: i64, j2: i64) {
        self.i.jmp_offset1 = j1;
        self.i.jmp_offset2 = j2;
    }

    /// Set the indirection data width.  Accepted values are 1, 2, 4 and 8 (bytes.)
    #[cfg(not(feature = "aeneas_extract"))]
    pub fn ind_width(&mut self, w: u64) {
        self.i.ind_width = match w {
            1 | 2 | 4 | 8 => w,
            _ => {
                panic!("ZiskInstBuilder::indWidth() invalid widtch={w}");
            }
        };
    }

    /// Set the indirection data width.  Accepted values are 1, 2, 4 and 8 (bytes.)
    #[cfg(feature = "aeneas_extract")]
    pub fn ind_width(&mut self, w: u64) {
        self.i.ind_width = match w {
            1 | 2 | 4 | 8 => w,
            _ => panic!("ZiskInstBuilder::indWidth() invalid width"),
        };
    }

    /// Sets the end flag to true, to be called only by the last instruction in any execution path
    pub fn end(&mut self) {
        self.i.end = true;
    }

    // #[cfg(feature = "sp")]
    // pub fn inc_sp(&mut self, inc: u64) {
    //     self.i.inc_sp += inc;
    // }

    /// Sets a verbose description of the instruction
    #[cfg(not(feature = "aeneas_extract"))]
    pub fn verbose(&mut self, s: &str) {
        self.i.verbose = s.to_owned();
    }

    /// Sets a verbose description of the instruction
    #[cfg(feature = "aeneas_extract")]
    pub fn verbose(&mut self, s: &str) {
        let _ = s;
    }

    /// Called when the instruction has been built
    pub fn build(&mut self) {
        //print!("ZiskInstBuilder::build() i=[ {} ]\n", self.i.to_string());
    }
}
