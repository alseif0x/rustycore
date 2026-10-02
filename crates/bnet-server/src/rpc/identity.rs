//! Per-connection identity for the BNet `ConnectionService.Connect` exchange.
//!
//! TrinityCore master (`6ebe044cbb9895b458fcd3244639acadff287809`),
//! `src/server/bnetserver/Services/ConnectionService.cpp::HandleConnect`,
//! synthesizes a client `ProcessId` when the request omits one, formats a
//! connection identity (CIID), and stores it on the session.  Its
//! `src/server/bnetserver/Server/Session.cpp::SendResponse` and
//! `::SendRequest` then copy that CIID into every outgoing RPC header.
//!
//! This module owns only the identity state.  `RpcSession` owns one instance
//! and is responsible for putting [`ConnectionIdentity::ciid`] on headers.

use std::sync::{
    OnceLock,
    atomic::{AtomicU32, Ordering},
};
use std::time::{SystemTime, UNIX_EPOCH};

use wow_proto::bgs::protocol::ProcessId;
use wow_proto::bgs::protocol::connection::v1::{ConnectRequest, ConnectResponse};

static SERVER_ID: OnceLock<ProcessId> = OnceLock::new();
static SESSION_ID: AtomicU32 = AtomicU32::new(1);

/// Initialize the process-wide server identity at application startup.
///
/// `RpcSession::new` also calls the same lazy initializer so isolated unit
/// tests and embedders remain safe when they do not run the normal `main`
/// startup path first.
pub(super) fn initialize_server_identity() {
    let _ = SERVER_ID.get_or_init(capture_server_id);
}

/// Identity state belonging to one RPC connection.
pub(super) struct ConnectionIdentity {
    server_id: ProcessId,
    fallback_client_id: ProcessId,
    ciid: Option<String>,
}

impl ConnectionIdentity {
    /// Create an identity for one connection.
    pub(super) fn new() -> Self {
        let server_id = SERVER_ID.get_or_init(capture_server_id).clone();
        let session_id = next_session_id();
        let creation_epoch = unix_seconds();
        Self::from_parts(server_id, session_id, creation_epoch)
    }

    /// Build a deterministic identity snapshot for this module's tests.
    fn from_parts(server_id: ProcessId, session_id: u32, creation_epoch: u32) -> Self {
        Self {
            server_id,
            fallback_client_id: ProcessId {
                label: session_id,
                epoch: creation_epoch,
            },
            ciid: None,
        }
    }

    /// Handle `Connect`, retaining the CIID selected for the latest request.
    pub(super) fn connect(&mut self, request: &ConnectRequest) -> ConnectResponse {
        let client_id = request
            .client_id
            .clone()
            .unwrap_or_else(|| self.fallback_client_id.clone());
        let ciid = format_ciid(&self.server_id, &client_id);
        self.ciid = Some(ciid.clone());

        ConnectResponse {
            server_id: self.server_id.clone(),
            client_id: Some(client_id),
            server_time: Some(unix_millis()),
            use_bindless_rpc: Some(request.use_bindless_rpc.unwrap_or(true)),
            ciid: Some(ciid),
            ..Default::default()
        }
    }

    /// Return the CIID selected by the latest successful `Connect`.
    pub(super) fn ciid(&self) -> Option<&str> {
        self.ciid.as_deref()
    }
}

fn capture_server_id() -> ProcessId {
    ProcessId {
        label: std::process::id(),
        epoch: unix_seconds(),
    }
}

fn next_session_id() -> u32 {
    SESSION_ID.fetch_add(1, Ordering::Relaxed)
}

fn unix_seconds() -> u32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .try_into()
        .unwrap_or(u32::MAX)
}

fn unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

fn format_ciid(server_id: &ProcessId, client_id: &ProcessId) -> String {
    format!(
        "{:08X}{:08X}-{:08X}{:08X}",
        server_id.label, server_id.epoch, client_id.label, client_id.epoch
    )
}

#[cfg(test)]
mod tests {
    use prost::Message;

    use super::*;

    fn server_id() -> ProcessId {
        ProcessId {
            label: 0x12,
            epoch: 0x34,
        }
    }

    #[test]
    fn missing_client_id_uses_stable_connection_identity() {
        let mut identity = ConnectionIdentity::from_parts(server_id(), 0x56, 0x78);
        let request = ConnectRequest::default();

        let first = identity.connect(&request);
        let second = identity.connect(&request);

        assert_eq!(first.server_id, server_id());
        assert_eq!(first.client_id, second.client_id);
        assert_eq!(first.ciid, second.ciid);
        assert_eq!(
            first.client_id,
            Some(ProcessId {
                label: 0x56,
                epoch: 0x78
            })
        );
        assert_eq!(identity.ciid(), first.ciid.as_deref());
    }

    #[test]
    fn provided_client_id_updates_the_connection_identity() {
        let mut identity = ConnectionIdentity::from_parts(server_id(), 1, 2);
        let request = ConnectRequest {
            client_id: Some(ProcessId {
                label: 0x9A,
                epoch: 0xBC,
            }),
            ..Default::default()
        };

        let response = identity.connect(&request);

        assert_eq!(response.client_id, request.client_id);
        assert_eq!(
            response.ciid.as_deref(),
            Some("0000001200000034-0000009A000000BC")
        );
        assert_eq!(identity.ciid(), response.ciid.as_deref());
    }

    #[test]
    fn ciid_field_has_exact_wire_bytes() {
        let mut identity = ConnectionIdentity::from_parts(server_id(), 0x56, 0x78);
        let response = identity.connect(&ConnectRequest::default());
        let encoded = response.encode_to_vec();
        let expected = b"0000001200000034-0000005600000078";

        assert_eq!(
            response.ciid.as_deref(),
            Some(std::str::from_utf8(expected).unwrap())
        );
        assert!(encoded.ends_with(&[vec![0x4A, expected.len() as u8], expected.to_vec()].concat()));
    }

    #[test]
    fn bindless_default_is_true_and_explicit_false_is_preserved() {
        let mut identity = ConnectionIdentity::from_parts(server_id(), 1, 2);

        let default_response = identity.connect(&ConnectRequest::default());
        assert_eq!(default_response.use_bindless_rpc, Some(true));

        let false_response = identity.connect(&ConnectRequest {
            use_bindless_rpc: Some(false),
            ..Default::default()
        });
        assert_eq!(false_response.use_bindless_rpc, Some(false));
    }

    #[test]
    fn connection_lifetime_ids_are_session_scoped() {
        let server = server_id();
        let mut first = ConnectionIdentity::from_parts(server.clone(), 1, 2);
        let mut second = ConnectionIdentity::from_parts(server.clone(), 3, 4);

        let first_response = first.connect(&ConnectRequest::default());
        let second_response = second.connect(&ConnectRequest::default());

        assert_eq!(first_response.server_id, second_response.server_id);
        assert_ne!(first_response.client_id, second_response.client_id);
        assert_ne!(first_response.ciid, second_response.ciid);
    }
}
