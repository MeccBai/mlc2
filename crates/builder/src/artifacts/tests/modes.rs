use super::*;
use mlc_syntax::serialization::to_toml;
use std::fs;

fn fixture(mode: ObjectMode) -> (tempfile::TempDir, ArtifactPaths, Manifest, GenericBundle) {
    let root = tempfile::tempdir().unwrap();
    let source = "func<T> f(x:T) -> T { return x; }";
    let source_path = root.path().join("test.m2");
    fs::write(&source_path, source).unwrap();
    let module = parse(source);
    let mut manifest = Manifest {
        config: Metadata {
            format_version: FORMAT_VERSION,
            object: mode,
            source_file: source_path.to_str().unwrap().into(),
            module_name: "test".into(),
            file_hash: content_hash(source.as_bytes()),
            generic_hash: semantic_hash(&module).unwrap(),
            ..Metadata::default()
        },
        symbols: Symbols::from_module(&module),
        requires: vec![],
    };
    manifest.config.symbols_hash = manifest.computed_symbols_hash().unwrap();
    let bundle = GenericBundle {
        format_version: FORMAT_VERSION,
        source_file: manifest.config.source_file.clone(),
        module_name: "test".into(),
        module,
    };
    let paths = ArtifactPaths::new(&root.path().join("test"), "windows");
    (root, paths, manifest, bundle)
}

#[test]
fn source_checks_source_object_declarations_and_templates() {
    for component in ["source", "object", "symbols", "templates"] {
        let (_root, paths, manifest, bundle) = fixture(ObjectMode::Source);
        let artifact = publish(&paths, manifest, Some(bundle), Some(b"object")).unwrap();
        match component {
            "source" => fs::write(&artifact.manifest.config.source_file, "changed").unwrap(),
            "object" => fs::write(&paths.object, "changed").unwrap(),
            "templates" => fs::write(&paths.generic, "changed").unwrap(),
            _ => {
                let mut manifest = artifact.manifest;
                manifest.config.symbols_hash = "wrong".into();
                fs::write(&paths.manifest, to_toml(&manifest).unwrap()).unwrap();
            }
        }
        assert!(load(&paths.manifest).is_err(), "{component}");
    }
}

#[test]
fn only_ignores_source_symbols_and_object_hash_but_checks_templates() {
    let (_root, paths, mut manifest, bundle) = fixture(ObjectMode::Only);
    fs::remove_file(&manifest.config.source_file).unwrap();
    manifest.config.file_hash = "wrong".into();
    manifest.config.symbols_hash = "wrong".into();
    let initial = publish(&paths, manifest, Some(bundle), Some(b"object")).unwrap();
    fs::write(&paths.object, "modified binary").unwrap();
    let changed = load(&paths.manifest).unwrap();
    assert_ne!(initial.object_hash, changed.object_hash);
    fs::write(&paths.generic, "modified templates").unwrap();
    assert!(load(&paths.manifest).is_err());
}

#[test]
fn none_ignores_all_hashes_and_missing_source() {
    let (_root, paths, mut manifest, _bundle) = fixture(ObjectMode::None);
    fs::remove_file(&manifest.config.source_file).unwrap();
    manifest.config.file_hash = "wrong".into();
    manifest.config.symbols_hash = "wrong".into();
    manifest.config.generic_hash = "wrong".into();
    manifest.config.object_hash = Some("wrong".into());
    manifest.config.generic_file_hash = Some("wrong".into());
    let artifact = publish(&paths, manifest, None, None).unwrap();
    assert!(artifact.templates.is_none());
    assert!(artifact.object.is_none());
    let hash = artifact.manifest.dependency_hash().unwrap();
    assert_ne!(hash.symbols_hash, "wrong");
    assert!(hash.generic_hash.is_empty());
}

#[test]
fn none_rejects_binary_payload_and_only_still_rejects_path_traversal() {
    let (_root, paths, manifest, _bundle) = fixture(ObjectMode::None);
    assert!(publish(&paths, manifest, None, Some(b"object")).is_err());
    let (_root, paths, mut manifest, _bundle) = fixture(ObjectMode::Only);
    manifest.config.generic_hash.clear();
    manifest.config.object_file = Some("../outside.obj".into());
    fs::write(&paths.manifest, to_toml(&manifest).unwrap()).unwrap();
    assert!(load(&paths.manifest).is_err());
}

#[test]
fn only_requires_both_template_hashes() {
    for missing_file_hash in [false, true] {
        let (_root, paths, manifest, bundle) = fixture(ObjectMode::Only);
        let mut manifest = publish(&paths, manifest, Some(bundle), None)
            .unwrap()
            .manifest;
        if missing_file_hash {
            manifest.config.generic_file_hash = None;
        } else {
            manifest.config.generic_hash = "wrong".into();
        }
        fs::write(&paths.manifest, to_toml(&manifest).unwrap()).unwrap();
        assert!(load(&paths.manifest).is_err());
    }
}
