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

/// CMSG_LOOT_UNIT — player right-clicks a dead creature to loot it.
///
/// C++ `WorldSession::HandleLootOpcode` (`Handlers/LootHandler.cpp:216`,
/// registered `Opcodes.cpp:590` as `STATUS_LOGGEDIN`/`PROCESS_THREADUNSAFE`).
/// The body keeps its original parse, gates, order, log strings and packets;
/// only the access path changed.
async fn handle_loot_unit_with_catalogs_like_cpp<H, C>(
    host: &mut H,
    item_valuation: &ItemValuationCatalogsLikeCpp,
    mut pkt: WorldPacket,
) where
    H: LootHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    let req = match LootUnit::read(&mut pkt) {
        Ok(r) => r,
        Err(e) => {
            warn!("Bad LootUnit: {e}");
            return;
        }
    };

    let player_guid = match host.loot_unit_hub_ref_like_cpp().core.player_guid() {
        Some(g) => g,
        None => return,
    };

    debug!(account = host.loot_unit_hub_ref_like_cpp().core.account_id, target = ?req.unit, "CMSG_LOOT_UNIT");

    if host
        .loot_unit_hub_ref_like_cpp()
        .resolved_player_is_alive_like_cpp()
        != Some(true)
    {
        return;
    }

    if !req.unit.is_creature_or_vehicle() {
        return;
    }

    // Check creature exists and is dead.
    let creature_state = match host.loot_unit_represented_creature_loot_state_like_cpp(req.unit) {
        Some(state) => state,
        None => {
            warn!("LootUnit: creature {:?} not found", req.unit);
            return;
        }
    };

    if creature_state.is_alive {
        return;
    }

    if host
        .loot_unit_hub_ref_like_cpp()
        .player_position_like_cpp()
        .is_some_and(|player| !player.is_within_dist(&creature_state.position, 30.0))
    {
        return;
    }

    host.loot_unit_interrupt_non_melee_spell_cast_like_cpp();
    host.loot_unit_remove_auras_with_looting_interrupt_flags_like_cpp();

    let ae_owner_guids = if host
        .loot_unit_hub_ref_like_cpp()
        .config
        .enable_ae_loot_like_cpp()
    {
        host.loot_unit_ae_loot_creature_targets_like_cpp(req.unit, player_guid)
            .await
    } else {
        Vec::new()
    };

    if !ae_owner_guids.is_empty() {
        host.loot_unit_hub_ref_like_cpp()
            .core
            .send_packet(&AELootTargets {
                count: ae_owner_guids.len() as u32 + 1,
            });
    }

    let Some(response) = host
        .loot_unit_loot_response_for_owner_like_cpp(req.unit, player_guid, false)
        .await
    else {
        return;
    };
    if host
        .loot_unit_loot_ref_like_cpp()
        .has_active_non_item_loot_views_like_cpp()
    {
        host.loot_unit_release_all_like_cpp(player_guid).await;
    }
    host.loot_unit_loot_mut_like_cpp()
        .set_active_loot_guid(req.unit);
    host.loot_unit_on_loot_opened_with_catalogs_like_cpp(
        item_valuation,
        req.unit,
        player_guid,
        response,
    );

    if !ae_owner_guids.is_empty() {
        host.loot_unit_hub_ref_like_cpp()
            .core
            .send_packet(&AELootTargetsAck);

        for owner_guid in ae_owner_guids {
            if let Some(response) = host
                .loot_unit_loot_response_for_owner_like_cpp(owner_guid, player_guid, true)
                .await
            {
                host.loot_unit_loot_mut_like_cpp()
                    .add_active_loot_view_owner_like_cpp(owner_guid);
                host.loot_unit_on_loot_opened_with_catalogs_like_cpp(
                    item_valuation,
                    owner_guid,
                    player_guid,
                    response,
                );
                host.loot_unit_hub_ref_like_cpp()
                    .core
                    .send_packet(&AELootTargetsAck);
            }
        }
    }
}

