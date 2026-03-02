//! SRP-6 implementation for the WoW 3.4.3 (WotLK) GruntSRP6 authentication
//! protocol.
//!
//! All big-integer values are stored in **little-endian** byte order when
//! serialised over the wire (WoW convention). The `num-bigint` crate uses
//! little-endian internally for `from_bytes_le` / `to_bytes_le`, so
//! conversions are straightforward.

use digest::Digest;
use num_bigint::BigUint;
use num_integer::Integer;
use num_traits::Zero;
use rand::RngCore;
use sha1::Sha1;

// ---------------------------------------------------------------------------
// Protocol constants
// ---------------------------------------------------------------------------

/// Safe prime N (256-bit) used by WoW's SRP-6 variant.
///
/// Big-endian hex:
/// `894B645E89E1535BBDAD5B8B290650530801B18EBFBF5E8FAB3C82872A3E9BB7`
fn prime_n() -> BigUint {
    let be_hex = "894B645E89E1535BBDAD5B8B290650530801B18EBFBF5E8FAB3C82872A3E9BB7";
    BigUint::from_bytes_be(&hex_decode(be_hex))
}

/// Generator g = 7.
fn generator_g() -> BigUint {
    BigUint::from(7u32)
}

/// Multiplier k = 3.
fn multiplier_k() -> BigUint {
    BigUint::from(3u32)
}

// ---------------------------------------------------------------------------
// Public parameters
// ---------------------------------------------------------------------------

/// Public SRP-6 parameters (N, g) along with helpers for the WoW variant.
pub struct Srp6Params {
    pub n: BigUint,
    pub g: BigUint,
    pub k: BigUint,
}

