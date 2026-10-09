use super::*;
use crate::ast::expression::{Access, InitialList, operators::Operator};
use crate::ast::symbols::EnumBool;

impl Expander<'_> {
    pub(super) fn expression_type(&self, expr: &Expression) -> Option<TypeIndex> {
        match expr {
            Expression::ConstValueE(v) => Some(v.ty),
            Expression::VarValueE(v) => Some(v.var_type),
            Expression::CompositeE(c) => self.composite_type(c),
            Expression::FuncCallE(call) => match call.func {
                EnumBool::True(i) => self.symbols.get_interface(i, false).ret_type,
                EnumBool::False(i) => self.symbols.get_function(i, false).ret_type,
            },
            Expression::UnaryExprE(unary) => self.unary_type(unary),
            Expression::InitListE(InitialList::Array { ty, .. }) => Some(*ty),
            Expression::InitListE(InitialList::List { onwer, .. }) => *onwer,
            _ => None,
        }
    }

    pub(super) fn atom_type(&self, atom: &CompAtom) -> Option<TypeIndex> {
        match atom {
            CompAtom::ConstValueA(v) => Some(v.ty),
            CompAtom::VarValueA(v) => Some(v.var_type),
            CompAtom::CompositeA(c) => self.composite_type(c),
            CompAtom::FuncCallA(c) => self.expression_type(&Expression::FuncCallE(c.clone())),
            CompAtom::UnaryExprA(u) => self.unary_type(u),
            CompAtom::Poison => None,
        }
    }

    fn composite_type(&self, composite: &crate::ast::expression::Composite) -> Option<TypeIndex> {
        let source = self.atom_type(composite.members.first()?)?;
        if composite
            .operators
            .first()
            .is_some_and(|op| op.changes_binary_result_type())
        {
            Some(
                self.symbols
                    .file(source.file_id())?
                    .get_base(DataType::Boolean, 8, false),
            )
        } else {
            Some(source)
        }
    }

    fn unary_type(&self, unary: &UnaryExpr) -> Option<TypeIndex> {
        match unary {
            UnaryExpr::Function { ty, .. } => Some(*ty),
            UnaryExpr::Operator {
                op: Operator::Dereference,
                value,
            } => match self.symbols.get_type(self.atom_type(value)?).unqualified() {
                CompileType::Ref(reference) => match reference.deref_reference() {
                    Some(remaining) => {
                        let name = remaining.format(self.symbols);
                        self.symbols
                            .arenas()
                            .iter()
                            .find_map(|(_, file)| file.types.get_by_name(&name))
                    }
                    None if reference.level == 1 => Some(reference.base),
                    None => None,
                },
                _ => None,
            },
            UnaryExpr::Operator {
                op: Operator::AddressOf | Operator::MutOf,
                ..
            } => None,
            UnaryExpr::Operator { value, .. } => self.atom_type(value),
            UnaryExpr::Access(access) => self.access_type(access).map(|(_, ty)| ty),
        }
    }

    fn access_type(&self, access: &Access) -> Option<(TypeIndex, TypeIndex)> {
        match access {
            Access::Member {
                base,
                indirect,
                name,
            } => {
                let mut owner = self.atom_type(base)?;
                if *indirect {
                    if let CompileType::Ref(reference) = self.symbols.get_type(owner).unqualified()
                    {
                        owner = reference.base;
                    }
                    // self uses -> even though its frontend type is the owner unit.
                }
                let CompileType::Unit(unit) = self.symbols.get_type(owner).unqualified() else {
                    return None;
                };
                Some((
                    owner,
                    unit.members.iter().find(|m| m.name == *name)?.member_type,
                ))
            }
            Access::Index { base, .. } => {
                let owner = self.expression_type(base)?;
                match self.symbols.get_type(owner).unqualified() {
                    CompileType::List(list) => Some((owner, list.element_type)),
                    CompileType::Ref(reference) if reference.level == 1 => {
                        Some((reference.base, reference.base))
                    }
                    CompileType::Ref(reference) => {
                        let name = reference.deref_reference()?.format(self.symbols);
                        let element = self
                            .symbols
                            .arenas()
                            .iter()
                            .find_map(|(_, file)| file.types.get_by_name(&name))?;
                        Some((element, element))
                    }
                    _ => None,
                }
            }
        }
    }

    pub(super) fn address(&mut self, value: &mut LlvmValue) -> IrValue {
        let reg = value.reg.unwrap_or_else(|| unsupported("void address"));
        if !value.in_reg {
            return IrValue::Reg(reg);
        }
        let slot = self.allocate();
        value.code.push(Instruction::Alloca {
            target: slot,
            ty: value.ty.clone(),
            count: None,
            align: None,
        });
        value.code.push(Instruction::Store {
            pointer: IrValue::Reg(slot),
            value: TypedValue {
                ty: value.ty.clone(),
                value: IrValue::Reg(reg),
            },
            align: None,
        });
        value.reg = Some(slot);
        value.in_reg = false;
        IrValue::Reg(slot)
    }

    pub(super) fn access(&mut self, access: &Access) -> Lowered {
        let (owner, result) = self
            .access_type(access)
            .unwrap_or_else(|| fail("Checked expression has inconsistent lowering types"));
        let (mut base, index, index_code) = match access {
            Access::Member {
                base,
                indirect,
                name,
            } => {
                let source = self
                    .atom_type(base)
                    .unwrap_or_else(|| fail("Checked expression has inconsistent lowering types"));
                let mut value = self.atom(base).value;
                if *indirect
                    && matches!(
                        self.symbols.get_type(source).unqualified(),
                        CompileType::Ref(_)
                    )
                {
                    self.load(&mut value);
                    value.in_reg = false;
                    value.ty = IrGenerator::type_lowering(owner, self.symbols);
                }
                let CompileType::Unit(unit) = self.symbols.get_type(owner).unqualified() else {
                    fail("Checked expression has inconsistent lowering types");
                };
                let position = unit
                    .members
                    .iter()
                    .position(|m| m.name == *name)
                    .unwrap_or_else(|| fail("Checked expression has inconsistent lowering types"));
                (
                    value,
                    TypedValue {
                        ty: LlvmType::Int(32),
                        value: IrValue::Integer(position as i128),
                    },
                    vec![],
                )
            }
            Access::Index { base, index } => {
                let source = self
                    .expression_type(base)
                    .unwrap_or_else(|| fail("Missing index base type"));
                let is_reference = matches!(
                    self.symbols.get_type(source).unqualified(),
                    CompileType::Ref(_)
                );
                let mut base = self.expression(base).value;
                if is_reference {
                    self.load(&mut base);
                    base.in_reg = false;
                } else {
                    self.address(&mut base);
                }
                let mut index = self.expression(index).value;
                if !matches!(index.ty, LlvmType::Int(_)) {
                    fail("Checked expression has inconsistent lowering types");
                }
                let reg = self.load(&mut index);
                (
                    base,
                    TypedValue {
                        ty: index.ty,
                        value: reg,
                    },
                    index.code,
                )
            }
        };
        let pointer = self.address(&mut base);
        base.code.extend(index_code);
        let target = self.allocate();
        let pointer_index = matches!(access, Access::Index { base, .. }
            if self.expression_type(base).is_some_and(|ty|
                matches!(self.symbols.get_type(ty).unqualified(), CompileType::Ref(_))));
        let indices = if pointer_index {
            vec![index]
        } else {
            vec![
                TypedValue {
                    ty: LlvmType::Int(32),
                    value: IrValue::Integer(0),
                },
                index,
            ]
        };
        base.code.push(Instruction::Gep {
            target,
            ty: IrGenerator::type_lowering(owner, self.symbols),
            pointer,
            indices,
            inbounds: false,
        });
        Lowered {
            signed: self.signed(result),
            value: LlvmValue {
                code: base.code,
                ty: IrGenerator::type_lowering(result, self.symbols),
                reg: Some(target),
                in_reg: false,
            },
        }
    }
}
