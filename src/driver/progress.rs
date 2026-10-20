//! Terminal-only progress; summaries remain readable when output is redirected.
use mlc_builder::schedule::NodeResult;
use std::{
    io::{self, IsTerminal, Write},
    sync::{Arc, Mutex, MutexGuard},
    time::Instant,
};

#[derive(Clone)]
pub(super) struct Progress {
    state: Arc<Mutex<State>>,
    started: Instant,
    operation: Option<&'static str>,
    terminal: bool,
}

#[derive(Default)]
struct State {
    stage: &'static str,
    done: usize,
    total: usize,
    hits: usize,
    checks: usize,
    declarations: usize,
    links: usize,
    link_hits: usize,
    changed: usize,
    unchanged: usize,
}

impl Progress {
    pub fn new(operation: Option<&'static str>) -> Self {
        Self {
            state: Arc::new(Mutex::new(State::default())),
            started: Instant::now(),
            operation,
            terminal: operation.is_some() && io::stderr().is_terminal(),
        }
    }

    fn state(&self) -> MutexGuard<'_, State> {
        self.state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub fn stage(&self, stage: &'static str, total: usize) {
        let mut state = self.state();
        state.stage = stage;
        state.done = 0;
        state.total = total;
        self.draw(&state);
    }

    pub fn advance(&self) {
        let mut state = self.state();
        state.done += 1;
        self.draw(&state);
    }

    pub fn node(&self, result: &NodeResult) {
        let mut state = self.state();
        match result {
            NodeResult::Cached(_) => {
                state.checks += 1;
                state.hits += 1;
            }
            NodeResult::Built(_) => state.checks += 1,
            NodeResult::Declaration(_) => state.declarations += 1,
            NodeResult::Failed { .. } | NodeResult::Blocked { .. } => (),
        }
        state.done += 1;
        self.draw(&state);
    }

    pub fn object(&self, hit: bool) {
        let mut state = self.state();
        state.checks += 1;
        state.hits += usize::from(hit);
    }

    pub fn link(&self, hit: bool) {
        let mut state = self.state();
        state.links += 1;
        state.link_hits += usize::from(hit);
    }

    pub fn formatted(&self, changed: bool) {
        let mut state = self.state();
        if changed {
            state.changed += 1;
        } else {
            state.unchanged += 1;
        }
    }

    fn draw(&self, state: &State) {
        if !self.terminal {
            return;
        }
        let filled = if state.total == 0 {
            0
        } else {
            (state.done.min(state.total) * 20) / state.total
        };
        let message = if state.total == 0 {
            format!("[--------------------] {}", state.stage)
        } else {
            format!(
                "[{}{}] {:>4}/{:<4} {}",
                "#".repeat(filled),
                "-".repeat(20 - filled),
                state.done,
                state.total,
                state.stage
            )
        };
        let _ = write!(io::stderr().lock(), "\r{message:<72}");
        let _ = io::stderr().flush();
    }

    pub fn clear(&self) {
        if self.terminal {
            let _guard = self.state();
            let _ = write!(io::stderr().lock(), "\r{:72}\r", "");
            let _ = io::stderr().flush();
        }
    }

    pub fn finish(&self, success: bool) {
        let Some(operation) = self.operation else {
            return;
        };
        self.clear();
        let state = self.state();
        let status = if success { "completed" } else { "failed" };
        let mut summary = format!(
            "{operation} {status} in {:.3}s.",
            self.started.elapsed().as_secs_f64()
        );
        if operation == "Build" {
            summary.push_str(&format!(
                "\nCache hits: {} ({}/{} object checks); links reused: {}/{}; declarations: {}.",
                rate(state.hits, state.checks),
                state.hits,
                state.checks,
                state.link_hits,
                state.links,
                state.declarations
            ));
        } else {
            summary.push_str(&format!(
                " {} changed, {} unchanged.",
                state.changed, state.unchanged
            ));
        }
        if success {
            println!("{summary}");
        } else {
            eprintln!("{summary}");
        }
    }
}

fn rate(hits: usize, checks: usize) -> String {
    if checks == 0 {
        "N/A".into()
    } else {
        format!("{:.1}%", hits as f64 * 100.0 / checks as f64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rates_have_an_explicit_empty_denominator() {
        assert_eq!(rate(0, 0), "N/A");
        assert_eq!(rate(0, 2), "0.0%");
        assert_eq!(rate(1, 2), "50.0%");
        assert_eq!(rate(2, 2), "100.0%");
    }
}
