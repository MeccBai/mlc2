use super::*;
use crate::diagnostic::error::{
    CompileError, ErrorHandle, ErrorInfo, IllegalUseError, ResolveError,
};
use crate::diagnostic::warning::WarningHandle;
use crate::parser::out::TempPath;

fn config() -> Config {
    Config::new(
        crate::ast::config::FileId::new(0),
        Vec::new(),
        String::new(),
        String::new(),
        ErrorHandle::new("test".into()),
        WarningHandle::new("test".into()),
    )
}

#[test]
fn unknown_type_submits_resolution_error_at_type_span() {
    let mut config = config();
    let mut symbols = SymbolTable::new();
    let span = (10..17).into();
    let path = TempPath {
        segments: vec!["Missing".into()],
    };
    assert_eq!(
        resolve_type(
            &mut config,
            (TempType::Path(path), span),
            &mut symbols,
            None
        ),
        None
    );
    assert!(config.error_handle().errors.contains(&ErrorInfo::new(
        CompileError::Resolve(ResolveError::UnknownType),
        span,
    )));
}

#[test]
fn generic_argument_count_submits_illegal_use_error() {
    let mut config = config();
    let mut symbols = SymbolTable::new();
    let name = config.symbol_name("Box");
    let mut unit = UnitType::empty();
    unit.generics.push(String::new());
    symbols.generics.units.insert(get_ident(&name), unit);
    let span = (20..27).into();
    let ty = TempType::Generic {
        base: TempPath {
            segments: vec!["Box".into()],
        },
        args: Vec::new(),
    };
    assert_eq!(
        resolve_type(&mut config, (ty, span), &mut symbols, None),
        None
    );
    assert!(config.error_handle().errors.contains(&ErrorInfo::new(
        CompileError::IllegalUse(IllegalUseError::GenericCountMismatch),
        span,
    )));
}

mod value;
