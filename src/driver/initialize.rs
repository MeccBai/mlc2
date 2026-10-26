//! Create a new project without modifying an existing directory.
use mlc_builder::project::{Project, ProjectFile, Target, TargetKind};
use std::{fs, path::Path};

pub(super) fn run(cwd: &Path, project: &Path) -> Result<i32, String> {
    let destination = cwd.join(project);
    let name = project
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty() && !name.contains(['/', '\\', ':']))
        .ok_or("Project must name a new directory")?;
    let config = ProjectFile {
        project: Project {
            mode: None,
            std_type: None,
            lib_dirs: None,
            lib_cache_dir: None,
            triplet: None,
            name: name.into(),
            version: "0.1.0".into(),
            output_dir: "build".into(),
            targets: vec![Target {
                mode: None,
                std_type: None,
                lib_dirs: None,
                lib_cache_dir: None,
                triplet: None,
                name: name.into(),
                entry: "src/main.m2".into(),
                kind: TargetKind::Bin,
            }],
        },
    };
    let text = toml::to_string_pretty(&config).map_err(|error| error.to_string())?;
    fs::create_dir(&destination).map_err(|error| {
        format!(
            "Cannot create project directory: {error}\nExpected new directory: {}",
            destination.display()
        )
    })?;
    let source = destination.join("src");
    fs::create_dir(&source).map_err(|error| format!("{}: {error}", source.display()))?;
    fs::write(destination.join("Project.toml"), text)
        .map_err(|error| format!("{}: {error}", destination.join("Project.toml").display()))?;
    fs::write(source.join("main.m2"), "import std;\n\nfunc main()->i32 {\n    std::println!(\"Hello, world!\");\n    return 0;\n}\n")
        .map_err(|error| format!("{}: {error}", source.join("main.m2").display()))?;
    println!("Created project {} at {}", name, destination.display());
    Ok(0)
}
