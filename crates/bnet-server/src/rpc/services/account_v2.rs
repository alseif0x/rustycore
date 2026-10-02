//! Account V2 login queries. Source: TC 6ebe044c Services/AccountService.cpp,
//! V2 methods 101/104/201/203 and generated account_service.pb.cc dispatch.
//! AccountInfo remains the canonical authenticated snapshot. No new DB owner.

use anyhow::Result;
use prost::Message;
use tokio::io::{AsyncRead, AsyncWrite};
use wow_proto::bgs::protocol::account::v2::{self as wire, client::*};
use wow_proto::status;

use crate::rpc::session::{RpcSession, RpcStatusError};
use crate::state::{AccountInfo, GameAccountInfo};

fn decode<M: Message + Default>(payload: &[u8]) -> Result<M> {
    M::decode(payload).map_err(|_| RpcStatusError::new(status::ERROR_RPC_MALFORMED_REQUEST).into())
}

pub async fn handle<S: AsyncRead + AsyncWrite + Unpin>(
    session: &mut RpcSession<S>,
    method: u32,
    payload: &[u8],
) -> Result<Option<Vec<u8>>> {
    let account = session.account_info.as_ref().filter(|_| session.authed);
    response(account, method & 0x3FFF_FFFF, payload).map(Some)
}

fn response(account: Option<&AccountInfo>, method: u32, payload: &[u8]) -> Result<Vec<u8>> {
    if !matches!(method, 101 | 104 | 201 | 203) {
        let code = if matches!(method, 1 | 103 | 105..=108) {
            status::ERROR_RPC_NOT_IMPLEMENTED
        } else {
            status::ERROR_RPC_INVALID_METHOD
        };
        return Err(RpcStatusError::new(code).into());
    }
    // Explicit fail-closed admission; unauthenticated sessions cannot expose a
    // snapshot. V2 C++ handlers assume its account is already available.
    let account = account.ok_or_else(|| RpcStatusError::new(status::ERROR_DENIED))?;
    Ok(match method {
        101 => {
            let _: GetAccountInfoRequest = decode(payload)?;
            GetAccountInfoResponse {
                info: Some(wire::AccountInfo {
                    account_id: Some(u64::from(account.id)),
                    flags: vec![7],
                }),
            }
            .encode_to_vec()
        }
        104 => {
            let _: GetRestrictionRequest = decode(payload)?;
            GetRestrictionResponse::default().encode_to_vec()
        }
        201 => {
            let request: GetGameAccountInfoRequest = decode(payload)?;
            let game = game_account(account, request.game_account.as_ref());
            GetGameAccountInfoResponse {
                info: game.map(|game| wire::GameAccountInfo {
                    account_id: Some(u64::from(game.id)),
                    name: Some(game.display_name.clone()),
                }),
            }
            .encode_to_vec()
        }
        203 => {
            let request: GetGameAccountRestrictionRequest = decode(payload)?;
            GetGameAccountRestrictionResponse {
                restrictions: game_account(account, request.game_account.as_ref())
                    .map(restrictions)
                    .unwrap_or_default(),
            }
            .encode_to_vec()
        }
        _ => unreachable!(),
    })
}

fn game_account<'a>(
    account: &'a AccountInfo,
    handle: Option<&wire::GameAccountHandle>,
) -> Option<&'a GameAccountInfo> {
    let id = u32::try_from(handle?.id.unwrap_or_default()).ok()?;
    account.game_accounts.get(&id)
}

fn restrictions(game: &GameAccountInfo) -> Vec<wire::Restriction> {
    let mut restrictions = Vec::new();
    let restriction = |kind, expiry| wire::Restriction {
        title_id: Some(0x0057_6F57),
        r#type: Some(kind),
        created_time_ms: Some(game.ban_date * 1000),
        expire_time_ms: expiry,
    };
    // C++ uses two independent ifs, preserving order if both flags are set.
    if game.is_permanently_banned {
        restrictions.push(restriction(1, None));
    }
    if game.is_banned {
        restrictions.push(restriction(2, Some(game.unban_date * 1000)));
    }
    restrictions
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    fn account() -> AccountInfo {
        AccountInfo {
            id: 1,
            login: "test".into(),
            is_locked_to_ip: false,
            lock_country: String::new(),
            last_ip: String::new(),
            failed_logins: 0,
            is_banned: false,
            is_permanently_banned: false,
            game_accounts: HashMap::from([(
                2,
                GameAccountInfo {
                    id: 2,
                    name: "1#1".into(),
                    display_name: "WoW1".into(),
                    ban_date: 11,
                    unban_date: 13,
                    is_banned: false,
                    is_permanently_banned: false,
                    security_level: 0,
                    char_counts: HashMap::new(),
                    last_played_chars: HashMap::new(),
                },
            )]),
        }
    }
    #[test]
    fn account_and_game_info_have_exact_modern_fields_and_unknown_is_empty() {
        let account = account();
        assert_eq!(
            response(Some(&account), 101, &[]).unwrap(),
            [0x0a, 4, 8, 1, 0x70, 7]
        );
        assert!(response(Some(&account), 104, &[]).unwrap().is_empty());
        let request = GetGameAccountInfoRequest {
            game_account: Some(wire::GameAccountHandle {
                id: Some(2),
                ..Default::default()
            }),
        }
        .encode_to_vec();
        let result = response(Some(&account), 201, &request).unwrap();
        assert_eq!(
            GetGameAccountInfoResponse::decode(result.as_slice())
                .unwrap()
                .info
                .unwrap()
                .name
                .as_deref(),
            Some("WoW1")
        );
        assert!(response(Some(&account), 201, &[]).unwrap().is_empty());
        assert!(response(Some(&account), 203, &request).unwrap().is_empty());
    }
    #[test]
    fn restriction_order_types_and_millisecond_timestamps_match_cpp() {
        let mut account = account();
        let game = account.game_accounts.get_mut(&2).unwrap();
        game.is_permanently_banned = true;
        game.is_banned = true;
        let out = restrictions(game);
        assert_eq!(out.len(), 2);
        assert_eq!(
            (out[0].r#type, out[0].created_time_ms, out[0].expire_time_ms),
            (Some(1), Some(11000), None)
        );
        assert_eq!(
            (out[1].r#type, out[1].created_time_ms, out[1].expire_time_ms),
            (Some(2), Some(11000), Some(13000))
        );
    }
    #[test]
    fn malformed_unknown_and_unauthenticated_queries_fail_closed() {
        let account = account();
        for (account, method, data, code) in [
            (None, 101, &b""[..], status::ERROR_DENIED),
            (
                Some(&account),
                201,
                &b"\x0a\xff"[..],
                status::ERROR_RPC_MALFORMED_REQUEST,
            ),
            (
                Some(&account),
                999,
                &b""[..],
                status::ERROR_RPC_INVALID_METHOD,
            ),
            (
                Some(&account),
                103,
                &b""[..],
                status::ERROR_RPC_NOT_IMPLEMENTED,
            ),
        ] {
            let error = response(account, method, data).unwrap_err();
            assert_eq!(
                error.downcast_ref::<RpcStatusError>().unwrap().status(),
                code
            );
        }
    }
}
