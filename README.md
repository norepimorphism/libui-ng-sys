# libui-ng-sys

[![crates.io](https://img.shields.io/crates/v/libui-ng-sys)](https://crates.io/crates/libui-ng-sys)
[![docs](https://docs.rs/libui-ng-sys/badge.svg)](https://docs.rs/libui-ng-sys)
[![MPL 2.0 licensed](https://img.shields.io/github/license/norepimorphism/libui-ng-sys)](./LICENSE)

Rust bindings to [*libui-ng*](https://github.com/libui-ng/libui-ng).

## Features

### `build`

This feature is enabled by default. When `build` is enabled, the *libui-ng-sys* build script automatically fetches, compiles, and statically links *libui-ng* to the final build product. Otherwise, when `build` is disabled, the system *libui-ng* is linked dynamically.

This feature may require external dependencies that cannot be automatically fetched by *libui-ng-sys*. The following sections list requirements for different values of `$CARGO_CFG_TARGET_OS`.

#### All

* [Python](https://www.python.org/) &ge; v3.4
    * Used to run [Meson](https://mesonbuild.com/).

#### `linux`

* [GTK3](https://gtk.org) &ge; v3.10.0
    * Note: GTK4 is not currently supported; this is a limitation of *libui-ng*.
* [pkg-config](https://www.freedesktop.org/wiki/Software/pkg-config/)
    * Used to detect GTK dependency libraries and include search paths.

#### `windows`

* Windows 10 or 11 SDK

These may be acquired from the [Visual Studio Installer](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022).

#### Other

Other targets are not currently supported.

### `build-with-ninja`

This feature is enabled by default and implies `build`. When `build-with-ninja` is enabled, *libui-ng* is built with [Ninja](https://ninja-build.org/).

#### `windows`

Building with Ninja requires that certain compiler components are included in your `$PATH`. In this case, it may be best to build *libui-ng-sys* while in a Developer Command Prompt.

### `build-ninja`

This feature is enabled by default and implies `build-with-ninja`. When `build-ninja` is enabled, Ninja is automatically fetched, compiled, and used to build *libui-ng*.

### `build-with-msvc`

This feature implies `build`. When `build-with-msvc` is enabled, *libui-ng* is built with the system MSVC.

#### `windows`

The following [Visual Studio](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022) components are required:

* MSVC C++ x64/x86 build tools
* C++ Clang Compiler for Windows

### `build-with-xcode`

This feature implies `build`. When `build-with-xcode` is enabled, *libui-ng* is built with Xcode.

#### `macos`

* Xcode

## Version Mapping

When the `build` feature is enabled, *libui-ng-sys* requires *libui-ng*, Meson, and possibly Ninja as dependencies, but Rust crates do not exist for them yet. To work around this, these are included as Git submodules. The latest commit hashes of the submodules are documented below for each version of *libui-ng-sys*.

### 0.2.0&ndash;0.4.2

| Dependency | Commit Hash                                |
| ---------- | ------------------------------------------ |
| *libui-ng* | `42641e3d6bfb2c49ca4cc3b03d8ae277d9841a5d` |
| Meson      | `09ad4e28f1a59ab3d87de6f36540a108e836cfe5` |
| Ninja      | `25cdbae0ee1270a5c8dd6ba67696e29ad8076919` |

### 0.1.0

| Dependency | Commit Hash                                |
| ---------- | ------------------------------------------ |
| *libui-ng* | `42641e3d6bfb2c49ca4cc3b03d8ae277d9841a5d` |