/// CMSG_LOOT_ROLL — vote on a pending group loot roll.
///
/// C++ `HandleLootRoll` silently returns when `GetLootRoll` finds no
/// canonical roll state. Rust does not yet port that state machine, so this
/// represented handler preserves the current wire behavior without emitting
/// synthetic errors.
async fn handle_loot_roll_with_generator_like_cpp<H, C>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    item_valuation: &ItemValuationCatalogsLikeCpp,
    roll: LootRoll,
) where
    H: LootHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    let Some(player_guid) = host.loot_unit_hub_ref_like_cpp().core.player_guid() else {
        return;
    };

    if host
        .loot_roll_player_vote_like_cpp(item_guid_generator, item_valuation, &roll, player_guid)
        .await
    {
        return;
    }

    if host
        .loot_unit_loot_ref_like_cpp()
        .route_represented_remote_loot_roll_vote_to_owner_like_cpp(
            host.loot_unit_hub_ref_like_cpp(),
            &roll,
            player_guid,
        )
    {
        return;
    }

    debug!(
        account = host.loot_unit_hub_ref_like_cpp().core.account_id,
        loot_obj = ?roll.loot_obj,
        loot_list_id = roll.loot_list_id,
        roll_type = roll.roll_type,
        "CMSG_LOOT_ROLL ignored: canonical LootRoll state is not ported yet"
    );
}

