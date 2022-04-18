// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{env, fmt, io, path::{Path, PathBuf}};

#[derive(Debug)]
enum Error {
    Bindgen,
    Git(git2::Error),
    Meson(io::Error),
    Ninja(io::Error),
}

fn main() -> Result<(), Error> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let libui_dir = out_dir.join("libui-ng");

    let repo = clone_libui(&libui_dir)?;
    update_libui(&repo)?;
    setup_libui(&libui_dir)?;
    build_libui(&libui_dir)?;
    gen_bindings(&out_dir, &libui_dir)?;

    println!(
        "cargo:rustc-link-search={}",
        libui_dir.join("build/meson-out/").display(),
    );
    println!("cargo:rustc-link-lib=ui");

    println!("cargo:rerun-if-changed=build.rs");

    Ok(())
}

fn clone_libui(libui_dir: &Path) -> Result<git2::Repository, Error> {
    static REPO_URL: &str = "https://github.com/libui-ng/libui-ng.git";

    match git2::Repository::clone_recurse(REPO_URL, libui_dir) {
        Ok(repo) => Ok(repo),
        Err(e) if git_error_is_already_exists(&e) => {
            git2::Repository::open(libui_dir)
        }
        Err(e) => Err(e),
    }
    .map_err(Error::Git)
}

fn git_error_is_already_exists(e: &git2::Error) -> bool {
    (e.code() == git2::ErrorCode::Exists) &&
    (e.class() == git2::ErrorClass::Invalid)
}

fn update_libui(repo: &git2::Repository) -> Result<(), Error> {
    const HEAD: &str = "42641e3d6bfb2c49ca4cc3b03d8ae277d9841a5d";

    repo.set_head_detached(git2::Oid::from_str(HEAD).unwrap()).map_err(Error::Git)?;
    repo.checkout_head(None).map_err(Error::Git)
}

fn setup_libui(libui_dir: &Path) -> Result<(), Error> {
    static LIBRARY_KIND: &str = if cfg!(feature = "static-libui") {
        "static"
    } else {
        "shared"
    };

    std::process::Command::new("meson")
        .arg("setup")
        .arg(format!("--default-library={}", LIBRARY_KIND))
        .arg(format!("--buildtype={}", env::var("PROFILE").unwrap()))
        .arg("build")
        .current_dir(libui_dir)
        .output()
        .map(|_| ())
        .map_err(Error::Meson)
}

fn build_libui(libui_dir: &Path) -> Result<(), Error> {
    std::process::Command::new("ninja")
        .args(["-C", "build"])
        .current_dir(libui_dir)
        .output()
        .map(|_| ())
        .map_err(Error::Ninja)
}

fn gen_bindings(out_dir: &Path, libui_dir: &Path) -> Result<(), Error> {
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

    for wrapper in WRAPPERS {
        gen_bindings_for_wrapper(out_dir, libui_dir, wrapper)?;
    }

    Ok(())
}

enum WrapperHeader {
    Main,
    Ext {
        name: &'static str,
        dep: &'static str,
    },
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

fn gen_bindings_for_wrapper(
    out_dir: &Path,
    libui_dir: &Path,
    wrapper: &WrapperHeader,
) -> Result<(), Error> {
    let header_contents = wrapper.contents(libui_dir);
    let mut builder = create_bindgen_builder(&header_contents);
    builder = bindgen_builder_with_clang_args(builder);

    if matches!(wrapper, WrapperHeader::Ext { .. }) {
        builder = builder.blocklist_file(".*ui\\.h");
    }

    consume_bindgen_builder(builder, wrapper, out_dir)
}

fn create_bindgen_builder(header_contents: &str) -> bindgen::Builder {
    static LIBUI_REGEX: &str = "ui(?:[A-Z][a-z0-9]*)*";

    bindgen::builder()
        .header_contents("wrapper.h", &header_contents)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .allowlist_function(LIBUI_REGEX)
        .allowlist_type(LIBUI_REGEX)
        .allowlist_var(LIBUI_REGEX)
}

fn bindgen_builder_with_clang_args(mut builder: bindgen::Builder) -> bindgen::Builder {
    #[cfg(feature = "unix-ext")]
    {
        builder = bindgen_builder_with_unix_clang_args(builder);
    }

    builder
}

fn bindgen_builder_with_unix_clang_args(builder: bindgen::Builder) -> bindgen::Builder {
    let gtk = pkg_config::Config::new()
        .atleast_version("3.10")
        .probe("gtk+-3.0")
        .unwrap();

    bindgen_builder_with_clang_args_for_pkg(builder, gtk)
}

fn bindgen_builder_with_clang_args_for_pkg(
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

fn consume_bindgen_builder(
    builder: bindgen::Builder,
    wrapper: &WrapperHeader,
    out_dir: &Path,
) -> Result<(), Error> {
    builder
        .generate()
        .unwrap()
        .write_to_file(match wrapper {
            WrapperHeader::Main => {
                out_dir.join("bindings.rs")
            }
            WrapperHeader::Ext { name, .. } => {
                out_dir.join(format!("bindings-{}.rs", name))
            }
        })
        .map_err(|_| Error::Bindgen)
}
