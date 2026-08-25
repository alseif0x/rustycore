// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The session transport kernel: which logical connection is primary, and the
//! ordering between the two.
//!
//! This crate is the first piece of `WorldSession` to earn its own home (#297).
//! The epic's P4 rule is that the kernel is promoted only when it compiles
//! without `wow-world`, gameplay, databases or catalogs — so this crate depends
//! on `wow-network` for the socket primitives, `wow-packet` for the wire types,
//! and nothing else. It cannot reach a `Player`, a `Map`, a catalog or a
//! database, and the compiler is what enforces that rather than a convention.
//!
//! What lives here is the *decision* layer over network-owned handles: attach
//! the instance link when `SMSG_CONNECT_TO` completes, switch the primary
//! channels, restore the realm channels on logout, and select which logical
//! connection a packet takes. The physical mechanics stay in `wow-network`:
//! sockets, authentication, framing, the encrypted writer tasks, the byte
//! channels themselves, the FIFO [`SocketWriteFenceLikeCpp`] primitives and
//! [`InstanceLink`].
//!
//! The cross-connection fence waits are here rather than in `wow-network`
//! because the ordering they preserve is a C++ *session update* invariant: C++
//! enqueues both `SendDirectMessage` calls inside one update, while Rust drives
//! two independent writer tasks. Deciding that a realm packet must not overtake
//! an earlier instance packet is application ordering; the fence that reports
//! physical completion remains network-owned.
//!
//! Two things this kernel deliberately does not own. **Account identity** is
//! session identity, not transport, and is read at hundreds of sites; it is
//! passed in as log context so no second copy exists. **Login-loading state**
//! (`player_loading`) feeds visibility gates and character handlers, so the
//! poll below reports what happened and the session clears its own state.

use std::time::Duration;

use tracing::{info, warn};
use wow_network::{InstanceLink, SocketWriteFenceLikeCpp, SocketWriteFenceWaitResultLikeCpp};
use wow_packet::WorldPacket;

// TrinityCore enqueues cross-connection sends without waiting for physical TCP
// progress. RustyCore waits briefly to retain the order observed in captures,
// then completes the already-committed gameplay fanout if a writer stalls.
const CROSS_SOCKET_WRITE_FENCE_TIMEOUT: Duration = Duration::from_millis(250);

/// What one [`SessionConnection::poll_instance_link`] observed.
///
/// The transport work is done by the time this returns; the variants name the
/// session-level step the caller still owes, so the kernel never reaches into
/// login or persistence itself.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InstanceLinkPollOutcome {
    /// No link receiver is installed, or the link has not arrived yet.
    Pending,
    /// The link attached and the primary channels swapped. The caller continues
    /// the player login sequence on the instance socket.
    Attached,
    /// The link channel closed before delivering. The caller releases the
    /// character login claim and clears its own loading state.
    Failed,
}

/// The realm/instance connection state for one session.
pub struct SessionConnection {
    send_tx: flume::Sender<Vec<u8>>,
    packet_rx: flume::Receiver<WorldPacket>,
    realm_send_tx: Option<flume::Sender<Vec<u8>>>,
    realm_packet_rx: Option<flume::Receiver<WorldPacket>>,
    send_write_fence_like_cpp: Option<SocketWriteFenceLikeCpp>,
    realm_send_write_fence_like_cpp: Option<SocketWriteFenceLikeCpp>,
    instance_address: [u8; 4],
    instance_port: u16,
    instance_link_rx: Option<tokio::sync::oneshot::Receiver<InstanceLink>>,
    connect_to_key: Option<i64>,
    connect_to_serial: Option<wow_packet::packets::auth::ConnectToSerial>,
}

impl SessionConnection {
    /// Open a session's transport on its initial realm channels.
    #[must_use]
    pub fn new(send_tx: flume::Sender<Vec<u8>>, packet_rx: flume::Receiver<WorldPacket>) -> Self {
        Self {
            send_tx,
            packet_rx,
            realm_send_tx: None,
            realm_packet_rx: None,
            send_write_fence_like_cpp: None,
            realm_send_write_fence_like_cpp: None,
            instance_address: [0; 4],
            instance_port: 0,
            instance_link_rx: None,
            connect_to_key: None,
            connect_to_serial: None,
        }
    }

