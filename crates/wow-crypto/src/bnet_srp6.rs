//! BNet SRP-6 authentication for the Battle.net login server.
//!
//! This implements the BNet variant of SRP-6 used by the REST login service.
//! There are four variants depending on version (v1/v2) and hash (SHA-256/SHA-512):
//!
//! - **v1 + SHA-256**: 128-byte modulus, SHA-256 for k/u/evidence, SHA-256 for x
//! - **v1 + SHA-512**: 128-byte modulus, SHA-512 for k/u/evidence, SHA-256 for x
//! - **v2 + SHA-256**: 256-byte modulus (RFC 3526), SHA-256 for k/u/evidence, PBKDF2-SHA-512 for x
//! - **v2 + SHA-512**: 256-byte modulus (RFC 3526), SHA-512 for k/u/evidence, PBKDF2-SHA-512 for x
//!
//! Key differences from GruntSRP6:
//! - Big-endian wire format (not little-endian)
//! - `k = H(N || pad || g)` instead of fixed 3
//! - `g = 2` instead of 7
//! - Evidence: `M1 = H(A || B || S)` with "broken padding"
//! - Username is `hex(SHA256(email.to_uppercase()))`

use digest::Digest;
use num_bigint::BigUint;
use num_traits::Zero;
use sha2::{Sha256, Sha512};

// ── SRP Parameters ──────────────────────────────────────────────────────────

/// BNet SRP6 v1 modulus (1024-bit / 128 bytes).
const N_V1_HEX: &str = concat!(
    "86A7F6DEEB306CE519770FE37D556F29944132554DED0BD68205E27F3231FEF5",
    "A10108238A3150C59CAF7B0B6478691C13A6ACF5E1B5ADAFD4A943D4A21A142B",
    "800E8A55F8BFBAC700EB77A7235EE5A609E350EA9FC19F10D921C2FA832E4461",
    "B7125D38D254A0BE873DFC27858ACB3F8B9F258461E4373BC3A6C2A9634324AB",
);

/// BNet SRP6 v2 modulus (2048-bit / 256 bytes, RFC 3526 Group 14).
const N_V2_HEX: &str = concat!(
    "AC6BDB41324A9A9BF166DE5E1389582FAF72B6651987EE07FC3192943DB56050",
    "A37329CBB4A099ED8193E0757767A13DD52312AB4B03310DCD7F48A9DA04FD50",
    "E8083969EDB767B0CF6095179A163AB3661A05FBD5FAAAE82918A9962F0B93B8",
    "55F97993EC975EEAA80D740ADBF4FF747359D041D5C33EA71D281E446B14773B",
    "CA97B43A23FB801676BD207A436C6481F1D2B9078717461A5B9D32E688F87748",
    "544523B524B0D57D5EA77A2775D2ECFA032CFBDBF52FB3786160279004E57AE6",
    "AF874E7303CE53299CCC041C7BC308D82A5698F3A8D0C38271AE35F8E9DBFBB6",
    "94B5C803D89F7AE435DE236D525F54759B65E372FCD68EF20FA7111F9E4AFF73",
);

/// Generator for both v1 and v2.
const G: u32 = 2;

/// PBKDF2 iteration count for v2 password hashing.
const PBKDF2_ITERATIONS: u32 = 15_000;

// ── Public Types ────────────────────────────────────────────────────────────

/// SRP version (stored in `battlenet_accounts.srp_version`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SrpVersion {
    V1 = 1,
    V2 = 2,
}

/// Hash function variant (stored in DB alongside srp_version).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SrpHashFunction {
    Sha256 = 1,
    Sha512 = 2,
}

/// Parameters returned in the SRP challenge response (sent to client).
#[derive(Debug)]
pub struct BnetSrpChallenge {
    /// SRP version (1 or 2).
    pub version: u32,
    /// Number of PBKDF2 iterations (0 for v1).
    pub iterations: u32,
    /// Modulus N in big-endian bytes.
    pub modulus: Vec<u8>,
    /// Generator g in big-endian bytes.
    pub generator: Vec<u8>,
    /// Hash function name ("SHA-256" or "SHA-512").
    pub hash_function: &'static str,
    /// Username = hex(SHA-256(email.to_uppercase())).
    pub username: String,
    /// Salt from the database.
    pub salt: Vec<u8>,
    /// Server public ephemeral B in big-endian bytes.
    pub public_b: Vec<u8>,
}

