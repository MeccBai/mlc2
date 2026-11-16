use super::BackendError;

use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct BackendConfig {
    pub executable: PathBuf,
    pub runtimes: PathBuf,
    pub tools: PathBuf,
}

impl BackendConfig {
    pub fn current() -> Result<Self, BackendError> {
        let executable =
            std::env::current_exe().map_err(|e| BackendError::Config(e.to_string()))?;
        Self::from_executable(executable)
    }

    /// Distribution layout: packages in packages/, native runtimes in runtimes/, tools in tools/.
    pub fn from_executable(executable: PathBuf) -> Result<Self, BackendError> {
        let root = executable
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .ok_or_else(|| {
                BackendError::Config("executable must have a parent directory".into())
            })?;
        Ok(Self {
            runtimes: root.join(mlc_manifest::RUNTIMES_DIR),
            tools: root.join(mlc_manifest::TOOLS_DIR),
            executable,
        })
    }
}
