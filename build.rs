// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#[macro_use]
extern crate build_cfg;

use std::{env, io, path::{Path, PathBuf}};

/// The error type returned by [`main`].
#[derive(Debug)]
pub enum Error {
    /// Failed to [sync](`dep::sync`) dependencies.
    SyncDep(anyhow::Error),
    SetPermissions(io::Error),
    /// Failed to build *libui*.
    #[cfg(feature = "build")]
    BuildLibui(build::Error),
    /// Failed to include Windows resources.
    IncludeWinres(io::Error),
    /// Failed to generate bindings to *libui*.
    GenBindings(bindings::Error),
}

#[build_cfg_main]
fn main() -> Result<(), Error> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let libui_dir = out_dir.join("libui-ng");
    let meson_dir = out_dir.join("meson");
    let ninja_dir = out_dir.join("ninja");

    // Cargo will prevent this crate from being published if the build script modifies files outside
    // `$OUT_DIR` during its operation. To work around this for the purpose of building *libui*, we
    // copy all non-Rust build dependencies to `$OUT_DIR`.
    dep::sync("libui-ng", &libui_dir).map_err(Error::SyncDep)?;

    #[cfg(feature = "build")]
    if env::var("DOCS_RS").is_err() {
        let backend = build::Backend::default();

        dep::sync("meson", &meson_dir).map_err(Error::SyncDep)?;
        // Ninja only needs to be synced if it's selected as a build backend.
        if let build::Backend::Ninja = backend {
            // When downloading crates from *crates.io*, file execute permissions are *not*
            // respected. This is a problem for Ninja, which attempts to execute a file named
            // *inline.sh*. For this reason, we manually mark it as executable.
            #[cfg(unix)]
            mark_executable("dep/ninja/src/inline.sh");

            dep::sync("ninja", &ninja_dir).map_err(Error::SyncDep)?;
        }

        backend.build_libui(&libui_dir, &meson_dir, &ninja_dir).map_err(Error::BuildLibui)?;

        // Tell Cargo where to find the copy of *libui* that we just built.
        println!(
            "cargo:rustc-link-search={}",
            libui_dir.join("build/meson-out/").display(),
        );

        // Because we are building *libui* from scratch and placing it in `$OUT_DIR`, it makes sense
        // to link statically. Consequently, as static libraries *do not* contain information on the
        // shared objects that must be imported, we must tell Cargo (and, by extension, the dynamic
        // linker) which shared objects we need.
        import_dylibs();

        if build_cfg!(target_os = "windows") && cfg!(feature = "include-win-manifest") {
            include_winres().map_err(Error::IncludeWinres)?;
        }
    }

    // Instruct Cargo to link to *libui*.
    println!("cargo:rustc-link-lib={}=ui", link_kind());

    bindings::generate(&libui_dir, &out_dir).map_err(Error::GenBindings)?;

    // Recompile *libui-ng-sys* whenever this build script is modified.
    println!("cargo:rerun-if-changed=build.rs");

    Ok(())
}

#[cfg(all(feature = "build", unix))]
fn mark_executable(path: impl AsRef<Path>) -> Result<(), Error> {
    use std::{fs, os::unix::fs::PermissionsExt as _};

    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).map_err(Error::SetPermissions)
}

#[cfg(feature = "build")]
fn import_dylibs() {
    macro_rules! dyn_link {
        ($($name:tt)*) => {
            $(
                println!("cargo:rustc-link-lib=dylib={}", stringify!($name));
            )*
        };
    }

    if build_cfg!(target_os = "linux") {
        // While unintuitive, we don't actually need to specify any shared objects here---the
        // `pkg_config` crate will do that automatically in [`bindings::ClangArgs::new_linux`].
    } else if build_cfg!(target_os = "windows") {
        // See `dep/libui-ng/windows/meson.build`.
        dyn_link! {
            comctl32
            comdlg32
            d2d1
            dwrite
            gdi32
            kernel32
            msimg32
            ole32
            oleacc
            oleaut32
            user32
            uuid
            uxtheme
            windowscodecs
        };
    }
}

fn include_winres() -> io::Result<()> {
    winres::WindowsResource::new()
        .set_manifest_file(&Path::new("res/libui.manifest").display().to_string())
        .compile()
}

fn link_kind() -> &'static str {
    if cfg!(feature = "build") {
        "static"
    } else {
        "dylib"
    }
}

mod dep {
    use std::path::Path;

