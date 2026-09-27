use std::collections::HashMap;

use crate::ast::arena::{FuncIndex, InterfaceIndex, get_ident};
use crate::ast::stmt::Statement;
use crate::ast::types::resolve_type;
use crate::ast::{Config, GenericIndex, SymbolTable, TypeIndex};
use crate::error::{CompileError, ResolveError};
use crate::parser::Scope;
use crate::parser::out::{TempFuncSymbol, TempInterfaceSymbol, TempType, TempVisibility};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FuncBody {
    pub name: FuncIndex,
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
    pub fn new(config: &mut Config, prototype: TempFuncSymbol, symbols: &mut SymbolTable) -> Self {
        let params = prototype
            .params
            .into_iter()
            .filter_map(|param| {
                let ty = resolve_type(config, param.ty?, symbols)?;
                Some((ty, param.name))
            })
            .collect();
        let ret_type = prototype
            .return_type
            .and_then(|ty| resolve_type(config, ty, symbols));

        let mut generic_map = HashMap::<String, GenericIndex>::new();

        let generics = prototype
            .generics
            .into_iter()
            .filter_map(|generic| {
                let (path, span) = generic.constraint?;
                let constraint = path.join();
                let ident = get_ident(&constraint);
                let Some(index) = symbols.generics.requires.get_by_ident(ident) else {
                    config
                        .submit_error(CompileError::Resolve(ResolveError::UnknownConstraint), span);
                    return None;
                };
                generic_map.insert(generic.name.clone(), index);
                Some(generic.name)
            })
            .collect::<Vec<_>>();

        Self {
            name: prototype.name,
            params,
            ret_type,
            generics: generics,
            generic_map: generic_map,
            attributes: prototype.attributes,
            exported: prototype.visibility.normal_export(),
        }
    }
}

impl FuncBody {
    pub fn new(
        config: &mut Config,
        index: FuncIndex,
        prototype: Option<Scope>,
        symbols: &mut SymbolTable,
    ) -> Self {
        todo!("Implement FuncBody::new")
    }
}

impl InterfaceSymbol {
    pub fn new(
        config: &mut Config,
        prototype: TempInterfaceSymbol,
        symbols: &mut SymbolTable,
    ) -> Self {
        let params = prototype
            .params
            .into_iter()
            .filter_map(|param| {
                let ty = resolve_type(config, param.ty?, symbols)?;
                Some((ty, param.name))
            })
            .collect();
        let ret_type = prototype
            .return_type
            .and_then(|ty| resolve_type(config, ty, symbols));

        let (public, exported) = match prototype.visibility {
            TempVisibility::Public => (true, true),
            TempVisibility::Private => (false, false),
            TempVisibility::Export => (false, true),
            TempVisibility::Api => (true, true),
        };

        let mut generic_map = HashMap::<String, GenericIndex>::new();

        let generics = prototype
            .generics
            .into_iter()
            .filter_map(|generic| {
                let (path, span) = generic.constraint?;
                let constraint = path.join();
                let ident = get_ident(&constraint);
                let Some(index) = symbols.generics.requires.get_by_ident(ident) else {
                    config
                        .submit_error(CompileError::Resolve(ResolveError::UnknownConstraint), span);
                    return None;
                };
                generic_map.insert(generic.name.clone(), index);
                Some(generic.name)
            })
            .collect::<Vec<_>>();

        let owner = prototype
            .owner
            .and_then(|(path, span)| resolve_type(config, (TempType::Path(path), span), symbols))
            .unwrap_or(TypeIndex::empty());

        Self {
            name: prototype.name,
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
        }
    }
}

impl Interface {
    pub fn new(
        config: &mut Config,
        index: InterfaceIndex,
        prototype: Option<Scope>,
        symbols: &mut SymbolTable,
    ) -> Self {
        todo!("Implement FuncBody::new")
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
