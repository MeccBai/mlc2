use crate::{
    ast::config::Config,
    diagnostic::error::{CompileError, IllegalUseError},
    parser::out::{Span, TempVisibility},
};

pub trait TempVisibilityExt {
    fn normal_export(self, config: &mut Config, span: &Span) -> bool;
}
impl TempVisibilityExt for TempVisibility {
    fn normal_export(self, config: &mut Config, span: &Span) -> bool {
        match self {
            Self::Export => true,
            Self::Private => false,
            _ => {
                config.submit_error(
                    CompileError::IllegalUse(IllegalUseError::IllegalVisibility),
                    *span,
                );
                false
            }
        }
    }
}
