use mlc_builder::artifacts::content_hash;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Fingerprint {
    pub version: u32,
    pub compiler_build: u32,
    pub compiler_hash: String,
    pub temp_ast_hash: String,
    pub call_hash: String,
    pub layout_hash: String,
    pub host: String,
    pub target: String,
    pub ptr_size: usize,
    pub ptr_align: usize,
    pub optimization: String,
    pub library_file: String,
}
#[derive(Serialize, Deserialize)]
struct Record {
    #[serde(flatten)]
    fingerprint: Fingerprint,
    library_hash: String,
    result_hash: String,
}
pub(super) struct Entry {
    directory: PathBuf,
    fingerprint: Fingerprint,
}
impl Entry {
    pub fn new(root: &Path, fingerprint: Fingerprint) -> Self {
        Self {
            directory: root.join(&fingerprint.host).join(&fingerprint.call_hash),
            fingerprint,
        }
    }
    pub fn load(&self) -> Option<Value> {
        let record: Record =
            toml::from_str(&fs::read_to_string(self.directory.join("cache.toml")).ok()?).ok()?;
        if record.fingerprint != self.fingerprint {
            return None;
        }
        let library = fs::read(self.directory.join(&self.fingerprint.library_file)).ok()?;
        let result = fs::read(self.directory.join("result.json")).ok()?;
        if content_hash(&library) != record.library_hash
            || content_hash(&result) != record.result_hash
        {
            return None;
        }
        serde_json::from_slice(&result).ok()
    }
    pub fn staging(&self) -> Result<tempfile::TempDir, String> {
        fs::create_dir_all(&self.directory).map_err(|e| e.to_string())?;
        tempfile::Builder::new()
            .prefix(".building-")
            .tempdir_in(&self.directory)
            .map_err(|e| e.to_string())
    }
    pub fn publish(&self, staging: &Path, object: &str) -> Result<(), String> {
        let library =
            fs::read(staging.join(&self.fingerprint.library_file)).map_err(|e| e.to_string())?;
        let result = fs::read(staging.join("result.json")).map_err(|e| e.to_string())?;
        let record = Record {
            fingerprint: self.fingerprint.clone(),
            library_hash: content_hash(&library),
            result_hash: content_hash(&result),
        };
        atomic(
            &self.directory.join(&self.fingerprint.library_file),
            &library,
        )?;
        atomic(&self.directory.join("result.json"), &result)?;
        for file in [object, "layout.json"] {
            atomic(
                &self.directory.join(file),
                &fs::read(staging.join(file)).map_err(|e| e.to_string())?,
            )?;
        }
        // Publish the record after all files it describes have reached their destinations.
        atomic(
            &self.directory.join("cache.toml"),
            toml::to_string_pretty(&record)
                .map_err(|e| e.to_string())?
                .as_bytes(),
        )
    }
}
fn atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file =
        tempfile::NamedTempFile::new_in(path.parent().ok_or("Cache file has no parent")?)
            .map_err(|e| e.to_string())?;
    file.write_all(bytes).map_err(|e| e.to_string())?;
    file.as_file().sync_all().map_err(|e| e.to_string())?;
    file.persist(path).map_err(|e| e.to_string())?;
    Ok(())
}
