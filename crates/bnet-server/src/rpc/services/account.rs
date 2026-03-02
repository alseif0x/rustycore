//! Account service handler (hash 0x62DA0891).

use anyhow::Result;
use prost::Message;
use wow_proto::bgs::protocol::account::v1::*;

use crate::rpc::session::RpcSession;
use tokio::io::{AsyncRead, AsyncWrite};

pub async fn handle<S: AsyncRead + AsyncWrite + Unpin>(
    session: &mut RpcSession<S>,
    method_id: u32,
    payload: &[u8],
) -> Result<Option<Vec<u8>>> {
    match method_id {
        30 => handle_get_account_state(session, payload).await,
        31 => handle_get_game_account_state(session, payload).await,
        _ => {
            tracing::warn!("AccountService: unknown method {method_id}");
            Ok(None)
        }
    }
}

/// Method 30: GetAccountState
async fn handle_get_account_state<S: AsyncRead + AsyncWrite + Unpin>(
    _session: &mut RpcSession<S>,
    payload: &[u8],
) -> Result<Option<Vec<u8>>> {
    let request = GetAccountStateRequest::decode(payload)?;

    let mut response = GetAccountStateResponse::default();

    // Check if privacy info was requested
    let wants_privacy = request.options
        .as_ref()
        .is_some_and(|o| o.field_privacy_info.unwrap_or(false) || o.all_fields.unwrap_or(false));

    if wants_privacy {
        response.state = Some(AccountState {
            privacy_info: Some(PrivacyInfo {
                is_using_rid: Some(false),
                is_visible_for_view_friends: Some(false),
                is_hidden_from_friend_finder: Some(true),
            }),
        });
        response.tags = Some(AccountFieldTags {
            privacy_info_tag: Some(0xD7CA_834D),
        });
    }

    Ok(Some(response.encode_to_vec()))
}

/// Method 31: GetGameAccountState
async fn handle_get_game_account_state<S: AsyncRead + AsyncWrite + Unpin>(
    session: &mut RpcSession<S>,
    payload: &[u8],
) -> Result<Option<Vec<u8>>> {
    let request = GetGameAccountStateRequest::decode(payload)?;

    let mut response = GetGameAccountStateResponse::default();
    let mut state = GameAccountState::default();
    let mut tags = GameAccountFieldTags::default();

    let options = request.options.as_ref();
    let wants_level = options.is_some_and(|o| o.field_game_level_info.unwrap_or(false) || o.all_fields.unwrap_or(false));
    let wants_status = options.is_some_and(|o| o.field_game_status.unwrap_or(false) || o.all_fields.unwrap_or(false));

    // Look up game account info
    let ga_id = request.game_account_id
        .as_ref()
        .map(|id| id.low as u32)
        .unwrap_or(0);

    let account = session.account_info.as_ref();

    if wants_level {
        let display_name = account
            .and_then(|a| a.game_accounts.get(&ga_id))
            .map(|ga| ga.display_name.clone())
            .unwrap_or_default();

        state.game_level_info = Some(GameLevelInfo {
            name: Some(display_name),
            program: Some(0x0057_6F57), // "WoW" as u32
        });
        tags.game_level_info_tag = Some(0x5C46_D483);
    }

    if wants_status {
        let (is_banned, is_suspended, suspension_expires) = account
            .and_then(|a| a.game_accounts.get(&ga_id))
            .map(|ga| (ga.is_banned, ga.is_banned && !ga.is_permanently_banned, ga.unban_date))
            .unwrap_or_default();

        state.game_status = Some(GameStatus {
            is_suspended: Some(is_suspended),
            is_banned: Some(is_banned),
            suspension_expires: if is_suspended { Some(suspension_expires) } else { None },
            program: Some(0x0057_6F57),
        });
        tags.game_status_tag = Some(0x98B7_5F99);
    }

    response.state = Some(state);
    response.tags = Some(tags);

    Ok(Some(response.encode_to_vec()))
}
