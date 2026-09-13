
use std::path::{Path, PathBuf};
use crate::error::ErrorHandle;

pub struct PathResolver {
    self_path: PathBuf,
    lib_paths: Vec<PathBuf>,
}


impl PathResolver {
    pub fn new(self_path: PathBuf,err_h: &ErrorHandle) -> Self {
        let executable = self_path.file_name().unwrap();

        if (executable.to_str().unwrap() != "mlc.exe") {
            panic!()
        }
        let parent = self_path.parent().unwrap();
        let stdlib_path = parent.join("lib/std");
        let lib_paths = vec![stdlib_path];

        Self {
            self_path,
            lib_paths
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