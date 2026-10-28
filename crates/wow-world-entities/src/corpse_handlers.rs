// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Death and corpse handlers (#1363 slice 4b; moved from the World
//! `handlers/entities/corpse.rs`).
//!
//! C++ refs: `WorldSession::HandleRequestCemeteryList`,
//! `HandleResurrectResponse`, `HandleRepopRequest` and `HandleReclaimCorpse`
//! (`Handlers/MiscHandler.cpp`). The four registrations stay in the
//! `register_corpse_handlers_like_cpp` registrar (`corpse_registrations`). The graveyard
//! condition check, the resurrection request, the teleport and the ghost flag
//! stay with the host; the former `#[cfg(test)]` fixture step runs through
//! `SessionHubHostLikeCpp::test_fixtures_like_cpp`. The resurrection-request
//! clear and take (C++ `Player::ClearResurrectRequestData` and
//! `IsResurrectRequestedBy`) moved here from the World `session/quest/state.rs`
//! in #1363 slice 12c.

use std::future::Future;

use tracing::{info, warn};
use wow_core::{ObjectGuid, Position};
use wow_entities::{Player, PlayerResurrectionPhase, PlayerResurrectionRequestLikeCpp};
use wow_packet::ClientPacket;
use wow_packet::packets::misc::{
    ReclaimCorpse, RepopRequest, RequestCemeteryListResponse, ResurrectResponse,
};

use crate::TELE_TO_NONE_LIKE_CPP;
use crate::movement_host::PlayerMovementHostLikeCpp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResurrectionSaveOutcome {
    Applied,
    Deferred,
    Unresolved,
}

/// The World operations the moved corpse bodies still call; each is the
/// same-named `WorldSession` operation.
pub trait CorpseHostLikeCpp: PlayerMovementHostLikeCpp {
    /// C++ Player::RemovePet(nullptr, PET_SAVE_NOT_IN_SLOT, true).
    fn remove_pet_for_repop_like_cpp(
        &mut self,
        catalogs: crate::CorpseLifecycleCatalogsLikeCpp<'_>,
    ) -> impl Future<Output = Result<(), &'static str>> + Send;
    fn graveyard_conditions_meet_like_cpp(
        &mut self,
        conditions_ref: &wow_data::ConditionsReference,
    ) -> bool;
    /// The same-named `WorldSession` fixture read (World test build; called only
    /// when `test_fixtures_ref_like_cpp` is `Some`, so never this default).
    #[cfg(any(test, feature = "test-fixtures"))]
    fn represented_resurrection_requested_by_like_cpp(&self, _resurrecter: ObjectGuid) -> bool {
        false
    }
    fn teleport_to(&mut self, new_map: u32, new_pos: Position) -> impl Future<Output = ()> + Send;
    /// C++ `Player::TeleportTo(loc, options)`.
    fn teleport_to_with_options_like_cpp(
        &mut self,
        catalogs: crate::ResurrectionOperationCatalogs<'_>,
        new_map: u32,
        new_pos: Position,
        options: wow_world_core::session::movement_protocol::TeleportToOptionsLikeCpp,
    ) -> impl Future<Output = ()> + Send;
    fn pending_teleport_like_cpp(&self) -> Option<(u32, Position)>;

    /// The lifecycle-owned C++ `Corpse::SaveToDB`/`DeleteFromDB` port.
    fn map_corpse_persistence_port_like_cpp(
        &self,
    ) -> Option<std::sync::Arc<dyn wow_persistence::MapCorpsePersistencePortLikeCpp>>;
    /// C++ `Unit::RemoveAurasDueToSpell(spellId)`.
    fn remove_auras_due_to_spell_like_cpp(&mut self, spell_id: i32) -> Result<usize, &'static str>;
    /// C++ `Player::DurabilityLossAll(0.25f, true)`.
    fn apply_spirit_healer_durability_loss(&mut self);
    /// C++ `Player::HasSpell`.
    fn player_has_spell_like_cpp(&self, spell_id: i32) -> bool;
    /// C++ `Player::CastAllObtainSpells`, called by `ResurrectPlayer` after
    /// visibility and before its optional sickness cast.
    fn cast_all_obtain_spells_like_cpp<'a>(
        &'a mut self,
        catalogs: crate::ResurrectionOperationCatalogs<'a>,
    ) -> impl Future<Output = Result<(), &'static str>> + Send + 'a;
    /// C++ `CastSpell(this, spellId, true)`: a triggered self cast through
    /// the spell executor.
    fn cast_triggered_self_spell_like_cpp(
        &mut self,
        catalogs: crate::ResurrectionOperationCatalogs<'_>,
        spell_id: i32,
        sickness_remaining_ms: Option<u32>,
        original_caster_guid: Option<ObjectGuid>,
    ) -> impl Future<Output = bool> + Send;
    /// C++ `WorldSession::isLogingOut()` guard for `BuildPlayerRepop`
    /// (`WorldSession.h:1001`).
    fn is_logging_out_for_repop(&self) -> bool;
    /// C++ `WorldSession::PlayerLogoutWithSave`.
    fn player_logout_with_save_like_cpp(&self) -> bool;
    /// C++ `Player::SaveToDB()` as `SpawnCorpseBones` triggers it.
    fn save_player_after_corpse_bones_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
    ) -> impl Future<Output = ResurrectionSaveOutcome> + Send;
}

