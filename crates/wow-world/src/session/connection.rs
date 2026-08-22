// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Logical realm/instance connection state for a Session.
//!
//! This module owns the *application* side of the dual-socket flow: which
//! logical connection is currently primary, how the instance link is attached
//! when `SMSG_CONNECT_TO` completes, how the realm channels are restored on
//! logout, and which logical connection a packet is routed to.
//!
//! The physical mechanics stay in `wow-network`: sockets, authentication,
//! framing, the encrypted writer tasks, the byte channels themselves, the
//! FIFO `SocketWriteFenceLikeCpp` primitives and `InstanceLink`. What lives
//! here is only the decision layer over those handles — attach, switch,
//! restore and select — so the Session task remains the single owner of the
//! transition while never implementing transport itself.
//!
//! The cross-connection fence waits are here rather than in `wow-network`
//! because the ordering they preserve is a C++ *session update* invariant:
//! C++ enqueues both `SendDirectMessage` calls inside one update, while Rust
//! drives two independent writer tasks. Deciding that a realm packet must not
//! overtake an earlier instance packet is application ordering; the fence that
//! reports physical completion remains network-owned.

use std::time::Duration;

use tracing::{info, warn};
use wow_network::{InstanceLink, SocketWriteFenceLikeCpp, SocketWriteFenceWaitResultLikeCpp};

use super::WorldSession;

// TrinityCore enqueues cross-connection sends without waiting for physical TCP
// progress. RustyCore waits briefly to retain the order observed in captures,
// then completes the already-committed gameplay fanout if a writer stalls.
const CROSS_SOCKET_WRITE_FENCE_TIMEOUT: Duration = Duration::from_millis(250);

impl WorldSession {
    /// Set the instance server address and port.
    pub fn set_instance_endpoint(&mut self, addr: [u8; 4], port: u16) {
        self.instance_address = addr;
        self.instance_port = port;
    }
    /// Get the instance server address.
    pub fn instance_address(&self) -> [u8; 4] {
        self.instance_address
    }

    /// Get the instance server port.
    pub fn instance_port(&self) -> u16 {
        self.instance_port
    }
    /// Set the ConnectTo key.
    pub fn set_connect_to_key(&mut self, key: Option<i64>) {
        self.connect_to_key = key;
    }

    /// Set the ConnectTo serial.
    pub fn set_connect_to_serial(
        &mut self,
        serial: Option<wow_packet::packets::auth::ConnectToSerial>,
    ) {
        self.connect_to_serial = serial;
    }

