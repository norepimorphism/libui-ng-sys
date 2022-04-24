fn main() {
    use libui_ng_sys::*;

    unsafe {
        let mut options = uiInitOptions { Size: 0 };
        uiInit(std::ptr::addr_of_mut!(options));
        let name = std::ffi::CString::new("libui-ng-sys").unwrap();
        let window = uiNewWindow(name.as_ptr(), 640, 480, 0);
        uiControlShow(window.cast());
        uiMain();
    }
}
