//! REST API handler implementations.

use num_bigint::BigUint;
use num_traits::Zero;
use rand::Rng;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use wow_crypto::{BnetSrp6, SrpHashFunction, SrpVersion, srp_username};
use wow_database::LoginStatements;

use super::HttpResponse;
use super::types::*;
use crate::state::{AppState, RestSessionState};

const BOT_SRP_N_HEX: &str = "894B645E89E1535BBDAD5B8B290650530801B18EBFBF5E8FAB3C82872A3E9BB7";

#[derive(Default)]
pub struct RestConnectionState {
    bot_srp: Option<BotSrpState>,
}

struct BotSrpState {
    username: String,
    verifier: BigUint,
    b: BigUint,
    public_b: BigUint,
}

struct BotSrpProof {
    server_m2: BigUint,
    session_key: Vec<u8>,
}

#[derive(serde::Deserialize)]
struct BotSrpChallengeRequest {
    username: Option<String>,
    password: Option<String>,
}

#[derive(serde::Deserialize)]
struct BotLoginRequest {
    username: Option<String>,
    #[serde(rename = "A")]
    public_a: Option<String>,
    #[serde(rename = "M1")]
    client_m1: Option<String>,
}

#[derive(serde::Serialize)]
struct BotSrpChallengeResponse {
    salt: String,
    #[serde(rename = "public_B")]
    public_b: String,
}

#[derive(serde::Serialize)]
struct BotLoginResponse {
    #[serde(rename = "M2")]
    server_m2: String,
    login_ticket: String,
    session_key: String,
}

/// Route an HTTP request to the appropriate handler.
pub async fn route(
    state: &AppState,
    method: &str,
    path: &str,
    headers: &HashMap<String, String>,
    body: Option<&[u8]>,
    connection_state: &mut RestConnectionState,
) -> HttpResponse {
    match (method, path) {
        ("GET", "/bnetserver/login/") => get_form(state),
        ("POST", "/bnetserver/login/") => post_login(state, headers, body).await,
        ("POST", "/bnetserver/login/srp/") => post_login_srp_challenge(state, headers, body).await,
        ("POST", "/login/srp/") => post_bot_srp_challenge(connection_state, body),
        ("POST", "/login/") => post_bot_login(state, connection_state, body).await,
        ("GET", "/bnetserver/gameAccounts/") => get_game_accounts(state, headers).await,
        ("GET", "/bnetserver/portal/") => get_portal(state, headers),
        ("POST", "/bnetserver/refreshLoginTicket/") => refresh_login_ticket(state, headers).await,
        _ => {
            tracing::warn!("REST fallback: {method} {path} — no matching route");
            HttpResponse {
                status_code: 404,
                status_text: "Not Found",
                headers: vec![],
                body: format!("Not found: {method} {path}"),
            }
        }
    }
}

