//! ABI boundary adapters. Statements and expressions only see logical values.
use super::*;
use crate::gens::{
    LlvmValue,
    error::fail,
    instruction::{Instruction, TypedValue},
};

struct Adapter {
    next: usize,
    code: Vec<Instruction>,
}
impl Adapter {
    fn new(next: usize) -> Self {
        Self { next, code: vec![] }
    }
    fn reg(&mut self) -> usize {
        let reg = self.next;
        self.next = reg
            .checked_add(1)
            .unwrap_or_else(|| fail("ABI adapter register overflow"));
        reg
    }
    fn storage(&mut self, ty: LlvmType) -> LlvmValue {
        let reg = self.reg();
        self.code.push(Instruction::Alloca {
            target: reg,
            ty: ty.clone(),
            count: None,
            align: None,
        });
        LlvmValue {
            code: vec![],
            ty,
            reg: Some(reg),
            in_reg: false,
        }
    }
    fn load(&mut self, value: &LlvmValue) -> IrValue {
        let reg = value.reg.unwrap_or_else(|| fail("Void ABI argument"));
        if value.in_reg {
            return IrValue::Reg(reg);
        }
        let target = self.reg();
        self.code.push(Instruction::Load {
            target,
            ty: value.ty.clone(),
            pointer: IrValue::Reg(reg),
            align: None,
        });
        IrValue::Reg(target)
    }
    fn address(&mut self, value: &LlvmValue) -> IrValue {
        if !value.in_reg {
            return IrValue::Reg(value.reg.unwrap_or_else(|| fail("Void ABI address")));
        }
        let storage = self.storage(value.ty.clone());
        let pointer = IrValue::Reg(storage.reg.unwrap());
        let operand = self.load(value);
        self.code.push(Instruction::Store {
            pointer: pointer.clone(),
            value: TypedValue {
                ty: value.ty.clone(),
                value: operand,
            },
            align: None,
        });
        pointer
    }
    fn offset(&mut self, pointer: IrValue, offset: usize) -> IrValue {
        let target = self.reg();
        self.code.push(Instruction::Gep {
            target,
            ty: LlvmType::Int(8),
            pointer,
            indices: vec![TypedValue {
                ty: LlvmType::Int(64),
                value: IrValue::Integer(offset as i128),
            }],
            inbounds: false,
        });
        IrValue::Reg(target)
    }
    fn split(&mut self, value: &LlvmValue, parts: &[LlvmType]) -> Vec<TypedValue> {
        let pointer = self.address(value);
        let mut offset = 0;
        parts
            .iter()
            .map(|ty| {
                let address = self.offset(pointer.clone(), offset);
                let target = self.reg();
                self.code.push(Instruction::Load {
                    target,
                    ty: ty.clone(),
                    pointer: address,
                    align: Some(1),
                });
                offset += chunk_size(ty).unwrap_or_else(|| fail("Invalid split chunk"));
                TypedValue {
                    ty: ty.clone(),
                    value: IrValue::Reg(target),
                }
            })
            .collect()
    }
    fn merge(&mut self, ty: LlvmType, parts: &[TypedValue]) -> LlvmValue {
        let storage = self.storage(ty);
        let mut offset = 0;
        for part in parts {
            let pointer = self.offset(IrValue::Reg(storage.reg.unwrap()), offset);
            self.code.push(Instruction::Store {
                pointer,
                value: part.clone(),
                align: Some(1),
            });
            offset += chunk_size(&part.ty).unwrap_or_else(|| fail("Invalid split chunk"));
        }
        storage
    }
}

fn chunk_size(ty: &LlvmType) -> Option<usize> {
    match ty {
        LlvmType::Int(bits) if *bits > 0 && bits.is_multiple_of(8) => Some(*bits as usize / 8),
        LlvmType::Float => Some(4),
        LlvmType::Double => Some(8),
        _ => None,
    }
}
fn check_parts(parts: &[LlvmType], size: usize) -> Result<(), DeclarationError> {
    let total = parts
        .iter()
        .try_fold(0usize, |n, ty| n.checked_add(chunk_size(ty)?));
    if parts.len() < 2 || total != Some(size) {
        return Err(DeclarationError::InvalidSplitMapping);
    }
    Ok(())
}

impl LlvmFunc {
    /// ABI classification chooses the chunks; this layer never guesses a target's C ABI.
    /// Chunks cover the source's complete byte representation, including padding.
    pub fn with_split_parameter(
        mut self,
        index: usize,
        parts: Vec<LlvmType>,
        package: &PackageSymbolTable,
    ) -> Result<Self, DeclarationError> {
        let param = self
            .params
            .get_mut(index)
            .ok_or(DeclarationError::InvalidSplitMapping)?;
        check_parts(&parts, param.source.size(package.arenas()))?;
        param.lowered = parts
            .into_iter()
            .map(|ty| AbiParameter {
                ty,
                mode: ParameterMode::Direct,
                align: 1,
            })
            .collect();
        Ok(self)
    }
    pub fn with_split_return(
        mut self,
        parts: Vec<LlvmType>,
        package: &PackageSymbolTable,
    ) -> Result<Self, DeclarationError> {
        let source = self
            .result
            .source
            .ok_or(DeclarationError::InvalidSplitMapping)?;
        check_parts(&parts, source.size(package.arenas()))?;
        self.result.hidden.clear();
        self.result.ty = LlvmType::Struct(parts.clone());
        self.result.parts = parts;
        Ok(self)
    }