fn clear_represented_resurrection_request_like_cpp<H: CorpseHostLikeCpp>(host: &mut H) -> bool {
    #[cfg(any(test, feature = "test-fixtures"))]
    let world_test = host.test_fixtures_ref_like_cpp().is_some();
    #[cfg(not(any(test, feature = "test-fixtures")))]
    let world_test = false;
    let mut hub = host.hub_mut();
    let canonical = hub
        .core
        .with_owned_player_mut_like_cpp(Player::clear_resurrection_request_like_cpp)
        .is_some();
    #[cfg(any(test, feature = "test-fixtures"))]
    if world_test && (canonical || hub.core.player_handle_like_cpp.is_none()) {
        hub.fixtures
            .combat
            .represented_resurrection_request_like_cpp = None;
    }
    canonical || world_test && hub.core.player_handle_like_cpp.is_none()
}

/// CMSG_REQUEST_CEMETERY_LIST — client asks for graveyards in zone.
/// C++ ref: `WorldSession::HandleRequestCemeteryList`.
pub async fn handle_request_cemetery_list_with_catalog_like_cpp<H: CorpseHostLikeCpp>(
    host: &mut H,
    graveyard_store: &wow_data::GraveyardStore,
    _pkt: wow_packet::WorldPacket,
) {
    if std::env::var_os("RUSTYCORE_PACKET_SEQUENCE_TRACE").is_some() {
        info!(
            account = host.core().account_id,
            state = ?host.state(),
            "RUST_CEMETERY_TRACE handler entry"
        );
    }
    let Some((zone_id, area_id)) = host.hub_ref().player_zone_area_like_cpp() else {
        return;
    };
    if std::env::var_os("RUSTYCORE_PACKET_SEQUENCE_TRACE").is_some() {
        info!(
            account = host.core().account_id,
            state = ?host.state(),
            zone = zone_id,
            area = area_id,
            map_id = host.core().player_map_id_like_cpp(),
            player = ?host.player_guid(),
            "RUST_CEMETERY_TRACE handler resolved zone_area"
        );
    }
    let Some(graveyards) = graveyard_store.graveyards_for_zone(zone_id) else {
        info!(
            zone = zone_id,
            area = area_id,
            map_id = host.core().player_map_id_like_cpp(),
            player = ?host.player_guid(),
            "No graveyards found in CMSG_REQUEST_CEMETERY_LIST"
        );
        return;
    };

    let mut cemetery_ids = Vec::new();
    for graveyard in graveyards {
        if cemetery_ids.len() >= 16 {
            break;
        }
        if host.graveyard_conditions_meet_like_cpp(&graveyard.conditions) {
            cemetery_ids.push(graveyard.safe_loc_id);
        }
    }

    if cemetery_ids.is_empty() {
        info!(
            zone = zone_id,
            area = area_id,
            map_id = host.core().player_map_id_like_cpp(),
            candidate_count = graveyards.len(),
            player = ?host.player_guid(),
            "No graveyards passed conditions in CMSG_REQUEST_CEMETERY_LIST"
        );
        return;
    }

    info!(
        zone = zone_id,
        area = area_id,
        map_id = host.core().player_map_id_like_cpp(),
        candidate_count = graveyards.len(),
        accepted_count = cemetery_ids.len(),
        cemetery_ids = ?cemetery_ids,
        player = ?host.player_guid(),
        "Sending C++ RequestCemeteryListResponse"
    );
    host.send_packet(&RequestCemeteryListResponse {
        is_gossip_triggered: false,
        cemetery_ids,
    });
}

