use anyhow::Result;
use serde::Deserialize;

pub fn deserialize<'a, T: Deserialize<'a>>(raw: &'a [u8]) -> Result<T> {
    Ok(postcard::from_bytes(raw)?)
}
