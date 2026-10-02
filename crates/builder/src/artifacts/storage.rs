use super::*;
use mlc_syntax::serialization::{from_toml, to_toml};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone)]
pub struct ArtifactPaths {
    pub manifest: PathBuf,
    pub object: PathBuf,
    pub generic: PathBuf,
}
impl ArtifactPaths {
    pub fn new(stem: &Path, target: &str) -> Self {
        Self {
            manifest: stem.with_extension("toml"),
            object: stem.with_extension(if target.contains("windows") {
                "obj"
            } else {
                "o"
            }),
            generic: stem.with_extension("mg"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Artifact {
    pub warnings: Vec<String>,
    pub path: PathBuf,
    pub manifest: Manifest,
    pub templates: Option<GenericBundle>,
    pub object: Option<PathBuf>,
    pub object_hash: Option<String>,
}

pub fn load(path: &Path) -> Result<Artifact, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let manifest: Manifest = from_toml(&text)?;
    if manifest.config.format_version != FORMAT_VERSION {
        return Err("Unsupported artifact format version".into());
    }
    if manifest.config.object == ObjectMode::Source
        && manifest.computed_symbols_hash()? != manifest.config.symbols_hash
    {
        return Err("Declaration hash mismatch".into());
    }
    if manifest.config.object == ObjectMode::Source {
        let source = fs::read(&manifest.config.source_file)
            .map_err(|e| format!("Source unavailable: {e}"))?;
        if content_hash(&source) != manifest.config.file_hash {
            return Err("Source hash mismatch".into());
        }
    }
    let root = path.parent().ok_or("Manifest has no parent")?;
    if manifest.config.object == ObjectMode::Source
        && (manifest.config.object_file.is_some() != manifest.config.object_hash.is_some()
            || manifest.config.generic.is_some() != manifest.config.generic_file_hash.is_some())
    {
        return Err("Incomplete artifact hash metadata".into());
    }
    if manifest.config.object == ObjectMode::None
        && (manifest.config.object_file.is_some() || manifest.config.generic.is_some())
    {
        return Err("Declaration-only artifacts cannot contain objects or templates".into());
    }
    let object = match &manifest.config.object_file {
        Some(name) => {
            let file = referenced_file(root, name)?;
            if manifest.config.object == ObjectMode::Source {
                let hash = manifest
                    .config
                    .object_hash
                    .as_deref()
                    .ok_or("Missing object hash")?;
                checked_file(root, name, hash)?;
            }
            Some(file)
        }
        None => None,
    };
    let templates = match &manifest.config.generic {
        Some(name) => {
            let file = referenced_file(root, name)?;
            if let Some(hash) = &manifest.config.generic_file_hash {
                checked_file(root, name, hash)?;
            } else {
                return Err("Missing generic file hash".into());
            }
            let bundle: GenericBundle =
                from_toml(&fs::read_to_string(file).map_err(|e| e.to_string())?)?;
            if bundle.format_version != FORMAT_VERSION
                || bundle.module_name != manifest.config.module_name
                || (manifest.config.object == ObjectMode::Source
                    && bundle.source_file != manifest.config.source_file)
                || semantic_hash(&bundle.module)? != manifest.config.generic_hash
            {
                return Err("Generic metadata/hash mismatch".into());
            }
            Some(bundle)
        }
        None if manifest.config.object == ObjectMode::None
            || manifest.config.generic_hash.is_empty() =>
        {
            None
        }
        _ => return Err("Incomplete generic metadata".into()),
    };
    let object_hash = object
        .as_ref()
        .map(|file| {
            fs::read(file)
                .map(|bytes| content_hash(&bytes))
                .map_err(|e| e.to_string())
        })
        .transpose()?;
    Ok(Artifact {
        warnings: vec![],
        path: path.to_owned(),
        manifest,
        templates,
        object,
        object_hash,
    })
}

fn checked_file(root: &Path, name: &str, expected: &str) -> Result<PathBuf, String> {
    let path = referenced_file(root, name)?;
    let bytes = fs::read(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    if content_hash(&bytes) != expected {
        return Err(format!("Artifact hash mismatch: {}", path.display()));
    }
    Ok(path)
}

fn referenced_file(root: &Path, name: &str) -> Result<PathBuf, String> {
    // Produced references are sibling filenames, never absolute or traversal paths.
    let path = Path::new(name);
    if path.components().count() != 1
        || !matches!(
            path.components().next(),
            Some(std::path::Component::Normal(_))
        )
    {
        return Err("Artifact reference must be a sibling filename".into());
    }
    let path = root.join(path);
    if !path.is_file() {
        return Err(format!("Artifact file not found: {}", path.display()));
    }
    Ok(path)
}

/// Publish data first, manifest last. A failed publication never looks like a valid cache hit.
pub fn publish(
    paths: &ArtifactPaths,
    mut manifest: Manifest,
    templates: Option<GenericBundle>,
    object: Option<&[u8]>,
) -> Result<Artifact, String> {
    if manifest.config.object == ObjectMode::None && (object.is_some() || templates.is_some()) {
        return Err("Declaration-only publication cannot contain objects or templates".into());
    }
    if let Some(bytes) = object {
        write_atomic(&paths.object, bytes)?;
        manifest.config.object_file = Some(filename(&paths.object)?);
        manifest.config.object_hash = Some(content_hash(bytes));
    }
    if let Some(bundle) = &templates {
        let text = to_toml(bundle)?;
        write_atomic(&paths.generic, text.as_bytes())?;
        manifest.config.generic = Some(filename(&paths.generic)?);
        manifest.config.generic_file_hash = Some(content_hash(text.as_bytes()));
    }
    write_atomic(&paths.manifest, to_toml(&manifest)?.as_bytes())?;
    load(&paths.manifest)
}

fn filename(path: &Path) -> Result<String, String> {
    path.file_name()
        .and_then(|v| v.to_str())
        .map(str::to_owned)
        .ok_or("Non-UTF8 artifact filename".into())
}
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let root = path.parent().ok_or("Artifact has no parent")?;
    fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let mut file = tempfile::NamedTempFile::new_in(root).map_err(|e| e.to_string())?;
    file.write_all(bytes)
        .and_then(|_| file.as_file().sync_all())
        .map_err(|e| e.to_string())?;
    file.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}