fn post_bot_srp_challenge(
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

async fn post_bot_login(
    state: &AppState,
    connection_state: &mut RestConnectionState,
    body: Option<&[u8]>,
) -> HttpResponse {
    let Some(body_bytes) = body else {
        return empty_response(400, "Bad Request");
    };

    let request: BotLoginRequest = match serde_json::from_slice(body_bytes) {
        Ok(request) => request,
        Err(_) => return empty_response(400, "Bad Request"),
    };

    let Some(username) = request.username.filter(|value| !value.is_empty()) else {
        return empty_response(400, "Bad Request");
    };
    let Some(public_a_hex) = request.public_a.filter(|value| !value.is_empty()) else {
        return empty_response(400, "Bad Request");
    };
    let Some(client_m1_hex) = request.client_m1.filter(|value| !value.is_empty()) else {
        return empty_response(400, "Bad Request");
    };

    let Some(bot_srp) = connection_state.bot_srp.as_ref() else {
        return empty_response(400, "Bad Request");
    };
    if bot_srp.username != username {
        return empty_response(400, "Bad Request");
    }

    let Some(proof) = verify_bot_srp_evidence_like_cpp(bot_srp, &public_a_hex, &client_m1_hex)
    else {
        return empty_response(401, "Unauthorized");
    };
    let login_ticket = make_login_ticket();

    let mut stmt = state
        .login_db
        .prepare(LoginStatements::SEL_BNET_ACCOUNT_ID_BY_EMAIL);
    stmt.set_string(0, &username);
    let result = match state.login_db.query(&stmt).await {
        Ok(result) => result,
        Err(error) => {
            tracing::error!("DB error during bot login account lookup: {error}");
            return json_error_response(500, "Internal Server Error", "Internal error");
        }
    };
    if result.is_empty() {
        return json_error_response_with_content_type(
            401,
            "Unauthorized",
            "account_not_found",
            "application/json",
        );
    }

    let account_id: u32 = result.read(0);
    if let Err(error) = store_login_ticket(state, account_id, &login_ticket).await {
        tracing::error!("DB error storing bot login ticket for account {account_id}: {error}");
        return json_error_response(500, "Internal Server Error", "Internal error");
    }

    json_response_with_content_type(
        BotLoginResponse {
            server_m2: proof.server_m2.to_str_radix(16).to_uppercase(),
            login_ticket,
            session_key: hex_encode_upper(&proof.session_key),
        },
        "application/json",
    )
}

// ── Handlers ─────────────────────────────────────────────────────────────────

/// GET /bnetserver/login/ — Return login form definition.
fn get_form(state: &AppState) -> HttpResponse {
    tracing::debug!("REST: GET /bnetserver/login/ — serving form");
    let form = FormResponse {
        form_type: "LOGIN_FORM",
        inputs: vec![
            FormInput {
                input_id: "account_name",
                input_type: "text",
                label: "E-mail",
                max_length: 320,
            },
            FormInput {
                input_id: "password",
                input_type: "password",
                label: "Password",
                max_length: 128,
            },
            FormInput {
                input_id: "log_in_submit",
                input_type: "submit",
                label: "Log In",
                max_length: 0,
            },
        ],
        srp_url: format!(
            "https://{}:{}/bnetserver/login/srp/",
            state.external_address, state.rest_port
        ),
        srp_js: None,
    };

    let json = serde_json::to_string(&form).unwrap_or_default();
    tracing::debug!("REST: form response = {json}");

    // Generate JSESSIONID cookie matching C# exactly
    let session_id = generate_session_id();
    let domain = state
        .external_address
        .split('.')
        .take(2)
        .collect::<Vec<_>>()
        .join(".");
    let cookie = format!(
        "JSESSIONID={session_id}; Path=/bnetserver; Domain={domain}; Secure; HttpOnly; SameSite=None"
    );

    HttpResponse {
        status_code: 200,
        status_text: "OK",
        headers: vec![
            ("Set-Cookie", cookie),
            ("Content-Type", "application/json;charset=utf-8".to_string()),
        ],
        body: json,
    }
}

/// POST /bnetserver/login/ — Authenticate with credentials (direct or SRP M1).
async fn post_login(
    state: &AppState,
    headers: &HashMap<String, String>,
    body: Option<&[u8]>,
) -> HttpResponse {
    let Some(body_bytes) = body else {
        return json_response(error_result("Missing body"));
    };

    let form: LoginForm = match serde_json::from_slice(body_bytes) {
        Ok(f) => f,
        Err(e) => {
            tracing::warn!("REST: invalid JSON in POST /login/: {e}");
            return json_response(error_result("Invalid request"));
        }
    };

    tracing::debug!(
        "REST: POST /bnetserver/login/ — {} inputs",
        form.inputs.len()
    );
    for input in &form.inputs {
        let val = if input.input_id == "password" {
            "***"
        } else {
            &input.value
        };
        tracing::debug!("  input: {} = {val}", input.input_id);
    }

    // Extract fields
    let account_name = find_input(&form, "account_name");
    let password = find_input(&form, "password");
    let client_a = find_input(&form, "public_A");
    let client_m1 = find_input(&form, "client_evidence_M1");

    let session_id = extract_session_id(headers);

    // SRP challenge-response flow (client sends A and M1)
    if let (Some(a_hex), Some(m1_hex)) = (client_a, client_m1) {
        if let Some(sid) = &session_id {
            if let Some(mut session) = state.rest_sessions.get_mut(sid) {
                if let Some(srp) = session.srp.take() {
                    let a_bytes = hex_decode(&a_hex);
                    let m1_bytes = hex_decode(&m1_hex);
                    let srp_account_id = session.account_id;
                    if let Some(proof) = srp.verify_client_evidence(&a_bytes, &m1_bytes) {
                        let m2_hex = hex_encode(&proof.server_evidence.to_bytes_be());
                        return match create_login_ticket(state, srp_account_id).await {
                            Ok(ticket) => json_response(AuthResult {
                                authentication_state: "DONE",
                                error_code: None,
                                error_message: None,
                                url: None,
                                login_ticket: Some(ticket),
                                server_evidence_m2: Some(m2_hex),
                            }),
                            Err(e) => json_response(AuthResult {
                                authentication_state: "LOGIN",
                                error_code: Some("UNABLE_TO_DECODE".to_string()),
                                error_message: Some(e.to_string()),
                                url: None,
                                login_ticket: None,
                                server_evidence_m2: None,
                            }),
                        };
                    }
                }
            }
        }
        return json_response(AuthResult {
            authentication_state: "DONE",
            error_code: None,
            error_message: None,
            url: None,
            login_ticket: None,
            server_evidence_m2: None,
        });
    }

    // Direct password verification
    let Some(email) = account_name else {
        return json_response(error_result("Missing account name"));
    };
    let Some(password) = password else {
        return json_response(error_result("Missing password"));
    };

    let email_upper = email.to_uppercase();
    let username = srp_username(&email_upper);

    // Query account
    let mut stmt = state
        .login_db
        .prepare(LoginStatements::SEL_BNET_AUTHENTICATION);
    stmt.set_string(0, &email_upper);
    let result = match state.login_db.query(&stmt).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("DB error during login: {e}");
            return json_response(error_result("Internal error"));
        }
    };

    if result.is_empty() {
        return json_response(error_result("Invalid credentials"));
    }

    // Columns: id(0), srp_version(1), salt(2), verifier(3), failed_logins(4),
    //          LoginTicket(5), LoginTicketExpiry(6), isBanned(7)
    let account_id: u32 = result.read(0);
    let failed_logins: u32 = result.try_read::<u32>(4).unwrap_or(0);
    let is_banned: bool = result.try_read::<bool>(7).unwrap_or(false);
    // Note: srp_version is tinyint(4) (signed) in MySQL → i8 in sqlx
    let srp_version: u8 = result.try_read::<i8>(1).map(|v| v as u8).unwrap_or(1);
    let salt: Vec<u8> = result.try_read::<Vec<u8>>(2).unwrap_or_default();
    let verifier: Vec<u8> = result.try_read::<Vec<u8>>(3).unwrap_or_default();

    let version = if srp_version == 2 {
        SrpVersion::V2
    } else {
        SrpVersion::V1
    };
    let password_for_srp = if version == SrpVersion::V1 {
        password.to_uppercase()
    } else {
        password
    };

    tracing::info!(
        "SRP login: version={:?}, salt_len={}, verifier_len={}, salt_first8={:02x?}, verifier_first8={:02x?}",
        version,
        salt.len(),
        verifier.len(),
        &salt[..salt.len().min(8)],
        &verifier[..verifier.len().min(8)],
    );
    let srp = BnetSrp6::new(
        version,
        SrpHashFunction::Sha256,
        &username,
        &salt,
        &verifier,
    );
    tracing::info!(
        "SRP: checking credentials for user={}, password_len={}",
        &username[..username.len().min(16)],
        password_for_srp.len()
    );
    if srp.check_credentials(&username, &password_for_srp) {
        match create_login_ticket(state, account_id).await {
            Ok(ticket) => json_response(AuthResult {
                authentication_state: "DONE",
                error_code: None,
                error_message: None,
                url: None,
                login_ticket: Some(ticket),
                server_evidence_m2: None,
            }),
            Err(e) => json_response(error_result(&e.to_string())),
        }
    } else {
        apply_wrong_password_policy_like_cpp(
            state,
            account_id,
            &email_upper,
            failed_logins,
            is_banned,
            headers,
        )
        .await;
        json_response(error_result("Invalid credentials"))
    }
}

