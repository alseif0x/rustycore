// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Battle-pet handlers for the C++ `BattlePetHandler.cpp` family.
//!
//! C++ source of truth: `src/server/game/Handlers/BattlePetHandler.cpp`
//! (`HandleBattlePetRequestJournal`, `HandleBattlePetRequestJournalLock`,
//! `HandleBattlePetSetBattleSlot`, `HandleQueryBattlePetName`,
//! `HandleBattlePetSetFlags`, `HandleBattlePetClearFanfare`,
//! `HandleBattlePetSummon` and `HandleBattlePetUpdateNotify`). The family owns
//! the packet bodies plus the represented journal, journal-lock, battle-slot and
//! companion-publication transitions they drive. The account-scoped battle-pet
//! owner, the DB2 catalogs and the realm/instance packet publication already
//! live in the Core hub, so the World session only builds the borrowed context
//! (#1263 F5).
//!
//! `DismissCritter` (C++ `PetHandler.cpp`, a different family) stays in the World
//! shell. `BattlePetUpdateDisplayNotify` is not registered anywhere: 3.4.3 leaves
//! it `STATUS_UNHANDLED` / `Handle_NULL` (`Opcodes.cpp:243`), and the 2026-10-07
//! #1263 F6 decision (D5) removed the empty registered body.

use std::sync::Arc;

