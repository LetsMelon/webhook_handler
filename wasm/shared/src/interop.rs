use anyhow::Result;
use serde::{Deserialize, Serialize};

#[inline]
pub fn deserialize<'a, T: Deserialize<'a>>(raw: &'a [u8]) -> Result<T> {
    Ok(postcard::from_bytes(raw)?)
}

#[inline]
pub fn serialize<T: Serialize + ?Sized>(value: &T) -> Result<Vec<u8>> {
    Ok(postcard::to_allocvec(value)?)
}