    /// The channel currently carrying primary traffic.
    #[must_use]
    pub fn send_tx(&self) -> &flume::Sender<Vec<u8>> {
        &self.send_tx
    }

    /// The channel currently delivering client packets.
    #[must_use]
    pub fn packet_rx(&self) -> &flume::Receiver<WorldPacket> {
        &self.packet_rx
    }

    /// The pending ConnectTo key, if a redirect is in flight.
    #[must_use]
    pub fn connect_to_key(&self) -> Option<i64> {
        self.connect_to_key
    }

    /// Whether a ConnectTo serial is still recorded.
    #[must_use]
    pub fn has_connect_to_serial(&self) -> bool {
        self.connect_to_serial.is_some()
    }

    /// Whether an instance link receiver is installed and still awaited.
    #[must_use]
    pub fn is_awaiting_instance_link(&self) -> bool {
        self.instance_link_rx.is_some()
    }

    /// Whether a realm send channel is parked, i.e. a ConnectTo flow moved the
    /// primary channel to the instance socket.
    #[must_use]
    pub fn has_parked_realm_send_channel(&self) -> bool {
        self.realm_send_tx.is_some()
    }

    /// The channel a realm-routed packet takes: the parked realm channel when a
    /// ConnectTo flow moved the primary to the instance socket, otherwise the
    /// primary itself.
    #[must_use]
    pub fn realm_route_tx(&self) -> &flume::Sender<Vec<u8>> {
        self.realm_send_tx.as_ref().unwrap_or(&self.send_tx)
    }

    /// A clone of the parked realm receive channel, if a ConnectTo flow moved
    /// the primary channel to the instance socket.
    #[must_use]
    pub fn realm_packet_rx(&self) -> Option<flume::Receiver<WorldPacket>> {
        self.realm_packet_rx.clone()
    }

    /// Drop the parked realm receive channel after its writer disconnected.
    ///
    /// The instance socket may still be healthy, so this is not a session
    /// teardown — only the realm half going away.
    pub fn clear_realm_packet_rx(&mut self) {
        self.realm_packet_rx = None;
    }

    /// Set the instance server address and port.
    pub fn set_instance_endpoint(&mut self, addr: [u8; 4], port: u16) {
        self.instance_address = addr;
        self.instance_port = port;
    }

    /// Get the instance server address.
    #[must_use]
    pub fn instance_address(&self) -> [u8; 4] {
        self.instance_address
    }

    /// Get the instance server port.
    #[must_use]
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

    /// Poll the instance link oneshot, swapping the primary channels when it
    /// arrives.
    ///
    /// `account` is log context only. The caller owns the follow-up named by
    /// the returned [`InstanceLinkPollOutcome`].
    pub fn poll_instance_link(&mut self, account: u32) -> InstanceLinkPollOutcome {
        let rx = match self.instance_link_rx.as_mut() {
            Some(rx) => rx,
            None => return InstanceLinkPollOutcome::Pending,
        };

        // Non-blocking check
        match rx.try_recv() {
            Ok(link) => {
                info!("Instance link received for account {account}, swapping channels");

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
                InstanceLinkPollOutcome::Attached
            }
            Err(tokio::sync::oneshot::error::TryRecvError::Empty) => {
                // Not ready yet, keep waiting
                InstanceLinkPollOutcome::Pending
            }
            Err(tokio::sync::oneshot::error::TryRecvError::Closed) => {
                warn!(
                    "Instance link channel closed for account {account} — instance connection failed"
                );
                self.instance_link_rx = None;
                self.connect_to_key = None;
                InstanceLinkPollOutcome::Failed
            }
        }
    }

    /// Send pre-serialized bytes on the **realm** connection.
    ///
    /// Some packets (e.g. `QueryPlayerNamesResponse`) must travel on the realm
    /// socket, not the instance socket. Falls back to the primary channel if no
    /// realm channel exists (pre-ConnectTo or single-connection mode).
    pub fn send_raw_packet_realm(&self, data: &[u8], account: u32) {
        self.send_realm_bytes(data.to_vec(), account);
    }