impl Default for Srp6Params {
    fn default() -> Self {
        Self {
            n: prime_n(),
            g: generator_g(),
            k: multiplier_k(),
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// SHA-1 hash (returns 20 bytes).
fn sha1(data: &[u8]) -> [u8; 20] {
    let mut h = Sha1::new();
    h.update(data);
    h.finalize().into()
}

/// SHA-1 over the concatenation of multiple slices.
fn sha1_multi(parts: &[&[u8]]) -> [u8; 20] {
    let mut h = Sha1::new();
    for p in parts {
        h.update(p);
    }
    h.finalize().into()
}

/// Convert a `BigUint` to a fixed-width little-endian byte array, zero-padded
/// to `len` bytes.
fn biguint_to_le_fixed(val: &BigUint, len: usize) -> Vec<u8> {
    let mut bytes = val.to_bytes_le();
    bytes.resize(len, 0);
    bytes
}

/// Decode a hex string to bytes.
fn hex_decode(s: &str) -> Vec<u8> {
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap())
        .collect()
}

// ---------------------------------------------------------------------------
// Compute x = SHA1(salt || SHA1(username || ":" || password))
// ---------------------------------------------------------------------------

/// Compute the private key `x` from credentials and salt.
///
/// ```text
/// x = SHA1(salt || SHA1(upper(username) || ":" || upper(password)))
/// ```
///
/// WoW normalises both username and password to uppercase.
pub fn compute_x(username: &str, password: &str, salt: &[u8]) -> BigUint {
    let identity = format!("{}:{}", username.to_ascii_uppercase(), password.to_ascii_uppercase());
    let identity_hash = sha1(identity.as_bytes());

    let mut buf = Vec::with_capacity(salt.len() + 20);
    buf.extend_from_slice(salt);
    buf.extend_from_slice(&identity_hash);

    let x_hash = sha1(&buf);
    BigUint::from_bytes_le(&x_hash)
}

// ---------------------------------------------------------------------------
// Verifier v = g^x mod N
// ---------------------------------------------------------------------------

/// Compute the password verifier stored in the auth database.
pub fn compute_verifier(username: &str, password: &str, salt: &[u8]) -> BigUint {
    let params = Srp6Params::default();
    let x = compute_x(username, password, salt);
    params.g.modpow(&x, &params.n)
}

// ---------------------------------------------------------------------------
// Server ephemeral B = (k*v + g^b) mod N
// ---------------------------------------------------------------------------

/// Generate a random 32-byte server secret `b` and compute the public
/// ephemeral `B = (k*v + g^b) mod N`.
///
/// Returns `(b, B)`.
pub fn generate_server_ephemeral(v: &BigUint) -> (BigUint, BigUint) {
    let params = Srp6Params::default();
    let mut rng = rand::thread_rng();

    loop {
        let mut b_bytes = [0u8; 32];
        rng.fill_bytes(&mut b_bytes);
        let b = BigUint::from_bytes_le(&b_bytes);

        let gb = params.g.modpow(&b, &params.n);
        let kv = (&params.k * v) % &params.n;
        let big_b = (kv + &gb) % &params.n;

        if !big_b.is_zero() {
            return (b, big_b);
        }
    }
}

// ---------------------------------------------------------------------------
// Scrambler u = SHA1(A || B)
// ---------------------------------------------------------------------------

/// Compute the scrambling parameter `u = SHA1(A || B)`.
///
/// Both A and B are serialised as 32-byte little-endian values.
pub fn compute_u(a: &BigUint, b: &BigUint) -> BigUint {
    let a_bytes = biguint_to_le_fixed(a, 32);
    let b_bytes = biguint_to_le_fixed(b, 32);
    let hash = sha1_multi(&[&a_bytes, &b_bytes]);
    BigUint::from_bytes_le(&hash)
}

// ---------------------------------------------------------------------------
// Server session key S = (A * v^u)^b mod N
// ---------------------------------------------------------------------------

/// Compute the raw server-side session secret `S`.
pub fn compute_server_s(a: &BigUint, v: &BigUint, u: &BigUint, b: &BigUint) -> BigUint {
    let params = Srp6Params::default();
    let vu = v.modpow(u, &params.n);
    let avu = (a * &vu) % &params.n;
    avu.modpow(b, &params.n)
}

// ---------------------------------------------------------------------------
// Interleaved session key K = SHA1_Interleave(S)
// ---------------------------------------------------------------------------

/// Compute the 40-byte interleaved session key `K` from `S` using the WoW
/// SHA-1 interleave algorithm.
///
/// 1. Serialise S as 32 bytes LE.
/// 2. If the first byte is zero, skip leading zero bytes (find first non-zero).
///    Actually the WoW algorithm strips leading zero bytes from S.
/// 3. Ensure even length by skipping one more byte if necessary.
/// 4. Split into even-indexed and odd-indexed byte arrays.
/// 5. Hash each half with SHA-1.
/// 6. Interleave the two 20-byte hashes.
pub fn compute_session_key(s: &BigUint) -> [u8; 40] {
    let s_bytes = biguint_to_le_fixed(s, 32);

    // Find the start offset: skip leading zero bytes, then ensure even count.
    let mut start = 0;
    while start < s_bytes.len() && s_bytes[start] == 0 {
        start += 1;
    }
    let remaining = s_bytes.len() - start;
    if remaining.is_odd() {
        start += 1;
    }

    let buf = &s_bytes[start..];
    let half_len = buf.len() / 2;

    // Even-indexed bytes → part1, odd-indexed bytes → part2.
    let mut part1 = Vec::with_capacity(half_len);
    let mut part2 = Vec::with_capacity(half_len);
    for (i, &byte) in buf.iter().enumerate() {
        if i % 2 == 0 {
            part1.push(byte);
        } else {
            part2.push(byte);
        }
    }

    let hash1 = sha1(&part1);
    let hash2 = sha1(&part2);

    // Interleave the two hashes.
    let mut k = [0u8; 40];
    for i in 0..20 {
        k[i * 2] = hash1[i];
        k[i * 2 + 1] = hash2[i];
    }
    k
}

// ---------------------------------------------------------------------------
// Client evidence M1
// ---------------------------------------------------------------------------

/// Compute M1 = SHA1( H(N) xor H(g) || H(username) || salt || A || B || K )
pub fn compute_client_evidence(
    username: &str,
    salt: &[u8],
    a: &BigUint,
    b: &BigUint,
    k: &[u8; 40],
) -> [u8; 20] {
    let params = Srp6Params::default();

    let n_bytes = biguint_to_le_fixed(&params.n, 32);
    let g_bytes = biguint_to_le_fixed(&params.g, 32);

    let hn = sha1(&n_bytes);
    let hg = sha1(&g_bytes);

    // H(N) xor H(g)
    let mut hn_xor_hg = [0u8; 20];
    for i in 0..20 {
        hn_xor_hg[i] = hn[i] ^ hg[i];
    }

    let h_username = sha1(username.to_ascii_uppercase().as_bytes());
    let a_bytes = biguint_to_le_fixed(a, 32);
    let b_bytes = biguint_to_le_fixed(b, 32);

    sha1_multi(&[
        &hn_xor_hg,
        &h_username,
        salt,
        &a_bytes,
        &b_bytes,
        k,
    ])
}

// ---------------------------------------------------------------------------
// Server evidence M2
// ---------------------------------------------------------------------------

/// Compute M2 = SHA1( A || M1 || K )
pub fn compute_server_evidence(
    a: &BigUint,
    m1: &[u8; 20],
    k: &[u8; 40],
) -> [u8; 20] {
    let a_bytes = biguint_to_le_fixed(a, 32);
    sha1_multi(&[&a_bytes, m1, k])
}

// ---------------------------------------------------------------------------
// High-level server-side verifier
// ---------------------------------------------------------------------------

/// Result of a successful server-side SRP6 authentication.
pub struct Srp6ServerProof {
    /// Server secret `b`.
    pub b: BigUint,
    /// Server public ephemeral `B`.
    pub big_b: BigUint,
    /// 40-byte session key `K`.
    pub session_key: [u8; 40],
    /// Server evidence `M2` to send to the client.
    pub m2: [u8; 20],
}

/// Run the full server-side SRP-6 verification against a client's `A` and
/// `M1`.
///
/// Returns `Some(proof)` if the client's proof is valid, `None` otherwise.
pub fn verify_client_proof(
    username: &str,
    salt: &[u8],
    v: &BigUint,
    a: &BigUint,
    client_m1: &[u8; 20],
) -> Option<Srp6ServerProof> {
    let params = Srp6Params::default();

    // Reject A == 0 mod N.
    if (a % &params.n).is_zero() {
        return None;
    }

    let (b, big_b) = generate_server_ephemeral(v);

    let u = compute_u(a, &big_b);
    if u.is_zero() {
        return None;
    }

    let s = compute_server_s(a, v, &u, &b);
    let k = compute_session_key(&s);
    let expected_m1 = compute_client_evidence(username, salt, a, &big_b, &k);

    if expected_m1 != *client_m1 {
        return None;
    }

    let m2 = compute_server_evidence(a, &expected_m1, &k);

    Some(Srp6ServerProof {
        b,
        big_b,
        session_key: k,
        m2,
    })
}

// ---------------------------------------------------------------------------
// Generate a random 32-byte salt
// ---------------------------------------------------------------------------

/// Generate a cryptographically random 32-byte salt.
pub fn generate_salt() -> [u8; 32] {
    let mut salt = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut salt);
    salt
}

#[cfg(test)]
mod tests {
    use super::*;
    use num_traits::One;

