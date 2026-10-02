use crate::visibility::TempVisibilityExt;

use crate::ast::arena::{
    ArenaIndex, FuncArena, FuncIndex, GenericArena, GenericIndex, InterfaceArena, InterfaceIndex,
    NamedArena,
};
use crate::ast::generic;
use crate::ast::symbol_name::SymbolName;
use crate::ast::types::CompileType::{Base, Enum};
use crate::ast::types::{CompileType, UnitType, resolve_type};
use crate::ast::{
    TypeIndex,
    config::Config,
    symbols::{EnumBool, SymbolTable},
};
use crate::diagnostic::error::{CompileError, ConstraintError, IllegalUseError, ResolveError};
use crate::parser::Scope;
use crate::parser::out::{Span, TempConstraints, TempGeneric, TempInterfaceSymbol, TempPath};
use std::collections::HashMap;
use std::sync::LazyLock;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterfaceRequire {
    pub name: String,
    pub ret_type: Option<TypeIndex>,
    pub params: Vec<(TypeIndex, String)>,
    pub has_self: bool,
    pub mutable: bool,
    pub owner: bool,
}

impl InterfaceRequire {
    pub fn new(
        config: &mut Config,
        prototype: TempInterfaceSymbol,
        symbols: &mut SymbolTable,
        span: Span,
    ) -> Option<Self> {
        let name = SymbolName::callable(None, &prototype.name);

        let temp_ret_temp = prototype.return_type;

        let ret_type = match temp_ret_temp {
            Some(temp_ret) => Some(resolve_type(config, temp_ret, symbols, None)?),
            None => None,
        };

        let params = prototype
            .params
            .into_iter()
            .map(|param| {
                let temp_ty = match param.ty {
                    Some(ty) => ty,
                    None => {
                        config.submit_error(CompileError::Resolve(ResolveError::MissingType), span);
                        return None;
                    }
                };
                resolve_type(config, temp_ty, symbols, None).map(|ty| (ty, param.name))
            })
            .collect::<Option<Vec<_>>>()?;

        Some(Self {
            name,
            ret_type,
            params,
            has_self: prototype.has_self,
            mutable: prototype.mutable,
            owner: prototype.owner.is_some(),
        })
    }

