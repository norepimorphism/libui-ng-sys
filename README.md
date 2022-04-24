# libui-ng-sys

![Crates.io](https://img.shields.io/crates/v/libui-ng-sys)

Rust bindings for [libui-ng](https://github.com/libui-ng/libui-ng).

## Prerequisites

If the `build` feature is enabled, external dependencies may be required. The following sections list requirements for different values of `$CARGO_CFG_TARGET_OS`.

### Linux

* [GTK3](https://gtk.org) &ge; v3.10.0
* [pkg-config](https://www.freedesktop.org/wiki/Software/pkg-config/)
* [Python](https://www.python.org/) &ge; v3.4

### Windows

The following Visual Studio components are required:

* MSVC C++ x64/x86 build tools
* C++ Clang Compiler for Windows
* Windows 10 or 11 SDK

These may be acquired from the [Visual Studio Installer](https://visualstudio.microsoft.com/downloads/#build-tools-for-visual-studio-2022).

### Other

Other targets are not currently supported.

## Version Mapping

When the `build` feature is enabled, libui-ng-sys requires libui-ng, [Meson](https://github.com/mesonbuild/meson), and possibly [Ninja](https://github.com/ninja-build/ninja) as dependencies, but Rust crates do not exist for them yet. To work around this, these are included as Git submodules, and Ninja in particular&mdash;being a C++ project&mdash;is automatically built in the libui-ng-sys build script. The latest commit hashes of the submodules are documented below for each version of libui-ng-sys.

### 0.2.0&ndash;0.2.2

| Dependency | Commit Hash                              |
| ---------- | ---------------------------------------- |
| libui-ng   | 42641e3d6bfb2c49ca4cc3b03d8ae277d9841a5d |
| Meson      | 09ad4e28f1a59ab3d87de6f36540a108e836cfe5 |
| Ninja      | 25cdbae0ee1270a5c8dd6ba67696e29ad8076919 |

### 0.1.0

| Dependency | Commit Hash                              |
| ---------- | ---------------------------------------- |
| libui-ng   | 42641e3d6bfb2c49ca4cc3b03d8ae277d9841a5d |

## TODO

* Accept previously-fetched libui-ng repo
* Accept previously-compiled libui-ng
