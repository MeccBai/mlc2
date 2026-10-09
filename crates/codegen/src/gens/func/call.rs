use super::{AbiParameter, LlvmFunc, ParameterMode};
use crate::gens::{
    LlvmValue,
    instruction::{Instruction, IrValue, LlvmType, TypedValue},
};
use std::fmt::{self, Write};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CallError {
    RegisterOverflow,
    UnsupportedReturnMapping,
    ArgumentCount {
        expected: usize,
        actual: usize,
    },
    ArgumentType {
        index: usize,
        expected: LlvmType,
        actual: LlvmType,
    },
    VoidResultTarget,
    InvalidVariadicType {
        index: usize,
    },
    Write,
}
impl fmt::Display for CallError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "LLVM call error: {self:?}")
    }
}
impl std::error::Error for CallError {}
impl From<fmt::Error> for CallError {
    fn from(_: fmt::Error) -> Self {
        Self::Write
    }
}

impl AbiParameter {
    pub fn llvm_type(&self) -> LlvmType {
        match self.mode {
            ParameterMode::Direct => self.ty.clone(),
            _ => LlvmType::Ptr,
        }
    }
}

impl LlvmFunc {
    /// Arguments are already lowered, in declaration order: hidden result,
    /// receiver, mapped explicit parameters, then optional variadic arguments.
    /// This method does not allocate storage, copy values or split aggregates.
    pub fn write_call(
        &self,
        output: &mut impl Write,
        target: Option<usize>,
        args: &[TypedValue],
    ) -> Result<(), CallError> {
        self.write_call_to(output, target, args, &IrValue::Global(self.name.clone()))
    }

    pub fn write_call_to(
        &self,
        output: &mut impl Write,
        target: Option<usize>,
        args: &[TypedValue],
        callee: &IrValue,
    ) -> Result<(), CallError> {
        self.validate_call(target, args)?;
        let fixed = self.parameters().count();
        if let Some(target) = target {
            write!(output, "%r{target} = ")?;
        }
        write!(output, "call {} ", self.result.ty)?;
        if self.variadic {
            output.write_char('(')?;
            for (i, param) in self.parameters().enumerate() {
                if i != 0 {
                    output.write_str(", ")?;
                }
                write!(output, "{}", param.llvm_type())?;
            }
            if fixed != 0 {
                output.write_str(", ")?;
            }
            output.write_str("...) ")?;
        }
        write!(output, "{callee}(")?;
        for (i, (param, arg)) in self.parameters().zip(args).enumerate() {
            if i != 0 {
                output.write_str(", ")?;
            }
            write!(output, "{param} {}", arg.value)?;
        }
        for (i, arg) in args.iter().enumerate().skip(fixed) {
            if i != 0 {
                output.write_str(", ")?;
            }
            write!(output, "{arg}")?;
        }
        output.write_char(')')?;
        Ok(())
    }

    fn validate_call(&self, target: Option<usize>, args: &[TypedValue]) -> Result<(), CallError> {
        let fixed = self.parameters().count();
        if args.len() < fixed || (!self.variadic && args.len() != fixed) {
            return Err(CallError::ArgumentCount {
                expected: fixed,
                actual: args.len(),
            });
        }
        if target.is_some() && self.result.ty == LlvmType::Void {
            return Err(CallError::VoidResultTarget);
        }
        for (i, (param, arg)) in self.parameters().zip(args).enumerate() {
            let expected = param.llvm_type();
            if arg.ty != expected {
                return Err(CallError::ArgumentType {
                    index: i,
                    expected,
                    actual: arg.ty.clone(),
                });
            }
        }
        for (index, arg) in args.iter().enumerate().skip(fixed) {
            if arg.ty == LlvmType::Void {
                return Err(CallError::InvalidVariadicType { index });
            }
        }
        Ok(())
    }

    pub fn call_text(
        &self,
        target: Option<usize>,
        args: &[TypedValue],
    ) -> Result<String, CallError> {
        let mut text = String::new();
        self.write_call(&mut text, target, args)?;
        Ok(text)
    }
    /// Input arguments exclude the hidden return pointer. start is the first
    /// free SSA ID; the returned ID is the next free ID after this expression.
    pub fn call(&self, start: usize, args: &[TypedValue]) -> Result<(usize, LlvmValue), CallError> {
        if self.result.hidden.len() > 1 {
            return Err(CallError::UnsupportedReturnMapping);
        }
        let indirect = self.result.hidden.first();
        let has_result = indirect.is_some() || self.result.ty != LlvmType::Void;
        let next = if has_result {
            start.checked_add(1).ok_or(CallError::RegisterOverflow)?
        } else {
            start
        };
        let reg = has_result.then_some(start);
        let target = if indirect.is_some() { None } else { reg };
        let mut lowered = Vec::with_capacity(args.len() + usize::from(indirect.is_some()));
        let mut code = Vec::with_capacity(2);
        let ty = if let Some(output) = indirect {
            lowered.push(TypedValue {
                ty: LlvmType::Ptr,
                value: IrValue::Reg(start),
            });
            code.push(Instruction::Alloca {
                target: start,
                ty: output.ty.clone(),
                count: None,
                align: Some(output.align),
            });
            output.ty.clone()
        } else {
            self.result.ty.clone()
        };
        lowered.extend_from_slice(args);
        self.validate_call(target, &lowered)?;
        code.push(Instruction::MappedCall {
            func: Box::new(self.clone()),
            target,
            args: lowered,
        });
        Ok((
            next,
            LlvmValue {
                code,
                ty,
                reg,
                in_reg: indirect.is_none(),
            },
        ))
    }
}
