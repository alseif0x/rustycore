//! Tickets packets.
//!
//! Separated from handlers.rs under #693.

use super::*;

#[derive(Default)]
pub struct RestConnectionState {
    pub(super) bot_srp: Option<BotSrpState>,
    pub(super) bnet_srp: Option<BnetRestSrpState>,
}

pub(super) fn login_form_headers_like_cpp() -> Vec<(&'static str, String)> {
    vec![("Content-Type", "application/json;charset=utf-8".to_string())]
}

/// GET /bnetserver/gameAccounts/
pub(super) async fn get_game_accounts(
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

/// POST /bnetserver/refreshLoginTicket/
pub(super) async fn refresh_login_ticket(
    state: &AppState,
    headers: &HashMap<String, String>,
) -> HttpResponse {
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

pub(super) fn extract_auth_ticket(headers: &HashMap<String, String>) -> Option<String> {
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

pub(super) fn make_login_ticket() -> String {
    use rand::Rng;
    let mut bytes = [0u8; 20];
    rand::thread_rng().fill(&mut bytes);
    format!("TC-{}", hex_encode(&bytes))
}

pub(super) async fn create_login_ticket(
    state: &AppState,
    account_id: u32,
) -> anyhow::Result<String> {
    let ticket = make_login_ticket();
    store_login_ticket(state, account_id, &ticket).await?;

    tracing::info!("Login ticket created for account_id={account_id}: {ticket}");
    Ok(ticket)
}

pub(super) async fn store_login_ticket(
    state: &AppState,
    account_id: u32,
    ticket: &str,
) -> anyhow::Result<()> {
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

pub(super) fn unix_timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
