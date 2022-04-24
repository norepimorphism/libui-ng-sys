// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{env, path::PathBuf};

/// The error type returned by [`main`].
#[derive(Debug)]
pub enum Error {
    /// Failed to [sync](`dep::sync`) dependencies.
    SyncDep(anyhow::Error),
    /// Failed to build *libui*.
    BuildLibui(libui::Error),
    /// Failed to generate bindings.
    GenBindings(bindings::Error),
}

fn main() -> Result<(), Error> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());

    let libui_dir = out_dir.join("libui-ng");
    let meson_dir = out_dir.join("meson");
    let ninja_dir = out_dir.join("ninja");

    #[cfg(feature = "build")]
    {
        // Cargo will prevent this crate from being published if the build script modifies files
        // outside `$OUT_DIR` during its operation. To work around this for the purpose of building
        // *libui*, we copy all non-Rust build dependencies to `$OUT_DIR`.
        dep::sync("libui-ng", &libui_dir).map_err(Error::SyncDep)?;
        dep::sync("meson", &meson_dir).map_err(Error::SyncDep)?;
        dep::sync("ninja", &ninja_dir).map_err(Error::SyncDep)?;

        libui::build(&libui_dir, &meson_dir, &ninja_dir).map_err(Error::BuildLibui)?;

        // Tell Cargo where to find the copy of *libui* that we just built.
        println!(
            "cargo:rustc-link-search={}",
            libui_dir.join("build/meson-out/").display(),
        );
    }

    // Instruct Cargo to link---either statically or dynamically---to *libui*.
    println!("cargo:rustc-link-lib=ui");

    bindings::generate(&libui_dir, &out_dir).map_err(Error::GenBindings)?;

    // Recompile *libui-ng-sys* whenever this build script is modified.
    println!("cargo:rerun-if-changed=build.rs");

    Ok(())
}

#[cfg(feature = "build")]
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

#[cfg(feature = "build")]
mod libui {
    use std::path::Path;

    /// The error type returned by *libui* functions.
    #[derive(Debug)]
    pub enum Error {
        /// Failed to setup *libui*.
        SetupLibui(crate::meson::Error),
        /// Failed to build Ninja.
        BuildNinja(crate::ninja::Error),
        /// Failed to build *libui*.
        BuildLibui(crate::ninja::Error),
    }

    /// Builds *libui*.
    pub fn build(libui_dir: &Path, meson_dir: &Path, ninja_dir: &Path) -> Result<(), Error> {
        crate::meson::setup_libui(meson_dir, libui_dir).map_err(Error::SetupLibui)?;
        crate::ninja::build(ninja_dir).map_err(Error::BuildNinja)?;
        crate::ninja::build_libui(ninja_dir, libui_dir).map_err(Error::BuildLibui)
    }
}

#[cfg(feature = "build")]
mod meson {
    use std::{env, io, path::Path, process};

    /// The error type returned by *meson* functions.
    #[derive(Debug)]
    pub enum Error {
        /// Failed to run Python.
        RunPython(io::Error),
        /// The process run by Python failed.
        Python { out: process::Output },
    }

    /// Prepares *libui* to be built.
    pub fn setup_libui(meson_dir: &Path, libui_dir: &Path) -> Result<(), Error> {
        let out = process::Command::new("python")
            .arg(meson_dir.join("meson.py"))
            .arg("setup")
            .arg(format!("--default-library=static"))
            .arg(format!("--buildtype={}", env::var("PROFILE").unwrap()))
            .arg(libui_dir.join("build"))
            .arg(libui_dir)
            .output()
            .map_err(Error::RunPython)?;

        if out.status.success() {
            Ok(())
        } else {
            Err(Error::Python { out })
        }
    }
}

#[cfg(feature = "build")]
mod ninja {
    use std::{io, path::Path, process};

    /// The error type returned by *ninja* functions.
    #[derive(Debug)]
    pub enum Error {
        /// Failed to run Python.
        RunPython(io::Error),
        /// The process run by Python failed.
        Python { out: process::Output },
    }

    /// Builds Ninja.
    pub fn build(ninja_dir: &Path) -> Result<(), Error> {
        if ninja_dir.join("ninja").exists() {
            return Ok(());
        }

        let out = std::process::Command::new("python3")
            .arg("configure.py")
            .arg("--bootstrap")
            .current_dir(ninja_dir)
            .output()
            .map_err(Error::RunPython)?;

        if out.status.success() {
            Ok(())
        } else {
            Err(Error::Python { out })
        }
    }

    /// Builds *libui* with Ninja after configuration with Meson.
    pub fn build_libui(ninja_dir: &Path, libui_dir: &Path) -> Result<(), Error> {
        let out = std::process::Command::new(ninja_dir.join("ninja"))
            .args(["-C", "build"])
            .current_dir(libui_dir)
            .output()
            .map_err(Error::RunPython)?;

        if out.status.success() {
            Ok(())
        } else {
            Err(Error::Python { out })
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
        #[cfg(target_os = "macos")]
        Header::darwin().generate(libui_dir, out_dir)?;
        #[cfg(target_os = "linux")]
        Header::unix().generate(libui_dir, out_dir)?;
        #[cfg(target_os = "windows")]
        Header::windows().generate(libui_dir, out_dir)?;

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

        #[cfg(target_os = "macos")]
        fn darwin() -> Self {
            Self::ext("darwin", "Cocoa/Cocoa.h")
        }

        #[cfg(target_os = "linux")]
        fn unix() -> Self {
            Self::ext("unix", "gtk/gtk.h")
        }

        #[cfg(target_os = "windows")]
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
                .allowlist_var(LIBUI_REGEX);

            if self.blocklists_main {
                builder = builder.blocklist_file(".*ui\\.h");
            }

            builder
                .clang_args(ClangArgs::new().map(ClangArgs::as_args).unwrap_or_default())
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
        fn new() -> Option<Self> {
            if cfg!(target_os = "macos") {
                Some(Self::new_darwin())
            } else if cfg!(target_os = "linux") {
                Some(Self::new_unix())
            } else if cfg!(target_os = "windows") {
                Some(Self::new_windows())
            } else {
                None
            }
        }

        fn new_darwin() -> Self {
            // TODO
            Self {
                defines: Vec::new(),
                include_paths: Vec::new(),
            }
        }

        fn new_unix() -> Self {
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
                .map(|path| format!("{}", path.display()))
                .collect();

            Self {
                defines,
                include_paths,
            }
        }

        fn new_windows() -> Self {
            // TODO
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