use tracing::warn;
use wow_constants::ClientOpcodes;
use wow_core::ObjectGuid;
use wow_handler::{
    DuplicateHandlerRegistrationLikeCpp, HandlerFuture, PacketHandlerEntry, PacketProcessing,
    RegistryBuilder, SessionStatus,
};
use wow_packet::packets::misc::{
    BattlePetClearFanfare, BattlePetJournal, BattlePetJournalLockAcquired,
    BattlePetJournalLockDenied, BattlePetRequestJournal, BattlePetSetBattleSlot, BattlePetSetFlags,
    BattlePetSummon, BattlePetUpdateNotify, QueryBattlePetName, QueryBattlePetNameResponse,
};
use wow_packet::{ClientPacket, WorldPacket};
use wow_world_core::session::battle_pet_adapter::BATTLE_PET_FLAGS_CONTROL_TYPE_APPLY_LIKE_CPP;
use wow_world_core::session::{
    BATTLE_PET_FLAG_FANFARE_NEEDED_LIKE_CPP, HubMut, HubRef, RepresentedBattlePetDataLikeCpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
use wow_world_core::session::{
    BATTLE_PET_SLOT_COUNT_LIKE_CPP, RepresentedBattlePetSaveInfoLikeCpp,
};
use wow_world_lifecycle::SessionLifecycleState;

/// C++ `BattlePetMgr::HasJournalLock`.
pub fn has_represented_battle_pet_journal_lock_like_cpp(
    hub: HubRef<'_>,
    lifecycle: &SessionLifecycleState,
    world_test_consumer: bool,
) -> bool {
    if let Some(attachment) = lifecycle.battle_pet_account_attachment_like_cpp() {
        return attachment.has_lease_like_cpp();
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    if world_test_consumer {
        return hub
            .fixtures
            .pets
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pet_journal_lock_like_cpp;
    }
    #[cfg(not(any(test, feature = "test-fixtures")))]
    let _ = (hub, world_test_consumer);
    false
}

/// C++ `BattlePetMgr::SendJournal` packet body builder.
pub fn represented_battle_pet_journal_like_cpp(
    hub: HubRef<'_>,
    lifecycle: &SessionLifecycleState,
    world_test_consumer: bool,
) -> Option<BattlePetJournal> {
    if let Some(attachment) = lifecycle.battle_pet_account_attachment_like_cpp() {
        return Some(
            attachment
                .owner_like_cpp()
                .journal_like_cpp(attachment.lease_id_like_cpp(), hub.core.player_guid()),
        );
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    if world_test_consumer {
        let player_guid = hub.core.player_guid();
        let mut journal = BattlePetJournal {
            trap: 0,
            has_journal_lock: has_represented_battle_pet_journal_lock_like_cpp(
                hub,
                lifecycle,
                world_test_consumer,
            ),
            slots: Vec::with_capacity(BATTLE_PET_SLOT_COUNT_LIKE_CPP),
            pets: Vec::new(),
        };

        for (pet_guid, pet) in &hub
            .fixtures
            .pets
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pets_like_cpp
        {
            if pet.save_info == RepresentedBattlePetSaveInfoLikeCpp::Removed {
                continue;
            }

            if pet
                .owner_info
                .is_some_and(|owner_info| Some(owner_info.guid) != player_guid)
            {
                continue;
            }

            journal.pets.push(pet.packet_info_like_cpp(*pet_guid));
        }

        for slot in &hub
            .fixtures
            .pets
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pet_slots_like_cpp
        {
            let mut packet_slot = slot.packet_slot_like_cpp();
            if packet_slot.pet_guid != wow_packet::packets::misc::empty_battle_pet_guid_like_cpp()
                && !hub
                    .fixtures
                    .pets
                    .battle_pet_test_fixture_like_cpp
                    .represented_battle_pets_like_cpp
                    .contains_key(&packet_slot.pet_guid)
            {
                packet_slot.pet_guid = wow_packet::packets::misc::empty_battle_pet_guid_like_cpp();
            }
            journal.slots.push(packet_slot);
        }

        return Some(journal);
    }
    #[cfg(not(any(test, feature = "test-fixtures")))]
    let _ = world_test_consumer;
    None
}

/// One represented battle pet from the canonical owner or the World-test fixture.
pub fn represented_battle_pet_like_cpp(
    hub: HubRef<'_>,
    lifecycle: &SessionLifecycleState,
    world_test_consumer: bool,
    pet_guid: ObjectGuid,
) -> Option<RepresentedBattlePetDataLikeCpp> {
    if let Some(attachment) = lifecycle.battle_pet_account_attachment_like_cpp() {
        return attachment.owner_like_cpp().pet_snapshot_like_cpp(pet_guid);
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    if world_test_consumer {
        return hub
            .fixtures
            .pets
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pets_like_cpp
            .get(&pet_guid)
            .cloned();
    }
    #[cfg(not(any(test, feature = "test-fixtures")))]
    let _ = (hub, world_test_consumer);
    None
}

/// C++ `BattlePetMgr::SendJournalLockStatus`, represented as the successful
/// local acquisition path until the global world journal-lock owner exists.
pub async fn send_battle_pet_journal_lock_status_like_cpp(
    hub: &mut HubMut<'_>,
    lifecycle: &SessionLifecycleState,
    world_test_consumer: bool,
) {
    if let Some(attachment) = lifecycle.battle_pet_account_attachment_like_cpp() {
        let acquired = attachment.try_acquire_lease_like_cpp().await;
        if acquired {
            hub.core.send_packet_realm(&BattlePetJournalLockAcquired);
        } else {
            hub.core.send_packet_realm(&BattlePetJournalLockDenied);
        }
        return;
    }
    #[cfg(any(test, feature = "test-fixtures"))]
    if world_test_consumer {
        hub.fixtures
            .pets
            .battle_pet_test_fixture_like_cpp
            .represented_battle_pet_journal_lock_like_cpp = true;
        hub.core.send_packet_realm(&BattlePetJournalLockAcquired);
        return;
    }
    #[cfg(not(any(test, feature = "test-fixtures")))]
    let _ = world_test_consumer;
    hub.core.send_packet_realm(&BattlePetJournalLockDenied);
}

/// Borrowed inputs of one battle-pet handler invocation.
pub struct BattlePetHandlerCxLikeCpp<'a> {
    hub: HubMut<'a>,
    lifecycle: &'a SessionLifecycleState,
    /// The host's World-test flag (World passes `cfg!(test)`).
    world_test_consumer: bool,
}

impl<'a> BattlePetHandlerCxLikeCpp<'a> {
    pub fn new(
        hub: HubMut<'a>,
        lifecycle: &'a SessionLifecycleState,
        world_test_consumer: bool,
    ) -> Self {
        Self {
            hub,
            lifecycle,
            world_test_consumer,
        }
    }

    /// C++ `BattlePetMgr::HasJournalLock`.
    fn has_represented_battle_pet_journal_lock_like_cpp(&self) -> bool {
        has_represented_battle_pet_journal_lock_like_cpp(
            self.hub.shared(),
            self.lifecycle,
            self.world_test_consumer,
        )
    }

    /// C++ `BattlePetMgr::SendJournal` packet body builder.
    fn represented_battle_pet_journal_like_cpp(&self) -> Option<BattlePetJournal> {
        represented_battle_pet_journal_like_cpp(
            self.hub.shared(),
            self.lifecycle,
            self.world_test_consumer,
        )
    }

    fn represented_battle_pet_like_cpp(
        &self,
        pet_guid: ObjectGuid,
    ) -> Option<RepresentedBattlePetDataLikeCpp> {
        represented_battle_pet_like_cpp(
            self.hub.shared(),
            self.lifecycle,
            self.world_test_consumer,
            pet_guid,
        )
    }

    /// C++ `BattlePetMgr::SendJournalLockStatus`.
    async fn send_battle_pet_journal_lock_status_like_cpp(&mut self) {
        send_battle_pet_journal_lock_status_like_cpp(
            &mut self.hub,
            self.lifecycle,
            self.world_test_consumer,
        )
        .await;
    }

    /// CMSG_BATTLE_PET_REQUEST_JOURNAL — send represented journal.
    ///
    /// C++ `BattlePetMgr::SendJournal` first acquires/sends journal-lock status
    /// when needed, then sends `SMSG_BATTLE_PET_JOURNAL`.
    pub async fn handle_battle_pet_request_journal(&mut self, mut pkt: WorldPacket) {
        if let Err(error) = BattlePetRequestJournal::read(&mut pkt) {
            warn!(
                account = self.hub.shared().core.account_id,
                "BattlePetRequestJournal parse failed: {error}"
            );
            return;
        }

        if !self.has_represented_battle_pet_journal_lock_like_cpp() {
            self.send_battle_pet_journal_lock_status_like_cpp().await;
        }

        if let Some(journal) = self.represented_battle_pet_journal_like_cpp() {
            self.hub.core.send_packet_realm(&journal);
        }
    }

    /// CMSG_BATTLE_PET_REQUEST_JOURNAL_LOCK — acquire represented journal lock.
    ///
    /// C++ `HandleBattlePetRequestJournalLock` sends lock status and, when the
    /// lock is held, sends the journal.
    pub async fn handle_battle_pet_request_journal_lock(&mut self, _pkt: WorldPacket) {
        self.send_battle_pet_journal_lock_status_like_cpp().await;
        if self.has_represented_battle_pet_journal_lock_like_cpp() {
            if let Some(journal) = self.represented_battle_pet_journal_like_cpp() {
                self.hub.core.send_packet_realm(&journal);
            }
        }
    }

    /// CMSG_BATTLE_PET_CLEAR_FANFARE — clear the account battle-pet fanfare bit.
    ///
    /// C++ ref: `WorldSession::HandleBattlePetClearFanfare` forwards only the
    /// pet guid to `BattlePetMgr::ClearFanfare`, which silently ignores unknown
    /// pets.
    pub async fn handle_battle_pet_clear_fanfare(&mut self, mut pkt: WorldPacket) {
        let request = match BattlePetClearFanfare::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "BattlePetClearFanfare parse failed: {error}"
                );
                return;
            }
        };

        self.battle_pet_clear_fanfare_durable_like_cpp(request.pet_guid)
            .await;
    }

    /// CMSG_BATTLE_PET_SET_FLAGS — apply/remove represented battle-pet flags.
    ///
    /// C++ first requires the journal lock and then silently ignores unknown
    /// pets.
    pub async fn handle_battle_pet_set_flags(&mut self, mut pkt: WorldPacket) {
        let request = match BattlePetSetFlags::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "BattlePetSetFlags parse failed: {error}"
                );
                return;
            }
        };

        if !self.has_represented_battle_pet_journal_lock_like_cpp() {
            return;
        }

        self.battle_pet_set_flags_durable_like_cpp(
            request.pet_guid,
            request.flags,
            request.control_type,
        )
        .await;
    }

    /// CMSG_BATTLE_PET_SET_BATTLE_SLOT — assign an owned pet to a battle slot.
    ///
    /// C++ silently ignores unknown pets and invalid slots.
    pub async fn handle_battle_pet_set_battle_slot(&mut self, mut pkt: WorldPacket) {
        let request = match BattlePetSetBattleSlot::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "BattlePetSetBattleSlot parse failed: {error}"
                );
                return;
            }
        };

        self.battle_pet_set_battle_slot_durable_like_cpp(request.pet_guid, request.slot)
            .await;
    }

    /// CMSG_BATTLE_PET_SUMMON — toggle represented summoned battle-pet guid.
    ///
    /// C++ compares `ActivePlayerData::SummonedBattlePetGUID`; unknown pets are
    /// ignored by `BattlePetMgr::SummonPet`, and matching active pets dismiss.
    /// Full spell cast, creature summon/despawn and `SetBattlePetData` update
    /// fields remain part of the later live battle-pet runtime.
    pub async fn handle_battle_pet_summon(&mut self, mut pkt: WorldPacket) {
        let request = match BattlePetSummon::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "BattlePetSummon parse failed: {error}"
                );
                return;
            }
        };

        self.battle_pet_summon_toggle_like_cpp(request.pet_guid);
    }

    /// CMSG_BATTLE_PET_UPDATE_NOTIFY — represented update of active companion data.
    ///
    /// C++ `BattlePetMgr::UpdateBattlePetData` ignores unknown pets and only
    /// updates player/summoned-creature battle-pet fields when the currently
    /// summoned companion GUID matches the requested pet GUID.
    pub async fn handle_battle_pet_update_notify(&mut self, mut pkt: WorldPacket) {
        let request = match BattlePetUpdateNotify::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "BattlePetUpdateNotify parse failed: {error}"
                );
                return;
            }
        };

        self.battle_pet_update_notify_like_cpp(request.pet_guid);
    }

    /// CMSG_QUERY_BATTLE_PET_NAME — represented summoned-companion name lookup.
    ///
    /// C++ first resolves the requested unit through ObjectAccessor and requires
    /// a summon. Only after that does it copy `CreatureID` and companion-name
    /// timestamp, then it gates on player owner, known battle-pet row, and a
    /// non-empty name before setting `Allow=true`.
    pub async fn handle_query_battle_pet_name(&mut self, mut pkt: WorldPacket) {
        let request = match QueryBattlePetName::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.hub.shared().core.account_id,
                    "QueryBattlePetName parse failed: {error}"
                );
                return;
            }
        };

        let Some(companion) = self
            .hub
            .shared()
            .represented_battle_pet_query_companion_like_cpp(request.unit_guid)
        else {
            self.hub
                .core
                .send_packet(&QueryBattlePetNameResponse::not_allowed(
                    request.battle_pet_id,
                ));
            return;
        };

        if !companion.is_summon {
            self.hub
                .core
                .send_packet(&QueryBattlePetNameResponse::not_allowed(
                    request.battle_pet_id,
                ));
            return;
        }

        let mut response = QueryBattlePetNameResponse {
            battle_pet_id: request.battle_pet_id,
            creature_id: companion.creature_id,
            timestamp: companion.name_timestamp,
            allow: false,
            name: String::new(),
            declined_names: None,
        };

        if companion.owner_is_player {
            if let Some(pet) = self.represented_battle_pet_like_cpp(request.battle_pet_id) {
                response.name = pet.name;
                response.declined_names = pet.declined_names;
                response.allow = !response.name.is_empty();
            }
        }

        self.hub.core.send_packet(&response);
    }

    async fn battle_pet_clear_fanfare_durable_like_cpp(&mut self, pet_guid: ObjectGuid) -> bool {
        let Some(attachment) = self.lifecycle.battle_pet_account_attachment_like_cpp() else {
            #[cfg(any(test, feature = "test-fixtures"))]
            if self.world_test_consumer {
                return self
                    .hub
                    .fixtures
                    .pets
                    .battle_pet_clear_fanfare_like_cpp(pet_guid);
            }
            return false;
        };
        let owner = Arc::clone(attachment.owner_like_cpp());
        match owner
            .try_mutate_pet_without_lease_like_cpp(pet_guid, |pet| {
                pet.flags &= !BATTLE_PET_FLAG_FANFARE_NEEDED_LIKE_CPP;
            })
            .await
        {
            Ok(_) => true,
            Err(error) => {
                self.hub.shared().log_battle_pet_mutation_failure_like_cpp(
                    "clear fanfare",
                    pet_guid,
                    &error,
                );
                false
            }
        }
    }

    async fn battle_pet_set_flags_durable_like_cpp(
        &mut self,
        pet_guid: ObjectGuid,
        flags: u16,
        control_type: u8,
    ) -> bool {
        let Some(attachment) = self.lifecycle.battle_pet_account_attachment_like_cpp() else {
            #[cfg(any(test, feature = "test-fixtures"))]
            if self.world_test_consumer {
                return self.hub.fixtures.pets.battle_pet_set_flags_like_cpp(
                    pet_guid,
                    flags,
                    control_type,
                );
            }
            return false;
        };
        let owner = Arc::clone(attachment.owner_like_cpp());
        let lease = attachment.lease_id_like_cpp();
        match owner
            .try_mutate_pet_like_cpp(lease, pet_guid, move |pet| {
                if control_type == BATTLE_PET_FLAGS_CONTROL_TYPE_APPLY_LIKE_CPP {
                    pet.flags |= flags;
                } else {
                    pet.flags &= !flags;
                }
            })
            .await
        {
            Ok(_) => true,
            Err(error) => {
                self.hub.shared().log_battle_pet_mutation_failure_like_cpp(
                    "set flags",
                    pet_guid,
                    &error,
                );
                false
            }
        }
    }

    async fn battle_pet_set_battle_slot_durable_like_cpp(
        &mut self,
        pet_guid: ObjectGuid,
        slot: u8,
    ) -> bool {
        let Some(attachment) = self.lifecycle.battle_pet_account_attachment_like_cpp() else {
            #[cfg(any(test, feature = "test-fixtures"))]
            if self.world_test_consumer {
                return self.hub.battle_pet_set_battle_slot_like_cpp(pet_guid, slot);
            }
            return false;
        };
        let owner = Arc::clone(attachment.owner_like_cpp());
        let lease = attachment.lease_id_like_cpp();
        match owner.try_set_slot_like_cpp(lease, pet_guid, slot).await {
            Ok(_) => {
                self.hub
                    .core
                    .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
                true
            }
            Err(error) => {
                self.hub.shared().log_battle_pet_mutation_failure_like_cpp(
                    "set battle slot",
                    pet_guid,
                    &error,
                );
                false
            }
        }
    }

    /// C++ `WorldSession::HandleBattlePetSummon` represented toggle.
    ///
    /// `BattlePetMgr::SummonPet` silently ignores unknown pets before casting
    /// the summon spell; `DismissPet` clears the active summoned companion.
    /// Public because the World test shims still drive the represented toggle
    /// directly; the packet body above is the only production caller.
    pub fn battle_pet_summon_toggle_like_cpp(&mut self, pet_guid: ObjectGuid) -> bool {
        if self
            .hub
            .shared()
            .represented_summoned_battle_pet_guid_like_cpp()
            == Some(pet_guid)
        {
            return self
                .hub
                .set_represented_summoned_battle_pet_guid_like_cpp(None);
        }

        if self.represented_battle_pet_like_cpp(pet_guid).is_none() {
            return false;
        }

        self.hub
            .set_represented_summoned_battle_pet_guid_like_cpp(Some(pet_guid))
    }

    /// C++ `BattlePetMgr::UpdateBattlePetData`, represented at the gate level.
    ///
    /// Public because the World test shims still drive the represented update
    /// directly; the packet body above is the only production caller.
    pub fn battle_pet_update_notify_like_cpp(&mut self, pet_guid: ObjectGuid) -> bool {
        let Some(pet) = self.represented_battle_pet_like_cpp(pet_guid) else {
            return false;
        };

        if self
            .hub
            .shared()
            .represented_summoned_battle_pet_guid_like_cpp()
            != Some(pet_guid)
        {
            return false;
        }

        let _ = self.hub.core.mutate_canonical_player_like_cpp(|player| {
            player.set_battle_pet_data_like_cpp(pet_guid, pet.quality, pet.level);
        });
        #[cfg(any(test, feature = "test-fixtures"))]
        if self.world_test_consumer {
            self.hub
                .fixtures
                .pets
                .battle_pet_test_fixture_like_cpp
                .represented_battle_pet_data_updates_like_cpp
                .push(pet_guid);
        }
        true
    }
}

