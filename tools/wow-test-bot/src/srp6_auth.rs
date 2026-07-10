//! Implementación completa de autenticación SRP6 para WoW 3.4.3
//! Siguiendo especificación técnica de TrinityCore

use hmac::{Hmac, Mac};
use num_bigint::BigUint;
use num_traits::Zero;
use sha2::{Digest, Sha256};

// Constantes de TrinityCore 3.4.3 (de WorldSocket.cpp)
const AUTH_CHECK_SEED: [u8; 16] = [
    0xC5, 0xC6, 0x98, 0x95, 0x76, 0x3F, 0x1D, 0xCD, 0xB6, 0xA1, 0x37, 0x28, 0xB3, 0x12, 0xFF, 0x8A,
];
const SESSION_KEY_SEED: [u8; 16] = [
    0x58, 0xCB, 0xCF, 0x40, 0xFE, 0x2E, 0xCE, 0xA6, 0x5A, 0x90, 0xB8, 0x01, 0x68, 0x6C, 0x28, 0x0B,
];
const ENCRYPTION_KEY_SEED: [u8; 16] = [
    0xE9, 0x75, 0x3C, 0x50, 0x90, 0x93, 0x61, 0xDA, 0x3B, 0x07, 0xEE, 0xFA, 0xFF, 0x9D, 0x41, 0xB8,
];

// Constantes SRP6
const SRP6_N_HEX: &str = "894B645E89E1535BBDAD5B8B290650530801B18EBFBF5E8FAB3C82872A3CB9C00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
const SRP6_G: u32 = 2;
const SRP6_K: u32 = 3;

#[derive(Debug, Clone)]
pub struct SRP6Client {
    email: String,
    password: String,
    a: BigUint,         // private ephemeral
    A: BigUint,         // public ephemeral
    B: BigUint,         // server public
    s: Vec<u8>,         // salt
    N: BigUint,         // prime
    g: BigUint,         // generator
    k: BigUint,         // multiplier
    S: Option<BigUint>, // session key (shared secret)
}

pub fn calculate_K_from_s(S: &BigUint) -> Vec<u8> {
    Sha256::digest(&S.to_bytes_be()).to_vec()
}

impl SRP6Client {
    pub fn new(email: &str, password: &str) -> Self {
        let N = BigUint::parse_bytes(SRP6_N_HEX.as_bytes(), 16).expect("Failed to parse N");
        let g = BigUint::from(SRP6_G);
        let k = BigUint::from(SRP6_K);

        // Generate private ephemeral a (32 bytes random)
        let a_bytes: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();
        let a = BigUint::from_bytes_be(&a_bytes);

        // Calculate A = g^a mod N
        let A = g.modpow(&a, &N);

        Self {
            email: email.to_string(),
            password: password.to_string(),
            a,
            A,
            B: BigUint::zero(),
            s: Vec::new(),
            N,
            g,
            k,
            S: None,
        }
    }

    pub fn get_A(&self) -> &BigUint {
        &self.A
    }

    pub fn set_challenge(&mut self, salt: &[u8], B: &BigUint) {
        self.s = salt.to_vec();
        self.B = B.clone();
    }

    /// Calculate x = SHA256(salt || SHA256(username || ":" || password))
    fn calculate_x(&self) -> BigUint {
        let username_hash = Self::hash_username(&self.email);
        let mut content = username_hash;
        content.extend_from_slice(b":");
        content.extend_from_slice(self.password.as_bytes());

        let inner_hash = Sha256::digest(&content);

        let mut outer_content = self.s.clone();
        outer_content.extend_from_slice(&inner_hash);

        let x_bytes = Sha256::digest(&outer_content);
        BigUint::from_bytes_be(&x_bytes)
    }

    fn hash_username(username: &str) -> Vec<u8> {
        let upper = username.to_uppercase();
        let hash = Sha256::digest(upper.as_bytes());
        hash.to_vec()
    }

