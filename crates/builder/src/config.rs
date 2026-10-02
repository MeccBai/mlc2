//! User-wide configuration; absence means defaults, malformed files are errors.
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", deny_unknown_fields)]
pub struct GlobalConfig {
    #[serde(default)]
    pub lib_dirs: Option<Vec<PathBuf>>,
    #[serde(default)]
    pub lib_cache_dir: Option<PathBuf>,
}

impl GlobalConfig {
    pub fn load_default() -> Result<Self, String> {
        let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
            .ok_or("Cannot locate user home directory")?;
        Self::load(&PathBuf::from(home).join(".mlc/config.toml"))
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let text = match fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default());
            }
            Err(error) => return Err(format!("{}: {error}", path.display())),
        };
        let mut config: Self =
            toml::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))?;
        if let Some(cache) = &mut config.lib_cache_dir {
            if cache.is_relative() {
                *cache = path
                    .parent()
                    .ok_or("Configuration has no parent")?
                    .join(&*cache);
            }
        }
        if let Some(dirs) = &mut config.lib_dirs {
            for dir in dirs {
                if dir.is_relative() {
                    *dir = path
                        .parent()
                        .ok_or("Configuration has no parent")?
                        .join(&*dir);
                }
            }
        }
        Ok(config)
    }
}
