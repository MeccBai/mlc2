use super::*;
use mlc_syntax::{ImportModule, parser::out::TempGlobalStmt};
use std::{collections::HashMap, fs};

#[derive(Clone, Copy, PartialEq)]
enum State {
    Resolving,
    Resolved,
}
struct Scanner<'a> {
    paths: Option<&'a crate::paths::PathResolver>,
    resolver: &'a ImportResolver,
    project: PathBuf,
    targets: Vec<Target>,
    known: HashMap<PathBuf, (TargetIndex, State)>,
    stack: Vec<PathBuf>,
    order: Vec<TargetIndex>,
}

pub(super) fn discover(
    entry: &Path,
    resolver: &ImportResolver,
    paths: Option<&crate::paths::PathResolver>,
) -> Result<BuildPlan, String> {
    let entry = entry
        .canonicalize()
        .map_err(|e| format!("{}: {e}", entry.display()))?;
    let mut scan = Scanner {
        paths,
        resolver,
        project: entry.parent().ok_or("Entry has no parent")?.to_owned(),
        targets: vec![],
        known: HashMap::new(),
        stack: vec![],
        order: vec![],
    };
    let entry = scan.visit(entry)?;
    Ok(BuildPlan {
        triplet: paths.map(|paths| paths.triplet.clone()),
        entry,
        targets: scan.targets,
        order: scan.order,
    })
}

impl Scanner<'_> {
    fn visit(&mut self, file: PathBuf) -> Result<TargetIndex, String> {
        if let Some((id, state)) = self.known.get(&file) {
            if *state == State::Resolved {
                return Ok(*id);
            }
            let start = self.stack.iter().position(|p| p == &file).unwrap_or(0);
            let cycle: Vec<_> = self.stack[start..]
                .iter()
                .chain(std::iter::once(&file))
                .map(|p| p.display().to_string())
                .collect();
            return Err(format!("Import cycle: {}", cycle.join(" -> ")));
        }
        let (input, imports, declared_name) = if file.extension().is_some_and(|ext| ext == "sym") {
            let manifest = crate::artifacts::load(&file)?.manifest;
            (
                TargetInput::Declaration(manifest.clone()),
                manifest.requires.clone(),
                Some(manifest.config.module_name),
            )
        } else {
            let text = fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
            let diagnostics =
                crate::diagnostic::error::ErrorHandle::new(file.to_string_lossy().into_owned());
            let tokens = mlc_syntax::lexer::tokenize(&text).map_err(|e| {
                let message = e.message();
                crate::diagnostic::error::ErrorHandle::summarize(
                    &diagnostics.render_error(&text, e.span, &message),
                    1,
                )
            })?;
            let (module, errors) = mlc_syntax::parser::parse(&tokens.tokens, text.len());
            if !errors.is_empty() {
                let rendered = errors
                    .iter()
                    .map(|e| {
                        diagnostics.render_error(
                            &text,
                            e.span().into_range(),
                            &mlc_syntax::parser::diagnostic::message(e),
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                return Err(crate::diagnostic::error::ErrorHandle::summarize(
                    &rendered,
                    errors.len(),
                ));
            }
            let module = module.ok_or("Parser returned no module")?;
            let imports: Vec<ImportModule> = module
                .iter()
                .filter_map(|(item, _)| match item {
                    TempGlobalStmt::Import(import) => Some(import.clone()),
                    _ => None,
                })
                .collect();
            (TargetInput::Source { text, module }, imports, None)
        };
        let module_name = declared_name.unwrap_or_else(|| self.module_name(&file));
        if self
            .targets
            .iter()
            .any(|target| target.module_name == module_name)
        {
            return Err(format!("Ambiguous module name: {module_name}"));
        }
        let id = TargetIndex(self.targets.len());
        self.known.insert(file.clone(), (id, State::Resolving));
        self.targets.push(Target {
            artifact_stems: self
                .paths
                .map(|paths| paths.artifact_stems(&file, &module_name))
                .transpose()?
                .unwrap_or_default(),
            id,
            file: file.clone(),
            module_name,
            input,
            requires: vec![],
            supers: vec![],
        });
        self.stack.push(file.clone());
        for import in imports {
            let dependency = self.resolver.resolve(&file, &import)?;
            let dependency = self.visit(dependency)?;
            if !self.targets[id.0].requires.contains(&dependency) {
                self.targets[id.0].requires.push(dependency);
                self.targets[dependency.0].supers.push(id);
            }
        }
        self.stack.pop();
        self.known.insert(file, (id, State::Resolved));
        self.order.push(id);
        Ok(id)
    }

    fn module_name(&self, file: &Path) -> String {
        // Canonical paths also normalize relative library roots and Windows prefixes.
        let library_relative = self.resolver.lib_dirs.iter().find_map(|root| {
            let root = root.canonicalize().ok()?;
            file.strip_prefix(root).ok().map(Path::to_owned)
        });
        let relative = library_relative
            .as_deref()
            .or_else(|| file.strip_prefix(&self.project).ok())
            .unwrap_or(file);
        relative
            .with_extension("")
            .components()
            .filter_map(|part| match part {
                std::path::Component::Normal(v) => Some(v.to_string_lossy().into_owned()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("::")
    }
}
