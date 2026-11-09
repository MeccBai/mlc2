use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum HostOs {
    Windows,
    Linux,
    Macos,
}
impl HostOs {
    pub fn for_os(os: &str) -> Result<Self, String> {
        match os {
            "windows" => Ok(Self::Windows),
            "linux" => Ok(Self::Linux),
            "macos" => Ok(Self::Macos),
            _ => Err(format!(
                "Constant shared-library execution is not configured for host OS {os}"
            )),
        }
    }
    pub fn current() -> Result<Self, String> {
        Self::for_os(std::env::consts::OS)
    }
    pub fn library(self) -> &'static str {
        match self {
            Self::Windows => "constant.dll",
            Self::Linux => "libconstant.so",
            Self::Macos => "libconstant.dylib",
        }
    }
    pub fn object(self) -> &'static str {
        if self == Self::Windows {
            "constant.obj"
        } else {
            "constant.o"
        }
    }
    pub fn export_attribute(self) -> &'static str {
        if self == Self::Windows {
            "dllexport "
        } else {
            ""
        }
    }
    fn linker(self) -> &'static str {
        match self {
            Self::Windows => "lld-link",
            Self::Linux => "ld.lld",
            Self::Macos => "ld64.lld",
        }
    }
    pub fn arguments(
        self,
        object: &Path,
        library: &Path,
        arch: &str,
        version: &str,
    ) -> Result<Vec<OsString>, String> {
        let mut args: Vec<OsString> = match self {
            Self::Windows => vec![
                "/dll".into(),
                "/noentry".into(),
                "/nologo".into(),
                "/export:__mlc_ctfe_entry".into(),
                format!("/out:{}", library.display()).into(),
            ],
            Self::Linux => vec![
                "-shared".into(),
                "-Bsymbolic".into(),
                "-o".into(),
                library.as_os_str().to_owned(),
            ],
            Self::Macos => {
                let arch = match arch {
                    "aarch64" | "arm64" => "arm64",
                    "x86_64" => "x86_64",
                    _ => return Err(format!("Unsupported macOS host architecture {arch}")),
                };
                vec![
                    "-dylib".into(),
                    "-arch".into(),
                    arch.into(),
                    "-platform_version".into(),
                    "macos".into(),
                    version.into(),
                    version.into(),
                    "-undefined".into(),
                    "dynamic_lookup".into(),
                    "-exported_symbol".into(),
                    "___mlc_ctfe_entry".into(),
                    "-o".into(),
                    library.as_os_str().to_owned(),
                ]
            }
        };
        args.push(object.as_os_str().to_owned());
        Ok(args)
    }
    pub fn link(
        self,
        root: &Path,
        object: &Path,
        library: &Path,
        triplet: &str,
    ) -> Result<(), String> {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
        let linker = find_linker(self.linker(), root, repository)?;
        let version = if self == Self::Macos {
            let output = Command::new("sw_vers")
                .arg("-productVersion")
                .output()
                .map_err(|e| format!("Cannot read macOS host version: {e}"))?;
            if !output.status.success() {
                return Err("Cannot read macOS host version with sw_vers".into());
            }
            let version = String::from_utf8(output.stdout).map_err(|e| e.to_string())?;
            let version = version.trim();
            if version.is_empty() || !version.chars().all(|c| c.is_ascii_digit() || c == '.') {
                return Err("Invalid macOS host version".into());
            }
            version.to_string()
        } else {
            String::new()
        };
        let mut command = Command::new(linker);
        command.args(self.arguments(
            object,
            library,
            triplet.split('-').next().unwrap_or(""),
            &version,
        )?);
        if self == Self::Windows {
            let runtime = [
                root.join("runtimes").join(triplet),
                repository.join("runtimes").join(triplet),
            ]
            .into_iter()
            .find(|p| p.is_dir())
            .unwrap_or_else(|| root.join("runtimes").join(triplet));
            for path in
                super::super::super::link::targets::profile(triplet)?.library_roots(&runtime)?
            {
                command.arg(format!("/libpath:{}", path.display()));
            }
            command.args(["msvcrt.lib", "vcruntime.lib", "ucrt.lib", "kernel32.lib"]);
        }
        let output = command.output().map_err(|e| e.to_string())?;
        if !output.status.success() {
            return Err(format!(
                "Cannot link constant shared library: {}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        Ok(())
    }
}
fn find_linker(name: &str, root: &Path, repository: &Path) -> Result<PathBuf, String> {
    let filename = format!("{name}{}", std::env::consts::EXE_SUFFIX);
    let mut candidates = vec![
        root.join("tools").join(&filename),
        repository.join("tools").join(&filename),
    ];
    if let Some(paths) = std::env::var_os("PATH") {
        candidates.extend(std::env::split_paths(&paths).map(|p| p.join(&filename)));
    }
    candidates
        .iter()
        .find(|p| p.is_file())
        .cloned()
        .ok_or_else(|| {
            format!(
                "Host linker {name} not found. Expected paths:\n{}",
                candidates
                    .iter()
                    .map(|p| format!("  {}", p.display()))
                    .collect::<Vec<_>>()
                    .join("\n")
            )
        })
}

pub(super) struct Library(*mut std::ffi::c_void);
impl Library {
    pub fn open(path: &Path) -> Result<Self, String> {
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            let path: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            let handle = unsafe { native::LoadLibraryW(path.as_ptr()) };
            if handle.is_null() {
                return Err(format!(
                    "Cannot load constant library: {}",
                    std::io::Error::last_os_error()
                ));
            }
            return Ok(Self(handle));
        }
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        {
            use std::os::unix::ffi::OsStrExt;
            let path =
                std::ffi::CString::new(path.as_os_str().as_bytes()).map_err(|e| e.to_string())?;
            let flags = 2 | if cfg!(target_os = "macos") { 4 } else { 0 }; // RTLD_NOW | RTLD_LOCAL.
            let handle = unsafe { native::dlopen(path.as_ptr(), flags) };
            if handle.is_null() {
                return Err(native::error());
            }
            return Ok(Self(handle));
        }
        #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
        {
            let _ = path;
            Err("Unsupported constant-library host OS".into())
        }
    }
    pub unsafe fn entry(&self) -> Result<unsafe extern "C" fn(*mut u8), String> {
        #[cfg(windows)]
        let address =
            unsafe { native::GetProcAddress(self.0, c"__mlc_ctfe_entry".as_ptr().cast()) };
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        let address = unsafe { native::dlsym(self.0, c"__mlc_ctfe_entry".as_ptr()) };
        #[cfg(not(any(windows, target_os = "linux", target_os = "macos")))]
        let address = std::ptr::null_mut::<std::ffi::c_void>();
        if address.is_null() {
            return Err("Constant entry export is missing".into());
        }
        Ok(unsafe {
            std::mem::transmute::<*mut std::ffi::c_void, unsafe extern "C" fn(*mut u8)>(address)
        })
    }
}
impl Drop for Library {
    fn drop(&mut self) {
        #[cfg(windows)]
        unsafe {
            native::FreeLibrary(self.0);
        }
        #[cfg(any(target_os = "linux", target_os = "macos"))]
        unsafe {
            native::dlclose(self.0);
        }
    }
}
#[cfg(windows)]
mod native {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        pub fn LoadLibraryW(path: *const u16) -> *mut std::ffi::c_void;
        pub fn GetProcAddress(
            module: *mut std::ffi::c_void,
            name: *const u8,
        ) -> *mut std::ffi::c_void;
        pub fn FreeLibrary(module: *mut std::ffi::c_void) -> i32;
    }
}
#[cfg(any(target_os = "linux", target_os = "macos"))]
mod native {
    use std::ffi::CStr;
    #[cfg_attr(target_os = "linux", link(name = "dl"))]
    unsafe extern "C" {
        pub fn dlopen(path: *const std::ffi::c_char, flags: i32) -> *mut std::ffi::c_void;
        pub fn dlsym(
            module: *mut std::ffi::c_void,
            name: *const std::ffi::c_char,
        ) -> *mut std::ffi::c_void;
        pub fn dlclose(module: *mut std::ffi::c_void) -> i32;
        fn dlerror() -> *const std::ffi::c_char;
    }
    pub fn error() -> String {
        unsafe {
            let error = dlerror();
            if error.is_null() {
                "Cannot load constant library".into()
            } else {
                format!(
                    "Cannot load constant library: {}",
                    CStr::from_ptr(error).to_string_lossy()
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn host_os_controls_library_format_and_link_flags() {
        for (os, name, flag) in [
            ("windows", "constant.dll", "/dll"),
            ("linux", "libconstant.so", "-shared"),
            ("macos", "libconstant.dylib", "-dylib"),
        ] {
            let host = HostOs::for_os(os).unwrap();
            assert_eq!(host.library(), name);
            let args = host
                .arguments(Path::new("constant.o"), Path::new(name), "x86_64", "14.0")
                .unwrap();
            assert!(args.iter().any(|a| a == flag));
            if os != "windows" {
                assert!(!args.iter().any(|a| a.to_string_lossy().starts_with('/')));
                assert_eq!(host.export_attribute(), "");
            }
        }
        let args = HostOs::Macos
            .arguments(
                Path::new("constant.o"),
                Path::new("libconstant.dylib"),
                "aarch64",
                "14.0",
            )
            .unwrap();
        assert!(
            args.windows(2)
                .any(|pair| pair == [OsString::from("-arch"), OsString::from("arm64")])
        );
        assert!(HostOs::for_os("unknown").is_err());
    }
    #[test]
    fn failed_library_load_returns_an_error() {
        assert!(Library::open(Path::new("__mlc_missing_constant_library__")).is_err());
    }
}

#[cfg(test)]
mod cross_link_tests {
    use super::*;
    #[test]
    #[ignore = "Requires ld.lld and ld64.lld on PATH or in tools/"]
    fn link_pic_objects_for_elf_and_macho() {
        let root = tempfile::tempdir().unwrap();
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
        for (os, triple, arch) in [
            (HostOs::Linux, "x86_64-unknown-linux-gnu", "x86_64"),
            (HostOs::Linux, "aarch64-unknown-linux-gnu", "aarch64"),
            (HostOs::Macos, "x86_64-apple-darwin", "x86_64"),
            (HostOs::Macos, "aarch64-apple-darwin", "aarch64"),
        ] {
            let compiler = mlc_builder::llvm::IrCompiler::with_o3(triple).unwrap();
            let object = root.path().join(os.object());
            let library = root.path().join(os.library());
            let ir = format!(
                "target triple = \"{triple}\"\n@payload = internal constant [4 x i8] c\"abc\\00\"\ndeclare ptr @malloc(i64)\ndefine void @__mlc_ctfe_entry(ptr %out) {{\nentry:\n  %p = call ptr @malloc(i64 4)\n  store ptr %p, ptr %out\n  %data = getelementptr [4 x i8], ptr @payload, i64 0, i64 0\n  %slot = getelementptr i8, ptr %out, i64 8\n  store ptr %data, ptr %slot\n  ret void\n}}\n"
            );
            compiler.emit_o3(ir, &object).unwrap();
            let linker = find_linker(os.linker(), root.path(), repository).unwrap();
            let output = Command::new(linker)
                .args(os.arguments(&object, &library, arch, "14.0").unwrap())
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{triple}: {}{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            let image = std::fs::read(library).unwrap();
            if os == HostOs::Linux {
                assert_eq!(&image[..4], b"\x7fELF");
                assert_eq!(u16::from_le_bytes(image[16..18].try_into().unwrap()), 3);
            } else {
                assert_eq!(&image[..4], &[0xcf, 0xfa, 0xed, 0xfe]);
                assert_eq!(u32::from_le_bytes(image[12..16].try_into().unwrap()), 6);
            }
        }
    }
}
