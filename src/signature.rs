use crate::*;

use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub fn verify_signature(base64_url: &str, signature_hex: &str) -> bool {
    let Ok(signature) = hex::decode(signature_hex) else {
        return false;
    };
    let Ok(mut mac) = HmacSha256::new_from_slice(IMAGES_HMAC_SECRET.as_bytes()) else {
        return false;
    };
    mac.update(base64_url.as_bytes());
    mac.verify_slice(&signature).is_ok()
}
