use std::borrow::Borrow;
use std::cell::RefCell;
use std::ffi::{CStr, CString};

use shared::constants::MAX_ERR_MSG_LEN;

thread_local! {
    static ERR_NO: RefCell<i32> = RefCell::new(0);
    static ERR_MSG: RefCell<[u8; 1024]> = RefCell::new([0; MAX_ERR_MSG_LEN]);
}

#[no_mangle]
pub extern "C" fn err_clear() {
    set_err_no(0);

    ERR_MSG.with_borrow_mut(|item| *item = [0; MAX_ERR_MSG_LEN]);
}

#[no_mangle]
pub extern "C" fn set_err_no(err: i32) {
    ERR_NO.set(err);
}

#[no_mangle]
pub extern "C" fn get_err_no() -> i32 {
    ERR_NO.borrow().take()
}

#[no_mangle]
pub extern "C" fn set_err_msg(msg: *const i8) {
    let c_string = unsafe { CStr::from_ptr(msg) };

    ERR_MSG.with_borrow_mut(|item| {
        let bytes = c_string.to_bytes_with_nul();
        let len = bytes.len().min(item.len() - 1);

        item[..len].copy_from_slice(&bytes[..len]);
        item[item.len() - 1] = b'\0';
    });
}

pub fn set_err_msg_str(msg: &str) {
    let cstring = CString::new(msg).unwrap();

    set_err_msg(cstring.as_ptr());
}

#[no_mangle]
pub extern "C" fn get_err_msg() -> *const u8 {
    ERR_MSG.with(|err_msg| {
        let borrowed_msg = err_msg.borrow();
        borrowed_msg.as_ptr()
    })
}