/// CMSG_RESURRECT_RESPONSE — answer to a pending resurrection request.
/// C++ ref: `WorldSession::HandleResurrectResponse`.
pub async fn handle_resurrect_response<H: CorpseHostLikeCpp>(
    host: &mut H,
    catalogs: crate::ResurrectionOperationCatalogs<'_>,
    mut pkt: wow_packet::WorldPacket,
) {
    let response = match ResurrectResponse::read(&mut pkt) {
        Ok(response) => response,
        Err(error) => {
            warn!(
                account = host.core().account_id,
                "ResurrectResponse parse failed: {error}"
            );
            return;
        }
    };

    if host.hub_ref().resolved_player_is_alive_like_cpp() != Some(false) {
        return;
    }

    if response.response != 0 {
        clear_represented_resurrection_request_like_cpp(host);
        return;
    }

    let Some(continuation) =
        crate::resurrection_request::accept_request(host, response.resurrecter)
    else {
        return;
    };
    let PlayerResurrectionRequestLikeCpp {
        map_id, position, ..
    } = match continuation.operation {
        wow_entities::PlayerResurrectionOperation::RequestData(request) => request,
        wow_entities::PlayerResurrectionOperation::TeleportRevivePercent { .. }
        | wow_entities::PlayerResurrectionOperation::ReclaimCorpse { .. }
        | wow_entities::PlayerResurrectionOperation::SpiritHealer { .. }
        | wow_entities::PlayerResurrectionOperation::CorpseLogin { .. }
        | wow_entities::PlayerResurrectionOperation::ForcedRepop
        | wow_entities::PlayerResurrectionOperation::SelfRes { .. }
        | wow_entities::PlayerResurrectionOperation::Repop
        | wow_entities::PlayerResurrectionOperation::CreateCorpse => return,
    };

    // C++ `ResurrectUsingRequestData` teleports first; the continuation has
    // already taken Player ownership before this await can be cancelled.
    host.teleport_to_with_options_like_cpp(catalogs, map_id, position, TELE_TO_NONE_LIKE_CPP)
        .await;
    let teleport = host.hub_ref().player_teleport_state_snapshot_like_cpp();
    let near_pending = teleport.is_some_and(|state| state.near_pending);
    let far_pending = host.pending_teleport_like_cpp();
    if near_pending || far_pending.is_some() {
        if !crate::resurrection_request::advance_phase(
            host,
            PlayerResurrectionPhase::TeleportInFlight,
            PlayerResurrectionPhase::AwaitingTeleport,
        ) {
            return;
        }
    } else {
        if !crate::resurrection_request::advance_phase(
            host,
            PlayerResurrectionPhase::TeleportInFlight,
            PlayerResurrectionPhase::TeleportCompleted,
        ) {
            return;
        }
        let completed = crate::resurrection_request::process_after_teleport(host, catalogs).await;
        if !completed
            && !crate::resurrection_request::player_resurrection_save_waiting_for_transfer(host)
        {
            host.core_mut()
                .kick("immediate resurrection continuation did not complete");
        }
    }
}

/// CMSG_REPOP_REQUEST — release spirit.
/// C++ ref: `WorldSession::HandleRepopRequest` (`MiscHandler.cpp:59-83`); the
/// body is [`crate::repop_request_like_cpp`].
pub async fn handle_repop_request<H: CorpseHostLikeCpp>(
    host: &mut H,
    catalogs: crate::CorpseLifecycleCatalogsLikeCpp<'_>,
    mut pkt: wow_packet::WorldPacket,
) {
    if let Err(error) = RepopRequest::read(&mut pkt) {
        warn!(
            account = host.core().account_id,
            "RepopRequest parse failed: {error}"
        );
        return;
    }
    crate::repop_request_like_cpp(host, catalogs).await;
}

/// CMSG_RECLAIM_CORPSE — resurrect at corpse.
/// C++ ref: `WorldSession::HandleReclaimCorpse` (`MiscHandler.cpp:416-445`);
/// the body is [`crate::reclaim_corpse_like_cpp`].
pub async fn handle_reclaim_corpse<H: CorpseHostLikeCpp>(
    host: &mut H,
    catalogs: crate::CorpseLifecycleCatalogsLikeCpp<'_>,
    mut pkt: wow_packet::WorldPacket,
) {
    if let Err(error) = ReclaimCorpse::read(&mut pkt) {
        warn!(
            account = host.core().account_id,
            "ReclaimCorpse parse failed: {error}"
        );
        return;
    }
    crate::reclaim_corpse_like_cpp(host, catalogs).await;
}
