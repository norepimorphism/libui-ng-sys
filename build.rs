// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::{env, io, path::{Path, PathBuf}};

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

    let repo = clone_libui(libui_dir.as_path()).map_err(Error::Git)?;
    update_libui(&repo).map_err(Error::Git)?;
    build_libui(libui_dir.as_path())?;
    gen_bindings(out_dir.as_path(), libui_dir.as_path())?;

    println!(
        "cargo:rustc-link-search={}",
        libui_dir.join("build/meson-out/").display(),
    );
    println!("cargo:rustc-link-lib=static=ui");

    //#[cfg(feature = "unix-ext")]
    //#[cfg(target_family = "unix")]
    {
        println!("cargo:rustc-link-lib=glib");
        println!("cargo:rustc-link-lib=gtk");
    }

    println!("cargo:rerun-if-changed=build.rs");

    Ok(())
}

fn clone_libui(libui_dir: &Path) -> Result<git2::Repository, git2::Error> {
    static REPO_URL: &str = "https://github.com/libui-ng/libui-ng.git";

    match git2::Repository::clone_recurse(REPO_URL, libui_dir) {
        Ok(repo) => Ok(repo),
        Err(e) if git_error_is_already_exists(&e) => {
            git2::Repository::open(libui_dir)
        }
        Err(e) => Err(e),
    }
}

fn git_error_is_already_exists(e: &git2::Error) -> bool {
    (e.code() == git2::ErrorCode::Exists) &&
    (e.class() == git2::ErrorClass::Invalid)
}

fn update_libui(repo: &git2::Repository) -> Result<(), git2::Error> {
    const HEAD: &str = "42641e3d6bfb2c49ca4cc3b03d8ae277d9841a5d";

    repo.set_head_detached(git2::Oid::from_str(HEAD).unwrap())?;
    repo.checkout_head(None)?;

    Ok(())
}

fn build_libui(libui_dir: &Path) -> Result<(), Error> {
    std::process::Command::new("meson")
        .args(["setup", "--default-library=static"])
        .arg(format!("--buildtype={}", env::var("PROFILE").unwrap()))
        .arg("build")
        .current_dir(libui_dir)
        .output()
        .map(|_| ())
        .map_err(Error::Meson)?;

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

fn gen_bindings_for_wrapper(
    out_dir: &Path,
    libui_dir: &Path,
    wrapper: &WrapperHeader,
) -> Result<(), Error> {
    static LIBUI_REGEX: &str = "ui(?:[A-Z][a-z0-9]*)*";

    let mut header_contents = format!(
        "#include \"{}\"\n",
        libui_dir.join(format!("ui.h")).display(),
    );

    if let WrapperHeader::Ext { name, dep } = wrapper {
        header_contents.push_str(format!("#include <{}>\n", dep).as_str());
        header_contents.push_str(
            format!(
                "#include \"{}\"\n",
                libui_dir.join(format!("ui_{}.h", name)).display(),
            )
            .as_str()
        );
    }

    let mut builder = bindgen::builder()
        .header_contents("wrapper.h", header_contents.as_str())
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .allowlist_function(LIBUI_REGEX)
        .allowlist_type(LIBUI_REGEX)
        .allowlist_var(LIBUI_REGEX);

    #[cfg(feature = "unix-ext")]
    #[cfg(target_family = "unix")]
    {
        builder = builder
            .clang_args(["-I", "/usr/include/atk-1.0"])
            .clang_args(["-I", "/usr/include/cairo"])
            .clang_args(["-I", "/usr/include/gdk-pixbuf-2.0"])
            .clang_args(["-I", "/usr/include/glib-2.0"])
            .clang_args(["-I", "/usr/lib/glib-2.0/include"])
            .clang_args(["-I", "/usr/include/graphene-1.0"])
            .clang_args(["-I", "/usr/lib/graphene-1.0/include"])
            .clang_args(["-I", "/usr/include/gtk-3.0"])
            .clang_args(["-I", "/usr/include/harfbuzz"])
            .clang_args(["-I", "/usr/include/pango-1.0"]);
    }

    if matches!(wrapper, WrapperHeader::Ext { .. }) {
        builder = builder.blocklist_file(".*ui\\.h");
    }

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
