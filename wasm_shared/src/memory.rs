use std::fmt::Debug;
use std::ops::Deref;

use crate::err_no::{err_clear, set_err_msg_str, set_err_no};

/// Allocate memory into the module's linear memory
/// and return the offset to the start of the block.
#[no_mangle]
pub extern "C" fn alloc(len: usize) -> *mut u8 {
    // ! Copied from https://radu-matei.com/blog/practical-guide-to-wasm-memory/#passing-arrays-to-rust-webassembly-modules

    // create a new mutable buffer with capacity `len`
    let mut buf = Vec::with_capacity(len);
    // take a mutable pointer to the buffer
    let ptr = buf.as_mut_ptr();
    // take ownership of the memory block and
    // ensure that its destructor is not
    // called when the object goes out of scope
    // at the end of the function
    std::mem::forget(buf);
    // return the pointer so the runtime
    // can write data at this offset
    return ptr;
}

#[no_mangle]
pub extern "C" fn dealloc(ptr: *mut u8, size: usize) {
    // ! Copied from https://radu-matei.com/blog/practical-guide-to-wasm-memory/#passing-arrays-to-rust-webassembly-modules
    let data = unsafe { Vec::from_raw_parts(ptr, size, size) };

    std::mem::drop(data);
}

#[inline]
pub fn get_slice_from_ptr_and_len_safe<'a, T>(ptr: *const T, len: u32) -> Result<&'a [T], ()> {
    err_clear();

    if ptr.is_null() {
        set_err_no(-1);
        set_err_msg_str("get_slice_from_ptr_and_len_safe: ptr is not allowed to be null");

        return Err(());
    }

    if len == 0 {
        set_err_no(-2);
        set_err_msg_str(
            "get_slice_from_ptr_and_len_safe: the len of the slice if not allowed to be 0",
        );

        return Err(());
    }

    Ok(unsafe { std::slice::from_raw_parts(ptr, len as usize) })
}

pub struct WasmOwnedMemory<'a, T> {
    ptr: *const T,
    len: u32,
    data: &'a [T],
}

impl<'a, T> WasmOwnedMemory<'a, T> {
    pub fn get(ptr: *const T, len: u32) -> anyhow::Result<WasmOwnedMemory<'a, T>> {
        let slice = get_slice_from_ptr_and_len_safe(ptr, len)
            .map_err(|_| anyhow::anyhow!("Could not get data from memory"))?;

        Ok(WasmOwnedMemory {
            ptr,
            len,
            data: slice,
        })
    }
}

impl<'a, T> Deref for WasmOwnedMemory<'a, T> {
    type Target = &'a [T];

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<'a, T> AsRef<&'a [T]> for WasmOwnedMemory<'a, T> {
    fn as_ref(&self) -> &&'a [T] {
        &self.data
    }
}

// TODO maybe implement the trait `Borrow`
// impl<'a, T> Borrow<&'a [T]> for WasmOwnedMemory<'a, T> {
//     fn borrow(&self) -> &&'a [T] {
//         &self.data
//     }
// }

impl<T> Debug for WasmOwnedMemory<'_, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmOwnedMemory")
            .field("ptr", &self.ptr)
            .field("len", &self.len)
            .finish()
    }
}

impl<'a, T> Drop for WasmOwnedMemory<'a, T> {
    fn drop(&mut self) {
        // TODO is this safe?... or sound?
        dealloc(self.ptr as *mut u8, self.len as usize)
    }
}
