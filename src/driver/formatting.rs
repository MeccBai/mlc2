//! Formatting owns file discovery and publication, never build/import resolution.
use mlc_builder::project::ProjectFile;
use mlc_formatter::{FormatError, format};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub(super) fn run(
    cwd: &Path,
    input: Option<&Path>,
    progress: &super::Progress,
) -> Result<i32, String> {
    progress.stage("Discovering sources", 0);
    let mut files = Vec::new();
    if let Some(input) = input {
        let file = cwd.join(input);
        let metadata = fs::symlink_metadata(&file).map_err(|e| error(&file, e))?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return Err(error(
                &file,
                "Expected a regular source file, not a directory or symlink",
            ));
        }
        files.push(file);
    } else {
        let root = cwd
            .ancestors()
            .find(|root| root.join("Project.toml").is_file() || root.join("project.toml").is_file())
            .unwrap_or(cwd);
        let project = ProjectFile::load(root)?;
        let mut excluded = vec![];
        if let Some(project) = project {
            excluded.push(root.join(project.project.output_dir));
            if let Some(cache) = project.project.lib_cache_dir {
                excluded.push(root.join(cache));
            }
            for target in project.project.targets {
                if let Some(cache) = target.lib_cache_dir {
                    excluded.push(root.join(cache));
                }
            }
        }
        // Canonical paths normalize '.', '..' and Windows path prefixes.
        let excluded: Vec<_> = excluded
            .into_iter()
            .filter_map(|p| p.canonicalize().ok())
            .collect();
        discover(root, &excluded, &mut files)?;
    }
    files.sort();
    progress.stage("Formatting sources", files.len());

    // Prepare every output first: one syntax error must not partially format a project.
    let mut changed = Vec::new();
    let mut diagnostics = Vec::new();
    for file in files {
        let source = fs::read_to_string(&file).map_err(|e| error(&file, e))?;
        match format(&source) {
            Ok(output) if output != source => changed.push((file, source, output)),
            Ok(_) => progress.formatted(false),
            Err(issue) => diagnostics.push(render_error(&file, &source, issue)),
        }
        progress.advance();
    }
    if !diagnostics.is_empty() {
        return Err(diagnostics.join("\n"));
    }

    // Stage in the same directory, retaining permissions; never truncate originals.
    let mut staged = Vec::new();
    progress.stage("Staging formatted files", changed.len());
    for (file, source, output) in &changed {
        let mut temp =
            tempfile::NamedTempFile::new_in(file.parent().ok_or("Source has no parent")?)
                .map_err(|e| error(file, e))?;
        temp.write_all(output.as_bytes())
            .map_err(|e| error(file, e))?;
        let permissions = fs::metadata(file)
            .map_err(|e| error(file, e))?
            .permissions();
        temp.as_file()
            .set_permissions(permissions)
            .map_err(|e| error(file, e))?;
        if fs::read_to_string(file).map_err(|e| error(file, e))? != *source {
            return Err(error(
                file,
                "File changed during formatting; not overwritten",
            ));
        }
        staged.push((file, temp));
        progress.advance();
    }
    progress.stage("Writing formatted files", staged.len());
    for (file, temp) in staged {
        temp.persist(file).map_err(|e| error(file, e))?;
        progress.formatted(true);
        progress.advance();
        progress.clear();
        println!("Formatted {}", file.display());
    }
    progress.clear();
    println!("{} file(s) formatted.", changed.len());
    Ok(0)
}

fn discover(root: &Path, excluded: &[PathBuf], files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(root).map_err(|e| error(root, e))? {
        let entry = entry.map_err(|e| error(root, e))?;
        let path = entry.path();
        let kind = entry.file_type().map_err(|e| error(&path, e))?;
        // Do not escape the project, follow symlink cycles, or format dependencies via links.
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with('.')
                || matches!(
                    name.as_ref(),
                    "build" | "target" | "objects" | "tools" | "runtimes"
                )
            {
                continue;
            }
            let canonical = path.canonicalize().map_err(|e| error(&path, e))?;
            if excluded
                .iter()
                .any(|excluded| canonical.starts_with(excluded))
            {
                continue;
            }
            discover(&path, excluded, files)?;
        } else if kind.is_file()
            && path.extension().is_some_and(|extension| {
                extension == mlc_syntax::manifest::SOURCE_SUFFIX.trim_start_matches('.')
            })
        {
            files.push(path);
        }
    }
    Ok(())
}

fn error(path: &Path, message: impl std::fmt::Display) -> String {
    format!("{}: {message}", path.display())
}

fn render_error(file: &Path, source: &str, issue: FormatError) -> String {
    use mlc_core::diagnostic::error::ErrorHandle;
    let handle = ErrorHandle::new(file.to_string_lossy().into_owned());
    let issues = match issue {
        FormatError::Lexical(issue) => vec![issue],
        FormatError::Syntax(issues) if !issues.is_empty() => issues,
        other => return error(file, other),
    };
    let rendered = issues
        .iter()
        .map(|issue| handle.render_error(source, issue.span.clone(), &issue.message))
        .collect::<Vec<_>>()
        .join("\n");
    ErrorHandle::summarize(&rendered, issues.len())
}
