// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot handlers that only need the loot state and the hub.
//!
//! C++ source of truth: `src/server/game/Handlers/LootHandler.cpp`. The loot
//! specialization transition lives in the loot domain owner and the
//! specialization/class catalogs are reached through the hub, so the World
//! session only builds the borrowed context (#1263 F5). `CMSG_LOOT_UNIT` also
//! lives here as the C++ `WorldSession::HandleLootOpcode` body; the shell-only
//! capabilities it needs (spell/aura interruption, the represented AE-loot
//! search, the represented loot request/open transitions and the release
//! transition) stay behind the owner's host trait. The loot-item and money
//! bodies stay in the World shell while they need the item, quest and creature
//! orchestration.
//!
//! The packet bodies live in the private submodules of this owner
//! (`specialization`, `unit`, `roll`, `master_loot`); this root keeps the
//! facades, the host trait, the thunks and the registrar, and the bodies are
//! moved unchanged.

use std::future::Future;

use tracing::{debug, warn};
use wow_constants::ClientOpcodes;
use wow_core::ObjectGuid;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_loot::{
    LOOT_METHOD_MASTER_LIKE_CPP, LootClaimLease, LootClaimPayload, LootEntry, OwnedLootAuthority,
};
use wow_packet::packets::loot::{
    AELootTargets, AELootTargetsAck, LOOT_ERROR_DIDNT_KILL_LIKE_CPP,
    LOOT_ERROR_MASTER_OTHER_LIKE_CPP, LOOT_ERROR_PLAYER_NOT_FOUND_LIKE_CPP, LootResponse, LootRoll,
    LootUnit, MasterLootItem, SetLootSpecialization,
};
use wow_packet::{ClientPacket, WorldPacket};
use wow_world_core::session::mailbox::MasterLootGiveResult;
use wow_world_core::session::{HubMut, HubRef, ItemValuationCatalogsLikeCpp};
use wow_world_loot::{LootState, RepresentedCreatureLootStateLikeCpp};

mod master_loot;
mod roll;
mod specialization;
mod unit;

// The registrar's thunks keep their exact call text, so the moved bodies are
// brought back into this scope under their original names.
use master_loot::handle_master_loot_item_with_generator_like_cpp;
use roll::handle_loot_roll_with_generator_like_cpp;
use unit::handle_loot_unit_with_catalogs_like_cpp;

/// Borrowed inputs of one loot handler invocation.
pub struct LootHandlerCxLikeCpp<'a> {
    hub: HubMut<'a>,
    loot: &'a mut LootState,
}

impl<'a> LootHandlerCxLikeCpp<'a> {
    pub fn new(hub: HubMut<'a>, loot: &'a mut LootState) -> Self {
        Self { hub, loot }
    }
}

/// Builds a loot handler context from a host's loot state and hub, and lends the
/// World shell capabilities the moved `CMSG_LOOT_UNIT` body still needs.
///
/// Every method delegates to the existing World operation at the exact point the
/// World body invoked it, so no World body is duplicated and no step value or
/// resumption is needed.
pub trait LootHandlerHostLikeCpp<C> {
    fn loot_handler_cx_like_cpp<'a>(&'a mut self, catalogs: &'a C) -> LootHandlerCxLikeCpp<'a>;

