//! Bot SRP6 Authentication for TrinityCore 3.4.3 bnetserver bot endpoint
//! Uses /login/srp/ and /login/ REST endpoints (stateless bot SRP6)
//!
//! Parameters (from TrinityCore LoginRESTService.cpp, all big-endian to match
//! `BigNumber(..., false)` and `ToByteArray<32>(false)` on the server side):
//! N = 894B645E89E1535BBDAD5B8B290650530801B18EBFBF5E8FAB3C82872A3E9BB7
//! g = 2
//! k  = SHA256(N_BE32 || g_BE32)               interpreted BIG-endian
//! x  = SHA256(salt || SHA256(username || ":" || password))   interpreted BIG-endian
//! u  = SHA256(A_BE32 || B_BE32)               interpreted BIG-endian
//! M1 = SHA256(BE(A) || BE(B) || BE(S))        with BE() = ToByteVector((bits+8)/8, false)
//! K  = SHA256(BE(S))                          (computed server-side, returned in proof JSON)

use num_bigint::BigUint;
use num_traits::Zero;
use rand::{thread_rng, RngCore};
use reqwest::Client;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::time::Duration;

const HTTP_CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const HTTP_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

const BOT_SRP_N_HEX: &str = "894B645E89E1535BBDAD5B8B290650530801B18EBFBF5E8FAB3C82872A3E9BB7";
const BNET_SRP_V1_N_HEX: &str = concat!(
    "86A7F6DEEB306CE519770FE37D556F29944132554DED0BD68205E27F3231FEF5",
    "A10108238A3150C59CAF7B0B6478691C13A6ACF5E1B5ADAFD4A943D4A21A142B",
    "800E8A55F8BFBAC700EB77A7235EE5A609E350EA9FC19F10D921C2FA832E4461",
    "B7125D38D254A0BE873DFC27858ACB3F8B9F258461E4373BC3A6C2A9634324AB",
);
const BOT_SRP_G: u32 = 2;

fn sha256(data: &[u8]) -> Vec<u8> {
    Sha256::digest(data).to_vec()
}

fn sha256_concat(parts: &[&[u8]]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    for part in parts {
        hasher.update(part);
    }
    hasher.finalize().to_vec()
}

pub fn utf8_to_upper_only_latin_like_cpp(input: &str) -> String {
    input
        .chars()
        .map(|ch| match ch {
            'a'..='z' => ((ch as u8) - b'a' + b'A') as char,
            _ => ch,
        })
        .collect()
}

pub fn bnet_srp_username_like_cpp(email: &str) -> String {
    let normalized = utf8_to_upper_only_latin_like_cpp(email);
    hex::encode_upper(Sha256::digest(normalized.as_bytes()))
}

pub fn bnet_v1_registration_material_like_cpp(
    email: &str,
    password: &str,
) -> (String, [u8; 32], Vec<u8>) {
    let mut salt = [0u8; 32];
    thread_rng().fill_bytes(&mut salt);
    let (normalized_email, verifier) = bnet_v1_verifier_for_salt_like_cpp(email, password, &salt);
    (normalized_email, salt, verifier)
}

/// Recompute the Battle.net SRP v1 verifier for an existing salt without
/// mutating account state. The loot QA preflight uses this to prove that the
/// configured credentials already belong to the exact disposable fixture.
pub fn bnet_v1_verifier_for_salt_like_cpp(
    email: &str,
    password: &str,
    salt: &[u8],
) -> (String, Vec<u8>) {
    let normalized_email = utf8_to_upper_only_latin_like_cpp(email);
    let username = bnet_srp_username_like_cpp(&normalized_email);
    let password = utf8_to_upper_only_latin_like_cpp(password);
    let inner = sha256_concat(&[username.as_bytes(), b":", password.as_bytes()]);
    let outer = sha256_concat(&[salt, &inner]);
    let x = BigUint::from_bytes_le(&outer);
    let n = BigUint::parse_bytes(BNET_SRP_V1_N_HEX.as_bytes(), 16)
        .expect("failed to parse BNet SRP v1 modulus");
    let g = BigUint::from(BOT_SRP_G);
    let verifier = g.modpow(&x, &n).to_bytes_le();
    (normalized_email, verifier)
}

fn decode_server_hex(value: &str) -> Result<Vec<u8>, hex::FromHexError> {
    if value.len() % 2 == 0 {
        hex::decode(value)
    } else {
        hex::decode(format!("0{value}"))
    }
}

