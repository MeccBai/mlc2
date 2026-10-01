use crate::ast::arena::{FuncIndex, InterfaceIndex};
use crate::ast::config::Config;
use crate::ast::expression::Expression;
use crate::ast::symbol_name::SymbolName;
use crate::ast::{EnumBool, SymbolTable};
use crate::diagnostic::error::{CompileError, IllegalUseError, ResolveError};
use crate::diagnostic::ice::ice;
use crate::parser::out::{
    Spanned, TempExpr,
    TempExpr::{Member, Path},
};

impl Expression {
    pub(super) fn is_receiver(
        &self,
        context: Option<&crate::ast::symbols::StatementContext>,
    ) -> bool {
        matches!(self, Self::VarValueE(variable) if context
            .and_then(|context| context.lookup("self"))
            .is_some_and(|receiver| std::rc::Rc::ptr_eq(variable, receiver)))
    }

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
                if owner.is_poisoned() {
                    return None;
                }
                let receiver = owner.is_receiver(context);
                if receiver && !indirect {
                    config.submit_error(
                        CompileError::IllegalUse(IllegalUseError::MemberAccessViolation),
                        span,
                    );
                    return None;
                }
                let owner_type = owner.type_inference(config, symbols);
                let owner_type = if indirect && !receiver && !owner_type.is_empty() {
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
                if !receiver && !symbols.interfaces.get(index).public {
                    config.submit_error(
                        CompileError::IllegalUse(IllegalUseError::MemberAccessViolation),
                        span,
                    );
                    return None;
                }
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
                let public = if generic {
                    symbols.generics.interfaces.get(index).public
                } else {
                    symbols.interfaces.get(index).public
                };
                if !public {
                    config.submit_error(
                        CompileError::IllegalUse(IllegalUseError::MemberAccessViolation),
                        span,
                    );
                    return None;
                }
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