/// POST /bnetserver/login/srp/ — SRP challenge request.
async fn post_login_srp_challenge(
    state: &AppState,
    headers: &HashMap<String, String>,
    body: Option<&[u8]>,
) -> HttpResponse {
    tracing::debug!("REST: POST /bnetserver/login/srp/ — SRP challenge request");

    let Some(body_bytes) = body else {
        return json_error_response(400, "Bad Request", "Missing body");
    };
    let form: LoginForm = match serde_json::from_slice(body_bytes) {
        Ok(f) => f,
        Err(e) => {
            tracing::warn!("REST: invalid JSON in POST /login/srp/: {e}");
            return json_error_response(400, "Bad Request", "Invalid JSON");
        }
    };

    let Some(email) = find_input(&form, "account_name") else {
        return json_error_response(400, "Bad Request", "Missing account_name");
    };

    let email_upper = email.to_uppercase();
    let username = srp_username(&email_upper);

    let mut stmt = state
        .login_db
        .prepare(LoginStatements::SEL_BNET_CHECK_PASSWORD_BY_EMAIL);
    stmt.set_string(0, &email_upper);
    let result = match state.login_db.query(&stmt).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("DB error during SRP challenge: {e}");
            return json_error_response(500, "Internal Server Error", "Internal error");
        }
    };

    if result.is_empty() {
        return json_error_response(400, "Bad Request", "Account not found");
    }

    // Columns: id(0), srp_version(1), salt(2), verifier(3)
    let account_id: u32 = result.read(0);
    // Note: srp_version is tinyint(4) (signed) in MySQL → i8 in sqlx
    let srp_version: u8 = result.try_read::<i8>(1).map(|v| v as u8).unwrap_or(1);
    let salt: Vec<u8> = result.try_read::<Vec<u8>>(2).unwrap_or_default();
    let verifier: Vec<u8> = result.try_read::<Vec<u8>>(3).unwrap_or_default();

    let version = if srp_version == 2 {
        SrpVersion::V2
    } else {
        SrpVersion::V1
    };
    let srp = BnetSrp6::new(
        version,
        SrpHashFunction::Sha256,
        &username,
        &salt,
        &verifier,
    );
    let challenge = srp.challenge(&email_upper);

    let session_id = extract_session_id(headers).unwrap_or_else(generate_session_id);
    state.rest_sessions.insert(
        session_id.clone(),
        RestSessionState {
            srp: Some(srp),
            account_id,
        },
    );

    let response = SrpLoginChallenge {
        version: challenge.version,
        iterations: challenge.iterations,
        modulus: hex_encode(&challenge.modulus),
        generator: hex_encode(&challenge.generator),
        hash_function: challenge.hash_function,
        username: challenge.username,
        salt: hex_encode(&challenge.salt),
        public_b: hex_encode(&challenge.public_b),
    };

    let body = serde_json::to_string(&response).unwrap_or_default();
    let cookie =
        format!("JSESSIONID={session_id}; Path=/bnetserver; Secure; HttpOnly; SameSite=None");

    HttpResponse {
        status_code: 200,
        status_text: "OK",
        headers: vec![
            ("Set-Cookie", cookie),
            ("Content-Type", "application/json;charset=utf-8".to_string()),
        ],
        body,
    }
}

