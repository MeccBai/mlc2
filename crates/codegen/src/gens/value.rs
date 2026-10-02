use super::instruction::{Instruction, LlvmType};

/// Logical expression result. For memory-backed values, reg is its address;
/// ty remains the stored value type. Void expressions have no register.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LlvmValue {
    pub code: Vec<Instruction>,
    pub ty: LlvmType,
    pub reg: Option<usize>,
    pub in_reg: bool,
}