/// Server-side proof after successful client evidence verification.
#[derive(Debug)]
pub struct BnetSrpProof {
    /// Server evidence M2 as `BigUint`.
    pub server_evidence: BigUint,
    /// Session secret S (raw, not interleaved like GruntSRP6).
    pub session_key: BigUint,
}

/// BNet SRP6 server-side state.
pub struct BnetSrp6 {
    version: SrpVersion,
    hash_fn: SrpHashFunction,
    n: BigUint,
    g: BigUint,
    k: BigUint,
    v: BigUint,
    salt: Vec<u8>,
    b: BigUint,      // server secret
    big_b: BigUint,  // server public ephemeral
}

impl BnetSrp6 {
    /// Create a new BNet SRP6 instance from stored account data.
    ///
    /// - `username`: The SRP username = `hex(SHA256(email.to_uppercase()))`
    /// - `salt`: Salt from the database
    /// - `verifier`: Password verifier from the database (little-endian bytes,
    ///   matching C# `BigInteger.ToByteArray()` which defaults to LE)
    pub fn new(
        version: SrpVersion,
        hash_fn: SrpHashFunction,
        _username: &str,
        salt: &[u8],
        verifier: &[u8],
    ) -> Self {
        let n = parse_modulus(version);
        let g = BigUint::from(G);
        // C# stores verifier as LE via `new BigInteger(v, true)` (2-param = LE unsigned)
        let v = BigUint::from_bytes_le(verifier);
        let k = compute_k(version, hash_fn, &n, &g);

        // Generate server ephemeral: b random, B = (g^b + k*v) mod N
        let b = generate_private_b(&n);
        let big_b = compute_public_b(&n, &g, &k, &v, &b);

        Self {
            version,
            hash_fn,
            n,
            g,
            k,
            v,
            salt: salt.to_vec(),
            b,
            big_b,
        }
    }

    /// Get the SRP challenge parameters to send to the client.
    pub fn challenge(&self, email: &str) -> BnetSrpChallenge {
        let username = srp_username(email);
        let hash_function = match self.hash_fn {
            SrpHashFunction::Sha256 => "SHA-256",
            SrpHashFunction::Sha512 => "SHA-512",
        };
        let iterations = match self.version {
            SrpVersion::V1 => 0,
            SrpVersion::V2 => PBKDF2_ITERATIONS,
        };

        BnetSrpChallenge {
            version: self.version as u32,
            iterations,
            modulus: self.n.to_bytes_be(),
            generator: self.g.to_bytes_be(),
            hash_function,
            username,
            salt: self.salt.clone(),
            public_b: self.big_b.to_bytes_be(),
        }
    }

    /// Verify client evidence M1. Returns proof if valid.
    ///
    /// - `client_a`: Client public ephemeral A (big-endian bytes)
    /// - `client_m1`: Client evidence M1 (big-endian bytes)
    pub fn verify_client_evidence(
        &self,
        client_a: &[u8],
        client_m1: &[u8],
    ) -> Option<BnetSrpProof> {
        let a = BigUint::from_bytes_be(client_a);
        let m1 = BigUint::from_bytes_be(client_m1);

        // Validate A != 0 mod N
        if (&a % &self.n).is_zero() {
            return None;
        }

        // u = H(A || B)
        let u = compute_u(self.hash_fn, &a, &self.big_b);
        if (&u % &self.n).is_zero() {
            return None;
        }

        // S = (A * v^u)^b mod N
        let vu = self.v.modpow(&u, &self.n);
        let avu = (&a * &vu) % &self.n;
        let s = avu.modpow(&self.b, &self.n);

        // M1_expected = H(A || B || S)
        let expected_m1 = compute_evidence(self.hash_fn, &[&a, &self.big_b, &s]);
        if expected_m1 != m1 {
            return None;
        }

        // M2 = H(A || M1 || S)
        let m2 = compute_evidence(self.hash_fn, &[&a, &m1, &s]);

        Some(BnetSrpProof {
            server_evidence: m2,
            session_key: s,
        })
    }

