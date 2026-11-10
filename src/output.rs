#![allow(dead_code)]
//! The CLI output boundary shared by the launcher and command workers.
use colored::Colorize;
use mlc_core::diagnostic::{Diagnostic, Report, Severity};
use std::sync::atomic::{AtomicBool, Ordering};
static JSON: AtomicBool = AtomicBool::new(false);
use serde_json::{Value, json};
use std::{path::Path, sync::Mutex};
static STATE: Mutex<State> = Mutex::new(State {
    artifacts: Vec::new(),
    messages: Vec::new(),
    diagnostics: Vec::new(),
    run: None,
});
struct State {
    artifacts: Vec<Value>,
    messages: Vec<String>,
    diagnostics: Vec<Diagnostic>,
    run: Option<Value>,
}
pub fn enabled() -> bool {
    JSON.load(Ordering::Relaxed)
}
pub fn initialize(json: bool) {
    if json {
        JSON.store(true, Ordering::Relaxed);
        colored::control::set_override(false);
    }
}
pub fn parse() -> crate::cli::Cli {
    use clap::Parser;
    initialize(std::env::args_os().any(|arg| arg == "--json"));
    match crate::cli::Cli::try_parse() {
        Ok(cli) => cli,
        Err(error) => {
            let code = error.exit_code();
            if enabled() {
                if code == 0 {
                    info(error.to_string());
                    finish("cli", Ok(0));
                } else {
                    finish("cli", Err(error.to_string().into()));
                }
                std::process::exit(code);
            }
            error.exit()
        }
    }
}
pub fn info(message: String) {
    if enabled() {
        STATE.lock().unwrap().messages.push(message);
    } else {
        if let Some((label, detail)) = message.split_once(' ') {
            if matches!(
                label,
                "Written" | "Prepared" | "Created" | "Formatted" | "Removed"
            ) {
                println!("{} {}", label.bright_green().bold(), detail.cyan());
                return;
            }
        }
        println!("{message}");
    }
}
pub fn diagnostic(diagnostic: Diagnostic) {
    let mut state = STATE.lock().unwrap();
    if !state.diagnostics.iter().any(|d| {
        d.severity == diagnostic.severity
            && d.message == diagnostic.message
            && d.source.as_ref().map(|s| (&s.file, &s.span))
                == diagnostic.source.as_ref().map(|s| (&s.file, &s.span))
    }) {
        state.diagnostics.push(diagnostic);
    }
}
pub fn warning(message: String) {
    if !message.is_empty() {
        diagnostic(Diagnostic::warning(message));
    }
}
pub fn artifact(kind: &str, path: &Path) {
    if enabled() {
        STATE
            .lock()
            .unwrap()
            .artifacts
            .push(json!({"kind":kind,"path":path}));
    }
}
pub fn program(output: &std::process::Output) {
    STATE.lock().unwrap().run = Some(
        json!({"stdout":String::from_utf8_lossy(&output.stdout),"stderr":String::from_utf8_lossy(&output.stderr),"exit_code":output.status.code()}),
    );
}
pub fn finish(command: &str, result: Result<i32, Report>) -> i32 {
    let code = result.as_ref().copied().unwrap_or(1);
    let mut state = STATE.lock().unwrap();
    if let Err(report) = result {
        state.diagnostics.extend(report.diagnostics);
    }
    if !enabled() {
        for diagnostic in &state.diagnostics {
            eprintln!("{}", diagnostic.render());
        }
        let errors = state
            .diagnostics
            .iter()
            .filter(|d| d.severity == Severity::Error && d.source.is_some())
            .count();
        if errors > 0 {
            eprintln!(
                "{errors} {} generated.",
                if errors == 1 { "error" } else { "errors" }
            );
        }
        return code;
    }
    let diagnostics = state
        .diagnostics
        .iter()
        .map(|d| {
            let mut value = json!({"severity":d.severity.name(),"message":d.message});
            if let Some(source) = &d.source {
                let (line, column) = d.position().unwrap();
                value["file"] = json!(source.file);
                value["span"] = json!({"start":source.span.start,"end":source.span.end});
                value["line"] = json!(line);
                value["column"] = json!(column);
            }
            value
        })
        .collect::<Vec<_>>();
    println!(
        "{}",
        json!({"version":1,"command":command,"success":code==0,"exit_code":code,"artifacts":state.artifacts,"diagnostics":diagnostics,"messages":state.messages,"run":state.run})
    );
    code
}

pub fn command(cli: &crate::cli::Cli) -> &'static str {
    use crate::cli::Commands::*;
    match cli.command {
        Clean => "clean",
        Setup => "setup",
        Init { .. } => "init",
        Fmt { .. } => "fmt",
        Ll { .. } => "ll",
        Symbols { .. } => "symbols",
        Example { .. } => "example",
        Build { .. } => "build",
        Run { .. } => "run",
    }
}
