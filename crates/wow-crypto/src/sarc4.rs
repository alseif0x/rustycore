//! RC4 (ARC4 / SARC4) stream cipher used by WoW for legacy packet header
//! encryption (pre-AES-GCM era, still present in 3.4.3 login flow).

/// Standard RC4 stream cipher state.
pub struct SArc4 {
    s: [u8; 256],
    i: u8,
    j: u8,
}

impl SArc4 {
    /// Initialise the cipher with the given key (standard RC4 KSA).
    pub fn new(key: &[u8]) -> Self {
        let mut s = [0u8; 256];
        for (idx, b) in s.iter_mut().enumerate() {
            *b = idx as u8;
        }

        let mut j: u8 = 0;
        for i in 0..256 {
            j = j
                .wrapping_add(s[i])
                .wrapping_add(key[i % key.len()]);
            s.swap(i, j as usize);
        }

        Self { s, i: 0, j: 0 }
    }

    /// Encrypt / decrypt `data` in-place (RC4 is symmetric).
    pub fn process(&mut self, data: &mut [u8]) {
        for byte in data.iter_mut() {
            self.i = self.i.wrapping_add(1);
            self.j = self.j.wrapping_add(self.s[self.i as usize]);
            self.s.swap(self.i as usize, self.j as usize);
            let k = self.s[(self.s[self.i as usize].wrapping_add(self.s[self.j as usize])) as usize];
            *byte ^= k;
        }
    }

    /// Discard the first `n` bytes of the key-stream (common hardening step).
    pub fn drop(&mut self, n: usize) {
        let mut discard = vec![0u8; n];
        self.process(&mut discard);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        let key = b"WoW session key!";
        let plaintext = b"Hello, Azeroth!";
        let mut buf = plaintext.to_vec();

        let mut enc = SArc4::new(key);
        enc.process(&mut buf);
        // After encryption the buffer must differ from plaintext.
        assert_ne!(&buf[..], &plaintext[..]);

        let mut dec = SArc4::new(key);
        dec.process(&mut buf);
        // After decryption we must recover the original plaintext.
        assert_eq!(&buf[..], &plaintext[..]);
    }

    #[test]
    fn drop_prefix_changes_stream() {
        let key = b"TestKey";
        let mut buf1 = [0u8; 16];
        let mut buf2 = [0u8; 16];

        let mut c1 = SArc4::new(key);
        c1.process(&mut buf1);

        let mut c2 = SArc4::new(key);
        c2.drop(1024);
        c2.process(&mut buf2);

        // After dropping 1024 bytes the stream must differ.
        assert_ne!(buf1, buf2);
    }

    #[test]
    fn known_rc4_vector() {
        // RFC 6229 test vector: Key = 0102030405 (first 16 output bytes)
        let key: [u8; 5] = [0x01, 0x02, 0x03, 0x04, 0x05];
        let mut buf = [0u8; 16];
        let mut c = SArc4::new(&key);
        c.process(&mut buf);
        let expected: [u8; 16] = [
            0xb2, 0x39, 0x63, 0x05, 0xf0, 0x3d, 0xc0, 0x27,
            0xcc, 0xc3, 0x52, 0x4a, 0x0a, 0x11, 0x18, 0xa8,
        ];
        assert_eq!(buf, expected);
    }
}
