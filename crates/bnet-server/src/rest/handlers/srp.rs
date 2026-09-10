//! Srp packets.
//!
//! Separated from handlers.rs under #693.

use super::*;

const BOT_SRP_N_HEX: &str = "894B645E89E1535BBDAD5B8B290650530801B18EBFBF5E8FAB3C82872A3E9BB7";

pub(super) struct BnetRestSrpState {
    pub(super) srp: BnetSrp6,
    pub(super) account_id: u32,
}

pub(super) struct BotSrpState {
    pub(super) username: String,
    pub(super) verifier: BigUint,
    pub(super) b: BigUint,
    pub(super) public_b: BigUint,
}

pub(super) struct BotSrpProof {
    pub(super) server_m2: BigUint,
    pub(super) session_key: Vec<u8>,
}

#[derive(serde::Deserialize)]
struct BotSrpChallengeRequest {
    username: Option<String>,
    password: Option<String>,
}

#[derive(serde::Serialize)]
struct BotSrpChallengeResponse {
    salt: String,
    #[serde(rename = "public_B")]
    public_b: String,
}

pub(super) fn post_bot_srp_challenge(
    connection_state: &mut RestConnectionState,
    body: Option<&[u8]>,
) -> HttpResponse {
    let Some(body_bytes) = body else {
        return empty_response(400, "Bad Request");
    };

    let request: BotSrpChallengeRequest = match serde_json::from_slice(body_bytes) {
        Ok(request) => request,
        Err(_) => return empty_response(400, "Bad Request"),
    };

    let Some(username) = request.username.filter(|value| !value.is_empty()) else {
        return empty_response(400, "Bad Request");
    };
    let Some(password) = request.password.filter(|value| !value.is_empty()) else {
        return empty_response(400, "Bad Request");
    };

    let mut salt = [0u8; 32];
    rand::thread_rng().fill(&mut salt);
    let up_hash = Sha256::digest(format!("{username}:{password}").as_bytes());
    let mut x_input = salt.to_vec();
    x_input.extend_from_slice(&up_hash);
    let x = BigUint::from_bytes_be(&Sha256::digest(&x_input));

    let n = bot_srp_n_like_cpp();
    let g = BigUint::from(2u32);
    let k = bot_srp_k_like_cpp(&n, &g);
    let verifier = g.modpow(&x, &n);
    let b = bot_srp_private_b_like_cpp(&n);
    let public_b = (g.modpow(&b, &n) + (&verifier * &k)) % &n;

    connection_state.bot_srp = Some(BotSrpState {
        username,
        verifier,
        b,
        public_b: public_b.clone(),
    });

    json_response_with_content_type(
        BotSrpChallengeResponse {
            salt: hex_encode_upper(&salt),
            public_b: public_b.to_str_radix(16).to_uppercase(),
        },
        "application/json",
    )
}

pub(super) fn srp_challenge_headers_like_cpp() -> Vec<(&'static str, String)> {
    vec![("Content-Type", "application/json;charset=utf-8".to_string())]
}

pub(super) fn srp_challenge_missing_account_response_like_cpp() -> HttpResponse {
    json_response(error_result("Account not found"))
}

pub(super) fn bot_srp_n_like_cpp() -> BigUint {
    BigUint::parse_bytes(BOT_SRP_N_HEX.as_bytes(), 16).expect("valid bot SRP modulus")
}

pub(super) fn bot_srp_k_like_cpp(n: &BigUint, g: &BigUint) -> BigUint {
    let mut data = bot_fixed_32_be_like_cpp(n);
    data.extend_from_slice(&bot_fixed_32_be_like_cpp(g));
    BigUint::from_bytes_be(&Sha256::digest(data))
}

fn bot_srp_private_b_like_cpp(n: &BigUint) -> BigUint {
    let mut bytes = vec![0u8; n.bits().div_ceil(8) as usize];
    rand::thread_rng().fill(bytes.as_mut_slice());
    let n_minus_one = n - BigUint::from(1u32);
    BigUint::from_bytes_be(&bytes) % n_minus_one
}

pub(super) fn bot_fixed_32_be_like_cpp(value: &BigUint) -> Vec<u8> {
    let bytes = value.to_bytes_be();
    if bytes.len() >= 32 {
        return bytes;
    }

    let mut padded = vec![0u8; 32 - bytes.len()];
    padded.extend_from_slice(&bytes);
    padded
}

pub(super) fn bot_broken_evidence_vector_like_cpp(value: &BigUint) -> Vec<u8> {
    let target_len = (value.bits() as usize + 8) >> 3;
    let bytes = value.to_bytes_be();
    if bytes.len() >= target_len {
        return bytes;
    }

    let mut padded = vec![0u8; target_len - bytes.len()];
    padded.extend_from_slice(&bytes);
    padded
}

pub(super) fn bot_srp_evidence_hash_like_cpp(values: &[&BigUint]) -> BigUint {
    let chunks = values
        .iter()
        .map(|value| bot_broken_evidence_vector_like_cpp(value))
        .collect::<Vec<_>>();
    bot_srp_evidence_hash_from_bytes_like_cpp(&chunks)
}

fn bot_srp_evidence_hash_from_bytes_like_cpp(chunks: &[Vec<u8>]) -> BigUint {
    let mut data = Vec::new();
    for chunk in chunks {
        data.extend_from_slice(chunk);
    }

    BigUint::from_bytes_be(&Sha256::digest(data))
}

pub(super) fn verify_bot_srp_evidence_like_cpp(
    bot_srp: &BotSrpState,
    public_a_hex: &str,
    client_m1_hex: &str,
) -> Option<BotSrpProof> {
    let public_a = BigUint::parse_bytes(public_a_hex.as_bytes(), 16)?;
    let client_m1 = BigUint::parse_bytes(client_m1_hex.as_bytes(), 16)?;

    let n = bot_srp_n_like_cpp();
    if (&public_a % &n).is_zero() {
        return None;
    }

    let u = BigUint::from_bytes_be(&Sha256::digest(
        [
            bot_fixed_32_be_like_cpp(&public_a).as_slice(),
            bot_fixed_32_be_like_cpp(&bot_srp.public_b).as_slice(),
        ]
        .concat(),
    ));
    if (&u % &n).is_zero() {
        return None;
    }

    let s = (&public_a * bot_srp.verifier.modpow(&u, &n)).modpow(&bot_srp.b, &n);
    let expected_m1 = bot_srp_evidence_hash_like_cpp(&[&public_a, &bot_srp.public_b, &s]);
    if expected_m1 != client_m1 {
        return None;
    }

    let session_key = Sha256::digest(bot_broken_evidence_vector_like_cpp(&s)).to_vec();
    let server_m2 = bot_srp_evidence_hash_from_bytes_like_cpp(&[
        bot_broken_evidence_vector_like_cpp(&public_a),
        bot_broken_evidence_vector_like_cpp(&client_m1),
        session_key.clone(),
    ]);

    Some(BotSrpProof {
        server_m2,
        session_key,
    })
}
