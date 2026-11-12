//! Project configuration is independent of command-line parsing.
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TargetKind {
    Bin,
    Static,
    Shared,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BuildMode {
    #[default]
    Default,
    Debug,
    Release,
    #[serde(rename = "rel-with-info", alias = "relwithdebinfo")]
    RelWithInfo,
}

impl BuildMode {
    pub const fn directory(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Debug => "debug",
            Self::Release => "release",
            Self::RelWithInfo => "rel-with-info",
        }
    }
    pub const fn optimized(self) -> bool {
        matches!(self, Self::Release | Self::RelWithInfo)
    }
    pub const fn debug_info(self) -> bool {
        matches!(self, Self::Debug | Self::RelWithInfo)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StdType {
    Debug,
    Release,
}
impl StdType {
    pub const fn for_mode(mode: BuildMode) -> Self {
        if mode.optimized() {
            Self::Release
        } else {
            Self::Debug
        }
    }
    pub const fn mode(self) -> BuildMode {
        match self {
            Self::Debug => BuildMode::Debug,
            Self::Release => BuildMode::Release,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", deny_unknown_fields)]
pub struct ProjectFile {
    #[serde(default, rename = "CTFE")]
    pub ctfe: CtfeConfig,
    pub project: Project,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", deny_unknown_fields)]
pub struct Project {
    #[serde(default)]
    pub mode: Option<BuildMode>,
    #[serde(default, rename = "std_type")]
    pub std_type: Option<StdType>,
    #[serde(default)]
    pub lib_dirs: Option<Vec<PathBuf>>,
    #[serde(default)]
    pub lib_cache_dir: Option<PathBuf>,
    #[serde(default)]
    pub triplet: Option<String>,
    pub name: String,
    pub version: String,
    pub output_dir: PathBuf,
    pub targets: Vec<Target>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", deny_unknown_fields)]
pub struct Target {
    #[serde(default)]
    pub mode: Option<BuildMode>,
    #[serde(default, rename = "std_type")]
    pub std_type: Option<StdType>,
    #[serde(default)]
    pub lib_dirs: Option<Vec<PathBuf>>,
    #[serde(default)]
    pub lib_cache_dir: Option<PathBuf>,
    #[serde(default)]
    pub triplet: Option<String>,
    pub name: String,
    pub entry: PathBuf,
    #[serde(rename = "Type")]
    pub kind: TargetKind,
    #[serde(default, rename = "Profile")]
    pub profile: Option<BuildMode>,
}

impl ProjectFile {
    pub fn load(root: &std::path::Path) -> Result<Option<Self>, mlc_core::diagnostic::Report> {
        for name in ["Project.toml", "project.toml"] {
            let path = root.join(name);
            match std::fs::read_to_string(&path) {
                Ok(text) => {
                    return toml::from_str(&text)
                        .map(Some)
                        .map_err(|e: toml::de::Error| {
                            mlc_core::diagnostic::Report::new(vec![
                                mlc_core::diagnostic::Diagnostic::error(e.message()).at(
                                    path.to_string_lossy(),
                                    std::sync::Arc::<str>::from(text.as_str()),
                                    e.span().unwrap_or(0..0),
                                ),
                            ])
                        });
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                Err(e) => return Err(format!("{}: {e}", path.display()).into()),
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_accepts_exactly_three_target_kinds() {
        for (kind, expected) in [
            ("bin", TargetKind::Bin),
            ("static", TargetKind::Static),
            ("shared", TargetKind::Shared),
        ] {
            let text = format!(
                "[Project]\nName='demo'\nVersion='1.0.0'\nOutputDir='build'\n\
                 [[Project.Targets]]\nName='main'\nEntry='main.m2'\nType='{kind}'"
            );
            let project: ProjectFile = toml::from_str(&text).unwrap();
            assert_eq!(project.project.targets[0].kind, expected);
            assert!(toml::from_str::<ProjectFile>(&text.replace(kind, "lib")).is_err());
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", deny_unknown_fields)]
pub struct CtfeConfig {
    #[serde(default = "default_ctfe_time", with = "ctfe_seconds")]
    pub max_time: std::time::Duration,
}
fn default_ctfe_time() -> std::time::Duration {
    std::time::Duration::from_secs(3)
}
impl Default for CtfeConfig {
    fn default() -> Self {
        Self {
            max_time: default_ctfe_time(),
        }
    }
}
mod ctfe_seconds {
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(
        value: &std::time::Duration,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.serialize_f64(value.as_secs_f64())
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<std::time::Duration, D::Error> {
        let seconds = f64::deserialize(deserializer)?;
        if !seconds.is_finite() || seconds <= 0.0 {
            return Err(serde::de::Error::custom(
                "CTFE.MaxTime must be a finite positive number of seconds",
            ));
        }
        let duration =
            std::time::Duration::try_from_secs_f64(seconds).map_err(serde::de::Error::custom)?;
        if duration.is_zero() {
            return Err(serde::de::Error::custom(
                "CTFE.MaxTime is below timer resolution",
            ));
        }
        Ok(duration)
    }
}
