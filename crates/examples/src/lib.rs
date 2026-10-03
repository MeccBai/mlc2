//! Bundled, self-contained language examples. No compiler or runtime dependency.
use std::fmt::Write;

pub struct Example {
    pub name: &'static str,
    pub description: &'static str,
    pub source: &'static str,
}

macro_rules! example {
    ($name:literal, $description:literal, $file:literal) => {
        Example {
            name: $name,
            description: $description,
            source: include_str!(concat!("../sources/", $file, ".m2")),
        }
    };
}

pub const EXAMPLES: &[Example] = &[
    example!("hello-world", "Print a static C string", "hello_world"),
    example!(
        "variables",
        "var, val, const and explicit casts",
        "variables"
    ),
    example!(
        "arrays",
        "Array initialization, indexing and C strings",
        "arrays"
    ),
    example!(
        "control-flow",
        "Numeric for, if/else and match",
        "control_flow"
    ),
    example!("functions", "Functions and recursive calls", "functions"),
    example!("generics", "Monomorphized generic swap", "generics"),
    example!("units", "Unit initialization and public members", "units"),
    example!(
        "references",
        "Immutable and mutable references",
        "references"
    ),
    example!(
        "memory",
        "Explicit std::mem allocation and deallocation",
        "memory"
    ),
];

pub fn find(name: &str) -> Option<&'static Example> {
    EXAMPLES.iter().find(|example| example.name == name)
}

/// `list` lists names; every other argument prints the exact bundled source.
pub fn render(name: &str) -> Result<String, String> {
    if name == "list" {
        let mut output = String::new();
        for example in EXAMPLES {
            writeln!(output, "{:<14} {}", example.name, example.description)
                .expect("writing to String cannot fail");
        }
        return Ok(output);
    }
    find(name)
        .map(|example| example.source.to_owned())
        .ok_or_else(|| format!("Unknown example `{name}`; run 'mlc example list'"))
}

#[cfg(test)]
mod tests;
