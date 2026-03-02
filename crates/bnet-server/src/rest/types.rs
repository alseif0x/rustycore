//! JSON types for REST API requests and responses.

use serde::{Deserialize, Serialize};

/// Login form definition returned by GET /bnetserver/login/.
#[derive(Serialize)]
pub struct FormInputs {
    #[serde(rename = "type")]
    pub form_type: &'static str,
    pub inputs: Vec<FormInput>,
}

#[derive(Serialize)]
pub struct FormInput {
    pub input_id: &'static str,
    #[serde(rename = "type")]
    pub input_type: &'static str,
    pub label: &'static str,
    pub max_length: u32,
}

/// Full login form response (includes SRP URL).
#[derive(Serialize)]
pub struct FormResponse {
    #[serde(rename = "type")]
    pub form_type: &'static str,
    pub inputs: Vec<FormInput>,
    pub srp_url: String,
    /// SRP JavaScript URL (null in WotLK 3.4.3 — always None but serialized as null to match C#).
    pub srp_js: Option<String>,
}

/// Login request body (POST /bnetserver/login/).
#[derive(Deserialize)]
pub struct LoginForm {
    pub inputs: Vec<LoginInput>,
}

#[derive(Deserialize)]
pub struct LoginInput {
    pub input_id: String,
    pub value: String,
}

/// SRP challenge response.
#[derive(Serialize)]
pub struct SrpLoginChallenge {
    pub version: u32,
    pub iterations: u32,
    pub modulus: String,
    pub generator: String,
    pub hash_function: &'static str,
    pub username: String,
    pub salt: String,
    pub public_b: String,
}

/// Authentication result.
/// All fields are serialized (including null) to match C# behavior exactly.
#[derive(Serialize)]
pub struct AuthResult {
    pub authentication_state: &'static str,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub url: Option<String>,
    pub login_ticket: Option<String>,
    #[serde(rename = "server_evidence_M2")]
    pub server_evidence_m2: Option<String>,
}

/// Game account list entry.
#[derive(Serialize)]
pub struct GameAccountEntry {
    pub display_name: String,
    pub expansion: u32,
    pub is_suspended: bool,
    pub is_banned: bool,
    pub suspension_expires: u64,
    pub suspension_reason: String,
}

/// Game accounts response.
#[derive(Serialize)]
pub struct GameAccountsResponse {
    pub game_accounts: Vec<GameAccountEntry>,
}
