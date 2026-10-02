use crate::{
    ast::{
        arena::TypeIndex,
        symbols::PackageSymbolTable,
        types::{
            BaseType,
            CompileType::{Base, List, Ref, Unit},
            ListType, UnitType,
            base_type::DataType,
        },
    },
    gens::error::{fail, unsupported},
    gens::instruction::LlvmType,
};

use std::{
    collections::{HashMap, HashSet},
    sync::LazyLock,
};

use super::IrGenerator;

impl IrGenerator {
    pub fn base_lowering(base: &BaseType) -> LlvmType {
        match base.data_type() {
            DataType::Integer => LlvmType::Int(base.bits() as u32),
            DataType::Float => match base.bits() {
                32 => LlvmType::Float,
                64 => LlvmType::Double,
                _ => unsupported(format!("Unsupported float size: {}", base.size()).as_str()),
            },
            DataType::Boolean => LlvmType::Int(1),
        }
    }

    pub fn list_lowering(list: &ListType, symbols: &PackageSymbolTable) -> LlvmType {
        let element = Self::type_lowering(list.element_type.clone(), symbols);
        LlvmType::Array {
            element: Box::new(element),
            length: list.length,
        }
    }

    pub fn unit_lowering(unit: &UnitType, symbols: &PackageSymbolTable) -> LlvmType {
        LlvmType::Named(format!("struct.{}", unit.name))
    }

    pub fn type_lowering(index: TypeIndex, symbols: &PackageSymbolTable) -> LlvmType {
        let ty = symbols.get_type(index);
        match ty.unqualified() {
            Base(base) => Self::base_lowering(base),
            Ref(ref_ty) => LlvmType::Ptr,
            List(list) => Self::list_lowering(list, symbols),
            Unit(unit) => Self::unit_lowering(unit, symbols),
            _ => fail("Generic type cannot to be lowering!"),
        }
    }
}
