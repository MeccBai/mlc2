//! Definitions shared by temporary syntax and semantic ASTs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ValueType {
    Flex,
    Final,
    Constant,
}
impl ValueType {
    pub fn prefix(self) -> &'static str {
        match self {
            Self::Flex => "",
            Self::Final => "val ",
            Self::Constant => "const ",
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ImportModule {
    path: Vec<String>,
    export: bool,
}
impl ImportModule {
    pub fn new(path: Vec<String>, export: bool) -> Self {
        Self { path, export }
    }
    pub fn path(&self) -> &[String] {
        &self.path
    }
    pub fn exported(&self) -> bool {
        self.export
    }
}
