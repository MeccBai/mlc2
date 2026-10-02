use mlc_syntax::manifest::SOURCE_SUFFIX;

use super::*;

#[test]
fn recursive_units_share_a_monotonic_file_id_allocator() {
    let global = GlobalConfig::new();
    assert_eq!(global.next_file_id(), FileId::new(0));
    let nested = global.clone();
    assert_eq!(nested.next_file_id(), FileId::new(1));
    let config = global.config(
        Vec::new(),
        "unit".into(),
        format!("unit{}", SOURCE_SUFFIX),
        ErrorHandle::new(format!("unit{}", SOURCE_SUFFIX)),
        WarningHandle::new(format!("unit{}", SOURCE_SUFFIX)),
    );
    assert_eq!(config.file_id(), FileId::new(2));
    assert_eq!(nested.next_file_id(), FileId::new(3));
}

#[test]
fn file_identity_is_explicit_and_survives_cloning() {
    let config = Config::new(
        FileId::new(7),
        Vec::new(),
        "unit".into(),
        format!("unit{}", SOURCE_SUFFIX),
        ErrorHandle::new(format!("unit{}", SOURCE_SUFFIX)),
        WarningHandle::new(format!("unit{}", SOURCE_SUFFIX)),
    );
    assert_eq!(config.file_id(), FileId::new(7));
    assert_eq!(config.clone().file_id(), config.file_id());
    assert_ne!(config.file_id(), FileId::new(8));
}
