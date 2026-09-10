//! Wrong password packets.
//!
//! Separated from handlers.rs under #693.

use super::*;

pub(super) async fn apply_wrong_password_policy_like_cpp(
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

pub(super) fn wrong_password_remote_ip_from_headers_like_cpp(
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