    /// Set the instance link receiver.
    pub fn set_instance_link_rx(
        &mut self,
        rx: Option<tokio::sync::oneshot::Receiver<InstanceLink>>,
    ) {
        self.instance_link_rx = rx;
    }
    /// Install the FIFO completion fence paired with the session's initial
    /// realm socket. Runtime does this immediately after constructing the
    /// session; unit sessions that never own a physical writer leave it empty.
    pub fn set_send_write_fence_like_cpp(&mut self, fence: SocketWriteFenceLikeCpp) {
        self.send_write_fence_like_cpp = Some(fence);
    }
    /// Poll the instance link oneshot. When received, swap channels and
    /// continue the player login on the instance socket.
    pub(super) async fn poll_instance_link(&mut self) {
        let rx = match self.instance_link_rx.as_mut() {
            Some(rx) => rx,
            None => return,
        };

        // Non-blocking check
        match rx.try_recv() {
            Ok(link) => {
                info!(
                    "Instance link received for account {}, swapping channels",
                    self.account_id
                );

                // Keep the old realm channels alive — if either TCP connection
                // drops the WoW client disconnects the whole session.
                // The realm reader/writer tasks hold the other ends of these
                // channels, so keeping these receivers/senders prevents the
                // realm socket from closing.
                let old_send_tx = std::mem::replace(&mut self.send_tx, link.send_tx);
                self.realm_send_tx = Some(old_send_tx);
                let next_write_fence = link
                    .send_write_fence_like_cpp
                    .or_else(|| self.send_write_fence_like_cpp.clone());
                let old_write_fence =
                    std::mem::replace(&mut self.send_write_fence_like_cpp, next_write_fence);
                self.realm_send_write_fence_like_cpp = old_write_fence;

                if let Some(pkt_rx) = link.pkt_rx {
                    let old_packet_rx = std::mem::replace(&mut self.packet_rx, pkt_rx);
                    self.realm_packet_rx = Some(old_packet_rx);
                }

                self.instance_link_rx = None;

                // Continue the player login sequence on the instance socket
                self.handle_continue_player_login().await;
            }
            Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {
                // Not ready yet, keep waiting
            }
            Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
                warn!(
                    "Instance link channel closed for account {} — instance connection failed",
                    self.account_id
                );
                self.instance_link_rx = None;
                self.player_loading = None;
                self.connect_to_key = None;
                self.release_character_login_claim_like_cpp();
            }
        }
    }
    /// Send a server packet on the **realm** connection.
    ///
    /// Some packets (e.g. `QueryPlayerNamesResponse`) must travel on the
    /// realm socket, not the instance socket.  Falls back to `send_tx` if
    /// no realm channel exists (pre-ConnectTo or single-connection mode).
    pub fn send_packet_realm(&self, pkt: &impl wow_packet::ServerPacket) {
        let data = pkt.to_bytes();
        let tx = self.realm_send_tx.as_ref().unwrap_or(&self.send_tx);
        if tx.send(data).is_err() {
            warn!("Realm send channel closed for account {}", self.account_id);
        }
    }

    /// Wait for current-instance packets to reach their physical socket before
    /// emitting a later realm packet. C++ enqueues both `SendDirectMessage`
    /// calls during one session update; Rust's two independent writer tasks
    /// need this FIFO completion fence to retain observed cross-connection
    /// order.
    pub(crate) async fn wait_for_instance_send_before_realm_send_like_cpp(&self) -> bool {
        let Some(realm_send_tx) = self.realm_send_tx.as_ref() else {
            return true;
        };
        if realm_send_tx.same_channel(&self.send_tx) {
            return true;
        }
        let Some(write_fence) = self.send_write_fence_like_cpp.as_ref() else {
            warn!(
                account = self.account_id,
                "instance/realm ordering fence unavailable for separate sockets"
            );
            return false;
        };
        match write_fence
            .wait_for_prior_packets_written_like_cpp(
                &self.send_tx,
                CROSS_SOCKET_WRITE_FENCE_TIMEOUT,
            )
            .await
        {
            SocketWriteFenceWaitResultLikeCpp::Written => true,
            SocketWriteFenceWaitResultLikeCpp::TimedOut => {
                warn!(
                    account = self.account_id,
                    timeout_ms = CROSS_SOCKET_WRITE_FENCE_TIMEOUT.as_millis(),
                    "instance writer did not acknowledge the ordering fence before timeout"
                );
                false
            }
            SocketWriteFenceWaitResultLikeCpp::WriterClosed => {
                warn!(
                    account = self.account_id,
                    "instance writer closed before acknowledging the ordering fence"
                );
                false
            }
        }
    }

    /// Wait for realm packets to reach their physical socket before emitting a
    /// later instance update. This mirrors `Player::SendNewItem` preceding the
    /// deferred `Map::SendObjectUpdates` player-field flush in C++.
    pub(crate) async fn wait_for_realm_send_before_instance_update_like_cpp(&self) -> bool {
        let Some(realm_send_tx) = self.realm_send_tx.as_ref() else {
            return true;
        };
        if realm_send_tx.same_channel(&self.send_tx) {
            return true;
        }
        let Some(write_fence) = self.realm_send_write_fence_like_cpp.as_ref() else {
            warn!(
                account = self.account_id,
                "realm/instance ordering fence unavailable for separate sockets"
            );
            return false;
        };
        match write_fence
            .wait_for_prior_packets_written_like_cpp(
                realm_send_tx,
                CROSS_SOCKET_WRITE_FENCE_TIMEOUT,
            )
            .await
        {
            SocketWriteFenceWaitResultLikeCpp::Written => true,
            SocketWriteFenceWaitResultLikeCpp::TimedOut => {
                warn!(
                    account = self.account_id,
                    timeout_ms = CROSS_SOCKET_WRITE_FENCE_TIMEOUT.as_millis(),
                    "realm writer did not acknowledge the ordering fence before timeout"
                );
                false
            }
            SocketWriteFenceWaitResultLikeCpp::WriterClosed => {
                warn!(
                    account = self.account_id,
                    "realm writer closed before acknowledging the ordering fence"
                );
                false
            }
        }
    }

    #[cfg(test)]
    pub(crate) fn install_realm_send_channel_for_test(&mut self, tx: flume::Sender<Vec<u8>>) {
        self.realm_send_tx = Some(tx);
    }

    #[cfg(test)]
    pub(crate) fn install_realm_send_write_fence_for_test(
        &mut self,
        fence: SocketWriteFenceLikeCpp,
    ) {
        self.realm_send_write_fence_like_cpp = Some(fence);
    }
    /// Send pre-serialized packet bytes on the realm connection.
    ///
    /// This is the cross-session counterpart of [`Self::send_packet_realm`]:
    /// registry commands already carry serialized bytes, but C++ opcode
    /// routing still requires packets such as `SMSG_PARTY_INVITE` and
    /// `SMSG_PARTY_MEMBER_FULL_STATE` to use `CONNECTION_TYPE_REALM`.
    pub(crate) fn send_raw_packet_realm(&self, data: &[u8]) {
        let tx = self.realm_send_tx.as_ref().unwrap_or(&self.send_tx);
        if tx.send(data.to_vec()).is_err() {
            warn!("Realm send channel closed for account {}", self.account_id);
        }
    }
    /// Restore the realm socket as the primary send/receive channel.
    ///
    /// After a ConnectTo flow, `send_tx` and `packet_rx` point to the
    /// instance socket while the realm channels are stored in
    /// `realm_send_tx` / `realm_packet_rx`.  On logout the client
    /// returns to character select on the REALM connection, so we must
    /// swap back.  The old instance channels are simply dropped — the
    /// instance reader/writer tasks will notice and exit.
    pub(crate) fn restore_realm_channels(&mut self) {
        if let Some(realm_tx) = self.realm_send_tx.take() {
            info!(
                "Restoring realm send channel as primary for account {}",
                self.account_id
            );
            self.send_tx = realm_tx;
        }
        if let Some(realm_write_fence) = self.realm_send_write_fence_like_cpp.take() {
            self.send_write_fence_like_cpp = Some(realm_write_fence);
        }
        if let Some(realm_rx) = self.realm_packet_rx.take() {
            info!(
                "Restoring realm packet channel as primary for account {}",
                self.account_id
            );
            self.packet_rx = realm_rx;
        }
        // Clear any pending ConnectTo state
        self.instance_link_rx = None;
        self.connect_to_key = None;
        self.connect_to_serial = None;
        self.player_loading = None;
    }
}
