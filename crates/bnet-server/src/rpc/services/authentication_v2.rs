//! Modern authentication adapter, not a hash alias for the V1 wire protocol.
//!
//! TC 6ebe044cbb9895b458fcd3244639acadff287809:
//! Services/AuthenticationService.cpp, V2::{HandleLogon,HandleVerifyAuthToken,
//! HandleGenerateAuthToken}; generated authentication_service.pb.cc dispatch.
//! Account/ticket admission is shared with V1; framing and publication stay here.

mod client_info;

use anyhow::Result;
use prost::Message;
use tokio::io::{AsyncRead, AsyncWrite};
use wow_database::LoginStatements;
use wow_proto::bgs::protocol::account::v2::GameAccountHandle;
use wow_proto::bgs::protocol::authentication::v2::client::*;
use wow_proto::{service_hash, status};

use super::authentication::load_authenticated_account_like_cpp;
use crate::rpc::session::{RpcSession, RpcStatusError};
use crate::state::AccountInfo;

enum Reply {
    Challenge(ExternalChallengeNotification),
    Authenticated(AccountInfo),
    Token(GenerateAuthTokenResponse),
}

/// V2 owns the reply because auth success must precede its listener notification.
/// Transport failures propagate without attempting a second reply/publication.
pub async fn handle<S: AsyncRead + AsyncWrite + Unpin>(
    session: &mut RpcSession<S>,
    method: u32,
    token: u32,
    payload: &[u8],
) -> Result<()> {
    let reply = match prepare(session, method & 0x3FFF_FFFF, payload).await {
        Ok(reply) => reply,
        Err(error) => {
            let code = error
                .downcast_ref::<RpcStatusError>()
                .map(|error| error.status())
                .unwrap_or(status::ERROR_INTERNAL);
            return session.send_response_status(token, code).await;
        }
    };
    match reply {
        Reply::Challenge(challenge) => {
            // AuthenticationListener.pb.cc::OnExternalChallenge is method 4.
            session
                .send_request(
                    service_hash::AUTHENTICATION_LISTENER_V2,
                    4,
                    &challenge.encode_to_vec(),
                )
                .await?;
            session.send_response(token, 0, &[]).await?;
        }
        Reply::Authenticated(account) => {
            let mut key = [0_u8; 64];
            rand::Rng::fill(&mut rand::thread_rng(), key.as_mut_slice());
            let notification = logon_complete(&account, &session.ip_country, &key);
            // Shared::HandleVerifyAuthToken: reply -> notification -> OnLogonSuccess.
            session.send_response(token, 0, &[]).await?;
            session
                .send_request(
                    service_hash::AUTHENTICATION_LISTENER_V2,
                    1,
                    &notification.encode_to_vec(),
                )
                .await?;
            session.account_info = Some(account);
            session.authed = true;
        }
        Reply::Token(response) => {
            session
                .send_response(token, 0, &response.encode_to_vec())
                .await?;
        }
    }
    Ok(())
}

fn decode<M: Message + Default>(payload: &[u8]) -> Result<M> {
    M::decode(payload).map_err(|_| RpcStatusError::new(status::ERROR_RPC_MALFORMED_REQUEST).into())
}

async fn prepare<S: AsyncRead + AsyncWrite + Unpin>(
    session: &mut RpcSession<S>,
    method: u32,
    payload: &[u8],
) -> Result<Reply> {
    match method {
        1 => {
            let request: LogonRequest = decode(payload)?;
            let platform = request.platform.as_deref().unwrap_or_default();
            let locale = request.locale.as_deref().unwrap_or_default();
            client_info::validate(request.title_id.unwrap_or_default(), platform, locale)?;
            session.os = platform.to_owned();
            session.locale = locale.to_owned();
            session.build = request.application_version.unwrap_or_default();
            session.timezone_offset = client_info::timezone(
                request
                    .logon_options
                    .as_ref()
                    .and_then(|options| options.device_id.as_deref()),
            );
            if let Some(token) = request
                .logon_options
                .and_then(|options| options.auth_token)
                .filter(|token| !token.is_empty())
            {
                return authenticate(session, token).await;
            }
            let state = session.state();
            let host = if session.addr().ip().is_loopback() {
                &state.local_address
            } else {
                &state.external_address
            };
            Ok(Reply::Challenge(ExternalChallengeNotification {
                payload_type: Some("web_auth_url".into()),
                payload: Some(
                    format!("https://{host}:{}/bnetserver/login/", state.rest_port).into_bytes(),
                ),
                ..Default::default()
            }))
        }
        2 => {
            let request: VerifyAuthTokenRequest = decode(payload)?;
            let token = request
                .auth_token
                .ok_or_else(|| RpcStatusError::new(status::ERROR_DENIED))?;
            authenticate(session, token).await
        }
        3 => {
            let _: GenerateAuthTokenRequest = decode(payload)?;
            if !session.authed {
                return Err(RpcStatusError::new(status::ERROR_DENIED).into());
            }
            let account = session
                .account_info
                .as_ref()
                .ok_or_else(|| RpcStatusError::new(status::ERROR_DENIED))?;
            let mut stmt = session
                .state()
                .login_db
                .prepare(LoginStatements::SEL_BNET_EXISTING_AUTHENTICATION_BY_ID);
            stmt.set_u32(0, account.id);
            let result = session.state().login_db.query(&stmt).await?;
            if result.is_empty() {
                return Err(RpcStatusError::new(status::ERROR_DENIED).into());
            }
            let token: String = result.read(0);
            Ok(Reply::Token(GenerateAuthTokenResponse {
                auth_token: Some(token.into_bytes()),
            }))
        }
        _ => Err(RpcStatusError::new(status::ERROR_RPC_INVALID_METHOD).into()),
    }
}

async fn authenticate<S: AsyncRead + AsyncWrite + Unpin>(
    session: &mut RpcSession<S>,
    token: Vec<u8>,
) -> Result<Reply> {
    let account =
        load_authenticated_account_like_cpp(session, String::from_utf8_lossy(&token).into_owned())
            .await?;
    Ok(Reply::Authenticated(account))
}

fn logon_complete(
    account: &AccountInfo,
    country: &str,
    key: &[u8; 64],
) -> LogonCompleteNotification {
    LogonCompleteNotification {
        error_code: Some(0),
        record: Some(LogonRecord {
            account_id: Some(u64::from(account.id)),
            game_account: account
                .game_accounts
                .values()
                .map(|game| GameAccountHandle {
                    id: Some(u64::from(game.id)),
                    title_id: Some(client_info::WOW_TITLE_ID),
                    region: Some(2),
                })
                .collect(),
            geoip_country: (!country.is_empty()).then(|| country.to_owned()),
            session_key: Some(key.to_vec()),
            ..Default::default()
        }),
    }
}

#[cfg(test)]
mod tests;