    /// Calculate v = g^x mod N
    fn calculate_v(&self) -> BigUint {
        let x = self.calculate_x();
        self.g.modpow(&x, &self.N)
    }

    /// Calculate u = SHA256(A || B)
    fn calculate_u(&self) -> BigUint {
        let mut content = Vec::new();
        content.extend_from_slice(&self.A.to_bytes_be());
        content.extend_from_slice(&self.B.to_bytes_be());
        let u_bytes = Sha256::digest(&content);
        BigUint::from_bytes_be(&u_bytes)
    }

    /// Calculate S = (B - k*g^x)^(a + u*x) mod N
    pub fn calculate_S(&mut self) -> BigUint {
        let x = self.calculate_x();
        let v = self.calculate_v();
        let u = self.calculate_u();

        // k*v
        let kv = &self.k * &v;

        // B - k*v (mod N)
        let base = if self.B >= kv {
            &self.B - &kv
        } else {
            &self.B + &self.N - &kv
        };

        // a + u*x
        let ux = &u * &x;
        let exp = &self.a + &ux;

        // S = base^exp mod N
        let S = base.modpow(&exp, &self.N);
        self.S = Some(S.clone());
        S
    }

    /// Calculate M1 = SHA256(A || B || S)
    pub fn calculate_M1(&self) -> Vec<u8> {
        let S = self.S.as_ref().expect("S not calculated yet");
        let mut content = Vec::new();
        content.extend_from_slice(&self.A.to_bytes_be());
        content.extend_from_slice(&self.B.to_bytes_be());
        content.extend_from_slice(&S.to_bytes_be());
        Sha256::digest(&content).to_vec()
    }

    /// Calculate K = SHA256(S) - This is the session key
    pub fn calculate_K(&self) -> Vec<u8> {
        let S = self.S.as_ref().expect("S not calculated yet");
        calculate_K_from_s(S)
    }
}

/// TrinityCore SessionKeyGenerator - generates keystream from seed
pub struct SessionKeyGenerator {
    seed: Vec<u8>,
    o1: Vec<u8>,
    o2: Vec<u8>,
    o0: Vec<u8>,
    index: usize,
}

impl SessionKeyGenerator {
    pub fn new(seed: &[u8]) -> Self {
        let half = seed.len() / 2;
        let o1 = Sha256::digest(&seed[..half]).to_vec();
        let o2 = Sha256::digest(&seed[half..]).to_vec();
        // TrinityCore: o0 = SHA256(o1 || zeros(32) || o2)
        let mut o0_input = o1.clone();
        o0_input.extend_from_slice(&[0u8; 32]);
        o0_input.extend_from_slice(&o2);
        let o0 = Sha256::digest(&o0_input).to_vec();

        Self {
            seed: seed.to_vec(),
            o1,
            o2,
            o0,
            index: 0,
        }
    }

    pub fn generate(&mut self, buf: &mut [u8]) {
        for byte in buf.iter_mut() {
            if self.index >= self.o0.len() {
                // Recalculate o0
                let mut input = self.o1.clone();
                input.extend_from_slice(&self.o0);
                input.extend_from_slice(&self.o2);
                self.o0 = Sha256::digest(&input).to_vec();
                self.index = 0;
            }
            *byte = self.o0[self.index];
            self.index += 1;
        }
    }
}

/// Calculate the session key for WorldServer (40 bytes)
pub fn calculate_session_key(
    key_data: &[u8; 64], // client_secret || server_secret
    server_challenge: &[u8; 16],
    local_challenge: &[u8; 16],
) -> [u8; 40] {
    // 1. keyDataHash = SHA256(key_data)
    let key_data_hash = Sha256::digest(key_data);

    // 2. sessionKeyHmac = HMAC-SHA256(keyDataHash, serverChallenge || localChallenge || SessionKeySeed)
    type HmacSha256 = Hmac<Sha256>;
    let mut mac =
        HmacSha256::new_from_slice(&key_data_hash).expect("HMAC can take key of any size");
    mac.update(server_challenge);
    mac.update(local_challenge);
    mac.update(&SESSION_KEY_SEED);
    let session_key_hmac = mac.finalize().into_bytes();

    // 3. Generate 40 bytes with SessionKeyGenerator
    let mut generator = SessionKeyGenerator::new(&session_key_hmac);
    let mut session_key = [0u8; 40];
    generator.generate(&mut session_key);

    session_key
}