    /// Check credentials directly (without SRP challenge-response).
    ///
    /// - `username`: SRP username = `hex(SHA256(email.to_uppercase()))`
    /// - `password`: Password (uppercased for v1, case-sensitive for v2)
    pub fn check_credentials(&self, username: &str, password: &str) -> bool {
        let x = compute_x(self.version, username, password, &self.salt);
        let computed_v = self.g.modpow(&x, &self.n);
        let matches = computed_v == self.v;

        #[cfg(feature = "srp-debug")]
        {
            let v_stored_bytes = self.v.to_bytes_le();
            let v_computed_bytes = computed_v.to_bytes_le();
            eprintln!("[SRP-DEBUG] version={:?}, hash_fn={:?}", self.version, self.hash_fn);
            eprintln!("[SRP-DEBUG] username={username}");
            eprintln!("[SRP-DEBUG] password=<{} chars>", password.len());
            eprintln!("[SRP-DEBUG] salt ({} bytes): {:02x?}", self.salt.len(), &self.salt[..self.salt.len().min(16)]);
            eprintln!("[SRP-DEBUG] x = {} (bits={})", x, x.bits());
            eprintln!("[SRP-DEBUG] v_stored  ({} bytes): first16={:02x?}", v_stored_bytes.len(), &v_stored_bytes[..v_stored_bytes.len().min(16)]);
            eprintln!("[SRP-DEBUG] v_computed({} bytes): first16={:02x?}", v_computed_bytes.len(), &v_computed_bytes[..v_computed_bytes.len().min(16)]);
            eprintln!("[SRP-DEBUG] match={matches}");
        }

        // Always log at trace level for production debugging
        tracing::debug!(
            version = ?self.version,
            salt_len = self.salt.len(),
            x_bits = x.bits(),
            matches,
            "SRP check_credentials"
        );

        matches
    }
}

// ── Public Utility Functions ────────────────────────────────────────────────

/// Compute the SRP username from an email address.
///
/// Returns `hex(SHA256(email.to_uppercase()))` (lowercase hex).
pub fn srp_username(email: &str) -> String {
    let hash = Sha256::digest(email.to_uppercase().as_bytes());
    // C# uses b.ToString("X2") → UPPERCASE hex
    hex::encode_upper(hash)
}

/// Generate a random 32-byte salt.
pub fn generate_bnet_salt() -> [u8; 32] {
    let mut salt = [0u8; 32];
    rand::Rng::fill(&mut rand::thread_rng(), &mut salt);
    salt
}

/// Compute a password verifier for registration.
///
/// - `version`: SRP version
/// - `username`: SRP username = `hex(SHA256(email.to_uppercase()))`
/// - `password`: Password (uppercased for v1, case-sensitive for v2)
/// - `salt`: Random salt
pub fn compute_bnet_verifier(
    version: SrpVersion,
    username: &str,
    password: &str,
    salt: &[u8],
) -> Vec<u8> {
    let n = parse_modulus(version);
    let g = BigUint::from(G);
    let x = compute_x(version, username, password, salt);
    // Return LE bytes to match C# BigInteger.ToByteArray() (LE default)
    g.modpow(&x, &n).to_bytes_le()
}

// ── Private Helpers ─────────────────────────────────────────────────────────

fn parse_modulus(version: SrpVersion) -> BigUint {
    let hex = match version {
        SrpVersion::V1 => N_V1_HEX,
        SrpVersion::V2 => N_V2_HEX,
    };
    BigUint::parse_bytes(hex.as_bytes(), 16).expect("invalid modulus hex")
}

fn compute_k(version: SrpVersion, hash_fn: SrpHashFunction, n: &BigUint, g: &BigUint) -> BigUint {
    let n_bytes = n.to_bytes_be();
    let g_bytes = g.to_bytes_be();

    match (version, hash_fn) {
        // v1Hash256: k = SHA256(N || pad(127) || g)
        (SrpVersion::V1, SrpHashFunction::Sha256) => {
            let mut data = n_bytes;
            data.resize(data.len() + 127, 0);
            data.extend_from_slice(&g_bytes);
            BigUint::from_bytes_be(&Sha256::digest(&data))
        }
        // v1Hash512: k = SHA512(N || pad(127) || g)
        (SrpVersion::V1, SrpHashFunction::Sha512) => {
            let mut data = n_bytes;
            data.resize(data.len() + 127, 0);
            data.extend_from_slice(&g_bytes);
            BigUint::from_bytes_be(&Sha512::digest(&data))
        }
        // v2Hash256: k = SHA256(N || pad(255) || g)
        (SrpVersion::V2, SrpHashFunction::Sha256) => {
            let mut data = n_bytes;
            data.resize(data.len() + 255, 0);
            data.extend_from_slice(&g_bytes);
            BigUint::from_bytes_be(&Sha256::digest(&data))
        }
        // v2Hash512: k = SHA512(N || g) — NO padding!
        (SrpVersion::V2, SrpHashFunction::Sha512) => {
            let mut data = n_bytes;
            data.extend_from_slice(&g_bytes);
            BigUint::from_bytes_be(&Sha512::digest(&data))
        }
    }
}