/// Pad BigUint to 32 bytes BIG-endian — matches BigNumber::ToByteArray<32>(false)
/// on the server (where `littleEndian=false` means big-endian, see BigNumber.cpp).
fn pad_be_32(bn: &BigUint) -> Vec<u8> {
    let bytes = bn.to_bytes_be();
    if bytes.len() >= 32 {
        return bytes;
    }
    let mut out = vec![0u8; 32 - bytes.len()];
    out.extend_from_slice(&bytes);
    out
}

/// Broken evidence vector: BIG-endian bytes with length `(num_bits + 8) >> 3`,
/// matching TrinityCore's GetBrokenEvidenceVector — `bn.ToByteVector(len, false)`
/// returns big-endian, and "+8" adds one extra (most-significant in BE, leading)
/// zero byte whenever num_bits is a multiple of 8.
fn broken_evidence_be(bn: &BigUint) -> Vec<u8> {
    let bit_len = bn.bits() as usize;
    let byte_len = bit_len / 8 + 1;
    let bytes = bn.to_bytes_be();
    if bytes.len() >= byte_len {
        return bytes;
    }
    let mut out = vec![0u8; byte_len - bytes.len()];
    out.extend_from_slice(&bytes);
    out
}

pub struct BotSrp6Client {
    email: String,
    password: String,
    a: BigUint,
    A: BigUint,
    B: BigUint,
    salt: Vec<u8>,
    n: BigUint,
    g: BigUint,
    k: BigUint,
    s: Option<BigUint>,
}

impl BotSrp6Client {
    pub fn new(email: &str, password: &str) -> Self {
        let n =
            BigUint::parse_bytes(BOT_SRP_N_HEX.as_bytes(), 16).expect("Failed to parse BOT_SRP_N");
        let g = BigUint::from(BOT_SRP_G);

        // k = SHA256(N_BE32 || g_BE32) interpreted big-endian, matching
        //   BigNumber(SHA256::GetDigestOf(N.ToByteArray<32>(false), g.ToByteArray<32>(false)), false)
        let k_bytes = sha256_concat(&[&pad_be_32(&n), &pad_be_32(&g)]);
        let k = BigUint::from_bytes_be(&k_bytes);

        // a = random 32 bytes
        let mut a_bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut a_bytes);
        let a = BigUint::from_bytes_be(&a_bytes);

        // A = g^a mod N
        let A = g.modpow(&a, &n);

        Self {
            email: email.to_string(),
            password: password.to_string(),
            a,
            A,
            B: BigUint::zero(),
            salt: Vec::new(),
            n,
            g,
            k,
            s: None,
        }
    }

    pub fn get_a_hex(&self) -> String {
        hex::encode(&self.A.to_bytes_be())
    }

    pub fn set_challenge(&mut self, salt: &[u8], B: &BigUint) {
        self.salt = salt.to_vec();
        self.B = B.clone();
    }

    /// x = SHA256(salt || SHA256(username || ":" || password)), interpreted big-endian.
    /// Server hashes the username verbatim and treats the SHA256 output as a
    /// big-endian BigNumber (BigNumber(xHash, false) → BN_bin2bn → big-endian).
    fn calculate_x(&self) -> BigUint {
        let inner = sha256_concat(&[self.email.as_bytes(), b":", self.password.as_bytes()]);
        let x_bytes = sha256_concat(&[&self.salt, &inner]);
        BigUint::from_bytes_be(&x_bytes)
    }

    /// u = SHA256(A_BE32 || B_BE32) interpreted big-endian.
    fn calculate_u(&self) -> BigUint {
        let u_bytes = sha256_concat(&[&pad_be_32(&self.A), &pad_be_32(&self.B)]);
        BigUint::from_bytes_be(&u_bytes)
    }

    /// S = (B - k*v)^(a + u*x) mod N
    pub fn calculate_s(&mut self) -> BigUint {
        let x = self.calculate_x();
        let v = self.g.modpow(&x, &self.n);
        let kv = (&self.k * &v) % &self.n;

        let base = if self.B >= kv {
            &self.B - &kv
        } else {
            &self.B + &self.n - &kv
        };

        let u = self.calculate_u();
        // exp = a + u*x — DO NOT reduce u*x mod N here. modpow accepts arbitrarily
        // large exponents, and reducing mod N (instead of mod N-1) would change the
        // value of g^exp and break S.
        let exp = &self.a + &u * &x;

        let S = base.modpow(&exp, &self.n);
        self.s = Some(S.clone());
        S
    }

    /// M1 = SHA256(broken_evidence_be(A) || broken_evidence_be(B) || broken_evidence_be(S))
    pub fn calculate_m1(&self) -> Vec<u8> {
        let S = self.s.as_ref().expect("S not calculated yet");
        sha256_concat(&[
            &broken_evidence_be(&self.A),
            &broken_evidence_be(&self.B),
            &broken_evidence_be(S),
        ])
    }
}

