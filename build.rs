// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{env, path::PathBuf};

struct Dep {
    dir: PathBuf,
    head: &'static str,
}

impl Dep {
    fn libui() -> Self {
        Dep {
            dir: PathBuf::from("dep/libui-ng"),
            head: "42641e3d6bfb2c49ca4cc3b03d8ae277d9841a5d",
        }
    }

    fn meson() -> Self {
        Dep {
            dir: PathBuf::from("dep/meson"),
            head: "09ad4e28f1a59ab3d87de6f36540a108e836cfe5",
        }
    }

    fn ninja() -> Self {
        Dep {
            dir: PathBuf::from("dep/ninja"),
            head: "25cdbae0ee1270a5c8dd6ba67696e29ad8076919",
        }
    }
}

#[derive(Debug)]
pub enum Error {
    Repo(repo::Error),
    Meson(meson::Error),
    Ninja(ninja::Error),
    Bindings(bindings::Error),
}

fn main() -> Result<(), Error> {
    let libui = Dep::libui();
    let meson = Dep::meson();
    let ninja = Dep::ninja();

    libui.update()?;
    meson.update()?;
    ninja.update()?;

    libui::build(&libui.dir, &meson.dir, &ninja.dir)?;

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings::gen(&libui.dir, &out_dir).map_err(Error::Bindings)?;

    println!(
        "cargo:rustc-link-search={}",
        libui.dir.join("build/meson-out/").display(),
    );
    println!("cargo:rustc-link-lib=ui");

    println!("cargo:rerun-if-changed=build.rs");

    Ok(())
}

impl Dep {
    fn update(&self) -> Result<(), Error> {
        repo::update(&self.dir, self.head).map_err(Error::Repo)
    }
}

mod repo {
    use std::path::Path;

    #[derive(Debug)]
    pub enum Error {
        Open(git2::Error),
        CreateOid(git2::Error),
        SetHead(git2::Error),
        CheckoutHead(git2::Error),
    }

    pub fn update(repo_dir: &Path, new_head: &str) -> Result<(), Error> {
        let repo = git2::Repository::open(repo_dir).map_err(Error::Open)?;
        let new_head = git2::Oid::from_str(new_head).map_err(Error::CreateOid)?;
        repo.set_head_detached(new_head).map_err(Error::SetHead)?;
        repo.checkout_head(None).map_err(Error::CheckoutHead)
    }
}

mod libui {
    use crate::Error;
    use std::path::Path;

    pub fn build(libui_dir: &Path, meson_dir: &Path, ninja_dir: &Path) -> Result<(), Error> {
        crate::meson::setup_libui(meson_dir, libui_dir).map_err(Error::Meson)?;
        crate::ninja::build(ninja_dir).map_err(Error::Ninja)?;
        crate::ninja::build_libui(ninja_dir, libui_dir).map_err(Error::Ninja)
    }
}

mod meson {
    use std::{env, io, path::Path};

    #[derive(Debug)]
    pub enum Error {
        SetupLibui(io::Error),
    }

    pub fn setup_libui(meson_dir: &Path, libui_dir: &Path) -> Result<(), Error> {
        static LIBRARY_KIND: &str = if cfg!(feature = "static-libui") {
            "static"
        } else {
            "shared"
        };

        std::process::Command::new("python")
            .arg(meson_dir.join("meson.py"))
            .arg("setup")
            .arg(format!("--default-library={}", LIBRARY_KIND))
            .arg(format!("--buildtype={}", env::var("PROFILE").unwrap()))
            .arg(libui_dir.join("build"))
            .output()
            .map(|_| ())
            .map_err(Error::SetupLibui)
    }
}

mod ninja {
    use std::{io, path::Path};

    #[derive(Debug)]
    pub enum Error {
        Build(io::Error),
        BuildLibui(io::Error),
    }

    pub fn build(ninja_dir: &Path) -> Result<(), Error> {
        if ninja_dir.join("ninja").exists() {
            return Ok(());
        }

        std::process::Command::new("python")
            .arg("configure.py")
            .arg("--bootstrap")
            .current_dir(ninja_dir)
            .output()
            .map(|_| ())
            .map_err(Error::Build)
    }

    pub fn build_libui(ninja_dir: &Path, libui_dir: &Path) -> Result<(), Error> {
        std::process::Command::new(ninja_dir.join("ninja"))
            .arg("-C")
            .arg(libui_dir.join("build"))
            .output()
            .map(|_| ())
            .map_err(Error::BuildLibui)
    }
}

mod bindings {
    use std::{fmt, io, path::Path};

    #[derive(Debug)]
    pub enum Error {
        Generate,
        WriteToFile(io::Error),
    }

    pub fn gen(libui_dir: &Path, out_dir: &Path) -> Result<(), Error> {
        for wrapper in WRAPPERS {
            gen_wrapper(out_dir, libui_dir, wrapper)?;
        }

        Ok(())
    }