    pub fn sync(name: &str, to: &Path) -> Result<(), anyhow::Error> {
        rusync::Syncer::new(
            &Path::new("dep").join(name),
            to,
            rusync::SyncOptions {
                preserve_permissions: true,
            },
            Box::new(FakeProgressInfo),
        )
        .sync()
        .map(|_| ())
    }

    struct FakeProgressInfo;

    impl rusync::progress::ProgressInfo for FakeProgressInfo {}
}

mod build {
    use std::{env, fs, io, path::{Path, PathBuf}, process};

    /// The error type returned by [`Backend`] functions.
    #[derive(Debug)]
    pub enum Error {
        /// Failed to setup *libui*.
        SetupLibui(PythonError),
        /// Failed to build Ninja.
        BuildNinja(PythonError),
        /// Failed to compile *libui*.
        CompileLibui(PythonError),
        /// Failed to rename `libui.a` to `ui.lib`.
        ///
        /// This error *should* only occur when `$CARGO_CFG_TARGET_OS` is `windows`.
        RenameLibui(io::Error),
    }

    #[derive(Debug)]
    pub enum PythonError {
        /// Failed to run Python.
        RunPython(io::Error),
        /// The process run by Python failed.
        Python { out: process::Output },
    }

    pub enum Backend {
        Msvc,
        Ninja,
        Xcode,
    }

    impl Default for Backend {
        fn default() -> Self {
            if build_cfg!(feature = "build-with-msvc") {
                Self::Msvc
            } else if build_cfg!(feature = "build-with-xcode") {
                Self::Xcode
            // Ninja is last because it is the default option. This way, even if the user forgets to
            // pass `--no-default-options` and both `build-with-ninja` and, e.g., `build-with-msvc`
            // are enabled, only `build-with-msvc` will take effect, and the build backend will be
            // MSVC.
            } else if build_cfg!(feature = "build-with-ninja") {
                Self::Ninja
            } else {
                panic!(
                    "
                    The `build` feature is enabled but no `build-with-*` feature is not enabled. \
                    *libui-ng-sys* doesn't know which build backend to use. \
                    "
                );
            }
        }
    }

    impl Backend {
        /// Builds *libui*.
        pub fn build_libui(
            self,
            libui_dir: &Path,
            meson_dir: &Path,
            ninja_dir: &Path,
        ) -> Result<(), Error> {
            if Self::libui_path(libui_dir).exists() {
                // We'll give the benefit of the doubt that this is actually a complete, working
                // library.
                return Ok(());
            }

            if let Self::Ninja = self {
                // This must precede setting up *libui* as Meson requires Ninja even in the
                // configuration phase.
                Self::build_ninja(ninja_dir).map_err(Error::BuildNinja)?;
            }

            self.setup_libui(libui_dir, meson_dir, ninja_dir).map_err(Error::SetupLibui)?;
            self.compile_libui(libui_dir, meson_dir, ninja_dir)
                .map_err(Error::CompileLibui)?;
            self.rename_libui(libui_dir).map_err(Error::RenameLibui)?;

            Ok(())
        }

        fn libui_path(libui_dir: &Path) -> PathBuf {
            libui_dir.join("libui.a")
        }

        fn ninja_path(ninja_dir: &Path) -> PathBuf {
            let ext = env::consts::EXE_EXTENSION;
            ninja_dir.join("ninja").with_extension(ext)
        }

        fn run_python(
            f: impl Fn(&mut process::Command),
            ninja_dir: Option<&Path>,
        ) -> Result<(), PythonError> {
            let mut cmd = process::Command::new("python3");
            f(&mut cmd);

            if let Some(dir) = ninja_dir {
                cmd.env("NINJA", Self::ninja_path(dir));
            }

            let out = cmd.output().map_err(PythonError::RunPython)?;
            if out.status.success() {
                Ok(())
            } else {
                Err(PythonError::Python { out })
            }
        }

        /// Builds Ninja.
        fn build_ninja(ninja_dir: &Path) -> Result<(), PythonError> {
            if Self::ninja_path(ninja_dir).exists() {
                // We'll give the benefit of the doubt that this is actually a complete, working
                // binary.
                return Ok(());
            }

            Self::run_python(
                |cmd| {
                    cmd
                        .arg("configure.py")
                        .arg("--bootstrap")
                        .current_dir(ninja_dir);
                },
                None,
            )
        }

