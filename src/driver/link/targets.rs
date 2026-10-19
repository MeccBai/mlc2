use mlc_builder::project::TargetKind;
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

pub struct Profile {
    pub linker: &'static str,
    flags: &'static [&'static str],
    libraries: &'static [&'static str],
}

// Native linker arguments, in link order. Add a profile only with its matching CRT.
pub fn profile(triplet: &str) -> Result<&'static Profile, String> {
    static WINDOWS_GNU: Profile = Profile {
        linker: "ld.lld",
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
    match triplet {
        "x86_64-pc-windows-gnu" => Ok(&WINDOWS_GNU),
        _ => Err(format!(
            "No native linker argument table configured for triplet: {triplet}"
        )),
    }
}

impl Profile {
    pub fn arguments(
        &self,
        runtime: &Path,
        objects: &[PathBuf],
        output: &Path,
        kind: TargetKind,
    ) -> Result<Vec<OsString>, String> {
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