    #[test]
    fn constants_are_correct() {
        let n = prime_n();
        // N should be odd (it's a prime).
        assert!(n.bit(0));
        // N should be 256 bits.
        assert_eq!(n.bits(), 256);

        let g = generator_g();
        assert_eq!(g, BigUint::from(7u32));

        let k = multiplier_k();
        assert_eq!(k, BigUint::from(3u32));
    }

    #[test]
    fn verifier_is_deterministic() {
        let salt = [0xAA; 32];
        let v1 = compute_verifier("TESTUSER", "TESTPASS", &salt);
        let v2 = compute_verifier("TESTUSER", "TESTPASS", &salt);
        assert_eq!(v1, v2);
    }

    #[test]
    fn verifier_case_insensitive() {
        let salt = [0xBB; 32];
        let v1 = compute_verifier("TestUser", "TestPass", &salt);
        let v2 = compute_verifier("TESTUSER", "TESTPASS", &salt);
        assert_eq!(v1, v2);
    }

    #[test]
    fn verifier_differs_for_different_passwords() {
        let salt = [0xCC; 32];
        let v1 = compute_verifier("USER", "PASS1", &salt);
        let v2 = compute_verifier("USER", "PASS2", &salt);
        assert_ne!(v1, v2);
    }

    #[test]
    fn verifier_differs_for_different_salts() {
        let salt1 = [0x01; 32];
        let salt2 = [0x02; 32];
        let v1 = compute_verifier("USER", "PASS", &salt1);
        let v2 = compute_verifier("USER", "PASS", &salt2);
        assert_ne!(v1, v2);
    }

