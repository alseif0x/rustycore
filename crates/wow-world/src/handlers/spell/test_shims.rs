// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only entry points for the spell cancel handlers moved to
//! `wow-world-application` (#1263 F5).
//!
//! Handler entry points dispatch through the registered production thunks;
//! the cancellation helpers the scenarios drive directly build the context
//! through the production host trait.

use wow_constants::ClientOpcodes;
use wow_core::ObjectGuid;
use wow_packet::WorldPacket;
use wow_world_application::SpellHandlerHostLikeCpp;

use crate::session::{SessionHandlerCatalogsLikeCpp, WorldSession};

async fn dispatch_registered_like_cpp(
    session: &mut WorldSession,
    opcode: ClientOpcodes,
    pkt: WorldPacket,
) {
    let entry = crate::session::registry::registered_handler_entries_like_cpp()
        .find(|entry| entry.opcode == opcode)
        .expect("registered spell handler");
    let catalogs = SessionHandlerCatalogsLikeCpp::default();
    (entry.handler)(session, &catalogs, pkt).await;
}

impl WorldSession {
    pub async fn handle_cancel_cast(&mut self, pkt: WorldPacket) {
        dispatch_registered_like_cpp(self, ClientOpcodes::CancelCast, pkt).await;
    }

    pub async fn handle_cancel_aura(&mut self, pkt: WorldPacket) {
        dispatch_registered_like_cpp(self, ClientOpcodes::CancelAura, pkt).await;
    }

    pub async fn handle_cancel_auto_repeat_spell(&mut self, pkt: WorldPacket) {
        dispatch_registered_like_cpp(self, ClientOpcodes::CancelAutoRepeatSpell, pkt).await;
    }

    pub async fn handle_cancel_channelling(&mut self, pkt: WorldPacket) {
        dispatch_registered_like_cpp(self, ClientOpcodes::CancelChannelling, pkt).await;
    }

    pub async fn handle_cancel_growth_aura(&mut self, pkt: WorldPacket) {
        dispatch_registered_like_cpp(self, ClientOpcodes::CancelGrowthAura, pkt).await;
    }

    pub async fn handle_cancel_mount_aura(&mut self, pkt: WorldPacket) {
        dispatch_registered_like_cpp(self, ClientOpcodes::CancelMountAura, pkt).await;
    }

    pub async fn handle_cancel_queued_spell(&mut self, pkt: WorldPacket) {
        dispatch_registered_like_cpp(self, ClientOpcodes::CancelQueuedSpell, pkt).await;
    }

    pub async fn handle_pet_cancel_aura(&mut self, pkt: WorldPacket) {
        dispatch_registered_like_cpp(self, ClientOpcodes::PetCancelAura, pkt).await;
    }

    pub async fn handle_totem_destroyed(&mut self, pkt: WorldPacket) {
        dispatch_registered_like_cpp(self, ClientOpcodes::TotemDestroyed, pkt).await;
    }

    pub(crate) fn cancel_client_cast_request_like_cpp(&mut self, spell_id: Option<i32>) {
        let catalogs = SessionHandlerCatalogsLikeCpp::default();
        self.spell_handler_cx_like_cpp(&catalogs)
            .cancel_client_cast_request_like_cpp(spell_id);
    }

    pub(crate) fn remove_represented_cancelable_owned_aura_like_cpp(
        &mut self,
        spell_id: i32,
        caster_guid: ObjectGuid,
    ) -> usize {
        let catalogs = SessionHandlerCatalogsLikeCpp::default();
        self.spell_handler_cx_like_cpp(&catalogs)
            .remove_represented_cancelable_owned_aura_like_cpp(spell_id, caster_guid)
    }

    pub(crate) fn remove_represented_mount_auras_cancelable_like_cpp(&mut self) -> usize {
        let catalogs = SessionHandlerCatalogsLikeCpp::default();
        self.spell_handler_cx_like_cpp(&catalogs)
            .remove_represented_mount_auras_cancelable_like_cpp()
    }
}
