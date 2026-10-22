use mlc_syntax::parser::out::{Span, Spanned, TempPath, TempType};

#[derive(Clone, Copy)]
pub(super) struct DataModel;
impl DataModel {
    pub fn new(target: &str) -> Option<Self> {
        // The semantic type store currently uses 64-bit references.
        if !matches!(target.split('-').next()?, "x86_64" | "aarch64") {
            return None;
        }
        if !(target.contains("windows")
            || target.contains("linux")
            || target.contains("apple-darwin"))
        {
            return None;
        }
        Some(Self)
    }
}

pub(super) struct CType {
    pub ty: Option<Spanned<TempType>>,
    pub constant: bool,
}

pub(super) fn path(name: &str, span: Span) -> Spanned<TempType> {
    (
        TempType::Path(TempPath {
            segments: vec![name.into()],
        }),
        span,
    )
}

pub(super) fn primitive(name: &str) -> Option<Option<&'static str>> {
    Some(Some(match name {
        "void" => return Some(None),
        "_Bool" | "bool" => "bool",
        "int8_t" => "i8",
        "uint8_t" => "u8",
        "int16_t" => "i16",
        "uint16_t" => "u16",
        "int32_t" => "i32",
        "uint32_t" => "u32",
        "int64_t" => "i64",
        "uint64_t" => "u64",
        "float" => "f32",
        "double" => "f64",
        _ => return None,
    }))
}
