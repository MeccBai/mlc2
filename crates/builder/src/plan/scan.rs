use super::paths::ImportSearch;
use super::*;
use crate::paths::PathResolver;
use mlc_core::ast::builtins::BuiltinModule;
use mlc_syntax::{ImportModule, parser::out::TempGlobalStmt};
use std::{collections::HashMap, fs};

#[derive(Clone, Copy, PartialEq)]
enum State {
    Resolving,
    Resolved,
}
struct Scanner<'a> {
    paths: Option<&'a PathResolver>,
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
    paths: Option<&PathResolver>,
) -> Result<BuildPlan, mlc_core::diagnostic::Report> {
    let entry = entry.canonicalize().map_err(|e| {
        format!(
            "Cannot open source: {e}\nExpected path: {}",
            std::path::absolute(entry)
                .unwrap_or_else(|_| entry.to_owned())
                .display()
        )
    })?;
    let mut scan = Scanner {
        paths,
        resolver,
        project: entry.parent().ok_or("Entry has no parent")?.to_owned(),
        targets: vec![],
        known: HashMap::new(),
        stack: vec![],
        order: vec![],
    };
    let entry = scan.visit(entry, None)?;
    Ok(BuildPlan {
        triplet: paths.map(|paths| paths.triplet.clone()),
        entry,
        targets: scan.targets,
        order: scan.order,
    })
}

impl Scanner<'_> {
    fn visit_builtin(&mut self, module: BuiltinModule) -> TargetIndex {
        if let Some(target) = self.targets.iter().find(
            |target| matches!(target.input, TargetInput::Builtin(existing) if existing == module),
        ) {
            return target.id;
        }
        let id = TargetIndex(self.targets.len());
        self.targets.push(Target {
            artifact_stems: vec![],
            id,
            file: PathBuf::from("<builtin>").join(module.name().replace("::", "/")),
            module_name: module.name().into(),
            input: TargetInput::Builtin(module),
            requires: vec![],
            reexports: vec![],
            supers: vec![],
        });
        self.order.push(id);
        id
    }

    fn visit(
        &mut self,
        file: PathBuf,
        builtin_shadow: Option<BuiltinModule>,
    ) -> Result<TargetIndex, mlc_core::diagnostic::Report> {
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
            return Err(format!("Import cycle: {}", cycle.join(" -> ")).into());
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
            let source: std::sync::Arc<str> = text.clone().into();
            let tokens = mlc_syntax::lexer::tokenize(&text).map_err(|e| {
                mlc_core::diagnostic::Report::new(vec![
                    mlc_core::diagnostic::Diagnostic::error(e.message()).at(
                        file.to_string_lossy(),
                        source.clone(),
                        e.span,
                    ),
                ])
            })?;
            let (module, errors) = mlc_syntax::parser::parse(&tokens.tokens, text.len());
            if !errors.is_empty() {
                return Err(mlc_core::diagnostic::Report::new(
                    errors
                        .iter()
                        .map(|e| {
                            mlc_core::diagnostic::Diagnostic::error(
                                mlc_syntax::parser::diagnostic::message(e),
                            )
                            .at(
                                file.to_string_lossy(),
                                source.clone(),
                                e.span().into_range(),
                            )
                        })
                        .collect(),
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
            (
                TargetInput::Source {
                    text,
                    module,
                    lines: tokens.lines,
                },
                imports,
                None,
            )
        };
        let module_name = declared_name.unwrap_or_else(|| {
            builtin_shadow
                .map(|module| module.name().into())
                .unwrap_or_else(|| self.module_name(&file))
        });
        if self
            .targets
            .iter()
            .any(|target| target.module_name == module_name)
        {
            return Err(format!("Ambiguous module name: {module_name}").into());
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
            reexports: vec![],
            supers: vec![],
        });
        self.stack.push(file.clone());
        for import in imports {
            let cached_declarations = self
                .paths
                .map(|paths| paths.cached_declaration_roots(&import.path().join("::")))
                .unwrap_or_default();
            let builtin = BuiltinModule::for_import(import.path());
            let local = if builtin.is_some() {
                let roots = self
                    .paths
                    .map(|paths| paths.project_root.as_path())
                    .into_iter()
                    .chain(std::iter::once(self.project.as_path()))
                    .chain(self.resolver.lib_dirs.iter().map(PathBuf::as_path));
                match self.resolver.resolve_in_roots(&import, roots, &[], true)? {
                    ImportSearch::Found(path) => Some(path),
                    ImportSearch::Missing(_) => None,
                }
            } else {
                None
            };
            let dependency = match local {
                Some(path) => Some(path),
                None => self
                    .paths
                    .map(|paths| paths.package_import(&file, &import))
                    .transpose()?
                    .flatten(),
            };
            let dependency = match dependency {
                Some(path) => ImportSearch::Found(path),
                None => {
                    let mut roots = Vec::new();
                    if builtin.is_none() {
                        roots.push(file.parent().ok_or("Importing file has no directory")?);
                        roots.extend(self.resolver.lib_dirs.iter().map(PathBuf::as_path));
                    } else if self.paths.is_some() {
                        roots.extend(
                            self.resolver
                                .lib_dirs
                                .iter()
                                .filter_map(|root| root.parent()),
                        );
                    }
                    self.resolver.resolve_in_roots(
                        &import,
                        roots,
                        &cached_declarations,
                        builtin.is_some(),
                    )?
                }
            };
            let dependency = match (dependency, builtin) {
                (ImportSearch::Missing(_), Some(module)) => self.visit_builtin(module),
                (search, _) => self.visit(search.require(&file, &import)?, builtin)?,
            };
            if import.exported() && !self.targets[id.0].reexports.contains(&dependency) {
                self.targets[id.0].reexports.push(dependency);
            }
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
        if let Some(name) = self.paths.and_then(|p| p.package_module_name(file)) {
            return name;
        }
        // Canonical paths also normalize relative library roots and Windows prefixes.
        let library_relative = self.resolver.source_roots().iter().find_map(|root| {
            let root = root.canonicalize().ok()?;
            file.strip_prefix(root).ok().map(Path::to_owned)
        });
        let project_relative = self
            .paths
            .and_then(|paths| paths.project_root.canonicalize().ok())
            .and_then(|root| file.strip_prefix(root).ok());
        let relative = library_relative
            .as_deref()
            .or_else(|| file.strip_prefix(&self.project).ok())
            .or(project_relative)
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
