use anyhow::Result;
use hmac::{Hmac, Mac};
use sha2::Sha256;

pub fn verify(secret: &[u8], signature: &[u8], payload: &[u8]) -> Result<()> {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret)?;
    mac.update(payload);

    mac.verify_slice(signature)?;

    Ok(())
}
