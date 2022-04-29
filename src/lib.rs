// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Bindings to *[libui-ng]*.
//!
//! [libui-ng]: https://github.com/libui-ng/libui-ng

#![allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
)]

macro_rules! include_bindings {
    ($name:literal) => {
        include!(concat!(env!("OUT_DIR"), "/", $name, ".rs"));
    };
}

include_bindings!("bindings");
include_bindings!("bindings-control-sigs");

/// Platform-specific functionality.
pub mod platform {
    macro_rules! def_platform {
        ($mod:tt, $platform:literal, $header:literal, $os:literal $(,)?) => {
            #[doc = concat!("Additional features available on ", $platform, " platforms.")]
            #[cfg(target_os = $os)]
            pub mod $mod {
                use crate::*;

                include_bindings!($header);
            }
        };
    }

    def_platform!(darwin, "Darwin", "bindings-darwin", "macos");
    def_platform!(unix, "Unix", "bindings-unix", "linux");
    def_platform!(windows, "Windows", "bindings-windows", "windows");
}