/// Builds a battle-pet handler context from a host's hub.
pub trait BattlePetHandlerHostLikeCpp<C> {
    fn battle_pet_handler_cx_like_cpp<'a>(
        &'a mut self,
        catalogs: &'a C,
    ) -> BattlePetHandlerCxLikeCpp<'a>;
}

fn handle_battle_pet_request_journal_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: BattlePetHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .battle_pet_handler_cx_like_cpp(catalogs)
            .handle_battle_pet_request_journal(pkt)
            .await;
    })
}

fn handle_battle_pet_request_journal_lock_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: BattlePetHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .battle_pet_handler_cx_like_cpp(catalogs)
            .handle_battle_pet_request_journal_lock(pkt)
            .await;
    })
}

fn handle_battle_pet_clear_fanfare_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: BattlePetHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .battle_pet_handler_cx_like_cpp(catalogs)
            .handle_battle_pet_clear_fanfare(pkt)
            .await;
    })
}

fn handle_battle_pet_set_flags_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: BattlePetHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .battle_pet_handler_cx_like_cpp(catalogs)
            .handle_battle_pet_set_flags(pkt)
            .await;
    })
}

fn handle_battle_pet_set_battle_slot_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: BattlePetHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .battle_pet_handler_cx_like_cpp(catalogs)
            .handle_battle_pet_set_battle_slot(pkt)
            .await;
    })
}

