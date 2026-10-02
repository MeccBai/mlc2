//! Task-local generation diagnostics. Workers collect independently and the entry
//! unit merges handles before checking the final result or printing diagnostics.
use std::{
    any::Any,
    fmt,
    panic::{AssertUnwindSafe, catch_unwind, resume_unwind},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationErrorKind {
    Internal,
    Unsupported,
    UnexpectedPanic,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationError {
    pub task: String,
    pub kind: GenerationErrorKind,
    pub reason: String,
}

/// Private payload: expected generation failures do not invoke a panic hook.
#[derive(Debug)]
struct GenerationFailure {
    kind: GenerationErrorKind,
    reason: String,
}

pub(super) fn fail(reason: &str) -> ! {
    resume_unwind(Box::new(GenerationFailure {
        kind: GenerationErrorKind::Internal,
        reason: reason.into(),
    }))
}

pub(super) fn unsupported(reason: &str) -> ! {
    resume_unwind(Box::new(GenerationFailure {
        kind: GenerationErrorKind::Unsupported,
        reason: reason.into(),
    }))
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct GenerationErrorHandle {
    errors: Vec<GenerationError>,
}

impl GenerationErrorHandle {
    pub fn errors(&self) -> &[GenerationError] {
        &self.errors
    }

    /// A failed task produces no value. Its caller must discard any external
    /// partial writes. IrGenerator::collect handles its own buffer rollback.
    /// Real Rust panics are also captured, but their existing hook still runs.
    /// Requires panic=unwind; native aborts and process exits cannot be recovered.
    pub fn collect<T>(&mut self, task: impl Into<String>, work: impl FnOnce() -> T) -> Option<T> {
        match catch_unwind(AssertUnwindSafe(work)) {
            Ok(value) => Some(value),
            Err(payload) => {
                let task = task.into();
                match payload.downcast::<GenerationFailure>() {
                    Ok(failure) => self.errors.push(GenerationError {
                        task,
                        kind: failure.kind,
                        reason: failure.reason,
                    }),
                    Err(payload) => self.errors.push(GenerationError {
                        task,
                        kind: GenerationErrorKind::UnexpectedPanic,
                        reason: panic_reason(&*payload),
                    }),
                }
                None
            }
        }
    }

    /// Merge in scheduler-defined order for deterministic diagnostic ordering.
    pub fn append(&mut self, mut other: Self) {
        self.errors.append(&mut other.errors);
    }

    /// Only the entry boundary turns collected failures into a Result.
    pub fn finish<T>(self, output: T) -> Result<T, GenerationErrors> {
        if self.errors.is_empty() {
            Ok(output)
        } else {
            Err(GenerationErrors {
                errors: self.errors,
            })
        }
    }
}

fn panic_reason(payload: &(dyn Any + Send)) -> String {
    if let Some(reason) = payload.downcast_ref::<String>() {
        reason.clone()
    } else if let Some(reason) = payload.downcast_ref::<&str>() {
        (*reason).into()
    } else {
        "non-string panic payload during generation".into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationErrors {
    pub errors: Vec<GenerationError>,
}

impl GenerationErrors {
    pub fn print(&self) {
        eprint!("{self}");
    }
}

impl fmt::Display for GenerationErrors {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for error in &self.errors {
            writeln!(
                f,
                "Generation {:?} [{}]: {}",
                error.kind, error.task, error.reason
            )?;
        }
        Ok(())
    }
}
impl std::error::Error for GenerationErrors {}

#[cfg(test)]
mod tests;
