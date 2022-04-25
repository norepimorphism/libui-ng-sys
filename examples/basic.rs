#![windows_subsystem = "windows"]

use libui_ng_sys::*;
use std::{ffi, os::raw::c_void, ptr};

fn main() {
    unsafe {
        let mut options = uiInitOptions { Size: 0 };
        uiInit(ptr::addr_of_mut!(options));
        let name = ffi::CString::new("libui-ng-sys").unwrap();
        let window = uiNewWindow(name.as_ptr(), 640, 480, 0);
        uiWindowOnClosing(window, Some(close_window), ptr::null_mut());
        uiOnShouldQuit(Some(quit_ui), window.cast());
        uiControlShow(window.cast());
        uiMain();
    }
}

unsafe extern "C" fn close_window(_: *mut uiWindow, _: *mut c_void) -> i32 {
    uiQuit();
    0
}

unsafe extern "C" fn quit_ui(window: *mut c_void) -> i32 {
    uiControlDestroy(window.cast());
    1
}
