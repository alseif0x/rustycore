//! GameUtilities service handler (hash 0x3FC1274D).

use anyhow::{Result, bail};
use prost::Message;
use std::collections::HashMap;
use wow_database::LoginStatements;
use wow_proto::bgs::protocol::{Attribute, Variant};
use wow_proto::bgs::protocol::game_utilities::v1::*;

use crate::rpc::session::RpcSession;
use tokio::io::{AsyncRead, AsyncWrite};

pub async fn handle<S: AsyncRead + AsyncWrite + Unpin>(
    session: &mut RpcSession<S>,
    method_id: u32,
    payload: &[u8],
) -> Result<Option<Vec<u8>>> {
    match method_id {
        1 => handle_process_client_request(session, payload).await,
        10 => handle_get_all_values_for_attribute(session, payload).await,
        _ => {
            tracing::warn!("GameUtilitiesService: unknown method {method_id}");
            Ok(None)
        }
    }
}

/// Strip the last `_suffix` from a string (e.g. `Command_Foo_v1_wotlk1` → `Command_Foo_v1`).
/// Matches C# `removeSuffix` which uses `str.LastIndexOf('_')`.
fn remove_suffix(s: &str) -> &str {
    match s.rfind('_') {
        Some(pos) => &s[..pos],
        None => s,
    }
}

/// Find an attribute whose name starts with `prefix` (after suffix removal).
fn find_attr<'a>(attrs: &'a [Attribute], prefix: &str) -> Option<&'a Attribute> {
    attrs.iter().find(|a| a.name.starts_with(prefix))
}

/// Method 1: ProcessClientRequest — dispatches based on Command_* attribute.
async fn handle_process_client_request<S: AsyncRead + AsyncWrite + Unpin>(
    session: &mut RpcSession<S>,
    payload: &[u8],
) -> Result<Option<Vec<u8>>> {
    let request = ClientRequest::decode(payload)?;

    // Find the command attribute and normalize its name (strip client-specific suffix like _wotlk1).
    let command_attr = request.attribute.iter()
        .find(|a| a.name.starts_with("Command_"));

    let command = command_attr.map(|a| remove_suffix(&a.name));

    tracing::debug!("GameUtilities command: {command:?}");

    match command {
        Some("Command_RealmListTicketRequest_v1") => {
            get_realm_list_ticket(session, &request).await
        }
        Some("Command_LastCharPlayedRequest_v1") => {
            get_last_char_played(session, &request).await
        }
        Some("Command_RealmListRequest_v1") => {
            get_realm_list(session, &request).await
        }
        Some("Command_RealmJoinRequest_v1") => {
            join_realm(session, &request).await
        }
        _ => {
            tracing::warn!("Unknown GameUtilities command: {command:?} (raw={:?})",
                command_attr.map(|a| a.name.as_str()));
            bail!("Unknown command")
        }
    }
}

/// RealmListTicketRequest — validates identity, stores client secret.
async fn get_realm_list_ticket<S: AsyncRead + AsyncWrite + Unpin>(
    session: &mut RpcSession<S>,
    request: &ClientRequest,
) -> Result<Option<Vec<u8>>> {
    if !session.authed {
        bail!("Not authenticated");
    }

    // Extract Param_Identity (prefixed JSON: "JSONRealmListTicketIdentity:{...}\0")
    let identity_attr = request.attribute.iter()
        .find(|a| a.name == "Param_Identity");
    if let Some(attr) = identity_attr {
        if let Some(blob) = &attr.value.blob_value {
            let text = String::from_utf8_lossy(blob);
            let json_str = text.trim_end_matches('\0');
            // Strip any "JSON*:" prefix
            let json_str = json_str.find(':')
                .map(|pos| &json_str[pos + 1..])
                .unwrap_or(json_str);
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_str) {
                let _game_account_id = json.get("gameAccountID")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
            }
        }
    }

    // Extract Param_ClientInfo (prefixed JSON: "JSONRealmListTicketClientInformation:{...}\0")
    let client_info_attr = request.attribute.iter()
        .find(|a| a.name == "Param_ClientInfo");
    if let Some(attr) = client_info_attr {
        if let Some(blob) = &attr.value.blob_value {
            let text = String::from_utf8_lossy(blob);
            // Strip the "JSONRealmListTicketClientInformation:" prefix and trailing null
            let json_str = text.trim_end_matches('\0');
            let json_str = json_str.strip_prefix("JSONRealmListTicketClientInformation:")
                .unwrap_or(json_str);
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_str) {
                if let Some(secret) = json.get("info").and_then(|i| i.get("secret")) {
                    if let Some(arr) = secret.as_array() {
                        session.client_secret = arr.iter()
                            .filter_map(|v| v.as_i64().map(|n| n as u8))
                            .collect();
                        tracing::info!("Extracted client_secret: {} bytes", session.client_secret.len());
                    }
                }
            } else {
                tracing::warn!("Param_ClientInfo: failed to parse JSON");
            }
        }
    }

    // Update last login info: SET last_ip=?, locale=?, os=? WHERE id=?
    if let Some(account) = &session.account_info {
        let mut stmt = session.state().login_db.prepare(LoginStatements::UPD_BNET_LAST_LOGIN_INFO);
        stmt.set_string(0, &session.addr().ip().to_string());
        stmt.set_string(1, &session.locale);
        stmt.set_string(2, &session.os);
        stmt.set_u32(3, account.id);
        let _ = session.state().login_db.execute(&stmt).await;
    }

    // Return realm list ticket
    let response = ClientResponse {
        attribute: vec![
            make_blob_attribute("Param_RealmListTicket", b"AuthRealmListTicket"),
        ],
    };

    Ok(Some(response.encode_to_vec()))
}

