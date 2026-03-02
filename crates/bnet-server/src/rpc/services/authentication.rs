//! Authentication service handler (hash 0x0DECFC01).

use anyhow::Result;
use prost::Message;
use wow_database::LoginStatements;
use wow_proto::bgs::protocol::EntityId;
use wow_proto::bgs::protocol::authentication::v1::*;
use wow_proto::bgs::protocol::challenge::v1::ChallengeExternalRequest;
use wow_proto::service_hash;

use crate::rpc::session::RpcSession;
use tokio::io::{AsyncRead, AsyncWrite};
use crate::state::{AccountInfo, GameAccountInfo, LastPlayedCharInfo};
use std::collections::HashMap;

pub async fn handle<S: AsyncRead + AsyncWrite + Unpin>(
    session: &mut RpcSession<S>,
    method_id: u32,
    payload: &[u8],
) -> Result<Option<Vec<u8>>> {
    match method_id {
        1 => handle_logon(session, payload).await,
        7 => handle_verify_web_credentials(session, payload).await,
        8 => handle_generate_web_credentials(session, payload).await,
        _ => {
            tracing::warn!("AuthenticationService: unknown method {method_id}");
            Ok(None)
        }
    }
}

/// Method 1: Logon — validates client info and sends web auth URL challenge.
async fn handle_logon<S: AsyncRead + AsyncWrite + Unpin>(
    session: &mut RpcSession<S>,
    payload: &[u8],
) -> Result<Option<Vec<u8>>> {
    let request = LogonRequest::decode(payload)?;

    // Validate program
    let program = request.program.as_deref().unwrap_or("");
    if program != "WoW" {
        tracing::warn!("Invalid program: {program}");
        return Ok(None);
    }

    // Validate platform
    let platform = request.platform.as_deref().unwrap_or("");
    if !matches!(platform, "Win" | "Wn64" | "Mc64") {
        tracing::warn!("Invalid platform: {platform}");
        return Ok(None);
    }

    // Store session info
    session.locale = request.locale.clone().unwrap_or_default();
    session.os = platform.to_string();
    session.build = request.application_version.unwrap_or(0) as u32;

    // Extract timezone offset from DeviceId JSON
    if let Some(device_id) = &request.device_id {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(device_id) {
            session.timezone_offset = json.get("UTCO")
                .and_then(|v| v.as_i64())
                .unwrap_or(0) as i32;
        }
    }

    tracing::info!(
        "AuthenticationService: Logon from {platform}/{} locale={} build={}",
        request.locale.as_deref().unwrap_or("?"),
        session.locale,
        session.build,
    );

    // Send ChallengeExternalRequest to client with web auth URL
    let state = session.state();
    let web_url = format!(
        "https://{}:{}/bnetserver/login/",
        state.external_address, state.rest_port
    );

    let challenge = ChallengeExternalRequest {
        payload_type: Some("web_auth_url".to_string()),
        payload: Some(web_url.into_bytes()),
        ..Default::default()
    };

    session.send_request(
        service_hash::CHALLENGE_LISTENER,
        3, // OnExternalChallenge
        &challenge.encode_to_vec(),
    ).await?;

    Ok(None)
}

