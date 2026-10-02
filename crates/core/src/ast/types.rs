use std::collections::HashMap;
pub mod base_type;
pub use base_type::BaseType;

pub mod ref_type;
pub use ref_type::RefType;

pub mod unit_type;
pub use unit_type::UnitType;

pub mod list_type;
pub use list_type::ListType;
pub mod enum_type;

pub use enum_type::EnumType;

mod lookup;
mod resolve;
pub use lookup::{TypeLookup, TypeStorage};
pub use resolve::{TypeContext, resolve_type, resolve_type_with_bindings};

use crate::ast::arena::get_ident;
use crate::ast::config::Config;
use crate::ast::symbol_name::SymbolName;
use crate::ast::types::CompileType::{Base, Enum, Generic, List, Ref, Unit};
use crate::ast::{
    GenericIndex, TypeArena, TypeIndex,
    symbols::{EnumBool, SymbolTable},
};
use crate::diagnostic::error::{CompileError, ResolveError};
use crate::diagnostic::ice::ice;
use crate::parser::out::{Spanned, TempType};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompileType {
    Base(BaseType),
    Ref(RefType),
    Unit(UnitType),
    List(ListType),
    Enum(EnumType),
    Generic(GenericIndex),
    Qualified {
        base: Box<CompileType>,
        value: ValueType,
    },
}

pub use mlc_syntax::ValueType;

impl CompileType {
    pub fn value_type(&self) -> ValueType {
        match self {
            Self::Qualified { value, .. } => *value,
            _ => ValueType::Flex,
        }
    }

    pub fn unqualified(&self) -> &Self {
        match self {
            Self::Qualified { base, .. } => base.unqualified(),
            _ => self,
        }
    }

    pub fn is_generic(&self, arena: &(impl crate::ast::types::TypeLookup + ?Sized)) -> bool {
        match self {
            Self::Qualified { base, .. } => base.is_generic(arena),
            Generic(_) => true,
            Base(_) | Enum(_) => false,
            Ref(ref_type) => ref_type.base.is_generic(arena),
            Unit(unit_type) => unit_type.has_generic(),
            List(list_type) => list_type.is_generic(arena),
        }
    }

    pub fn format(&self, arena: &(impl crate::ast::types::TypeLookup + ?Sized)) -> String {
        match self {
            Self::Qualified { base, value } => format!("{}{}", value.prefix(), base.format(arena)),
            Base(base_type) => base_type.name(),
            Ref(ref_type) => ref_type.format(arena),
            Unit(unit_type) => unit_type.format(),
            List(list_type) => list_type.format(arena),
            Enum(enum_type) => enum_type.name.clone(),
            Generic(index) => SymbolName::generic_type(*index),
        }
    }

    pub fn dump(&self, arena: &(impl crate::ast::types::TypeLookup + ?Sized)) -> String {
        match self {
            Self::Qualified { base, value } => format!("{}{}", value.prefix(), base.dump(arena)),
            Base(base) => base.dump(),
            Ref(reference) => reference.dump(arena),
            Unit(unit) => unit.dump(arena),
            List(list) => list.dump(arena),
            Enum(enm) => enm.dump(),
            Generic(index) => SymbolName::generic_type(*index),
        }
    }

    pub fn size(&self, arena: &(impl crate::ast::types::TypeLookup + ?Sized)) -> usize {
        match self {
            Self::Qualified { base, .. } => base.size(arena),
            Base(base) => base.size(),
            Ref(reference) => reference.size(),
            Unit(unit) => unit.size(arena),
            List(list) => list.size(arena),
            Enum(enm) => enm.size(),
            Generic(_) => ice("Cannot get size of a Generic type."),
        }
    }
}

impl TypeArena {
    pub fn new() -> (Self, HashMap<usize, TypeIndex>) {
        Self::new_for_file(crate::ast::config::FileId::new(0))
    }