/// LastCharPlayedRequest — returns last played character info.
async fn get_last_char_played<S: AsyncRead + AsyncWrite + Unpin>(
    session: &mut RpcSession<S>,
    request: &ClientRequest,
) -> Result<Option<Vec<u8>>> {
    if !session.authed {
        bail!("Not authenticated");
    }

    // The sub-region value is stored on the command attribute itself
    let sub_region = find_attr(&request.attribute, "Command_LastCharPlayedRequest_v1")
        .and_then(|a| a.value.string_value.as_deref())
        .unwrap_or("");

    let account = session.account_info.as_ref()
        .ok_or_else(|| anyhow::anyhow!("No account info"))?;

    let mut response_attrs = Vec::new();

    // Find last played character for this sub-region across game accounts
    for ga in account.game_accounts.values() {
        if let Some(lpc) = ga.last_played_chars.get(sub_region) {
            // Get realm entry JSON
            let realm_mgr = session.state().realm_mgr.read();
            if let Some(realm) = realm_mgr.realms.get(&lpc.realm_address) {
                let realm_json = realm_mgr.get_realm_entry_json(realm, Some(session.addr().ip()));
                response_attrs.push(make_blob_attribute("Param_RealmEntry", &realm_json));
            }

            response_attrs.push(make_string_attribute("Param_CharacterName", &lpc.character_name));
            response_attrs.push(make_uint_attribute("Param_CharacterGUID", lpc.character_guid));
            response_attrs.push(make_uint_attribute("Param_LastPlayedTime", lpc.last_played_time));
            break;
        }
    }

    Ok(Some(ClientResponse { attribute: response_attrs }.encode_to_vec()))
}

/// RealmListRequest — returns compressed JSON realm list.
async fn get_realm_list<S: AsyncRead + AsyncWrite + Unpin>(
    session: &mut RpcSession<S>,
    request: &ClientRequest,
) -> Result<Option<Vec<u8>>> {
    if !session.authed {
        bail!("Not authenticated");
    }

    let sub_region = find_attr(&request.attribute, "Command_RealmListRequest_v1")
        .and_then(|a| a.value.string_value.as_deref())
        .unwrap_or("");

    let account = session.account_info.as_ref()
        .ok_or_else(|| anyhow::anyhow!("No account info"))?;

    // Aggregate character counts across game accounts
    let mut char_counts: HashMap<u32, u8> = HashMap::new();
    for ga in account.game_accounts.values() {
        for (&realm_id, &count) in &ga.char_counts {
            *char_counts.entry(realm_id).or_default() += count;
        }
    }

    let realm_mgr = session.state().realm_mgr.read();
    let realm_builds: Vec<(u32, u32)> = realm_mgr.realms.values().map(|r| (r.id, r.build)).collect();
    tracing::info!("RealmListRequest: session.build={}, sub_region={sub_region:?}, realm_builds={realm_builds:?}", session.build);
    let (realm_data, count_data) = realm_mgr.get_realm_list_json(
        session.build,
        sub_region,
        &char_counts,
    );

    let response = ClientResponse {
        attribute: vec![
            make_blob_attribute("Param_RealmList", &realm_data),
            make_blob_attribute("Param_CharacterCountList", &count_data),
        ],
    };

    Ok(Some(response.encode_to_vec()))
}

