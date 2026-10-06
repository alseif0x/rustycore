// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Combat packet handlers that only need the canonical combat state.
//!
//! C++ source of truth: `WorldSession::HandleAttackStopOpcode` and
//! `WorldSession::HandleSetSheathedOpcode`
//! (`src/server/game/Handlers/CombatHandler.cpp`). The family owns these two
//! packet bodies; the World session only builds the borrowed hub context
//! (#1263 F5). The attack-swing handler stays in the World shell because its
//! admission seam `start_player_attack_like_cpp` is still shell-owned.

use tracing::debug;
use wow_constants::ClientOpcodes;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::packets::combat::{SAttackStop, SetSheathed};
use wow_packet::{ClientPacket, WorldPacket};
use wow_world_core::session::{HubMut, PacketPublicationAccessLikeCpp};

/// Borrowed inputs of one combat handler invocation.
pub struct CombatHandlerCxLikeCpp<'a> {
    hub: HubMut<'a>,
}

impl<'a> CombatHandlerCxLikeCpp<'a> {
    pub fn new(hub: HubMut<'a>) -> Self {
        Self { hub }
    }

    fn publication_like_cpp(&self) -> PacketPublicationAccessLikeCpp<'_> {
        self.hub.shared().core.packet_publication_access_like_cpp()
    }

    /// CMSG_ATTACK_STOP — client stops attacking.
    pub async fn handle_attack_stop(&mut self, _pkt: WorldPacket) {
        let Some(player_guid) = self.hub.shared().core.player_guid() else {
            return;
        };

        debug!(
            account = self.hub.shared().core.account_id,
            "CMSG_ATTACK_STOP"
        );

        if let Some(target) = self.hub.stop_player_attack_like_cpp() {
            self.publication_like_cpp().send_packet(&SAttackStop {
                attacker: player_guid,
                victim: target,
                now_dead: false,
            });
        }
    }

    /// CMSG_SET_SHEATHED — client changes weapon sheathe state.
    ///
    /// We just ack silently; the client manages the visual state.
    pub fn handle_set_sheathed(&mut self, mut pkt: WorldPacket) {
        if let Ok(sheathed) = SetSheathed::read(&mut pkt) {
            debug!(
                account = self.hub.shared().core.account_id,
                state = sheathed.current_sheath_state,
                "SetSheathed"
            );
        }
    }
}

/// Builds a combat handler context from a host's hub.
pub trait CombatHandlerHostLikeCpp<C> {
    fn combat_handler_cx_like_cpp<'a>(&'a mut self, catalogs: &'a C) -> CombatHandlerCxLikeCpp<'a>;
}

fn handle_attack_stop_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CombatHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .combat_handler_cx_like_cpp(catalogs)
            .handle_attack_stop(pkt)
            .await;
    })
}

fn handle_set_sheathed_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: CombatHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .combat_handler_cx_like_cpp(catalogs)
            .handle_set_sheathed(pkt);
    })
}

/// Registers the combat handlers on the packet registry.
pub fn register_combat_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: CombatHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::AttackStop,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_attack_stop",
        handler: handle_attack_stop_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SetSheathed,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_set_sheathed",
        handler: handle_set_sheathed_thunk::<S, C>,
    })?;
    Ok(())
}
