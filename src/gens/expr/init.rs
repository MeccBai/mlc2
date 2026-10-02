use super::*;

impl Expander<'_> {
    pub(super) fn initializer(&mut self, list: &InitialList) -> Lowered {
        let (owner, values) = match list {
            InitialList::Array { ty, values } => (*ty, values),
            InitialList::List {
                onwer: Some(ty),
                values,
            } => (*ty, values),
            InitialList::List { onwer: None, .. } => {
                unsupported("untyped initializer");
            }
            InitialList::String { .. } => {
                unsupported("string storage/layout");
            }
        };
        if owner.is_empty() {
            fail("Checked expression has inconsistent lowering types");
        }
        let (fields, structure) = match self.symbols.get_type(owner).unqualified() {
            CompileType::List(list) => (vec![list.element_type; list.length], false),
            CompileType::Unit(unit) => (unit.members.iter().map(|m| m.member_type).collect(), true),
            _ => fail("Checked expression has inconsistent lowering types"),
        };
        if values.len() != fields.len() {
            fail("Checked expression has inconsistent lowering types");
        }
        let ty = IrGenerator::type_lowering(owner, self.symbols);
        let slot = self.allocate();
        let mut code = vec![Instruction::Alloca {
            target: slot,
            ty: ty.clone(),
            count: None,
            align: None,
        }];
        for (index, (expression, field)) in values.iter().zip(fields).enumerate() {
            let mut value = self.expression(expression).value;
            if value.ty != IrGenerator::type_lowering(field, self.symbols) {
                fail("Checked expression has inconsistent lowering types");
            }
            let operand = self.load(&mut value);
            code.extend(value.code);
            let target = self.allocate();
            code.push(Instruction::Gep {
                target,
                ty: ty.clone(),
                pointer: IrValue::Reg(slot),
                indices: vec![
                    TypedValue {
                        ty: LlvmType::Int(32),
                        value: IrValue::Integer(0),
                    },
                    TypedValue {
                        ty: LlvmType::Int(if structure { 32 } else { 64 }),
                        value: IrValue::Integer(index as i128),
                    },
                ],
                inbounds: false,
            });
            code.push(Instruction::Store {
                pointer: IrValue::Reg(target),
                value: TypedValue {
                    ty: value.ty,
                    value: operand,
                },
                align: None,
            });
        }
        Lowered {
            signed: false,
            value: LlvmValue {
                code,
                ty,
                reg: Some(slot),
                in_reg: false,
            },
        }
    }
}
