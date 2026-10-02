//! Modern GameUtilities V2 RealmJoin projection for Classic beta 1.60.1.
//!
//! TrinityCore `RealmList::JoinRealm` (advocaite/TrinityCore 02245dcd)
//! persists a 64-byte client/server key, records the client build, and emits
//! a JSON.RealmList::RealmJoinTicket.  Keep this path separate from the V1
//! account-name ticket so the old wire contract remains unchanged.

use super::super::{
    account_info_or_status_like_cpp, bnet_session_key_data_like_cpp, find_param_like_cpp,
    locale_string_to_id_like_cpp, selected_game_account_like_cpp,
};
use crate::rpc::session::{RpcSession, RpcStatusError};
use anyhow::{Result, bail};
use prost::Message;
use serde::Serialize;
use tokio::io::{AsyncRead, AsyncWrite};
use wow_database::{LoginStatements, PreparedStatement};
use wow_proto::bgs::protocol::game_utilities::v1::{ClientRequest, ClientResponse};
use wow_proto::bgs::protocol::{Attribute, Variant};
use wow_proto::status;

const FOREVER_BUILD: u32 = 70170;

pub(super) async fn handle<S: AsyncRead + AsyncWrite + Unpin>(
    session: &mut RpcSession<S>,
    request: &ClientRequest,
) -> Result<Option<Vec<u8>>> {
    if session.build != FOREVER_BUILD {
        return Err(RpcStatusError::new(status::ERROR_RPC_NOT_IMPLEMENTED).into());
    }

    let account = account_info_or_status_like_cpp(
        session.account_info.as_ref(),
        status::ERROR_USER_SERVER_BAD_WOW_ACCOUNT,
    )?;
    let (account_name, security_level) = {
        let game = selected_game_account_like_cpp(account, session.selected_game_account_id)?;
        (game.name.clone(), game.security_level)
    };
    let client_secret = session.client_secret.clone();
    let variant = session
        .client_build_variant()
        .ok_or_else(|| RpcStatusError::new(status::ERROR_WOW_SERVICES_DENIED_REALM_LIST_TICKET))?;
    let realm_address = parse_realm_address(&request.attribute)?;

    // The realm manager remains the sole authority for current realm state.
    // Drop its read guard before generating/persisting secrets or awaiting DB.
    let server_addresses = {
        let realms = session.state().realm_mgr.read();
        session
            .state()
            .forever_catalog()
            .prepare_join_realm(
                &realms,
                realm_address,
                FOREVER_BUILD,
                security_level,
                Some(session.addr().ip()),
            )
            .map_err(join_status)?
            .server_addresses
    };

    let mut server_secret = vec![0_u8; 32];
    rand::Rng::fill(&mut rand::thread_rng(), server_secret.as_mut_slice());
    let key_data = bnet_session_key_data_like_cpp(&client_secret, &server_secret)
        .ok_or_else(|| RpcStatusError::new(status::ERROR_WOW_SERVICES_DENIED_REALM_LIST_TICKET))?;

    let update = JoinRealmLoginInfoUpdateV2 {
        key_data,
        client_ip: session.addr().ip().to_string(),
        build: session.build,
        locale: locale_string_to_id_like_cpp(&session.locale),
        os: session.os.clone(),
        timezone_offset: session.timezone_offset as i16,
        account_name,
    };
    let mut stmt = session
        .state()
        .login_db
        .prepare(LoginStatements::UPD_BNET_GAME_ACCOUNT_LOGIN_INFO_V2);
    apply_join_realm_login_info_update_v2(&mut stmt, &update);
    let persisted = session
        .state()
        .login_db
        .execute(&stmt)
        .await
        .map_err(Into::into);
    finish_join(
        persisted,
        &RealmJoinTicket {
            game_account: &update.account_name,
            platform: variant.platform(),
            kind: variant.kind(),
            client_arch: variant.arch(),
        },
        &server_addresses,
        &server_secret,
    )
}

/// The production publication fence: even an unknown persistence outcome must
/// not produce a success payload. This is not a rollback or durability claim.
fn finish_join(
    persisted: Result<u64>,
    ticket: &RealmJoinTicket<'_>,
    server_addresses: &[u8],
    server_secret: &[u8],
) -> Result<Option<Vec<u8>>> {
    // Do not publish a ticket when the account row was not updated.  A
    // transport error is likewise propagated before the response exists.
    require_exactly_one_row(persisted?)?;

    let ticket = serde_json::to_vec(ticket)?;
    Ok(Some(
        ClientResponse {
            attribute: join_realm_response_attributes_v2(&ticket, server_addresses, server_secret),
        }
        .encode_to_vec(),
    ))
}

