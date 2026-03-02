//! AES-128-GCM packet encryption used by the WoW 3.4.3 world-server.
//!
//! Each direction (server-to-client, client-to-server) maintains its own
//! monotonic counter. The 12-byte nonce is constructed as:
//!
//! ```text
//! [ counter (8 bytes LE) | suffix (4 bytes LE) ]
//! ```
//!
//! where `suffix` is `0x52565253` ("SRVR") for server encryption and
//! `0x544E4C43` ("CLNT") for client decryption.
//!
//! WoW 3.4.3 uses a 12-byte GCM authentication tag (not the standard 16-byte).
//! This matches the C# implementation: `new AesGcm(key, tagSizeInBytes: 12)`.

use aes_gcm::{
    AesGcm, KeyInit, Nonce,
    aead::{Aead, Payload},
    aes::Aes128,
};
use thiserror::Error;

/// Tag size used by the WoW protocol (12-byte GCM tag, not the standard 16).
pub const TAG_SIZE: usize = 12;

/// 4-byte direction marker for server-originated packets ("SRVR").
const SERVER_SUFFIX: u32 = 0x5256_5253;
/// 4-byte direction marker for client-originated packets ("CLNT").
const CLIENT_SUFFIX: u32 = 0x544E_4C43;

/// AES-128-GCM with 12-byte nonce and 12-byte authentication tag.
///
/// Standard GCM uses 16-byte tags, but WoW 3.4.3 (and the C# RustyCore
/// implementation) uses 12-byte tags: `new AesGcm(key, tagSizeInBytes: 12)`.
/// The `aes-gcm` crate supports this via the third type parameter.
type WowAesGcm = AesGcm<Aes128, aes_gcm::aead::consts::U12, aes_gcm::aead::consts::U12>;

#[derive(Debug, Error)]
pub enum WorldCryptError {
    #[error("AES-GCM encryption failed")]
    EncryptionFailed,
    #[error("AES-GCM decryption failed")]
    DecryptionFailed,
}

/// AES-128-GCM encryption state for the WoW world protocol.
///
/// Maintains separate monotonic counters for server→client (encrypt)
/// and client→server (decrypt) directions.
pub struct WorldCrypt {
    cipher: WowAesGcm,
    server_counter: u64,
    client_counter: u64,
}

impl WorldCrypt {
    /// Create a new `WorldCrypt` from a 16-byte AES key.
    pub fn new(key: &[u8; 16]) -> Self {
        let cipher = WowAesGcm::new_from_slice(key).expect("valid 16-byte key");
        Self {
            cipher,
            server_counter: 0,
            client_counter: 0,
        }
    }

    /// Create a new `WorldCrypt` with a pre-set server counter.
    ///
    /// This is needed because the C# server always increments the server
    /// counter in `Encrypt()`, even for packets sent before encryption is
    /// enabled. The WoW client mirrors this by incrementing its receive
    /// counter for every packet header it reads. So the first encrypted
    /// packet must use the correct counter offset.
    pub fn new_with_server_counter(key: &[u8; 16], server_counter: u64) -> Self {
        let cipher = WowAesGcm::new_from_slice(key).expect("valid 16-byte key");
        Self {
            cipher,
            server_counter,
            client_counter: 0,
        }
    }

    /// Create a new `WorldCrypt` with a pre-set client counter.
    ///
    /// The WoW client always increments its send counter in `Encrypt()`,
    /// even for packets sent before encryption is enabled (AuthSession,
    /// EnterEncryptedModeAck). The server's decrypt nonce must match,
    /// so the reader starts at the number of unencrypted client packets.
    pub fn new_with_client_counter(key: &[u8; 16], client_counter: u64) -> Self {
        let cipher = WowAesGcm::new_from_slice(key).expect("valid 16-byte key");
        Self {
            cipher,
            server_counter: 0,
            client_counter,
        }
    }

    // -- Nonce construction ---------------------------------------------------

    fn make_nonce(counter: u64, suffix: u32) -> [u8; 12] {
        let mut nonce = [0u8; 12];
        nonce[..8].copy_from_slice(&counter.to_le_bytes());
        nonce[8..12].copy_from_slice(&suffix.to_le_bytes());
        nonce
    }

    // -- Encrypt (server → client) --------------------------------------------