    pub fn new_for_file(file_id: crate::ast::config::FileId) -> (Self, HashMap<usize, TypeIndex>) {
        let mut view = HashMap::<usize, TypeIndex>::new();
        let mut arena = Self::for_file(file_id);
        for (index, ty) in BaseType::base_types() {
            let name = match &ty {
                Base(base) => get_ident(&base.name()),
                _ => ice("New type arena cannot contain other types."),
            };
            let ty_index = arena.insert_type(name, ty);
            view.insert(index, ty_index);
        }
        (arena, view)
    }
}

impl TypeIndex {
    pub fn value_type(&self, arena: &(impl crate::ast::types::TypeLookup + ?Sized)) -> ValueType {
        arena.get_type(*self).value_type()
    }

    pub fn into_value_type(
        self,
        value: ValueType,
        arena: &mut (impl crate::ast::types::TypeStorage + ?Sized),
    ) -> Self {
        if self.is_empty() {
            return self;
        }
        if self.value_type(arena) == value {
            return self;
        }
        let base = arena.get_type(self).unqualified().clone();
        let qualified = match value {
            ValueType::Flex => base,
            _ => CompileType::Qualified {
                base: Box::new(base),
                value,
            },
        };
        let name = qualified.format(arena);
        arena.insert_type(get_ident(&name), qualified)
    }

