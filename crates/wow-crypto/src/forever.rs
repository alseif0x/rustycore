//! WoW Classic Forever 1.60.1 world authentication and packet crypt.
//!
//! The Forever world protocol uses the 32-byte challenges and SHA-512
//! derivation from the TrinityCore `WorldSocket` implementation.  It is
//! deliberately separate from [`crate::world_crypt`], whose AES-128-GCM
//! contract is used by the 3.4.3 world protocol.

use aes_gcm::{
    AesGcm, KeyInit, Nonce,
    aead::{Aead, Payload},
    aes::Aes256,
};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha512};
use thiserror::Error;

use crate::ed25519ctx::sign_ed25519ctx;

/// Number of bytes compared from the AuthSession HMAC.
pub const AUTH_DIGEST_LEN: usize = 24;
/// Length of the derived world session key.
pub const SESSION_KEY_LEN: usize = 40;
/// Length of the derived AES key.
pub const ENCRYPTION_KEY_LEN: usize = 32;
/// Length of the AES-GCM packet tag.
pub const FOREVER_TAG_SIZE: usize = 12;

const SERVER_SUFFIX: u32 = 0x5256_5253;
const CLIENT_SUFFIX: u32 = 0x544E_4C43;
const AUTH_CHECK_SEED: [u8; 32] = [
    0xDE, 0x3A, 0x2A, 0x8E, 0x6B, 0x89, 0x52, 0x66, 0x88, 0x9D, 0x7E, 0x7A, 0x77, 0x1D, 0x5D, 0x1F,
    0x4E, 0xD9, 0x0C, 0x23, 0x9B, 0xCD, 0x0E, 0xDC, 0xD2, 0xE8, 0x04, 0x3A, 0x68, 0x64, 0xC7, 0xB0,
];
const SESSION_KEY_SEED: [u8; 32] = [
    0xE8, 0x1E, 0x8B, 0x59, 0x27, 0x62, 0x1E, 0xAA, 0x86, 0x15, 0x18, 0xEA, 0xC0, 0xBF, 0x66, 0x8C,
    0x6D, 0xBF, 0x83, 0x93, 0xBC, 0xAA, 0x80, 0x52, 0x5B, 0x1E, 0xDC, 0x23, 0xA0, 0x12, 0xB7, 0x50,
];
const ENCRYPTION_KEY_SEED: [u8; 32] = [
    0x71, 0xC9, 0xED, 0x5A, 0xA7, 0x0E, 0x4D, 0xFF, 0x4C, 0x36, 0xA6, 0x5A, 0x3E, 0x46, 0x8A, 0x4A,
    0x5D, 0xA1, 0x48, 0xC8, 0x30, 0x47, 0x4A, 0xDE, 0xF6, 0x0D, 0x6C, 0xBE, 0x6F, 0xE4, 0x55, 0x73,
];
const ENABLE_ENCRYPTION_SEED: [u8; 32] = [
    0x66, 0xBE, 0x29, 0x79, 0xEF, 0xF2, 0xD5, 0xB5, 0x61, 0x53, 0xF6, 0x5F, 0x45, 0xAE, 0x81, 0xCB,
    0x32, 0xEC, 0x94, 0xEC, 0x75, 0xB3, 0x5F, 0x44, 0x6A, 0x63, 0x43, 0x67, 0x17, 0x20, 0x44, 0x34,
];
const ENABLE_ENCRYPTION_CONTEXT: [u8; 16] = [
    0xA7, 0x1F, 0xB6, 0x9B, 0xC9, 0x7C, 0xDD, 0x96, 0xE9, 0xBB, 0xB8, 0x21, 0x39, 0x8D, 0x5A, 0xD4,
];

type HmacSha512 = Hmac<Sha512>;
type ForeverAesGcm = AesGcm<Aes256, aes_gcm::aead::consts::U12, aes_gcm::aead::consts::U12>;

/// Errors returned while authenticating a world session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum AuthError {
    /// The 24-byte client digest did not match the derived HMAC.
    #[error("world authentication digest mismatch")]
    DigestMismatch,
}