/// Method 7: VerifyWebCredentials — validates login ticket and sends LogonResult.
async fn handle_verify_web_credentials<S: AsyncRead + AsyncWrite + Unpin>(
    session: &mut RpcSession<S>,
    payload: &[u8],
) -> Result<Option<Vec<u8>>> {
    let request = VerifyWebCredentialsRequest::decode(payload)?;

    let ticket = request.web_credentials
        .map(|b| String::from_utf8_lossy(&b).to_string())
        .unwrap_or_default();

    if ticket.is_empty() {
        return send_logon_error(session, 3).await;
    }

    // Query account + game account info using the login ticket.
    // SEL_BNET_ACCOUNT_INFO is a multi-row JOIN:
    //   Columns 0-7: BNet account (same across all rows)
    //     0: ba.id, 1: UPPER(ba.email), 2: ba.locked, 3: ba.lock_country,
    //     4: ba.last_ip, 5: ba.LoginTicketExpiry, 6: is_banned, 7: is_permanently_banned
    //   Columns 8-12: Game account (one row per game account)
    //     8: a.id, 9: a.username, 10: ab.unbandate, 11: ab.permanently_banned, 12: aa.SecurityLevel
    let state = session.state();
    let mut stmt = state.login_db.prepare(LoginStatements::SEL_BNET_ACCOUNT_INFO);
    stmt.set_string(0, &ticket);
    let mut result = state.login_db.query(&stmt).await?;

    if result.is_empty() {
        tracing::debug!("VerifyWebCredentials: invalid ticket");
        return send_logon_error(session, 3).await;
    }

    // Parse BNet account info from first row
    let account_id: u32 = result.read(0);
    let login: String = result.read(1);
    let is_locked_to_ip: bool = result.try_read::<bool>(2).unwrap_or(false);
    let lock_country: String = result.try_read::<String>(3).unwrap_or_default();
    let last_ip: String = result.try_read::<String>(4).unwrap_or_default();
    // Column 5 is LoginTicketExpiry — not needed here
    let is_banned: bool = result.try_read::<bool>(6).unwrap_or(false);
    let is_permanently_banned: bool = result.try_read::<bool>(7).unwrap_or(false);

    // Parse game accounts from all rows (columns 8-12)
    let mut game_accounts = HashMap::new();
    loop {
        let ga_id: u32 = result.try_read::<u32>(8).unwrap_or(0);
        if ga_id != 0 {
            let ga_name: String = result.try_read::<String>(9).unwrap_or_default();
            let ga_unban: u64 = result.try_read::<u64>(10).unwrap_or(0);
            let ga_perma_banned: bool = result.try_read::<bool>(11).unwrap_or(false);
            let ga_security: u8 = result.try_read::<u8>(12).unwrap_or(0);

            // Generate display name: "email#N" → "WoWN"
            let display_name = ga_name
                .rsplit_once('#')
                .map(|(_, n)| format!("WoW{n}"))
                .unwrap_or_else(|| ga_name.clone());

            game_accounts.insert(ga_id, GameAccountInfo {
                id: ga_id,
                name: ga_name,
                display_name,
                unban_date: ga_unban,
                is_permanently_banned: ga_perma_banned,
                is_banned: ga_perma_banned || ga_unban > 0,
                security_level: ga_security,
                char_counts: HashMap::new(),
                last_played_chars: HashMap::new(),
            });
        }

        if !result.next_row() {
            break;
        }
    }

    // Load character counts: acctid(0), numchars(1), realm_id(2), Region(3), Battlegroup(4)
    let mut stmt = state.login_db.prepare(LoginStatements::SEL_BNET_CHARACTER_COUNTS_BY_BNET_ID);
    stmt.set_u32(0, account_id);
    let mut cc_result = state.login_db.query(&stmt).await?;

    if !cc_result.is_empty() {
        loop {
            let ga_id: u32 = cc_result.read(0);
            let count: u8 = cc_result.read(1);
            let realm_id: u32 = cc_result.read(2);

            if let Some(ga) = game_accounts.get_mut(&ga_id) {
                ga.char_counts.insert(realm_id, count);
            }

            if !cc_result.next_row() {
                break;
            }
        }
    }

    // Load last played characters:
    //   accountId(0), region(1), battlegroup(2), realmId(3),
    //   characterName(4), characterGUID(5), lastPlayedTime(6)
    let mut stmt = state.login_db.prepare(LoginStatements::SEL_BNET_LAST_PLAYER_CHARACTERS);
    stmt.set_u32(0, account_id);
    let mut lp_result = state.login_db.query(&stmt).await?;

    if !lp_result.is_empty() {
        loop {
            let ga_id: u32 = lp_result.read(0);
            let region: u8 = lp_result.try_read::<u8>(1).unwrap_or(0);
            let battlegroup: u8 = lp_result.try_read::<u8>(2).unwrap_or(0);
            let realm_id: u32 = lp_result.try_read::<u32>(3).unwrap_or(0);
            let char_name: String = lp_result.try_read::<String>(4).unwrap_or_default();
            let char_guid: u64 = lp_result.try_read::<u64>(5).unwrap_or(0);
            let last_played: u64 = lp_result.try_read::<u64>(6).unwrap_or(0);

            let sub_region = format!("{region}-{battlegroup}-0");

            if let Some(ga) = game_accounts.get_mut(&ga_id) {
                ga.last_played_chars.insert(sub_region, LastPlayedCharInfo {
                    realm_address: realm_id,
                    character_name: char_name,
                    character_guid: char_guid,
                    last_played_time: last_played,
                });
            }

            if !lp_result.next_row() {
                break;
            }
        }
    }

    // Check IP lock
    if is_locked_to_ip && last_ip != session.addr().ip().to_string() {
        tracing::debug!("Account {account_id} is locked to IP {last_ip}");
        return send_logon_error(session, 12).await;
    }

    // Check ban
    if is_banned {
        tracing::debug!("Account {account_id} is banned");
        return send_logon_error(session, 3).await;
    }

    // Build LogonResult
    let mut game_account_ids = Vec::new();
    for ga in game_accounts.values() {
        game_account_ids.push(EntityId {
            high: 0x0200_0002_0057_6F57, // "WoW" encoded in high bits
            low: u64::from(ga.id),
        });
    }

    // Generate random 64-byte session key
    let mut session_key = vec![0u8; 64];
    rand::Rng::fill(&mut rand::thread_rng(), session_key.as_mut_slice());

    let logon_result = LogonResult {
        error_code: 0,
        account_id: Some(EntityId {
            high: 0x0100_0000_0000_0000,
            low: u64::from(account_id),
        }),
        game_account_id: game_account_ids,
        session_key: Some(session_key),
        geoip_country: if session.ip_country.is_empty() {
            None
        } else {
            Some(session.ip_country.clone())
        },
        ..Default::default()
    };

    // Store account info in session
    session.authed = true;
    session.account_info = Some(AccountInfo {
        id: account_id,
        login,
        is_locked_to_ip,
        lock_country,
        last_ip,
        failed_logins: 0,
        is_banned,
        is_permanently_banned,
        game_accounts,
    });

    // Send LogonResult to AuthenticationListener method 5
    session.send_request(
        service_hash::AUTHENTICATION_LISTENER,
        5, // OnLogonComplete
        &logon_result.encode_to_vec(),
    ).await?;

    tracing::info!("Account {account_id} authenticated successfully");
    Ok(None)
}