    pub fn into(
        self,
        value: ValueType,
        arena: &mut (impl crate::ast::types::TypeStorage + ?Sized),
    ) -> Self {
        self.into_value_type(value, arena)
    }
    pub fn size(&self, arena: &(impl crate::ast::types::TypeLookup + ?Sized)) -> usize {
        let ty = arena.get_type(*self).unqualified();
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

    pub fn align(&self, arena: &(impl crate::ast::types::TypeLookup + ?Sized)) -> usize {
        let ty = arena.get_type(*self).unqualified();
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

    pub fn dump(&self, arena: &(impl crate::ast::types::TypeLookup + ?Sized)) -> String {
        let ty = arena.get_type(*self);
        if let CompileType::Qualified { .. } = ty {
            return ty.dump(arena);
        }
        match ty {
            Base(base) => base.dump(),
            Ref(ref_type) => ref_type.dump(arena),
            Unit(unit) => unit.dump(arena),
            List(list) => list.dump(arena),
            Enum(enm) => enm.dump(),
            Generic(index) => SymbolName::generic_type(*index),
            CompileType::Qualified { .. } => unreachable!(),
        }
    }

    pub fn format(&self, arena: &(impl crate::ast::types::TypeLookup + ?Sized)) -> String {
        let ty = arena.get_type(*self);
        if let CompileType::Qualified { .. } = ty {
            return ty.format(arena);
        }
        match ty {
            Base(base_type) => base_type.name(),
            Ref(ref_type) => ref_type.format(arena),
            Unit(unit) => unit.format(),
            List(list) => list.format(arena),
            Enum(enum_type) => enum_type.name.clone(),
            Generic(index) => SymbolName::generic_type(*index),
            CompileType::Qualified { .. } => unreachable!(),
        }
    }

    pub fn is_generic(&self, arena: &(impl crate::ast::types::TypeLookup + ?Sized)) -> bool {
        let ty = arena.get_type(*self).unqualified();
        ty.is_generic(arena)
    }

    pub fn has_generic(&self, arena: &(impl crate::ast::types::TypeLookup + ?Sized)) -> bool {
        let ty = arena.get_type(*self).unqualified();
        match ty {
            Base(_) | Enum(_) => false,
            Ref(ref_type) => ref_type.base.has_generic(arena),
            Unit(unit) => unit.has_generic(),
            List(list) => list.is_generic(arena),
            Generic(_) => false,
            CompileType::Qualified { .. } => unreachable!(),
        }
    }

    pub fn is_base(&self, arena: &(impl crate::ast::types::TypeLookup + ?Sized)) -> bool {
        matches!(arena.get_type(*self).unqualified(), Base(_))
    }

    pub fn is_ref(&self, arena: &(impl crate::ast::types::TypeLookup + ?Sized)) -> bool {
        matches!(arena.get_type(*self).unqualified(), Ref(_))
    }

    pub fn is_unit(&self, arena: &(impl crate::ast::types::TypeLookup + ?Sized)) -> bool {
        matches!(arena.get_type(*self).unqualified(), Unit(_))
    }

    pub fn is_integer(&self, arena: &(impl crate::ast::types::TypeLookup + ?Sized)) -> bool {
        matches!(arena.get_type(*self).unqualified(), Base(base) if base.data_type() == base_type::DataType::Integer)
    }

    pub fn is_float(&self, arena: &(impl crate::ast::types::TypeLookup + ?Sized)) -> bool {
        matches!(arena.get_type(*self).unqualified(), Base(base) if base.data_type() == base_type::DataType::Float)
    }

    pub fn is_signed(&self, arena: &(impl crate::ast::types::TypeLookup + ?Sized)) -> bool {
        matches!(arena.get_type(*self).unqualified(), Base(base) if base.signed())
    }

    pub fn get_generic_index(
        &self,
        arena: &(impl crate::ast::types::TypeLookup + ?Sized),
    ) -> GenericIndex {
        let ty = arena.get_type(*self).unqualified();
        match ty {
            Generic(index) => *index,
            _ => {
                ice("TypeIndex is not a Generic type.");
            }
        }
    }

    pub fn symbol_name(&self, arena: &(impl crate::ast::types::TypeLookup + ?Sized)) -> String {
        if self.is_generic(arena) {
            ice("TypeIndex is a Generic type. Cannot get symbol name of a Generic type.")
        } else {
            self.format(arena)
        }
    }

    pub fn generic_instance_name(
        &self,
        arena: &(impl crate::ast::types::TypeLookup + ?Sized),
        params: &HashMap<GenericIndex, TypeIndex>,
    ) -> String {
        if self.has_generic(arena) {
            let ty = arena.get_type(*self).unqualified();
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
                self.format(arena)
            }
        }
    }

    pub fn deref(
        &self,
        arena: &mut (impl crate::ast::types::TypeStorage + ?Sized),
    ) -> Option<TypeIndex> {
        let ty = arena.get_type(*self).unqualified();
        match ty {
            Ref(ref_type) => {
                let mutable = ref_type.mut_base;
                ref_type.clone().deref(arena).map(|base| {
                    let value = if mutable {
                        ValueType::Flex
                    } else {
                        ValueType::Final
                    };
                    base.into_value_type(value, arena)
                })
            }
            _ => ice("Non-ref type cannot be de referenced."),
        }
    }

    pub fn make_ref(
        &self,
        arena: &mut (impl crate::ast::types::TypeStorage + ?Sized),
        mut_base: bool,
    ) -> TypeIndex {
        let base = self.into_value_type(ValueType::Flex, arena);
        let ty = arena.get_type(base).unqualified();
        match ty {
            Ref(ref_type) => {
                let new_ref = RefType::new(ref_type.base, ref_type.level + 1, mut_base);
                let type_str = new_ref.format(arena);
                let ident = get_ident(&type_str);
                arena.insert_type(ident, Ref(new_ref))
            }
            _ => {
                let new_ref = RefType::new(base, 1, mut_base);
                let type_str = new_ref.format(arena);
                let ident = get_ident(&type_str);
                arena.insert_type(ident, Ref(new_ref))
            }
        }
    }

    pub fn type_check(
        &self,
        tolerance: bool,
        other: &TypeIndex,
        arena: &(impl crate::ast::types::TypeLookup + ?Sized),
    ) -> bool {
        if self == other {
            return true;
        }

        let target = arena.get_type(*self).unqualified();
        let other = arena.get_type(*other).unqualified();
        match (target, other) {
            (Base(base), Base(other)) => base.type_check(tolerance, other),
            (Ref(ref_t), Ref(ref2)) => ref_t.type_check(ref2, arena),
            (List(list), List(list2)) => list.type_check(list2, arena),
            (Unit(unit), Unit(other)) => unit.format() == other.format(),
            (Enum(enm), Enum(other)) => enm.name == other.name,
            (Generic(index), Generic(other)) => index == other,
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests;
