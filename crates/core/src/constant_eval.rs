use crate::ast::{
    Config,
    expression::{Expression, FuncCall},
    symbols::{EnumBool, Resolution},
};
use crate::diagnostic::error::{CompileError, IllegalUseError};
use crate::parser::out::Span;
use std::{cell::RefCell, rc::Rc};

pub trait Executor {
    fn evaluate(
        &self,
        call: &FuncCall,
        symbols: &dyn Resolution,
        config: &mut Config,
        span: Span,
    ) -> Result<Option<Expression>, String>;
}
thread_local! { static EXECUTOR: RefCell<Option<Rc<dyn Executor>>> = RefCell::new(None); }
pub struct Guard(Option<Rc<dyn Executor>>);
pub fn install(executor: Rc<dyn Executor>) -> Guard {
    Guard(EXECUTOR.replace(Some(executor)))
}
impl Drop for Guard {
    fn drop(&mut self) {
        EXECUTOR.with(|slot| {
            slot.replace(self.0.take());
        });
    }
}
pub(crate) fn evaluate(
    call: &FuncCall,
    config: &mut Config,
    symbols: &dyn Resolution,
    span: Span,
) -> Option<Expression> {
    let EnumBool::False(index) = call.func else {
        return None;
    };
    if call.callee.is_some()
        || !symbols.get_function_regular(index).constant
        || !call.args.iter().all(|a| a.is_const(symbols))
    {
        return None;
    }
    if let Err((reason, span)) = crate::ast::function::constant::validate_call(call, symbols, span)
    {
        config.submit_error(
            CompileError::IllegalUse(IllegalUseError::ConstantEvaluation { reason }),
            span,
        );
        return Some(Expression::Poison(span));
    }
    if call.args.iter().any(|a| a.contains_todo(symbols)) {
        return None;
    }
    let executor = EXECUTOR.with(|slot| slot.borrow().clone())?;
    match executor.evaluate(call, symbols, config, span) {
        Ok(value) => value.map(|value| value.with_span(span)),
        Err(reason) => {
            config.submit_error(
                CompileError::IllegalUse(IllegalUseError::ConstantEvaluation { reason }),
                span,
            );
            Some(Expression::Poison(span))
        }
    }
}
