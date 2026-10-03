use super::*;
use std::fs;

#[test]
fn binary_temp_roundtrip_preserves_recursive_nodes_and_rejects_corruption() {
    let module = parse(
        "unit P<T> { pub a:T; }; func<T> f(x:T) -> T { if (true) { return x; } return x; } func main() { var a = [1,2]; var p = P<i32>{1}; var s = \"hello\"; for i in [0,2] { a[i] = i; } match(1) { 1 => { return; } } }",
    );
    let bytes = to_binary(&module).unwrap();
    assert_eq!(from_binary::<TempModule>(&bytes).unwrap(), module);
    let text = mlc_syntax::serialization::to_toml(&GenericBundle {
        format_version: FORMAT_VERSION,
        source_file: "test.m2".into(),
        module_name: "test".into(),
        module: module.clone(),
    })
    .unwrap();
    assert!(bytes.len() < text.len());
    for end in [0, 8, 11, bytes.len() - 1] {
        assert!(from_binary::<TempModule>(&bytes[..end]).is_err());
    }
    let mut wrong_version = bytes.clone();
    wrong_version[8] = 0;
    assert!(
        from_binary::<TempModule>(&wrong_version)
            .unwrap_err()
            .contains("version")
    );
    let mut trailing = bytes;
    trailing.push(0);
    assert!(
        from_binary::<TempModule>(&trailing)
            .unwrap_err()
            .contains("Trailing")
    );
}

#[test]
fn toml_conversion_requires_declaration_only_and_preserves_existing_output_on_error() {
    let root = tempfile::tempdir().unwrap();
    let input = root.path().join("io.toml");
    let output = root.path().join("io.sym");
    let text = include_str!("../../../../../lib/universal/c_std/io.toml");
    fs::write(&input, text).unwrap();
    convert_toml(&input, &output).unwrap();
    let expected = fs::read(&output).unwrap();
    assert_eq!(
        load(&output).unwrap().manifest.config.format_version,
        FORMAT_VERSION
    );
    for invalid in [
        text.replace("Object = \"None\"", "Object = \"Only\""),
        text.replace("FormatVersion = 2", "FormatVersion = 999"),
        "not a valid TOML [[[".into(),
    ] {
        fs::write(&input, invalid).unwrap();
        assert!(convert_toml(&input, &output).is_err());
        assert_eq!(fs::read(&output).unwrap(), expected);
    }
}
