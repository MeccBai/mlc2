use super::{
    IrGenerator,
    instruction::{Instruction, InstructionBuffer, IrValue, LlvmType, TypedValue},
};
use crate::ast::{
    TypeIndex,
    symbols::PackageSymbolTable,
    types::{CompileType, UnitType},
};

impl IrGenerator {
    pub(super) fn append_drop(
        ty: TypeIndex,
        slot: usize,
        next: &mut usize,
        code: &mut Vec<Instruction>,
        package: &PackageSymbolTable,
    ) {
        match package.get_type(ty).unqualified() {
            CompileType::Ref(reference) if reference.ownership => {
                let pointer = *next;
                *next += 1;
                code.push(Instruction::Load {
                    target: pointer,
                    ty: LlvmType::Ptr,
                    pointer: IrValue::Reg(slot),
                    align: None,
                });
                code.push(Instruction::Call {
                    target: None,
                    func: IrValue::Global("free".into()),
                    return_type: LlvmType::Void,
                    args: vec![TypedValue {
                        ty: LlvmType::Ptr,
                        value: IrValue::Reg(pointer),
                    }],
                    variadic_params: None,
                });
                code.push(Instruction::Store {
                    pointer: IrValue::Reg(slot),
                    value: TypedValue {
                        ty: LlvmType::Ptr,
                        value: IrValue::Null,
                    },
                    align: None,
                });
            }
            CompileType::Unit(unit) => {
                if let Some(body) = &unit.deconstruct {
                    code.push(Instruction::Call {
                        target: None,
                        func: IrValue::Global(body.name.clone()),
                        return_type: LlvmType::Void,
                        args: vec![TypedValue {
                            ty: LlvmType::Ptr,
                            value: IrValue::Reg(slot),
                        }],
                        variadic_params: None,
                    });
                }
            }
            CompileType::List(list) if list.element_type.has_res(package) => {
                for index in (0..list.length).rev() {
                    let element = *next;
                    *next += 1;
                    code.push(Instruction::Gep {
                        target: element,
                        ty: Self::type_lowering(ty, package),
                        pointer: IrValue::Reg(slot),
                        indices: vec![
                            TypedValue {
                                ty: LlvmType::Int(32),
                                value: IrValue::Integer(0),
                            },
                            TypedValue {
                                ty: LlvmType::Int(64),
                                value: IrValue::Integer(index as i128),
                            },
                        ],
                        inbounds: false,
                    });
                    Self::append_drop(list.element_type, element, next, code, package);
                }
            }
            _ => {}
        }
    }
    /// 仅翻译解析阶段构建好的析构体，不重新决定成员清理顺序。
    pub(super) fn add_deconstruct(&mut self, unit: &UnitType, package: &PackageSymbolTable) {
        let Some(body) = &unit.deconstruct else {
            return;
        };
        if !self.remember_symbol(format!("definition::{}", body.name)) {
            return;
        }
        let mut code = Vec::new();
        let mut next = 1;
        if unit.variant.is_some() {
            Self::variant_deconstruct(unit, &mut next, &mut code, package);
        } else { for member in &body.members {
            let slot = next;
            next += 1;
            code.push(Instruction::Gep {
                target: slot,
                ty: Self::unit_lowering(unit, package),
                pointer: IrValue::Reg(0),
                indices: vec![
                    TypedValue {
                        ty: LlvmType::Int(32),
                        value: IrValue::Integer(0),
                    },
                    TypedValue {
                        ty: LlvmType::Int(32),
                        value: IrValue::Integer(member.index as i128),
                    },
                ],
                inbounds: false,
            });
            Self::append_drop(member.ty, slot, &mut next, &mut code, package);
        } }
        code.push(Instruction::Return { value: None });
        self.add_resources(&[], &["free".into()]);
        // pub 不等于 export；合并同一具体类型在不同 OBJ 中的内部生成实现。
        self.bodys.push_str(&format!(
            "define linkonce_odr void {}(ptr %r0) {{\nentry:\n",
            IrValue::Global(body.name.clone())
        ));
        InstructionBuffer { instructions: code }
            .write_to(&mut self.bodys)
            .expect("String write");
        self.bodys.push_str("}\n");
    }
}
