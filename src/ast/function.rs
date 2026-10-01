use std::collections::HashMap;

use crate::ast::arena::{FuncIndex, InterfaceIndex, get_ident};
use crate::ast::symbol_name::SymbolName;
use crate::ast::types::resolve_type;
use crate::ast::{Config, GenericIndex, TypeIndex};
use crate::ast::{
    statement::Statement,
    symbols::{EnumBool, SymbolTable},
};
use crate::error::{CompileError, ResolveError};
use crate::lexer::Span;
use crate::parser::Scope;
use crate::parser::out::{TempFuncSymbol, TempInterfaceSymbol, TempType, TempVisibility};

pub(crate) mod bindings;
pub(crate) mod instantiate;
pub use instantiate::InstantiationActives;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuncBody {
    pub symbol: FuncIndex,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuncSymbol {
    pub name: String,
    pub params: Vec<(TypeIndex, String)>,
    pub ret_type: Option<TypeIndex>,
    pub generics: Vec<String>,
    pub generic_map: HashMap<String, GenericIndex>,
    pub attributes: Vec<String>,
    pub exported: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Interface {
    pub symbol: InterfaceIndex,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InterfaceSymbol {
    pub public: bool,
    pub has_self: bool,
    pub mutable: bool,
    pub exported: bool,
    pub owner: TypeIndex,
    pub attributes: Vec<String>,
    pub generics: Vec<String>,
    pub generic_map: HashMap<String, GenericIndex>,
    pub name: String,
    pub params: Vec<(TypeIndex, String)>,
    pub ret_type: Option<TypeIndex>,
}

impl FuncSymbol {
    pub fn has_generics(&self) -> bool {
        !self.generics.is_empty()
    }

    pub fn new(
        config: &mut Config,
        prototype: TempFuncSymbol,
        symbols: &mut SymbolTable,
    ) -> (Self, Span) {
        let name_span = prototype.name_span;
        let name = SymbolName::callable(None, &prototype.name);
        let (generics, generic_map, types) =
            bindings::generics(config, "func", &name, prototype.generics, symbols);
        let (params, ret_type) = bindings::signature(
            config,
            prototype.params,
            prototype.return_type,
            &types,
            symbols,
        );

        (
            Self {
                name,
                params,
                ret_type,
                generics,
                generic_map,
                attributes: prototype.attributes,
                exported: prototype.visibility.normal_export(config, &name_span),
            },
            name_span,
        )
    }
}

impl FuncBody {
    pub fn new(
        config: &mut Config,
        index: FuncIndex,
        prototype: Option<Scope>,
        symbols: &mut SymbolTable,
    ) -> Self {
        let body = if let Some(prototype) = prototype {
            let symbol = symbols.functions.get(index).clone();
            let mut context = bindings::context(
                config,
                EnumBool::False(index),
                &symbol.params,
                &symbol.generic_map,
                &HashMap::new(),
                None,
                (0..0).into(),
            );
            Statement::parse_scope(config, prototype, symbols, &mut context)
        } else {
            Vec::new()
        };

        Self {
            symbol: index,
            body,
        }
    }
}

impl InterfaceSymbol {
    pub fn has_generics(&self) -> bool {
        !self.generics.is_empty()
    }

    pub fn new(
        config: &mut Config,
        prototype: TempInterfaceSymbol,
        symbols: &mut SymbolTable,
    ) -> (Self, Span) {
        let name_span = prototype.name_span;
        let owner_name = prototype
            .owner
            .as_ref()
            .map(|(path, _)| config.symbol_name(&SymbolName::path(&path.segments)));
        let name = SymbolName::callable(owner_name.as_deref(), &prototype.name);
        let (generics, generic_map, types) =
            bindings::generics(config, "interface", &name, prototype.generics, symbols);
        let (params, ret_type) = bindings::signature(
            config,
            prototype.params,
            prototype.return_type,
            &types,
            symbols,
        );

        let (public, exported) = match prototype.visibility {
            TempVisibility::Public => (true, true),
            TempVisibility::Private => (false, false),
            TempVisibility::Export => (false, true),
            TempVisibility::Api => (true, true),
        };

        let owner = prototype
            .owner
            .and_then(|(path, span)| {
                crate::ast::types::resolve_type_with_bindings(
                    config,
                    (TempType::Path(path), span),
                    symbols,
                    &types,
                )
            })
            .unwrap_or(TypeIndex::empty());
        let name = if owner.is_empty() {
            name
        } else {
            SymbolName::callable(Some(&owner.format(&symbols.types)), &prototype.name)
        };

        (
            Self {
                name,
                params,
                ret_type,
                generics: generics,
                generic_map: generic_map,
                attributes: prototype.attributes,
                public,
                exported,
                mutable: prototype.mutable,
                has_self: prototype.has_self,
                owner: owner,
            },
            name_span,
        )
    }
}

impl Interface {
    pub fn new(
        config: &mut Config,
        index: InterfaceIndex,
        prototype: Option<Scope>,
        symbols: &mut SymbolTable,
    ) -> Self {
        let body = if let Some(prototype) = prototype {
            let symbol = symbols.interfaces.get(index).clone();
            let receiver = if symbol.has_self {
                let value = if symbol.mutable {
                    crate::ast::types::ValueType::Flex
                } else {
                    crate::ast::types::ValueType::Final
                };
                Some(symbol.owner.into_value_type(value, &mut symbols.types))
            } else {
                None
            };
            let mut context = bindings::context(
                config,
                EnumBool::True(index),
                &symbol.params,
                &symbol.generic_map,
                &HashMap::new(),
                receiver,
                (0..0).into(),
            );
            Statement::parse_scope(config, prototype, symbols, &mut context)
        } else {
            Vec::new()
        };

        Self {
            symbol: index,
            body,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::{ErrorHandle, ErrorInfo};
    use crate::parser::out::TempPath;

    #[test]
    fn unknown_interface_owner_reports_owner_span() {
        let mut config = Config::new(
            Vec::new(),
            String::new(),
            String::new(),
            ErrorHandle::new("test".into()),
        );
        let mut symbols = SymbolTable::new();
        let span = (4..17).into();
        let prototype = TempInterfaceSymbol {
            visibility: TempVisibility::Private,
            owner: Some((
                TempPath {
                    segments: vec!["Missing".into()],
                },
                span,
            )),
            has_self: false,
            mutable: false,
            name: "run".into(),
            name_span: span,
            generics: Vec::new(),
            params: Vec::new(),
            return_type: None,
            attributes: Vec::new(),
        };

        InterfaceSymbol::new(&mut config, prototype, &mut symbols);
        assert!(config.error_handle().errors.contains(&ErrorInfo::new(
            CompileError::Resolve(ResolveError::UnknownType),
            span,
        )));
    }
}
