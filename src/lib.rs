// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

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

pub mod platform {
    macro_rules! def_platform {
        ($mod:tt, $header:literal, $os:literal $(,)?) => {
            #[cfg(target_os = $os)]
            pub mod $mod {
                use crate::*;

                include_bindings!($header);
            }
        };
    }

    def_platform!(darwin, "bindings-darwin", "macos");
    def_platform!(unix, "bindings-unix", "linux");
    def_platform!(windows, "bindings-windows", "windows");
}