fn require_exactly_one_row(affected: u64) -> Result<()> {
    if affected == 1 {
        Ok(())
    } else {
        bail!("modern realm join update affected {affected} rows; expected one");
    }
}

fn parse_realm_address(attrs: &[Attribute]) -> Result<u32> {
    let value = find_param_like_cpp(attrs, "Param_RealmAddress")
        .and_then(|attr| attr.value.uint_value)
        .ok_or_else(|| RpcStatusError::new(status::ERROR_UTIL_SERVER_UNKNOWN_REALM))?;
    u32::try_from(value)
        .map_err(|_| RpcStatusError::new(status::ERROR_UTIL_SERVER_UNKNOWN_REALM).into())
}

fn join_status(error: crate::realm::JoinRealmPrepareErrorLikeCpp) -> RpcStatusError {
    match error {
        crate::realm::JoinRealmPrepareErrorLikeCpp::UnknownRealm => {
            RpcStatusError::new(status::ERROR_UTIL_SERVER_UNKNOWN_REALM)
        }
        crate::realm::JoinRealmPrepareErrorLikeCpp::UserServerNotPermittedOnRealm => {
            RpcStatusError::new(status::ERROR_USER_SERVER_NOT_PERMITTED_ON_REALM)
        }
    }
}

#[derive(Debug, Serialize)]
struct RealmJoinTicket<'a> {
    #[serde(rename = "gameAccount")]
    game_account: &'a str,
    platform: u32,
    #[serde(rename = "type")]
    kind: u32,
    #[serde(rename = "clientArch")]
    client_arch: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct JoinRealmLoginInfoUpdateV2 {
    key_data: Vec<u8>,
    client_ip: String,
    build: u32,
    locale: u8,
    os: String,
    timezone_offset: i16,
    account_name: String,
}

fn apply_join_realm_login_info_update_v2(
    stmt: &mut PreparedStatement,
    update: &JoinRealmLoginInfoUpdateV2,
) {
    stmt.set_bytes(0, update.key_data.clone());
    stmt.set_string(1, &update.client_ip);
    stmt.set_u32(2, update.build);
    stmt.set_u8(3, update.locale);
    stmt.set_string(4, &update.os);
    stmt.set_i16(5, update.timezone_offset);
    stmt.set_string(6, &update.account_name);
}

fn join_realm_response_attributes_v2(
    ticket: &[u8],
    server_addresses: &[u8],
    server_secret: &[u8],
) -> Vec<Attribute> {
    vec![
        make_blob("Param_RealmJoinTicket", ticket),
        make_blob("Param_ServerAddresses", server_addresses),
        make_blob("Param_JoinSecret", server_secret),
    ]
}

