use std::collections::HashMap;
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

use crate::ast::arena::get_ident;
use crate::ast::config::Config;
use crate::ast::symbol_name::SymbolName;
use crate::ast::types::CompileType::{Base, Enum, Generic, List, Ref, Unit};
use crate::ast::{GenericIndex, SymbolTable, TypeArena, TypeIndex};
use crate::error::ice::ice;
use crate::error::{CompileError, ResolveError};
use crate::parser::out::{Spanned, TempType};

#[derive(Debug, Clone, PartialEq, Eq)]
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

    pub fn format(&self, arena: &TypeArena) -> String {
        match self {
            Base(base_type) => base_type.name(),
            Ref(ref_type) => ref_type.format(arena),
            Unit(unit_type) => unit_type.format(),
            List(list_type) => list_type.format(arena),
            Enum(enum_type) => enum_type.name.clone(),
            Generic(_) => ice("Cannot format a Generic type."),
        }
    }

    
}

impl TypeArena {
    pub fn new() -> (Self, HashMap<usize, TypeIndex>) {
        let mut view = HashMap::<usize, TypeIndex>::new();
        let mut arena = Self::empty();
        for (index, ty) in BaseType::base_types() {
            let name = match &ty {
                Base(base) => get_ident(&base.name()),
                _ => ice("New type arena cannot contain other types."),
            };
            let ty_index = arena.insert(name, ty);
            view.insert(index, ty_index);
        }
        (arena, view)
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
                ice("Cannot format a Generic type.");
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

    pub fn deref(&self, arena: &mut TypeArena) -> Option<TypeIndex> {
        let ty = arena.get(*self);
        match ty {
            Ref(ref_type) => ref_type.clone().deref(arena),
            _ => ice("Non-ref type cannot be de referenced."),
        }
    }

    pub fn make_ref(&self, arena: &mut TypeArena, mut_base: bool) -> TypeIndex {
        let ty = arena.get(*self);
        match ty {
            Ref(ref_type) => {
                let new_ref = RefType::new(ref_type.base, ref_type.level + 1, mut_base);
                let type_str = new_ref.format(arena);
                let ident = get_ident(&type_str);
                arena.insert(ident, Ref(new_ref))
            }
            _ => {
                let new_ref = RefType::new(*self, 1, mut_base);
                let type_str = new_ref.format(arena);
                let ident = get_ident(&type_str);
                arena.insert(ident, Ref(new_ref))
            }
        }
    }

    pub fn type_check(&self, tolerance: bool, other: &TypeIndex, arena: &TypeArena) -> bool {
        if self == other {
            return true;
        }

        let target = arena.get(*self);
        let other = arena.get(*other);
        match (target, other) {
            (Base(base), Base(other)) => base.type_check(tolerance, other),
            (Ref(ref_t), Ref(ref2)) => ref_t.type_check(ref2, arena),
            (List(list), List(list2)) => list.type_check(list2, arena),
            _ => false,
        }
    }
}

pub fn resolve_type(
    config: &mut Config,
    temp: Spanned<TempType>,
    symbols: &mut SymbolTable,
) -> Option<TypeIndex> {
    let (temp, span) = temp;
    match temp {
        TempType::Path(path) => {
            let path = SymbolName::path(&path.segments);
            let local = config.symbol_name(&path);
            match symbols
                .types
                .get_by_name(&local)
                .or_else(|| symbols.types.get_by_name(&path))
            {
                Some(ty) => Some(ty),
                None => {
                    config.submit_error(CompileError::Resolve(ResolveError::UnknownType), span);
                    None
                }
            }
        }
        TempType::Generic { base, args } => {
            let name = config.symbol_name(&SymbolName::path(&base.segments));
            let ident = get_ident(&name);
            let index = match symbols.generics.units.get_by_ident(ident) {
                Some(index) => index,
                None => {
                    config.submit_error(CompileError::Resolve(ResolveError::UnknownGeneric), span);
                    return None;
                }
            };

            let unit = symbols.generics.units.get(index).clone();
            if args.len() != unit.generics.len() {
                config.submit_error(
                    CompileError::IllegalUse(crate::error::IllegalUseError::GenericCountMismatch),
                    span,
                );
                return None;
            }
            let resolved = args
                .into_iter()
                .map(|arg| resolve_type(config, arg, symbols))
                .collect::<Option<Vec<_>>>()?;

            let mut params = HashMap::new();
            for (name, ty) in unit.generics.iter().zip(resolved) {
                let Some(&index) = unit.generic_map.get(name) else {
                    config.submit_error(CompileError::Resolve(ResolveError::UnknownGeneric), span);
                    return None;
                };
                params.insert(index, ty);
            }
            unit.instantiation(config, &params, symbols, None, span)
        }
        TempType::Reference { inner, mutable } => {
            let base = resolve_type(config, *inner, symbols)?;
            let ty = symbols.types.get(base);

            if let CompileType::Ref(ref_type) = ty {
                let new_ref = RefType::new(ref_type.base, ref_type.level + 1, mutable);
                let type_str = new_ref.format(&symbols.types);
                let ident = get_ident(&type_str);
                Some(symbols.types.insert(ident, CompileType::Ref(new_ref)))
            } else {
                let new_ref = RefType::new(base, 1, mutable);
                let type_str = new_ref.format(&symbols.types);
                let ident = get_ident(&type_str);
                Some(symbols.types.insert(ident, CompileType::Ref(new_ref)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{CompileError, ErrorHandle, ErrorInfo, IllegalUseError, ResolveError};
    use crate::parser::out::TempPath;

    fn config() -> Config {
        Config::new(
            Vec::new(),
            String::new(),
            String::new(),
            ErrorHandle::new("test".into()),
        )
    }

    #[test]
    fn unknown_type_submits_resolution_error_at_type_span() {
        let mut config = config();
        let mut symbols = SymbolTable::new();
        let span = (10..17).into();
        let path = TempPath {
            segments: vec!["Missing".into()],
        };
        assert_eq!(
            resolve_type(&mut config, (TempType::Path(path), span), &mut symbols),
            None
        );
        assert!(config.error_handle().errors.contains(&ErrorInfo::new(
            CompileError::Resolve(ResolveError::UnknownType),
            span,
        )));
    }

    #[test]
    fn generic_argument_count_submits_illegal_use_error() {
        let mut config = config();
        let mut symbols = SymbolTable::new();
        let name = config.symbol_name("Box");
        let mut unit = UnitType::empty();
        unit.generics.push(String::new());
        symbols.generics.units.insert(get_ident(&name), unit);
        let span = (20..27).into();
        let ty = TempType::Generic {
            base: TempPath {
                segments: vec!["Box".into()],
            },
            args: Vec::new(),
        };
        assert_eq!(resolve_type(&mut config, (ty, span), &mut symbols), None);
        assert!(config.error_handle().errors.contains(&ErrorInfo::new(
            CompileError::IllegalUse(IllegalUseError::GenericCountMismatch),
            span,
        )));
    }
}
