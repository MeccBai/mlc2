//! Typed instruction collection. Rendering writes directly into one shared buffer.
use std::fmt::{self, Write};

mod format;
#[cfg(test)]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlvmType {
    Void,
    Int(u32),
    Float,
    Double,
    Ptr,
    Array {
        element: Box<LlvmType>,
        length: usize,
    },
    Struct(Vec<LlvmType>),
    Named(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrValue {
    /// Printed as %rN, avoiding LLVM's sequential unnamed-value numbering rule.
    Reg(usize),
    Global(String),
    Integer(i128),
    Bool(bool),
    Null,
    ZeroInitializer,
    /// LLVM literal spelling, e.g. a floating-point hex literal. Not source text.
    Val(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypedValue {
    pub ty: LlvmType,
    pub value: IrValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    Add,
    Sub,
    Mul,
    SDiv,
    UDiv,
    SRem,
    URem,
    FAdd,
    FSub,
    FMul,
    FDiv,
    FRem,
    And,
    Or,
    Xor,
    Shl,
    LShr,
    AShr,
}

/// Explicit LLVM predicates: signedness and floating-point NaN semantics matter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Condition {
    Eq,
    Ne,
    SLt,
    SLe,
    SGt,
    SGe,
    ULt,
    ULe,
    UGt,
    UGe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FloatCondition {
    OEq,
    ONe,
    OLt,
    OLe,
    OGt,
    OGe,
    UEq,
    UNe,
    ULt,
    ULe,
    UGt,
    UGe,
    Ord,
    Uno,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cast {
    Trunc,
    ZExt,
    SExt,
    FPTrunc,
    FPExt,
    FPToUI,
    FPToSI,
    UIToFP,
    SIToFP,
    PtrToInt,
    IntToPtr,
    Bitcast,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instruction {
    MappedCall {
        func: Box<crate::gens::func::LlvmFunc>,
        target: Option<usize>,
        args: Vec<TypedValue>,
    },
    /// Count is an element count, not a byte size. None allocates one element.
    Alloca {
        target: usize,
        ty: LlvmType,
        count: Option<TypedValue>,
        align: Option<usize>,
    },
    Store {
        pointer: IrValue,
        value: TypedValue,
        align: Option<usize>,
    },
    Load {
        target: usize,
        ty: LlvmType,
        pointer: IrValue,
        align: Option<usize>,
    },
    Gep {
        target: usize,
        ty: LlvmType,
        pointer: IrValue,
        indices: Vec<TypedValue>,
        inbounds: bool,
    },
    /// variadic_params contains the fixed signature for a variadic call.
    Call {
        target: Option<usize>,
        func: IrValue,
        return_type: LlvmType,
        args: Vec<TypedValue>,
        variadic_params: Option<Vec<LlvmType>>,
    },
    Operation {
        target: usize,
        op: Operator,
        ty: LlvmType,
        lhs: IrValue,
        rhs: IrValue,
    },
    Compare {
        target: usize,
        condition: Condition,
        ty: LlvmType,
        lhs: IrValue,
        rhs: IrValue,
    },
    FloatCompare {
        target: usize,
        condition: FloatCondition,
        ty: LlvmType,
        lhs: IrValue,
        rhs: IrValue,
    },
    Cast {
        target: usize,
        op: Cast,
        value: TypedValue,
        to: LlvmType,
    },
    Phi {
        target: usize,
        ty: LlvmType,
        incoming: Vec<(IrValue, String)>,
    },
    BasicBlock {
        label: String,
    },
    Branch {
        label: String,
    },
    ConditionalBranch {
        condition: IrValue,
        then_label: String,
        else_label: String,
    },
    Return {
        value: Option<TypedValue>,
    },
    Unreachable,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct InstructionBuffer {
    pub instructions: Vec<Instruction>,
}

impl InstructionBuffer {
    pub fn push(&mut self, instruction: Instruction) {
        self.instructions.push(instruction);
    }

    /// Append without allocating a separate string per instruction.
    pub fn write_to(&self, output: &mut impl Write) -> fmt::Result {
        for instruction in &self.instructions {
            writeln!(output, "{instruction}")?;
        }
        Ok(())
    }

    pub fn format(&self) -> String {
        let mut output = String::with_capacity(self.instructions.len().saturating_mul(64));
        self.write_to(&mut output)
            .expect("writing to String cannot fail");
        output
    }
}