    #[test]
    fn server_ephemeral_is_nonzero() {
        let salt = [0xDD; 32];
        let v = compute_verifier("USER", "PASS", &salt);
        let (_b, big_b) = generate_server_ephemeral(&v);
        assert!(!big_b.is_zero());
    }

    #[test]
    fn scrambler_u_is_nonzero_for_typical_values() {
        let a = BigUint::from(12345u64);
        let b = BigUint::from(67890u64);
        let u = compute_u(&a, &b);
        assert!(!u.is_zero());
    }

    #[test]
    fn session_key_is_40_bytes() {
        let s = BigUint::from(999999u64);
        let k = compute_session_key(&s);
        assert_eq!(k.len(), 40);
        // Should not be all zeros.
        assert!(k.iter().any(|&b| b != 0));
    }

    #[test]
    fn full_srp6_self_test() {
        // Simulate a full client+server handshake using the same code.
        let params = Srp6Params::default();
        let username = "TESTUSER";
        let password = "TESTPASS";
        let salt = generate_salt();

        // --- Registration (server stores v and salt) ---
        let v = compute_verifier(username, password, &salt);

        // --- Server generates B ---
        let (b, big_b) = generate_server_ephemeral(&v);

        // --- Client generates A ---
        let mut a_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut a_bytes);
        let a_secret = BigUint::from_bytes_le(&a_bytes);
        let big_a = params.g.modpow(&a_secret, &params.n);

        // --- Both compute u ---
        let u = compute_u(&big_a, &big_b);

        // --- Server computes S ---
        let server_s = compute_server_s(&big_a, &v, &u, &b);

        // --- Client computes S ---
        let x = compute_x(username, password, &salt);
        // Client S = (B - k * g^x) ^ (a + u*x) mod N
        let gx = params.g.modpow(&x, &params.n);
        let kgx = (&params.k * &gx) % &params.n;
        // B - k*g^x mod N: add N to avoid underflow
        let client_base = if big_b >= kgx {
            (&big_b - &kgx) % &params.n
        } else {
            (&big_b + &params.n - &kgx) % &params.n
        };
        let client_exp = (&a_secret + &u * &x) % (&params.n - BigUint::one());
        let client_s = client_base.modpow(&client_exp, &params.n);

        // --- Both should derive the same S ---
        assert_eq!(server_s, client_s, "Server and client S must match");

        // --- Both derive K ---
        let server_k = compute_session_key(&server_s);
        let client_k = compute_session_key(&client_s);
        assert_eq!(server_k, client_k, "Session keys must match");

        // --- Client computes M1 ---
        let m1 = compute_client_evidence(username, &salt, &big_a, &big_b, &client_k);

        // --- Server verifies M1 ---
        let expected_m1 = compute_client_evidence(username, &salt, &big_a, &big_b, &server_k);
        assert_eq!(m1, expected_m1, "M1 must match");

        // --- Server computes M2 ---
        let m2 = compute_server_evidence(&big_a, &m1, &server_k);
        assert!(m2.iter().any(|&b| b != 0), "M2 should not be all zeros");
    }
}