/// Derived material for the world session.
///
/// The fields intentionally have no `Debug` implementation and can only be
/// accessed through fixed-size getters.
pub struct SessionKeys {
    session_key: [u8; SESSION_KEY_LEN],
    encryption_key: [u8; ENCRYPTION_KEY_LEN],
}

impl SessionKeys {
    /// Return the 40-byte session key used by continued-session authentication.
    pub fn session_key(&self) -> &[u8; SESSION_KEY_LEN] {
        &self.session_key
    }

    /// Return the 32-byte AES-256-GCM packet key.
    pub fn encryption_key(&self) -> &[u8; ENCRYPTION_KEY_LEN] {
        &self.encryption_key
    }
}

/// Verify the truncated AuthSession digest and derive both world keys.
///
/// This is the exact SHA-512 path used by the Forever world reference.  It
/// requires the 64-byte BNet realm-join key and the 16-byte build key; there
/// is intentionally no missing-key or default-key mode here.
pub fn verify_and_derive(
    join_key: &[u8; 64],
    build_key: &[u8; 16],
    local_challenge: &[u8; 32],
    server_challenge: &[u8; 32],
    digest: &[u8; AUTH_DIGEST_LEN],
) -> Result<SessionKeys, AuthError> {
    let digest_key_hash = sha512_parts(&[join_key, build_key]);

    let mut auth_hmac = <HmacSha512 as Mac>::new_from_slice(&digest_key_hash)
        .expect("HMAC-SHA512 accepts the fixed digest key");
    auth_hmac.update(local_challenge);
    auth_hmac.update(server_challenge);
    auth_hmac.update(&AUTH_CHECK_SEED);
    auth_hmac
        .verify_truncated_left(digest)
        .map_err(|_| AuthError::DigestMismatch)?;

    let key_data_hash = Sha512::digest(join_key);
    let session_hmac = hmac_sha512_parts(
        &key_data_hash,
        &[server_challenge, local_challenge, &SESSION_KEY_SEED],
    );
    let session_key = generate_sha512_session_key(&session_hmac);

    let encryption_hmac = hmac_sha512_parts(
        &session_key,
        &[local_challenge, server_challenge, &ENCRYPTION_KEY_SEED],
    );
    let mut encryption_key = [0u8; ENCRYPTION_KEY_LEN];
    encryption_key.copy_from_slice(&encryption_hmac[..ENCRYPTION_KEY_LEN]);

    Ok(SessionKeys {
        session_key,
        encryption_key,
    })
}

/// Sign an `SMSG_ENTER_ENCRYPTED_MODE` proof with Ed25519ctx.
pub fn enable_encryption_signature(
    encryption_key: &[u8; ENCRYPTION_KEY_LEN],
    signer_seed: &[u8; 32],
    enabled: bool,
) -> [u8; 64] {
    let enabled_byte = [enabled as u8];
    let digest = hmac_sha512_parts(encryption_key, &[&enabled_byte, &ENABLE_ENCRYPTION_SEED]);
    sign_ed25519ctx(signer_seed, &digest, &ENABLE_ENCRYPTION_CONTEXT)
}

fn sha512_parts(parts: &[&[u8]]) -> [u8; 64] {
    let mut hasher = Sha512::new();
    for part in parts {
        hasher.update(part);
    }
    hasher.finalize().into()
}

fn hmac_sha512_parts(key: &[u8], parts: &[&[u8]]) -> [u8; 64] {
    let mut mac =
        <HmacSha512 as Mac>::new_from_slice(key).expect("HMAC-SHA512 accepts fixed non-empty keys");
    for part in parts {
        mac.update(part);
    }
    mac.finalize().into_bytes().into()
}

fn generate_sha512_session_key(seed: &[u8; 64]) -> [u8; SESSION_KEY_LEN] {
    let o1 = Sha512::digest(&seed[..32]);
    let o2 = Sha512::digest(&seed[32..]);
    let zero = [0u8; 64];
    let o0 = sha512_parts(&[&o1, &zero, &o2]);

    let mut session_key = [0u8; SESSION_KEY_LEN];
    session_key.copy_from_slice(&o0[..SESSION_KEY_LEN]);
    session_key
}