    /// Builds the application loot-release context from the host's disjoint state.
    fn loot_release_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
    ) -> crate::LootReleaseCxLikeCpp<'a>;

    /// The process-owned item valuation catalogs of C++'s static item helpers
    /// (`Entities/Item/Item.cpp`). The World dispatch carries them in the
    /// handler-catalogs bundle this crate cannot name, so the host projects the
    /// capability the moved `HandleLootOpcode` body reads.
    fn loot_unit_item_valuation_like_cpp<'c>(
        &self,
        catalogs: &'c C,
    ) -> &'c ItemValuationCatalogsLikeCpp;

    /// The session hub, which the moved body uses for the player guid, the
    /// account id, `Player::IsAlive`, the player position and the packets.
    fn loot_unit_hub_ref_like_cpp(&self) -> HubRef<'_>;

    /// The loot state owner: the moved body reads the active non-item views and
    /// sets the active loot guid and the AE loot view owners on it.
    fn loot_unit_loot_ref_like_cpp(&self) -> &LootState;
    fn loot_unit_loot_mut_like_cpp(&mut self) -> &mut LootState;

    /// C++ `ObjectAccessor::GetCreature` plus the `AELootCreatureCheck` state of
    /// the corpse the body is about to open.
    fn loot_unit_represented_creature_loot_state_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> Option<RepresentedCreatureLootStateLikeCpp>;

    /// C++ `Player::InterruptNonMeleeSpells(false)` reached by `SendLoot`; the
    /// represented equivalent owns the session cast slots and the canonical Unit
    /// spell slots, which the application owner does not reach.
    fn loot_unit_interrupt_non_melee_spell_cast_like_cpp(&mut self);

    /// C++ `RemoveAurasWithInterruptFlags(SpellAuraInterruptFlags::Looting)`
    /// reads the session's aura container.
    fn loot_unit_remove_auras_with_looting_interrupt_flags_like_cpp(&mut self);

    /// C++ `Trinity::CreatureListSearcher<AELootCreatureCheck>`: the represented
    /// grid search, corpse-pool reconciliation and `isAllowedToLoot` reads live
    /// in the World loot tree.
    fn loot_unit_ae_loot_creature_targets_like_cpp<'a>(
        &'a mut self,
        main_loot_target: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> impl Future<Output = Vec<ObjectGuid>> + Send + 'a;

    /// C++ `Player::SendLoot` request half: `GetLootForPlayer` plus
    /// `Player::isAllowedToLoot` over the represented loot authority.
    fn loot_unit_loot_response_for_owner_like_cpp<'a>(
        &'a mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
        ae_looting: bool,
    ) -> impl Future<Output = Option<LootResponse>> + Send + 'a;

    /// The close-out half of C++ `Player::SendLoot` when the session still holds
    /// other loot views; it owns the application release context.
    fn loot_unit_release_all_like_cpp<'a>(
        &'a mut self,
        player_guid: ObjectGuid,
    ) -> impl Future<Output = ()> + Send + 'a;

    /// C++ `Player::SendLoot` publication: the response, the loot-list
    /// notification and the first-open loot generation stay in the World loot
    /// tree, which owns the represented loot authority.
    fn loot_unit_on_loot_opened_with_catalogs_like_cpp(
        &mut self,
        item_valuation: &ItemValuationCatalogsLikeCpp,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
        response: LootResponse,
    );

    /// The item GUID generator of the dispatch catalogs bundle (C++
    /// `sObjectMgr->GenerateItemLowGuid`); the moved `HandleLootRoll` and
    /// `HandleLootMasterGiveOpcode` bodies hand it to the item-storage
    /// transitions they delegate.
    fn loot_roll_item_guid_generator_like_cpp<'c>(
        &self,
        catalogs: &'c C,
    ) -> &'c wow_core::ObjectGuidGenerator;

    /// C++ `HandleLootRoll`'s represented vote half: the canonical roll state,
    /// the winner/disenchant transitions, the item storage and the social reads
    /// stay in the World loot tree.
    fn loot_roll_player_vote_like_cpp<'a>(
        &'a mut self,
        item_guid_generator: &'a wow_core::ObjectGuidGenerator,
        item_valuation: &'a ItemValuationCatalogsLikeCpp,
        roll: &'a LootRoll,
        player_guid: ObjectGuid,
    ) -> impl Future<Output = bool> + Send + 'a;

    /// C++ `Player::m_group`, resolved through the generation-checked canonical
    /// Player handle of the shell-owned session social state.
    fn master_loot_resolved_group_guid_like_cpp(&self) -> Option<u64>;

    /// The World-test fixture gate of the loot tree; `cfg!(test)` is evaluated
    /// in the shell crate, which owns the represented fixtures.
    fn master_loot_local_fixture_allowed_like_cpp(&self) -> bool;

    /// C++ `Loot::GetLootForPlayer`'s fixture bridge, which installs the
    /// represented cache as the object-owned authority before a reservation can
    /// await.
    fn master_loot_prepare_owned_authority_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        scope_player: ObjectGuid,
    ) -> Option<OwnedLootAuthority>;

    /// C++ `Loot::NotifyLootList` refresh: the packet-building session window
    /// and the canonical object authority stay in the World loot tree.
    fn master_loot_reconcile_loot_cache_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> bool;

    /// C++ `Player::CanStoreNewItem` probe of a master-loot assignment; the
    /// inventory storage plan is shell-owned.
    fn master_loot_can_store_error_like_cpp(
        &self,
        target: ObjectGuid,
        item_id: u32,
        count: u32,
    ) -> Option<u8>;

    /// C++ `Player::StoreLootItem` for the master looter's own award, including
    /// the canonical claim commit and the item storage engine.
    fn master_loot_store_claimed_direct_item_like_cpp<'a>(
        &'a mut self,
        item_guid_generator: &'a wow_core::ObjectGuidGenerator,
        loot_entry: &'a LootEntry,
        dungeon_encounter_id: u32,
        owner_guid: ObjectGuid,
        loot_obj: ObjectGuid,
        claim: &'a LootClaimLease,
    ) -> impl Future<Output = bool> + Send + 'a;

    /// C++ `Player::StoreLootItem` for an authoritative master-loot award.
    fn master_loot_store_direct_item_like_cpp<'a>(
        &'a mut self,
        item_guid_generator: &'a wow_core::ObjectGuidGenerator,
        loot_entry: &'a LootEntry,
        dungeon_encounter_id: u32,
        owner_guid: ObjectGuid,
    ) -> impl Future<Output = bool> + Send + 'a;

    /// The `LOOT_MASTER` list refresh and `LootRemoved` publication after an
    /// authoritative assignment; the publication path is shell-owned.
    fn master_loot_mark_item_removed_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        loot_obj: ObjectGuid,
        loot_list_id: u8,
        target: ObjectGuid,
    );
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

