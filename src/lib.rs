// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#![allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
)]

macro_rules! include_bindings {
    () => {
        include_bindings!("", "");
    };
    ($name:literal) => {
        include_bindings!("-", $name);
    };
    ($sep:literal, $name:literal) => {
        include!(concat!(env!("OUT_DIR"), "/bindings", $sep, $name, ".rs"));
    };
}

include_bindings!();

pub mod platform {
    macro_rules! def_platform {
        ($mod_name:tt, $header_name:literal, $feature_name:literal $(,)?) => {
            #[cfg(feature = $feature_name)]
            pub mod $mod_name {
                use crate::*;

                include_bindings!($header_name);
            }
        };
    }

    def_platform!(darwin, "darwin", "darwin-ext");
    def_platform!(unix, "unix", "unix-ext");
    def_platform!(windows, "windows", "windows-ext");
}
