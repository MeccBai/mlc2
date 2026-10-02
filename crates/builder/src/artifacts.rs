//! Portable declarations and template artifacts; never persist arena indices.
mod hashes;
mod storage;
mod symbols;
#[cfg(test)]
mod tests;
pub use hashes::{content_hash, semantic_hash};
use mlc_syntax::{ImportModule, parser::out::TempModule};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub use storage::{Artifact, ArtifactPaths, load, publish};
pub use symbols::Symbols;
pub(crate) use symbols::has_templates;

pub const FORMAT_VERSION: u32 = 2;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObjectMode {
    Source,
    #[default]
    None,
    Only,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct DependencyHash {
    pub symbols_hash: String,
    pub generic_hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct Metadata {
    #[serde(default = "format_version")]
    pub format_version: u32,
    pub source_file: String,
    pub module_name: String,
    pub file_hash: String,
    pub symbols_hash: String,
    pub generic_hash: String,
    pub build_key: String,
    pub target: String,
    pub object: ObjectMode,
    pub object_file: Option<String>,
    pub object_hash: Option<String>,
    pub generic: Option<String>,
    pub generic_file_hash: Option<String>,
    pub dependencies: BTreeMap<String, DependencyHash>,
}

fn format_version() -> u32 {
    FORMAT_VERSION
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Manifest {
    pub config: Metadata,
    pub symbols: Symbols,
    #[serde(default)]
    pub requires: Vec<ImportModule>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct GenericBundle {
    pub format_version: u32,
    pub source_file: String,
    pub module_name: String,
    /// Conservative closure: retain private helpers, using and globals, too.
    pub module: TempModule,
}

impl Manifest {
    /// Temp signatures are not resolved layouts yet. Include dependency fingerprints
    /// conservatively so a changed imported type cannot leave transitive users stale.
    pub fn computed_symbols_hash(&self) -> Result<String, String> {
        semantic_hash(&(self.symbols.public_hash()?, &self.config.dependencies))
    }

    pub fn dependency_hash(&self) -> Result<DependencyHash, String> {
        Ok(DependencyHash {
            symbols_hash: self.computed_symbols_hash()?,
            generic_hash: if self.config.object == ObjectMode::None {
                String::new()
            } else {
                self.config.generic_hash.clone()
            },
        })
    }
}
