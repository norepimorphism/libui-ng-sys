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

## TODO

* Accept previously-fetched libui-ng repo
* Accept previously-compiled libui-ng
