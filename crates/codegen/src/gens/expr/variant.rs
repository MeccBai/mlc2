use super::*;

impl Expander<'_> {
    pub(super) fn variant_initializer(
        &mut self,
        owner: TypeIndex,
        values: &[Expression],
    ) -> Lowered {
        let CompileType::Unit(unit) = self.symbols.get_type(owner).unqualified() else {
            unreachable!()
        };
        let variant = unit.variant.as_ref().expect("checked union initializer");
        let source = self
            .expression_type(&values[0])
            .unwrap_or_else(|| fail("Missing union candidate type"));
        let tag = variant
            .candidates
            .iter()
            .position(|candidate| {
                self.symbols
                    .get_type(*candidate)
                    .unqualified()
                    .format(self.symbols)
                    == self
                        .symbols
                        .get_type(source)
                        .unqualified()
                        .format(self.symbols)
            })
            .unwrap_or_else(|| fail("Unchecked union candidate"));
        let candidate = variant.candidates[tag];
        let ty = IrGenerator::type_lowering(owner, self.symbols);
        let slot = self.allocate();
        let mut code = vec![
            Instruction::Alloca {
                target: slot,
                ty: ty.clone(),
                count: None,
                align: Some(owner.align(self.symbols)),
            },
            Instruction::Store {
                pointer: IrValue::Reg(slot),
                value: TypedValue {
                    ty: ty.clone(),
                    value: IrValue::ZeroInitializer,
                },
                align: None,
            },
        ];
        let mut value = self.expression_expected(&values[0], Some(candidate)).value;
        let operand = self.load(&mut value);
        code.extend(value.code);
        for (field, value) in [
            (
                0,
                TypedValue {
                    ty: LlvmType::Int(32),
                    value: IrValue::Integer(tag as i128),
                },
            ),
            (
                1,
                TypedValue {
                    ty: IrGenerator::type_lowering(candidate, self.symbols),
                    value: operand,
                },
            ),
        ] {
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
                        ty: LlvmType::Int(32),
                        value: IrValue::Integer(field),
                    },
                ],
                inbounds: false,
            });
            code.push(Instruction::Store {
                pointer: IrValue::Reg(target),
                value,
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
