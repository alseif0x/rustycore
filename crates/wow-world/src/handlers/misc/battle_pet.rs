// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private battle_pet capability handlers extracted from the legacy misc owner.

use tracing::warn;
use wow_constants::ClientOpcodes;
use wow_core::GameTime;
use wow_handler::{PacketHandlerEntry, PacketProcessing, SessionStatus};
use wow_packet::ClientPacket;
use wow_packet::packets::misc::{
    BattlePetClearFanfare, BattlePetDeletePet, BattlePetModifyName, BattlePetRequestJournal,
    BattlePetSetBattleSlot, BattlePetSetFlags, BattlePetSummon, BattlePetUpdateNotify,
    CageBattlePet, QueryBattlePetName, QueryBattlePetNameResponse,
};
use wow_packet::packets::pet::DismissCritter;

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BattlePetRequestJournal,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battle_pet_request_journal",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BattlePetRequestJournalLock,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battle_pet_request_journal_lock",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BattlePetClearFanfare,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battle_pet_clear_fanfare",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BattlePetSetFlags,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battle_pet_set_flags",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BattlePetSetBattleSlot,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battle_pet_set_battle_slot",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BattlePetSummon,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_battle_pet_summon",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BattlePetUpdateNotify,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battle_pet_update_notify",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::BattlePetUpdateDisplayNotify,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_battle_pet_update_display_notify",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::DismissCritter,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_dismiss_critter",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryBattlePetName,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_query_battle_pet_name",
    }
}

