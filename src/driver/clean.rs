//! Remove only a validated, project-local build output directory.
use mlc_builder::project::ProjectFile;
use std::{fs, io::ErrorKind, path::Path};

pub(super) fn run(root: &Path, output: Option<&Path>) -> Result<i32, String> {
    let configured = match output {
        Some(output) => output.to_path_buf(),
        None => ProjectFile::load(root)?
            .map(|file| file.project.output_dir)
            .unwrap_or_else(|| "build".into()),
    };
    let root = root.canonicalize().map_err(|error| error.to_string())?;
    let requested = root.join(configured);
    let metadata = match fs::symlink_metadata(&requested) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            println!("Nothing to clean: {}", requested.display());
            return Ok(0);
        }
        Err(error) => return Err(format!("{}: {error}", requested.display())),
    };
    let target = requested
        .canonicalize()
        .map_err(|error| format!("{}: {error}", requested.display()))?;
    if target == root || !target.starts_with(&root) {
        return Err(format!(
            "Refusing to clean outside the project or the project itself: {}",
            target.display()
        ));
    }
    if !metadata.is_dir() || metadata.is_symlink() {
        return Err(format!(
            "Refusing to clean a file or directory link: {}",
            requested.display()
        ));
    }
    if target.join(".git").exists()
        || target.join("Project.toml").exists()
        || target.join("project.toml").exists()
    {
        return Err(format!(
            "Refusing to clean a project directory: {}",
            target.display()
        ));
    }
    fs::remove_dir_all(&target)
        .map_err(|error| format!("Cannot clean {}: {error}", target.display()))?;
    println!("Removed {}", target.display());
    Ok(0)
}