    /// Send an already-owned buffer on the **realm** connection.
    ///
    /// A typed send has just serialized into a fresh `Vec`, so it moves that
    /// buffer here instead of handing out a slice for this side to copy again.
    /// The borrowed [`Self::send_raw_packet_realm`] keeps its copying semantics
    /// for callers that only hold a slice.
    pub fn send_realm_bytes(&self, data: Vec<u8>, account: u32) {
        if self.realm_route_tx().send(data).is_err() {
            warn!("Realm send channel closed for account {account}");
        }
    }

    /// Wait for current-instance packets to reach their physical socket before
    /// emitting a later realm packet. C++ enqueues both `SendDirectMessage`
    /// calls during one session update; Rust's two independent writer tasks
    /// need this FIFO completion fence to retain observed cross-connection
    /// order.
    pub async fn wait_for_instance_send_before_realm_send_like_cpp(&self, account: u32) -> bool {
        let Some(realm_send_tx) = self.realm_send_tx.as_ref() else {
            return true;
        };
        if realm_send_tx.same_channel(&self.send_tx) {
            return true;
        }
        let Some(write_fence) = self.send_write_fence_like_cpp.as_ref() else {
            warn!(
                account,
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
                    account,
                    timeout_ms = CROSS_SOCKET_WRITE_FENCE_TIMEOUT.as_millis(),
                    "instance writer did not acknowledge the ordering fence before timeout"
                );
                false
            }
            SocketWriteFenceWaitResultLikeCpp::WriterClosed => {
                warn!(
                    account,
                    "instance writer closed before acknowledging the ordering fence"
                );
                false
            }
        }
    }

    /// Wait for realm packets to reach their physical socket before emitting a
    /// later instance update. This mirrors `Player::SendNewItem` preceding the
    /// deferred `Map::SendObjectUpdates` player-field flush in C++.
    pub async fn wait_for_realm_send_before_instance_update_like_cpp(&self, account: u32) -> bool {
        let Some(realm_send_tx) = self.realm_send_tx.as_ref() else {
            return true;
        };
        if realm_send_tx.same_channel(&self.send_tx) {
            return true;
        }
        let Some(write_fence) = self.realm_send_write_fence_like_cpp.as_ref() else {
            warn!(
                account,
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
                    account,
                    timeout_ms = CROSS_SOCKET_WRITE_FENCE_TIMEOUT.as_millis(),
                    "realm writer did not acknowledge the ordering fence before timeout"
                );
                false
            }
            SocketWriteFenceWaitResultLikeCpp::WriterClosed => {
                warn!(
                    account,
                    "realm writer closed before acknowledging the ordering fence"
                );
                false
            }
        }
    }

    /// Restore the realm socket as the primary send/receive channel.
    ///
    /// After a ConnectTo flow the primary channels point at the instance socket
    /// while the realm channels are parked here. On logout the client returns to
    /// character select on the realm connection, so we must swap back. The old
    /// instance channels are simply dropped — the instance reader/writer tasks
    /// will notice and exit.
    ///
    /// The caller still clears its own login-loading state; that is not
    /// transport.
    pub fn restore_realm_channels(&mut self, account: u32) {
        if let Some(realm_tx) = self.realm_send_tx.take() {
            info!("Restoring realm send channel as primary for account {account}");
            self.send_tx = realm_tx;
        }
        if let Some(realm_write_fence) = self.realm_send_write_fence_like_cpp.take() {
            self.send_write_fence_like_cpp = Some(realm_write_fence);
        }
        if let Some(realm_rx) = self.realm_packet_rx.take() {
            info!("Restoring realm packet channel as primary for account {account}");
            self.packet_rx = realm_rx;
        }
        // Clear any pending ConnectTo state
        self.instance_link_rx = None;
        self.connect_to_key = None;
        self.connect_to_serial = None;
    }

    /// Replace the primary receive channel.
    ///
    /// Behind `test-support` alongside the other installers: the real channel
    /// arrives with the session at construction or through the instance-link
    /// swap, so no production caller replaces it. The earlier doc here claimed a
    /// runtime caller that does not exist.
    #[cfg(feature = "test-support")]
    pub fn set_packet_rx(&mut self, rx: flume::Receiver<WorldPacket>) {
        self.packet_rx = rx;
    }

    /// Park a realm receive channel without going through a ConnectTo flow.
    ///
    /// Behind `test-support`. A plain `#[cfg(test)]` gate would not reach the
    /// crates that need this, but leaving it on the stable API would let a
    /// production caller park a sender without its matching receiver and fence —
    /// a transport state the atomic instance-link transition never produces.
    #[cfg(feature = "test-support")]
    pub fn install_realm_packet_channel(&mut self, rx: flume::Receiver<WorldPacket>) {
        self.realm_packet_rx = Some(rx);
    }

    /// Park a realm send channel without going through a ConnectTo flow.
    ///
    /// Behind `test-support`; see [`Self::install_realm_packet_channel`].
    #[cfg(feature = "test-support")]
    pub fn install_realm_send_channel(&mut self, tx: flume::Sender<Vec<u8>>) {
        self.realm_send_tx = Some(tx);
    }

    /// Park a realm write fence, the companion of [`Self::install_realm_send_channel`].
    ///
    /// Behind `test-support`; see [`Self::install_realm_packet_channel`].
    #[cfg(feature = "test-support")]
    pub fn install_realm_send_write_fence(&mut self, fence: SocketWriteFenceLikeCpp) {
        self.realm_send_write_fence_like_cpp = Some(fence);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ACCOUNT: u32 = 42;

    fn connection() -> (
        SessionConnection,
        flume::Receiver<Vec<u8>>,
        flume::Sender<WorldPacket>,
    ) {
        let (send_tx, send_rx) = flume::unbounded::<Vec<u8>>();
        let (pkt_tx, pkt_rx) = flume::unbounded::<WorldPacket>();
        (SessionConnection::new(send_tx, pkt_rx), send_rx, pkt_tx)
    }

    /// No receiver installed is not a failure: nothing to attach, nothing to tear
    /// down.
    #[test]
    fn polling_without_an_instance_link_receiver_is_pending() {
        let (mut connection, _send_rx, _pkt_tx) = connection();
        assert_eq!(
            connection.poll_instance_link(ACCOUNT),
            InstanceLinkPollOutcome::Pending
        );
    }

    /// An installed but undelivered link leaves the primary channel alone, so a
    /// later poll can still attach.
    #[test]
    fn an_undelivered_instance_link_leaves_the_primary_untouched() {
        let (mut connection, send_rx, _pkt_tx) = connection();
        let (link_tx, link_rx) = tokio::sync::oneshot::channel::<InstanceLink>();
        connection.set_instance_link_rx(Some(link_rx));

        assert_eq!(
            connection.poll_instance_link(ACCOUNT),
            InstanceLinkPollOutcome::Pending
        );
        assert!(connection.is_awaiting_instance_link());

        connection.send_raw_packet_realm(&[0xAA], ACCOUNT);
        assert_eq!(send_rx.try_recv().unwrap(), vec![0xAA]);
        drop(link_tx);
    }

    /// A delivered link becomes the primary channel and the old realm channel is
    /// parked, not dropped: the client disconnects if either TCP side closes.
    #[test]
    fn a_delivered_instance_link_swaps_the_primary_and_parks_the_realm() {
        let (mut connection, realm_rx, _pkt_tx) = connection();
        let (instance_tx, instance_rx) = flume::unbounded::<Vec<u8>>();
        let (link_tx, link_rx) = tokio::sync::oneshot::channel::<InstanceLink>();
        connection.set_instance_link_rx(Some(link_rx));
        link_tx
            .send(InstanceLink {
                send_tx: instance_tx,
                send_write_fence_like_cpp: None,
                pkt_rx: None,
            })
            .ok()
            .expect("instance link delivered");

        assert_eq!(
            connection.poll_instance_link(ACCOUNT),
            InstanceLinkPollOutcome::Attached
        );
        assert!(!connection.is_awaiting_instance_link());
        assert!(connection.has_parked_realm_send_channel());

        // Primary traffic now takes the instance socket...
        connection.send_tx().send(vec![0xBB]).unwrap();
        assert_eq!(instance_rx.try_recv().unwrap(), vec![0xBB]);
        // ...while realm-routed traffic still reaches the parked realm channel.
        connection.send_raw_packet_realm(&[0xCC], ACCOUNT);
        assert_eq!(realm_rx.try_recv().unwrap(), vec![0xCC]);
    }

    /// A closed sender means the instance connection never arrived. The pending
    /// ConnectTo state is torn down rather than leaked, and the caller is told to
    /// release its login claim.
    #[test]
    fn a_closed_instance_link_reports_failure_and_clears_pending_connect_to() {
        let (mut connection, _send_rx, _pkt_tx) = connection();
        let (link_tx, link_rx) = tokio::sync::oneshot::channel::<InstanceLink>();
        connection.set_instance_link_rx(Some(link_rx));
        connection.set_connect_to_key(Some(11));
        drop(link_tx);

        assert_eq!(
            connection.poll_instance_link(ACCOUNT),
            InstanceLinkPollOutcome::Failed
        );
        assert!(!connection.is_awaiting_instance_link());
        assert!(connection.connect_to_key().is_none());
    }

    /// Logout returns the client to character select on the realm connection, so
    /// the parked channel is promoted back and the ConnectTo state cleared.
    /// Restoring twice must not promote a dropped channel.
    #[test]
    fn restoring_realm_channels_promotes_the_parked_channel_and_is_idempotent() {
        let (mut connection, realm_rx, _pkt_tx) = connection();
        let (instance_tx, instance_rx) = flume::unbounded::<Vec<u8>>();
        let (link_tx, link_rx) = tokio::sync::oneshot::channel::<InstanceLink>();
        connection.set_instance_link_rx(Some(link_rx));
        link_tx
            .send(InstanceLink {
                send_tx: instance_tx,
                send_write_fence_like_cpp: None,
                pkt_rx: None,
            })
            .ok()
            .expect("instance link delivered");
        connection.poll_instance_link(ACCOUNT);
        connection.set_connect_to_key(Some(7));

        connection.restore_realm_channels(ACCOUNT);

        connection.send_tx().send(vec![0xDD]).unwrap();
        assert_eq!(realm_rx.try_recv().unwrap(), vec![0xDD]);
        assert!(instance_rx.try_recv().is_err());
        assert!(connection.connect_to_key().is_none());
        assert!(!connection.has_connect_to_serial());
        assert!(!connection.is_awaiting_instance_link());
        assert!(!connection.has_parked_realm_send_channel());

        connection.restore_realm_channels(ACCOUNT);
        connection.send_tx().send(vec![0xEE]).unwrap();
        assert_eq!(realm_rx.try_recv().unwrap(), vec![0xEE]);
    }

    /// With no realm channel parked, a realm-routed packet takes the primary —
    /// the pre-ConnectTo and single-connection case.
    #[test]
    fn realm_routing_falls_back_to_the_primary_channel() {
        let (connection, send_rx, _pkt_tx) = connection();
        assert!(
            connection
                .realm_route_tx()
                .same_channel(connection.send_tx())
        );
        connection.send_raw_packet_realm(&[0xFF], ACCOUNT);
        assert_eq!(send_rx.try_recv().unwrap(), vec![0xFF]);
    }

    /// The ordering fences are a no-op while both logical connections are the
    /// same channel: there is nothing to overtake.
    #[tokio::test]
    async fn ordering_fences_pass_when_there_is_one_physical_connection() {
        let (connection, _send_rx, _pkt_tx) = connection();
        assert!(
            connection
                .wait_for_instance_send_before_realm_send_like_cpp(ACCOUNT)
                .await
        );
        assert!(
            connection
                .wait_for_realm_send_before_instance_update_like_cpp(ACCOUNT)
                .await
        );
    }
}