    /// Evaluate/pack source arguments in order, call, then unpack the source return.
    pub fn call_values(
        &self,
        start: usize,
        values: &[LlvmValue],
        package: &PackageSymbolTable,
    ) -> (usize, LlvmValue) {
        let mappings: Vec<_> = self.receiver.iter().chain(&self.params).collect();
        if values.len() < mappings.len() || (!self.variadic && values.len() != mappings.len()) {
            fail("Checked call argument count mismatch");
        }
        let mut adapter = Adapter::new(start);
        let mut args = vec![];
        for (index, value) in values.iter().enumerate() {
            adapter.code.extend(value.code.clone());
            let Some(mapping) = mappings.get(index) else {
                let operand = adapter.load(value);
                args.push(TypedValue {
                    ty: value.ty.clone(),
                    value: operand,
                });
                continue;
            };
            let logical = IrGenerator::type_lowering(mapping.source, package);
            if mapping.lowered.len() > 1 {
                if value.ty != logical {
                    fail("Split argument source type mismatch");
                }
                let parts: Vec<_> = mapping.lowered.iter().map(|part| part.ty.clone()).collect();
                check_parts(&parts, mapping.source.size(package.arenas()))
                    .unwrap_or_else(|e| e.abort_generation());
                args.extend(adapter.split(value, &parts));
            } else {
                let part = mapping
                    .lowered
                    .first()
                    .unwrap_or_else(|| fail("Empty parameter mapping"));
                let receiver_ref =
                    part.mode == ParameterMode::Receiver && value.ty == LlvmType::Ptr;
                if !receiver_ref && value.ty != logical {
                    fail("Checked argument source type mismatch");
                }
                let operand = if receiver_ref || part.mode == ParameterMode::Direct {
                    adapter.load(value)
                } else {
                    adapter.address(value)
                };
                args.push(TypedValue {
                    ty: part.llvm_type(),
                    value: operand,
                });
            }
        }
        let (next, mut result) = self
            .call(adapter.next, &args)
            .unwrap_or_else(|e| fail(&e.to_string()));
        adapter.next = next;
        adapter.code.append(&mut result.code);
        if !self.result.parts.is_empty() {
            let aggregate = TypedValue {
                ty: self.result.ty.clone(),
                value: IrValue::Reg(result.reg.unwrap()),
            };
            let parts: Vec<_> = self
                .result
                .parts
                .iter()
                .enumerate()
                .map(|(index, ty)| {
                    let target = adapter.reg();
                    adapter.code.push(Instruction::ExtractValue {
                        target,
                        aggregate: aggregate.clone(),
                        index,
                    });
                    TypedValue {
                        ty: ty.clone(),
                        value: IrValue::Reg(target),
                    }
                })
                .collect();
            result = adapter.merge(
                IrGenerator::type_lowering(self.result.source.unwrap(), package),
                &parts,
            );
        }
        result.code = adapter.code;
        (adapter.next, result)
    }

    pub(in crate::gens) fn merge_parameter(
        start: usize,
        source: TypeIndex,
        parts: &[TypedValue],
        package: &PackageSymbolTable,
    ) -> (usize, LlvmValue) {
        check_parts(
            &parts.iter().map(|p| p.ty.clone()).collect::<Vec<_>>(),
            source.size(package.arenas()),
        )
        .unwrap_or_else(|e| e.abort_generation());
        let mut adapter = Adapter::new(start);
        let mut value = adapter.merge(IrGenerator::type_lowering(source, package), parts);
        value.code = adapter.code;
        (adapter.next, value)
    }

    pub(in crate::gens) fn encode_return(
        &self,
        start: usize,
        value: &LlvmValue,
    ) -> (usize, Vec<Instruction>, TypedValue) {
        let mut adapter = Adapter::new(start);
        let parts = adapter.split(value, &self.result.parts);
        let mut aggregate = TypedValue {
            ty: self.result.ty.clone(),
            value: IrValue::ZeroInitializer,
        };
        for (index, part) in parts.into_iter().enumerate() {
            let target = adapter.reg();
            adapter.code.push(Instruction::InsertValue {
                target,
                aggregate: aggregate.clone(),
                value: part,
                index,
            });
            aggregate.value = IrValue::Reg(target);
        }
        (adapter.next, adapter.code, aggregate)
    }
}
