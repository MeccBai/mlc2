use super::*;
use crate::ast::{builtins::Builtin, function::FuncSymbol};
use crate::gens::instruction::{Cast, Condition, Operator};

impl Expander<'_> {
    pub(super) fn builtin(
        &mut self,
        kind: Builtin,
        symbol: &FuncSymbol,
        argument: &Expression,
    ) -> Lowered {
        if kind == Builtin::ConstCStr {
            let Expression::InitListE(InitialList::String { value }) = argument else {
                fail("Checked const_c_str argument is not a constant string");
            };
            let mut bytes = value.as_bytes().to_vec();
            bytes.push(0);
            let name = crate::gens::memory::string_name(&bytes);
            let target = self.allocate();
            return Lowered {
                signed: false,
                value: LlvmValue {
                    code: vec![
                        Instruction::StringConstant {
                            name: name.clone(),
                            bytes,
                        },
                        Instruction::Gep {
                            target,
                            ty: LlvmType::Int(8),
                            pointer: IrValue::Global(name),
                            indices: vec![TypedValue {
                                ty: LlvmType::Int(64),
                                value: IrValue::Integer(0),
                            }],
                            inbounds: false,
                        },
                    ],
                    ty: LlvmType::Ptr,
                    reg: Some(target),
                    in_reg: true,
                },
            };
        }
        // Literals have a default frontend i32 type. Explicit casts must not
        // truncate large literals to that default before converting to their target.
        let mut input = if let Expression::ConstValueE(value) = argument {
            if let Ok(integer) = value.value.parse::<i128>() {
                if value.ty.is_integer(self.symbols)
                    && (integer < i32::MIN as i128 || integer > i32::MAX as i128)
                {
                    let width = if kind == Builtin::Alloc { 64 } else { 128 };
                    let target = self.allocate();
                    Lowered {
                        value: LlvmValue {
                            code: vec![Instruction::Operation {
                                target,
                                op: Operator::Add,
                                ty: LlvmType::Int(width),
                                lhs: IrValue::Integer(0),
                                rhs: IrValue::Integer(integer),
                            }],
                            ty: LlvmType::Int(width),
                            reg: Some(target),
                            in_reg: true,
                        },
                        signed: integer < 0,
                    }
                } else {
                    self.expression(argument)
                }
            } else {
                self.expression(argument)
            }
        } else {
            self.expression(argument)
        };
        let operand = self.load(&mut input.value);
        match kind {
            Builtin::ConstCStr => unreachable!("constant string handled before argument lowering"),
            Builtin::Cast => {
                let result = symbol
                    .ret_type
                    .unwrap_or_else(|| fail("Missing cast target"));
                let ty = IrGenerator::type_lowering(result, self.symbols);
                let signed = self.signed(result);
                let op = match (&input.value.ty, &ty) {
                    (a, b) if a == b => None,
                    (LlvmType::Int(a), LlvmType::Int(b)) if a > b => Some(Cast::Trunc),
                    (LlvmType::Int(_), LlvmType::Int(_)) => {
                        Some(if input.signed { Cast::SExt } else { Cast::ZExt })
                    }
                    (LlvmType::Int(_), LlvmType::Float | LlvmType::Double) => {
                        Some(if input.signed {
                            Cast::SIToFP
                        } else {
                            Cast::UIToFP
                        })
                    }
                    (LlvmType::Float | LlvmType::Double, LlvmType::Int(_)) => {
                        Some(if signed { Cast::FPToSI } else { Cast::FPToUI })
                    }
                    (LlvmType::Float, LlvmType::Double) => Some(Cast::FPExt),
                    (LlvmType::Double, LlvmType::Float) => Some(Cast::FPTrunc),
                    _ => fail("Checked cast has incompatible lowering types"),
                };
                if let Some(op) = op {
                    let target = self.allocate();
                    input.value.code.push(Instruction::Cast {
                        target,
                        op,
                        value: TypedValue {
                            ty: input.value.ty.clone(),
                            value: operand,
                        },
                        to: ty.clone(),
                    });
                    input.value.reg = Some(target);
                }
                input.value.ty = ty;
                input.value.in_reg = true;
                input.signed = signed;
                input
            }
            Builtin::Dealloc => {
                input.value.code.push(Instruction::Call {
                    target: None,
                    func: IrValue::Global("free".into()),
                    return_type: LlvmType::Void,
                    args: vec![TypedValue {
                        ty: LlvmType::Ptr,
                        value: operand,
                    }],
                    variadic_params: None,
                });
                Lowered {
                    value: LlvmValue {
                        code: input.value.code,
                        ty: LlvmType::Void,
                        reg: None,
                        in_reg: true,
                    },
                    signed: false,
                }
            }
            Builtin::Alloc => {
                let result = symbol
                    .ret_type
                    .unwrap_or_else(|| fail("Missing allocation type"));
                let CompileType::Ref(reference) = self.symbols.get_type(result).unqualified()
                else {
                    fail("Invalid allocation result");
                };
                let size = reference.base.size(self.symbols.arenas());
                let mut count = operand;
                if input.value.ty != LlvmType::Int(64) {
                    let target = self.allocate();
                    input.value.code.push(Instruction::Cast {
                        target,
                        op: if input.signed { Cast::SExt } else { Cast::ZExt },
                        value: TypedValue {
                            ty: input.value.ty,
                            value: count,
                        },
                        to: LlvmType::Int(64),
                    });
                    count = IrValue::Reg(target);
                }
                // Invalid counts and multiplication overflow return null without allocating.
                let checked = self.allocate();
                let allocated = self.allocate();
                let bytes = self.allocate();
                let result = self.allocate();
                let success = format!("alloc.{result}.success");
                let failure = format!("alloc.{result}.failure");
                let end = format!("alloc.{result}.end");
                input.value.code.push(Instruction::Compare {
                    target: checked,
                    condition: Condition::ULe,
                    ty: LlvmType::Int(64),
                    lhs: count.clone(),
                    rhs: IrValue::Integer((u64::MAX / (size.max(1) as u64)) as i128),
                });
                let condition = if input.signed {
                    let nonnegative = self.allocate();
                    let valid = self.allocate();
                    input.value.code.extend([
                        Instruction::Compare {
                            target: nonnegative,
                            condition: Condition::SGe,
                            ty: LlvmType::Int(64),
                            lhs: count.clone(),
                            rhs: IrValue::Integer(0),
                        },
                        Instruction::Operation {
                            target: valid,
                            op: Operator::And,
                            ty: LlvmType::Int(1),
                            lhs: IrValue::Reg(checked),
                            rhs: IrValue::Reg(nonnegative),
                        },
                    ]);
                    IrValue::Reg(valid)
                } else {
                    IrValue::Reg(checked)
                };
                input.value.code.extend([
                    Instruction::ConditionalBranch {
                        condition,
                        then_label: success.clone(),
                        else_label: failure.clone(),
                    },
                    Instruction::BasicBlock {
                        label: success.clone(),
                    },
                    Instruction::Operation {
                        target: bytes,
                        op: Operator::Mul,
                        ty: LlvmType::Int(64),
                        lhs: count,
                        rhs: IrValue::Integer(size as i128),
                    },
                    Instruction::Call {
                        target: Some(allocated),
                        func: IrValue::Global("malloc".into()),
                        return_type: LlvmType::Ptr,
                        args: vec![TypedValue {
                            ty: LlvmType::Int(64),
                            value: IrValue::Reg(bytes),
                        }],
                        variadic_params: None,
                    },
                    Instruction::Branch { label: end.clone() },
                    Instruction::BasicBlock {
                        label: failure.clone(),
                    },
                    Instruction::Branch { label: end.clone() },
                    Instruction::BasicBlock { label: end },
                    Instruction::Phi {
                        target: result,
                        ty: LlvmType::Ptr,
                        incoming: vec![
                            (IrValue::Reg(allocated), success),
                            (IrValue::Null, failure),
                        ],
                    },
                ]);
                Lowered {
                    value: LlvmValue {
                        code: input.value.code,
                        ty: LlvmType::Ptr,
                        reg: Some(result),
                        in_reg: true,
                    },
                    signed: false,
                }
            }
        }
    }
}