/// Errors returned by [`ForeverWorldCrypt`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum ForeverWorldCryptError {
    /// AES-GCM could not encrypt the packet.
    #[error("Forever AES-GCM encryption failed")]
    EncryptionFailed,
    /// The packet tag or ciphertext was invalid.
    #[error("Forever AES-GCM authentication failed")]
    AuthenticationFailed,
    /// No nonce remains in this direction.
    #[error("Forever packet counter exhausted")]
    CounterExhausted,
}

/// AES-256-GCM state for the Forever world wire protocol.
///
/// Server and client counters are kept independently because one instance is
/// used by the world socket for both directions. The caller supplies the
/// offset after the plaintext handshake: challenge/auth and enter-mode/ACK
/// are two unencrypted frames per direction in the normal Forever flow.
pub struct ForeverWorldCrypt {
    cipher: ForeverAesGcm,
    server_counter: u64,
    client_counter: u64,
}

impl ForeverWorldCrypt {
    /// Create a crypt state with explicit post-plaintext counters.
    pub fn new(key: &[u8; ENCRYPTION_KEY_LEN], server_counter: u64, client_counter: u64) -> Self {
        Self {
            cipher: ForeverAesGcm::new_from_slice(key).expect("valid AES-256 key"),
            server_counter,
            client_counter,
        }
    }

    /// Encrypt one server-to-client packet and return ciphertext plus its tag.
    pub fn encrypt(
        &mut self,
        plaintext: &[u8],
    ) -> Result<(Vec<u8>, [u8; FOREVER_TAG_SIZE]), ForeverWorldCryptError> {
        let counter = checked_counter(self.server_counter)?;
        let nonce_bytes = make_nonce(counter, SERVER_SUFFIX);
        let result = self
            .cipher
            .encrypt(
                Nonce::from_slice(&nonce_bytes),
                Payload {
                    msg: plaintext,
                    aad: &[],
                },
            )
            .map_err(|_| ForeverWorldCryptError::EncryptionFailed)?;

        let packet = split_ciphertext(result)?;
        self.server_counter = counter + 1;
        Ok(packet)
    }

    /// Decrypt one client-to-server packet using its accompanying tag.
    ///
    /// The counter advances only after authentication succeeds, matching the
    /// reference `WorldPacketCrypt::DecryptRecv` path. The world socket closes
    /// the connection after a failed tag, so an invalid packet is not retried.
    pub fn decrypt(
        &mut self,
        ciphertext: &[u8],
        tag: &[u8; FOREVER_TAG_SIZE],
    ) -> Result<Vec<u8>, ForeverWorldCryptError> {
        let counter = checked_counter(self.client_counter)?;
        let nonce_bytes = make_nonce(counter, CLIENT_SUFFIX);
        let mut combined = Vec::with_capacity(ciphertext.len() + FOREVER_TAG_SIZE);
        combined.extend_from_slice(ciphertext);
        combined.extend_from_slice(tag);
        let plaintext = self
            .cipher
            .decrypt(
                Nonce::from_slice(&nonce_bytes),
                Payload {
                    msg: &combined,
                    aad: &[],
                },
            )
            .map_err(|_| ForeverWorldCryptError::AuthenticationFailed)?;
        self.client_counter = counter + 1;
        Ok(plaintext)
    }

    /// Current server-to-client packet counter.
    pub fn server_counter(&self) -> u64 {
        self.server_counter
    }

    /// Current client-to-server packet counter.
    pub fn client_counter(&self) -> u64 {
        self.client_counter
    }
}

fn checked_counter(counter: u64) -> Result<u64, ForeverWorldCryptError> {
    if counter == u64::MAX {
        Err(ForeverWorldCryptError::CounterExhausted)
    } else {
        Ok(counter)
    }
}

fn make_nonce(counter: u64, suffix: u32) -> [u8; 12] {
    let mut nonce = [0u8; 12];
    nonce[..8].copy_from_slice(&counter.to_le_bytes());
    nonce[8..].copy_from_slice(&suffix.to_le_bytes());
    nonce
}