/// GET /bnetserver/gameAccounts/
async fn get_game_accounts(state: &AppState, headers: &HashMap<String, String>) -> HttpResponse {
    tracing::debug!("REST: GET /bnetserver/gameAccounts/");
    let Some(ticket) = extract_auth_ticket(headers) else {
        return json_error_response(401, "Unauthorized", "Missing ticket");
    };

    let mut stmt = state
        .login_db
        .prepare(LoginStatements::SEL_BNET_GAME_ACCOUNT_LIST);
    stmt.set_string(0, &ticket);
    let mut result = match state.login_db.query(&stmt).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("DB error getting game accounts: {e}");
            return json_error_response(500, "Internal Server Error", "Internal error");
        }
    };

    let mut accounts = Vec::new();
    if !result.is_empty() {
        loop {
            let username: String = result.try_read::<String>(0).unwrap_or_default();
            let expansion: u32 = result.try_read::<u32>(1).unwrap_or(2);
            let ban_date: u64 = result.try_read::<u64>(2).unwrap_or(0);
            let unban_date: u64 = result.try_read::<u64>(3).unwrap_or(0);

            let display_name = username
                .rsplit_once('#')
                .map(|(_, n)| format!("WoW{n}"))
                .unwrap_or_else(|| username.clone());

            accounts.push(GameAccountEntry {
                display_name,
                expansion,
                is_suspended: ban_date > 0 && unban_date > 0 && ban_date != unban_date,
                is_banned: ban_date > 0 && ban_date == unban_date,
                suspension_expires: unban_date,
                suspension_reason: String::new(),
            });

            if !result.next_row() {
                break;
            }
        }
    }

    json_response(GameAccountsResponse {
        game_accounts: accounts,
    })
}

/// GET /bnetserver/portal/
fn get_portal(state: &AppState, headers: &HashMap<String, String>) -> HttpResponse {
    tracing::debug!("REST: GET /bnetserver/portal/");
    let client_ip = headers
        .get("x-forwarded-for")
        .map(|s| s.as_str())
        .unwrap_or(&state.external_address);
    let body = format!("{}:{}", client_ip, state.rpc_port);

    HttpResponse {
        status_code: 200,
        status_text: "OK",
        headers: vec![],
        body,
    }
}

