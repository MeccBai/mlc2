use super::Spanned;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TempPath {
    pub segments: Vec<String>,
}

impl TempPath {
    pub fn join(self) -> String {
        self.segments.join("::")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum TempType {
    Path(TempPath),
    Generic {
        base: TempPath,
        args: Vec<Spanned<TempType>>,
    },
    Reference(Box<Spanned<TempType>>),
}