fn make_blob(name: &str, value: &[u8]) -> Attribute {
    Attribute {
        name: name.to_owned(),
        value: Variant {
            blob_value: Some(value.to_vec()),
            ..Default::default()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn production_publication_fence_rejects_failed_unknown_and_wrong_row_outcomes() {
        let ticket = RealmJoinTicket {
            game_account: "1#1",
            platform: 1,
            kind: 2,
            client_arch: 3,
        };
        for result in [
            Ok(0),
            Ok(2),
            Err(anyhow::anyhow!("definite failure")),
            Err(anyhow::anyhow!("unknown commit outcome")),
        ] {
            assert!(finish_join(result, &ticket, b"addresses", &[7; 32]).is_err());
        }
        let bytes = finish_join(Ok(1), &ticket, b"addresses", &[7; 32])
            .unwrap()
            .unwrap();
        let response = ClientResponse::decode(bytes.as_slice()).unwrap();
        assert_eq!(
            response
                .attribute
                .iter()
                .map(|a| a.name.as_str())
                .collect::<Vec<_>>(),
            [
                "Param_RealmJoinTicket",
                "Param_ServerAddresses",
                "Param_JoinSecret"
            ]
        );
        assert_eq!(
            response.attribute[1].value.blob_value.as_deref(),
            Some(b"addresses".as_slice())
        );
        assert_eq!(
            response.attribute[2].value.blob_value.as_deref(),
            Some([7; 32].as_slice())
        );
    }

    #[test]
    fn realm_address_parser_rejects_missing_and_u64_overflow() {
        let missing = Vec::new();
        assert_eq!(
            parse_realm_address(&missing)
                .unwrap_err()
                .downcast_ref::<RpcStatusError>()
                .unwrap()
                .status(),
            status::ERROR_UTIL_SERVER_UNKNOWN_REALM
        );

        let overflow = vec![Attribute {
            name: "Param_RealmAddress".into(),
            value: Variant {
                uint_value: Some(u64::from(u32::MAX) + 1),
                ..Default::default()
            },
        }];
        assert_eq!(
            parse_realm_address(&overflow)
                .unwrap_err()
                .downcast_ref::<RpcStatusError>()
                .unwrap()
                .status(),
            status::ERROR_UTIL_SERVER_UNKNOWN_REALM
        );
    }

    #[test]
    fn ticket_json_is_unprefixed_and_uses_target_field_names() {
        let ticket = serde_json::to_vec(&RealmJoinTicket {
            game_account: "2#1",
            platform: u32::from_be_bytes(*b"\0Win"),
            kind: u32::from_be_bytes(*b"WoWB"),
            client_arch: u32::from_be_bytes(*b"\0x64"),
        })
        .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&ticket).unwrap();
        assert_eq!(value["gameAccount"], "2#1");
        assert_eq!(value["platform"], u32::from_be_bytes(*b"\0Win"));
        assert_eq!(value["type"], u32::from_be_bytes(*b"WoWB"));
        assert_eq!(value["clientArch"], u32::from_be_bytes(*b"\0x64"));
        assert!(!ticket.starts_with(b"JSON"));
        assert!(!ticket.ends_with(&[0]));
    }

    #[test]
    fn modern_update_binds_build_between_key_and_locale() {
        use wow_database::StatementDef;
        let update = JoinRealmLoginInfoUpdateV2 {
            key_data: (0..64).collect(),
            client_ip: "127.0.0.1".into(),
            build: FOREVER_BUILD,
            locale: 6,
            os: "Wn64".into(),
            timezone_offset: 0,
            account_name: "2#1".into(),
        };
        let sql = LoginStatements::UPD_BNET_GAME_ACCOUNT_LOGIN_INFO_V2.sql();
        assert!(sql.contains("client_build = ?"));
        assert!(
            !LoginStatements::UPD_BNET_GAME_ACCOUNT_LOGIN_INFO
                .sql()
                .contains("client_build")
        );
        assert_eq!(sql.matches('?').count(), 7);
        let mut stmt = PreparedStatement::with_capacity_like_cpp(sql, 7);
        apply_join_realm_login_info_update_v2(&mut stmt, &update);
        assert_eq!(
            stmt.params()[0],
            wow_database::SqlParam::Bytes((0..64).collect())
        );
        assert_eq!(
            stmt.params()[1],
            wow_database::SqlParam::String("127.0.0.1".into())
        );
        assert_eq!(stmt.params()[2], wow_database::SqlParam::U32(FOREVER_BUILD));
        assert_eq!(stmt.params()[3], wow_database::SqlParam::U8(6));
        assert_eq!(
            stmt.params()[4],
            wow_database::SqlParam::String("Wn64".into())
        );
        assert_eq!(stmt.params()[5], wow_database::SqlParam::I16(0));
        assert_eq!(
            stmt.params()[6],
            wow_database::SqlParam::String("2#1".into())
        );
    }

    #[test]
    fn row_count_guard_blocks_ticket_publication_for_zero_or_multiple_rows() {
        assert!(require_exactly_one_row(1).is_ok());
        assert!(require_exactly_one_row(0).is_err());
        assert!(require_exactly_one_row(2).is_err());
    }

    #[test]
    fn realm_preparation_statuses_match_cpp() {
        assert_eq!(
            join_status(crate::realm::JoinRealmPrepareErrorLikeCpp::UnknownRealm).status(),
            status::ERROR_UTIL_SERVER_UNKNOWN_REALM
        );
        assert_eq!(
            join_status(crate::realm::JoinRealmPrepareErrorLikeCpp::UserServerNotPermittedOnRealm)
                .status(),
            status::ERROR_USER_SERVER_NOT_PERMITTED_ON_REALM
        );
    }
}
