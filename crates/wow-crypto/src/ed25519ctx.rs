// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Ed25519ctx signing (RFC 8032, phflag=0) for EnterEncryptedMode.
//!
//! WoW 3.4.3 uses Ed25519ctx (contextualized Ed25519) with the context
//! `EnableEncryptionContext` when signing the EnterEncryptedMode packet.
//! Standard Ed25519 (as provided by `ed25519-dalek`) does NOT include
//! the context prefix, producing a completely different signature.
//!
//! This module implements Ed25519ctx from scratch using `curve25519-dalek`
//! primitives for the elliptic curve operations and `sha2::Sha512` for
//! hashing, matching the C# RustyCore `Ed25519Operations.crypto_sign`
//! implementation when `phflag == 0`.

use curve25519_dalek::constants::ED25519_BASEPOINT_TABLE;
use curve25519_dalek::scalar::Scalar;
use sha2::{Digest, Sha512};

/// Domain separator string for Ed25519ctx/Ed25519ph (RFC 8032 Section 2).
const DOM2_PREFIX: &[u8] = b"SigEd25519 no Ed25519 collisions";

/// Sign a message using Ed25519ctx (RFC 8032, phflag=0).
///
/// This matches the C# `Ed25519Operations.crypto_sign` with `phflag=0`
/// and a non-null context. The context bytes are included in the SHA-512
/// hash computations as a domain separator.
///
/// # Parameters
/// - `seed`: 32-byte private key seed
/// - `message`: the message to sign (typically 32-byte HMAC digest)
/// - `context`: context bytes (e.g. `EnableEncryptionContext`, 16 bytes)
///
/// # Returns
/// 64-byte Ed25519 signature `(R || s)`.
pub fn sign_ed25519ctx(seed: &[u8; 32], message: &[u8], context: &[u8]) -> [u8; 64] {
    // Step 1: Expand seed → az = SHA-512(seed) → 64 bytes
    //   az[0..32] → clamped scalar `a`
    //   az[32..64] → nonce key
    let az = Sha512::digest(seed);
    let mut scalar_bytes = [0u8; 32];
    scalar_bytes.copy_from_slice(&az[..32]);

    // Clamp the scalar (RFC 8032 Section 5.1.5)
    scalar_bytes[0] &= 248;
    scalar_bytes[31] &= 127;
    scalar_bytes[31] |= 64;

    // from_bytes_mod_order reduces mod l, which is fine because all
    // subsequent scalar operations are mod l anyway.
    let a = Scalar::from_bytes_mod_order(scalar_bytes);

    // Derive public key: A = a * B
    let public_key = (&a * &*ED25519_BASEPOINT_TABLE).compress();

    // Step 2: Compute nonce r
    //   r = SHA-512(dom2(0, ctx) || az[32..64] || message) mod l
    let mut hasher = Sha512::new();
    write_dom2(&mut hasher, context);
    hasher.update(&az[32..64]); // nonce key
    hasher.update(message);
    let r_hash: [u8; 64] = hasher.finalize().into();
    let r = Scalar::from_bytes_mod_order_wide(&r_hash);

    // Step 3: R = r * B
    let big_r = (&r * &*ED25519_BASEPOINT_TABLE).compress();

    // Step 4: Compute challenge k
    //   k = SHA-512(dom2(0, ctx) || R || A || message) mod l
    let mut hasher = Sha512::new();
    write_dom2(&mut hasher, context);
    hasher.update(big_r.as_bytes());
    hasher.update(public_key.as_bytes());
    hasher.update(message);
    let k_hash: [u8; 64] = hasher.finalize().into();
    let k = Scalar::from_bytes_mod_order_wide(&k_hash);

    // Step 5: s = (r + k * a) mod l
    let s = r + k * a;

    // Signature = (R, s)
    let mut signature = [0u8; 64];
    signature[..32].copy_from_slice(big_r.as_bytes());
    signature[32..].copy_from_slice(s.as_bytes());
    signature
}

/// Write the dom2(phflag=0, context) prefix to a SHA-512 hasher.
///
/// dom2(F, C) = "SigEd25519 no Ed25519 collisions" || F || len(C) || C
fn write_dom2(hasher: &mut Sha512, context: &[u8]) {
    hasher.update(DOM2_PREFIX); // 32 bytes
    hasher.update(&[0u8]); // phflag = 0
    hasher.update(&[context.len() as u8]); // context length
    hasher.update(context); // context bytes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sign_produces_64_bytes() {
        let seed = [0x42u8; 32];
        let message = [0u8; 32];
        let context = [0u8; 16];
        let sig = sign_ed25519ctx(&seed, &message, &context);
        assert_eq!(sig.len(), 64);
    }

    #[test]
    fn sign_is_deterministic() {
        let seed = [0x42u8; 32];
        let message = b"hello world test message 1234567";
        let context = [0xA7u8; 16];

        let sig1 = sign_ed25519ctx(&seed, message, &context);
        let sig2 = sign_ed25519ctx(&seed, message, &context);
        assert_eq!(sig1, sig2);
    }

    #[test]
    fn sign_differs_with_different_context() {
        let seed = [0x42u8; 32];
        let message = [0u8; 32];
        let ctx1 = [0xAAu8; 16];
        let ctx2 = [0xBBu8; 16];

        let sig1 = sign_ed25519ctx(&seed, &message, &ctx1);
        let sig2 = sign_ed25519ctx(&seed, &message, &ctx2);
        assert_ne!(sig1, sig2);
    }

    #[test]
    fn sign_differs_from_standard_ed25519() {
        let seed = [0x42u8; 32];
        let message = [0u8; 32];
        let context = [0xA7u8; 16];

        // Ed25519ctx signature
        let sig_ctx = sign_ed25519ctx(&seed, &message, &context);

        // Standard Ed25519 signature (via ed25519-dalek)
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&seed);
        use ed25519_dalek::Signer;
        let sig_std = signing_key.sign(&message);

        // They MUST be different
        assert_ne!(sig_ctx, sig_std.to_bytes());
    }
}
