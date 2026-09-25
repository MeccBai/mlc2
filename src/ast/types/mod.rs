use std::collections::HashMap;
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

use crate::ast::types::CompileType::{Base, Enum, Generic, List, Ref, Unit};
use crate::ast::{GenericIndex, TypeArena, TypeIndex};
use crate::error::ice::ice;
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
impl CompileType {
    pub fn is_generic(&self, arena: &TypeArena) -> bool {
        match self {
            Generic(_) => true,
            Base(_) | Enum(_) => false,
            Ref(ref_type) => ref_type.base.is_generic(arena),
            Unit(unit_type) => unit_type.has_generic(),
            List(list_type) => list_type.is_generic(arena),
        }
    }
}

impl TypeArena {
    pub fn new() -> Self {
        let mut arena = Self::empty();
        let mut interner = lasso::Rodeo::new();
        for ty in BaseType::base_types() {
            let name = match &ty {
                Base(base) => interner.get_or_intern(base.name()),
                _ => unreachable!(),
            };
            arena.insert(name, ty);
        }
        arena
    }
}

impl TypeIndex {
    pub fn size(&self, arena: &TypeArena) -> usize {
        let ty = arena.get(*self);
        match ty {
            Base(base) => base.size(),
            Ref(ref_type) => ref_type.size(),
            Unit(unit) => unit.size(arena),
            List(list) => list.size(arena),
            Enum(enm) => enm.size(),
            _ => {
                ice("Cannot get size of a Generic type.");
            }
        }
    }

    pub fn align(&self, arena: &TypeArena) -> usize {
        let ty = arena.get(*self);
        match ty {
            Base(base) => base.align(),
            Ref(ref_type) => ref_type.align(),
            Unit(unit) => unit.align(arena),
            List(list) => list.align(arena),
            Enum(enm) => enm.align(),
            _ => {
                ice("Cannot get alignment of a Generic type.");
            }
        }
    }

    pub fn dump(&self, arena: &TypeArena) -> String {
        let ty = arena.get(*self);
        match ty {
            Base(base) => base.dump(),
            Ref(ref_type) => ref_type.dump(arena),
            Unit(unit) => unit.dump(arena),
            List(list) => list.dump(arena),
            Enum(enm) => enm.dump(),
            _ => {
                ice("Cannot dump a Generic type.");
            }
        }
    }

    pub fn format(&self, arena: &TypeArena) -> String {
        let ty = arena.get(*self);
        match ty {
            Base(base_type) => base_type.name(),
            Ref(ref_type) => ref_type.format(arena),
            Unit(unit) => unit.format(),
            List(list) => list.format(arena),
            Enum(enum_type) => enum_type.name.clone(),
            _ => {
                ice("Type Index cannot contain a Generic type. This is a bug in the compiler.");
            }
        }
    }
    pub fn is_generic(&self, arena: &TypeArena) -> bool {
        let ty = arena.get(*self);
        ty.is_generic(arena)
    }
    pub fn has_generic(&self, arena: &TypeArena) -> bool {
        let ty = arena.get(*self);
        match ty {
            Base(_) | Enum(_) => false,
            Ref(ref_type) => ref_type.base.has_generic(arena),
            Unit(unit) => unit.has_generic(),
            List(list) => list.is_generic(arena),
            Generic(_) => false,
        }
    }
    pub fn is_base(&self, arena: &TypeArena) -> bool {
        matches!(arena.get(*self), Base(_))
    }
    pub fn is_ref(&self, arena: &TypeArena) -> bool {
        matches!(arena.get(*self), Ref(_))
    }
    pub fn is_unit(&self, arena: &TypeArena) -> bool {
        matches!(arena.get(*self), Unit(_))
    }
    pub fn is_integer(&self, arena: &TypeArena) -> bool {
        matches!(arena.get(*self), Base(base) if base.data_type() == base_type::DataType::Integer)
    }
    pub fn is_float(&self, arena: &TypeArena) -> bool {
        matches!(arena.get(*self), Base(base) if base.data_type() == base_type::DataType::Float)
    }
    pub fn is_signed(&self, arena: &TypeArena) -> bool {
        matches!(arena.get(*self), Base(base) if base.signed())
    }
    pub fn get_generic_index(&self, arena: &TypeArena) -> GenericIndex {
        let ty = arena.get(*self);
        match ty {
            Generic(index) => *index,
            _ => {
                ice("TypeIndex is not a Generic type.");
            }
        }
    }
    pub fn symbol_name(&self, arena: &TypeArena) -> String {
        if self.is_generic(arena) {
            ice("TypeIndex is a Generic type. Cannot get symbol name of a Generic type.")
        } else {
            self.format(arena)
        }
    }

    pub fn generic_instance_name(
        &self,
        arena: &TypeArena,
        params: &HashMap<GenericIndex, TypeIndex>,
    ) -> String {
        if self.has_generic(arena) {
            let ty = arena.get(*self);
            match ty {
                Ref(ref_type) => ref_type.generic_instance_name(arena, params),
                Unit(unit) => unit.generic_instance_name(arena, params),
                List(list) => list.generic_instance_name(arena, params),
                _ => ice(
                    "TypeIndex is not a Generic type. Cannot get generic instance name of a non-generic type.",
                ),
            }
        } else {
            if self.is_generic(arena) {
                let ty = params.get(&self.get_generic_index(arena));
                if let Some(ty) = ty {
                    ty.format(arena)
                } else {
                    ice(
                        "TypeIndex is not a Generic type. Cannot get generic instance name of a non-generic type.",
                    )
                }
            } else {
                ice(
                    "TypeIndex is not a Generic type. Cannot get generic instance name of a non-generic type.",
                )
            }
        }
    }
}
