#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct EnumType {
    pub name: String,
    pub variants: Vec<String>,
}

impl EnumType {
    pub fn new(name: String, variants: Vec<String>) -> Self {
        Self { name, variants }
    }

    pub fn format(&self) -> String {
        self.name.clone()
    }

    pub fn dump(&self) {

    }
}