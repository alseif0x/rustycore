//! GameUtilities service handler (hash 0x3FC1274D).

use anyhow::{Result, bail};
use prost::Message;
use wow_database::{LoginStatements, PreparedStatement};
use wow_proto::bgs::protocol::game_utilities::v1::*;
use wow_proto::bgs::protocol::{Attribute, Variant};
use wow_proto::status;

use crate::rpc::session::{RpcSession, RpcStatusError};
use crate::state::{AccountInfo, GameAccountInfo, LastPlayedCharInfo};
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
    let command_attr = request
        .attribute
        .iter()
        .find(|a| a.name.starts_with("Command_"));

    let command = command_attr.map(|a| remove_suffix(&a.name));

    tracing::debug!("GameUtilities command: {command:?}");

    let command = process_client_request_command_like_cpp(session.authed, command)?;
    match command {
        "Command_RealmListTicketRequest_v1" => get_realm_list_ticket(session, &request).await,
        "Command_LastCharPlayedRequest_v1" => get_last_char_played(session, &request).await,
        "Command_RealmListRequest_v1" => get_realm_list(session, &request).await,
        "Command_RealmJoinRequest_v1" => join_realm(session, &request).await,
        _ => unreachable!("validated GameUtilities command"),
    }
}

fn process_client_request_command_like_cpp<'a>(
    authed: bool,
    command: Option<&'a str>,
) -> Result<&'a str> {
    if !authed {
        return Err(RpcStatusError::new(status::ERROR_DENIED).into());
    }

    let command =
        command.ok_or_else(|| RpcStatusError::new(status::ERROR_RPC_MALFORMED_REQUEST))?;

    match command {
        "Command_RealmListTicketRequest_v1"
        | "Command_LastCharPlayedRequest_v1"
        | "Command_RealmListRequest_v1"
        | "Command_RealmJoinRequest_v1" => Ok(command),
        _ => Err(RpcStatusError::new(status::ERROR_RPC_NOT_IMPLEMENTED).into()),
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

    let game_account_id = parse_realm_list_ticket_game_account_id_like_cpp(&request.attribute)
        .ok_or_else(|| RpcStatusError::new(status::ERROR_UTIL_SERVER_INVALID_IDENTITY_ARGS))?;

    let (is_permanently_banned, is_banned) = {
        let account = account_info_or_status_like_cpp(
            session.account_info.as_ref(),
            status::ERROR_UTIL_SERVER_INVALID_IDENTITY_ARGS,
        )?;
        let game_account = account
            .game_accounts
            .get(&game_account_id)
            .ok_or_else(|| RpcStatusError::new(status::ERROR_UTIL_SERVER_INVALID_IDENTITY_ARGS))?;
        (game_account.is_permanently_banned, game_account.is_banned)
    };

    session.selected_game_account_id = Some(game_account_id);

    if is_permanently_banned {
        return Err(RpcStatusError::new(status::ERROR_GAME_ACCOUNT_BANNED).into());
    }
    if is_banned {
        return Err(RpcStatusError::new(status::ERROR_GAME_ACCOUNT_SUSPENDED).into());
    }

    let Some(client_secret) = parse_realm_list_ticket_client_secret_like_cpp(&request.attribute)
    else {
        tracing::warn!("Param_ClientInfo: failed to parse valid 32-byte client secret");
        return Err(
            RpcStatusError::new(status::ERROR_WOW_SERVICES_DENIED_REALM_LIST_TICKET).into(),
        );
    };
    session.client_secret = client_secret;
    tracing::info!(
        "Extracted client_secret: {} bytes",
        session.client_secret.len()
    );

    // Update last login info: SET last_ip=?, locale=?, os=? WHERE id=?
    if let Some(account) = &session.account_info {
        let last_login_update = BnetLastLoginInfoUpdateLikeCpp {
            client_ip: session.addr().ip().to_string(),
            locale: locale_string_to_id_like_cpp(&session.locale),
            os: session.os.clone(),
            account_id: account.id,
        };
        let mut stmt = session
            .state()
            .login_db
            .prepare(LoginStatements::UPD_BNET_LAST_LOGIN_INFO);
        apply_bnet_last_login_info_update_like_cpp(&mut stmt, &last_login_update);
        let _ = session.state().login_db.execute(&stmt).await;
    }

    // Return realm list ticket
    let response = ClientResponse {
        attribute: vec![make_blob_attribute(
            "Param_RealmListTicket",
            b"AuthRealmListTicket",
        )],
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

    // The sub-region value is stored on the command attribute itself.
    let Some(command_attr) = find_attr(&request.attribute, "Command_LastCharPlayedRequest_v1")
    else {
        return Err(RpcStatusError::new(status::ERROR_UTIL_SERVER_UNKNOWN_REALM).into());
    };
    let sub_region = command_attr.value.string_value.as_deref().unwrap_or("");

    let account = account_info_or_status_like_cpp(
        session.account_info.as_ref(),
        status::ERROR_USER_SERVER_BAD_WOW_ACCOUNT,
    )?;
    let game_account = selected_game_account_like_cpp(account, session.selected_game_account_id)?;

    let mut response_attrs = Vec::new();

    if let Some(lpc) = game_account.last_played_chars.get(sub_region) {
        // Get realm entry JSON
        let realm_mgr = session.state().realm_mgr.read();
        let realm_json = realm_mgr.get_realm_entry_json_like_cpp(lpc.realm_address, session.build);
        if realm_json.is_empty() {
            return Err(RpcStatusError::new(
                status::ERROR_UTIL_SERVER_FAILED_TO_SERIALIZE_RESPONSE,
            )
            .into());
        }
        response_attrs = last_char_played_response_attributes_like_cpp(&realm_json, lpc);
    }

    Ok(Some(
        ClientResponse {
            attribute: response_attrs,
        }
        .encode_to_vec(),
    ))
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

    let account = account_info_or_status_like_cpp(
        session.account_info.as_ref(),
        status::ERROR_USER_SERVER_BAD_WOW_ACCOUNT,
    )?;
    let game_account = selected_game_account_like_cpp(account, session.selected_game_account_id)?;

    let realm_mgr = session.state().realm_mgr.read();
    let realm_builds: Vec<(u32, u32)> =
        realm_mgr.realms.values().map(|r| (r.id, r.build)).collect();
    tracing::info!(
        "RealmListRequest: session.build={}, sub_region={sub_region:?}, realm_builds={realm_builds:?}",
        session.build
    );
    let (realm_data, count_data) =
        realm_mgr.get_realm_list_json(session.build, sub_region, &game_account.char_counts);

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

    let account = account_info_or_status_like_cpp(
        session.account_info.as_ref(),
        status::ERROR_USER_SERVER_BAD_WOW_ACCOUNT,
    )?;
    let game_account = selected_game_account_like_cpp(account, session.selected_game_account_id)?;

    // Extract realm address from attribute
    let realm_address = request
        .attribute
        .iter()
        .find(|a| a.name == "Param_RealmAddress")
        .and_then(|a| a.value.uint_value)
        .ok_or_else(|| RpcStatusError::new(status::ERROR_WOW_SERVICES_INVALID_JOIN_TICKET))?
        as u32;

    // Scope the realm_mgr guard — must drop before any .await
    let (server_addresses, realm_name) = {
        let realm_mgr = session.state().realm_mgr.read();
        let prepared = realm_mgr
            .prepare_join_realm_like_cpp(realm_address, session.build, Some(session.addr().ip()))
            .map_err(|error| match error {
                crate::realm::JoinRealmPrepareErrorLikeCpp::UnknownRealm => {
                    RpcStatusError::new(status::ERROR_UTIL_SERVER_UNKNOWN_REALM)
                }
                crate::realm::JoinRealmPrepareErrorLikeCpp::UserServerNotPermittedOnRealm => {
                    RpcStatusError::new(status::ERROR_USER_SERVER_NOT_PERMITTED_ON_REALM)
                }
            })?;
        (prepared.server_addresses, prepared.realm_name)
    };

    // Generate 32-byte server secret
    let mut server_secret = vec![0u8; 32];
    rand::Rng::fill(&mut rand::thread_rng(), server_secret.as_mut_slice());

    // Combine client + server secrets for session key.
    //
    // TC builds a fixed std::array<uint8, 64> and stores it with setBinary():
    // first the 32-byte client secret, then the 32-byte server secret.
    tracing::info!(
        "join_realm: client_secret={} bytes, server_secret={} bytes",
        session.client_secret.len(),
        server_secret.len()
    );
    let combined = match bnet_session_key_data_like_cpp(&session.client_secret, &server_secret) {
        Some(key) => key,
        None => {
            tracing::warn!(
                "join_realm: rejecting invalid secret lengths client={} server={}",
                session.client_secret.len(),
                server_secret.len()
            );
            return Err(
                RpcStatusError::new(status::ERROR_WOW_SERVICES_DENIED_REALM_LIST_TICKET).into(),
            );
        }
    };
    tracing::info!(
        "join_realm: combined session key = {} bytes",
        combined.len()
    );

    // Store session key in DB as raw bytes (64-byte BLOB), matching C++ setBinary().
    // C++ params: [0]=keyData(bytes), [1]=last_ip, [2]=locale(u8), [3]=os, [4]=timezone_offset(i16), [5]=username
    let ga_username = game_account.name.clone();

    let locale_id = locale_string_to_id_like_cpp(&session.locale);
    let login_info_update = JoinRealmLoginInfoUpdateLikeCpp {
        key_data: combined,
        client_ip: session.addr().ip().to_string(),
        locale: locale_id,
        os: session.os.clone(),
        timezone_offset: session.timezone_offset as i16,
        account_name: ga_username.clone(),
    };

    let mut stmt = session
        .state()
        .login_db
        .prepare(LoginStatements::UPD_BNET_GAME_ACCOUNT_LOGIN_INFO);
    apply_join_realm_login_info_update_like_cpp(&mut stmt, &login_info_update);
    let _ = session.state().login_db.execute(&stmt).await;

    // Build response — ticket is game account name (e.g. "2#1"), sent as blob (matching C++)
    let response = ClientResponse {
        attribute: join_realm_response_attributes_like_cpp(
            &ga_username,
            &server_addresses,
            &server_secret,
        ),
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
    if should_write_sub_regions_like_cpp(session.authed, key)? {
        let realm_mgr = session.state().realm_mgr.read();
        let values = realm_mgr.write_sub_regions_like_cpp();

        let response = GetAllValuesForAttributeResponse {
            attribute_value: values,
        };
        Ok(Some(response.encode_to_vec()))
    } else {
        Ok(Some(
            GetAllValuesForAttributeResponse::default().encode_to_vec(),
        ))
    }
}

fn should_write_sub_regions_like_cpp(authed: bool, attribute_key: &str) -> Result<bool> {
    if !authed {
        return Err(RpcStatusError::new(status::ERROR_DENIED).into());
    }

    Ok(attribute_key.starts_with("Command_RealmListRequest_v1"))
}

// ── Locale conversion ─────────────────────────────────────────────────────

/// Convert locale string (e.g. "esES") to Trinity `LocaleConstant`.
/// Matches C++ `GetLocaleByName`: unknown names return `TOTAL_LOCALES`.
fn locale_string_to_id_like_cpp(locale: &str) -> u8 {
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
        "none" => 9,
        "ptBR" => 10,
        "itIT" => 11,
        _ => 12,
    }
}

fn bnet_session_key_data_like_cpp(client_secret: &[u8], server_secret: &[u8]) -> Option<Vec<u8>> {
    if client_secret.len() != 32 || server_secret.len() != 32 {
        return None;
    }

    let mut key_data = Vec::with_capacity(64);
    key_data.extend_from_slice(client_secret);
    key_data.extend_from_slice(server_secret);
    Some(key_data)
}

fn parse_realm_list_ticket_game_account_id_like_cpp(attrs: &[Attribute]) -> Option<u32> {
    let attr = attrs.iter().find(|a| a.name == "Param_Identity")?;
    let blob = attr.value.blob_value.as_ref()?;
    let text = String::from_utf8_lossy(blob);
    let json_str = text.trim_end_matches('\0');
    let json_str = json_str
        .find(':')
        .map(|pos| &json_str[pos + 1..])
        .unwrap_or(json_str);
    let json = serde_json::from_str::<serde_json::Value>(json_str).ok()?;
    json.get("gameAccountID")
        .and_then(|value| value.as_u64())
        .and_then(|value| u32::try_from(value).ok())
}

fn parse_realm_list_ticket_client_secret_like_cpp(attrs: &[Attribute]) -> Option<Vec<u8>> {
    let attr = attrs.iter().find(|a| a.name == "Param_ClientInfo")?;
    let blob = attr.value.blob_value.as_ref()?;
    let text = String::from_utf8_lossy(blob);
    let json_str = text.trim_end_matches('\0');
    let json_str = json_str
        .find(':')
        .map(|pos| &json_str[pos + 1..])
        .unwrap_or(json_str);
    let json = serde_json::from_str::<serde_json::Value>(json_str).ok()?;
    let values = json.get("info")?.get("secret")?.as_array()?;
    if values.len() != 32 {
        return None;
    }

    let mut secret = Vec::with_capacity(32);
    for value in values {
        let byte = value.as_u64().and_then(|value| u8::try_from(value).ok())?;
        secret.push(byte);
    }
    Some(secret)
}

fn selected_game_account_like_cpp(
    account: &AccountInfo,
    selected_game_account_id: Option<u32>,
) -> Result<&GameAccountInfo> {
    let selected_game_account_id = selected_game_account_id
        .ok_or_else(|| RpcStatusError::new(status::ERROR_USER_SERVER_BAD_WOW_ACCOUNT))?;
    account
        .game_accounts
        .get(&selected_game_account_id)
        .ok_or_else(|| RpcStatusError::new(status::ERROR_USER_SERVER_BAD_WOW_ACCOUNT).into())
}

fn account_info_or_status_like_cpp(
    account: Option<&AccountInfo>,
    missing_status: u32,
) -> Result<&AccountInfo> {
    account.ok_or_else(|| RpcStatusError::new(missing_status).into())
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BnetLastLoginInfoUpdateLikeCpp {
    client_ip: String,
    locale: u8,
    os: String,
    account_id: u32,
}

fn apply_bnet_last_login_info_update_like_cpp(
    stmt: &mut PreparedStatement,
    update: &BnetLastLoginInfoUpdateLikeCpp,
) {
    stmt.set_string(0, &update.client_ip);
    stmt.set_u8(1, update.locale);
    stmt.set_string(2, &update.os);
    stmt.set_u32(3, update.account_id);
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct JoinRealmLoginInfoUpdateLikeCpp {
    key_data: Vec<u8>,
    client_ip: String,
    locale: u8,
    os: String,
    timezone_offset: i16,
    account_name: String,
}

fn apply_join_realm_login_info_update_like_cpp(
    stmt: &mut PreparedStatement,
    update: &JoinRealmLoginInfoUpdateLikeCpp,
) {
    stmt.set_bytes(0, update.key_data.clone());
    stmt.set_string(1, &update.client_ip);
    stmt.set_u8(2, update.locale);
    stmt.set_string(3, &update.os);
    stmt.set_i16(4, update.timezone_offset);
    stmt.set_string(5, &update.account_name);
}

fn join_realm_response_attributes_like_cpp(
    account_name: &str,
    server_addresses: &[u8],
    server_secret: &[u8],
) -> Vec<Attribute> {
    vec![
        make_blob_attribute("Param_RealmJoinTicket", account_name.as_bytes()),
        make_blob_attribute("Param_ServerAddresses", server_addresses),
        make_blob_attribute("Param_JoinSecret", server_secret),
    ]
}

fn last_char_played_response_attributes_like_cpp(
    realm_entry: &[u8],
    last_played: &LastPlayedCharInfo,
) -> Vec<Attribute> {
    vec![
        make_blob_attribute("Param_RealmEntry", realm_entry),
        make_string_attribute("Param_CharacterName", &last_played.character_name),
        make_blob_attribute(
            "Param_CharacterGUID",
            &last_played.character_guid.to_le_bytes(),
        ),
        make_int_attribute(
            "Param_LastPlayedTime",
            i64::from(last_played.last_played_time as i32),
        ),
    ]
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

fn make_int_attribute(name: &str, value: i64) -> Attribute {
    Attribute {
        name: name.to_string(),
        value: Variant {
            int_value: Some(value),
            ..Default::default()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BnetLastLoginInfoUpdateLikeCpp, JoinRealmLoginInfoUpdateLikeCpp,
        account_info_or_status_like_cpp, apply_bnet_last_login_info_update_like_cpp,
        apply_join_realm_login_info_update_like_cpp, bnet_session_key_data_like_cpp,
        join_realm_response_attributes_like_cpp, last_char_played_response_attributes_like_cpp,
        locale_string_to_id_like_cpp, parse_realm_list_ticket_client_secret_like_cpp,
        parse_realm_list_ticket_game_account_id_like_cpp, process_client_request_command_like_cpp,
        remove_suffix, selected_game_account_like_cpp, should_write_sub_regions_like_cpp,
    };
    use crate::rpc::session::RpcStatusError;
    use crate::state::{AccountInfo, GameAccountInfo, LastPlayedCharInfo};
    use std::collections::HashMap;
    use wow_database::{PreparedStatement, SqlParam};
    use wow_proto::bgs::protocol::{Attribute, Variant};
    use wow_proto::status;

    #[test]
    fn bnet_session_key_data_is_raw_client_then_server_secret_like_cpp() {
        let client_secret: Vec<u8> = (0..32).collect();
        let server_secret: Vec<u8> = (32..64).collect();

        let key_data = bnet_session_key_data_like_cpp(&client_secret, &server_secret).unwrap();

        assert_eq!(key_data.len(), 64);
        assert_eq!(&key_data[..32], client_secret.as_slice());
        assert_eq!(&key_data[32..], server_secret.as_slice());
    }

    #[test]
    fn bnet_session_key_data_rejects_non_32_byte_secrets_like_cpp_array_contract() {
        assert!(bnet_session_key_data_like_cpp(&[0; 31], &[1; 32]).is_none());
        assert!(bnet_session_key_data_like_cpp(&[0; 32], &[1; 31]).is_none());
    }

    #[test]
    fn realm_list_ticket_identity_selects_requested_game_account_like_cpp() {
        let attrs = vec![Attribute {
            name: "Param_Identity".to_string(),
            value: Variant {
                blob_value: Some(
                    b"JSONRealmListTicketIdentity:{\"gameAccountID\":42,\"gameAccountRegion\":1}\0"
                        .to_vec(),
                ),
                ..Default::default()
            },
        }];

        assert_eq!(
            parse_realm_list_ticket_game_account_id_like_cpp(&attrs),
            Some(42)
        );
    }

    #[test]
    fn realm_list_ticket_client_secret_accepts_exact_32_byte_array_like_cpp() {
        let attrs = vec![client_info_attr(
            "JSONRealmListTicketClientInformation:{\"info\":{\"secret\":[0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31]}}\0",
        )];

        assert_eq!(
            parse_realm_list_ticket_client_secret_like_cpp(&attrs),
            Some((0..32).collect())
        );
    }

    #[test]
    fn realm_list_ticket_client_secret_rejects_malformed_secret_like_cpp() {
        let too_short = vec![client_info_attr(
            "JSONRealmListTicketClientInformation:{\"info\":{\"secret\":[0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30]}}\0",
        )];
        let too_long = vec![client_info_attr(
            "JSONRealmListTicketClientInformation:{\"info\":{\"secret\":[0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32]}}\0",
        )];
        let out_of_range = vec![client_info_attr(
            "JSONRealmListTicketClientInformation:{\"info\":{\"secret\":[0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,256]}}\0",
        )];
        let negative = vec![client_info_attr(
            "JSONRealmListTicketClientInformation:{\"info\":{\"secret\":[0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,-1]}}\0",
        )];
        let non_integer = vec![client_info_attr(
            "JSONRealmListTicketClientInformation:{\"info\":{\"secret\":[0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,\"31\"]}}\0",
        )];
        let no_protocol_prefix = vec![client_info_attr(
            "{\"info\":{\"secret\":[0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31]}}\0",
        )];

        assert!(parse_realm_list_ticket_client_secret_like_cpp(&too_short).is_none());
        assert!(parse_realm_list_ticket_client_secret_like_cpp(&too_long).is_none());
        assert!(parse_realm_list_ticket_client_secret_like_cpp(&out_of_range).is_none());
        assert!(parse_realm_list_ticket_client_secret_like_cpp(&negative).is_none());
        assert!(parse_realm_list_ticket_client_secret_like_cpp(&non_integer).is_none());
        assert!(parse_realm_list_ticket_client_secret_like_cpp(&no_protocol_prefix).is_none());
    }

    #[test]
    fn selected_game_account_like_cpp_uses_identity_selection_not_hashmap_order() {
        let mut game_accounts = HashMap::new();
        game_accounts.insert(1, test_game_account(1, "2#1"));
        game_accounts.insert(42, test_game_account(42, "2#42"));
        let account = AccountInfo {
            id: 2,
            login: "user@example.test".to_string(),
            is_locked_to_ip: false,
            lock_country: String::new(),
            last_ip: String::new(),
            failed_logins: 0,
            is_banned: false,
            is_permanently_banned: false,
            game_accounts,
        };

        let selected = selected_game_account_like_cpp(&account, Some(42)).unwrap();
        assert_eq!(selected.name, "2#42");
        assert!(selected_game_account_like_cpp(&account, Some(7)).is_err());
        assert!(selected_game_account_like_cpp(&account, None).is_err());
    }

    #[test]
    fn missing_account_info_returns_caller_status_like_cpp() {
        let err =
            account_info_or_status_like_cpp(None, status::ERROR_UTIL_SERVER_INVALID_IDENTITY_ARGS)
                .expect_err("missing account info must become the caller's BNet status");
        let status_err = err
            .downcast_ref::<RpcStatusError>()
            .expect("expected RpcStatusError");
        assert_eq!(
            status_err.status(),
            status::ERROR_UTIL_SERVER_INVALID_IDENTITY_ARGS
        );

        let err = account_info_or_status_like_cpp(None, status::ERROR_USER_SERVER_BAD_WOW_ACCOUNT)
            .expect_err("missing game account context must become BAD_WOW_ACCOUNT");
        let status_err = err
            .downcast_ref::<RpcStatusError>()
            .expect("expected RpcStatusError");
        assert_eq!(
            status_err.status(),
            status::ERROR_USER_SERVER_BAD_WOW_ACCOUNT
        );
    }

    #[test]
    fn realm_utility_status_constants_match_cpp() {
        assert_eq!(status::ERROR_DENIED, 3);
        assert_eq!(status::ERROR_RPC_MALFORMED_REQUEST, 0x0000_0BC5);
        assert_eq!(status::ERROR_RPC_NOT_IMPLEMENTED, 0x0000_0BC7);
        assert_eq!(status::ERROR_UTIL_SERVER_UNKNOWN_REALM, 0x8000_0069);
        assert_eq!(status::ERROR_UTIL_SERVER_INVALID_IDENTITY_ARGS, 0x8000_006E);
        assert_eq!(
            status::ERROR_UTIL_SERVER_FAILED_TO_SERIALIZE_RESPONSE,
            0x8000_0073
        );
        assert_eq!(status::ERROR_USER_SERVER_BAD_WOW_ACCOUNT, 0x8000_00D3);
        assert_eq!(
            status::ERROR_USER_SERVER_NOT_PERMITTED_ON_REALM,
            0x8000_00E1
        );
        assert_eq!(status::ERROR_WOW_SERVICES_INVALID_JOIN_TICKET, 0x8000_012E);
        assert_eq!(
            status::ERROR_WOW_SERVICES_DENIED_REALM_LIST_TICKET,
            0x8000_0132
        );
    }

    #[test]
    fn get_all_values_for_attribute_auth_and_prefix_match_cpp() {
        assert!(should_write_sub_regions_like_cpp(true, "Command_RealmListRequest_v1").unwrap());
        assert!(
            should_write_sub_regions_like_cpp(true, "Command_RealmListRequest_v1_wotlk1").unwrap()
        );
        assert!(
            !should_write_sub_regions_like_cpp(true, "Other_Command_RealmListRequest_v1").unwrap()
        );
        assert!(!should_write_sub_regions_like_cpp(true, "Command_Other_v1").unwrap());

        let err = should_write_sub_regions_like_cpp(false, "Command_RealmListRequest_v1")
            .expect_err("unauthenticated requests must be denied like C++");
        let status = err
            .downcast_ref::<RpcStatusError>()
            .expect("expected RpcStatusError");
        assert_eq!(status.status(), status::ERROR_DENIED);
    }

    #[test]
    fn process_client_request_dispatch_statuses_match_cpp() {
        let err =
            process_client_request_command_like_cpp(false, Some("Command_RealmListRequest_v1"))
                .expect_err("unauthenticated ProcessClientRequest must be denied before dispatch");
        assert_eq!(
            err.downcast_ref::<RpcStatusError>().unwrap().status(),
            status::ERROR_DENIED
        );

        let err = process_client_request_command_like_cpp(true, None)
            .expect_err("missing command must be malformed like C++");
        assert_eq!(
            err.downcast_ref::<RpcStatusError>().unwrap().status(),
            status::ERROR_RPC_MALFORMED_REQUEST
        );

        let err = process_client_request_command_like_cpp(true, Some("Command_Unknown_v1"))
            .expect_err("unknown command must be not implemented like C++");
        assert_eq!(
            err.downcast_ref::<RpcStatusError>().unwrap().status(),
            status::ERROR_RPC_NOT_IMPLEMENTED
        );

        let suffixed = remove_suffix("Command_RealmListRequest_v1_wotlk1");
        assert_eq!(
            process_client_request_command_like_cpp(true, Some(suffixed)).unwrap(),
            "Command_RealmListRequest_v1"
        );
    }

    #[test]
    fn locale_string_to_id_matches_cpp_get_locale_by_name() {
        let locales = [
            ("enUS", 0),
            ("koKR", 1),
            ("frFR", 2),
            ("deDE", 3),
            ("zhCN", 4),
            ("zhTW", 5),
            ("esES", 6),
            ("esMX", 7),
            ("ruRU", 8),
            ("none", 9),
            ("ptBR", 10),
            ("itIT", 11),
            ("bad", 12),
        ];

        for (locale, id) in locales {
            assert_eq!(locale_string_to_id_like_cpp(locale), id, "{locale}");
        }
    }

    #[test]
    fn bnet_last_login_info_update_binds_locale_as_u8_like_cpp() {
        let update = BnetLastLoginInfoUpdateLikeCpp {
            client_ip: "203.0.113.44".to_string(),
            locale: locale_string_to_id_like_cpp("esES"),
            os: "Win".to_string(),
            account_id: 77,
        };

        let mut stmt = PreparedStatement::with_capacity_like_cpp(
            "UPDATE battlenet_accounts SET last_ip = ?, last_login = NOW(), locale = ?, failed_logins = 0, os = ? WHERE id = ?",
            4,
        );
        apply_bnet_last_login_info_update_like_cpp(&mut stmt, &update);

        assert_eq!(stmt.params().len(), 4);
        assert_eq!(
            stmt.params()[0],
            SqlParam::String("203.0.113.44".to_string())
        );
        assert_eq!(stmt.params()[1], SqlParam::U8(6));
        assert_eq!(stmt.params()[2], SqlParam::String("Win".to_string()));
        assert_eq!(stmt.params()[3], SqlParam::U32(77));
    }

    #[test]
    fn join_realm_login_info_update_binds_cpp_statement_params_in_order() {
        let update = JoinRealmLoginInfoUpdateLikeCpp {
            key_data: (0..64).collect(),
            client_ip: "203.0.113.44".to_string(),
            locale: 6,
            os: "Win".to_string(),
            timezone_offset: -60,
            account_name: "2#1".to_string(),
        };

        let mut stmt = PreparedStatement::with_capacity_like_cpp(
            "UPDATE account SET session_key_bnet = ?, last_ip = ?, locale = ?, os = ?, timezone_offset = ? WHERE username = ?",
            6,
        );
        apply_join_realm_login_info_update_like_cpp(&mut stmt, &update);

        assert_eq!(stmt.params().len(), 6);
        assert_eq!(stmt.params()[0], SqlParam::Bytes((0..64).collect()));
        assert_eq!(
            stmt.params()[1],
            SqlParam::String("203.0.113.44".to_string())
        );
        assert_eq!(stmt.params()[2], SqlParam::U8(6));
        assert_eq!(stmt.params()[3], SqlParam::String("Win".to_string()));
        assert_eq!(stmt.params()[4], SqlParam::I16(-60));
        assert_eq!(stmt.params()[5], SqlParam::String("2#1".to_string()));
    }

    #[test]
    fn join_realm_response_attributes_match_cpp_order_and_blob_values() {
        let server_addresses = vec![1, 2, 3, 4];
        let server_secret: Vec<u8> = (32..64).collect();

        let attrs =
            join_realm_response_attributes_like_cpp("2#1", &server_addresses, &server_secret);

        assert_eq!(attrs.len(), 3);
        assert_eq!(attrs[0].name, "Param_RealmJoinTicket");
        assert_eq!(
            attrs[0].value.blob_value.as_deref(),
            Some(b"2#1".as_slice())
        );
        assert_eq!(attrs[1].name, "Param_ServerAddresses");
        assert_eq!(
            attrs[1].value.blob_value.as_deref(),
            Some(server_addresses.as_slice())
        );
        assert_eq!(attrs[2].name, "Param_JoinSecret");
        assert_eq!(
            attrs[2].value.blob_value.as_deref(),
            Some(server_secret.as_slice())
        );
    }

    #[test]
    fn last_char_played_response_attributes_match_cpp_order_and_value_kinds() {
        let realm_entry = vec![1, 2, 3, 4];
        let last_played = LastPlayedCharInfo {
            realm_address: 0x0102_0003,
            character_name: "Tester".to_string(),
            character_guid: 0x0102_0304_0506_0708,
            last_played_time: 0xFFFF_FFFE,
        };

        let attrs = last_char_played_response_attributes_like_cpp(&realm_entry, &last_played);

        assert_eq!(attrs.len(), 4);
        assert_eq!(attrs[0].name, "Param_RealmEntry");
        assert_eq!(
            attrs[0].value.blob_value.as_deref(),
            Some(&[1, 2, 3, 4][..])
        );
        assert_eq!(attrs[1].name, "Param_CharacterName");
        assert_eq!(attrs[1].value.string_value.as_deref(), Some("Tester"));
        assert_eq!(attrs[2].name, "Param_CharacterGUID");
        assert_eq!(
            attrs[2].value.blob_value.as_deref(),
            Some(&0x0102_0304_0506_0708u64.to_le_bytes()[..])
        );
        assert!(attrs[2].value.uint_value.is_none());
        assert_eq!(attrs[3].name, "Param_LastPlayedTime");
        assert_eq!(attrs[3].value.int_value, Some(-2));
        assert!(attrs[3].value.uint_value.is_none());
    }

    fn test_game_account(id: u32, name: &str) -> GameAccountInfo {
        GameAccountInfo {
            id,
            name: name.to_string(),
            display_name: name.to_string(),
            unban_date: 0,
            is_permanently_banned: false,
            is_banned: false,
            security_level: 0,
            char_counts: HashMap::new(),
            last_played_chars: HashMap::new(),
        }
    }

    fn client_info_attr(blob: &str) -> Attribute {
        Attribute {
            name: "Param_ClientInfo".to_string(),
            value: Variant {
                blob_value: Some(blob.as_bytes().to_vec()),
                ..Default::default()
            },
        }
    }
}
