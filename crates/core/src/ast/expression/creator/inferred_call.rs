//! Infer only from argument types; reuse normal instantiation and argument validation.
use crate::ast::{
    GenericIndex, TypeIndex,
    arena::FuncIndex,
    config::Config,
    expression::{Expression, FuncCall},
    symbols::{EnumBool, Resolution, StatementContext},
    types::{CompileType, ValueType},
};
use crate::diagnostic::error::{CompileError, IllegalUseError};
use crate::parser::out::{Span, Spanned, TempExpr};
use std::collections::HashMap;

impl Expression {
    pub(super) fn new_inferred_call(
        config: &mut Config,
        template: FuncIndex,
        args: Vec<Spanned<TempExpr>>,
        span: Span,
        symbols: &mut dyn Resolution,
        context: Option<&StatementContext>,
    ) -> Self {
        let symbol = symbols.get_function(template, true).clone();
        let variadic = symbol.params.last().is_some_and(|(_, name)| name == "...");
        let fixed = symbol.params.len() - usize::from(variadic);
        if args.len() < fixed || (!variadic && args.len() != fixed) {
            config.submit_error(
                CompileError::IllegalUse(IllegalUseError::ArgumentCountMismatch),
                span,
            );
            return Self::Poison;
        }
        let args: Vec<_> = args
            .into_iter()
            .map(|arg| Self::new(config, arg, symbols, context))
            .collect();
        if config.is_poisoned() {
            return Self::Poison;
        }
        let mut bindings = HashMap::new();
        for ((expected, _), argument) in symbol.params.iter().take(fixed).zip(&args) {
            let found = argument.type_inference(config, symbols);
            if config.is_poisoned() {
                return Self::Poison;
            }
            if found.is_empty() {
                config.submit_error(
                    CompileError::Resolve(crate::diagnostic::error::ResolveError::MissingType),
                    span,
                );
                return Self::Poison;
            }
            if let Err(error) = infer(
                *expected,
                found,
                &mut bindings,
                &symbol.generic_map,
                symbols,
            ) {
                config.submit_error(CompileError::IllegalUse(error), span);
                return Self::Poison;
            }
        }
        for name in &symbol.generics {
            if !bindings.contains_key(&symbol.generic_map[name]) {
                config.submit_error(
                    CompileError::IllegalUse(IllegalUseError::GenericInferenceMissing {
                        name: name.clone(),
                    }),
                    span,
                );
                return Self::Poison;
            }
        }
        let index = template.instantiation(
            config,
            &bindings,
            symbols,
            context.and_then(|ctx| ctx.instantiation_actives()),
            span,
        );
        if config.is_poisoned() || !Self::check_function_args(config, index, &args, symbols, span) {
            return Self::Poison;
        }
        Self::FuncCallE(FuncCall {
            callee: None,
            func: EnumBool::False(index),
            args,
        })
    }
}

fn infer(
    expected: TypeIndex,
    found: TypeIndex,
    bindings: &mut HashMap<GenericIndex, TypeIndex>,
    names: &HashMap<String, GenericIndex>,
    symbols: &mut dyn Resolution,
) -> Result<(), IllegalUseError> {
    let target = symbols.get_type(expected).unqualified().clone();
    let actual = symbols.get_type(found).unqualified().clone();
    match (target, actual) {
        (CompileType::Generic(index), _) if names.values().any(|value| *value == index) => {
            let found = found.into_value_type(ValueType::Flex, symbols);
            if let Some(previous) = bindings.get(&index) {
                if !previous.type_check(false, &found, symbols) {
                    return Err(IllegalUseError::GenericInferenceConflict {
                        name: names
                            .iter()
                            .find(|(_, value)| **value == index)
                            .unwrap()
                            .0
                            .clone(),
                        expected: previous.format(symbols),
                        found: found.format(symbols),
                    });
                }
            } else {
                bindings.insert(index, found);
            }
            Ok(())
        }
        (CompileType::Ref(a), CompileType::Ref(b))
            if (!a.ownership || b.ownership)
                && a.level == b.level
                && (!a.mut_base || b.mut_base) =>
        {
            infer(a.base, b.base, bindings, names, symbols)
        }
        (CompileType::List(a), CompileType::List(b)) if a.length == b.length => {
            infer(a.element_type, b.element_type, bindings, names, symbols)
        }
        (CompileType::Function(a), CompileType::Function(b))
            if a.params.len() == b.params.len()
                && a.c_abi == b.c_abi
                && a.variadic == b.variadic
                && a.returns.is_some() == b.returns.is_some() =>
        {
            for (expected, found) in a
                .params
                .into_iter()
                .zip(b.params)
                .chain(a.returns.into_iter().zip(b.returns))
            {
                infer(expected, found, bindings, names, symbols)?;
            }
            Ok(())
        }
        (CompileType::Unit(a), CompileType::Unit(b))
            if a.application.is_some() && b.application.is_some() =>
        {
            let a = a.application.unwrap();
            let b = b.application.unwrap();
            if a.template == b.template && a.arguments.len() == b.arguments.len() {
                for (a, b) in a.arguments.into_iter().zip(b.arguments) {
                    infer(a, b, bindings, names, symbols)?;
                }
                return Ok(());
            }
            Err(mismatch(expected, found, symbols))
        }
        _ if expected.type_check(false, &found, symbols) => Ok(()),
        _ => Err(mismatch(expected, found, symbols)),
    }
}

fn mismatch(expected: TypeIndex, found: TypeIndex, symbols: &dyn Resolution) -> IllegalUseError {
    IllegalUseError::TypeMismatched {
        expected: expected.format(symbols),
        found: found.format(symbols),
    }
}
