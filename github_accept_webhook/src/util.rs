use crate::err_no::{err_clear, set_err_msg_str, set_err_no};

#[inline]
pub fn get_slice_from_ptr_and_len_safe<'a, T>(ptr: *const T, len: u32) -> Result<&'a [T], ()> {
    err_clear();

    if ptr.is_null() {
        set_err_no(-1);
        set_err_msg_str("get_slice_from_ptr_and_len_safe: ptr is not allowed to be null");

        return Err(());
    }

    if len == 0 {
        set_err_no(-1);
        set_err_msg_str(
            "get_slice_from_ptr_and_len_safe: the len of the slice if not allowed to be 0",
        );

        return Err(());
    }

    Ok(unsafe { std::slice::from_raw_parts(ptr, len as usize) })
}
