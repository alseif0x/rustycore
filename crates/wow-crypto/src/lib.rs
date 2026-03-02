//! WoW cryptography primitives for the RustyCore.
//!
//! This crate provides the following modules:
//!
//! - [`srp6`] -- SRP-6 authentication (GruntSRP6 for WoW 3.4.3).
//! - [`bnet_srp6`] -- BNet SRP-6 authentication (SHA-256/512 variants for BNet login).
//! - [`sarc4`] -- RC4 stream cipher used in legacy packet header encryption.
//! - [`world_crypt`] -- AES-128-GCM packet encryption for the world session.
//! - [`session_key`] -- Session-key generators (SHA-1 and SHA-256 variants).
//! - [`hmac_utils`] -- Thin wrappers around HMAC-SHA1 and HMAC-SHA256.

pub mod bnet_srp6;
pub mod ed25519ctx;
pub mod hmac_utils;
pub mod rsa_sign;
pub mod sarc4;
pub mod session_key;
pub mod srp6;
pub mod world_crypt;

// Re-export the most commonly used types at crate root for convenience.
pub use hmac_utils::{HmacSha1, HmacSha256};
pub use sarc4::SArc4;
pub use session_key::{SessionKeyGenerator, SessionKeyGenerator256};
pub use srp6::{
    Srp6Params, Srp6ServerProof, compute_session_key, compute_verifier, generate_salt,
    generate_server_ephemeral, verify_client_proof,
};
pub use bnet_srp6::{
    BnetSrp6, BnetSrpChallenge, BnetSrpProof, SrpHashFunction, SrpVersion,
    compute_bnet_verifier, generate_bnet_salt, srp_username,
};
pub use world_crypt::{WorldCrypt, WorldCryptPair};
