use super::*;
use crate::ast::expression::{Composite, operators::Operator as AstOp};
use crate::gens::instruction::{Condition, FloatCondition, Operator as IrOp};

impl Expander<'_> {
    pub(super) fn composite(&mut self, c: &Composite) -> Lowered {
        if c.members.is_empty() || c.members.len() != c.operators.len() + 1 {
            fail("Malformed checked Composite");
        }
        let mut cursor = 0;
        self.climb(c, &mut cursor, 0)
    }

    fn climb(&mut self, c: &Composite, cursor: &mut usize, min: u8) -> Lowered {
        let mut left = self.atom(&c.members[*cursor]);
        while let Some(op) = c.operators.get(*cursor) {
            let priority = op.precedence();
            if priority < min {
                break;
            }
            // Snapshot the left operand before evaluating a potentially side-effecting RHS.
            self.load(&mut left.value);
            *cursor += 1;
            // +1 implements left associativity for equal-precedence operators.
            let right = self.climb(c, cursor, priority + 1);
            left = self.binary(left, op, right);
        }
        left
    }

    fn binary(&mut self, mut left: Lowered, op: &AstOp, mut right: Lowered) -> Lowered {
        if left.value.ty != right.value.ty || left.signed != right.signed {
            fail("Checked expression has inconsistent lowering types");
        }
        if matches!(op, AstOp::LogicalAnd | AstOp::LogicalOr) {
            return self.short_circuit(left, op, right);
        }
        let ty = left.value.ty.clone();
        let float = matches!(ty, LlvmType::Float | LlvmType::Double);
        if !float && !matches!(ty, LlvmType::Int(_)) {
            unsupported("non-numeric binary operands");
        }
        let lhs = self.load(&mut left.value);
        let rhs = self.load(&mut right.value);
        let target = self.allocate();
        let (instruction, result_ty) = if let Some(condition) = comparison(op, left.signed) {
            let instruction = if float {
                Instruction::FloatCompare {
                    target,
                    condition: float_comparison(op).unwrap(),
                    ty,
                    lhs,
                    rhs,
                }
            } else {
                Instruction::Compare {
                    target,
                    condition,
                    ty,
                    lhs,
                    rhs,
                }
            };
            (instruction, LlvmType::Int(1))
        } else {
            let operation = operation(op, float, left.signed);
            (
                Instruction::Operation {
                    target,
                    op: operation,
                    ty: ty.clone(),
                    lhs,
                    rhs,
                },
                ty,
            )
        };
        left.value.code.extend(right.value.code);
        left.value.code.push(instruction);
        Lowered {
            signed: left.signed && result_ty != LlvmType::Int(1),
            value: LlvmValue {
                code: left.value.code,
                ty: result_ty,
                reg: Some(target),
                in_reg: true,
            },
        }
    }

    fn short_circuit(&mut self, mut left: Lowered, op: &AstOp, mut right: Lowered) -> Lowered {
        if left.value.ty != LlvmType::Int(1) {
            fail("Checked expression has inconsistent lowering types");
        }
        let lhs = self.load(&mut left.value);
        let rhs = self.load(&mut right.value);
        let slot = self.allocate();
        let target = self.allocate();
        let rhs_label = format!("expr.r{slot}.rhs");
        let short_label = format!("expr.r{slot}.short");
        let end_label = format!("expr.r{slot}.end");
        let is_and = *op == AstOp::LogicalAnd;
        let mut code = left.value.code;
        code.push(Instruction::Alloca {
            target: slot,
            ty: LlvmType::Int(1),
            count: None,
            align: None,
        });
        code.push(Instruction::ConditionalBranch {
            condition: lhs,
            then_label: if is_and {
                rhs_label.clone()
            } else {
                short_label.clone()
            },
            else_label: if is_and {
                short_label.clone()
            } else {
                rhs_label.clone()
            },
        });
        code.push(Instruction::BasicBlock { label: rhs_label });
        code.extend(right.value.code);
        code.push(Instruction::Store {
            pointer: IrValue::Reg(slot),
            value: TypedValue {
                ty: LlvmType::Int(1),
                value: rhs,
            },
            align: None,
        });
        code.push(Instruction::Branch {
            label: end_label.clone(),
        });
        code.push(Instruction::BasicBlock { label: short_label });
        code.push(Instruction::Store {
            pointer: IrValue::Reg(slot),
            value: TypedValue {
                ty: LlvmType::Int(1),
                value: IrValue::Bool(!is_and),
            },
            align: None,
        });
        code.push(Instruction::Branch {
            label: end_label.clone(),
        });
        code.push(Instruction::BasicBlock { label: end_label });
        code.push(Instruction::Load {
            target,
            ty: LlvmType::Int(1),
            pointer: IrValue::Reg(slot),
            align: None,
        });
        Lowered {
            signed: false,
            value: LlvmValue {
                code,
                ty: LlvmType::Int(1),
                reg: Some(target),
                in_reg: true,
            },
        }
    }

    pub(super) fn unary(&mut self, op: &AstOp, atom: &CompAtom) -> Lowered {
        let mut operand = self.atom(atom);
        if *op == AstOp::Dereference {
            let source = self
                .atom_type(atom)
                .unwrap_or_else(|| fail("Checked expression has inconsistent lowering types"));
            let CompileType::Ref(reference) = self.symbols.get_type(source).unqualified() else {
                fail("Checked expression has inconsistent lowering types");
            };
            let base = reference.base;
            self.load(&mut operand.value);
            operand.value.ty = if reference.deref_reference().is_some() {
                LlvmType::Ptr
            } else if reference.level == 1 {
                IrGenerator::type_lowering(base, self.symbols)
            } else {
                fail("Checked reference has zero indirection levels");
            };
            operand.value.in_reg = false;
            operand.signed = reference.level == 1 && self.signed(base);
            return operand;
        }
        if matches!(op, AstOp::AddressOf | AstOp::MutOf) {
            if operand.value.in_reg {
                unsupported("address of SSA-only value");
            }
            operand.value.ty = LlvmType::Ptr;
            operand.value.in_reg = true;
            operand.signed = false;
            return operand;
        }
        let rhs = self.load(&mut operand.value);
        let ty = operand.value.ty.clone();
        let float = matches!(ty, LlvmType::Float | LlvmType::Double);
        let target = self.allocate();
        let instruction = match op {
            AstOp::Negate if float => Instruction::Operation {
                target,
                op: IrOp::FSub,
                ty: ty.clone(),
                lhs: IrValue::Val("0x8000000000000000".into()),
                rhs,
            },
            AstOp::Negate if matches!(ty, LlvmType::Int(_)) => Instruction::Operation {
                target,
                op: IrOp::Sub,
                ty: ty.clone(),
                lhs: IrValue::Integer(0),
                rhs,
            },
            AstOp::BitNot if matches!(ty, LlvmType::Int(_)) => Instruction::Operation {
                target,
                op: IrOp::Xor,
                ty: ty.clone(),
                lhs: IrValue::Integer(-1),
                rhs,
            },
            AstOp::LogicalNot if ty == LlvmType::Int(1) => Instruction::Operation {
                target,
                op: IrOp::Xor,
                ty: ty.clone(),
                lhs: IrValue::Bool(true),
                rhs,
            },
            _ => {
                unsupported("unary operator");
            }
        };
        operand.value.code.push(instruction);
        operand.value.reg = Some(target);
        operand.value.in_reg = true;
        operand
    }
}