        /// Prepares *libui* to be built.
        fn setup_libui(
            &self,
            libui_dir: &Path,
            meson_dir: &Path,
            ninja_dir: &Path,
        ) -> Result<(), PythonError> {
            Self::run_python(
                |cmd| {
                    cmd
                        .arg(meson_dir.join("meson.py"))
                        .arg("setup")
                        .arg("--default-library=static")
                        .arg("--buildtype=release")
                        .arg(format!("--optimization={}", Self::optimization_level()))
                        .arg(format!("--backend={}", self.as_str()))
                        // It's OK that this option is hardcoded (which is MSVC-specific) for all
                        // backends; Meson will simply ignore it if MSVC isn't the selected backend.
                        .arg("-Db_vscrt=from_buildtype")
                        .arg(libui_dir.join("build"))
                        .arg(libui_dir);
                },
                Some(ninja_dir),
            )
        }

        // This may be used at some point.
        #[allow(dead_code)]
        fn is_debug() -> bool {
            !matches!(env::var("DEBUG").as_deref(), Ok("0" | "false"))
        }

        fn optimization_level() -> String {
            let level = env::var("OPT_LEVEL").expect("$OPT_LEVEL is unset");
            match level.as_str() {
                // Meson doesn't support "-Oz"; we'll try the next-closest option.
                "z" => String::from("s"),
                _ => level,
            }
        }

        fn as_str(&self) -> &'static str {
            match self {
                Self::Msvc => "vs",
                Self::Ninja => "ninja",
                Self::Xcode => "xcode",
            }
        }

        fn compile_libui(
            &self,
            libui_dir: &Path,
            meson_dir: &Path,
            ninja_dir: &Path,
        ) -> Result<(), PythonError> {
            Self::run_python(
                |cmd| {
                    cmd
                        .arg(meson_dir.join("meson.py"))
                        .arg("compile")
                        .arg(format!("-C={}", libui_dir.join("build").display()));
                },
                Some(ninja_dir),
            )
        }

        fn rename_libui(&self, libui_dir: &Path) -> Result<(), io::Error> {
            // Meson unconditionally names the library "libui.a", which prevents MSVC's `link.exe`
            // from finding it; we must manually rename it to "ui.lib".
            if let Self::Msvc = self {
                let build_dir = libui_dir.join("build/meson-out");
                fs::rename(Self::libui_path(libui_dir), build_dir.join("ui.lib"))?;
            }

            Ok(())
        }
    }
}

mod bindings {
    use std::{fmt, io, path::Path};

    /// The error type returned by binding functions.
    #[derive(Debug)]
    pub enum Error {
        /// Failed to generate bindings.
        Generate,
        /// Failed to write bindings to a file.
        WriteToFile(io::Error),
    }

    /// Generates bindings to *libui* and writes them to the given directory.
    pub fn generate(libui_dir: &Path, out_dir: &Path) -> Result<(), Error> {
        Header::main().generate(libui_dir, out_dir)?;
        Header::control_sigs().generate(libui_dir, out_dir)?;

        if build_cfg!(target_os = "macos") {
            Header::darwin().generate(libui_dir, out_dir)?;
        }
        if build_cfg!(target_os = "linux") {
            Header::unix().generate(libui_dir, out_dir)?;
        }
        if build_cfg!(target_os = "windows") {
            Header::windows().generate(libui_dir, out_dir)?;
        }

        Ok(())
    }

    struct Header {
        include_stmts: Vec<IncludeStmt>,
        filename: String,
        blocklists_main: bool,
    }

    impl Header {
        fn main() -> Self {
            Self {
                include_stmts: vec![
                    IncludeStmt {
                        kind: IncludeStmtKind::Local,
                        arg: "ui.h".to_string(),
                    },
                ],
                filename: "bindings".to_string(),
                blocklists_main: false,
            }
        }

        fn control_sigs() -> Self {
            Self {
                include_stmts: vec![
                    IncludeStmt {
                        kind: IncludeStmtKind::Local,
                        arg: "common/controlsigs.h".to_string(),
                    },
                ],
                filename: "bindings-control-sigs".to_string(),
                blocklists_main: true,
            }
        }

        fn darwin() -> Self {
            Self::ext("darwin", "Cocoa/Cocoa.h")
        }

        fn unix() -> Self {
            Self::ext("unix", "gtk/gtk.h")
        }

        fn windows() -> Self {
            Self::ext("windows", "windows.h")
        }

