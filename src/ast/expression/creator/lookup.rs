use crate::ast::arena::{FuncIndex, InterfaceIndex};
use crate::ast::config::Config;
use crate::ast::expression::Expression;
use crate::ast::symbol_name::SymbolName;
use crate::ast::{EnumBool, SymbolTable};
use crate::error::{CompileError, IllegalUseError, ResolveError, ice::ice};
use crate::parser::out::{
    Spanned, TempExpr,
    TempExpr::{Member, Path},
};

impl Expression {
    pub fn search_interface(
        config: &mut Config,
        temp_expr: Spanned<TempExpr>,
        symbols: &mut SymbolTable,
        context: Option<&crate::ast::symbols::StatementContext>,
    ) -> Option<(Expression, InterfaceIndex)> {
        if config.is_poisoned() {
            return None;
        }
        let (expr, span) = temp_expr;
        match expr {
            Member {
                base,
                indirect,
                name,
                ..
            } => {
                let owner = Expression::new(config, *base, symbols, context);
                let owner_type = owner.type_inference(config, symbols);
                let owner_type = if indirect && !owner_type.is_empty() {
                    if !owner_type.is_ref(&symbols.types) {
                        config.submit_error(
                            CompileError::IllegalUse(IllegalUseError::MemberAccessViolation),
                            span,
                        );
                        return None;
                    }
                    owner_type.deref(&mut symbols.types).unwrap()
                } else {
                    owner_type
                };
                if owner_type.is_empty() {
                    return None;
                }
                let owner_type =
                    owner_type.into(crate::ast::types::ValueType::Flex, &mut symbols.types);
                let interface_name =
                    SymbolName::callable(Some(&owner_type.format(&symbols.types)), &name);
                let Some(index) = symbols.interfaces.get_by_name(&interface_name) else {
                    config
                        .submit_error(CompileError::Resolve(ResolveError::UnknownInterface), span);
                    return None;
                };
                Some((owner, index))
            }
            _ => ice("Interface search cannot be performed on non-member expressions."),
        }
    }

    pub fn search_function(
        config: &mut Config,
        temp_expr: Spanned<TempExpr>,
        symbols: &mut SymbolTable,
        context: Option<&crate::ast::symbols::StatementContext>,
    ) -> Option<EnumBool<InterfaceIndex, FuncIndex>> {
        Self::search_path_callable(config, temp_expr, symbols, context, false)
    }

    pub fn search_generic_function(
        config: &mut Config,
        temp_expr: Spanned<TempExpr>,
        symbols: &mut SymbolTable,
        context: Option<&crate::ast::symbols::StatementContext>,
    ) -> Option<EnumBool<InterfaceIndex, FuncIndex>> {
        Self::search_path_callable(config, temp_expr, symbols, context, true)
    }

    fn search_path_callable(
        config: &mut Config,
        temp_expr: Spanned<TempExpr>,
        symbols: &SymbolTable,
        context: Option<&crate::ast::symbols::StatementContext>,
        requires_generic: bool,
    ) -> Option<EnumBool<InterfaceIndex, FuncIndex>> {
        if config.is_poisoned() {
            return None;
        }
        let (Path(path), span) = temp_expr else {
            ice("Callable path search requires a path.")
        };
        use crate::ast::symbols::PathSymbol;
        let error = match symbols.resolve_path(config, &path, context) {
            Some(PathSymbol::Interface { index, generic }) if generic == requires_generic => {
                return Some(EnumBool::True(index));
            }
            Some(PathSymbol::Function { index, generic }) if generic == requires_generic => {
                return Some(EnumBool::False(index));
            }
            Some(PathSymbol::Interface { .. } | PathSymbol::Function { .. }) => {
                CompileError::IllegalUse(IllegalUseError::UnsupportedGenericCall)
            }
            Some(PathSymbol::GenericParameter(_)) => {
                CompileError::IllegalUse(IllegalUseError::TypeUsedAsValue)
            }
            Some(PathSymbol::Variable(_) | PathSymbol::EnumValue(_)) => {
                CompileError::IllegalUse(IllegalUseError::SymbolNotCallable)
            }
            None => CompileError::Resolve(ResolveError::UnknownFunction),
        };
        config.submit_error(error, span);
        None
    }
}
