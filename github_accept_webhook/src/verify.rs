use std::collections::HashMap;
use std::fmt::Debug;

use hmac::{Hmac, Mac};
use sha2::Sha256;

use crate::util::get_slice_from_ptr_and_len_safe;

#[derive(Debug)]
#[repr(C)]
pub enum VerifyError {
    InvalidLength = 1,
    MacError,
    EmptySlice,
    NoSignature,
}

#[no_mangle]
pub extern "C" fn verify(
    secret: *const u8,
    secret_len: u32,
    payload: *const u8,
    payload_len: u32,
    hashmap_serialized: *const u8,
    hashmap_serialized_len: u32,
) -> i32 {
    let secret_slice = get_slice_from_ptr_and_len_safe(secret, secret_len).unwrap();
    let payload_slice = get_slice_from_ptr_and_len_safe(payload, payload_len).unwrap();
    let hashmap_slice =
        get_slice_from_ptr_and_len_safe(hashmap_serialized, hashmap_serialized_len).unwrap();

    let hashmap: HashMap<&str, &str> = postcard::from_bytes(hashmap_slice).unwrap();

    let value = hashmap
        .get("x-hub-signature-256")
        .ok_or(())
        .cloned()
        .map(|item| item.strip_prefix("sha256=").map(|item| item.to_string()));
    let Ok(Some(signature)) = value else {
        return VerifyError::NoSignature as i32;
    };

    dbg!(&signature);

    let result = verify_intern(
        secret_slice,
        &hex::decode(signature).unwrap(),
        payload_slice,
    );

    match result {
        Ok(_) => 0,
        Err(err) => err as i32,
    }
}

fn verify_intern(secret: &[u8], signature: &[u8], payload: &[u8]) -> Result<(), VerifyError> {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret).map_err(|_| VerifyError::InvalidLength)?;
    mac.update(payload);

    mac.verify_slice(signature)
        .map_err(|_| VerifyError::MacError)
}