        fn ext(name: impl fmt::Display, dep: impl Into<String>) -> Self {
            Self {
                include_stmts: vec![
                    IncludeStmt {
                        kind: IncludeStmtKind::Local,
                        arg: "ui.h".to_string(),
                    },
                    IncludeStmt {
                        kind: IncludeStmtKind::System,
                        arg: dep.into(),
                    },
                    IncludeStmt {
                        kind: IncludeStmtKind::Local,
                        arg: format!("ui_{}.h", name),
                    },
                ],
                filename: format!("bindings-{}", name),
                blocklists_main: true,
            }
        }

        fn generate(self, libui_dir: &Path, out_dir: &Path) -> Result<(), Error> {
            static LIBUI_REGEX: &str = "ui(?:[A-Z][a-z0-9]*)*";

            let mut builder = bindgen::builder()
                .header_contents("wrapper.h", &self.contents(libui_dir))
                .parse_callbacks(Box::new(bindgen::CargoCallbacks))
                .allowlist_function(LIBUI_REGEX)
                .allowlist_type(LIBUI_REGEX)
                .allowlist_var(LIBUI_REGEX)
                .blocklist_item("_bindgen.*");

            // Note: Virtually every wrapper except that for "ui.h" should blocklist "ui.h".
            if self.blocklists_main {
                builder = builder.blocklist_file(".*ui\\.h");
            }

            builder
                .clang_args(ClangArgs::new().as_args())
                .layout_tests(false)
                .generate()
                .map_err(|_| Error::Generate)?
                .write_to_file(out_dir.join(format!("{}.rs", self.filename)))
                .map_err(Error::WriteToFile)
        }

        fn contents(&self, libui_dir: &Path) -> String {
            self
                .include_stmts
                .iter()
                .map(|stmt| stmt.to_string(libui_dir))
                .collect::<Vec<String>>()
                .join("\n")
        }
    }

    struct IncludeStmt {
        kind: IncludeStmtKind,
        arg: String,
    }

    enum IncludeStmtKind {
        System,
        Local,
    }

    impl IncludeStmt {
        fn to_string(&self, libui_dir: &Path) -> String {
            format!(
                "#include {}",
                match self.kind {
                    IncludeStmtKind::System => format!("<{}>", self.arg),
                    IncludeStmtKind::Local => format!(
                        "\"{}\"",
                        libui_dir.join(&self.arg).display(),
                    ),
                },
            )
        }
    }

    struct ClangArgs {
        defines: Vec<ClangDefine>,
        include_paths: Vec<String>,
    }

    struct ClangDefine {
        key: String,
        value: Option<String>,
    }

    impl ClangArgs {
        fn new() -> Self {
            if build_cfg!(target_os = "macos") {
                Self::new_macos()
            } else if build_cfg!(target_os = "linux") {
                Self::new_linux()
            } else if build_cfg!(target_os = "windows") {
                Self::new_windows()
            } else {
                unimplemented!("Unsupported target OS");
            }
        }

        fn new_macos() -> Self {
            Self {
                defines: Vec::new(),
                include_paths: Vec::new(),
            }
        }

        fn new_linux() -> Self {
            let gtk = pkg_config::Config::new()
                .atleast_version("3.10.0")
                .print_system_cflags(true)
                .print_system_libs(true)
                .probe("gtk+-3.0")
                .unwrap();

            let defines = gtk
                .defines
                .into_iter()
                .map(|(key, value)| {
                    ClangDefine { key, value }
                })
                .collect();

            let include_paths = gtk
                .include_paths
                .into_iter()
                .map(|path| path.display().to_string())
                .collect();

            Self {
                defines,
                include_paths,
            }
        }

        fn new_windows() -> Self {
            Self {
                defines: Vec::new(),
                include_paths: Vec::new(),
            }
        }

        fn as_args(self) -> Vec<String> {
            let defines = self
                .defines
                .into_iter()
                .flat_map(|define| {
                    vec![
                        "-D".to_string(),
                        format!(
                            "{}{}",
                            define.key,
                            define.value.map(|it| format!("={}", it)).unwrap_or_default(),
                        ),
                    ]
                });

            let includes = self
                .include_paths
                .into_iter()
                .flat_map(|path| {
                    vec![
                        "-I".to_string(),
                        path.to_string(),
                    ]
                });

            defines.chain(includes).collect()
        }
    }
}