    /// Encrypt `plaintext` with optional associated data (`aad`).
    ///
    /// Returns `(ciphertext, tag)` where `tag` is [`TAG_SIZE`] bytes (12).
    /// The server counter is incremented after each call.
    pub fn encrypt(
        &mut self,
        plaintext: &[u8],
        aad: &[u8],
    ) -> Result<(Vec<u8>, [u8; TAG_SIZE]), WorldCryptError> {
        let nonce_bytes = Self::make_nonce(self.server_counter, SERVER_SUFFIX);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let payload = Payload { msg: plaintext, aad };
        let result = self
            .cipher
            .encrypt(nonce, payload)
            .map_err(|_| WorldCryptError::EncryptionFailed)?;

        // With TagSize=U12, aes-gcm appends the 12-byte tag at the end.
        let ct_len = result.len() - TAG_SIZE;
        let ciphertext = result[..ct_len].to_vec();

        let mut tag = [0u8; TAG_SIZE];
        tag.copy_from_slice(&result[ct_len..]);

        self.server_counter += 1;
        Ok((ciphertext, tag))
    }

    // -- Decrypt (client → server) --------------------------------------------

    /// Decrypt `ciphertext` that was encrypted by the client.
    ///
    /// `tag` is the 12-byte authentication tag that accompanied the packet.
    /// `aad` is the associated data (may be empty).
    ///
    /// The client counter is incremented after each successful call.
    pub fn decrypt(
        &mut self,
        ciphertext: &[u8],
        tag: &[u8; TAG_SIZE],
        aad: &[u8],
    ) -> Result<Vec<u8>, WorldCryptError> {
        let nonce_bytes = Self::make_nonce(self.client_counter, CLIENT_SUFFIX);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // aes-gcm expects ciphertext + tag concatenated.
        // With TagSize=U12, it reads exactly 12 bytes of tag — no padding needed.
        let mut combined = Vec::with_capacity(ciphertext.len() + TAG_SIZE);
        combined.extend_from_slice(ciphertext);
        combined.extend_from_slice(tag);

        let payload = Payload {
            msg: &combined,
            aad,
        };
        let plaintext = self
            .cipher
            .decrypt(nonce, payload)
            .map_err(|_| WorldCryptError::DecryptionFailed)?;

        self.client_counter += 1;
        Ok(plaintext)
    }

    /// Current server (encrypt) counter value.
    pub fn server_counter(&self) -> u64 {
        self.server_counter
    }

    /// Current client (decrypt) counter value.
    pub fn client_counter(&self) -> u64 {
        self.client_counter
    }
}

/// A paired encrypt/decrypt WorldCrypt that uses separate keys and counters
/// so a server can both encrypt outgoing and decrypt incoming packets.
pub struct WorldCryptPair {
    /// Encryptor (server → client).
    pub encryptor: WorldCrypt,
    /// Decryptor (client → server).
    pub decryptor: WorldCrypt,
}

