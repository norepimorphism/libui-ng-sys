#![windows_subsystem = "windows"]

use libui_ng_sys::*;
use std::{ffi, os::raw::c_void, ptr};

fn main() {
    unsafe {
        let mut options = uiInitOptions { Size: 0 };
        uiInit(ptr::addr_of_mut!(options));

        let file_menu_name = ffi::CString::new("File").unwrap();
        let file_menu = uiNewMenu(file_menu_name.as_ptr());
        uiMenuAppendQuitItem(file_menu);

        let window_name = ffi::CString::new("libui-ng-sys").unwrap();
        let window = uiNewWindow(window_name.as_ptr(), 200, 40, 1);
        uiWindowSetResizeable(window, 0);
        uiWindowSetMargined(window, 1);
        uiWindowOnClosing(window, Some(window_on_closing), ptr::null_mut());
        uiOnShouldQuit(Some(on_ui_should_quit), window.cast());

        let button_text = ffi::CString::new("Lorem Ipsum").unwrap();
        let button = uiNewButton(button_text.as_ptr());

        let hbox = uiNewHorizontalBox();
        let vbox = uiNewVerticalBox();
        uiBoxSetPadded(hbox, 1);
        uiBoxSetPadded(vbox, 1);
        uiBoxAppend(vbox, button.cast(), 1);
        uiBoxAppend(hbox, vbox.cast(), 1);
        uiWindowSetChild(window, hbox.cast());

        uiControlShow(window.cast());
        uiMain();
    }
}

unsafe extern "C" fn window_on_closing(_: *mut uiWindow, _: *mut c_void) -> i32 {
    uiQuit();
    0
}

unsafe extern "C" fn on_ui_should_quit(window: *mut c_void) -> i32 {
    uiControlDestroy(window.cast());
    1
}
