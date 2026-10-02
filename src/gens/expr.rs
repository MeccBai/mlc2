use super::error::{fail, unsupported};
use super::{
    IrGenerator, LlvmValue,
    instruction::{Instruction, IrValue, LlvmType, TypedValue},
};
use crate::ast::{
    arena::TypeIndex,
    expression::{CompAtom, ConstValue, Expression, InitialList, UnaryExpr},
    symbols::PackageSymbolTable,
    types::{CompileType, base_type::DataType},
};
use std::collections::HashMap;

mod access;
mod calls;
mod composite;
mod init;

#[cfg(test)]
mod tests;

pub type ExpressionBindings = HashMap<String, LlvmValue>;

struct Lowered {
    value: LlvmValue,
    signed: bool,
}

struct Expander<'a> {
    next: usize,
    symbols: &'a PackageSymbolTable,
    bindings: &'a ExpressionBindings,
}

impl IrGenerator {
    pub(crate) fn expression_type(
        expr: &Expression,
        symbols: &PackageSymbolTable,
    ) -> Option<TypeIndex> {
        let bindings = ExpressionBindings::new();
        Expander {
            next: 0,
            symbols,
            bindings: &bindings,
        }
        .expression_type(expr)
    }

    pub fn expression_expand(
        start_reg: usize,
        expr: &Expression,
        symbols: &PackageSymbolTable,
        bindings: &ExpressionBindings,
    ) -> (usize, LlvmValue) {
        let mut expander = Expander {
            next: start_reg,
            symbols,
            bindings,
        };
        let result = expander.expression(expr);
        (expander.next, result.value)
    }
}

impl Expander<'_> {
    fn allocate(&mut self) -> usize {
        let reg = self.next;
        self.next = reg
            .checked_add(1)
            .unwrap_or_else(|| fail("LLVM register identity space exhausted"));
        reg
    }

    fn signed(&self, ty: TypeIndex) -> bool {
        matches!(self.symbols.get_type(ty).unqualified(), CompileType::Base(base)
            if base.data_type() == DataType::Integer && base.signed())
    }

    fn expression(&mut self, expr: &Expression) -> Lowered {
        match expr {
            Expression::Poison => fail("Poisoned AST reached expression lowering"),
            Expression::ConstValueE(value) => self.constant(value),
            Expression::VarValueE(var) => {
                let binding = self.bindings.get(&var.name).unwrap_or_else(|| {
                    fail(&format!(
                        "Missing LLVM variable binding: {}",
                        var.name.clone()
                    ))
                });
                if binding.ty != IrGenerator::type_lowering(var.var_type, self.symbols) {
                    fail("Checked expression has inconsistent lowering types");
                }
                Lowered {
                    signed: self.signed(var.var_type),
                    value: LlvmValue {
                        code: vec![],
                        ty: binding.ty.clone(),
                        reg: binding.reg,
                        in_reg: binding.in_reg,
                    },
                }
            }
            Expression::CompositeE(c) => self.composite(c),
            Expression::FuncCallE(call) => self.call(call),
            Expression::UnaryExprE(UnaryExpr::Operator { op, value }) => self.unary(op, value),
            Expression::UnaryExprE(UnaryExpr::Access(access)) => self.access(access),
            Expression::InitListE(list) => self.initializer(list),
        }
    }

    fn atom(&mut self, atom: &CompAtom) -> Lowered {
        match atom {
            CompAtom::Poison => fail("Poisoned AST reached expression lowering"),
            CompAtom::ConstValueA(v) => self.constant(v),
            CompAtom::CompositeA(c) => self.composite(c),
            CompAtom::FuncCallA(c) => self.call(c),
            CompAtom::VarValueA(v) => self.expression(&Expression::VarValueE(v.clone())),
            CompAtom::UnaryExprA(UnaryExpr::Operator { op, value }) => self.unary(op, value),
            CompAtom::UnaryExprA(UnaryExpr::Access(access)) => self.access(access),
        }
    }

    fn constant(&mut self, constant: &ConstValue) -> Lowered {
        if constant.ty.is_empty() {
            unsupported("untyped null");
        }
        let ty = IrGenerator::type_lowering(constant.ty, self.symbols);
        let invalid = || -> ! { fail(&format!("Invalid checked literal: {}", constant.value)) };
        let value = match ty {
            LlvmType::Int(1) => IrValue::Bool(match constant.value.as_str() {
                "true" | "1" => true,
                "false" | "0" => false,
                _ => return invalid(),
            }),
            LlvmType::Int(_) => {
                IrValue::Integer(constant.value.parse().unwrap_or_else(|_| invalid()))
            }
            LlvmType::Float | LlvmType::Double => {
                let number: f64 = constant.value.parse().unwrap_or_else(|_| invalid());
                let number = if ty == LlvmType::Float {
                    number as f32 as f64
                } else {
                    number
                };
                IrValue::Val(format!("0x{:016X}", number.to_bits()))
            }
            _ => {
                unsupported("non-scalar literal");
            }
        };

        let reg = self.allocate();
        Lowered {
            signed: self.signed(constant.ty),
            value: LlvmValue {
                code: vec![
                    Instruction::Alloca {
                        target: reg,
                        ty: ty.clone(),
                        count: None,
                        align: None,
                    },
                    Instruction::Store {
                        pointer: IrValue::Reg(reg),
                        value: TypedValue {
                            ty: ty.clone(),
                            value,
                        },
                        align: None,
                    },
                ],
                ty,
                reg: Some(reg),
                in_reg: false,
            },
        }
    }

    fn load(&mut self, value: &mut LlvmValue) -> IrValue {
        let reg = value.reg.unwrap_or_else(|| unsupported("void operand"));
        if value.in_reg {
            return IrValue::Reg(reg);
        }
        let target = self.allocate();
        value.code.push(Instruction::Load {
            target,
            ty: value.ty.clone(),
            pointer: IrValue::Reg(reg),
            align: None,
        });
        value.reg = Some(target);
        value.in_reg = true;
        IrValue::Reg(target)
    }
}
