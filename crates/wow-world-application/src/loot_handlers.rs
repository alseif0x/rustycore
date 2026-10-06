// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot handlers that only need the loot state and the hub.
//!
//! C++ source of truth: `src/server/game/Handlers/LootHandler.cpp`. The loot
//! specialization transition lives in the loot domain owner and the
//! specialization/class catalogs are reached through the hub, so the World
//! session only builds the borrowed context (#1263 F5). The loot-item, money
//! and unit bodies stay in the World shell while they need the item, quest and
//! creature orchestration.

use wow_constants::ClientOpcodes;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::packets::loot::SetLootSpecialization;
use wow_packet::{ClientPacket, WorldPacket};
use wow_world_core::session::HubMut;
use wow_world_loot::LootState;

/// Borrowed inputs of one loot handler invocation.
pub struct LootHandlerCxLikeCpp<'a> {
    hub: HubMut<'a>,
    loot: &'a mut LootState,
}

impl<'a> LootHandlerCxLikeCpp<'a> {
    pub fn new(hub: HubMut<'a>, loot: &'a mut LootState) -> Self {
        Self { hub, loot }
    }

    /// CMSG_SET_LOOT_SPECIALIZATION — select or clear the loot specialization.
    ///
    /// C++ accepts non-zero values only when `sChrSpecializationStore` has the
    /// row and its `ClassID` matches the player's class; `SpecID == 0` clears.
    pub async fn handle_set_loot_specialization(&mut self, packet: SetLootSpecialization) {
        if self.hub.shared().core.player_guid().is_none() {
            return;
        }

        if packet.spec_id == 0 {
            self.loot
                .set_loot_specialization_id_like_cpp(&mut self.hub, 0);
            return;
        }

        let Some(store) = self.hub.catalogs.chr_specialization_store() else {
            return;
        };
        let Some(spec) = store.get(packet.spec_id) else {
            return;
        };
        if spec.class_id != self.hub.shared().player_class_like_cpp() {
            return;
        }

        self.loot
            .set_loot_specialization_id_like_cpp(&mut self.hub, packet.spec_id);
    }
}

/// Builds a loot handler context from a host's loot state and hub.
pub trait LootHandlerHostLikeCpp<C> {
    fn loot_handler_cx_like_cpp<'a>(&'a mut self, catalogs: &'a C) -> LootHandlerCxLikeCpp<'a>;

    /// Builds the application loot-release context from the host's disjoint state.
    fn loot_release_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
    ) -> crate::LootReleaseCxLikeCpp<'a>;
}

fn handle_set_loot_specialization_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: LootHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        let mut pkt = pkt;
        match SetLootSpecialization::read(&mut pkt) {
            Ok(packet) => {
                session
                    .loot_handler_cx_like_cpp(catalogs)
                    .handle_set_loot_specialization(packet)
                    .await;
            }
            Err(e) => tracing::warn!("Failed to read SetLootSpecialization: {e}"),
        }
    })
}

fn handle_loot_release_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: LootHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .loot_release_handler_cx_like_cpp(catalogs)
            .handle_loot_release(pkt)
            .await;
    })
}

/// Registers the loot handlers on the packet registry.
pub fn register_loot_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: LootHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::SetLootSpecialization,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_loot_specialization",
        handler: handle_set_loot_specialization_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::LootRelease,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_loot_release",
        handler: handle_loot_release_thunk::<S, C>,
    })?;
    Ok(())
}
