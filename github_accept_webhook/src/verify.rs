use anyhow::Result;
use hmac::{Hmac, Mac};
use sha2::Sha256;

pub fn verify(secret: &[u8], signature: &[u8], payload: &[u8]) -> Result<()> {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret)?;
    mac.update(payload);

    mac.verify_slice(signature)?;

    Ok(())
}

#[test]
fn github_demo() {
    // https://docs.github.com/en/webhooks/using-webhooks/validating-webhook-deliveries

    verify(
        b"It's a Secret to Everybody",
        &hex::decode("757107ea0eb2509fc211221cce984b8a37570b6d7586c22c46f4379c8b043e17").unwrap(),
        b"Hello, World!",
    )
    .unwrap();
}