impl crate::session::WorldSession {
    /// CMSG_BATTLE_PET_REQUEST_JOURNAL — send represented journal.
    ///
    /// C++ `BattlePetMgr::SendJournal` first acquires/sends journal-lock status
    /// when needed, then sends `SMSG_BATTLE_PET_JOURNAL`.
    pub async fn handle_battle_pet_request_journal(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = BattlePetRequestJournal::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "BattlePetRequestJournal parse failed: {error}"
            );
            return;
        }

        if !self.has_represented_battle_pet_journal_lock_like_cpp() {
            self.send_battle_pet_journal_lock_status_like_cpp().await;
        }

        self.send_packet_realm(&self.represented_battle_pet_journal_like_cpp());
    }

    /// CMSG_BATTLE_PET_REQUEST_JOURNAL_LOCK — acquire represented journal lock.
    ///
    /// C++ `HandleBattlePetRequestJournalLock` sends lock status and, when the
    /// lock is held, sends the journal.

    pub async fn handle_battle_pet_request_journal_lock(&mut self, _pkt: wow_packet::WorldPacket) {
        self.send_battle_pet_journal_lock_status_like_cpp().await;
        if self.has_represented_battle_pet_journal_lock_like_cpp() {
            self.send_packet_realm(&self.represented_battle_pet_journal_like_cpp());
        }
    }

    /// CMSG_BATTLE_PET_CLEAR_FANFARE — clear the account battle-pet fanfare bit.
    ///
    /// C++ ref: `WorldSession::HandleBattlePetClearFanfare` forwards only the
    /// pet guid to `BattlePetMgr::ClearFanfare`, which silently ignores unknown
    /// pets.

    pub async fn handle_battle_pet_clear_fanfare(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match BattlePetClearFanfare::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "BattlePetClearFanfare parse failed: {error}"
                );
                return;
            }
        };

        self.battle_pet_clear_fanfare_durable_like_cpp(request.pet_guid)
            .await;
    }

    /// CMSG_BATTLE_PET_DELETE_PET — represented battle-pet removal body.
    ///
    /// C++ registers this handler and forwards only the pet guid to
    /// `BattlePetMgr::RemovePet`, which requires the journal lock and silently
    /// ignores unknown pets. The archived opcode id is the unresolved `0xBADD`
    /// placeholder, so this method is intentionally not registered for
    /// production dispatch until the real client opcode is known.

    pub async fn handle_battle_pet_delete_pet_represented_like_cpp(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
    ) {
        let request = match BattlePetDeletePet::read_like_cpp(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "BattlePetDeletePet parse failed: {error}"
                );
                return;
            }
        };

        self.battle_pet_remove_pet_durable_like_cpp(request.pet_guid)
            .await;
    }

    /// CMSG_CAGE_BATTLE_PET — represented cage body.
    ///
    /// C++ registers this handler and forwards only the pet guid to
    /// `BattlePetMgr::CageBattlePet`. The manager then performs the journal,
    /// species, slot, health, inventory, item-store, remove, deleted-packet,
    /// and summoned-companion gates. The archived opcode id is still the
    /// unresolved `0xBADD` placeholder, so this method remains intentionally
    /// unregistered for production dispatch. Until the real inventory path is
    /// wired, this represented body exercises the successful inventory seam.

    pub async fn handle_cage_battle_pet_represented_like_cpp(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
    ) {
        let request = match CageBattlePet::read_like_cpp(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "CageBattlePet parse failed: {error}"
                );
                return;
            }
        };

        let _ = self.battle_pet_cage_battle_pet_represented_like_cpp(request.pet_guid, true, true);
    }

    /// CMSG_BATTLE_PET_MODIFY_NAME — represented rename body.
    ///
    /// C++ registers this handler and forwards the parsed guid/name/declined
    /// names to `BattlePetMgr::ModifyName`, which stamps `GameTime::GetGameTime`
    /// inside the manager. The archived opcode id remains the unresolved
    /// `0xBADD` placeholder, so this method is intentionally not registered for
    /// production dispatch until the real client opcode is known.

    pub async fn handle_battle_pet_modify_name_represented_like_cpp(
        &mut self,
        mut pkt: wow_packet::WorldPacket,
    ) {
        let request = match BattlePetModifyName::read_like_cpp(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "BattlePetModifyName parse failed: {error}"
                );
                return;
            }
        };

        let timestamp = i64::try_from(GameTime::now().as_secs()).unwrap_or(i64::MAX);
        let _ = self
            .battle_pet_modify_name_durable_like_cpp(
                request.pet_guid,
                request.name,
                request.declined_names,
                timestamp,
            )
            .await;
    }

    /// CMSG_BATTLE_PET_SET_FLAGS — apply/remove represented battle-pet flags.
    ///
    /// C++ first requires the journal lock and then silently ignores unknown
    /// pets.

    pub async fn handle_battle_pet_set_flags(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match BattlePetSetFlags::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
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

    pub async fn handle_battle_pet_set_battle_slot(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match BattlePetSetBattleSlot::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
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

    pub async fn handle_battle_pet_summon(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match BattlePetSummon::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
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

    pub async fn handle_battle_pet_update_notify(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match BattlePetUpdateNotify::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "BattlePetUpdateNotify parse failed: {error}"
                );
                return;
            }
        };

        self.battle_pet_update_notify_like_cpp(request.pet_guid);
    }

    /// CMSG_BATTLE_PET_UPDATE_DISPLAY_NOTIFY — explicit no-op.
    ///
    /// C++ registers this opcode as `STATUS_UNHANDLED` and dispatches it to
    /// `Handle_NULL`, so Rust intentionally performs no read or mutation.

    pub async fn handle_battle_pet_update_display_notify(&mut self, _pkt: wow_packet::WorldPacket) {
    }

    /// CMSG_DISMISS_CRITTER — represented companion dismissal.
    ///
    /// C++ reads a full `CritterGUID`, silently ignores missing/non-active
    /// critters, and sends no direct response. Real `TempSummon::UnSummon` and
    /// object update/despawn fanout remain part of the live companion runtime.

    pub async fn handle_dismiss_critter(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match DismissCritter::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "DismissCritter parse failed: {error}"
                );
                return;
            }
        };

        self.represented_dismiss_critter_like_cpp(request.critter_guid);
    }

    /// CMSG_QUERY_BATTLE_PET_NAME — represented summoned-companion name lookup.
    ///
    /// C++ first resolves the requested unit through ObjectAccessor and requires
    /// a summon. Only after that does it copy `CreatureID` and companion-name
    /// timestamp, then it gates on player owner, known battle-pet row, and a
    /// non-empty name before setting `Allow=true`.

    pub async fn handle_query_battle_pet_name(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match QueryBattlePetName::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "QueryBattlePetName parse failed: {error}"
                );
                return;
            }
        };

        let Some(companion) =
            self.represented_battle_pet_query_companion_like_cpp(request.unit_guid)
        else {
            self.send_packet(&QueryBattlePetNameResponse::not_allowed(
                request.battle_pet_id,
            ));
            return;
        };

        if !companion.is_summon {
            self.send_packet(&QueryBattlePetNameResponse::not_allowed(
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

        self.send_packet(&response);
    }
}
