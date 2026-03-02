//! Connection service handler (hash 0x65446991).

use anyhow::Result;
use prost::Message;
use wow_proto::bgs::protocol::ProcessId;
use wow_proto::bgs::protocol::connection::v1::*;

use crate::rpc::session::RpcSession;
use tokio::io::{AsyncRead, AsyncWrite};

pub async fn handle<S: AsyncRead + AsyncWrite + Unpin>(
    session: &mut RpcSession<S>,
    method_id: u32,
    payload: &[u8],
) -> Result<Option<Vec<u8>>> {
    match method_id {
        1 => handle_connect(session, payload).await,
        5 => handle_keep_alive(session).await,
        7 => handle_request_disconnect(session, payload).await,
        _ => {
            tracing::warn!("ConnectionService: unknown method {method_id}");
            Ok(None)
        }
    }
}

/// Method 1: Connect
async fn handle_connect<S: AsyncRead + AsyncWrite + Unpin>(
    _session: &mut RpcSession<S>,
    payload: &[u8],
) -> Result<Option<Vec<u8>>> {
    let request = ConnectRequest::decode(payload)?;

    let pid = std::process::id();
    let epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u32;

    let response = ConnectResponse {
        server_id: ProcessId { label: pid, epoch },
        client_id: request.client_id,
        server_time: Some(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64,
        ),
        use_bindless_rpc: request.use_bindless_rpc,
        ..Default::default()
    };

    tracing::debug!("ConnectionService: Connect handled");
    Ok(Some(response.encode_to_vec()))
}

/// Method 5: KeepAlive
async fn handle_keep_alive<S: AsyncRead + AsyncWrite + Unpin>(_session: &mut RpcSession<S>) -> Result<Option<Vec<u8>>> {
    Ok(None) // No response needed
}

/// Method 7: RequestDisconnect
async fn handle_request_disconnect<S: AsyncRead + AsyncWrite + Unpin>(
    session: &mut RpcSession<S>,
    payload: &[u8],
) -> Result<Option<Vec<u8>>> {
    let request = DisconnectRequest::decode(payload)?;
    tracing::debug!("ConnectionService: disconnect requested, code={}", request.error_code);

    // Send ForceDisconnect notification back (method 4)
    let notification = DisconnectNotification {
        error_code: request.error_code,
        reason: Some("Client requested disconnect".to_string()),
    };
    let _ = session.send_request(
        wow_proto::service_hash::CONNECTION_SERVICE,
        4,
        &notification.encode_to_vec(),
    ).await;

    // The session will be dropped when the caller detects the closed connection
    Ok(None)
}
