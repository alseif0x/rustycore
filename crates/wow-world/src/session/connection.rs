// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Session-side entry points into the transport kernel.
//!
//! The realm/instance decision layer moved to the `wow-session` crate (#297),
//! which compiles without gameplay, databases or catalogs. What remains here is
//! the seam: `WorldSession` forwards to that kernel and performs the session
//! steps the kernel deliberately cannot reach — continuing a player login,
//! releasing a character login claim, and clearing login-loading state.
//!
//! Account identity is passed in as log context rather than stored twice; see
//! the crate docs for why the kernel owns neither it nor `player_loading`.

use wow_network::{InstanceLink, SocketWriteFenceLikeCpp};
use wow_session::InstanceLinkPollOutcome;

use super::WorldSession;

impl WorldSession {
    /// Set the instance server address and port.
    pub fn set_instance_endpoint(&mut self, addr: [u8; 4], port: u16) {
        self.connection.set_instance_endpoint(addr, port);
    }

    /// Get the instance server address.
    pub fn instance_address(&self) -> [u8; 4] {
        self.connection.instance_address()
    }

    /// Get the instance server port.
    pub fn instance_port(&self) -> u16 {
        self.connection.instance_port()
    }

    /// Set the ConnectTo key.
    pub fn set_connect_to_key(&mut self, key: Option<i64>) {
        self.connection.set_connect_to_key(key);
    }

    /// Set the ConnectTo serial.
    pub fn set_connect_to_serial(
        &mut self,
        serial: Option<wow_packet::packets::auth::ConnectToSerial>,
    ) {
        self.connection.set_connect_to_serial(serial);
    }

    /// Set the instance link receiver.
    pub fn set_instance_link_rx(
        &mut self,
        rx: Option<tokio::sync::oneshot::Receiver<InstanceLink>>,
    ) {
        self.connection.set_instance_link_rx(rx);
    }

    /// Install the FIFO completion fence paired with the initial realm socket.
    pub fn set_send_write_fence_like_cpp(&mut self, fence: SocketWriteFenceLikeCpp) {
        self.connection.set_send_write_fence_like_cpp(fence);
    }

    /// The channel currently delivering client packets.
    pub(crate) fn packet_rx(&self) -> &flume::Receiver<wow_packet::WorldPacket> {
        self.connection.packet_rx()
    }

    /// The pending ConnectTo key, if a redirect is in flight.
    /// Test-only, like the kernel query it forwards to: production reads
    /// transport state through the operations above, never field by field.
    #[cfg(test)]
    pub(crate) fn connect_to_key(&self) -> Option<i64> {
        self.connection.connect_to_key()
    }

    /// Whether a ConnectTo serial is still recorded.
    /// Test-only, like the kernel query it forwards to: production reads
    /// transport state through the operations above, never field by field.
    #[cfg(test)]
    pub(crate) fn has_connect_to_serial(&self) -> bool {
        self.connection.has_connect_to_serial()
    }

    /// Whether an instance link receiver is installed and still awaited.
    /// Test-only, like the kernel query it forwards to: production reads
    /// transport state through the operations above, never field by field.
    #[cfg(test)]
    pub(crate) fn is_awaiting_instance_link(&self) -> bool {
        self.connection.is_awaiting_instance_link()
    }

    /// Whether a realm send channel is parked.
    /// Test-only, like the kernel query it forwards to: production reads
    /// transport state through the operations above, never field by field.
    #[cfg(test)]
    pub(crate) fn has_parked_realm_send_channel(&self) -> bool {
        self.connection.has_parked_realm_send_channel()
    }

    /// The channel a realm-routed packet takes.
    pub(crate) fn realm_route_tx(&self) -> &flume::Sender<Vec<u8>> {
        self.connection.realm_route_tx()
    }

    /// A clone of the parked realm receive channel.
    pub(crate) fn realm_packet_rx(&self) -> Option<flume::Receiver<wow_packet::WorldPacket>> {
        self.connection.realm_packet_rx()
    }

    /// Replace the primary receive channel.
    /// Test-only, like the kernel query it forwards to: production reads
    /// transport state through the operations above, never field by field.
    #[cfg(test)]
    pub(crate) fn set_packet_rx(&mut self, rx: flume::Receiver<wow_packet::WorldPacket>) {
        self.connection.set_packet_rx(rx);
    }

