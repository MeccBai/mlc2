use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::arena::{FuncIndex, InterfaceIndex, get_ident};
use crate::ast::config::Config;
use crate::ast::expression::Expression;
use crate::ast::statement::Variable;
use crate::ast::symbol_name::SymbolName;
use crate::ast::{EnumBool, SymbolTable};
use crate::error::{CompileError, IllegalUseError, ResolveError, ice::ice};
use crate::parser::out::{
    Spanned, TempExpr,
    TempExpr::{Member, Path},
};

impl Expression {
    pub fn search_variable(
        name: &String,
        symbols: &SymbolTable,
        context: &Option<&HashMap<String, Rc<Variable>>>,
    ) -> Option<Rc<Variable>> {
        if let Some(ctx) = context {
            match ctx.get(name) {
                Some(var) => return Some(Rc::clone(var)),
                None => {}
            }
        }

        if let Some(var) = symbols.globals.get(name) {
            return Some(Rc::clone(var));
        }

        None
    }

    pub fn search_interface(
        config: &mut Config,
        temp_expr: Spanned<TempExpr>,
        symbols: &mut SymbolTable,
        context: Option<&HashMap<String, Rc<Variable>>>,
    ) -> Option<(Expression, InterfaceIndex)> {
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
                let interface_name = SymbolName::member(&owner_type.format(&symbols.types), &name);
                Some((
                    owner,
                    symbols
                        .interfaces
                        .get_by_name(&interface_name)
                        .unwrap_or_else(|| {
                            config.submit_error(
                                CompileError::Resolve(ResolveError::UnknownInterface),
                                span,
                            );
                            InterfaceIndex::empty()
                        }),
                ))
            }
            _ => ice("Interface search cannot be performed on non-member expressions."),
        }
    }

    pub fn search_function(
        config: &mut Config,
        temp_expr: Spanned<TempExpr>,
        symbols: &mut SymbolTable,
        context: Option<&HashMap<String, Rc<Variable>>>,
    ) -> Option<EnumBool<InterfaceIndex, FuncIndex>> {
        let (expr, span) = temp_expr;
        match expr {
            Path(path) => {
                let name = SymbolName::path(&path.segments);
                let interface = symbols.interfaces.get_by_name(&name);
                match interface {
                    Some(interface_index) => Some(EnumBool::True(interface_index)),
                    None => {
                        let func = symbols.functions.get_by_name(&name);
                        match func {
                            Some(func_index) => Some(EnumBool::False(func_index)),
                            None => {
                                config.submit_error(
                                    CompileError::Resolve(ResolveError::UnknownFunction),
                                    span,
                                );
                                Some(EnumBool::False(FuncIndex::empty()))
                            }
                        }
                    }
                }
            }
            _ => ice("Function search cannot be performed on non-path expressions."),
        }
    }

    pub fn search_generic_function(
        config: &mut Config,
        temp_expr: Spanned<TempExpr>,
        symbols: &mut SymbolTable,
        context: Option<&HashMap<String, Rc<Variable>>>,
    ) -> Option<EnumBool<InterfaceIndex, FuncIndex>> {
        let (expr, span) = temp_expr;
        match expr {
            Path(path) => {
                let name = SymbolName::path(&path.segments);
                let ident = get_ident(&name);
                let interface = symbols.generics.interfaces.get_by_name(&ident);
                if let Some(interface) = interface {
                    return Some(EnumBool::True(interface));
                };
                let function = symbols.generics.functions.get_by_name(&ident);
                if let Some(func) = function {
                    Some(EnumBool::False(func))
                } else {
                    config.submit_error(CompileError::Resolve(ResolveError::UnknownFunction), span);
                    None
                }
            }
            _ => ice("Generic function search cannot be performed on non-path expressions."),
        }
    }
}
