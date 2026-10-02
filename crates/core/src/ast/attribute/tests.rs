use super::*;
use crate::ast::config::{Config, FileId};
use crate::diagnostic::{
    error::ErrorHandle,
    warning::{WarningHandle, WarningInfo},
};

fn config() -> Config {
    Config::new(
        FileId::new(0),
        vec![],
        "test".into(),
        "test".into(),
        ErrorHandle::new("test".into()),
        WarningHandle::new("test".into()),
    )
}

#[test]
fn known_attributes_are_deduplicated_and_unknown_attributes_are_not_stored() {
    let mut config = config();
    let span = (2..7).into();
    let names = vec!["c_abi".into(), "c_abi".into(), "unknown".into()];
    assert_eq!(
        FuncAttibute::parse(&mut config, names.clone(), span),
        HashSet::from([FuncAttibute::Cabi])
    );
    assert_eq!(
        UnitAttribute::parse(&mut config, names, span),
        HashSet::from([UnitAttribute::Cabi])
    );
    assert!(
        config
            .warning_handle()
            .warnings
            .contains(&WarningInfo::new(Warning::AttributeNotFound, span))
    );
    assert!(!config.is_poisoned());
    assert!(config.error_handle().errors.is_empty());
}

#[test]
fn warnings_at_different_positions_remain_distinct() {
    let mut config = config();
    FuncAttibute::new(&mut config, "unknown", (1..2).into());
    UnitAttribute::new(&mut config, "unknown", (3..4).into());
    assert_eq!(config.warning_handle().warnings.len(), 2);
}