    static WRAPPERS: &[WrapperHeader] = &[
        WrapperHeader::Main,
        #[cfg(feature = "darwin-ext")]
        WrapperHeader::Ext {
            name: "darwin",
            dep: "Cocoa/Cocoa.h",
        },
        #[cfg(feature = "unix-ext")]
        WrapperHeader::Ext {
            name: "unix",
            dep: "gtk/gtk.h",
        },
        #[cfg(feature = "windows-ext")]
        WrapperHeader::Ext {
            name: "windows",
            dep: "windows.h",
        },
    ];

    pub enum WrapperHeader {
        Main,
        Ext {
            name: &'static str,
            dep: &'static str,
        },
    }

    fn gen_wrapper(
        out_dir: &Path,
        libui_dir: &Path,
        wrapper: &WrapperHeader,
    ) -> Result<(), Error> {
        let header_contents = wrapper.contents(libui_dir);
        let mut builder = builder::create(&header_contents);
        builder = builder::with_clang_args(builder);

        if matches!(wrapper, WrapperHeader::Ext { .. }) {
            builder = builder.blocklist_file(".*ui\\.h");
        }

        builder::consume(builder, wrapper, out_dir)
    }

    impl WrapperHeader {
        fn contents(&self, libui_dir: &Path) -> String {
            self
                .as_include_stmts(libui_dir)
                .into_iter()
                .map(|stmt| stmt.to_string())
                .collect::<Vec<String>>()
                .join("\n")
        }

        fn as_include_stmts(&self, libui_dir: &Path) -> Vec<IncludeStmt> {
            let mut stmts = vec![
                IncludeStmt {
                    kind: IncludeStmtKind::Local,
                    arg: libui_dir.join(format!("ui.h")).display().to_string(),
                }
            ];

            if let WrapperHeader::Ext { name, dep } = *self {
                stmts.push(IncludeStmt {
                    kind: IncludeStmtKind::System,
                    arg: dep.to_string(),
                });
                stmts.push(IncludeStmt {
                    kind: IncludeStmtKind::Local,
                    arg: libui_dir.join(format!("ui_{}.h", name)).display().to_string(),
                });
            }

            stmts
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

    impl fmt::Display for IncludeStmt {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(
                f,
                "#include {}",
                match self.kind {
                    IncludeStmtKind::System => format!("<{}>", self.arg),
                    IncludeStmtKind::Local => format!("\"{}\"", self.arg),
                },
            )
        }
    }

    mod builder {
        use std::path::Path;
        use super::{Error, WrapperHeader};

        pub fn create(header_contents: &str) -> bindgen::Builder {
            static LIBUI_REGEX: &str = "ui(?:[A-Z][a-z0-9]*)*";

            bindgen::builder()
                .header_contents("wrapper.h", &header_contents)
                .parse_callbacks(Box::new(bindgen::CargoCallbacks))
                .allowlist_function(LIBUI_REGEX)
                .allowlist_type(LIBUI_REGEX)
                .allowlist_var(LIBUI_REGEX)
        }

        pub fn with_clang_args(mut builder: bindgen::Builder) -> bindgen::Builder {
            #[cfg(feature = "unix-ext")]
            {
                builder = with_unix_clang_args(builder);
            }

            builder
        }

        fn with_unix_clang_args(builder: bindgen::Builder) -> bindgen::Builder {
            let gtk = pkg_config::Config::new()
                .atleast_version("3.10")
                .print_system_cflags(true)
                .print_system_libs(true)
                .probe("gtk+-3.0")
                .unwrap();

            with_clang_args_for_pkg(builder, gtk)
        }

        fn with_clang_args_for_pkg(
            builder: bindgen::Builder,
            pkg: pkg_config::Library,
        ) -> bindgen::Builder {
            let defines = pkg
                .defines
                .into_iter()
                .flat_map(|(k, v)| {
                    vec![
                        "-D".to_string(),
                        format!("{}{}", k, v.map(|it| format!("={}", it)).unwrap_or_default()),
                    ]
                });

            let includes = pkg
                .include_paths
                .into_iter()
                .flat_map(|path| {
                    vec![
                        "-I".to_string(),
                        path.display().to_string(),
                    ]
                });

            for path in pkg.link_paths {
                println!("cargo:rustc-link-search={}", path.display());
            }

            for lib in pkg.libs {
                println!("cargo:rustc-link-lib={}", lib);
            }

            builder
                .clang_args(defines)
                .clang_args(includes)
        }

        pub fn consume(
            builder: bindgen::Builder,
            wrapper: &WrapperHeader,
            out_dir: &Path,
        ) -> Result<(), Error> {
            builder
                .generate()
                .map_err(|_| Error::Generate)?
                .write_to_file(match wrapper {
                    WrapperHeader::Main => {
                        out_dir.join("bindings.rs")
                    }
                    WrapperHeader::Ext { name, .. } => {
                        out_dir.join(format!("bindings-{}.rs", name))
                    }
                })
                .map_err(Error::WriteToFile)
        }
    }
}