/// RealmJoinRequest — generates join ticket for connecting to a realm.
async fn join_realm<S: AsyncRead + AsyncWrite + Unpin>(
    session: &mut RpcSession<S>,
    request: &ClientRequest,
) -> Result<Option<Vec<u8>>> {
    if !session.authed {
        bail!("Not authenticated");
    }

    let account = session.account_info.as_ref()
        .ok_or_else(|| anyhow::anyhow!("No account info"))?;

    // Extract realm address from attribute
    let realm_address = request.attribute.iter()
        .find(|a| a.name == "Param_RealmAddress")
        .and_then(|a| a.value.uint_value)
        .unwrap_or(0) as u32;

    // Scope the realm_mgr guard — must drop before any .await
    let (server_addresses, realm_name) = {
        let realm_mgr = session.state().realm_mgr.read();
        let realm = realm_mgr.realms.get(&realm_address)
            .ok_or_else(|| anyhow::anyhow!("Realm not found"))?;

        // C# check: reject if offline or build mismatch
        if (realm.flag & 0x02) != 0 || realm.build != session.build {
            bail!("Realm offline or version mismatch");
        }

        (realm_mgr.get_realm_entry_json(realm, Some(session.addr().ip())), realm.name.clone())
    };

    // Generate 32-byte server secret
    let mut server_secret = vec![0u8; 32];
    rand::Rng::fill(&mut rand::thread_rng(), server_secret.as_mut_slice());

    // Combine client + server secrets for session key
    tracing::info!("join_realm: client_secret={} bytes, server_secret={} bytes",
        session.client_secret.len(), server_secret.len());
    let mut combined = session.client_secret.clone();
    combined.extend_from_slice(&server_secret);
    tracing::info!("join_realm: combined session key = {} bytes", combined.len());

    // Store session key in DB as raw bytes (64-byte BLOB), matching C# SetBytes().
    // C# params: [0]=keyData(bytes), [1]=last_ip, [2]=locale(u8), [3]=os, [4]=timezone_offset(i16), [5]=username
    let ga_username = account.game_accounts.values().next()
        .map(|ga| ga.name.clone())
        .unwrap_or_default();

    let locale_id = locale_string_to_id(&session.locale);

    let mut stmt = session.state().login_db.prepare(LoginStatements::UPD_BNET_GAME_ACCOUNT_LOGIN_INFO);
    stmt.set_bytes(0, combined);
    stmt.set_string(1, &session.addr().ip().to_string());
    stmt.set_u8(2, locale_id);
    stmt.set_string(3, &session.os);
    stmt.set_i16(4, session.timezone_offset as i16);
    stmt.set_string(5, &ga_username);
    let _ = session.state().login_db.execute(&stmt).await;

    // Build response — ticket is game account name (e.g. "2#1"), sent as blob (matching C#)
    let response = ClientResponse {
        attribute: vec![
            make_blob_attribute("Param_RealmJoinTicket", ga_username.as_bytes()),
            make_blob_attribute("Param_ServerAddresses", &server_addresses),
            make_blob_attribute("Param_JoinSecret", &server_secret),
        ],
    };

    tracing::info!("Account {} joining realm {realm_name}", account.id);
    Ok(Some(response.encode_to_vec()))
}

/// Method 10: GetAllValuesForAttribute — returns sub-region list.
async fn handle_get_all_values_for_attribute<S: AsyncRead + AsyncWrite + Unpin>(
    session: &mut RpcSession<S>,
    payload: &[u8],
) -> Result<Option<Vec<u8>>> {
    let request = GetAllValuesForAttributeRequest::decode(payload)?;

    let key = request.attribute_key.as_deref().unwrap_or("");

    if key.contains("Command_RealmListRequest_v1") {
        let realm_mgr = session.state().realm_mgr.read();
        let values: Vec<Variant> = realm_mgr.sub_regions.iter()
            .map(|sr| Variant {
                string_value: Some(sr.clone()),
                ..Default::default()
            })
            .collect();

        let response = GetAllValuesForAttributeResponse {
            attribute_value: values,
        };
        Ok(Some(response.encode_to_vec()))
    } else {
        Ok(Some(GetAllValuesForAttributeResponse::default().encode_to_vec()))
    }
}

// ── Locale conversion ─────────────────────────────────────────────────────

/// Convert locale string (e.g. "esES") to C# Locale enum value.
/// Matches C# `SharedConst.Locale` enum.
fn locale_string_to_id(locale: &str) -> u8 {
    match locale {
        "enUS" => 0,
        "koKR" => 1,
        "frFR" => 2,
        "deDE" => 3,
        "zhCN" => 4,
        "zhTW" => 5,
        "esES" => 6,
        "esMX" => 7,
        "ruRU" => 8,
        "ptBR" => 10,
        "itIT" => 11,
        _ => 0, // default to enUS
    }
}

// ── Attribute helpers ───────────────────────────────────────────────────────

fn make_blob_attribute(name: &str, value: &[u8]) -> Attribute {
    Attribute {
        name: name.to_string(),
        value: Variant {
            blob_value: Some(value.to_vec()),
            ..Default::default()
        },
    }
}

fn make_string_attribute(name: &str, value: &str) -> Attribute {
    Attribute {
        name: name.to_string(),
        value: Variant {
            string_value: Some(value.to_string()),
            ..Default::default()
        },
    }
}

fn make_uint_attribute(name: &str, value: u64) -> Attribute {
    Attribute {
        name: name.to_string(),
        value: Variant {
            uint_value: Some(value),
            ..Default::default()
        },
    }
}
