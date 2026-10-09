//! Persist each complete LLVM module independently; IR modules cannot be concatenated.
use super::frontend::Generated;
use mlc_builder::plan::BuildPlan;
use std::{
    collections::HashSet,
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
};

pub(super) fn write(
    plan: &BuildPlan,
    generated: Generated,
    directory: &Path,
) -> Result<(), String> {
    let mut files = Vec::new();
    let mut names = HashSet::new();
    for (id, ir) in generated.modules {
        let name = &plan.targets[id].module_name;
        let mut relative = PathBuf::new();
        for segment in name.split("::") {
            if segment.is_empty()
                || segment.contains(['/', '\\', ':', '.'])
                || !matches!(
                    Path::new(segment).components().next(),
                    Some(Component::Normal(_))
                )
            {
                return Err(format!("Invalid IR module name: {name}"));
            }
            relative.push(segment);
        }
        relative.set_extension("ll");
        if !names.insert(relative.clone()) {
            return Err(format!("Duplicate IR output: {}", relative.display()));
        }
        files.push((directory.join(relative), ir));
    }
    if let Some(ir) = generated.supplement {
        let relative = PathBuf::from("__mlc_instances.ll");
        if !names.insert(relative.clone()) {
            return Err("IR module name conflicts with __mlc_instances".into());
        }
        files.push((directory.join(relative), ir));
    }
    for (path, ir) in files {
        let parent = path.parent().ok_or("IR output has no parent")?;
        fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
        let mut file =
            tempfile::NamedTempFile::new_in(parent).map_err(|error| error.to_string())?;
        file.write_all(ir.as_bytes())
            .and_then(|_| file.as_file().sync_all())
            .map_err(|error| error.to_string())?;
        file.persist(&path).map_err(|error| error.to_string())?;
        println!("Written {}", path.display());
    }
    Ok(())
}
