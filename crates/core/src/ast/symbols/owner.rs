//! Instantiate under the definition's namespace and import permissions.
use super::Resolution;
use crate::ast::config::{Config, FileId};
use crate::diagnostic::error::{CompileError, IllegalUseError};
use crate::parser::out::Span;

pub(crate) fn in_owner<R>(
    config: &mut Config,
    symbols: &mut dyn Resolution,
    owner: FileId,
    exported: bool,
    span: Span,
    work: impl FnOnce(&mut Config, &mut dyn Resolution) -> R,
) -> Option<R> {
    if config.is_poisoned() {
        return None;
    }
    if symbols.local().types.file_id() == owner {
        return Some(work(config, symbols));
    }
    if !exported {
        config.submit_error(
            CompileError::IllegalUse(IllegalUseError::PrivateInstantiation),
            span,
        );
        return None;
    }
    let Some(mut definition) = symbols.file_config(owner) else {
        config.submit_error(
            CompileError::IllegalUse(IllegalUseError::InvalidExportTable),
            span,
        );
        return None;
    };
    let Some(previous) = symbols.select_file(owner) else {
        config.submit_error(
            CompileError::IllegalUse(IllegalUseError::InvalidExportTable),
            span,
        );
        return None;
    };
    let result = work(&mut definition, symbols);
    symbols.restore_scope(previous);
    // The template spans refer to another source. Report at the instantiation
    // site rather than handing a foreign span to the caller's diagnostic printer.
    if let Some(error) = definition.error_handle().first_error() {
        config.submit_error(error, span);
        None
    } else {
        Some(result)
    }
}
