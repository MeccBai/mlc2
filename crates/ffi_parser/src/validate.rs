use crate::Error;
use mlc_syntax::parser::out::*;
use std::collections::{HashMap, HashSet};

pub(super) fn module(module: &TempModule) -> Result<(), Error> {
    let units: HashMap<_, _> = module
        .iter()
        .filter_map(|(item, _)| match item {
            TempGlobalStmt::Unit(unit) => Some((unit.name.as_str(), unit)),
            _ => None,
        })
        .collect();
    if let Some(unit) = units.values().find(|unit| builtin(&unit.name)) {
        return Err(Error {
            span: unit.name_span.into_range(),
            message: "C struct name conflicts with a language primitive type".into(),
        });
    }
    for (item, _) in module {
        match item {
            TempGlobalStmt::Unit(unit) => {
                accessible(&unit.name, unit.name_span)?;
                for member in &unit.members {
                    accessible(&member.name, member.name_span)?;
                    check_type(&member.ty, &units, false)?;
                }
            }
            TempGlobalStmt::Func(function) => {
                accessible(&function.symbol.name, function.symbol.name_span)?;
                for param in &function.symbol.params {
                    if let Some(ty) = &param.ty {
                        check_type(ty, &units, true)?;
                    }
                }
                if let Some(ty) = &function.symbol.return_type {
                    check_type(ty, &units, true)?;
                }
            }
            _ => (),
        }
    }
    let mut done = HashSet::new();
    for name in units.keys() {
        visit(name, &units, &mut HashSet::new(), &mut done)?;
    }
    let mut layouts = HashMap::new();
    for unit in units.values() {
        if layout(
            &TempType::Path(TempPath {
                segments: vec![unit.name.clone()],
            }),
            &units,
            &mut layouts,
        )
        .is_none()
        {
            return Err(Error {
                span: unit.name_span.into_range(),
                message: "C struct layout exceeds the supported size".into(),
            });
        }
    }
    Ok(())
}

fn accessible(name: &str, span: Span) -> Result<(), Error> {
    let valid = mlc_syntax::lexer::tokenize(name).is_ok_and(|lexed| {
        matches!(
            lexed.tokens.as_slice(),
            [(mlc_syntax::lexer::TokenPack::Ident(_), _)]
        )
    });
    if !valid {
        return Err(Error {
            span: span.into_range(),
            message: format!(
                "C name '{name}' conflicts with a language keyword; aliases are not supported yet"
            ),
        });
    }
    Ok(())
}

fn layout(
    ty: &TempType,
    units: &HashMap<&str, &TempUnit>,
    layouts: &mut HashMap<String, (usize, usize)>,
) -> Option<(usize, usize)> {
    if let TempType::Path(path) = ty {
        if let Some(layout) = layouts.get(&path.segments[0]) {
            return Some(*layout);
        }
    }
    let result = match ty {
        TempType::Reference { .. } => (8, 8),
        TempType::Array { element, length } => {
            let (size, align) = layout(&element.0, units, layouts)?;
            (size.checked_mul(*length)?, align)
        }
        TempType::Path(path) => match path.segments[0].as_str() {
            "bool" | "i8" | "u8" => (1, 1),
            "i16" | "u16" => (2, 2),
            "i32" | "u32" | "f32" => (4, 4),
            "i64" | "u64" | "f64" => (8, 8),
            name => {
                let mut offset = 0usize;
                let mut alignment = 1;
                for member in &units[name].members {
                    let (size, align) = layout(&member.ty.0, units, layouts)?;
                    offset = offset.checked_add(align - 1)? & !(align - 1);
                    offset = offset.checked_add(size)?;
                    alignment = alignment.max(align);
                }
                (
                    (offset.checked_add(alignment - 1)? & !(alignment - 1)),
                    alignment,
                )
            }
        },
        _ => return None,
    };
    if result.0 > isize::MAX as usize {
        return None;
    }
    if let TempType::Path(path) = ty {
        layouts.insert(path.segments[0].clone(), result);
    }
    Some(result)
}

fn builtin(name: &str) -> bool {
    matches!(
        name,
        "bool" | "i8" | "u8" | "i16" | "u16" | "i32" | "u32" | "i64" | "u64" | "f32" | "f64"
    )
}

fn check_type(
    (ty, span): &Spanned<TempType>,
    units: &HashMap<&str, &TempUnit>,
    argument: bool,
) -> Result<(), Error> {
    match ty {
        TempType::Path(path) => {
            let name = path.segments[0].as_str();
            if !builtin(name) && (!units.contains_key(name) || argument) {
                return Err(Error {
                    span: span.into_range(),
                    message: if units.contains_key(name) {
                        "C structs cannot be passed or returned by value yet; use a pointer".into()
                    } else {
                        format!("Incomplete or unknown C struct: {name}")
                    },
                });
            }
        }
        TempType::Reference { inner, .. } => check_type(inner, units, false)?,
        TempType::Array { element, .. } => check_type(element, units, argument)?,
        _ => unreachable!("ffi_parser constructs only primitive, struct, pointer and array types"),
    }
    Ok(())
}

fn visit<'a>(
    name: &'a str,
    units: &HashMap<&'a str, &'a TempUnit>,
    active: &mut HashSet<&'a str>,
    done: &mut HashSet<&'a str>,
) -> Result<(), Error> {
    if done.contains(name) {
        return Ok(());
    }
    let unit = units[name];
    if !active.insert(name) {
        return Err(Error {
            span: unit.name_span.into_range(),
            message: "Recursive by-value C struct layout".into(),
        });
    }
    for member in &unit.members {
        let mut ty = &member.ty.0;
        while let TempType::Array { element, .. } = ty {
            ty = &element.0;
        }
        if let TempType::Path(path) = ty {
            let name = path.segments[0].as_str();
            if units.contains_key(name) {
                visit(name, units, active, done)?;
            }
        }
    }
    active.remove(name);
    done.insert(name);
    Ok(())
}