    /// Park a realm receive channel directly.
    ///
    /// Test-only, like the kernel setter it forwards to: parking a channel
    /// outside the atomic instance-link transition is not a production state.
    #[cfg(test)]
    pub(crate) fn install_realm_packet_channel(
        &mut self,
        rx: flume::Receiver<wow_packet::WorldPacket>,
    ) {
        self.connection.install_realm_packet_channel(rx);
    }

    /// Drop the parked realm receive channel after its writer disconnected.
    pub(crate) fn clear_realm_packet_rx(&mut self) {
        self.connection.clear_realm_packet_rx();
    }

    /// Poll the instance link, then perform the session step it reports.
    ///
    /// The kernel swaps the channels; continuing the login and releasing the
    /// login claim stay here, because neither is transport.
    pub(super) async fn poll_instance_link_with_module_registry_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        modules: &wow_module_api::ModuleRegistry,
        creature_spawn_catalogs: &super::CreatureSpawnCatalogsLikeCpp,
        player_bootstrap: &super::PlayerBootstrapCatalogsLikeCpp,
        player_rest_rates: &super::PlayerRestRatePolicyLikeCpp,
        feature_policy: &super::SupportFeaturePolicyLikeCpp,
    ) {
        match self.connection.poll_instance_link(self.account_id) {
            InstanceLinkPollOutcome::Pending => {}
            InstanceLinkPollOutcome::Attached => {
                // Continue the player login sequence on the instance socket
                self.handle_continue_player_login_with_module_registry_like_cpp(
                    item_guid_generator,
                    modules,
                    creature_spawn_catalogs,
                    player_bootstrap,
                    player_rest_rates,
                    feature_policy,
                )
                .await;
            }
            InstanceLinkPollOutcome::Failed => {
                self.player_loading = None;
                self.release_character_login_claim_like_cpp();
            }
        }
    }

    #[cfg(test)]
    pub(super) async fn poll_instance_link(&mut self) {
        let modules = self
            .module_registry_like_cpp
            .clone()
            .unwrap_or_else(|| std::sync::Arc::new(wow_module_api::ModuleRegistry::new()));
        let generators = self.id_generators_for_test_like_cpp();
        let player_bootstrap = self.player_bootstrap_catalogs_for_test_like_cpp();
        let creature_spawn_catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        let player_rest_rates = self.player_rest_rate_policy_for_test_like_cpp();
        let feature_policy = self.support_feature_policy_for_test_like_cpp();
        self.poll_instance_link_with_module_registry_like_cpp(
            generators.item.as_ref(),
            modules.as_ref(),
            &creature_spawn_catalogs,
            &player_bootstrap,
            &player_rest_rates,
            &feature_policy,
        )
        .await;
    }

    /// Send a server packet on the **realm** connection.
    pub fn send_packet_realm(&self, pkt: &impl wow_packet::ServerPacket) {
        self.connection
            .send_realm_bytes(pkt.to_bytes(), self.account_id);
    }

    /// Send pre-serialized packet bytes on the realm connection.
    pub(crate) fn send_raw_packet_realm(&self, data: &[u8]) {
        self.connection.send_raw_packet_realm(data, self.account_id);
    }

    /// Wait for instance packets to be written before emitting a realm packet.
    pub(crate) async fn wait_for_instance_send_before_realm_send_like_cpp(&self) -> bool {
        self.connection
            .wait_for_instance_send_before_realm_send_like_cpp(self.account_id)
            .await
    }

    /// Wait for realm packets to be written before emitting an instance update.
    pub(crate) async fn wait_for_realm_send_before_instance_update_like_cpp(&self) -> bool {
        self.connection
            .wait_for_realm_send_before_instance_update_like_cpp(self.account_id)
            .await
    }

    /// Restore the realm socket as primary, and clear the login-loading state
    /// the kernel does not own.
    pub(crate) fn restore_realm_channels(&mut self) {
        self.connection.restore_realm_channels(self.account_id);
        self.player_loading = None;
    }

    #[cfg(test)]
    pub(crate) fn install_realm_send_channel_for_test(&mut self, tx: flume::Sender<Vec<u8>>) {
        self.connection.install_realm_send_channel(tx);
    }

    #[cfg(test)]
    pub(crate) fn install_realm_send_write_fence_for_test(
        &mut self,
        fence: SocketWriteFenceLikeCpp,
    ) {
        self.connection.install_realm_send_write_fence(fence);
    }
}