fn operation(op: &AstOp, float: bool, signed: bool) -> IrOp {
    use AstOp::*;
    match (op, float, signed) {
        (Add, true, _) => IrOp::FAdd,
        (Subtract, true, _) => IrOp::FSub,
        (Multiply, true, _) => IrOp::FMul,
        (Divide, true, _) => IrOp::FDiv,
        (Remainder, true, _) => IrOp::FRem,
        (Add, false, _) => IrOp::Add,
        (Subtract, false, _) => IrOp::Sub,
        (Multiply, false, _) => IrOp::Mul,
        (Divide, false, true) => IrOp::SDiv,
        (Divide, false, false) => IrOp::UDiv,
        (Remainder, false, true) => IrOp::SRem,
        (Remainder, false, false) => IrOp::URem,
        (BitAnd, false, _) => IrOp::And,
        (BitOr, false, _) => IrOp::Or,
        (BitXor, false, _) => IrOp::Xor,
        (ShiftLeft, false, _) => IrOp::Shl,
        (ShiftRight, false, true) => IrOp::AShr,
        (ShiftRight, false, false) => IrOp::LShr,
        _ => {
            unsupported("binary operator");
        }
    }
}

fn comparison(op: &AstOp, signed: bool) -> Option<Condition> {
    use AstOp::*;
    Some(match (op, signed) {
        (Equal, _) => Condition::Eq,
        (NotEqual, _) => Condition::Ne,
        (Less, true) => Condition::SLt,
        (LessOrEqual, true) => Condition::SLe,
        (Greater, true) => Condition::SGt,
        (GreaterOrEqual, true) => Condition::SGe,
        (Less, false) => Condition::ULt,
        (LessOrEqual, false) => Condition::ULe,
        (Greater, false) => Condition::UGt,
        (GreaterOrEqual, false) => Condition::UGe,
        _ => return None,
    })
}

fn float_comparison(op: &AstOp) -> Option<FloatCondition> {
    use AstOp::*;
    Some(match op {
        Equal => FloatCondition::OEq,
        NotEqual => FloatCondition::UNe,
        Less => FloatCondition::OLt,
        LessOrEqual => FloatCondition::OLe,
        Greater => FloatCondition::OGt,
        GreaterOrEqual => FloatCondition::OGe,
        _ => return None,
    })
}