fn split_ciphertext(
    result: Vec<u8>,
) -> Result<(Vec<u8>, [u8; FOREVER_TAG_SIZE]), ForeverWorldCryptError> {
    if result.len() < FOREVER_TAG_SIZE {
        return Err(ForeverWorldCryptError::EncryptionFailed);
    }
    let split = result.len() - FOREVER_TAG_SIZE;
    let mut tag = [0u8; FOREVER_TAG_SIZE];
    tag.copy_from_slice(&result[split..]);
    Ok((result[..split].to_vec(), tag))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha512_and_hmac_sha512_public_vectors() {
        let digest: [u8; 64] = Sha512::digest(b"abc").into();
        assert_eq!(
            hex(&digest),
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
        );

        let key = [0x0b_u8; 20];
        let mut mac = <HmacSha512 as Mac>::new_from_slice(&key).unwrap();
        mac.update(b"Hi There");
        let output: [u8; 64] = mac.finalize().into_bytes().into();
        assert_eq!(hex(&output), "87aa7cdea5ef619d4ff0b4241a1d6cb02379f4e2ce4ec2787ad0b30545e17cde daa833b7d6b8a702038b274eaea3f4e4be9d914eeb61f1702e696c203a126854".replace(' ', ""));
    }

    #[test]
    fn authentication_rejects_digest_and_input_mutations() {
        let join_key = [0x11_u8; 64];
        let build_key = [0x22_u8; 16];
        let local = [0x33_u8; 32];
        let server = [0x44_u8; 32];
        let digest = auth_digest_for_test(&join_key, &build_key, &local, &server);
        assert_eq!(
            hex(&digest),
            "ddd8745cec462f3eaa9195f58122a5092ab4d2e2232a4a99"
        );

        let keys = verify_and_derive(&join_key, &build_key, &local, &server, &digest).unwrap();
        assert_eq!(
            hex(keys.session_key()),
            "eb1cd3efe62d57594372fa1db16280d3a7b39e1fbc2c038a4467453e841939959482050a792d3f79"
        );
        assert_eq!(
            hex(keys.encryption_key()),
            "c9fdbadd41f423c30d99bd9e895a9cf2cec6902bc3c5dbab8c289b4ac6297eab"
        );

        let mut wrong_digest = digest;
        wrong_digest[0] ^= 1;
        assert!(matches!(
            verify_and_derive(&join_key, &build_key, &local, &server, &wrong_digest),
            Err(AuthError::DigestMismatch)
        ));

        let mut wrong_local = local;
        wrong_local[31] ^= 1;
        assert!(matches!(
            verify_and_derive(&join_key, &build_key, &wrong_local, &server, &digest),
            Err(AuthError::DigestMismatch)
        ));

        let mut wrong_build = build_key;
        wrong_build[15] ^= 1;
        assert!(matches!(
            verify_and_derive(&join_key, &wrong_build, &local, &server, &digest),
            Err(AuthError::DigestMismatch)
        ));
    }

    #[test]
    fn encryption_signature_binds_key_and_enabled_bit() {
        let encryption_key = [0x55_u8; 32];
        let signer_seed = [0x66_u8; 32];
        let enabled = enable_encryption_signature(&encryption_key, &signer_seed, true);
        assert_eq!(
            enabled,
            enable_encryption_signature(&encryption_key, &signer_seed, true)
        );
        assert_ne!(
            enabled,
            enable_encryption_signature(&encryption_key, &signer_seed, false)
        );

        let mut other_key = encryption_key;
        other_key[0] ^= 1;
        assert_ne!(
            enabled,
            enable_encryption_signature(&other_key, &signer_seed, true)
        );
    }

    #[test]
    fn aes256_gcm_roundtrip_and_tag_failure() {
        let key = [0xA5_u8; 32];
        let plaintext = b"Forever world packet";
        let mut sender = ForeverWorldCrypt::new(&key, 2, 2);
        let mut receiver = ForeverWorldCrypt::new(&key, 2, 2);

        let (ciphertext, tag) = sender.encrypt(plaintext).unwrap();
        assert_eq!(hex(&ciphertext), "719338528994445f1a74660d8fa531201cee54c7");
        assert_eq!(hex(&tag), "76716919decd6d44860851a4");
        assert_eq!(sender.server_counter(), 3);
        let (client_ciphertext, client_tag) = client_packet(&key, 2, plaintext);
        assert_eq!(
            hex(&client_ciphertext),
            "2e86ef179b532e4ecb36eb79282c912559284fc3"
        );
        assert_eq!(hex(&client_tag), "66cf3be9980ca75f51c806aa");
        assert_eq!(
            receiver.decrypt(&client_ciphertext, &client_tag).unwrap(),
            plaintext
        );
        assert_eq!(receiver.client_counter(), 3);

        let mut mutated_tag = client_tag;
        mutated_tag[0] ^= 1;
        let mut bad_receiver = ForeverWorldCrypt::new(&key, 2, 2);
        assert_eq!(
            bad_receiver.decrypt(&client_ciphertext, &mutated_tag),
            Err(ForeverWorldCryptError::AuthenticationFailed)
        );
        assert_eq!(bad_receiver.client_counter(), 2);
        assert_eq!(
            bad_receiver
                .decrypt(&client_ciphertext, &client_tag)
                .unwrap(),
            plaintext
        );
    }

    #[test]
    fn direction_and_counter_reuse_are_rejected() {
        let key = [0x77_u8; 32];
        let (ciphertext, tag) = client_packet(&key, 2, b"one");

        let mut replay = ForeverWorldCrypt::new(&key, 2, 2);
        assert_eq!(replay.decrypt(&ciphertext, &tag).unwrap(), b"one");
        assert_eq!(
            replay.decrypt(&ciphertext, &tag),
            Err(ForeverWorldCryptError::AuthenticationFailed)
        );

        let mut wrong_direction = ForeverWorldCrypt::new(&key, 2, 2);
        let (other_ciphertext, other_tag) = wrong_direction.encrypt(b"two").unwrap();
        let mut client_side = ForeverWorldCrypt::new(&key, 2, 2);
        assert_eq!(
            client_side.decrypt(&other_ciphertext, &other_tag),
            Err(ForeverWorldCryptError::AuthenticationFailed)
        );
    }

    #[test]
    fn counter_exhaustion_is_rejected_without_wraparound() {
        let key = [0x88_u8; 32];
        let mut crypt = ForeverWorldCrypt::new(&key, u64::MAX, u64::MAX);
        assert_eq!(
            crypt.encrypt(b"no nonce"),
            Err(ForeverWorldCryptError::CounterExhausted)
        );
        assert_eq!(
            crypt.decrypt(b"no nonce", &[0u8; FOREVER_TAG_SIZE]),
            Err(ForeverWorldCryptError::CounterExhausted)
        );
        assert_eq!(crypt.server_counter(), u64::MAX);
        assert_eq!(crypt.client_counter(), u64::MAX);
    }

    fn auth_digest_for_test(
        join_key: &[u8; 64],
        build_key: &[u8; 16],
        local: &[u8; 32],
        server: &[u8; 32],
    ) -> [u8; AUTH_DIGEST_LEN] {
        let key_hash = sha512_parts(&[join_key, build_key]);
        let full = hmac_sha512_parts(&key_hash, &[local, server, &AUTH_CHECK_SEED]);
        let mut digest = [0u8; AUTH_DIGEST_LEN];
        digest.copy_from_slice(&full[..AUTH_DIGEST_LEN]);
        digest
    }

    fn client_packet(
        key: &[u8; ENCRYPTION_KEY_LEN],
        counter: u64,
        plaintext: &[u8],
    ) -> (Vec<u8>, [u8; FOREVER_TAG_SIZE]) {
        let cipher = ForeverAesGcm::new_from_slice(key).unwrap();
        let nonce_bytes = make_nonce(counter, CLIENT_SUFFIX);
        let encrypted = cipher
            .encrypt(
                Nonce::from_slice(&nonce_bytes),
                Payload {
                    msg: plaintext,
                    aad: &[],
                },
            )
            .unwrap();
        split_ciphertext(encrypted).unwrap()
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }
}