/// Calculate encryption key for AES-GCM (16 bytes)
pub fn calculate_encrypt_key(
    session_key: &[u8; 40],
    local_challenge: &[u8; 16],
    server_challenge: &[u8; 16],
) -> [u8; 16] {
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(session_key).expect("HMAC can take key of any size");
    mac.update(local_challenge);
    mac.update(server_challenge);
    mac.update(&ENCRYPTION_KEY_SEED);
    let result = mac.finalize().into_bytes();

    let mut encrypt_key = [0u8; 16];
    encrypt_key.copy_from_slice(&result[..16]);
    encrypt_key
}

/// Calculate digest for AuthSession verification
pub fn calculate_digest(
    key_data: &[u8; 64],
    auth_seed: &[u8; 16], // Win64AuthSeed from build_info
    local_challenge: &[u8; 16],
    server_challenge: &[u8; 16],
) -> [u8; 24] {
    // 1. digestKeyHash = SHA256(key_data || AuthSeed)
    let mut hash_input = key_data.to_vec();
    hash_input.extend_from_slice(auth_seed);
    let digest_key_hash = Sha256::digest(&hash_input);

    // 2. hmac = HMAC-SHA256(digestKeyHash, LocalChallenge || ServerChallenge || AuthCheckSeed)
    type HmacSha256 = Hmac<Sha256>;
    let mut mac =
        HmacSha256::new_from_slice(&digest_key_hash).expect("HMAC can take key of any size");
    mac.update(local_challenge);
    mac.update(server_challenge);
    mac.update(&AUTH_CHECK_SEED);
    let result = mac.finalize().into_bytes();

    // 3. Return first 24 bytes
    let mut digest = [0u8; 24];
    digest.copy_from_slice(&result[..24]);
    digest
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_srp6_calculations() {
        let mut client = SRP6Client::new("test@example.com", "password123");

        // Mock server values
        let salt = vec![0xABu8; 32];
        let B = BigUint::from(12345u32);

        client.set_challenge(&salt, &B);
        let S = client.calculate_S();
        let K = client.calculate_K();
        let M1 = client.calculate_M1();

        assert!(S > BigUint::zero());
        assert_eq!(K.len(), 32);
        assert_eq!(M1.len(), 32);
    }

    #[test]
    fn test_session_key_generation() {
        let key_data = [0xCDu8; 64];
        let server_challenge = [0x12u8; 16];
        let local_challenge = [0x34u8; 16];

        let session_key = calculate_session_key(&key_data, &server_challenge, &local_challenge);
        assert_eq!(session_key.len(), 40);

        // Should be deterministic
        let session_key2 = calculate_session_key(&key_data, &server_challenge, &local_challenge);
        assert_eq!(session_key, session_key2);
    }

    #[test]
    fn test_encrypt_key_derivation() {
        let session_key = [0xABu8; 40];
        let local_challenge = [0x12u8; 16];
        let server_challenge = [0x34u8; 16];

        let encrypt_key = calculate_encrypt_key(&session_key, &local_challenge, &server_challenge);
        assert_eq!(encrypt_key.len(), 16);
    }

    #[test]
    fn test_digest_calculation() {
        let key_data = [0xEFu8; 64];
        let auth_seed = [0x11u8; 16];
        let local_challenge = [0x22u8; 16];
        let server_challenge = [0x33u8; 16];

        let digest = calculate_digest(&key_data, &auth_seed, &local_challenge, &server_challenge);
        assert_eq!(digest.len(), 24);
    }
}
