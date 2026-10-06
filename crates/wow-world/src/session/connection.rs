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
        self.core
            .transport
            .connection
            .set_instance_endpoint(addr, port);
    }

    pub fn instance_address(&self) -> [u8; 4] {
        self.core.instance_address()
    }

    pub fn instance_port(&self) -> u16 {
        self.core.instance_port()
    }

    pub fn set_connect_to_key(&mut self, key: Option<i64>) {
        self.core.set_connect_to_key(key)
    }

    pub fn set_connect_to_serial(
        &mut self,
        serial: Option<wow_packet::packets::auth::ConnectToSerial>,
    ) {
        self.core.set_connect_to_serial(serial)
    }

    pub fn set_instance_link_rx(
        &mut self,
        rx: Option<tokio::sync::oneshot::Receiver<InstanceLink>>,
    ) {
        self.core.set_instance_link_rx(rx)
    }

    /// Install the FIFO completion fence paired with the initial realm socket.
    pub fn set_send_write_fence_like_cpp(&mut self, fence: SocketWriteFenceLikeCpp) {
        self.core
            .transport
            .connection
            .set_send_write_fence_like_cpp(fence);
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
        progression: &super::ProgressionCatalogsLikeCpp,
        feature_policy: &super::SupportFeaturePolicyLikeCpp,
        player_grid_loader: &super::PlayerGridLoadResolverLikeCpp,
    ) {
        match self
            .core
            .transport
            .connection
            .poll_instance_link(self.core.account_id)
        {
            InstanceLinkPollOutcome::Pending => {}
            InstanceLinkPollOutcome::Attached => {
                // Continue the player login sequence on the instance socket
                self.handle_continue_player_login_with_module_registry_like_cpp(
                    item_guid_generator,
                    modules,
                    creature_spawn_catalogs,
                    player_bootstrap,
                    player_rest_rates,
                    progression,
                    feature_policy,
                    player_grid_loader,
                )
                .await;
            }
            InstanceLinkPollOutcome::Failed => {
                self.lifecycle.set_player_loading(None);
                self.lifecycle.release_character_login_claim_like_cpp();
            }
        }
    }

    #[cfg(test)]
    pub(super) async fn poll_instance_link(&mut self) {
        let modules = self
            .core
            .module_registry_like_cpp
            .clone()
            .unwrap_or_else(|| std::sync::Arc::new(wow_module_api::ModuleRegistry::new()));
        let generators = self.id_generators_for_test_like_cpp();
        let player_bootstrap = self.player_bootstrap_catalogs_for_test_like_cpp();
        let creature_spawn_catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        let player_rest_rates = self.player_rest_rate_policy_for_test_like_cpp();
        let progression = self.progression_catalogs_for_test_like_cpp();
        let feature_policy = self.support_feature_policy_for_test_like_cpp();
        self.poll_instance_link_with_module_registry_like_cpp(
            generators.item.as_ref(),
            modules.as_ref(),
            &creature_spawn_catalogs,
            &player_bootstrap,
            &player_rest_rates,
            &progression,
            &feature_policy,
            &super::SessionHandlerCatalogsLikeCpp::default().player_grid_loader,
        )
        .await;
    }

    pub fn send_packet_realm(&self, pkt: &impl wow_packet::ServerPacket) {
        self.core.send_packet_realm(pkt)
    }

    pub(crate) fn restore_realm_channels(&mut self) {
        let (state, mut hub) = crate::session::split_lifecycle_mut(self);
        state.restore_realm_channels(&mut hub)
    }
}

#[cfg(test)]
#[path = "../../unit_tests/session/connection/f3_shims.rs"]
mod f3_shims;
