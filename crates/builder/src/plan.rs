//! File identity, import discovery and cycle-free build dependencies.
mod paths;
mod scan;
use crate::artifacts::Manifest;
use mlc_syntax::parser::out::TempModule;
pub use paths::ImportResolver;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TargetIndex(pub usize);

#[derive(Debug, Clone)]
pub enum TargetInput {
    Source { text: String, module: TempModule },
    Declaration(Manifest),
}

#[derive(Debug, Clone)]
pub struct Target {
    pub artifact_stems: Vec<PathBuf>,
    pub id: TargetIndex,
    pub file: PathBuf,
    pub module_name: String,
    pub input: TargetInput,
    pub requires: Vec<TargetIndex>,
    pub supers: Vec<TargetIndex>,
}

#[derive(Debug, Clone)]
pub struct BuildPlan {
    pub triplet: Option<String>,
    pub entry: TargetIndex,
    pub targets: Vec<Target>,
    /// Dependencies precede consumers; ready nodes need no worker-side waiting.
    pub order: Vec<TargetIndex>,
}

impl BuildPlan {
    pub fn discover(entry: &Path, resolver: &ImportResolver) -> Result<Self, String> {
        scan::discover(entry, resolver, None)
    }

    pub fn discover_with_paths(
        entry: &Path,
        paths: &crate::paths::PathResolver,
    ) -> Result<Self, String> {
        scan::discover(entry, &paths.imports, Some(paths))
    }
}