    pub fn check(&self, param: TypeIndex, symbols: &SymbolTable) -> bool {
        let param_name = param.format(&symbols.types);
        let interface_name = SymbolName::callable(Some(&param_name), &self.name);
        let interface = symbols.interfaces.get_by_name(&interface_name);

        let interface = match interface {
            Some(interface) => symbols.interfaces.get(interface),
            None => return false,
        };

        let ret_type = &interface.ret_type;
        if self.ret_type != *ret_type {
            return false;
        }
        if self.mutable != interface.mutable {
            return false;
        }
        if self.has_self != interface.has_self {
            return false;
        }
        if self.params.len() != interface.params.len() {
            return false;
        }
        if self
            .params
            .iter()
            .zip(interface.params.iter())
            .any(|((param_type, _), (expected_type, _))| param_type != expected_type)
        {
            return false;
        }
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GenericTypeRequire {
    Integer,
    Float,
    Signed,
    MinBits(usize),
    MaxBits(usize),
}

static TYPE_REQUIRE_MAP: LazyLock<HashMap<&'static str, GenericTypeRequire>> =
    LazyLock::new(|| {
        let mut m = HashMap::new();
        m.insert("is_integer", GenericTypeRequire::Integer);
        m.insert("is_float", GenericTypeRequire::Float);
        m.insert("is_signed", GenericTypeRequire::Signed);
        m.insert("max_bits", GenericTypeRequire::MaxBits(0));
        m.insert("min_bits", GenericTypeRequire::MinBits(0));
        m
    });

impl GenericTypeRequire {
    pub fn convert(ident: &str) -> Option<Self> {
        TYPE_REQUIRE_MAP.get(ident).cloned()
    }

    pub fn new(
        config: &mut Config,
        prototype: (TempPath, Option<String>),
        span: Span,
    ) -> Option<Self> {
        let (name, arg) = prototype;
        let seg = name.segments;
        if seg.len() != 3 || seg[0] != "std" || seg[1] != "generic" {
            config.submit_error(
                CompileError::Resolve(ResolveError::Constraint(
                    ConstraintError::InvalidRequirement,
                )),
                span,
            );
            return None;
        }

        let Some(require) = Self::convert(&seg[2]) else {
            config.submit_error(
                CompileError::Resolve(ResolveError::Constraint(ConstraintError::NoRequirements)),
                span,
            );
            return None;
        };

        match require {
            Self::MaxBits(_) | Self::MinBits(_) => {
                let Some(arg) = arg else {
                    config.submit_error(
                        CompileError::Resolve(ResolveError::Constraint(
                            ConstraintError::NoArgument,
                        )),
                        span,
                    );
                    return None;
                };
                let Ok(bits) = arg.parse::<usize>() else {
                    config.submit_error(
                        CompileError::Resolve(ResolveError::Constraint(
                            ConstraintError::InvalidArgument,
                        )),
                        span,
                    );
                    return None;
                };
                Some(match require {
                    Self::MaxBits(_) => Self::MaxBits(bits),
                    Self::MinBits(_) => Self::MinBits(bits),
                    _ => unreachable!(),
                })
            }
            _ if arg.is_some() => {
                config.submit_error(
                    CompileError::Resolve(ResolveError::Constraint(
                        ConstraintError::InvalidArgument,
                    )),
                    span,
                );
                None
            }
            _ => Some(require),
        }
    }

    pub fn check(&self, param: TypeIndex, symbols: &SymbolTable) -> bool {
        match self {
            GenericTypeRequire::Integer => param.is_integer(&symbols.types),
            GenericTypeRequire::Float => param.is_float(&symbols.types),
            GenericTypeRequire::Signed => param.is_signed(&symbols.types),
            GenericTypeRequire::MinBits(bits) => {
                let size = param.size(&symbols.types);
                let param_bits = size * 8;
                param_bits >= *bits
            }
            GenericTypeRequire::MaxBits(bits) => {
                let size = param.size(&symbols.types);
                let param_bits = size * 8;
                param_bits <= *bits
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Constraints {
    Interface(InterfaceRequire),
    Type(GenericTypeRequire),
}

impl Constraints {
    pub fn check(&self, param: TypeIndex, symbols: &SymbolTable) -> bool {
        match self {
            Constraints::Interface(interface) => interface.check(param, symbols),
            Constraints::Type(requirement) => requirement.check(param, symbols),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericRequire {
    pub name: String,
    pub requires: Vec<Constraints>,
    pub attributes: Vec<String>,
    pub exported: bool,
}

impl GenericRequire {
    pub fn empty(name: String) -> Self {
        Self {
            name,
            requires: Vec::new(),
            attributes: Vec::new(),
            exported: false,
        }
    }

    pub fn new(
        config: &mut Config,
        prototype: TempGeneric,
        symbols: &mut SymbolTable,
    ) -> (Self, Span) {
        let name_span = prototype.name_span;
        (
            Self {
                name: prototype.name,
                requires: prototype
                    .requirements
                    .into_iter()
                    .filter_map(|(temp, span)| match temp {
                        TempConstraints::Type { path, argument } => {
                            GenericTypeRequire::new(config, (path, argument), span)
                                .map(Constraints::Type)
                        }
                        TempConstraints::Interface(func) => {
                            InterfaceRequire::new(config, func, symbols, span)
                                .map(Constraints::Interface)
                        }
                    })
                    .collect(),
                attributes: prototype.attributes,
                exported: prototype.visibility.normal_export(config, &name_span),
            },
            name_span,
        )
    }

    pub fn check(&self, param: TypeIndex, symbols: &SymbolTable) -> bool {
        self.requires
            .iter()
            .all(|requirement| requirement.check(param, symbols))
    }
}

pub type UnitArena = NamedArena<UnitType>;
pub type UnitIndex = ArenaIndex<UnitType>;

#[derive(Debug, Clone, PartialEq)]
pub struct GenericTable {
    pub requires: GenericArena,
    pub units: UnitArena,
    pub functions: FuncArena,
    pub interfaces: InterfaceArena,
    pub function_templates: HashMap<FuncIndex, Scope>,
    pub interface_templates: HashMap<InterfaceIndex, Scope>,
}

impl GenericTable {
    pub fn new() -> Self {
        Self::for_file(crate::ast::config::FileId::new(0))
    }

    pub fn for_file(file_id: crate::ast::config::FileId) -> Self {
        Self {
            requires: GenericArena::for_file(file_id),
            units: UnitArena::for_file(file_id),
            functions: FuncArena::for_file(file_id),
            interfaces: InterfaceArena::for_file(file_id),
            function_templates: HashMap::new(),
            interface_templates: HashMap::new(),
        }
    }
}

impl GenericIndex {
    pub fn check(&self, param: TypeIndex, symbols: &SymbolTable) -> bool {
        let generic = symbols.generics.requires.get(*self);
        generic.check(param, symbols)
    }

    pub fn instantiation(
        self,
        config: &mut Config,
        params: &HashMap<GenericIndex, TypeIndex>,
        symbols: &SymbolTable,
        span: Span,
    ) -> Option<TypeIndex> {
        let require = symbols.generics.requires.get(self);

        let param = match params.get(&self) {
            Some(param) => *param,
            None => {
                config.submit_error(CompileError::Resolve(ResolveError::UnknownGeneric), span);
                return None;
            }
        };

        if require.check(param, symbols) {
            Some(param)
        } else {
            config.submit_error(
                CompileError::IllegalUse(IllegalUseError::RequirementUnmet),
                span,
            );
            None
        }
    }
}

impl TypeIndex {
    pub fn instantiation(
        self,
        config: &mut Config,
        params: &HashMap<GenericIndex, TypeIndex>,
        symbols: &mut SymbolTable,
        actives: Option<&crate::ast::function::InstantiationActives>,
        span: Span,
    ) -> Option<TypeIndex> {
        if config.is_poisoned() {
            return None;
        }
        let value = self.value_type(&symbols.types);
        let ty = symbols.types.get(self).unqualified().clone();
        match ty {
            Base(_) | Enum(_) => Some(self),
            CompileType::Generic(generic) => generic
                .instantiation(config, params, symbols, span)
                .map(|index| index.into_value_type(value, &mut symbols.types)),
            CompileType::Unit(unit) if !unit.has_generic() => Some(self),
            CompileType::Unit(unit) => unit
                .instantiation(config, params, symbols, actives, span)
                .map(|index| index.into_value_type(value, &mut symbols.types)),
            CompileType::List(list) => list
                .instantiation(config, params, symbols, actives, span)
                .map(|index| index.into_value_type(value, &mut symbols.types)),
            CompileType::Ref(ref_type) => ref_type
                .instantiation(config, params, symbols, actives, span)
                .map(|index| index.into_value_type(value, &mut symbols.types)),
            CompileType::Qualified { .. } => unreachable!(),
        }
    }
}

#[cfg(test)]
mod tests;
