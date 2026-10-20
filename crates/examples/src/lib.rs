//! Bundled, self-contained language examples. No compiler or runtime dependency.
use std::fmt::Write;

pub struct Example {
    pub name: &'static str,
    pub description: &'static str,
    pub source: &'static str,
    pub category: &'static str,
    pub expected_stdout: &'static str,
}

macro_rules! example {
    ($name:literal, $description:literal, $file:literal, $category:literal, $stdout:literal) => {
        Example {
            name: $name,
            description: $description,
            source: include_str!(concat!("../sources/", $file, ".m2")),
            category: $category,
            expected_stdout: $stdout,
        }
    };
}

pub const EXAMPLES: &[Example] = &[
    example!(
        "hello-world",
        "Your first program: print text",
        "hello_world",
        "Basics",
        "Hello World!\n"
    ),
    example!(
        "variables",
        "var, val, const and explicit numeric casts",
        "variables",
        "Basics",
        "answer=42 counter=43 narrowed=44\nfraction=1.5\n"
    ),
    example!(
        "arrays",
        "Fixed arrays, indexing and byte-length string views",
        "arrays",
        "Basics",
        "Hello World!\nnumbers[1]=25\n"
    ),
    example!(
        "control-flow",
        "Range loops, if/else and value matching",
        "control_flow",
        "Basics",
        "sum=10\nmatched ten\n"
    ),
    example!(
        "while-loops",
        "while, break, continue and loop boundaries",
        "while_loops",
        "Basics",
        "sum=9\n"
    ),
    example!(
        "references",
        "Read-only references, mutable references and dereferencing",
        "references",
        "Basics",
        "value=42\n"
    ),
    example!(
        "functions",
        "Function parameters, return values and recursion",
        "functions",
        "Functions and generics",
        "fibonacci(10)=55\n"
    ),
    example!(
        "aliases",
        "Type, function and macro aliases with using",
        "aliases",
        "Functions and generics",
        "answer=42\n"
    ),
    example!(
        "pipelines",
        "Pipelines, argument unpacking and chained calls",
        "pipelines",
        "Functions and generics",
        "result=14\n"
    ),
    example!(
        "function-pointers",
        "Function pointers and callback parameters",
        "function_pointers",
        "Functions and generics",
        "answer=42\n"
    ),
    example!(
        "generics",
        "Generic functions: inferred and explicit type arguments",
        "generics",
        "Functions and generics",
        "a=20 b=10\n"
    ),
    example!(
        "associated-generics",
        "Base::swap: type inference for associated functions",
        "associated_generics",
        "Functions and generics",
        "a=100 b=10 fa=80 fb=20\n"
    ),
    example!(
        "constraints",
        "Combine integer, signedness and bit-width constraints with &&",
        "constraints",
        "Functions and generics",
        "answer=42\n"
    ),
    example!(
        "units",
        "Units, positional initialization and public fields",
        "units",
        "Types and resources",
        "point=(30,20)\n"
    ),
    example!(
        "generic-units",
        "Box<T> and structural type inference",
        "generic_units",
        "Types and resources",
        "value=42\n"
    ),
    example!(
        "methods",
        "self, mut self, private fields and reference calls",
        "methods",
        "Types and resources",
        "value=42\n"
    ),
    example!(
        "enums",
        "Enum values and match branches",
        "enums",
        "Types and resources",
        "running\n"
    ),
    example!(
        "unions",
        "Unions with payloads and variant matching",
        "unions",
        "Types and resources",
        "value=42\n"
    ),
    example!(
        "globals",
        "Global runtime initialization and function access",
        "globals",
        "Types and resources",
        "counter=42\n"
    ),
    example!(
        "memory",
        "Resource allocation, initialization and explicit release",
        "memory",
        "Types and resources",
        "allocated value=42\n"
    ),
    example!(
        "resources",
        "Resource borrowing, ownership transfer and automatic cleanup",
        "resources",
        "Types and resources",
        "observed=21 result=42\n"
    ),
    example!(
        "strings",
        "Owned String, byte-length views, append and clone",
        "strings",
        "Standard library",
        "Hello, world!\nbytes=13 empty=false\ncopy=Hello, world! original=Hello, world!?\n"
    ),
    example!(
        "formatting",
        "format!, print!, println! and escaped braces",
        "formatting",
        "Standard library",
        "answer=42 ready=true\n{value}=1.5\ndone\n\n"
    ),
    example!(
        "display",
        "Custom Display through the format interface",
        "display",
        "Standard library",
        "point=Point(3,4)\n"
    ),
    example!(
        "c-interop",
        "C ABI declarations and NUL-terminated C strings",
        "c_interop",
        "Interoperability",
        "C printf: 42\nbyte=65\n"
    ),
];

pub fn find(name: &str) -> Option<&'static Example> {
    EXAMPLES.iter().find(|example| example.name == name)
}

/// `list` lists names; every other argument prints the exact bundled source.
pub fn render(name: &str) -> Result<String, String> {
    if name == "list" {
        let mut output = format!(
            "MLC examples ({} total, grouped in learning order)\nView source: mlc example <name>\nSave and run: mlc example hello-world > hello.m2\n              mlc run hello.m2\nEach source includes explanations and expected output.\n",
            EXAMPLES.len()
        );
        let mut category = "";
        for example in EXAMPLES {
            if category != example.category {
                category = example.category;
                writeln!(output, "\n[{category}]").unwrap();
            }
            writeln!(output, "{:<22} {}", example.name, example.description)
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