fn compute_x(version: SrpVersion, username: &str, password: &str, salt: &[u8]) -> BigUint {
    match version {
        SrpVersion::V1 => {
            // x = SHA256(salt || SHA256(username || ":" || password))
            let inner = Sha256::digest(format!("{username}:{password}").as_bytes());
            let mut outer_data = salt.to_vec();
            outer_data.extend_from_slice(&inner);
            // C# uses unsigned, little-endian default for this BigInteger constructor
            BigUint::from_bytes_le(&Sha256::digest(&outer_data))
        }
        SrpVersion::V2 => {
            // x = PBKDF2-HMAC-SHA512(username:password, salt, 15000 iterations, 64 bytes)
            let input = format!("{username}:{password}");
            let mut x_bytes = [0u8; 64];
            pbkdf2::pbkdf2_hmac::<Sha512>(
                input.as_bytes(),
                salt,
                PBKDF2_ITERATIONS,
                &mut x_bytes,
            );

            let x_unsigned = BigUint::from_bytes_be(&x_bytes);
            let n = parse_modulus(SrpVersion::V2);
            let n_minus_1 = &n - BigUint::from(1u32);

            if x_bytes[0] & 0x80 != 0 {
                // C# sign handling: x_signed = x_unsigned - 2^512 (goes negative),
                // then x = x_signed % (N-1), if x < 0: x += (N-1).
                // In unsigned arithmetic: result = (N-1) - ((2^512 - x_unsigned) % (N-1))
                let two_pow_512 = BigUint::from(1u32) << 512;
                let neg_abs = &two_pow_512 - &x_unsigned;
                let neg_rem: BigUint = &neg_abs % &n_minus_1;
                if neg_rem.is_zero() {
                    BigUint::zero()
                } else {
                    &n_minus_1 - &neg_rem
                }
            } else {
                x_unsigned % n_minus_1
            }
        }
    }
}

fn generate_private_b(n: &BigUint) -> BigUint {
    let byte_len = (n.bits() as usize + 7) / 8;
    let mut bytes = vec![0u8; byte_len];
    rand::Rng::fill(&mut rand::thread_rng(), bytes.as_mut_slice());
    let b = BigUint::from_bytes_be(&bytes);
    let n_minus_1 = n - BigUint::from(1u32);
    b % n_minus_1
}

fn compute_public_b(n: &BigUint, g: &BigUint, k: &BigUint, v: &BigUint, b: &BigUint) -> BigUint {
    // B = (g^b mod N + k*v) % N
    let gb = g.modpow(b, n);
    let kv = (k * v) % n;
    (gb + kv) % n
}

fn compute_u(hash_fn: SrpHashFunction, a: &BigUint, b: &BigUint) -> BigUint {
    let a_bytes = a.to_bytes_be();
    let b_bytes = b.to_bytes_be();
    let mut data = a_bytes;
    data.extend_from_slice(&b_bytes);

    match hash_fn {
        SrpHashFunction::Sha256 => BigUint::from_bytes_be(&Sha256::digest(&data)),
        SrpHashFunction::Sha512 => BigUint::from_bytes_be(&Sha512::digest(&data)),
    }
}

/// Compute evidence using "broken evidence vector" padding.
///
/// `M = H(BrokenPad(a) || BrokenPad(b) || BrokenPad(c))`
fn compute_evidence(hash_fn: SrpHashFunction, values: &[&BigUint]) -> BigUint {
    let mut data = Vec::new();
    for val in values {
        data.extend_from_slice(&broken_evidence_vector(val));
    }

    match hash_fn {
        SrpHashFunction::Sha256 => BigUint::from_bytes_be(&Sha256::digest(&data)),
        SrpHashFunction::Sha512 => BigUint::from_bytes_be(&Sha512::digest(&data)),
    }
}

/// "Broken" padding: zero-pad big-endian bytes to `(bit_length + 8) / 8` bytes.
///
/// This matches the C# `GetBrokenEvidenceVector` method.
fn broken_evidence_vector(bn: &BigUint) -> Vec<u8> {
    let target_len = (bn.bits() as usize + 8) >> 3;
    let bytes = bn.to_bytes_be();
    if bytes.len() >= target_len {
        return bytes;
    }
    let mut padded = vec![0u8; target_len - bytes.len()];
    padded.extend_from_slice(&bytes);
    padded
}

