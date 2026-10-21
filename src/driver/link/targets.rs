mod msvc;
use mlc_builder::project::TargetKind;
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

pub struct Profile {
    pub linker: &'static str,
    flags: &'static [&'static str],
    libraries: &'static [&'static str],
    msvc: bool,
}

// Native linker arguments, in link order. Add a profile only with its matching CRT.
pub fn profile(triplet: &str) -> Result<&'static Profile, String> {
    static WINDOWS_GNU: Profile = Profile {
        linker: "ld.lld",
        msvc: false,
        flags: &["-m", "i386pep", "-Bdynamic"],
        libraries: &[
            "libmingw32.a",
            "libclang_rt.builtins-x86_64.a",
            "libmoldname.a",
            "libmingwex.a",
            "libmsvcrt.a",
            "libadvapi32.a",
            "libshell32.a",
            "libuser32.a",
            "libkernel32.a",
        ],
    };
    static WINDOWS_MSVC: Profile = Profile {
        linker: "lld-link",
        msvc: true,
        flags: &["/machine:x64", "/nologo"],
        libraries: &[
            "msvcrt.lib",
            "vcruntime.lib",
            "ucrt.lib",
            "kernel32.lib",
            "legacy_stdio_definitions.lib",
            "oldnames.lib",
        ],
    };
    match triplet {
        "x86_64-pc-windows-msvc" => Ok(&WINDOWS_MSVC),
        "x86_64-pc-windows-gnu" => Ok(&WINDOWS_GNU),
        _ => Err(format!(
            "No native linker argument table configured for triplet: {triplet}"
        )),
    }
}

impl Profile {
    pub fn library_roots(&self, runtime: &Path) -> Result<Vec<PathBuf>, String> {
        if !self.msvc {
            return Ok(vec![runtime.to_owned()]);
        }
        if self
            .libraries
            .iter()
            .all(|name| runtime.join(name).is_file())
        {
            return Ok(vec![runtime.to_owned()]);
        }
        let mut roots = vec![runtime.to_owned()];
        roots.extend(msvc::discover()?);
        for name in self.libraries {
            if !roots.iter().any(|root| root.join(name).is_file()) {
                return Err(format!(
                    "MSVC/UCRT library not found: {name}. Install the C++ Build Tools and Windows SDK, or provide runtimes/x86_64-pc-windows-msvc."
                ));
            }
        }
        Ok(roots)
    }

    pub fn runtime_files(&self, runtime: &Path) -> Result<Vec<PathBuf>, String> {
        if self.msvc {
            let roots = self.library_roots(runtime)?;
            // Hash all libraries in these search directories: CRT default-library
            // directives can introduce dependencies beyond our explicit table.
            let mut files = Vec::new();
            for root in roots.into_iter().filter(|root| root.is_dir()) {
                for entry in std::fs::read_dir(root).map_err(|e| e.to_string())? {
                    let path = entry.map_err(|e| e.to_string())?.path();
                    if path
                        .extension()
                        .is_some_and(|ext| ext.eq_ignore_ascii_case("lib"))
                    {
                        files.push(path);
                    }
                }
            }
            return Ok(files);
        }
        std::fs::read_dir(runtime)
            .map_err(|e| format!("{}: {e}", runtime.display()))?
            .map(|entry| entry.map(|entry| entry.path()).map_err(|e| e.to_string()))
            .collect()
    }

    pub fn arguments(
        &self,
        runtime: &Path,
        objects: &[PathBuf],
        output: &Path,
        kind: TargetKind,
    ) -> Result<Vec<OsString>, String> {
        if self.msvc {
            let roots = self.library_roots(runtime)?;
            let mut args: Vec<OsString> = self.flags.iter().map(OsString::from).collect();
            args.push(format!("/out:{}", output.display()).into());
            args.push(
                if kind == TargetKind::Shared {
                    "/dll"
                } else {
                    "/subsystem:console"
                }
                .into(),
            );
            args.extend(
                roots
                    .iter()
                    .map(|root| format!("/libpath:{}", root.display()).into()),
            );
            args.extend(objects.iter().map(|path| path.as_os_str().to_owned()));
            args.extend(self.libraries.iter().map(OsString::from));
            return Ok(args);
        }
        let mut args: Vec<OsString> = self.flags.iter().map(OsString::from).collect();
        if kind == TargetKind::Shared {
            args.push("--shared".into());
        }
        args.extend([OsString::from("-o"), output.as_os_str().to_owned()]);
        let startup = if kind == TargetKind::Shared {
            "dllcrt2.o"
        } else {
            "crt2.o"
        };
        let file = |name: &str| -> Result<OsString, String> {
            let path = runtime.join(name);
            if !path.is_file() {
                return Err(format!(
                    "Required runtime file not found: {}",
                    path.display()
                ));
            }
            Ok(path.into_os_string())
        };
        args.push(file(startup)?);
        args.push(file("crtbegin.o")?);
        args.push(format!("-L{}", runtime.display()).into());
        args.extend(objects.iter().map(|p| p.as_os_str().to_owned()));
        for lib in self.libraries {
            args.push(file(lib)?);
        }
        args.push(file("crtend.o")?);
        Ok(args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bundled_msvc_table_selects_coff_linker_and_avoids_sdk_discovery() {
        let runtime = tempfile::tempdir().unwrap();
        let profile = profile("x86_64-pc-windows-msvc").unwrap();
        for name in profile.libraries {
            std::fs::write(runtime.path().join(name), "").unwrap();
        }
        assert_eq!(profile.linker, "lld-link");
        assert_eq!(
            profile.library_roots(runtime.path()).unwrap(),
            vec![runtime.path()]
        );
        let args = profile
            .arguments(
                runtime.path(),
                &[PathBuf::from("main.obj")],
                Path::new("app.exe"),
                TargetKind::Bin,
            )
            .unwrap();
        assert!(args.contains(&OsString::from("/subsystem:console")));
        assert!(args.contains(&OsString::from("/out:app.exe")));
        assert!(args.contains(&OsString::from("ucrt.lib")));
        assert!(
            !args
                .iter()
                .any(|arg| arg.to_string_lossy().contains("mingw"))
        );
        let args = profile
            .arguments(
                runtime.path(),
                &[],
                Path::new("app.dll"),
                TargetKind::Shared,
            )
            .unwrap();
        assert!(args.contains(&OsString::from("/dll")));
        assert!(!args.contains(&OsString::from("/subsystem:console")));
    }
    #[test]
    fn gnu_profile_is_explicit_and_unknown_targets_fail() {
        assert_eq!(profile("x86_64-pc-windows-gnu").unwrap().linker, "ld.lld");
        assert!(profile("unconfigured-target").is_err());
    }
}
