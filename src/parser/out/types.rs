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

impl TempType {
    pub fn dump(&self) -> String {
        match self {
            Self::Path(path) => path.segments.join("::"),
            Self::Generic { base, args } => format!(
                "{}<{}>",
                base.segments.join("::"),
                args.iter()
                    .map(|(arg, _)| arg.dump())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Reference(inner) => format!("${}", inner.0.dump()),
        }
    }
}