/// Hex encoding matching C# `byte[].ToHexString()` which uses UPPERCASE.
mod hex {
    /// Uppercase hex, matching C# `b.ToString("X2")`.
    pub fn encode_upper(data: impl AsRef<[u8]>) -> String {
        data.as_ref()
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn srp_username_is_sha256_of_uppercased_email() {
        let email = "test@example.com";
        let username = srp_username(email);
        // SHA256("TEST@EXAMPLE.COM") — C# uses UPPERCASE hex
        let expected = Sha256::digest(b"TEST@EXAMPLE.COM");
        assert_eq!(username, hex::encode_upper(expected));
        // Verify it's uppercase
        assert_eq!(username, username.to_uppercase());
    }

    #[test]
    fn parse_modulus_v1_is_128_bytes() {
        let n = parse_modulus(SrpVersion::V1);
        let bytes = n.to_bytes_be();
        assert_eq!(bytes.len(), 128);
    }

    #[test]
    fn parse_modulus_v2_is_256_bytes() {
        let n = parse_modulus(SrpVersion::V2);
        let bytes = n.to_bytes_be();
        assert_eq!(bytes.len(), 256);
    }

    #[test]
    fn compute_k_v1_sha256_is_nonzero() {
        let n = parse_modulus(SrpVersion::V1);
        let g = BigUint::from(G);
        let k = compute_k(SrpVersion::V1, SrpHashFunction::Sha256, &n, &g);
        assert!(!k.is_zero());
    }

    #[test]
    fn compute_k_v2_sha512_no_padding() {
        let n = parse_modulus(SrpVersion::V2);
        let g = BigUint::from(G);
        let k = compute_k(SrpVersion::V2, SrpHashFunction::Sha512, &n, &g);
        assert!(!k.is_zero());
        // v2+sha512 uses SHA512(N || g) without padding, so k should be different
        // from v2+sha256 which uses SHA256(N || pad || g)
        let k256 = compute_k(SrpVersion::V2, SrpHashFunction::Sha256, &n, &g);
        assert_ne!(k, k256);
    }

    #[test]
    fn check_credentials_v1_roundtrip() {
        let email = "test@example.com";
        let username = srp_username(email);
        let password = "TEST_PASSWORD"; // v1 uses uppercased password
        let salt = generate_bnet_salt();

        // Registration: compute verifier
        let verifier = compute_bnet_verifier(SrpVersion::V1, &username, password, &salt);

        // Login: create SRP instance and verify
        let srp = BnetSrp6::new(
            SrpVersion::V1,
            SrpHashFunction::Sha256,
            &username,
            &salt,
            &verifier,
        );
        assert!(srp.check_credentials(&username, password));
        assert!(!srp.check_credentials(&username, "WRONG_PASSWORD"));
    }

    #[test]
    fn check_credentials_v2_roundtrip() {
        let email = "admin@example.com";
        let username = srp_username(email);
        let password = "SecurePassword123";
        let salt = generate_bnet_salt();

        let verifier = compute_bnet_verifier(SrpVersion::V2, &username, password, &salt);

        let srp = BnetSrp6::new(
            SrpVersion::V2,
            SrpHashFunction::Sha256,
            &username,
            &salt,
            &verifier,
        );
        assert!(srp.check_credentials(&username, password));
        assert!(!srp.check_credentials(&username, "wrong"));
    }

    #[test]
    fn broken_evidence_vector_adds_padding() {
        // A number with exactly 8 bits (1 byte) → target = (8+8)/8 = 2 bytes
        let bn = BigUint::from(255u32); // 0xFF, 8 bits
        let padded = broken_evidence_vector(&bn);
        assert_eq!(padded.len(), 2);
        assert_eq!(padded[0], 0);
        assert_eq!(padded[1], 0xFF);
    }

    #[test]
    fn challenge_returns_valid_data() {
        let email = "player@example.com";
        let username = srp_username(email);
        let salt = generate_bnet_salt();
        let verifier = compute_bnet_verifier(SrpVersion::V1, &username, "PASSWORD", &salt);

        let srp = BnetSrp6::new(
            SrpVersion::V1,
            SrpHashFunction::Sha256,
            &username,
            &salt,
            &verifier,
        );
        let challenge = srp.challenge(email);

        assert_eq!(challenge.version, 1);
        assert_eq!(challenge.iterations, 0);
        assert_eq!(challenge.hash_function, "SHA-256");
        assert_eq!(challenge.modulus.len(), 128);
        assert!(!challenge.public_b.is_empty());
        assert_eq!(challenge.username, username);
    }
}
