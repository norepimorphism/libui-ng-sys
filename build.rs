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
    let bindings_path = out_dir.join("bindings.rs");

    let repo = clone_libui(libui_dir.as_path()).map_err(Error::Git)?;
    update_libui(&repo).map_err(Error::Git)?;
    build_libui(libui_dir.as_path())?;

    for header_names in [
        vec!["\"ui.h\""],
        #[cfg(target_os = "macos")]
        vec!["<Cocoa/Cocoa.h>", "\"ui_darwin.h\""],
        #[cfg(target_os = "linux")]
        vec!["<gtk/gtk.h>", "\"ui_unix.h\""],
        #[cfg(target_os = "windows")]
        vec!["<windows.h>", "\"ui_windows.h\""],
    ] {
        gen_bindings(libui_dir.as_path(), bindings_path.as_path(), header_names.as_slice())?;
    }

    println!(
        "cargo:rustc-link-search={}",
        libui_dir.join("build/meson-out/").display(),
    );
    println!("cargo:rustc-link-lib=static=ui");
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

fn gen_bindings(
    libui_dir: &Path,
    bindings_path: &Path,
    header_names: &[&str],
) -> Result<(), Error> {
    static LIBUI_REGEX: &str = "ui(?:[A-Z][a-z]*)*";

    bindgen::builder()
        .header_contents(
            "wrapper.h",
            header_names
                .iter()
                .map(|name| {
                    format!("#include {}", libui_dir.join(header_name).display()).as_str(),
                })
                .collect::<Vec<String>>()
                .join('\n'),
        )
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .allowlist_function(LIBUI_REGEX)
        .allowlist_type(LIBUI_REGEX)
        .allowlist_var(LIBUI_REGEX)
        .generate()
        .unwrap()
        .write_to_file(bindings_path)
        .map_err(|_| Error::Bindgen)
}