/// Method 8: GenerateWebCredentials
async fn handle_generate_web_credentials<S: AsyncRead + AsyncWrite + Unpin>(
    session: &mut RpcSession<S>,
    payload: &[u8],
) -> Result<Option<Vec<u8>>> {
    let _request = GenerateWebCredentialsRequest::decode(payload)?;

    if !session.authed {
        return Ok(None);
    }

    let account = session.account_info.as_ref()
        .ok_or_else(|| anyhow::anyhow!("No account info"))?;

    // Query existing login ticket by account ID
    let mut stmt = session.state().login_db.prepare(LoginStatements::SEL_BNET_EXISTING_AUTHENTICATION_BY_ID);
    stmt.set_u32(0, account.id);
    let result = session.state().login_db.query(&stmt).await?;

    if result.is_empty() {
        return Ok(None);
    }

    let ticket: String = result.read(0);
    let response = GenerateWebCredentialsResponse {
        web_credentials: Some(ticket.into_bytes()),
    };
    Ok(Some(response.encode_to_vec()))
}

/// Send a LogonResult with an error code to the client.
async fn send_logon_error<S: AsyncRead + AsyncWrite + Unpin>(session: &mut RpcSession<S>, error_code: u32) -> Result<Option<Vec<u8>>> {
    let logon_result = LogonResult {
        error_code,
        ..Default::default()
    };

    session.send_request(
        service_hash::AUTHENTICATION_LISTENER,
        5,
        &logon_result.encode_to_vec(),
    ).await?;

    Ok(None)
}
