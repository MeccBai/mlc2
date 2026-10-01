use super::*;
use crate::diagnostic::error::{ErrorHandle, ErrorInfo};
use crate::diagnostic::warning::WarningHandle;
use crate::parser::out::TempPath;

#[test]
fn unknown_interface_owner_reports_owner_span() {
    let mut config = Config::new(
        crate::ast::config::FileId::new(0),
        Vec::new(),
        String::new(),
        String::new(),
        ErrorHandle::new("test".into()),
        WarningHandle::new("test".into()),
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
        name_span: span,
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