/// Authenticate with bnetserver bot endpoint and return (login_ticket, session_key)
pub async fn authenticate_bot(
    base_url: &str,
    email: &str,
    password: &str,
) -> Result<(String, Vec<u8>), Box<dyn std::error::Error>> {
    let mk_client = || -> Result<Client, reqwest::Error> {
        Client::builder()
            .danger_accept_invalid_certs(true)
            .http1_only()
            .connect_timeout(HTTP_CONNECT_TIMEOUT)
            .timeout(HTTP_REQUEST_TIMEOUT)
            .redirect(reqwest::redirect::Policy::none())
            .build()
    };
    let client = mk_client()?;

    let challenge_json = json!({
        "username": email,
        "password": password
    });

    println!("[BNET] Requesting bot SRP challenge for {}...", email);
    let challenge_paths = ["/login/srp/", "/bnetserver/login/srp/"];
    let mut challenge_errors = Vec::new();
    let mut selected_challenge = None;
    for path in challenge_paths {
        let challenge_url = format!("{}{}", base_url, path);
        println!("[BNET] Challenge URL: {}", challenge_url);
        let challenge_resp = client
            .post(&challenge_url)
            .json(&challenge_json)
            .send()
            .await?;

        let status = challenge_resp.status();
        let jsessionid = challenge_resp
            .headers()
            .get("set-cookie")
            .and_then(|v| v.to_str().ok())
            .and_then(|s| s.split(';').next())
            .and_then(|s| s.split('=').nth(1))
            .map(|s| s.to_string());

        println!("[BNET] Challenge response status: {:?}", status);
        println!(
            "[BNET] Challenge set-cookie: {:?}",
            challenge_resp.headers().get("set-cookie")
        );
        println!("[BNET] Extracted JSESSIONID: {:?}", jsessionid);

        if status.is_success() {
            selected_challenge = Some((challenge_resp, jsessionid, path));
            break;
        }

        let text = challenge_resp.text().await?;
        challenge_errors.push(format!("{} => {}", path, text));
    }

    let (challenge_resp, jsessionid, challenge_path) = match selected_challenge {
        Some(challenge) => challenge,
        None => {
            let err = format!(
                "Challenge failed on all paths: {}",
                challenge_errors.join(" | ")
            );
            return authenticate_direct_login_fallback(base_url, email, password)
                .await
                .map_err(|fallback_err| {
                    format!("{err}; direct login fallback failed: {fallback_err}").into()
                });
        }
    };

    let challenge_data: Value = challenge_resp.json().await?;
    println!("[BNET] Challenge received");

    let salt_hex = challenge_data
        .get("salt")
        .and_then(|s| s.as_str())
        .ok_or("No salt in challenge")?;
    let b_hex = challenge_data
        .get("public_B")
        .and_then(|b| b.as_str())
        .ok_or("No public_B in challenge")?;

    let salt = match decode_server_hex(salt_hex) {
        Ok(salt) => salt,
        Err(err) => {
            println!("[BNET] Invalid SRP salt hex ({err}); falling back to direct login");
            return authenticate_direct_login_fallback(base_url, email, password).await;
        }
    };
    let b_bytes = match decode_server_hex(b_hex) {
        Ok(public_b) => public_b,
        Err(err) => {
            println!("[BNET] Invalid SRP public_B hex ({err}); falling back to direct login");
            return authenticate_direct_login_fallback(base_url, email, password).await;
        }
    };
    let B = BigUint::from_bytes_be(&b_bytes);

    // Step 2: Initialize SRP6 client
    let mut srp = BotSrp6Client::new(email, password);
    srp.set_challenge(&salt, &B);

    let _S = srp.calculate_s();
    let M1 = srp.calculate_m1();
    let A_hex = srp.get_a_hex();

    // Step 3: Send proof
    let login_path = if challenge_path == "/bnetserver/login/srp/" {
        "/bnetserver/login/"
    } else {
        "/login/"
    };
    let login_url = format!("{}{}", base_url, login_path);
    let proof_json = json!({
        "username": email,
        "A": &A_hex,
        "M1": hex::encode(&M1)
    });

    println!("[BNET] Sending SRP proof to {}...", login_url);
    println!("[BNET] Proof body prepared");
    let mut proof_req = client.post(&login_url).json(&proof_json);

    if let Some(ref cookie) = jsessionid {
        proof_req = proof_req.header("Cookie", format!("JSESSIONID={}", cookie));
    }

    // Build and print the request for debugging
    let built_req = proof_req.build()?;
    println!("[BNET] Proof request method: {:?}", built_req.method());
    println!("[BNET] Proof request url: {:?}", built_req.url());
    println!("[BNET] Proof request headers: {:?}", built_req.headers());

    let proof_resp = client.execute(built_req).await?;

    // DEBUG: print everything regardless of status
    println!("Status code: {:?}", proof_resp.status());
    println!("Headers: {:?}", proof_resp.headers());
    let body_text = proof_resp.text().await?;
    println!(
        "[BNET] Proof response body received ({} bytes)",
        body_text.len()
    );

    let proof_data: Value = serde_json::from_str(&body_text).unwrap_or(Value::Null);
    println!("[BNET] Proof response parsed");

    let Some(ticket) = proof_data.get("login_ticket").and_then(|t| t.as_str()) else {
        println!(
            "[BNET] Bot SRP proof did not return a login ticket; falling back to direct login"
        );
        return authenticate_direct_login_fallback(base_url, email, password).await;
    };
    let session_key_hex = proof_data
        .get("session_key")
        .and_then(|t| t.as_str())
        .ok_or("No session_key in response")?;
    let session_key = decode_server_hex(session_key_hex)?;

    println!("[BNET] Authentication successful; ticket and session key received");

    Ok((ticket.to_string(), session_key))
}

