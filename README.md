# libui-ng-sys

Rust bindings for [libui-ng](https://github.com/libui-ng/libui-ng).

## Prerequisites

* [Python](https://www.python.org/)

If a platform-specific API feature is enabled, the following are also required:

* [pkg-config](https://www.freedesktop.org/wiki/Software/pkg-config/)

See the below sections for additional feature-specific prerequisites.

### darwin-ext

### unix-ext

* [GTK3](https://gtk.org) &ge; v3.10

### windows-ext

## Version Mapping

libui-ng-sys requires libui-ng, [Meson](https://github.com/mesonbuild/meson), and [Ninja](https://github.com/ninja-build/ninja) as build dependencies, but Rust crates do not exist for them yet. To work around this, these dependencies are included as Git submodules, and Ninja in particular&mdash;being a C++ project&mdash;is automatically built in the libui-ng-sys build script. The latest commit hashes of the submodules are documented below for each version of libui-ng-sys.

### 0.2.0&ndash;0.2.1

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
