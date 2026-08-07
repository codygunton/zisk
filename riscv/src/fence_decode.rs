#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FenceDecodeKind {
    Fence,
    FenceI,
    Reserved,
    NotFence,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FenceDecode {
    pub kind: FenceDecodeKind,
    pub funct3: u32,
    pub pred: u32,
    pub succ: u32,
}

pub fn decode_fence_raw(inst: u32) -> FenceDecode {
    if inst & 0x7F != 15 {
        return FenceDecode { kind: FenceDecodeKind::NotFence, funct3: 0, pred: 0, succ: 0 };
    }

    let funct3 = (inst & 0x7000) >> 12;
    if funct3 == 0 {
        if (inst & 0xF00F8F80) != 0 {
            FenceDecode { kind: FenceDecodeKind::Reserved, funct3, pred: 0, succ: 0 }
        } else {
            FenceDecode {
                kind: FenceDecodeKind::Fence,
                funct3,
                pred: (inst & 0x0F000000) >> 24,
                succ: (inst & 0x00F00000) >> 20,
            }
        }
    } else if funct3 == 1 {
        if (inst & 0xFFFF8F80) != 0 {
            FenceDecode { kind: FenceDecodeKind::Reserved, funct3, pred: 0, succ: 0 }
        } else {
            FenceDecode { kind: FenceDecodeKind::FenceI, funct3, pred: 0, succ: 0 }
        }
    } else {
        FenceDecode { kind: FenceDecodeKind::Reserved, funct3, pred: 0, succ: 0 }
    }
}
