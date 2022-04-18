# libui-ng-sys

Rust bindings for [libui-ng](https://github.com/libui-ng/libui-ng).

## Prerequisites

See [libui-ng build requirements](https://github.com/libui-ng/libui-ng#build-requirements) for more information.

* [Meson](https://mesonbuild.com/) &ge; v0.48.0
* [Ninja](https://ninja-build.org/) &ge; v1.8.2

If a platform-specific API feature is enabled, the following are also required:

* [pkg-config](https://www.freedesktop.org/wiki/Software/pkg-config/)

See the below sections for additional feature-specific prerequisites.

### darwin-ext

### unix-ext

* [GTK3](https://gtk.org) &ge; v3.10

### windows-ext

## Version Mapping

libui-ng-sys does not necessarily pull the most recent version of libui-ng; rather, each libui-ng-sys version is tied to a specific commit of libui-ng.

| libui-ng-sys Version | libui-ng Commit Hash                     |
| -------------------- | ---------------------------------------- |
| 0.1.0                | 42641e3d6bfb2c49ca4cc3b03d8ae277d9841a5d |

## TODO

* Accept previously-fetched libui-ng repo
* Accept previously-compiled libui-ng