/// POST /bnetserver/refreshLoginTicket/
async fn refresh_login_ticket(state: &AppState, headers: &HashMap<String, String>) -> HttpResponse {
    let Some(ticket) = extract_auth_ticket(headers) else {
        return json_error_response(401, "Unauthorized", "Missing ticket");
    };

    let mut stmt = state
        .login_db
        .prepare(LoginStatements::SEL_BNET_EXISTING_AUTHENTICATION);
    stmt.set_string(0, &ticket);
    let result = match state.login_db.query(&stmt).await {
        Ok(result) => result,
        Err(e) => {
            tracing::error!("Failed to load login ticket for refresh: {e}");
            return json_error_response(500, "Internal Server Error", "Internal error");
        }
    };

    let now = unix_timestamp();
    let current_expiry = if result.is_empty() {
        0
    } else {
        result.try_read::<u64>(0).unwrap_or(0)
    };

    if current_expiry <= now {
        return json_response(LoginRefreshResult {
            login_ticket_expiry: None,
            is_expired: Some(true),
        });
    }

    let new_expiry = now + state.ticket_duration;
    let mut stmt = state
        .login_db
        .prepare(LoginStatements::UPD_BNET_EXISTING_AUTHENTICATION);
    stmt.set_u64(0, new_expiry);
    stmt.set_string(1, &ticket);
    if let Err(e) = state.login_db.execute(&stmt).await {
        tracing::error!("Failed to refresh ticket: {e}");
        return json_error_response(500, "Internal Server Error", "Internal error");
    }

    json_response(LoginRefreshResult {
        login_ticket_expiry: Some(new_expiry),
        is_expired: None,
    })
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn json_response<T: serde::Serialize>(value: T) -> HttpResponse {
    json_response_with_content_type(value, "application/json;charset=utf-8")
}

fn json_response_with_content_type<T: serde::Serialize>(
    value: T,
    content_type: &'static str,
) -> HttpResponse {
    let body = serde_json::to_string(&value).unwrap_or_default();
    HttpResponse {
        status_code: 200,
        status_text: "OK",
        headers: vec![("Content-Type", content_type.to_string())],
        body,
    }
}

fn json_error_response(status_code: u16, status_text: &'static str, error: &str) -> HttpResponse {
    json_error_response_with_content_type(
        status_code,
        status_text,
        error,
        "application/json;charset=utf-8",
    )
}

fn json_error_response_with_content_type(
    status_code: u16,
    status_text: &'static str,
    error: &str,
    content_type: &'static str,
) -> HttpResponse {
    let body = serde_json::to_string(&serde_json::json!({"error": error})).unwrap_or_default();
    HttpResponse {
        status_code,
        status_text,
        headers: vec![("Content-Type", content_type.to_string())],
        body,
    }
}

fn empty_response(status_code: u16, status_text: &'static str) -> HttpResponse {
    HttpResponse {
        status_code,
        status_text,
        headers: vec![],
        body: String::new(),
    }
}

fn find_input(form: &LoginForm, id: &str) -> Option<String> {
    form.inputs
        .iter()
        .find(|i| i.input_id == id)
        .map(|i| i.value.clone())
}

fn extract_session_id(headers: &HashMap<String, String>) -> Option<String> {
    headers.get("cookie").and_then(|cookies| {
        cookies
            .split(';')
            .map(str::trim)
            .find(|c| c.starts_with("JSESSIONID="))
            .map(|c| c["JSESSIONID=".len()..].to_string())
    })
}

fn extract_auth_ticket(headers: &HashMap<String, String>) -> Option<String> {
    let mut authorization = headers.get("authorization")?.as_str();
    if let Some(rest) = authorization.strip_prefix("Basic ") {
        authorization = rest;
    }

    let decoded = decode_base64_standard_like_cpp(authorization)?;
    let decoded_header = String::from_utf8(decoded).ok()?;
    let ticket = decoded_header
        .split_once(':')
        .map(|(ticket, _)| ticket)
        .unwrap_or(&decoded_header);

    if ticket.is_empty() {
        None
    } else {
        Some(ticket.to_string())
    }
}

fn generate_session_id() -> String {
    use rand::Rng;
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill(&mut bytes);
    hex_encode(&bytes)
}

fn make_login_ticket() -> String {
    use rand::Rng;
    let mut bytes = [0u8; 20];
    rand::thread_rng().fill(&mut bytes);
    format!("TC-{}", hex_encode(&bytes))
}

async fn create_login_ticket(state: &AppState, account_id: u32) -> anyhow::Result<String> {
    let ticket = make_login_ticket();
    store_login_ticket(state, account_id, &ticket).await?;

    tracing::info!("Login ticket created for account_id={account_id}: {ticket}");
    Ok(ticket)
}

async fn store_login_ticket(state: &AppState, account_id: u32, ticket: &str) -> anyhow::Result<()> {
    let expiry = unix_timestamp() + state.ticket_duration;

    // UPD_BNET_AUTHENTICATION: SET LoginTicket = ?, LoginTicketExpiry = ? WHERE id = ?
    let mut stmt = state
        .login_db
        .prepare(LoginStatements::UPD_BNET_AUTHENTICATION);
    stmt.set_string(0, ticket);
    stmt.set_u64(1, expiry);
    stmt.set_u32(2, account_id);
    state.login_db.execute(&stmt).await?;
    Ok(())
}

async fn apply_wrong_password_policy_like_cpp(
    state: &AppState,
    account_id: u32,
    email: &str,
    failed_logins: u32,
    is_banned: bool,
    headers: &HashMap<String, String>,
) {
    if is_banned {
        return;
    }

    let remote_ip = wrong_password_remote_ip_like_cpp(state, headers);
    if state.wrong_pass_logging {
        tracing::debug!(
            "[{}, Account {}, Id {}] Attempted to connect with wrong password!",
            remote_ip,
            email,
            account_id
        );
    }

    if state.wrong_pass_max == 0 {
        return;
    }

    let next_failed_logins = failed_logins.saturating_add(1);
    let mut stmt = state
        .login_db
        .prepare(LoginStatements::UPD_BNET_FAILED_LOGINS);
    stmt.set_u32(0, account_id);

    let mut trans = wow_database::SqlTransaction::new();
    trans.append(stmt);

    tracing::debug!(
        "MaxWrongPass : {}, failed_login : {}",
        state.wrong_pass_max,
        account_id
    );

    if next_failed_logins >= state.wrong_pass_max {
        if state.wrong_pass_ban_type == 1 {
            let mut stmt = state
                .login_db
                .prepare(LoginStatements::INS_BNET_ACCOUNT_AUTO_BANNED);
            stmt.set_u32(0, account_id);
            stmt.set_u32(1, state.wrong_pass_ban_time);
            trans.append(stmt);
        } else {
            let mut stmt = state.login_db.prepare(LoginStatements::INS_IP_AUTO_BANNED);
            stmt.set_string(0, &remote_ip);
            stmt.set_u32(1, state.wrong_pass_ban_time);
            trans.append(stmt);
        }

        let mut stmt = state
            .login_db
            .prepare(LoginStatements::UPD_BNET_RESET_FAILED_LOGINS);
        stmt.set_u32(0, account_id);
        trans.append(stmt);
    }

    if let Err(e) = state.login_db.commit_transaction(trans).await {
        tracing::warn!("Failed to apply WrongPass policy for account {account_id} ({email}): {e}");
    }
}

fn wrong_password_remote_ip_like_cpp(
    state: &AppState,
    headers: &HashMap<String, String>,
) -> String {
    wrong_password_remote_ip_from_headers_like_cpp(headers, &state.external_address)
}

fn wrong_password_remote_ip_from_headers_like_cpp(
    headers: &HashMap<String, String>,
    fallback: &str,
) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(fallback)
        .to_string()
}

