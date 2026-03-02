//! HMAC-SHA1 and HMAC-SHA256 convenience wrappers.

use hmac::{Hmac, Mac};
use sha1::Sha1;
use sha2::Sha256;

/// HMAC-SHA1 wrapper (20-byte output).
pub struct HmacSha1 {
    mac: Hmac<Sha1>,
}

impl HmacSha1 {
    /// Create a new HMAC-SHA1 instance with the given `key`.
    pub fn new(key: &[u8]) -> Self {
        Self {
            mac: Hmac::<Sha1>::new_from_slice(key)
                .expect("HMAC-SHA1 accepts any key length"),
        }
    }

    /// Feed `data` into the HMAC.
    pub fn update(&mut self, data: &[u8]) {
        self.mac.update(data);
    }

    /// Finalize and return the 20-byte HMAC digest.
    pub fn finalize(self) -> [u8; 20] {
        let result = self.mac.finalize();
        result.into_bytes().into()
    }

    /// One-shot: compute HMAC-SHA1(key, data).
    pub fn digest(key: &[u8], data: &[u8]) -> [u8; 20] {
        let mut h = Self::new(key);
        h.update(data);
        h.finalize()
    }
}

/// HMAC-SHA256 wrapper (32-byte output).
pub struct HmacSha256 {
    mac: Hmac<Sha256>,
}

impl HmacSha256 {
    /// Create a new HMAC-SHA256 instance with the given `key`.
    pub fn new(key: &[u8]) -> Self {
        Self {
            mac: Hmac::<Sha256>::new_from_slice(key)
                .expect("HMAC-SHA256 accepts any key length"),
        }
    }

    /// Feed `data` into the HMAC.
    pub fn update(&mut self, data: &[u8]) {
        self.mac.update(data);
    }

    /// Finalize and return the 32-byte HMAC digest.
    pub fn finalize(self) -> [u8; 32] {
        let result = self.mac.finalize();
        result.into_bytes().into()
    }

    /// One-shot: compute HMAC-SHA256(key, data).
    pub fn digest(key: &[u8], data: &[u8]) -> [u8; 32] {
        let mut h = Self::new(key);
        h.update(data);
        h.finalize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmac_sha1_known_vector() {
        // RFC 2202 test case 1
        let key = [0x0b_u8; 20];
        let data = b"Hi There";
        let result = HmacSha1::digest(&key, data);
        let expected = hex_to_bytes("b617318655057264e28bc0b6fb378c8ef146be00");
        assert_eq!(&result[..], &expected[..]);
    }

    #[test]
    fn hmac_sha256_known_vector() {
        // RFC 4231 test case 1
        let key = [0x0b_u8; 20];
        let data = b"Hi There";
        let result = HmacSha256::digest(&key, data);
        let expected = hex_to_bytes(
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7",
        );
        assert_eq!(&result[..], &expected[..]);
    }

    fn hex_to_bytes(s: &str) -> Vec<u8> {
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
            .collect()
    }
}