fn handle_battle_pet_summon_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: BattlePetHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .battle_pet_handler_cx_like_cpp(catalogs)
            .handle_battle_pet_summon(pkt)
            .await;
    })
}

fn handle_battle_pet_update_notify_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: BattlePetHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .battle_pet_handler_cx_like_cpp(catalogs)
            .handle_battle_pet_update_notify(pkt)
            .await;
    })
}

fn handle_query_battle_pet_name_thunk<'a, S, C>(
    session: &'a mut S,
    catalogs: &'a C,
    pkt: WorldPacket,
) -> HandlerFuture<'a, ()>
where
    S: BattlePetHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    Box::pin(async move {
        session
            .battle_pet_handler_cx_like_cpp(catalogs)
            .handle_query_battle_pet_name(pkt)
            .await;
    })
}

/// Registers the C++ `BattlePetHandler.cpp` family on the packet registry.
pub fn register_battle_pet_handlers_like_cpp<S, C>(
    builder: &mut RegistryBuilder<S, C>,
) -> Result<(), DuplicateHandlerRegistrationLikeCpp>
where
    S: BattlePetHandlerHostLikeCpp<C> + Send,
    C: Sync,
{
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::BattlePetRequestJournal,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battle_pet_request_journal",
        handler: handle_battle_pet_request_journal_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::BattlePetRequestJournalLock,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battle_pet_request_journal_lock",
        handler: handle_battle_pet_request_journal_lock_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::BattlePetClearFanfare,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battle_pet_clear_fanfare",
        handler: handle_battle_pet_clear_fanfare_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::BattlePetSetFlags,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battle_pet_set_flags",
        handler: handle_battle_pet_set_flags_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::BattlePetSetBattleSlot,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battle_pet_set_battle_slot",
        handler: handle_battle_pet_set_battle_slot_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::BattlePetSummon,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_battle_pet_summon",
        handler: handle_battle_pet_summon_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::BattlePetUpdateNotify,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battle_pet_update_notify",
        handler: handle_battle_pet_update_notify_thunk::<S, C>,
    })?;
    builder.register(PacketHandlerEntry {
        opcode: ClientOpcodes::QueryBattlePetName,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_query_battle_pet_name",
        handler: handle_query_battle_pet_name_thunk::<S, C>,
    })?;
    Ok(())
}
