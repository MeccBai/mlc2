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

    pub fn dump(&self) -> String {
        format!("enum:{},variants:{:?}", self.name, self.variants)
    }

    pub fn size(&self) -> usize {
        4
    }

    pub fn align(&self) -> usize {
        4
    }
}