fn handle_loot_unit_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: LootHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        let item_valuation = session.loot_unit_item_valuation_like_cpp(catalogs);
        handle_loot_unit_with_catalogs_like_cpp::<S, C>(session, item_valuation, pkt).await;
    })
}

fn handle_loot_roll_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: LootHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match LootRoll::read(&mut pkt) {
            Ok(roll) => {
                let item_guid_generator = session.loot_roll_item_guid_generator_like_cpp(catalogs);
                let item_valuation = session.loot_unit_item_valuation_like_cpp(catalogs);
                handle_loot_roll_with_generator_like_cpp::<S, C>(
                    session,
                    item_guid_generator,
                    item_valuation,
                    roll,
                )
                .await
            }
            Err(e) => warn!("Failed to read LootRoll: {e}"),
        }
    })
}

fn handle_master_loot_item_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    mut pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: LootHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        match MasterLootItem::read(&mut pkt) {
            Ok(master_loot_item) => {
                let item_guid_generator = session.loot_roll_item_guid_generator_like_cpp(catalogs);
                handle_master_loot_item_with_generator_like_cpp::<S, C>(
                    session,
                    item_guid_generator,
                    master_loot_item,
                )
                .await
            }
            Err(e) => warn!("Failed to read MasterLootItem: {e}"),
        }
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
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::LootUnit,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_loot_unit",
        handler: handle_loot_unit_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::LootRoll,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_loot_roll",
        handler: handle_loot_roll_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::MasterLootItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_master_loot_item",
        handler: handle_master_loot_item_thunk::<S, C>,
    })?;
    Ok(())
}
