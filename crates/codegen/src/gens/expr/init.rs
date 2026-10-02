use super::*;
use crate::gens::memory;

impl Expander<'_> {
    pub(super) fn initializer(&mut self, list: &InitialList) -> Lowered {
        self.initializer_expected(list, None)
    }

    pub(super) fn initializer_expected(
        &mut self,
        list: &InitialList,
        expected: Option<TypeIndex>,
    ) -> Lowered {
        if let InitialList::String { value } = list {
            return self.string(value, expected);
        }
        let (owner, values) = match list {
            InitialList::Array { ty, values } => (*ty, values),
            InitialList::List { onwer, values } => (
                onwer
                    .or(expected)
                    .unwrap_or_else(|| fail("Initializer has no checked destination type")),
                values,
            ),
            InitialList::String { .. } => unreachable!(),
        };
        if owner.is_empty() {
            fail("Initializer has an empty destination type");
        }
        let (fields, structure) = match self.symbols.get_type(owner).unqualified() {
            CompileType::List(list) => (vec![list.element_type; list.length], false),
            CompileType::Unit(unit) => (unit.members.iter().map(|m| m.member_type).collect(), true),
            _ => fail("Initializer destination is not an aggregate"),
        };
        if values.len() != fields.len() {
            fail("Checked initializer field count mismatch");
        }
        let ty = IrGenerator::type_lowering(owner, self.symbols);
        let slot = self.allocate();
        let mut code = vec![Instruction::Alloca {
            target: slot,
            ty: ty.clone(),
            count: None,
            align: Some(owner.align(self.symbols.arenas())),
        }];
        if !structure {
            code.push(memory::zero(
                IrValue::Reg(slot),
                owner.size(self.symbols.arenas()),
            ));
        }
        for (index, (expression, field)) in values.iter().zip(fields).enumerate() {
            let mut value = self.expression_expected(expression, Some(field)).value;
            let operand = self.load(&mut value);
            let field_ty = IrGenerator::type_lowering(field, self.symbols);
            let operand = self.convert_initializer(&mut value, operand, &field_ty, expression);
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
                    ty: field_ty,
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

    fn convert_initializer(
        &mut self,
        value: &mut LlvmValue,
        operand: IrValue,
        to: &LlvmType,
        expression: &Expression,
    ) -> IrValue {
        use crate::gens::instruction::Cast;
        if &value.ty == to {
            return operand;
        }
        let signed = self
            .expression_type(expression)
            .is_some_and(|ty| self.signed(ty));
        let op = match (&value.ty, to) {
            (LlvmType::Int(a), LlvmType::Int(b)) if a > b => Cast::Trunc,
            (LlvmType::Int(_), LlvmType::Int(_)) if signed => Cast::SExt,
            (LlvmType::Int(_), LlvmType::Int(_)) => Cast::ZExt,
            (LlvmType::Float, LlvmType::Double) => Cast::FPExt,
            (LlvmType::Double, LlvmType::Float) => Cast::FPTrunc,
            _ => fail("Checked initializer field type mismatch"),
        };
        let target = self.allocate();
        value.code.push(Instruction::Cast {
            target,
            op,
            value: TypedValue {
                ty: value.ty.clone(),
                value: operand,
            },
            to: to.clone(),
        });
        IrValue::Reg(target)
    }

    fn string(&mut self, value: &str, expected: Option<TypeIndex>) -> Lowered {
        // The frontend represents strings as byte arrays, without an implicit NUL.
        let bytes = value.as_bytes().to_vec();
        let name = memory::string_name(&bytes);
        let length = expected
            .map(|ty| match self.symbols.get_type(ty).unqualified() {
                CompileType::List(array) => array.length,
                _ => fail("Checked string destination is not an array"),
            })
            .unwrap_or(bytes.len());
        if length < bytes.len() {
            fail("Checked string exceeds destination capacity");
        }
        let ty = LlvmType::Array {
            element: Box::new(LlvmType::Int(8)),
            length,
        };
        let slot = self.allocate();
        let mut code = vec![
            Instruction::StringConstant {
                name: name.clone(),
                bytes: bytes.clone(),
            },
            Instruction::Alloca {
                target: slot,
                ty: ty.clone(),
                count: None,
                align: Some(1),
            },
        ];
        if length > bytes.len() {
            code.push(memory::zero(IrValue::Reg(slot), length));
        }
        code.push(memory::copy(
            IrValue::Reg(slot),
            IrValue::Global(name),
            bytes.len(),
        ));
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
