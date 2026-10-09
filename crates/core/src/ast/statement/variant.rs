use super::*;
use crate::ast::expression::operators::Operator;
use crate::ast::{
    config::Config,
    symbols::{Resolution, StatementContext, SupperScopeType},
    types::{CompileType, resolve_type},
};
use crate::diagnostic::error::{CompileError, IllegalUseError};
use crate::parser::out::{Spanned, TempExpr, TempScope, TempType};

impl Statement {
    pub(super) fn create_variant_match(
        config: &mut Config,
        binding: Spanned<String>,
        value: Spanned<TempExpr>,
        branches: Vec<(Spanned<TempType>, TempScope)>,
        symbols: &mut dyn Resolution,
        context: Option<&StatementContext>,
    ) -> Self {
        let span = value.1;
        if !matches!(
            &value.0,
            TempExpr::Unary {
                op: Operator::AddressOf | Operator::MutOf,
                ..
            }
        ) {
            config.submit_error(
                CompileError::IllegalUse(IllegalUseError::InvalidUnion {
                    reason: "Union match requires a borrowed value: y = @x or y = @mut x".into(),
                }),
                span,
            );
            return Self::Poison;
        }
        if matches!(&value.0, TempExpr::Unary { value, .. } if !stable_temp_place(&value.0)) {
            config.submit_error(
                CompileError::IllegalUse(IllegalUseError::InvalidUnion {
                    reason: "Union match requires an existing value, not a temporary".into(),
                }),
                span,
            );
            return Self::Poison;
        }
        let value = Expression::new(config, value, symbols, context);
        if config.is_poisoned() {
            return Self::Poison;
        }
        let Expression::UnaryExprE(crate::ast::expression::UnaryExpr::Operator {
            value: borrowed,
            ..
        }) = &value
        else {
            return Self::Poison;
        };
        if !stable_place(&borrowed.clone().to_expression()) {
            config.submit_error(
                CompileError::IllegalUse(IllegalUseError::InvalidUnion {
                    reason: "Union match requires an existing value, not a temporary".into(),
                }),
                span,
            );
            return Self::Poison;
        }
        let ty = value.type_inference(config, symbols);
        let CompileType::Ref(reference) = symbols.get_type(ty).unqualified() else {
            unreachable!()
        };
        if reference.level != 1 {
            config.submit_error(CompileError::IllegalUse(IllegalUseError::InvalidUnion { reason: "Union match requires a reference to the union itself; dereference existing references before borrowing".into() }), span);
            return Self::Poison;
        }
        let owner = reference.base;
        let mutable = reference.mut_base;
        let Some(variant) = (match symbols.get_type(owner).unqualified() {
            CompileType::Unit(unit) => unit.variant.clone(),
            _ => None,
        }) else {
            config.submit_error(
                CompileError::IllegalUse(IllegalUseError::InvalidUnion {
                    reason: "Borrowed type match requires a union".into(),
                }),
                span,
            );
            return Self::Poison;
        };
        let mut parsed = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for (candidate, scope) in branches {
            let candidate_span = candidate.1;
            let Some(candidate) = resolve_type(
                config,
                candidate,
                symbols,
                context.map(|ctx| ctx as &dyn crate::ast::types::TypeContext),
            ) else {
                return Self::Poison;
            };
            let Some(tag) = variant.candidates.iter().position(|ty| {
                symbols.get_type(*ty).unqualified().format(symbols)
                    == symbols.get_type(candidate).unqualified().format(symbols)
            }) else {
                config.submit_error(
                    CompileError::IllegalUse(IllegalUseError::InvalidUnion {
                        reason: format!(
                            "{} is not a candidate of {}",
                            candidate.format(symbols),
                            owner.format(symbols)
                        ),
                    }),
                    candidate_span,
                );
                return Self::Poison;
            };
            if !seen.insert(tag) {
                config.submit_error(
                    CompileError::IllegalUse(IllegalUseError::InvalidUnion {
                        reason: "Duplicate union match branch".into(),
                    }),
                    candidate_span,
                );
                return Self::Poison;
            }
            let var_type = candidate.make_ref(symbols, mutable);
            let variable = Rc::new(Variable {
                read_count: Default::default(),
                declaration_span: binding.1,
                name: binding.0.clone(),
                var_type,
                init_val: Box::new(Expression::null()),
            });
            let mut child = context.cloned().unwrap_or_else(|| {
                StatementContext::new(crate::ast::symbols::EnumBool::False(
                    crate::ast::FuncIndex::empty(),
                ))
            });
            child.push_scope(SupperScopeType::Match);
            if !child.declare(config, variable.clone(), binding.1) {
                return Self::Poison;
            }
            let body = Self::parse_scope(config, scope, symbols, &mut child);
            if config.is_poisoned() {
                return Self::Poison;
            }
            parsed.push((tag, variable, body));
        }
        if seen.len() != variant.candidates.len() {
            let missing = variant
                .candidates
                .iter()
                .enumerate()
                .filter(|(tag, _)| !seen.contains(tag))
                .map(|(_, ty)| ty.format(symbols))
                .collect::<Vec<_>>()
                .join(", ");
            config.submit_error(
                CompileError::IllegalUse(IllegalUseError::InvalidUnion {
                    reason: format!("Union match must be exhaustive; missing: {missing}"),
                }),
                span,
            );
            return Self::Poison;
        }
        Self::VariantMatch(VariantMatch {
            value: Box::new(value),
            owner,
            branches: parsed,
        })
    }
}

fn stable_place(value: &Expression) -> bool {
    use crate::ast::expression::{Access, UnaryExpr};
    match value {
        Expression::VarValueE(_) => true,
        Expression::UnaryExprE(UnaryExpr::Access(Access::Member { base, indirect, .. })) => {
            *indirect || stable_place(&base.clone().to_expression())
        }
        Expression::UnaryExprE(UnaryExpr::Access(Access::Index { base, .. })) => stable_place(base),
        Expression::UnaryExprE(UnaryExpr::Operator {
            op: Operator::Dereference,
            ..
        }) => true,
        _ => false,
    }
}

fn stable_temp_place(value: &TempExpr) -> bool {
    match value {
        TempExpr::Path(_) => true,
        TempExpr::Group(value) => stable_temp_place(&value.0),
        TempExpr::Member { base, indirect, .. } => *indirect || stable_temp_place(&base.0),
        TempExpr::Unary {
            op: Operator::Dereference,
            ..
        } => true,
        TempExpr::Binary {
            operands,
            operators,
        } if operators == &[Operator::Index] => operands
            .first()
            .is_some_and(|base| stable_temp_place(&base.0)),
        _ => false,
    }
}