/// CMSG_MASTER_LOOT_ITEM — master looter assigns loot to a target.
///
/// C++ first rejects players that are not in a group or are not the group's
/// master looter with `LOOT_ERROR_DIDNT_KILL`. Current Rust group state has
/// loot method `MASTER_LOOT` and the stored master-looter GUID matching the
/// current player.
async fn handle_master_loot_item_with_generator_like_cpp<H, C>(
    host: &mut H,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    master_loot_item: MasterLootItem,
) where
    H: LootHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    let Some(player_guid) = host.loot_unit_hub_ref_like_cpp().core.player_guid() else {
        return;
    };

    let is_represented_master_looter = if let (Some(group_guid), Some(registry)) = (
        host.master_loot_resolved_group_guid_like_cpp(),
        host.loot_unit_hub_ref_like_cpp().core.group_registry(),
    ) {
        registry.get(&group_guid).is_some_and(|group| {
            group.loot_method == LOOT_METHOD_MASTER_LIKE_CPP
                && group.master_looter_guid == player_guid
        })
    } else {
        false
    };

    if !is_represented_master_looter {
        host.loot_unit_loot_ref_like_cpp().send_loot_error_like_cpp(
            host.loot_unit_hub_ref_like_cpp(),
            ObjectGuid::EMPTY,
            ObjectGuid::EMPTY,
            LOOT_ERROR_DIDNT_KILL_LIKE_CPP,
        );
        return;
    }

    if !host
        .loot_unit_loot_ref_like_cpp()
        .represented_master_loot_target_exists_like_cpp(
            host.loot_unit_hub_ref_like_cpp(),
            master_loot_item.target,
        )
    {
        host.loot_unit_loot_ref_like_cpp().send_loot_error_like_cpp(
            host.loot_unit_hub_ref_like_cpp(),
            ObjectGuid::EMPTY,
            ObjectGuid::EMPTY,
            LOOT_ERROR_PLAYER_NOT_FOUND_LIKE_CPP,
        );
        return;
    }

    let mut current_session_assignments = 0_u32;

    for req in &master_loot_item.loot {
        let Some(owner_guid) = host
            .loot_unit_loot_ref_like_cpp()
            .active_loot_owner_for_loot_object_like_cpp(req.object)
        else {
            return;
        };

        if !represented_master_loot_target_eligible_like_cpp(host, master_loot_item.target) {
            host.loot_unit_loot_ref_like_cpp().send_loot_error_like_cpp(
                host.loot_unit_hub_ref_like_cpp(),
                req.object,
                owner_guid,
                LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
            );
            return;
        }

        let owned_authority =
            host.master_loot_prepare_owned_authority_like_cpp(owner_guid, player_guid);
        let authority = owned_authority
            .as_ref()
            .filter(|authority| {
                authority
                    .snapshot_for_player_like_cpp(master_loot_item.target)
                    .is_some()
            })
            .cloned();
        if authority.is_none()
            && (owner_guid.is_creature_or_vehicle() || owner_guid.is_game_object())
            && (owned_authority.is_some() || !host.master_loot_local_fixture_allowed_like_cpp())
        {
            host.loot_unit_loot_ref_like_cpp().send_loot_error_like_cpp(
                host.loot_unit_hub_ref_like_cpp(),
                req.object,
                owner_guid,
                LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
            );
            return;
        }
        if let Some(authority) = authority.as_ref() {
            if !host
                .loot_unit_loot_ref_like_cpp()
                .represented_active_loot_generation_matches_like_cpp(
                    host.loot_unit_hub_ref_like_cpp(),
                    owner_guid,
                    authority,
                )
            {
                host.loot_unit_loot_ref_like_cpp().send_loot_error_like_cpp(
                    host.loot_unit_hub_ref_like_cpp(),
                    req.object,
                    owner_guid,
                    LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
                );
                return;
            }
            let _ =
                host.master_loot_reconcile_loot_cache_like_cpp(owner_guid, master_loot_item.target);
        }

        let Some(loot) = host
            .loot_unit_loot_ref_like_cpp()
            .cached_loot_for_owner_like_cpp(owner_guid)
        else {
            return;
        };
        let dungeon_encounter_id = loot.dungeon_encounter_id;

        if loot.loot_method != LOOT_METHOD_MASTER_LIKE_CPP {
            return;
        }

        if !loot.allowed_looters.contains(&master_loot_item.target) {
            host.loot_unit_loot_ref_like_cpp().send_loot_error_like_cpp(
                host.loot_unit_hub_ref_like_cpp(),
                req.object,
                owner_guid,
                LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
            );
            return;
        }

        if req.loot_list_id as usize >= loot.items.len() {
            return;
        }

        let item = &loot.items[req.loot_list_id as usize];
        if !item.allowed_looters.is_empty()
            && !item.allowed_looters.contains(&master_loot_item.target)
        {
            host.loot_unit_loot_ref_like_cpp().send_loot_error_like_cpp(
                host.loot_unit_hub_ref_like_cpp(),
                req.object,
                owner_guid,
                LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
            );
            return;
        }

        if let Some(error) = host.master_loot_can_store_error_like_cpp(
            master_loot_item.target,
            item.item_id,
            item.quantity,
        ) {
            host.loot_unit_loot_ref_like_cpp().send_loot_error_like_cpp(
                host.loot_unit_hub_ref_like_cpp(),
                req.object,
                owner_guid,
                error,
            );
            return;
        }

        let mut entry = item.clone();
        let claim = if let Some(authority) = authority {
            let Some(expected_generation) = host
                .loot_unit_loot_ref_like_cpp()
                .active_loot_view_generation_like_cpp(owner_guid)
                .copied()
            else {
                host.loot_unit_loot_ref_like_cpp().send_loot_error_like_cpp(
                    host.loot_unit_hub_ref_like_cpp(),
                    req.object,
                    owner_guid,
                    LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
                );
                return;
            };
            let claim = match authority
                .reserve_item_for_award_generation_like_cpp(
                    master_loot_item.target,
                    req.loot_list_id,
                    expected_generation,
                )
                .await
            {
                Ok(claim) => claim,
                Err(_) => {
                    host.loot_unit_loot_ref_like_cpp().send_loot_error_like_cpp(
                        host.loot_unit_hub_ref_like_cpp(),
                        req.object,
                        owner_guid,
                        LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
                    );
                    return;
                }
            };
            if !host
                .loot_unit_loot_ref_like_cpp()
                .represented_active_loot_claim_generation_matches_like_cpp(owner_guid, &claim)
            {
                claim.rollback_like_cpp();
                host.loot_unit_loot_ref_like_cpp().send_loot_error_like_cpp(
                    host.loot_unit_hub_ref_like_cpp(),
                    req.object,
                    owner_guid,
                    LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
                );
                return;
            }
            if let LootClaimPayload::Item(reserved_entry) = claim.payload_like_cpp() {
                entry = reserved_entry.clone();
            }
            Some(claim)
        } else {
            None
        };
        if master_loot_item.target == player_guid {
            let stored = if let Some(claim) = claim.as_ref() {
                host.master_loot_store_claimed_direct_item_like_cpp(
                    item_guid_generator,
                    &entry,
                    dungeon_encounter_id,
                    owner_guid,
                    req.object,
                    claim,
                )
                .await
            } else {
                host.master_loot_store_direct_item_like_cpp(
                    item_guid_generator,
                    &entry,
                    dungeon_encounter_id,
                    owner_guid,
                )
                .await
            };
            if !stored {
                return;
            }
            if claim.is_none() {
                host.master_loot_mark_item_removed_like_cpp(
                    owner_guid,
                    req.object,
                    req.loot_list_id,
                    master_loot_item.target,
                );
            }
            current_session_assignments = current_session_assignments.saturating_add(1);
        } else {
            let authoritative_claim = claim.is_some();
            match host
                .loot_unit_loot_ref_like_cpp()
                .request_represented_remote_master_loot_give_like_cpp(
                    host.loot_unit_hub_ref_like_cpp(),
                    master_loot_item.target,
                    owner_guid,
                    req.object,
                    req.loot_list_id,
                    dungeon_encounter_id,
                    entry,
                    claim,
                )
                .await
            {
                MasterLootGiveResult::Stored if !authoritative_claim => {
                    host.master_loot_mark_item_removed_like_cpp(
                        owner_guid,
                        req.object,
                        req.loot_list_id,
                        master_loot_item.target,
                    );
                }
                MasterLootGiveResult::Stored => {}
                MasterLootGiveResult::StoreFailed(error) => {
                    host.loot_unit_loot_ref_like_cpp().send_loot_error_like_cpp(
                        host.loot_unit_hub_ref_like_cpp(),
                        req.object,
                        owner_guid,
                        error,
                    );
                    return;
                }
                MasterLootGiveResult::TargetMismatch => {
                    host.loot_unit_loot_ref_like_cpp().send_loot_error_like_cpp(
                        host.loot_unit_hub_ref_like_cpp(),
                        ObjectGuid::EMPTY,
                        ObjectGuid::EMPTY,
                        LOOT_ERROR_PLAYER_NOT_FOUND_LIKE_CPP,
                    );
                    return;
                }
            }
        }
    }

    debug!(
        account = host.loot_unit_hub_ref_like_cpp().core.account_id,
        target = ?master_loot_item.target,
        request_count = master_loot_item.loot.len(),
        current_session_assignments,
        "CMSG_MASTER_LOOT_ITEM accepted; represented self and connected remote target assignments route through target session state"
    );
}

/// C++ `Group::IsMember`-style target gate of `HandleLootMasterGiveOpcode`;
/// moved with the handler from the World loot tree, whose only caller it was.
fn represented_master_loot_target_eligible_like_cpp<H, C>(host: &H, target: ObjectGuid) -> bool
where
    H: LootHandlerHostLikeCpp<C>,
{
    let Some(group_guid) = host.master_loot_resolved_group_guid_like_cpp() else {
        return false;
    };

    let Some(group_registry) = host.loot_unit_hub_ref_like_cpp().core.group_registry() else {
        return false;
    };

    group_registry
        .get(&group_guid)
        .is_some_and(|group| group.members.contains(&target))
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
