use std::collections::HashMap;

use hmac::{Hmac, Mac};
use sha2::Sha256;

#[derive(Debug)]
#[repr(C)]
pub enum VerifyError {
    Success = 0,
    InvalidLength,
    MacError,
    EmptySlice,
}

#[no_mangle]
pub extern "C" fn verify(
    secret: *const u8,
    secret_len: u32,
    signature: *const u8,
    signature_len: u32,
    payload: *const u8,
    payload_len: u32,
    hashmap_serialized: *const u8,
    hashmap_serialized_len: u32,
) -> i32 {
    if secret.is_null()
        || secret_len == 0
        || signature.is_null()
        || signature_len == 0
        || payload.is_null()
        || payload_len == 0
        || hashmap_serialized.is_null()
        || hashmap_serialized_len == 0
    {
        return VerifyError::EmptySlice as i32;
    }

    let secret_slice: &[u8] = unsafe { std::slice::from_raw_parts(secret, secret_len as usize) };
    let signature_slice: &[u8] =
        unsafe { std::slice::from_raw_parts(signature, signature_len as usize) };
    let payload_slice: &[u8] = unsafe { std::slice::from_raw_parts(payload, payload_len as usize) };

    let hashmap_slice: &[u8] =
        unsafe { std::slice::from_raw_parts(hashmap_serialized, hashmap_serialized_len as usize) };

    let hashmap: HashMap<&str, &str> = postcard::from_bytes(hashmap_slice).unwrap();
    dbg!(hashmap);

    let result = verify_intern(secret_slice, signature_slice, payload_slice);

    match result {
        Ok(_) => 0,
        Err(err) => err as i32,
    }
}

#[inline(always)]
fn verify_intern(secret: &[u8], signature: &[u8], payload: &[u8]) -> Result<(), VerifyError> {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).map_err(|_| VerifyError::InvalidLength)?;
    mac.update(payload);

    mac.verify_slice(signature)
        .map_err(|_| VerifyError::MacError)
}

/// Allocate memory into the module's linear memory
/// and return the offset to the start of the block.
#[no_mangle]
pub fn alloc(len: usize) -> *mut u8 {
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
pub unsafe fn dealloc(ptr: *mut u8, size: usize) {
    // ! Copied from https://radu-matei.com/blog/practical-guide-to-wasm-memory/#passing-arrays-to-rust-webassembly-modules
    let data = Vec::from_raw_parts(ptr, size, size);

    std::mem::drop(data);
}
