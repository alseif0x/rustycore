// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Session manager for the ConnectTo flow.
//!
//! When a client clicks "Enter World", the realm socket sends `SMSG_CONNECT_TO`
//! and registers a pending connection here. The client disconnects and reconnects
//! to the instance port. The instance socket validates `AuthContinuedSession`
//! against this manager, then links to the existing session.

use dashmap::DashMap;
use tracing::{debug, warn};

/// Information stored for a pending instance connection.
struct PendingEntry {
    /// The ConnectToKey.Raw value the client must present.
    connect_to_key: i64,
    /// Raw session key (40 bytes, NOT hex).
    session_key: Vec<u8>,
    /// Oneshot sender to deliver the instance link to the session.
    instance_link_tx: tokio::sync::oneshot::Sender<InstanceLink>,
}

/// Delivered to the WorldSession when the instance socket is ready.
pub struct InstanceLink {
    /// New send channel — session writes to this, instance socket reads from it.
    pub send_tx: flume::Sender<Vec<u8>>,
    /// Packet receiver — session reads decoded packets from the instance socket here.
    /// `None` in fallback mode (direct login on realm socket — keep existing packet_rx).
    pub pkt_rx: Option<flume::Receiver<wow_packet::WorldPacket>>,
}

/// Thread-safe manager for pending ConnectTo sessions.
pub struct SessionManager {
    /// Pending connections indexed by account_id.
    pending: DashMap<u32, PendingEntry>,
}

impl SessionManager {
    pub fn new() -> Self {
        Self {
            pending: DashMap::new(),
        }
    }

    /// Register a pending instance connection after sending ConnectTo.
    ///
    /// Returns a oneshot receiver that the session should poll. When the
    /// instance socket validates AuthContinuedSession, it sends an
    /// `InstanceLink` through this channel.
    pub fn register(
        &self,
        account_id: u32,
        connect_to_key: i64,
        session_key: Vec<u8>,
    ) -> tokio::sync::oneshot::Receiver<InstanceLink> {
        let (tx, rx) = tokio::sync::oneshot::channel();

        self.pending.insert(
            account_id,
            PendingEntry {
                connect_to_key,
                session_key,
                instance_link_tx: tx,
            },
        );

        debug!("Registered pending instance connection for account {account_id}");
        rx
    }

    /// Validate an AuthContinuedSession and link the instance socket.
    ///
    /// Returns `Ok(session_key)` on success, consuming the pending entry.
    /// The caller should then set up encryption, send EnterEncryptedMode,
    /// and deliver the `InstanceLink` via the returned oneshot sender.
    pub fn validate_and_take(
        &self,
        account_id: u32,
        connect_to_key: i64,
    ) -> Result<ValidatedSession, SessionMgrError> {
        let (_, entry) = self
            .pending
            .remove(&account_id)
            .ok_or(SessionMgrError::NotFound)?;

        if entry.connect_to_key != connect_to_key {
            warn!(
                "ConnectToKey mismatch for account {account_id}: expected {}, got {connect_to_key}",
                entry.connect_to_key
            );
            return Err(SessionMgrError::KeyMismatch);
        }

        debug!("Validated instance connection for account {account_id}");
        Ok(ValidatedSession {
            session_key: entry.session_key,
            instance_link_tx: entry.instance_link_tx,
        })
    }

    /// Remove a pending entry (e.g. on timeout or disconnect).
    pub fn remove(&self, account_id: u32) {
        self.pending.remove(&account_id);
    }
}

/// Returned by `validate_and_take` on success.
pub struct ValidatedSession {
    pub session_key: Vec<u8>,
    pub instance_link_tx: tokio::sync::oneshot::Sender<InstanceLink>,
}

#[derive(Debug)]
pub enum SessionMgrError {
    NotFound,
    KeyMismatch,
}

impl std::fmt::Display for SessionMgrError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound => write!(f, "no pending session found"),
            Self::KeyMismatch => write!(f, "ConnectToKey mismatch"),
        }
    }
}

impl std::error::Error for SessionMgrError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_and_validate() {
        let mgr = SessionManager::new();
        let key = 0x1234_5678_9ABC_DEF0_i64;
        let session_key = vec![0xAAu8; 40];

        let _rx = mgr.register(42, key, session_key.clone());

        let result = mgr.validate_and_take(42, key);
        assert!(result.is_ok());

        let validated = result.unwrap();
        assert_eq!(validated.session_key, session_key);
    }

    #[test]
    fn validate_wrong_key() {
        let mgr = SessionManager::new();
        let _rx = mgr.register(42, 100, vec![0u8; 40]);

        let result = mgr.validate_and_take(42, 999);
        assert!(result.is_err());
    }

    #[test]
    fn validate_not_found() {
        let mgr = SessionManager::new();
        let result = mgr.validate_and_take(99, 100);
        assert!(result.is_err());
    }

    #[test]
    fn validate_consumes_entry() {
        let mgr = SessionManager::new();
        let _rx = mgr.register(42, 100, vec![0u8; 40]);

        let _ = mgr.validate_and_take(42, 100);
        // Second attempt should fail
        let result = mgr.validate_and_take(42, 100);
        assert!(result.is_err());
    }

    #[test]
    fn remove_pending() {
        let mgr = SessionManager::new();
        let _rx = mgr.register(42, 100, vec![0u8; 40]);

        mgr.remove(42);
        let result = mgr.validate_and_take(42, 100);
        assert!(result.is_err());
    }
}
