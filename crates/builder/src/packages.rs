//! Installed package catalogue and exact-version target requirements.
use crate::project::BuildMode;
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PackageType {
    #[serde(alias = "source")]
    Source,
    #[serde(alias = "static")]
    Static,
    #[serde(alias = "shared")]
    Shared,
}
impl PackageType {
    pub fn directory(self) -> &'static str {
        match self {
            Self::Source => "source",
            Self::Static => "static",
            Self::Shared => "shared",
        }
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinkType {
    #[serde(alias = "static")]
    Static,
    #[serde(alias = "shared", alias = "Dynamic", alias = "dynamic")]
    Shared,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct Requirement {
    pub name: String,
    pub version: String,
    #[serde(rename = "Type")]
    pub kind: PackageType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub link_type: Option<LinkType>,
}
impl<'de> Deserialize<'de> for Requirement {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(rename_all = "PascalCase", deny_unknown_fields)]
        struct Raw {
            name: String,
            version: String,
            #[serde(rename = "Type")]
            kind: PackageType,
            #[serde(default)]
            link_type: Option<LinkType>,
        }
        let raw = Raw::deserialize(d)?;
        component(&raw.name).map_err(serde::de::Error::custom)?;
        version(&raw.version).map_err(serde::de::Error::custom)?;
        if raw.kind != PackageType::Source && raw.link_type.is_some() {
            return Err(serde::de::Error::custom(
                "LinkType is only valid for Type = Source; Static and Shared have fixed linkage",
            ));
        }
        Ok(Self {
            name: raw.name,
            version: raw.version,
            kind: raw.kind,
            link_type: raw.link_type,
        })
    }
}
impl Requirement {
    pub fn linkage(&self) -> LinkType {
        match self.kind {
            PackageType::Static => LinkType::Static,
            PackageType::Shared => LinkType::Shared,
            PackageType::Source => self.link_type.unwrap_or(LinkType::Static),
        }
    }
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", deny_unknown_fields)]
pub struct Registry {
    #[serde(default)]
    pub packages: Vec<Package>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", deny_unknown_fields)]
pub struct Package {
    pub name: String,
    pub version: String,
    #[serde(default)]
    pub requires: Vec<Requirement>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<Source>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub debug: Option<Variants>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub release: Option<Variants>,
}
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", deny_unknown_fields)]
pub struct Variants {
    #[serde(default, rename = "Static", skip_serializing_if = "Option::is_none")]
    pub static_lib: Option<Binary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shared: Option<Binary>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", deny_unknown_fields)]
pub struct Source {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub entry: Option<PathBuf>,
    #[serde(default)]
    pub declaration_only: bool,
    #[serde(default)]
    pub triplet: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", deny_unknown_fields)]
pub struct Binary {
    pub library: PathBuf,
    #[serde(default = "symbols_directory")]
    pub symbols: PathBuf,
    #[serde(default)]
    pub runtime: Option<PathBuf>,
    pub triplet: String,
}
fn symbols_directory() -> PathBuf {
    "modules".into()
}
#[derive(Debug, Clone)]
pub struct Resolved {
    pub requirement: Requirement,
    pub package: Package,
    pub root: PathBuf,
    pub entry: Option<PathBuf>,
    pub symbols: PathBuf,
    pub library: Option<PathBuf>,
    pub runtime: Option<PathBuf>,
    pub profile: BuildMode,
}
pub fn installation(executable: &Path) -> PathBuf {
    std::env::var_os("MLC")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            let root = executable.parent().unwrap_or(Path::new("."));
            if root.file_name().is_some_and(|s| s == "bin") {
                return root.parent().unwrap_or(root).to_owned();
            }
            if !root.join("packages/packages.toml").is_file() {
                // Cargo places development binaries under target/<profile>, with
                // an additional target triple directory for cross builds.
                for target in root.ancestors().skip(1).take(2) {
                    if target.file_name().is_some_and(|name| name == "target")
                        && let Some(workspace) = target.parent()
                        && workspace.join("Cargo.toml").is_file()
                        && workspace.join("packages/packages.toml").is_file()
                    {
                        return workspace.to_owned();
                    }
                }
            }
            root.to_owned()
        })
}
pub fn profile(mode: BuildMode) -> BuildMode {
    if mode.optimized() {
        BuildMode::Release
    } else {
        BuildMode::Debug
    }
}
pub fn component(value: &str) -> Result<(), String> {
    if value.is_empty()
        || !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        Err(format!("Invalid package name: {value}"))
    } else {
        Ok(())
    }
}
pub fn version(value: &str) -> Result<(), String> {
    let parts = value.split('.').collect::<Vec<_>>();
    if parts.len() != 3
        || parts
            .iter()
            .any(|p| p.is_empty() || !p.bytes().all(|c| c.is_ascii_digit()))
    {
        Err(format!(
            "Package Version must be an exact major.minor.patch version: {value}"
        ))
    } else {
        Ok(())
    }
}
pub fn relative(root: &Path, path: &Path) -> Result<PathBuf, String> {
    if path.is_absolute()
        || path
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
    {
        return Err(format!(
            "Package path must stay within its representation directory: {}",
            path.display()
        ));
    }
    Ok(root.join(path))
}
impl Registry {
    pub fn load(root: &Path) -> Result<Self, String> {
        let path = root.join("packages/packages.toml");
        match fs::read_to_string(&path) {
            Ok(text) => {
                let registry: Self =
                    toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
                registry.validate()?;
                Ok(registry)
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(format!("{}: {e}", path.display())),
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        let mut names = std::collections::HashSet::new();
        for p in &self.packages {
            component(&p.name)?;
            version(&p.version)?;
            if !names.insert((&p.name, &p.version)) {
                return Err(format!("Duplicate package {} {}", p.name, p.version));
            }
            if p.source.is_none() && p.debug.is_none() && p.release.is_none() {
                return Err(format!(
                    "Package {} has no Source, Static or Shared representation",
                    p.name
                ));
            }
            if let Some(source) = &p.source {
                if source.declaration_only == source.entry.is_some() {
                    return Err(format!(
                        "Package {} Source needs Entry, or DeclarationOnly=true without Entry",
                        p.name
                    ));
                }
            }
            for v in [&p.debug, &p.release].into_iter().flatten() {
                if v.static_lib.is_none() && v.shared.is_none() {
                    return Err(format!("Package {} profile needs Static or Shared", p.name));
                }
            }
        }
        Ok(())
    }
    pub fn resolve(
        &self,
        root: &Path,
        requirements: &[Requirement],
        mode: BuildMode,
        triplet: &str,
    ) -> Result<Vec<Resolved>, String> {
        let mut result = vec![];
        let mut versions = std::collections::HashMap::new();
        let mut stack = vec![];
        for r in requirements {
            self.visit(
                root,
                r,
                profile(mode),
                triplet,
                &mut versions,
                &mut stack,
                &mut result,
            )?;
        }
        Ok(result)
    }
    fn visit(
        &self,
        root: &Path,
        r: &Requirement,
        mode: BuildMode,
        triplet: &str,
        versions: &mut std::collections::HashMap<String, (String, PackageType, LinkType)>,
        stack: &mut Vec<String>,
        result: &mut Vec<Resolved>,
    ) -> Result<(), String> {
        if stack.contains(&r.name) {
            return Err(format!(
                "Package dependency cycle: {} -> {}",
                stack.join(" -> "),
                r.name
            ));
        }
        let identity = (r.version.clone(), r.kind, r.linkage());
        if let Some(previous) = versions.get(&r.name) {
            if previous != &identity {
                return Err(format!("Conflicting requirements for package {}", r.name));
            }
            return Ok(());
        }
        let p = self
            .packages
            .iter()
            .find(|p| p.name == r.name && p.version == r.version)
            .ok_or_else(|| {
                format!(
                    "Package {} {} is not installed\nExpected registry: {}",
                    r.name,
                    r.version,
                    root.join("packages/packages.toml").display()
                )
            })?;
        let base = root.join("packages").join(&r.name).join(&r.version);
        let directory = if r.kind == PackageType::Source {
            base.join("source")
        } else {
            base.join(mode.directory()).join(r.kind.directory())
        };
        let mut resolved = Resolved {
            requirement: r.clone(),
            package: p.clone(),
            root: directory.clone(),
            entry: None,
            symbols: directory.clone(),
            library: None,
            runtime: None,
            profile: mode,
        };
        match r.kind {
            PackageType::Source => {
                let source = p
                    .source
                    .as_ref()
                    .ok_or_else(|| format!("Package {} has no Source representation", r.name))?;
                if source.triplet.as_deref().is_some_and(|t| t != triplet) {
                    return Err(format!(
                        "Package {} Source is compiled for a different triplet",
                        r.name
                    ));
                }
                if let Some(input) = &source.entry {
                    let entry = relative(&directory, input)?;
                    if !entry.is_file() {
                        return Err(format!(
                            "Package source entry not found\nExpected path: {}",
                            entry.display()
                        ));
                    }
                    resolved.entry = Some(entry);
                } else if !directory.is_dir() {
                    return Err(format!(
                        "Package declarations not found\nExpected path: {}",
                        directory.display()
                    ));
                }
            }
            PackageType::Static | PackageType::Shared => {
                let variants = if mode == BuildMode::Release {
                    p.release.as_ref()
                } else {
                    p.debug.as_ref()
                }
                .ok_or_else(|| {
                    format!(
                        "Package {} {} has no {} profile",
                        r.name,
                        r.version,
                        mode.directory()
                    )
                })?;
                let binary = if r.kind == PackageType::Static {
                    variants.static_lib.as_ref()
                } else {
                    variants.shared.as_ref()
                }
                .ok_or_else(|| format!("Package {} has no {:?} representation", r.name, r.kind))?;
                if binary.triplet != triplet {
                    return Err(format!(
                        "Package {} binary target {} does not match {triplet}",
                        r.name, binary.triplet
                    ));
                }
                let library = relative(&directory, &binary.library)?;
                if !library.is_file() {
                    return Err(format!(
                        "Package library not found\nExpected path: {}",
                        library.display()
                    ));
                }
                resolved.symbols = relative(&directory, &binary.symbols)?;
                resolved.library = Some(library);
                if let Some(runtime) = &binary.runtime {
                    let runtime = relative(&directory, runtime)?;
                    if !runtime.is_file() {
                        return Err(format!(
                            "Package runtime not found\nExpected path: {}",
                            runtime.display()
                        ));
                    }
                    resolved.runtime = Some(runtime);
                }
            }
        }
        versions.insert(r.name.clone(), identity);
        stack.push(r.name.clone());
        for dependency in &p.requires {
            self.visit(root, dependency, mode, triplet, versions, stack, result)?;
        }
        stack.pop();
        result.push(resolved);
        Ok(())
    }
    pub fn save(&self, root: &Path) -> Result<(), String> {
        self.validate()?;
        fs::create_dir_all(root.join("packages")).map_err(|e| e.to_string())?;
        let mut file =
            tempfile::NamedTempFile::new_in(root.join("packages")).map_err(|e| e.to_string())?;
        use std::io::Write;
        file.write_all(
            toml::to_string_pretty(self)
                .map_err(|e| e.to_string())?
                .as_bytes(),
        )
        .map_err(|e| e.to_string())?;
        file.persist(root.join("packages/packages.toml"))
            .map_err(|e| e.to_string())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn catalogue_lives_inside_packages_and_never_reads_the_old_root_file() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("packages.toml"), "obsolete root catalogue").unwrap();
        assert!(Registry::load(root.path()).unwrap().packages.is_empty());
        Registry::default().save(root.path()).unwrap();
        assert!(root.path().join("packages/packages.toml").is_file());
        assert_eq!(
            fs::read_to_string(root.path().join("packages.toml")).unwrap(),
            "obsolete root catalogue"
        );
    }
    #[test]
    fn validates_requirement_linkage() {
        let source: Requirement =
            toml::from_str("Name='demo'\nVersion='1.5.1'\nType='Source'\nLinkType='Shared'")
                .unwrap();
        assert_eq!(source.linkage(), LinkType::Shared);
        for kind in ["Static", "Shared"] {
            let error = toml::from_str::<Requirement>(&format!(
                "Name='demo'\nVersion='1.5.1'\nType='{kind}'\nLinkType='Shared'"
            ))
            .unwrap_err();
            assert!(error.to_string().contains("only valid"));
        }
        assert!(version("1.0").is_err());
        assert!(component("../escape").is_err());
        assert!(relative(Path::new("root"), Path::new("../outside")).is_err());
    }
    #[test]
    fn rejects_empty_profiles_and_dependency_cycles() {
        let mut registry: Registry =
            toml::from_str("[[Packages]]\nName='a'\nVersion='1.0.0'\n[Packages.Debug]").unwrap();
        assert!(registry.validate().is_err());
        let root = tempfile::tempdir().unwrap();
        let directory = root.path().join("packages/a/1.0.0/source");
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("main.m2"), "").unwrap();
        let requirement = Requirement {
            name: "a".into(),
            version: "1.0.0".into(),
            kind: PackageType::Source,
            link_type: None,
        };
        registry.packages[0].debug = None;
        registry.packages[0].source = Some(Source {
            entry: Some("main.m2".into()),
            triplet: None,
            declaration_only: false,
        });
        registry.packages[0].requires.push(requirement.clone());
        assert!(
            registry
                .resolve(
                    root.path(),
                    &[requirement],
                    BuildMode::Default,
                    "x86_64-pc-windows-msvc"
                )
                .unwrap_err()
                .contains("cycle")
        );
    }
}

/// Source-package inputs, with target-specific files overriding portable ones.
pub fn source_inputs(
    package: &Resolved,
    triplet: &str,
) -> Result<Vec<(String, PathBuf, bool)>, String> {
    fn visit(root: &Path, current: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
        for entry in fs::read_dir(current).map_err(|e| format!("{}: {e}", current.display()))? {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
                if entry.file_name() != "targets"
                    && !entry.file_name().to_string_lossy().starts_with('.')
                {
                    visit(root, &path, files)?;
                }
            } else if path
                .extension()
                .is_some_and(|e| e == "m2" || e == "h" || e == "toml")
            {
                files.push(path.strip_prefix(root).unwrap().to_owned());
            }
        }
        Ok(())
    }
    let mut result = std::collections::BTreeMap::new();
    for (root, targeted) in [
        (package.root.clone(), false),
        (package.root.join("targets").join(triplet), true),
    ] {
        if !root.is_dir() {
            continue;
        }
        let mut files = vec![];
        visit(&root, &root, &mut files)?;
        for relative in files {
            let file = root.join(&relative);
            let name = if package.entry.as_ref().is_some_and(|entry| entry == &file) {
                package.requirement.name.clone()
            } else {
                format!(
                    "{}::{}",
                    package.requirement.name,
                    relative
                        .with_extension("")
                        .iter()
                        .map(|p| p.to_string_lossy())
                        .collect::<Vec<_>>()
                        .join("::")
                )
            };
            result.insert(name, (file, targeted));
        }
    }
    Ok(result
        .into_iter()
        .map(|(module, (path, targeted))| (module, path, targeted))
        .collect())
}
pub fn prepare_declarations(
    package: &Resolved,
    triplet: &str,
) -> Result<usize, mlc_core::diagnostic::Report> {
    let mut count = 0;
    for (module, input, targeted) in source_inputs(package, triplet)? {
        let extension = input.extension().unwrap();
        if extension == "m2" {
            continue;
        }
        let output = package
            .root
            .join(".compiled/declarations")
            .join(triplet)
            .join(module.split("::").collect::<PathBuf>())
            .with_extension("sym");
        if extension == "h" {
            crate::artifacts::convert_c_header(
                &input,
                &output,
                &module,
                if targeted { triplet } else { "universal" },
            )?;
        } else {
            crate::artifacts::convert_toml(&input, &output)?;
        }
        count += 1;
    }
    Ok(count)
}