impl WorldCryptPair {
    /// Create a paired crypt from the 16-byte AES keys derived during the
    /// session handshake.
    pub fn new(server_key: &[u8; 16], client_key: &[u8; 16]) -> Self {
        Self {
            encryptor: WorldCrypt::new(server_key),
            decryptor: WorldCrypt::new(client_key),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_roundtrip() {
        // Simulate client→server: raw-encrypt with CLNT nonce, then decrypt().
        let key = [0x42u8; 16];
        let plaintext = b"Hello, World of Warcraft!";
        let aad = b"";

        // Simulate client encrypting with CLIENT_SUFFIX nonce (counter=0)
        let cipher = WowAesGcm::new_from_slice(&key).unwrap();
        let nonce_bytes = WorldCrypt::make_nonce(0, CLIENT_SUFFIX);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let encrypted = cipher
            .encrypt(nonce, Payload { msg: &plaintext[..], aad })
            .unwrap();

        // Split into ciphertext + 12-byte tag
        let ct_len = encrypted.len() - TAG_SIZE;
        let ct = &encrypted[..ct_len];
        let mut tag = [0u8; TAG_SIZE];
        tag.copy_from_slice(&encrypted[ct_len..]);

        // Server decrypts with decrypt() — uses CLIENT_SUFFIX, counter=0
        let mut server = WorldCrypt::new(&key);
        let recovered = server.decrypt(ct, &tag, aad).unwrap();
        assert_eq!(&recovered[..], plaintext.as_slice());
        assert_eq!(server.client_counter(), 1);
    }

    #[test]
    fn server_encrypt_roundtrip() {
        // Simulate server→client: encrypt(), then raw-decrypt with SRVR nonce.
        let key = [0xAB_u8; 16];
        let plaintext = b"Server packet payload";
        let aad = b"";

        let mut server = WorldCrypt::new(&key);
        let (ct, tag) = server.encrypt(plaintext, aad).unwrap();
        assert_eq!(server.server_counter(), 1);

        // Simulate client decrypting with SERVER_SUFFIX nonce (counter=0)
        let cipher = WowAesGcm::new_from_slice(&key).unwrap();
        let nonce_bytes = WorldCrypt::make_nonce(0, SERVER_SUFFIX);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let mut combined = Vec::new();
        combined.extend_from_slice(&ct);
        combined.extend_from_slice(&tag);
        let recovered = cipher
            .decrypt(nonce, Payload { msg: &combined, aad })
            .unwrap();
        assert_eq!(&recovered[..], plaintext.as_slice());
    }

    #[test]
    fn counter_increments() {
        let key = [0x01u8; 16];
        let mut crypt = WorldCrypt::new(&key);
        assert_eq!(crypt.server_counter(), 0);
        assert_eq!(crypt.client_counter(), 0);

        let _ = crypt.encrypt(b"data", b"").unwrap();
        assert_eq!(crypt.server_counter(), 1);

        let _ = crypt.encrypt(b"more", b"").unwrap();
        assert_eq!(crypt.server_counter(), 2);
    }

    #[test]
    fn counter_offset_works() {
        let key = [0x01u8; 16];
        let mut crypt = WorldCrypt::new_with_server_counter(&key, 5);
        assert_eq!(crypt.server_counter(), 5);
        assert_eq!(crypt.client_counter(), 0);

        let _ = crypt.encrypt(b"data", b"").unwrap();
        assert_eq!(crypt.server_counter(), 6);
    }

    #[test]
    fn nonce_construction() {
        let nonce = WorldCrypt::make_nonce(0, SERVER_SUFFIX);
        assert_eq!(&nonce[..8], &[0u8; 8]);
        // suffix = 0x52565253 LE → bytes 53 52 56 52
        assert_eq!(&nonce[8..], &[0x53, 0x52, 0x56, 0x52]);

        let nonce2 = WorldCrypt::make_nonce(0, CLIENT_SUFFIX);
        // suffix = 0x544E4C43 LE → bytes 43 4C 4E 54
        assert_eq!(&nonce2[8..], &[0x43, 0x4C, 0x4E, 0x54]);
    }

    #[test]
    fn nonce_suffix_spells_srvr_and_clnt() {
        let srvr = SERVER_SUFFIX.to_le_bytes();
        assert_eq!(&srvr, b"SRVR");

        let clnt = CLIENT_SUFFIX.to_le_bytes();
        assert_eq!(&clnt, b"CLNT");
    }

    #[test]
    fn tag_size_is_twelve() {
        assert_eq!(TAG_SIZE, 12);
    }

    #[test]
    fn multiple_packets_roundtrip() {
        // Test that sequential encrypt/decrypt with incrementing counters work.
        let key = [0xCD_u8; 16];
        let cipher = WowAesGcm::new_from_slice(&key).unwrap();
        let mut server = WorldCrypt::new(&key);

        for i in 0..5u64 {
            let msg = format!("packet {i}");
            let (ct, tag) = server.encrypt(msg.as_bytes(), b"").unwrap();

            // Simulate client decrypting with matching counter
            let nonce_bytes = WorldCrypt::make_nonce(i, SERVER_SUFFIX);
            let nonce = Nonce::from_slice(&nonce_bytes);
            let mut combined = Vec::new();
            combined.extend_from_slice(&ct);
            combined.extend_from_slice(&tag);
            let recovered = cipher
                .decrypt(nonce, Payload { msg: &combined, aad: b"" })
                .unwrap();
            assert_eq!(&recovered[..], msg.as_bytes());
        }
        assert_eq!(server.server_counter(), 5);
    }
}
