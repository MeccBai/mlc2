use crate::diagnostic::error::ErrorHandle;
use crate::diagnostic::ice::ice;
use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::manifest::{EXE_SUFFIX, EXECUTABLE_NAME, LIB_DIR};

pub struct PathResolver {
    self_path: PathBuf,
    lib_paths: Vec<PathBuf>,
}

impl PathResolver {
    pub fn new(self_path: PathBuf) -> Self {
        let executable = self_path.file_name().unwrap();

        let mlc_name = format!("{EXECUTABLE_NAME}{EXE_SUFFIX}");

        if executable != OsStr::new(&mlc_name) {
            ice("Executable is not mlc");
        }

        let parent = self_path
            .parent()
            .unwrap_or_else(|| ice("Executable path has no parent"));

        let lib_paths = vec![parent.join(LIB_DIR)];

        Self {
            self_path,
            lib_paths,
        }
    }

    pub fn search_lib(&self, lib_name: &str) -> Option<PathBuf> {
        for lib_path in &self.lib_paths {
            let candidate = lib_path.join(lib_name);
            if candidate.exists() {
                return Some(candidate);
            }
        }
        None
    }
}
