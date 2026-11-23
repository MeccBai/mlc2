use super::*;
use crate::ast::{builtins::Builtin, function::FuncSymbol};
use crate::gens::instruction::{Cast, Condition, Operator};

impl Expander<'_> {
    pub(super) fn builtin(
        &mut self,
        kind: Builtin,
        symbol: &FuncSymbol,
        arguments: &[Expression],
    ) -> Lowered {
        let argument = &arguments[0];
        if kind == Builtin::StrFirstByte {
            return self.str_first_byte(argument);
        }
        if kind == Builtin::StrFromParts {
            let mut pointer = self.expression(argument).value;
            let mut length = self
                .expression_expected(&arguments[1], Some(symbol.params[1].0))
                .value;
            let pointer_operand = self.load(&mut pointer);
            let length_operand = self.load(&mut length);
            let length_operand = self.convert_initializer(
                &mut length,
                length_operand,
                &LlvmType::Int(64),
                &arguments[1],
            );
            let mut code = pointer.code;
            code.extend(length.code);
            let ty = LlvmType::Struct(vec![LlvmType::Ptr, LlvmType::Int(64)]);
            let aggregate = self.allocate();
            let result = self.allocate();
            code.push(Instruction::InsertValue {
                target: aggregate,
                aggregate: TypedValue {
                    ty: ty.clone(),
                    value: IrValue::ZeroInitializer,
                },
                value: TypedValue {
                    ty: LlvmType::Ptr,
                    value: pointer_operand,
                },
                index: 0,
            });
            code.push(Instruction::InsertValue {
                target: result,
                aggregate: TypedValue {
                    ty: ty.clone(),
                    value: IrValue::Reg(aggregate),
                },
                value: TypedValue {
                    ty: LlvmType::Int(64),
                    value: length_operand,
                },
                index: 1,
            });
            return Lowered {
                signed: false,
                value: LlvmValue {
                    code,
                    ty,
                    reg: Some(result),
                    in_reg: true,
                },
            };
        }
        if matches!(
            kind,
            Builtin::ConstCStr | Builtin::StrAsPtr | Builtin::StrLen
        ) {
            let mut value = self.expression(argument).value;
            let operand = self.load(&mut value);
            let target = self.allocate();
            value.code.push(Instruction::ExtractValue {
                target,
                aggregate: TypedValue {
                    ty: value.ty.clone(),
                    value: operand,
                },
                index: usize::from(kind == Builtin::StrLen),
            });
            return Lowered {
                signed: false,
                value: LlvmValue {
                    code: value.code,
                    ty: if kind == Builtin::StrLen {
                        LlvmType::Int(64)
                    } else {
                        LlvmType::Ptr
                    },
                    reg: Some(target),
                    in_reg: true,
                },
            };
        }
        // Literals have a default frontend i32 type. Explicit casts must not
        // truncate large literals to that default before converting to their target.
        let mut input = if matches!(kind, Builtin::Dealloc | Builtin::Address) {
            self.expression_expected(argument, Some(symbol.params[0].0))
        } else if let Expression::ConstValueE(value, ..) = argument {
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
            Builtin::Address => {
                let target = self.allocate();
                input.value.code.push(Instruction::Cast {
                    target,
                    op: Cast::IntToPtr,
                    value: TypedValue {
                        ty: input.value.ty,
                        value: operand,
                    },
                    to: LlvmType::Ptr,
                });
                input.value.ty = LlvmType::Ptr;
                input.value.reg = Some(target);
                input.value.in_reg = true;
                input.signed = false;
                input
            }
            Builtin::IsDebug
            | Builtin::HostOs
            | Builtin::Todo
            | Builtin::Size
            | Builtin::BitAnd
            | Builtin::BitXor
            | Builtin::BitOr
            | Builtin::BitNot
            | Builtin::BitShl
            | Builtin::BitShr => unreachable!(),
            Builtin::ConstCStr
            | Builtin::StrAsPtr
            | Builtin::StrLen
            | Builtin::StrFromParts
            | Builtin::StrFirstByte => {
                unreachable!("constant string handled before argument lowering")
            }
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
                let bits = (mlc_core::ast::types::TypeLookup::target_layout(self.symbols).ptr_size
                    * 8) as u32;
                let maximum = (1u128 << bits) - 1;
                let original_bits = match input.value.ty {
                    LlvmType::Int(bits) => bits,
                    _ => fail("allocation count is not an integer"),
                };
                let overflow = if original_bits > bits {
                    let valid = self.allocate();
                    input.value.code.push(Instruction::Compare {
                        target: valid,
                        condition: Condition::ULe,
                        ty: input.value.ty.clone(),
                        lhs: operand.clone(),
                        rhs: IrValue::Integer(maximum as i128),
                    });
                    Some(valid)
                } else {
                    None
                };
                let mut count = operand;
                if input.value.ty != LlvmType::Int(bits) {
                    let target = self.allocate();
                    input.value.code.push(Instruction::Cast {
                        target,
                        op: if original_bits > bits {
                            Cast::Trunc
                        } else if input.signed {
                            Cast::SExt
                        } else {
                            Cast::ZExt
                        },
                        value: TypedValue {
                            ty: input.value.ty,
                            value: count,
                        },
                        to: LlvmType::Int(bits),
                    });
                    count = IrValue::Reg(target);
                }
                // Invalid sizes and allocation failures terminate without unwinding.
                let checked = self.allocate();
                let bytes = self.allocate();
                let result = self.allocate();
                let nonnull = self.allocate();
                let success = format!("alloc.{result}.success");
                let failure = format!("alloc.{result}.failure");
                let end = format!("alloc.{result}.end");
                input.value.code.push(Instruction::Compare {
                    target: checked,
                    condition: Condition::ULe,
                    ty: LlvmType::Int(bits),
                    lhs: count.clone(),
                    rhs: IrValue::Integer((maximum / size.max(1) as u128) as i128),
                });
                let condition = if input.signed {
                    let nonnegative = self.allocate();
                    let valid = self.allocate();
                    input.value.code.extend([
                        Instruction::Compare {
                            target: nonnegative,
                            condition: Condition::SGe,
                            ty: LlvmType::Int(bits),
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
                let condition = if let Some(overflow) = overflow {
                    let valid = self.allocate();
                    input.value.code.push(Instruction::Operation {
                        target: valid,
                        op: Operator::And,
                        ty: LlvmType::Int(1),
                        lhs: condition,
                        rhs: IrValue::Reg(overflow),
                    });
                    IrValue::Reg(valid)
                } else {
                    condition
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
                        ty: LlvmType::Int(bits),
                        lhs: count,
                        rhs: IrValue::Integer(size as i128),
                    },
                    Instruction::Call {
                        target: Some(result),
                        func: IrValue::Global("malloc".into()),
                        return_type: LlvmType::Ptr,
                        args: vec![TypedValue {
                            ty: LlvmType::Int(bits),
                            value: IrValue::Reg(bytes),
                        }],
                        variadic_params: None,
                    },
                    Instruction::Compare {
                        target: nonnull,
                        condition: Condition::Ne,
                        ty: LlvmType::Ptr,
                        lhs: IrValue::Reg(result),
                        rhs: IrValue::Null,
                    },
                    Instruction::ConditionalBranch {
                        condition: IrValue::Reg(nonnull),
                        then_label: end.clone(),
                        else_label: failure.clone(),
                    },
                    Instruction::BasicBlock {
                        label: failure.clone(),
                    },
                    Instruction::Call {
                        target: None,
                        func: IrValue::Global("abort".into()),
                        return_type: LlvmType::Void,
                        args: vec![],
                        variadic_params: None,
                    },
                    Instruction::Unreachable,
                    Instruction::BasicBlock { label: end },
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

impl Expander<'_> {
    fn str_first_byte(&mut self, argument: &Expression) -> Lowered {
        let mut value = self.expression(argument).value;
        let operand = self.load(&mut value);
        let aggregate = TypedValue {
            ty: value.ty,
            value: operand,
        };
        let length = self.allocate();
        let has_byte = self.allocate();
        let pointer = self.allocate();
        let byte = self.allocate();
        let result = self.allocate();
        let nonempty = format!("first_byte.{result}.nonempty");
        let empty = format!("first_byte.{result}.empty");
        let end = format!("first_byte.{result}.end");
        let ty = LlvmType::Int(8);
        value.code.extend([
            Instruction::ExtractValue {
                target: length,
                aggregate: aggregate.clone(),
                index: 1,
            },
            Instruction::Compare {
                target: has_byte,
                condition: Condition::Ne,
                ty: LlvmType::Int(64),
                lhs: IrValue::Reg(length),
                rhs: IrValue::Integer(0),
            },
            Instruction::ConditionalBranch {
                condition: IrValue::Reg(has_byte),
                then_label: nonempty.clone(),
                else_label: empty.clone(),
            },
            Instruction::BasicBlock {
                label: nonempty.clone(),
            },
            Instruction::ExtractValue {
                target: pointer,
                aggregate,
                index: 0,
            },
            Instruction::Load {
                target: byte,
                ty: ty.clone(),
                pointer: IrValue::Reg(pointer),
                align: None,
            },
            Instruction::Branch { label: end.clone() },
            Instruction::BasicBlock {
                label: empty.clone(),
            },
            Instruction::Branch { label: end.clone() },
            Instruction::BasicBlock { label: end },
            Instruction::Phi {
                target: result,
                ty: ty.clone(),
                incoming: vec![(IrValue::Reg(byte), nonempty), (IrValue::Integer(0), empty)],
            },
        ]);
        Lowered {
            value: LlvmValue {
                code: value.code,
                ty,
                reg: Some(result),
                in_reg: true,
            },
            signed: false,
        }
    }

    pub(super) fn todo(&mut self, argument: &Expression, expected: Option<TypeIndex>) -> Lowered {
        let Expression::InitListE(InitialList::String { value: message }, ..) = argument else {
            unreachable!()
        };
        let mut bytes = message.as_bytes().to_vec();
        bytes.push(0);
        let name = super::super::memory::string_name(&bytes);
        let ty = expected
            .map(|t| IrGenerator::type_lowering(t, self.symbols))
            .unwrap_or(LlvmType::Int(32));
        let slot = self.allocate();
        Lowered {
            signed: false,
            value: LlvmValue {
                ty: ty.clone(),
                reg: Some(slot),
                in_reg: false,
                code: vec![
                    Instruction::StringConstant {
                        name: name.clone(),
                        bytes,
                    },
                    Instruction::Call {
                        target: None,
                        func: IrValue::Global("puts".into()),
                        return_type: LlvmType::Int(32),
                        args: vec![TypedValue {
                            ty: LlvmType::Ptr,
                            value: IrValue::Global(name),
                        }],
                        variadic_params: None,
                    },
                    Instruction::Call {
                        target: None,
                        func: IrValue::Global("exit".into()),
                        return_type: LlvmType::Void,
                        args: vec![TypedValue {
                            ty: LlvmType::Int(32),
                            value: IrValue::Integer(-1),
                        }],
                        variadic_params: None,
                    },
                    Instruction::Alloca {
                        target: slot,
                        ty: ty.clone(),
                        count: None,
                        align: None,
                    },
                    Instruction::Store {
                        pointer: IrValue::Reg(slot),
                        value: TypedValue {
                            ty,
                            value: IrValue::ZeroInitializer,
                        },
                        align: None,
                    },
                ],
            },
        }
    }
}
