use super::BackendError;
use crate::manifest::{LIB_DIR, TOOLS_DIR};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct BackendConfig {
    pub executable: PathBuf,
    pub lib: PathBuf,
    pub tools: PathBuf,
}

impl BackendConfig {
    pub fn current() -> Result<Self, BackendError> {
        let executable =
            std::env::current_exe().map_err(|e| BackendError::Config(e.to_string()))?;
        Self::from_executable(executable)
    }

    /// Distribution layout: executable and sibling lib/ and tools/ directories.
    pub fn from_executable(executable: PathBuf) -> Result<Self, BackendError> {
        let root = executable
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .ok_or_else(|| {
                BackendError::Config("executable must have a parent directory".into())
            })?;
        Ok(Self {
            lib: root.join(LIB_DIR),
            tools: root.join(TOOLS_DIR),
            executable,
        })
    }
}