/// Fallback for the active Rust `bnet-server`, which currently implements the
/// normal `/bnetserver/login/` form flow but not TrinityCore's bot-only
/// `/login/srp/` and `/login/` routes.
///
/// The world login below is driven by `account.session_key_bnet`: after this
/// function returns, `main.rs` writes the returned key into the auth DB and uses
/// the same key to compute CMSG_AUTH_SESSION. The REST login ticket is retained
/// as a credential sanity check, not as the world-session secret.
async fn authenticate_direct_login_fallback(
    base_url: &str,
    email: &str,
    password: &str,
) -> Result<(String, Vec<u8>), Box<dyn std::error::Error>> {
    let client = Client::builder()
        .danger_accept_invalid_certs(true)
        .http1_only()
        .connect_timeout(HTTP_CONNECT_TIMEOUT)
        .timeout(HTTP_REQUEST_TIMEOUT)
        .pool_max_idle_per_host(0)
        .redirect(reqwest::redirect::Policy::none())
        .build()?;

    let login_url = format!("{}/bnetserver/login/", base_url);
    let login_form = json!({
        "platform_id": "Win",
        "program_id": "WoW",
        "version": "3.4.3.52237",
        "inputs": [
            {"input_id": "account_name", "value": email},
            {"input_id": "password", "value": password}
        ]
    });

    println!("[BNET] Bot SRP unavailable; trying direct form login at {login_url}");
    let response = client.post(&login_url).json(&login_form).send().await?;
    let status = response.status();
    let body_text = response.text().await?;
    if !status.is_success() {
        return Err(format!("direct login HTTP {status}: {body_text}").into());
    }

    let data: Value = serde_json::from_str(&body_text)
        .map_err(|e| format!("direct login JSON parse failed: {e}. Body was: {body_text}"))?;
    let ticket = data
        .get("login_ticket")
        .and_then(|t| t.as_str())
        .ok_or_else(|| format!("direct login returned no login_ticket. Body was: {body_text}"))?;

    let mut session_key = vec![0u8; 32];
    thread_rng().fill_bytes(&mut session_key);
    println!("[BNET] Direct login fallback succeeded; generated local K");

    Ok((ticket.to_string(), session_key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stored_salt_verifier_preflight_is_deterministic() {
        let (email, salt, verifier) =
            bnet_v1_registration_material_like_cpp("testbot@bot.local", "fixture-password");
        let (recomputed_email, recomputed) =
            bnet_v1_verifier_for_salt_like_cpp("testbot@bot.local", "fixture-password", &salt);
        assert_eq!(email, recomputed_email);
        assert_eq!(verifier, recomputed);
        assert_ne!(
            verifier,
            bnet_v1_verifier_for_salt_like_cpp("testbot@bot.local", "different-password", &salt).1
        );
    }

    #[test]
    fn test_bot_srp6_calculations() {
        let mut client = BotSrp6Client::new("test@example.com", "password123");

        let salt = vec![0xABu8; 32];
        let B = BigUint::from(12345u32);

        client.set_challenge(&salt, &B);
        let S = client.calculate_s();
        let M1 = client.calculate_m1();

        assert!(S > BigUint::zero());
        assert_eq!(M1.len(), 32);
    }
}