/// C# returns "DONE" with no other fields for wrong password / account not found
/// to prevent account enumeration.
fn error_result(_msg: &str) -> AuthResult {
    AuthResult {
        authentication_state: "DONE",
        error_code: None,
        error_message: None,
        url: None,
        login_ticket: None,
        server_evidence_m2: None,
    }
}

fn unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn hex_encode(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02x}")).collect()
}

fn hex_encode_upper(data: &[u8]) -> String {
    data.iter().map(|b| format!("{b:02X}")).collect()
}

fn bot_srp_n_like_cpp() -> BigUint {
    BigUint::parse_bytes(BOT_SRP_N_HEX.as_bytes(), 16).expect("valid bot SRP modulus")
}

fn bot_srp_k_like_cpp(n: &BigUint, g: &BigUint) -> BigUint {
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

fn bot_fixed_32_be_like_cpp(value: &BigUint) -> Vec<u8> {
    let bytes = value.to_bytes_be();
    if bytes.len() >= 32 {
        return bytes;
    }

    let mut padded = vec![0u8; 32 - bytes.len()];
    padded.extend_from_slice(&bytes);
    padded
}

fn bot_broken_evidence_vector_like_cpp(value: &BigUint) -> Vec<u8> {
    let target_len = (value.bits() as usize + 8) >> 3;
    let bytes = value.to_bytes_be();
    if bytes.len() >= target_len {
        return bytes;
    }

    let mut padded = vec![0u8; target_len - bytes.len()];
    padded.extend_from_slice(&bytes);
    padded
}

fn bot_srp_evidence_hash_like_cpp(values: &[&BigUint]) -> BigUint {
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

fn verify_bot_srp_evidence_like_cpp(
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

fn decode_base64_standard_like_cpp(input: &str) -> Option<Vec<u8>> {
    fn value(byte: u8) -> Option<u8> {
        match byte {
            b'A'..=b'Z' => Some(byte - b'A'),
            b'a'..=b'z' => Some(byte - b'a' + 26),
            b'0'..=b'9' => Some(byte - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }

    let mut output = Vec::with_capacity(input.len() * 3 / 4);
    let mut chunk = [0u8; 4];
    let mut chunk_len = 0usize;
    let mut finished_padding = false;

    for byte in input.bytes() {
        let sextet = if byte == b'=' {
            finished_padding = true;
            64
        } else {
            if finished_padding {
                return None;
            }
            value(byte)?
        };

        chunk[chunk_len] = sextet;
        chunk_len += 1;

        if chunk_len == 4 {
            if chunk[0] == 64 || chunk[1] == 64 || (chunk[2] == 64 && chunk[3] != 64) {
                return None;
            }

            output.push((chunk[0] << 2) | (chunk[1] >> 4));
            if chunk[2] != 64 {
                output.push((chunk[1] << 4) | (chunk[2] >> 2));
            }
            if chunk[3] != 64 {
                output.push((chunk[2] << 6) | chunk[3]);
            }

            chunk_len = 0;
        }
    }

    match chunk_len {
        0 => Some(output),
        2 if !finished_padding => {
            output.push((chunk[0] << 2) | (chunk[1] >> 4));
            Some(output)
        }
        3 if !finished_padding => {
            output.push((chunk[0] << 2) | (chunk[1] >> 4));
            output.push((chunk[1] << 4) | (chunk[2] >> 2));
            Some(output)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrong_password_remote_ip_uses_forwarded_for_first_hop_like_cpp() {
        let headers = HashMap::from([(
            "x-forwarded-for".to_string(),
            "198.51.100.7, 198.51.100.8".to_string(),
        )]);

        assert_eq!(
            wrong_password_remote_ip_from_headers_like_cpp(&headers, "203.0.113.10"),
            "198.51.100.7"
        );
    }

    #[test]
    fn wrong_password_remote_ip_falls_back_to_external_address_like_cpp() {
        let headers = HashMap::new();

        assert_eq!(
            wrong_password_remote_ip_from_headers_like_cpp(&headers, "203.0.113.10"),
            "203.0.113.10"
        );
    }

    #[test]
    fn bot_srp_challenge_rejects_malformed_or_missing_inputs_like_cpp() {
        let mut connection_state = RestConnectionState::default();

        let bad_json = post_bot_srp_challenge(&mut connection_state, Some(b"not-json"));
        assert_eq!(bad_json.status_code, 400);
        assert!(bad_json.body.is_empty());

        let missing_password =
            post_bot_srp_challenge(&mut connection_state, Some(br#"{"username":"user"}"#));
        assert_eq!(missing_password.status_code, 400);
        assert!(missing_password.body.is_empty());
    }

    #[test]
    fn bot_srp_challenge_returns_cpp_shape_and_connection_state() {
        let mut connection_state = RestConnectionState::default();

        let response = post_bot_srp_challenge(
            &mut connection_state,
            Some(br#"{"username":"user@example.com","password":"secret"}"#),
        );

        assert_eq!(response.status_code, 200);
        assert_eq!(
            response.headers,
            vec![("Content-Type", "application/json".to_string())]
        );
        let body: serde_json::Value = serde_json::from_str(&response.body).unwrap();
        assert!(body.get("salt").and_then(|value| value.as_str()).is_some());
        assert!(
            body.get("public_B")
                .and_then(|value| value.as_str())
                .is_some()
        );
        assert!(connection_state.bot_srp.is_some());
    }

    #[test]
    fn bot_srp_fixed_32_and_broken_vectors_match_cpp_lengths() {
        assert_eq!(bot_fixed_32_be_like_cpp(&BigUint::from(2u32)).len(), 32);
        assert_eq!(
            hex_encode_upper(&bot_fixed_32_be_like_cpp(&BigUint::from(2u32))),
            "0000000000000000000000000000000000000000000000000000000000000002"
        );

        assert_eq!(
            bot_broken_evidence_vector_like_cpp(&BigUint::from(0x1234u32)),
            vec![0x12, 0x34]
        );
        assert_eq!(
            bot_broken_evidence_vector_like_cpp(&BigUint::from(0x80u32)),
            vec![0x00, 0x80]
        );
    }

    #[test]
    fn bot_srp_evidence_verifies_matching_client_proof_like_cpp() {
        let n = bot_srp_n_like_cpp();
        let g = BigUint::from(2u32);
        let k = bot_srp_k_like_cpp(&n, &g);
        let x = BigUint::from(11u32);
        let a = BigUint::from(17u32);
        let b = BigUint::from(19u32);
        let verifier = g.modpow(&x, &n);
        let public_a = g.modpow(&a, &n);
        let public_b = (g.modpow(&b, &n) + (&verifier * &k)) % &n;

        let u = BigUint::from_bytes_be(&Sha256::digest(
            [
                bot_fixed_32_be_like_cpp(&public_a).as_slice(),
                bot_fixed_32_be_like_cpp(&public_b).as_slice(),
            ]
            .concat(),
        ));
        let gx = g.modpow(&x, &n);
        let base = (&public_b + &n - ((&k * &gx) % &n)) % &n;
        let client_s = base.modpow(&(&a + (&u * &x)), &n);
        let client_m1 = bot_srp_evidence_hash_like_cpp(&[&public_a, &public_b, &client_s]);

        let bot_srp = BotSrpState {
            username: "user@example.com".to_string(),
            verifier,
            b,
            public_b,
        };
        let proof = verify_bot_srp_evidence_like_cpp(
            &bot_srp,
            &public_a.to_str_radix(16),
            &client_m1.to_str_radix(16),
        )
        .expect("matching bot proof");

        assert_eq!(proof.session_key.len(), 32);
        assert!(
            verify_bot_srp_evidence_like_cpp(&bot_srp, &public_a.to_str_radix(16), "deadbeef")
                .is_none()
        );
    }

    #[test]
    fn extract_auth_ticket_decodes_basic_and_truncates_at_colon_like_cpp() {
        let headers = HashMap::from([(
            "authorization".to_string(),
            "Basic VElDS0VUOnNlY3JldA==".to_string(),
        )]);

        assert_eq!(extract_auth_ticket(&headers).as_deref(), Some("TICKET"));
    }

    #[test]
    fn extract_auth_ticket_decodes_without_basic_prefix_like_cpp() {
        let headers =
            HashMap::from([("authorization".to_string(), "VEMtYWJjMTIzOg==".to_string())]);

        assert_eq!(extract_auth_ticket(&headers).as_deref(), Some("TC-abc123"));
    }

    #[test]
    fn extract_auth_ticket_accepts_decoded_value_without_colon_like_cpp() {
        let headers = HashMap::from([("authorization".to_string(), "VEMtcmF3".to_string())]);

        assert_eq!(extract_auth_ticket(&headers).as_deref(), Some("TC-raw"));
    }

    #[test]
    fn extract_auth_ticket_rejects_invalid_or_empty_ticket_like_cpp() {
        let invalid =
            HashMap::from([("authorization".to_string(), "Basic not base64".to_string())]);
        let empty = HashMap::from([("authorization".to_string(), "Og==".to_string())]);

        assert_eq!(extract_auth_ticket(&invalid), None);
        assert_eq!(extract_auth_ticket(&empty), None);
    }

    #[test]
    fn login_refresh_result_serializes_extended_ticket_shape_like_cpp() {
        let body = serde_json::to_string(&LoginRefreshResult {
            login_ticket_expiry: Some(1_700_000_600),
            is_expired: None,
        })
        .unwrap();

        assert_eq!(body, r#"{"login_ticket_expiry":1700000600}"#);
    }

    #[test]
    fn login_refresh_result_serializes_expired_ticket_shape_like_cpp() {
        let body = serde_json::to_string(&LoginRefreshResult {
            login_ticket_expiry: None,
            is_expired: Some(true),
        })
        .unwrap();

        assert_eq!(body, r#"{"is_expired":true}"#);
    }
}

fn hex_decode(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .filter_map(|i| {
            hex.get(i..i + 2)
                .and_then(|s| u8::from_str_radix(s, 16).ok())
        })
        .collect()
}
