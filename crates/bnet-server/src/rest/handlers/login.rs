//! Login packets.
//!
//! Separated from handlers.rs under #693.

use super::*;

#[derive(serde::Deserialize)]
struct BotLoginRequest {
    username: Option<String>,
    #[serde(rename = "A")]
    public_a: Option<String>,
    #[serde(rename = "M1")]
    client_m1: Option<String>,
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
        ("POST", "/bnetserver/login/") => post_login(state, headers, body, connection_state).await,
        ("POST", "/bnetserver/login/srp/") => {
            post_login_srp_challenge(state, body, connection_state).await
        }
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

    HttpResponse {
        status_code: 200,
        status_text: "OK",
        headers: login_form_headers_like_cpp(),
        body: json,
    }
}

/// POST /bnetserver/login/ — Authenticate with credentials (direct or SRP M1).
async fn post_login(
    state: &AppState,
    headers: &HashMap<String, String>,
    body: Option<&[u8]>,
    connection_state: &mut RestConnectionState,
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

    // SRP challenge-response flow (client sends A and M1)
    if let (Some(a_hex), Some(m1_hex)) = (client_a, client_m1) {
        if let Some(session) = connection_state.bnet_srp.take() {
            let a_bytes = hex_decode(&a_hex);
            let m1_bytes = hex_decode(&m1_hex);
            if let Some(proof) = session.srp.verify_client_evidence(&a_bytes, &m1_bytes) {
                let m2_hex = hex_encode(&proof.server_evidence.to_bytes_be());
                return match create_login_ticket(state, session.account_id).await {
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

    let email_normalized = utf8_to_upper_only_latin_like_cpp(&email);
    let username = srp_username(&email_normalized);

    // Query account
    let mut stmt = state
        .login_db
        .prepare(LoginStatements::SEL_BNET_AUTHENTICATION);
    stmt.set_string(0, &email_normalized);
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
        utf8_to_upper_only_latin_like_cpp(&password)
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
            &email_normalized,
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
    body: Option<&[u8]>,
    connection_state: &mut RestConnectionState,
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

    let email_normalized = utf8_to_upper_only_latin_like_cpp(&email);
    let username = srp_username(&email_normalized);

    let mut stmt = state
        .login_db
        .prepare(LoginStatements::SEL_BNET_CHECK_PASSWORD_BY_EMAIL);
    stmt.set_string(0, &email_normalized);
    let result = match state.login_db.query(&stmt).await {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("DB error during SRP challenge: {e}");
            return json_error_response(500, "Internal Server Error", "Internal error");
        }
    };

    if result.is_empty() {
        return srp_challenge_missing_account_response_like_cpp();
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
    let challenge = srp.challenge(&email_normalized);

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

    connection_state.bnet_srp = Some(BnetRestSrpState { srp, account_id });

    let body = serde_json::to_string(&response).unwrap_or_default();

    HttpResponse {
        status_code: 200,
        status_text: "OK",
        headers: srp_challenge_headers_like_cpp(),
        body,
    }
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

fn find_input(form: &LoginForm, id: &str) -> Option<String> {
    form.inputs
        .iter()
        .find(|i| i.input_id == id)
        .map(|i| i.value.clone())
}
