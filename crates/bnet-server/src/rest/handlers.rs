//! REST API handler implementations.

use std::collections::HashMap;
use wow_crypto::{BnetSrp6, SrpHashFunction, SrpVersion, srp_username};
use wow_database::LoginStatements;

use crate::state::{AppState, RestSessionState};
use super::HttpResponse;
use super::types::*;

/// Route an HTTP request to the appropriate handler.
pub async fn route(
    state: &AppState,
    method: &str,
    path: &str,
    headers: &HashMap<String, String>,
    body: Option<&[u8]>,
) -> HttpResponse {
    match (method, path) {
        ("GET", "/bnetserver/login/") => get_form(state),
        ("POST", "/bnetserver/login/") => post_login(state, headers, body).await,
        ("POST", "/bnetserver/login/srp/") => {
            post_login_srp_challenge(state, headers, body).await
        }
        ("GET", "/bnetserver/gameAccounts/") => get_game_accounts(state, headers).await,
        ("GET", "/bnetserver/portal/") => get_portal(state, headers),
        ("POST", "/bnetserver/refreshLoginTicket/") => {
            refresh_login_ticket(state, headers).await
        }
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
            (
                "Content-Type",
                "application/json;charset=utf-8".to_string(),
            ),
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
        version, salt.len(), verifier.len(),
        &salt[..salt.len().min(8)],
        &verifier[..verifier.len().min(8)],
    );
    let srp = BnetSrp6::new(version, SrpHashFunction::Sha256, &username, &salt, &verifier);
    tracing::info!("SRP: checking credentials for user={}, password_len={}", &username[..username.len().min(16)], password_for_srp.len());
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
        increment_failed_logins(state, &email_upper).await;
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
    let srp = BnetSrp6::new(version, SrpHashFunction::Sha256, &username, &salt, &verifier);
    let challenge = srp.challenge(&email_upper);

    let session_id = extract_session_id(headers).unwrap_or_else(generate_session_id);
    state
        .rest_sessions
        .insert(session_id.clone(), RestSessionState { srp: Some(srp), account_id });

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
    let cookie = format!(
        "JSESSIONID={session_id}; Path=/bnetserver; Secure; HttpOnly; SameSite=None"
    );

    HttpResponse {
        status_code: 200,
        status_text: "OK",
        headers: vec![
            ("Set-Cookie", cookie),
            (
                "Content-Type",
                "application/json;charset=utf-8".to_string(),
            ),
        ],
        body,
    }
}

/// GET /bnetserver/gameAccounts/
async fn get_game_accounts(
    state: &AppState,
    headers: &HashMap<String, String>,
) -> HttpResponse {
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
async fn refresh_login_ticket(
    state: &AppState,
    headers: &HashMap<String, String>,
) -> HttpResponse {
    let Some(ticket) = extract_auth_ticket(headers) else {
        return json_error_response(401, "Unauthorized", "Missing ticket");
    };

    let expiry = unix_timestamp() + state.ticket_duration;
    let mut stmt = state
        .login_db
        .prepare(LoginStatements::UPD_BNET_AUTHENTICATION);
    stmt.set_u64(0, expiry);
    stmt.set_string(1, &ticket);
    match state.login_db.execute(&stmt).await {
        Ok(_) => json_response(serde_json::json!({"login_ticket": ticket})),
        Err(e) => {
            tracing::error!("Failed to refresh ticket: {e}");
            json_error_response(500, "Internal Server Error", "Internal error")
        }
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn json_response<T: serde::Serialize>(value: T) -> HttpResponse {
    let body = serde_json::to_string(&value).unwrap_or_default();
    HttpResponse {
        status_code: 200,
        status_text: "OK",
        headers: vec![(
            "Content-Type",
            "application/json;charset=utf-8".to_string(),
        )],
        body,
    }
}

fn json_error_response(
    status_code: u16,
    status_text: &'static str,
    error: &str,
) -> HttpResponse {
    let body = serde_json::to_string(&serde_json::json!({"error": error})).unwrap_or_default();
    HttpResponse {
        status_code,
        status_text,
        headers: vec![(
            "Content-Type",
            "application/json;charset=utf-8".to_string(),
        )],
        body,
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
    headers
        .get("authorization")
        .and_then(|auth| auth.strip_prefix("Basic "))
        .map(|s| s.to_string())
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
    let expiry = unix_timestamp() + state.ticket_duration;

    // UPD_BNET_AUTHENTICATION: SET LoginTicket = ?, LoginTicketExpiry = ? WHERE id = ?
    let mut stmt = state
        .login_db
        .prepare(LoginStatements::UPD_BNET_AUTHENTICATION);
    stmt.set_string(0, &ticket);
    stmt.set_u64(1, expiry);
    stmt.set_u32(2, account_id);
    state.login_db.execute(&stmt).await?;

    tracing::info!("Login ticket created for account_id={account_id}: {ticket}");
    Ok(ticket)
}

async fn increment_failed_logins(state: &AppState, email: &str) {
    let mut stmt = state
        .login_db
        .prepare(LoginStatements::UPD_BNET_FAILED_LOGINS);
    stmt.set_string(0, email);
    if let Err(e) = state.login_db.execute(&stmt).await {
        tracing::warn!("Failed to increment failed logins for {email}: {e}");
    }
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

fn hex_decode(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .filter_map(|i| hex.get(i..i + 2).and_then(|s| u8::from_str_radix(s, 16).ok()))
        .collect()
}
