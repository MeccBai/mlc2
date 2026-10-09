use super::*;
use crate::diagnostic::error::ErrorHandle;
use crate::diagnostic::warning::WarningHandle;
use crate::parser::out::{TempGenericParam, TempPath, TempType, TempVisibility};

#[test]
fn unconstrained_generic_is_kept_and_can_be_instantiated() {
    let mut config = Config::new(
        crate::ast::config::FileId::new(0),
        Vec::new(),
        String::new(),
        String::new(),
        ErrorHandle::new("test".into()),
        WarningHandle::new("test".into()),
    );
    let mut symbols = SymbolTable::new();
    let span = (0..1).into();
    let name = config.symbol_name("Box");
    let prototype = TempUnit {
        is_union: false,
        visibility: TempVisibility::Private,
        name: name.clone(),
        name_span: span,
        generics: vec![TempGenericParam {
            name: "T".into(),
            name_span: span,
            constraint: None,
        }],
        members: Vec::new(),
        attributes: Vec::new(),
    };
    let (unit, _) = UnitType::finalize(&mut config, prototype, &mut symbols);
    assert_eq!(unit.generics, ["T"]);
    let requirement = unit.generic_map["T"];
    assert!(
        symbols
            .generics
            .requires
            .get(requirement)
            .requires
            .is_empty()
    );
    symbols.generics.units.insert(get_ident(&name), unit);

    let arg = TempType::Generic {
        base: TempPath {
            segments: vec!["Box".into()],
        },
        args: vec![(
            TempType::Path(TempPath {
                segments: vec!["bool".into()],
            }),
            span,
        )],
    };
    assert!(resolve_type(&mut config, (arg, span), &mut symbols, None).is_some());
    assert!(config.error_handle().errors.is_empty());
}
