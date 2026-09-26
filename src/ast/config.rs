use crate::error::ErrorHandle;

pub struct Config {
    system_path: Vec<String>,
    project_path: String,
    current_file: String,
    err_h: ErrorHandle,
}

impl Config {
    pub fn new(
        system_path: Vec<String>,
        project_path: String,
        current_file: String,
        err_h: ErrorHandle,
    ) -> Self {
        Self {
            system_path,
            project_path,
            current_file,
            err_h,
        }
    }

    pub fn symbol_prefix(&self) -> String {
        let mut prefix = self.system_path.join("::");
        if !prefix.is_empty() {
            prefix.push_str("::");
        }
        prefix.push_str(&self.project_path);
        prefix
    }

    pub fn symbol_name(&self, name: &str) -> String {
        format!("{}::{}", self.symbol_prefix(), name)
    }

    pub fn current_file(&self) -> &String {
        &self.current_file
    }
}
