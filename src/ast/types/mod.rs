use std::sync::Arc;
pub(crate) mod base_type;
pub(crate) use base_type::BaseType;

pub(crate) mod ref_type;
pub(crate) use ref_type::RefType;

pub(crate) mod unit_type;
pub(crate) use unit_type::UnitType;

pub(crate) mod list_type;
pub(crate) use list_type::ListType;
pub(crate) mod enum_type;

pub(crate) use enum_type::EnumType;

use crate::ast::{GenericIndex, TypeArena, TypeIndex};
use lasso::Rodeo;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub enum CompileType {
    Base(BaseType),
    Ref(RefType),
    Unit(UnitType),
    List(ListType),
    Enum(EnumType),
    Generic(GenericIndex),
}

impl TypeArena {
    pub fn new(interner: &mut Rodeo) -> Self {
        let mut arena = Self::empty();
        for ty in BaseType::base_types() {
            let name = match &ty {
                CompileType::Base(base) => interner.get_or_intern(base.name()),
                _ => unreachable!(),
            };
            arena.insert(name, ty);
        }
        arena
    }
}
