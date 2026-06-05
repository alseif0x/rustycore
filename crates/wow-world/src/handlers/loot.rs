// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot packet handlers — CMSG_LOOT_UNIT, CMSG_LOOT_ITEM, CMSG_LOOT_RELEASE.
//!
//! Reference: C# Game/Handlers/LootHandler.cs

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use rand::{
    Rng,
    distributions::{Distribution, WeightedIndex},
};
use tokio::time::timeout;
use tracing::{debug, info, warn};

use wow_constants::{
    ClientOpcodes, InventoryResult, InventoryType, ItemContext, ItemFlags, ItemFlags2, ItemQuality,
};
use wow_core::{ObjectGuid, guid::HighGuid};
use wow_data::{ItemRandomEnchantmentTemplateEntry, ItemRandomPropertyTemplateEntry};
use wow_database::{CharStatements, SqlTransaction, WorldStatements};
use wow_entities::{
    AccessorObjectKind, GAMEOBJECT_TYPE_AREADAMAGE, GAMEOBJECT_TYPE_BARBER_CHAIR,
    GAMEOBJECT_TYPE_BINDER, GAMEOBJECT_TYPE_CAMERA, GAMEOBJECT_TYPE_CHAIR, GAMEOBJECT_TYPE_CHEST,
    GAMEOBJECT_TYPE_DESTRUCTIBLE_BUILDING, GAMEOBJECT_TYPE_DOOR,
    GAMEOBJECT_TYPE_DUNGEON_DIFFICULTY, GAMEOBJECT_TYPE_FISHING_HOLE, GAMEOBJECT_TYPE_FISHING_NODE,
    GAMEOBJECT_TYPE_FLAGDROP, GAMEOBJECT_TYPE_FLAGSTAND, GAMEOBJECT_TYPE_GATHERING_NODE,
    GAMEOBJECT_TYPE_GOOBER, GAMEOBJECT_TYPE_GUILD_BANK, GAMEOBJECT_TYPE_MAILBOX,
    GAMEOBJECT_TYPE_MAP_OBJECT, GAMEOBJECT_TYPE_MINI_GAME, GAMEOBJECT_TYPE_QUESTGIVER,
    GAMEOBJECT_TYPE_TEXT, GO_DYNFLAG_LO_NO_INTERACT, GameObjectLootSource, GameObjectOwnedLoot,
    GatheringNodeUseSource, GoState, INVENTORY_DEFAULT_SIZE, INVENTORY_SLOT_BAG_0,
    INVENTORY_SLOT_ITEM_END, INVENTORY_SLOT_ITEM_START, Item, ItemPosCount, LootState,
    make_item_pos,
};
use wow_handler::{PacketHandlerEntry, PacketProcessing, SessionStatus};
use wow_loot::{
    GeneratedLootItem, LootConditionId, LootConditionRowLikeCpp, LootFillError, LootFillOptions,
    LootItemRandomProperties, LootItemTemplateMetadata, LootStoreItem, LootStoreItemContext,
    LootStoreKind, LootTemplate, condition_compare_values_like_cpp,
    generate_money_loot_with_rate_like_cpp, loot_condition_reference_ids_like_cpp,
    loot_condition_reference_self_references_like_cpp,
    loot_condition_row_normalize_without_external_stores_like_cpp,
    loot_conditions_allow_player_with_references_like_cpp_representable,
    loot_item_ui_type_for_player_like_cpp,
};
use wow_network::player_registry::{
    ApplyCreatureMeleeDamageLikeCppCommand, CreatureAttackStartLikeCppCommand,
    RefreshVisibleWorldCreaturesLikeCppCommand, SendIfVisibleLikeCppCommand,
    SendRepeatableTurnInRequestItemsLikeCppCommand, SetQuestSharingInfoAndSendDetailsCommand,
    SyncChestGameobjectStateAndRefreshLikeCppCommand,
    SyncGatheringNodeGameobjectStateAndRefreshLikeCppCommand,
    SyncGooberGameobjectStateAndRefreshLikeCppCommand,
};
use wow_network::{
    LootRollStoreWinnerCommand, LootRollVoteCommand, MasterLootGiveCommand, MasterLootGiveResult,
    PlayerRegistry, SessionCommand,
};
use wow_packet::packets::item::{
    ItemExpirePurchaseRefund, ItemInstance, ItemModList, ItemPushResult, ItemPushResultDisplayType,
};
use wow_packet::packets::loot::{
    AELootTargets, AELootTargetsAck, CoinRemoved, CreatureLoot, LOOT_ERROR_DIDNT_KILL_LIKE_CPP,
    LOOT_ERROR_MASTER_INV_FULL_LIKE_CPP, LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
    LOOT_ERROR_MASTER_UNIQUE_ITEM_LIKE_CPP, LOOT_ERROR_NO_LOOT_LIKE_CPP,
    LOOT_ERROR_PLAYER_NOT_FOUND_LIKE_CPP, LOOT_ERROR_TOO_FAR_LIKE_CPP, LOOT_TYPE_CHEST_LIKE_CPP,
    LOOT_TYPE_CORPSE_LIKE_CPP, LOOT_TYPE_DISENCHANTING_LIKE_CPP, LOOT_TYPE_FISHING_JUNK_LIKE_CPP,
    LOOT_TYPE_FISHING_LIKE_CPP, LOOT_TYPE_FISHINGHOLE_LIKE_CPP, LOOT_TYPE_INSIGNIA_LIKE_CPP,
    LOOT_TYPE_MILLING_LIKE_CPP, LOOT_TYPE_PROSPECTING_LIKE_CPP, LOOT_TYPE_SKINNING_LIKE_CPP,
    LootAllPassed, LootEntry, LootEntryFlags, LootItemData, LootItemPkt, LootList, LootMoney,
    LootMoneyNotify, LootRelease, LootReleaseAll, LootRemoved, LootResponse, LootRoll,
    LootRollBroadcast, LootRollWon, LootUnit, MasterLootCandidateList, MasterLootItem,
    NotNormalLootItem, SLootRelease, SetLootSpecialization, StartLootRoll,
};
use wow_packet::packets::update::{ItemCreateData, UpdateObject};
use wow_packet::{ClientPacket, ServerPacket};

use crate::session::{
    InventoryItem, RepresentedGameObjectSpellCaster, RepresentedGameObjectUseEffect,
    RepresentedLootRollState, RepresentedLootRollVote,
    RepresentedQuestObjectiveProgressEventLikeCpp, SessionState, WorldSession,
};

const LOOT_METHOD_MASTER_LIKE_CPP: u8 = 2;
const LOOT_METHOD_GROUP_LIKE_CPP: u8 = 3;
const LOOT_METHOD_NEED_BEFORE_GREED_LIKE_CPP: u8 = 4;
const MAX_NR_LOOT_ITEMS_LIKE_CPP: usize = 18;
const LOOT_ROLL_TIMEOUT_MS_LIKE_CPP: u32 = 60_000;
#[cfg(test)]
const ROLL_ALL_TYPE_NO_DISENCHANT_LIKE_CPP: u8 = 0x07;
const ROLL_ALL_TYPE_MASK_LIKE_CPP: u8 = 0x0F;
const ROLL_FLAG_TYPE_NEED_LIKE_CPP: u8 = 0x02;
const ROLL_FLAG_TYPE_DISENCHANT_LIKE_CPP: u8 = 0x08;
const LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP: u8 = 0;
const LOOT_SLOT_TYPE_ROLL_ONGOING_LIKE_CPP: u8 = 1;
const LOOT_SLOT_TYPE_LOCKED_LIKE_CPP: u8 = 2;
const DISENCHANT_LOOT_ROLL_CRITERIA_SPELL_LIKE_CPP: u32 = 13_262;
const LOOT_MODE_DEFAULT_LIKE_CPP: u16 = 0x01;
const LOOT_MODE_JUNK_FISH_LIKE_CPP: u16 = 0x8000;
const ITEM_FLAGS_CU_FOLLOW_LOOT_RULES_LIKE_CPP: u32 = 0x0004;
const ITEM_FLAGS_CU_IGNORE_QUEST_STATUS_LIKE_CPP: u32 = 0x0002;
const MAX_LOOT_REFERENCE_FRAMES_LIKE_CPP: u32 = 64;
const ROLL_VOTE_PASS_LIKE_CPP: u8 = 0;
const ROLL_VOTE_NEED_LIKE_CPP: u8 = 1;
const ROLL_VOTE_GREED_LIKE_CPP: u8 = 2;
const ROLL_VOTE_DISENCHANT_LIKE_CPP: u8 = 3;
const ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP: u8 = 4;
const ROLL_VOTE_NOT_VALID_LIKE_CPP: u8 = 5;
const CONDITION_OBJECT_ENTRY_GUID_LIKE_CPP: i32 = 51;
const CONDITION_TYPE_MASK_LIKE_CPP: i32 = 52;
const TYPEID_PLAYER_LIKE_CPP: u32 = 6;
const PLAYER_TYPE_MASK_LIKE_CPP: u32 = 0x0001 | 0x0020 | 0x0040;
const LOCK_KEY_SKILL_LIKE_CPP: u8 = 2;
const LOCK_KEY_SPELL_LIKE_CPP: u8 = 3;
const SPELL_EFFECT_OPEN_LOCK_LIKE_CPP: u32 = 33;
const REMOTE_MASTER_LOOT_COMMAND_TIMEOUT: Duration = Duration::from_millis(250);

// ── Handler registrations ─────────────────────────────────────────

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LootUnit,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_loot_unit",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LootItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_loot_item",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LootMoney,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_loot_money",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LootRelease,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_loot_release",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LootRoll,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_loot_roll",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::MasterLootItem,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_master_loot_item",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::SetLootSpecialization,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_set_loot_specialization",
    }
}

// ── Handler implementations ───────────────────────────────────────

impl WorldSession {
    /// CMSG_LOOT_UNIT — player right-clicks a dead creature to loot it.
    pub async fn handle_loot_unit(&mut self, mut pkt: wow_packet::WorldPacket) {
        let req = match LootUnit::read(&mut pkt) {
            Ok(r) => r,
            Err(e) => {
                warn!("Bad LootUnit: {e}");
                return;
            }
        };

        let player_guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };

        debug!(account = self.account_id, target = ?req.unit, "CMSG_LOOT_UNIT");

        if !self.player_is_alive_like_cpp() {
            return;
        }

        if !req.unit.is_creature_or_vehicle() {
            return;
        }

        // Check creature exists and is dead.
        let creature_state = match self.represented_creature_loot_state_like_cpp(req.unit) {
            Some(state) => state,
            None => {
                warn!("LootUnit: creature {:?} not found", req.unit);
                return;
            }
        };

        if creature_state.is_alive {
            return;
        }

        if self
            .player_position_like_cpp()
            .is_some_and(|player| !player.is_within_dist(&creature_state.position, 30.0))
        {
            return;
        }

        self.interrupt_non_melee_spell_cast_for_loot_like_cpp();
        self.remove_auras_with_looting_interrupt_flags_like_cpp();

        let ae_owner_guids = if self.enable_ae_loot_like_cpp() {
            self.represented_ae_loot_creature_targets_like_cpp(req.unit, player_guid)
                .await
        } else {
            Vec::new()
        };

        if !ae_owner_guids.is_empty() {
            self.send_packet(&AELootTargets {
                count: ae_owner_guids.len() as u32 + 1,
            });
        }

        let Some(response) = self
            .represented_loot_response_for_owner_like_cpp(req.unit, player_guid, false)
            .await
        else {
            return;
        };
        if !self.active_loot_guid.is_empty() && !self.active_loot_guid.is_item() {
            self.do_loot_release_all_like_cpp(player_guid).await;
        }
        self.set_active_loot_guid(req.unit);
        self.send_packet(&response);
        self.represented_on_loot_opened_like_cpp(req.unit, player_guid);

        if !ae_owner_guids.is_empty() {
            self.send_packet(&AELootTargetsAck);

            for owner_guid in ae_owner_guids {
                if let Some(response) = self
                    .represented_loot_response_for_owner_like_cpp(owner_guid, player_guid, true)
                    .await
                {
                    self.add_active_loot_view_owner_like_cpp(owner_guid);
                    self.send_packet(&response);
                    self.represented_on_loot_opened_like_cpp(owner_guid, player_guid);
                    self.send_packet(&AELootTargetsAck);
                }
            }
        }
    }

    pub(crate) async fn open_represented_gameobject_chest_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        source: GameObjectLootSource,
    ) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };
        if !self.player_is_alive_like_cpp() {
            return;
        }
        if !self.represented_gameobject_exists_for_loot_like_cpp(gameobject_guid) {
            return;
        }

        self.record_represented_gameobject_chest_release_metadata_like_cpp(gameobject_guid, source);

        let is_first_represented_unique_use = !self
            .represented_unique_gameobject_uses
            .contains(&gameobject_guid);
        if source.loot_id == 0 && is_first_represented_unique_use {
            self.represented_unique_gameobject_uses
                .insert(gameobject_guid);
            self.mutate_canonical_gameobject_by_guid_like_cpp(gameobject_guid, |gameobject| {
                gameobject.add_unique_use_like_cpp(player_guid);
            });
            if source.should_autostore_push_loot_like_cpp() {
                self.autostore_represented_gameobject_chest_push_loot_like_cpp(
                    gameobject_guid,
                    source,
                )
                .await;
            }
            self.record_represented_gameobject_use_effects_like_cpp(
                gameobject_guid,
                player_guid,
                source.triggered_event_id,
                source.linked_trap_entry,
            );
        }
        let activated_now = self
            .set_represented_gameobject_loot_state_activated_like_cpp(gameobject_guid, player_guid);
        if activated_now {
            let _ =
                self.queue_chest_gameobject_state_refresh_for_same_map_like_cpp(gameobject_guid);
        }
        if !source.has_open_loot_like_cpp() {
            return;
        }

        let should_record_generation_effects =
            source.loot_id != 0 && !self.loot_table.contains_key(&gameobject_guid);
        self.ensure_represented_gameobject_chest_loot_like_cpp(
            gameobject_guid,
            player_guid,
            source,
        )
        .await;
        if should_record_generation_effects && self.loot_table.contains_key(&gameobject_guid) {
            self.record_represented_gameobject_use_effects_like_cpp(
                gameobject_guid,
                player_guid,
                source.triggered_event_id,
                source.linked_trap_entry,
            );
        }

        if !source.is_personal_encounter_loot_like_cpp()
            && let Some(loot) = self.loot_table.get_mut(&gameobject_guid)
        {
            mark_loot_allowed_for_player_like_cpp(loot, player_guid);
        }
        self.sync_represented_gameobject_loot_to_canonical_like_cpp(gameobject_guid, player_guid);

        let Some(loot) = self.loot_table.get(&gameobject_guid) else {
            return;
        };
        if !self.represented_loot_can_be_opened_by_player_like_cpp(
            gameobject_guid,
            loot,
            player_guid,
        ) {
            return;
        }

        let response = LootResponse {
            owner: gameobject_guid,
            loot_obj: loot.loot_guid,
            failure_reason: 0,
            acquire_reason: loot_type_for_client_like_cpp(loot.loot_type),
            loot_method: loot.loot_method,
            threshold: 2,
            coins: self.represented_loot_money_for_player_like_cpp(
                gameobject_guid,
                loot,
                player_guid,
            ),
            items: represented_loot_response_items_like_cpp(loot, player_guid),
            currencies: vec![],
            acquired: true,
            ae_looting: false,
        };

        if !self.active_loot_guid.is_empty() && !self.active_loot_guid.is_item() {
            self.do_loot_release_all_like_cpp(player_guid).await;
        }
        self.set_active_loot_guid(gameobject_guid);
        self.send_packet(&response);
        self.represented_on_loot_opened_like_cpp(gameobject_guid, player_guid);
    }

    pub(crate) async fn open_represented_fishing_hole_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        gameobject_entry: u32,
        loot_id: u32,
    ) {
        let player_guid = self.player_guid();
        let should_update_criteria = player_guid.is_some()
            && loot_id != 0
            && self.player_is_alive_like_cpp()
            && self.represented_gameobject_exists_for_loot_like_cpp(gameobject_guid);
        self.open_represented_gameobject_personal_loot_like_cpp(
            gameobject_guid,
            loot_id,
            LOOT_TYPE_FISHINGHOLE_LIKE_CPP,
            true,
        )
        .await;
        if should_update_criteria {
            let player_guid = player_guid.expect("checked above");
            self.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::FishingHoleCatchCriteriaUpdated {
                    gameobject_guid,
                    player_guid,
                    gameobject_entry,
                },
            );
        }
    }

    pub(crate) async fn open_represented_fishing_node_loot_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        area_id: u32,
        junk: bool,
    ) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };
        if !self.player_is_alive_like_cpp() {
            return;
        }
        if !self.represented_gameobject_exists_for_loot_like_cpp(gameobject_guid) {
            return;
        }

        let loot_type = if junk {
            LOOT_TYPE_FISHING_JUNK_LIKE_CPP
        } else {
            LOOT_TYPE_FISHING_LIKE_CPP
        };
        let loot_mode = if junk {
            LOOT_MODE_JUNK_FISH_LIKE_CPP
        } else {
            LOOT_MODE_DEFAULT_LIKE_CPP
        };
        let items = self
            .generate_represented_fishing_loot_items_like_cpp(area_id, loot_mode)
            .await
            .unwrap_or_else(|| {
                debug!(
                    area_id,
                    gameobject = ?gameobject_guid,
                    junk,
                    "fishing loot template unavailable"
                );
                Vec::new()
            });

        self.loot_table.insert(
            gameobject_guid,
            CreatureLoot {
                loot_guid: represented_loot_object_guid_like_cpp(gameobject_guid),
                coins: 0,
                unlooted_count: 0,
                loot_type,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: vec![player_guid],
                items,
                looted_by_player: false,
            },
        );

        if let Some(loot) = self.loot_table.get_mut(&gameobject_guid) {
            mark_loot_allowed_for_player_like_cpp(loot, player_guid);
        }
        self.sync_represented_gameobject_loot_to_canonical_like_cpp(gameobject_guid, player_guid);

        let Some(loot) = self.loot_table.get(&gameobject_guid) else {
            return;
        };
        if !self.represented_loot_can_be_opened_by_player_like_cpp(
            gameobject_guid,
            loot,
            player_guid,
        ) {
            return;
        }

        let response = LootResponse {
            owner: gameobject_guid,
            loot_obj: loot.loot_guid,
            failure_reason: 0,
            acquire_reason: loot_type_for_client_like_cpp(loot.loot_type),
            loot_method: loot.loot_method,
            threshold: 2,
            coins: loot.coins,
            items: represented_loot_response_items_like_cpp(loot, player_guid),
            currencies: vec![],
            acquired: true,
            ae_looting: false,
        };

        if !self.active_loot_guid.is_empty() && !self.active_loot_guid.is_item() {
            self.do_loot_release_all_like_cpp(player_guid).await;
        }
        self.set_active_loot_guid(gameobject_guid);
        self.send_packet(&response);
        self.represented_on_loot_opened_like_cpp(gameobject_guid, player_guid);
    }

    pub(crate) async fn open_represented_gathering_node_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        gameobject_entry: u32,
        source: GatheringNodeUseSource,
    ) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };
        if !self.player_is_alive_like_cpp() {
            return;
        }
        if !self.represented_gameobject_exists_for_loot_like_cpp(gameobject_guid) {
            return;
        }

        let is_first_represented_use = !self
            .represented_unique_gameobject_uses
            .contains(&gameobject_guid);
        if is_first_represented_use {
            self.represented_unique_gameobject_uses
                .insert(gameobject_guid);
            self.mutate_canonical_gameobject_by_guid_like_cpp(gameobject_guid, |gameobject| {
                gameobject.add_unique_use_like_cpp(player_guid);
            });
        }

        self.open_represented_gameobject_personal_loot_like_cpp(
            gameobject_guid,
            source.loot_id,
            LOOT_TYPE_CHEST_LIKE_CPP,
            false,
        )
        .await;

        if is_first_represented_use {
            let xp = self.represented_gathering_node_xp_like_cpp(source.xp_difficulty);
            if xp != 0 {
                self.give_xp(xp, ObjectGuid::EMPTY, false).await;
            }
            self.record_represented_gameobject_use_effects_like_cpp(
                gameobject_guid,
                player_guid,
                source.triggered_event_id,
                source.linked_trap_entry,
            );
        }
        self.record_represented_gathering_node_runtime_state_like_cpp(
            gameobject_guid,
            gameobject_entry,
            player_guid,
            source,
            is_first_represented_use,
        );
        let _ = self
            .queue_gathering_node_gameobject_state_refresh_for_same_map_like_cpp(gameobject_guid);
    }

    fn gathering_node_gameobject_state_refresh_command_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
    ) -> Option<SyncGatheringNodeGameobjectStateAndRefreshLikeCppCommand> {
        let state = self
            .represented_gameobject_use_states
            .get(&gameobject_guid)?;
        Some(SyncGatheringNodeGameobjectStateAndRefreshLikeCppCommand {
            gameobject_guid,
            map_id: self.player_map_id_like_cpp(),
            instance_id: self
                .current_canonical_player_map_key_like_cpp()
                .map(|key| key.instance_id)
                .unwrap_or(0),
            go_type: state.go_type?,
            loot_state: state.loot_state.map(|loot_state| loot_state as u8),
            loot_state_unit_guid: state.loot_state_unit_guid,
            go_state: state.go_state.map(|go_state| go_state as i8),
            dynamic_flags: state.dynamic_flags,
            gathering_node_loot_id: state.gathering_node_loot_id,
            personal_loot_uses: state.personal_loot_uses,
            linked_trap_entry: state.linked_trap_entry,
            linked_trap_guid: state.linked_trap_guid,
        })
    }

    fn chest_gameobject_state_refresh_command_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
    ) -> Option<SyncChestGameobjectStateAndRefreshLikeCppCommand> {
        let state = self
            .represented_gameobject_use_states
            .get(&gameobject_guid)?;
        let source = state.chest_loot_source?;
        Some(SyncChestGameobjectStateAndRefreshLikeCppCommand {
            gameobject_guid,
            map_id: self.player_map_id_like_cpp(),
            instance_id: self
                .current_canonical_player_map_key_like_cpp()
                .map(|key| key.instance_id)
                .unwrap_or(0),
            go_type: state.go_type.unwrap_or(GAMEOBJECT_TYPE_CHEST as u8),
            loot_state: state.loot_state.map(|loot_state| loot_state as u8),
            loot_state_unit_guid: state.loot_state_unit_guid,
            chest_loot_id: source.loot_id,
            chest_personal_loot_id: source.personal_loot_id,
            chest_push_loot_id: source.push_loot_id,
            chest_quest_id: source.chest_quest_id,
            chest_restock_time_secs: source.chest_restock_time_secs,
            chest_consumable: source.chest_consumable,
            linked_trap_entry: state.linked_trap_entry,
            linked_trap_guid: state.linked_trap_guid,
        })
    }

    fn goober_gameobject_state_refresh_command_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
    ) -> Option<SyncGooberGameobjectStateAndRefreshLikeCppCommand> {
        let state = self
            .represented_gameobject_use_states
            .get(&gameobject_guid)?;
        Some(SyncGooberGameobjectStateAndRefreshLikeCppCommand {
            gameobject_guid,
            map_id: self.player_map_id_like_cpp(),
            instance_id: self
                .current_canonical_player_map_key_like_cpp()
                .map(|key| key.instance_id)
                .unwrap_or(0),
            go_type: state.go_type.unwrap_or(GAMEOBJECT_TYPE_GOOBER as u8),
            gameobject_flags: state.gameobject_flags,
            loot_state: state.loot_state.map(|loot_state| loot_state as u8),
            loot_state_unit_guid: state.loot_state_unit_guid,
            go_state: state.go_state.map(|go_state| go_state as i8),
            dynamic_flags: state.dynamic_flags,
            linked_trap_entry: state.linked_trap_entry,
            linked_trap_guid: state.linked_trap_guid,
        })
    }

    pub(crate) fn queue_chest_gameobject_state_refresh_for_same_map_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
    ) -> usize {
        let Some(player_guid) = self.player_guid() else {
            return 0;
        };
        let Some(registry) = self.player_registry() else {
            return 0;
        };
        let Some(command) = self.chest_gameobject_state_refresh_command_like_cpp(gameobject_guid)
        else {
            return 0;
        };
        let current_map_id = self.player_map_id_like_cpp();
        let current_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let mut queued = 0;

        for entry in registry.iter() {
            let (candidate_guid, candidate) = entry.pair();
            if *candidate_guid == player_guid {
                continue;
            }
            if !candidate.is_in_world
                || candidate.map_id != current_map_id
                || candidate.instance_id != current_instance_id
            {
                continue;
            }
            if candidate
                .command_tx
                .try_send(SessionCommand::SyncChestGameobjectStateAndRefreshLikeCpp(
                    command.clone(),
                ))
                .is_ok()
            {
                queued += 1;
            }
        }

        queued
    }

    pub(crate) fn queue_goober_gameobject_state_refresh_for_same_map_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
    ) -> usize {
        let Some(player_guid) = self.player_guid() else {
            return 0;
        };
        let Some(registry) = self.player_registry() else {
            return 0;
        };
        let Some(command) = self.goober_gameobject_state_refresh_command_like_cpp(gameobject_guid)
        else {
            return 0;
        };
        let current_map_id = self.player_map_id_like_cpp();
        let current_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let mut queued = 0;

        for entry in registry.iter() {
            let (candidate_guid, candidate) = entry.pair();
            if *candidate_guid == player_guid {
                continue;
            }
            if !candidate.is_in_world
                || candidate.map_id != current_map_id
                || candidate.instance_id != current_instance_id
            {
                continue;
            }
            if candidate
                .command_tx
                .try_send(SessionCommand::SyncGooberGameobjectStateAndRefreshLikeCpp(
                    command.clone(),
                ))
                .is_ok()
            {
                queued += 1;
            }
        }

        queued
    }

    pub(crate) fn queue_visible_gameobject_packet_for_same_map_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
        packet_bytes: Vec<u8>,
    ) -> usize {
        let Some(player_guid) = self.player_guid() else {
            return 0;
        };
        let Some(registry) = self.player_registry() else {
            return 0;
        };
        let current_map_id = self.player_map_id_like_cpp();
        let current_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let mut queued = 0;

        for entry in registry.iter() {
            let (candidate_guid, candidate) = entry.pair();
            if *candidate_guid == player_guid {
                continue;
            }
            if !candidate.is_in_world
                || candidate.map_id != current_map_id
                || candidate.instance_id != current_instance_id
            {
                continue;
            }
            if candidate
                .command_tx
                .try_send(SessionCommand::SendIfVisibleLikeCpp(
                    SendIfVisibleLikeCppCommand {
                        source_guid: gameobject_guid,
                        map_id: current_map_id,
                        instance_id: current_instance_id,
                        packet_bytes: packet_bytes.clone(),
                    },
                ))
                .is_ok()
            {
                queued += 1;
            }
        }

        queued
    }

    fn queue_gathering_node_gameobject_state_refresh_for_same_map_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
    ) -> usize {
        let Some(player_guid) = self.player_guid() else {
            return 0;
        };
        let Some(registry) = self.player_registry() else {
            return 0;
        };
        let Some(command) =
            self.gathering_node_gameobject_state_refresh_command_like_cpp(gameobject_guid)
        else {
            return 0;
        };
        let current_map_id = self.player_map_id_like_cpp();
        let current_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let mut queued = 0;

        for entry in registry.iter() {
            let (candidate_guid, candidate) = entry.pair();
            if *candidate_guid == player_guid {
                continue;
            }
            if !candidate.is_in_world
                || candidate.map_id != current_map_id
                || candidate.instance_id != current_instance_id
            {
                continue;
            }
            if candidate
                .command_tx
                .try_send(
                    SessionCommand::SyncGatheringNodeGameobjectStateAndRefreshLikeCpp(
                        command.clone(),
                    ),
                )
                .is_ok()
            {
                queued += 1;
            }
        }

        queued
    }

    fn set_represented_gameobject_loot_state_activated_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> bool {
        let state = self
            .represented_gameobject_use_states
            .entry(gameobject_guid)
            .or_default();
        if state.loot_state == Some(LootState::Activated) {
            return false;
        }

        state.loot_state = Some(LootState::Activated);
        state.loot_state_unit_guid = player_guid;
        true
    }

    fn record_represented_gameobject_chest_release_metadata_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        source: GameObjectLootSource,
    ) {
        let state = self
            .represented_gameobject_use_states
            .entry(gameobject_guid)
            .or_default();
        state.go_type = Some(GAMEOBJECT_TYPE_CHEST as u8);
        state.chest_restock_time_secs = Some(source.chest_restock_time_secs);
        state.chest_consumable = Some(source.chest_consumable);
        state.despawn_at_action = source.chest_consumable;
        state.chest_loot_source = Some(source);
        state.chest_personal_loot_id = Some(source.personal_loot_id);
        state.linked_trap_entry =
            (source.linked_trap_entry != 0).then_some(source.linked_trap_entry);
    }

    fn record_represented_gathering_node_runtime_state_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        gameobject_entry: u32,
        player_guid: ObjectGuid,
        source: GatheringNodeUseSource,
        is_first_represented_use: bool,
    ) {
        {
            let state = self
                .represented_gameobject_use_states
                .entry(gameobject_guid)
                .or_default();
            if is_first_represented_use {
                state.personal_loot_uses = state.personal_loot_uses.saturating_add(1);
            }
            state.go_type = Some(GAMEOBJECT_TYPE_GATHERING_NODE as u8);
            state.gathering_node_loot_id = Some(source.loot_id);
            if state.personal_loot_uses >= source.max_loots {
                state.go_state = Some(GoState::Active);
                state.dynamic_flags |= GO_DYNFLAG_LO_NO_INTERACT;
            }
            state.linked_trap_entry =
                (source.linked_trap_entry != 0).then_some(source.linked_trap_entry);
        }

        let activated_now = self
            .set_represented_gameobject_loot_state_activated_like_cpp(gameobject_guid, player_guid);
        if activated_now && source.despawn_delay_secs != 0 {
            if let Some(state) = self
                .represented_gameobject_use_states
                .get_mut(&gameobject_guid)
            {
                state.despawn_delay_secs = Some(source.despawn_delay_secs);
                state.despawn_delay_until = Some(
                    Instant::now() + Duration::from_secs(u64::from(source.despawn_delay_secs)),
                );
            }
        }

        if is_first_represented_use && source.spell_id != 0 {
            self.apply_represented_gameobject_post_use_spell_like_cpp(
                gameobject_guid,
                player_guid,
                gameobject_entry,
                GAMEOBJECT_TYPE_GATHERING_NODE,
                source.spell_id,
                false,
                RepresentedGameObjectSpellCaster::User,
                player_guid,
            );
        }
    }

    fn record_represented_gameobject_use_effects_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        triggered_event_id: u32,
        linked_trap_entry: u32,
    ) {
        if triggered_event_id != 0 {
            self.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::TriggerGameEvent {
                    gameobject_guid,
                    player_guid,
                    event_id: triggered_event_id,
                },
            );
        }
        if linked_trap_entry != 0 {
            self.represented_gameobject_use_effects.push(
                RepresentedGameObjectUseEffect::TriggerLinkedTrap {
                    gameobject_guid,
                    player_guid,
                    trap_entry: linked_trap_entry,
                },
            );
        }
    }

    fn represented_gathering_node_xp_like_cpp(&self, xp_difficulty: u32) -> u32 {
        if xp_difficulty == 0 || xp_difficulty >= 10 {
            return 0;
        }

        self.quest_xp_store
            .as_ref()
            .map(|store| {
                store.player_level_difficulty_xp_like_cpp(
                    self.player_level_like_cpp(),
                    xp_difficulty,
                )
            })
            .unwrap_or(0)
    }

    async fn open_represented_gameobject_personal_loot_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        loot_id: u32,
        loot_type: u8,
        replace_existing: bool,
    ) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };
        if loot_id == 0 || !self.player_is_alive_like_cpp() {
            return;
        }
        if !self.represented_gameobject_exists_for_loot_like_cpp(gameobject_guid) {
            return;
        }

        if replace_existing || !self.loot_table.contains_key(&gameobject_guid) {
            let items = self
                .generate_represented_gameobject_loot_items_for_store_like_cpp(
                    loot_id,
                    LootStoreKind::Gameobject,
                    LOOT_MODE_DEFAULT_LIKE_CPP,
                )
                .await
                .unwrap_or_else(|| {
                    debug!(
                        loot_id,
                        gameobject = ?gameobject_guid,
                        "gameobject personal loot template unavailable"
                    );
                    Vec::new()
                });
            self.loot_table.insert(
                gameobject_guid,
                CreatureLoot {
                    loot_guid: represented_loot_object_guid_like_cpp(gameobject_guid),
                    coins: 0,
                    unlooted_count: 0,
                    loot_type,
                    dungeon_encounter_id: 0,
                    loot_method: 0,
                    loot_master: ObjectGuid::EMPTY,
                    round_robin_player: ObjectGuid::EMPTY,
                    player_ffa_items: Vec::new(),
                    players_looting: Vec::new(),
                    allowed_looters: Vec::new(),
                    items,
                    looted_by_player: false,
                },
            );
        }

        if let Some(loot) = self.loot_table.get_mut(&gameobject_guid) {
            mark_loot_allowed_for_player_like_cpp(loot, player_guid);
        }
        self.sync_represented_gameobject_loot_to_canonical_like_cpp(gameobject_guid, player_guid);

        let Some(loot) = self.loot_table.get(&gameobject_guid) else {
            return;
        };
        if !self.represented_loot_can_be_opened_by_player_like_cpp(
            gameobject_guid,
            loot,
            player_guid,
        ) {
            return;
        }

        let response = LootResponse {
            owner: gameobject_guid,
            loot_obj: loot.loot_guid,
            failure_reason: 0,
            acquire_reason: loot_type_for_client_like_cpp(loot.loot_type),
            loot_method: loot.loot_method,
            threshold: 2,
            coins: loot.coins,
            items: represented_loot_response_items_like_cpp(loot, player_guid),
            currencies: vec![],
            acquired: true,
            ae_looting: false,
        };

        if !self.active_loot_guid.is_empty() && !self.active_loot_guid.is_item() {
            self.do_loot_release_all_like_cpp(player_guid).await;
        }
        self.set_active_loot_guid(gameobject_guid);
        self.send_packet(&response);
        self.represented_on_loot_opened_like_cpp(gameobject_guid, player_guid);
    }

    /// CMSG_LOOT_ITEM — player clicks to take a specific item from the loot.
    pub async fn handle_loot_item(&mut self, mut pkt: wow_packet::WorldPacket) {
        let req = match LootItemPkt::read(&mut pkt) {
            Ok(r) => r,
            Err(e) => {
                warn!("Bad LootItem: {e}");
                return;
            }
        };

        let player_guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };

        let mut taken_items: Vec<(ObjectGuid, ObjectGuid, u8, u32, u32, bool)> = Vec::new();
        let mut item_release: Vec<ObjectGuid> = Vec::new();

        for loot_req in &req.requests {
            let Some(owner_guid) = self.active_loot_owner_for_loot_object_like_cpp(loot_req.object)
            else {
                self.send_packet(&SLootRelease {
                    loot_obj: ObjectGuid::EMPTY,
                    owner: player_guid,
                });
                continue;
            };

            self.ensure_represented_player_looting_like_cpp(owner_guid, player_guid);

            if owner_guid.is_game_object()
                && !self.represented_gameobject_can_autostore_loot_item_like_cpp(
                    owner_guid,
                    player_guid,
                )
            {
                self.send_packet(&SLootRelease {
                    loot_obj: owner_guid,
                    owner: player_guid,
                });
                continue;
            }

            if owner_guid.is_creature_or_vehicle() {
                let Some(creature_position) =
                    self.represented_creature_position_for_loot_like_cpp(owner_guid)
                else {
                    self.send_loot_error_like_cpp(
                        loot_req.object,
                        owner_guid,
                        LOOT_ERROR_NO_LOOT_LIKE_CPP,
                    );
                    continue;
                };

                if self
                    .player_position_like_cpp()
                    .is_some_and(|player| !player.is_within_dist(&creature_position, 30.0))
                {
                    self.send_loot_error_like_cpp(
                        loot_req.object,
                        owner_guid,
                        LOOT_ERROR_TOO_FAR_LIKE_CPP,
                    );
                    continue;
                }
            }

            let Some((entry, dungeon_encounter_id)) =
                self.loot_table.get(&owner_guid).and_then(|loot| {
                    loot.items
                        .iter()
                        .find(|entry| {
                            entry.loot_list_id == loot_req.loot_list_id
                                && !loot_item_is_looted_for_player_like_cpp(
                                    loot,
                                    entry,
                                    player_guid,
                                )
                        })
                        .cloned()
                        .map(|entry| (entry, loot.dungeon_encounter_id))
                })
            else {
                self.send_equip_error(InventoryResult::LootGone, None, None, 0, 0);
                continue;
            };

            if !entry.has_allowed_looter_like_cpp(player_guid) {
                self.send_packet(&LootReleaseAll);
                continue;
            }

            if entry.flags.blocked {
                self.send_packet(&LootReleaseAll);
                continue;
            }

            if !entry.roll_winner_allows_like_cpp(player_guid) {
                self.send_packet(&LootReleaseAll);
                continue;
            }

            if !self
                .store_direct_loot_item_like_cpp(&entry, dungeon_encounter_id)
                .await
            {
                continue;
            }

            if owner_guid.is_item() {
                self.delete_stored_item_loot_item_like_cpp(
                    owner_guid,
                    entry.item_id,
                    entry.quantity,
                    entry.loot_list_id,
                )
                .await;
            }

            if let Some(loot) = self.loot_table.get_mut(&owner_guid) {
                if let Some(entry) = loot
                    .items
                    .iter()
                    .find(|entry| entry.loot_list_id == loot_req.loot_list_id)
                    .cloned()
                {
                    mark_loot_item_looted_for_player_like_cpp(
                        loot,
                        loot_req.loot_list_id,
                        player_guid,
                    );
                    taken_items.push((
                        owner_guid,
                        loot_req.object,
                        entry.loot_list_id,
                        entry.item_id,
                        entry.quantity,
                        entry.flags.freeforall,
                    ));
                }

                if owner_guid.is_item() && loot_is_looted_like_cpp(loot) {
                    item_release.push(owner_guid);
                }
            }
        }

        for (owner_guid, loot_obj, list_id, item_id, quantity, freeforall) in taken_items {
            if freeforall {
                let removed = LootRemoved {
                    owner: owner_guid,
                    loot_obj,
                    loot_list_id: list_id,
                };
                self.send_packet(&removed);
            } else {
                self.represented_notify_loot_item_removed_like_cpp(owner_guid, list_id);
            }
            debug!(
                account = self.account_id,
                item = item_id,
                quantity,
                "Looted item"
            );
        }

        item_release.sort_by_key(|guid| guid.counter());
        item_release.dedup();
        for loot_guid in item_release {
            self.loot_table.remove(&loot_guid);
            self.clear_active_loot_guid_if(loot_guid);
            self.send_packet(&SLootRelease {
                loot_obj: loot_guid,
                owner: player_guid,
            });
            self.destroy_fully_looted_direct_item(loot_guid).await;
        }
    }

    /// CMSG_LOOT_MONEY — player takes money from the current loot view.
    pub async fn handle_loot_money(&mut self, mut pkt: wow_packet::WorldPacket) {
        let req = match LootMoney::read(&mut pkt) {
            Ok(r) => r,
            Err(e) => {
                warn!("Bad LootMoney: {e}");
                return;
            }
        };

        let player_guid = match self.player_guid() {
            Some(guid) => guid,
            None => return,
        };

        debug!(
            account = self.account_id,
            is_soft_interact = req.is_soft_interact,
            "CMSG_LOOT_MONEY"
        );

        let mut active_owners: Vec<ObjectGuid> =
            self.active_loot_view_owners.iter().copied().collect();
        if active_owners.is_empty() && !self.active_loot_guid.is_empty() {
            active_owners.push(self.active_loot_guid);
        }
        active_owners.sort_by_key(|guid| (guid.high_value(), guid.low_value()));

        if active_owners.is_empty() {
            return;
        }

        let money_by_loot: Vec<(ObjectGuid, ObjectGuid, u32)> = active_owners
            .into_iter()
            .filter_map(|loot_guid| {
                let loot = self.loot_table.get(&loot_guid)?;
                if loot_guid.is_creature_or_vehicle()
                    && !loot.allowed_looters.contains(&player_guid)
                {
                    return None;
                }
                Some((
                    loot_guid,
                    loot.loot_guid,
                    self.represented_loot_money_for_player_like_cpp(loot_guid, loot, player_guid),
                ))
            })
            .collect();

        if money_by_loot.is_empty() {
            return;
        }

        let mut item_release: Vec<ObjectGuid> = Vec::new();
        let mut player_money_delta = 0u64;

        for (loot_guid, _loot_obj, money) in &money_by_loot {
            self.ensure_represented_player_looting_like_cpp(*loot_guid, player_guid);
            self.represented_notify_money_removed_like_cpp(*loot_guid);

            let recipients = self.represented_loot_money_recipients_like_cpp(*loot_guid);
            let money = u64::from(*money);
            let money_per_player = money / recipients.len() as u64;
            let sole_looter = recipients.len() <= 1;

            let notify = LootMoneyNotify {
                money: money_per_player,
                money_mod: 0,
                sole_looter,
            };

            for recipient in recipients {
                if recipient == player_guid {
                    self.send_packet(&notify);
                    player_money_delta = player_money_delta.saturating_add(money_per_player);
                } else if let Some(registry) = self.player_registry() {
                    if let Some(member) = registry.get(&recipient) {
                        let _ = member.send_tx.send(notify.to_bytes());
                    }
                }
            }

            let personal_money_owner = self.represented_personal_loot_owners.contains(loot_guid);
            if let Some(loot) = self.loot_table.get_mut(loot_guid) {
                if personal_money_owner {
                    self.represented_personal_loot_money
                        .insert((*loot_guid, player_guid), 0);
                } else {
                    loot.coins = 0;
                }

                if loot_guid.is_item() && loot_is_looted_like_cpp(loot) {
                    item_release.push(*loot_guid);
                }
            }
        }

        let old_money = self.player_gold_like_cpp();
        let new_money = old_money.saturating_add(player_money_delta);
        if player_money_delta != 0 {
            self.enqueue_represented_quest_objective_progress_like_cpp(
                RepresentedQuestObjectiveProgressEventLikeCpp::MoneyChanged {
                    old_money,
                    new_money,
                },
            );
        }
        self.set_player_gold_like_cpp(new_money);
        self.save_player_gold().await;
        self.drain_represented_quest_objective_progress_like_cpp()
            .await;

        for (loot_guid, _, _) in &money_by_loot {
            if loot_guid.is_item() {
                self.delete_stored_item_money_like_cpp(*loot_guid).await;
            }
        }

        for loot_guid in item_release {
            self.loot_table.remove(&loot_guid);
            self.clear_active_loot_guid_if(loot_guid);
            self.send_packet(&SLootRelease {
                loot_obj: loot_guid,
                owner: player_guid,
            });
            self.destroy_fully_looted_direct_item(loot_guid).await;
        }

        let _ = player_guid;
    }

    fn represented_loot_money_recipients_like_cpp(&self, loot_guid: ObjectGuid) -> Vec<ObjectGuid> {
        let Some(player_guid) = self.player_guid() else {
            return Vec::new();
        };

        if !loot_guid.is_creature() {
            return vec![player_guid];
        }

        let (Some(group_guid), Some(group_registry), Some(player_registry)) = (
            self.group_guid,
            self.group_registry(),
            self.player_registry(),
        ) else {
            return vec![player_guid];
        };

        let Some(group) = group_registry.get(&group_guid) else {
            return vec![player_guid];
        };

        let source_position = self.player_position_like_cpp().unwrap_or_default();
        let mut recipients = Vec::new();

        for member_guid in &group.members {
            if !self.represented_loot_money_allowed_for_member_like_cpp(loot_guid, *member_guid) {
                continue;
            }

            if *member_guid == player_guid {
                recipients.push(*member_guid);
                continue;
            }

            let Some(member) = player_registry.get(member_guid) else {
                continue;
            };

            if member.map_id != self.player_map_id_like_cpp() {
                continue;
            }

            if source_position.is_within_dist(&member.position, 74.0) {
                recipients.push(*member_guid);
            }
        }

        if recipients.is_empty() {
            recipients.push(player_guid);
        }

        recipients
    }

    fn represented_loot_money_for_player_like_cpp(
        &self,
        loot_guid: ObjectGuid,
        loot: &CreatureLoot,
        player_guid: ObjectGuid,
    ) -> u32 {
        if self.represented_personal_loot_owners.contains(&loot_guid) {
            return self
                .represented_personal_loot_money
                .get(&(loot_guid, player_guid))
                .copied()
                .unwrap_or(0);
        }

        loot.coins
    }

    fn represented_loot_unlooted_count_for_player_like_cpp(
        loot: &CreatureLoot,
        player_guid: ObjectGuid,
    ) -> u32 {
        loot.items
            .iter()
            .filter(|entry| {
                entry.has_allowed_looter_like_cpp(player_guid)
                    && !loot_item_is_looted_for_player_like_cpp(loot, entry, player_guid)
            })
            .count()
            .try_into()
            .unwrap_or(u32::MAX)
    }

    fn sync_represented_gameobject_loot_to_canonical_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> Option<()> {
        let loot = self.loot_table.get(&gameobject_guid)?.clone();
        let is_personal = self
            .represented_personal_loot_owners
            .contains(&gameobject_guid);
        let personal_money = self.represented_personal_loot_money.clone();
        self.mutate_canonical_gameobject_by_guid_like_cpp(gameobject_guid, |gameobject| {
            if is_personal {
                gameobject.clear_shared_loot_like_cpp();
                let mut represented_looters = loot.allowed_looters.clone();
                if represented_looters.is_empty() && !player_guid.is_empty() {
                    represented_looters.push(player_guid);
                }
                represented_looters
                    .sort_unstable_by_key(|guid| (guid.high_value(), guid.low_value()));
                represented_looters.dedup();
                for looter_guid in represented_looters {
                    let gold = personal_money
                        .get(&(gameobject_guid, looter_guid))
                        .copied()
                        .unwrap_or(0);
                    let unlooted_count = Self::represented_loot_unlooted_count_for_player_like_cpp(
                        &loot,
                        looter_guid,
                    );
                    gameobject.set_personal_loot_like_cpp(
                        looter_guid,
                        GameObjectOwnedLoot::new(gold, unlooted_count),
                    );
                }
            } else {
                gameobject.set_shared_loot_like_cpp(GameObjectOwnedLoot::new(
                    loot.coins,
                    u32::from(loot.unlooted_count),
                ));
            }
        })
        .map(|_| ())
    }

    fn canonical_gameobject_fully_looted_after_represented_sync_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        fallback_fully_looted: bool,
    ) -> bool {
        if self
            .sync_represented_gameobject_loot_to_canonical_like_cpp(gameobject_guid, player_guid)
            .is_some()
        {
            return self
                .canonical_gameobject_is_fully_looted_like_cpp(gameobject_guid)
                .unwrap_or(fallback_fully_looted);
        }

        fallback_fully_looted
    }

    fn represented_loot_can_be_opened_by_player_like_cpp(
        &self,
        loot_guid: ObjectGuid,
        loot: &CreatureLoot,
        player_guid: ObjectGuid,
    ) -> bool {
        if !loot.allowed_looters.contains(&player_guid) {
            return false;
        }

        if self.represented_loot_money_for_player_like_cpp(loot_guid, loot, player_guid) > 0 {
            return true;
        }

        loot_can_be_opened_by_player_like_cpp(loot, player_guid)
    }

    fn represented_loot_money_allowed_for_member_like_cpp(
        &self,
        loot_guid: ObjectGuid,
        member_guid: ObjectGuid,
    ) -> bool {
        let Some(loot) = self.loot_table.get(&loot_guid) else {
            return false;
        };

        if loot
            .items
            .iter()
            .all(|item| item.allowed_looters.is_empty())
        {
            return true;
        }

        loot.items
            .iter()
            .any(|item| item.allowed_looters.contains(&member_guid))
    }

    /// CMSG_LOOT_RELEASE — player closes the loot window.
    ///
    /// C# ref: `LootHandler.DoLootRelease` (creature branch):
    ///   if loot.IsLooted() && creature.IsFullyLooted() → RemoveDynamicFlag(Lootable)
    ///   → creature.AllLootRemovedFromCorpse() → sets `m_corpseRemoveTime = now + decay`
    pub async fn handle_loot_release(&mut self, mut pkt: wow_packet::WorldPacket) {
        let req = match LootRelease::read(&mut pkt) {
            Ok(r) => r,
            Err(e) => {
                warn!("Bad LootRelease: {e}");
                return;
            }
        };

        debug!(account = self.account_id, unit = ?req.unit, "CMSG_LOOT_RELEASE");

        let player_guid = match self.player_guid() {
            Some(g) => g,
            None => return,
        };

        self.do_loot_release_owner_like_cpp(req.unit, player_guid)
            .await;
    }

    /// CMSG_LOOT_ROLL — vote on a pending group loot roll.
    ///
    /// C++ `HandleLootRoll` silently returns when `GetLootRoll` finds no
    /// canonical roll state. Rust does not yet port that state machine, so this
    /// represented handler preserves the current wire behavior without emitting
    /// synthetic errors.
    pub async fn handle_loot_roll(&mut self, roll: LootRoll) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };

        if self
            .represented_player_vote_on_loot_roll_like_cpp(&roll, player_guid)
            .await
        {
            return;
        }

        if self.route_represented_remote_loot_roll_vote_to_owner_like_cpp(&roll, player_guid) {
            return;
        }

        debug!(
            account = self.account_id,
            loot_obj = ?roll.loot_obj,
            loot_list_id = roll.loot_list_id,
            roll_type = roll.roll_type,
            "CMSG_LOOT_ROLL ignored: canonical LootRoll state is not ported yet"
        );
    }

    fn route_represented_remote_loot_roll_vote_to_owner_like_cpp(
        &self,
        roll: &LootRoll,
        player_guid: ObjectGuid,
    ) -> bool {
        let Some(registry) = self.player_registry() else {
            return false;
        };

        let roll_key = (roll.loot_obj, roll.loot_list_id);
        let mut command_tx = None;
        for owner in registry.iter() {
            if *owner.key() == player_guid {
                continue;
            }
            if owner.map_id != self.player_map_id_like_cpp() {
                continue;
            }
            if owner.active_loot_rolls.contains(&roll_key) {
                command_tx = Some(owner.command_tx.clone());
                break;
            }
        }

        let Some(command_tx) = command_tx else {
            return false;
        };

        command_tx
            .try_send(SessionCommand::LootRollVote(LootRollVoteCommand {
                voter_guid: player_guid,
                loot_obj: roll.loot_obj,
                loot_list_id: roll.loot_list_id,
                roll_type: roll.roll_type,
                pass_on_group_loot: self.pass_on_group_loot,
            }))
            .is_ok()
    }

    async fn represented_player_vote_on_loot_roll_like_cpp(
        &mut self,
        roll: &LootRoll,
        player_guid: ObjectGuid,
    ) -> bool {
        self.represented_player_vote_on_loot_roll_with_pass_state_like_cpp(
            roll,
            player_guid,
            self.pass_on_group_loot,
        )
        .await
    }

    async fn represented_player_vote_on_loot_roll_with_pass_state_like_cpp(
        &mut self,
        roll: &LootRoll,
        player_guid: ObjectGuid,
        pass_on_group_loot: bool,
    ) -> bool {
        if pass_on_group_loot {
            return false;
        }

        let Some(owner_guid) = self.active_loot_owner_for_loot_object_like_cpp(roll.loot_obj)
        else {
            return false;
        };

        let Some(loot) = self.loot_table.get(&owner_guid) else {
            return false;
        };
        if !matches!(
            loot.loot_method,
            LOOT_METHOD_GROUP_LIKE_CPP | LOOT_METHOD_NEED_BEFORE_GREED_LIKE_CPP
        ) {
            return false;
        }
        let loot_guid = loot.loot_guid;
        let dungeon_encounter_id = loot.dungeon_encounter_id as i32;

        let Some(entry) = loot.items.iter().find(|entry| {
            entry.loot_list_id == roll.loot_list_id
                && entry.flags.blocked
                && entry.has_allowed_looter_like_cpp(player_guid)
        }) else {
            return false;
        };
        let entry = entry.clone();

        let Some(state) = self
            .represented_loot_rolls
            .get_mut(&(loot_guid, roll.loot_list_id))
        else {
            return false;
        };
        let Some(voter) = state.voters.get_mut(&player_guid) else {
            return false;
        };

        let roll_number = match roll.roll_type {
            ROLL_VOTE_PASS_LIKE_CPP => -1,
            ROLL_VOTE_NEED_LIKE_CPP => 0,
            ROLL_VOTE_GREED_LIKE_CPP | ROLL_VOTE_DISENCHANT_LIKE_CPP => -1,
            _ => return false,
        };
        voter.vote = roll.roll_type;
        voter.roll_number = represented_roll_number_like_cpp();

        let packet = LootRollBroadcast {
            loot_obj: loot_guid,
            player: player_guid,
            roll: roll_number,
            roll_type: roll.roll_type,
            item: loot_roll_broadcast_item_like_cpp(&entry, LOOT_SLOT_TYPE_ROLL_ONGOING_LIKE_CPP),
            autopassed: false,
            off_spec: false,
            dungeon_encounter_id,
        };

        let finish = represented_loot_roll_finish_winner_like_cpp(state);
        let finished_state = finish.as_ref().map(|_| state.clone());
        self.update_represented_loot_roll_vote_criteria_like_cpp(player_guid, roll.roll_type);
        self.broadcast_represented_loot_roll_packet_like_cpp(&packet, &entry, None);
        if let Some(winner) = finish {
            self.finish_represented_loot_roll_like_cpp(
                loot_guid,
                roll.loot_list_id,
                &entry,
                winner,
                finished_state.as_ref(),
            )
            .await;
        }
        true
    }

    async fn finish_represented_loot_roll_like_cpp(
        &mut self,
        loot_obj: ObjectGuid,
        loot_list_id: u8,
        entry: &LootEntry,
        winner: Option<(ObjectGuid, RepresentedLootRollVote)>,
        finished_state: Option<&RepresentedLootRollState>,
    ) {
        let Some(owner_guid) = self.active_loot_owner_for_loot_object_like_cpp(loot_obj) else {
            return;
        };
        let dungeon_encounter_id = self
            .loot_table
            .get(&owner_guid)
            .map(|loot| loot.dungeon_encounter_id as i32)
            .unwrap_or(0);

        if let Some(loot) = self.loot_table.get_mut(&owner_guid) {
            if let Some(loot_entry) = loot
                .items
                .iter_mut()
                .find(|loot_entry| loot_entry.loot_list_id == loot_list_id)
            {
                loot_entry.flags.blocked = false;
                if let Some((winner_guid, _)) = winner {
                    loot_entry.roll_winner = winner_guid;
                }
            }
        }

        self.represented_loot_rolls
            .remove(&(loot_obj, loot_list_id));
        self.publish_represented_loot_roll_ownership_like_cpp();

        let Some((winner_guid, winner_vote)) = winner else {
            let packet = LootAllPassed {
                loot_obj,
                item: loot_roll_broadcast_item_like_cpp(entry, LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP),
                dungeon_encounter_id,
            };
            if let Some(state) = finished_state {
                for (player_guid, vote) in &state.voters {
                    if vote.vote == ROLL_VOTE_NOT_VALID_LIKE_CPP {
                        self.send_represented_loot_roll_packet_to_player_like_cpp(
                            &packet,
                            *player_guid,
                        );
                    }
                }
            }
            return;
        };

        if let Some(state) = finished_state {
            self.send_represented_loot_roll_final_values_like_cpp(
                loot_obj,
                entry,
                winner_guid,
                state,
                dungeon_encounter_id,
            );
        }

        let locked = LootRollWon {
            loot_obj,
            winner: winner_guid,
            roll: i32::from(winner_vote.roll_number),
            roll_type: winner_vote.vote,
            item: loot_roll_broadcast_item_like_cpp(entry, LOOT_SLOT_TYPE_LOCKED_LIKE_CPP),
            main_spec: true,
            dungeon_encounter_id,
        };
        self.broadcast_represented_loot_roll_packet_like_cpp(&locked, entry, Some(winner_guid));

        let allow = LootRollWon {
            item: loot_roll_broadcast_item_like_cpp(entry, LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP),
            ..locked
        };
        self.send_represented_loot_roll_packet_to_player_like_cpp(&allow, winner_guid);
        self.update_represented_loot_roll_winner_criteria_like_cpp(
            winner_guid,
            entry.item_id,
            winner_vote,
        );
        self.store_represented_loot_roll_winner_item_like_cpp(
            owner_guid,
            loot_obj,
            loot_list_id,
            entry,
            winner_guid,
            winner_vote,
        )
        .await;
    }

    fn update_represented_loot_roll_vote_criteria_like_cpp(
        &mut self,
        player_guid: ObjectGuid,
        roll_type: u8,
    ) {
        match roll_type {
            ROLL_VOTE_NEED_LIKE_CPP => {
                self.record_represented_roll_any_need_criteria_like_cpp(player_guid, 1)
            }
            ROLL_VOTE_GREED_LIKE_CPP | ROLL_VOTE_DISENCHANT_LIKE_CPP => {
                self.record_represented_roll_any_greed_criteria_like_cpp(player_guid, 1)
            }
            _ => {}
        }
    }

    fn update_represented_loot_roll_winner_criteria_like_cpp(
        &mut self,
        player_guid: ObjectGuid,
        item_id: u32,
        winner_vote: RepresentedLootRollVote,
    ) {
        match winner_vote.vote {
            ROLL_VOTE_NEED_LIKE_CPP => self.record_represented_roll_need_criteria_like_cpp(
                player_guid,
                item_id,
                winner_vote.roll_number,
            ),
            ROLL_VOTE_DISENCHANT_LIKE_CPP => self.record_represented_disenchant_criteria_like_cpp(
                player_guid,
                DISENCHANT_LOOT_ROLL_CRITERIA_SPELL_LIKE_CPP,
            ),
            ROLL_VOTE_GREED_LIKE_CPP => self.record_represented_roll_greed_criteria_like_cpp(
                player_guid,
                item_id,
                winner_vote.roll_number,
            ),
            _ => {}
        }
    }

    fn record_represented_roll_any_need_criteria_like_cpp(
        &mut self,
        _player_guid: ObjectGuid,
        _quantity: u32,
    ) {
        #[cfg(test)]
        self.represented_loot_roll_criteria_events.push(
            crate::session::RepresentedLootRollCriteriaEvent::RollAnyNeed {
                player_guid: _player_guid,
                quantity: _quantity,
            },
        );
    }

    fn record_represented_roll_any_greed_criteria_like_cpp(
        &mut self,
        _player_guid: ObjectGuid,
        _quantity: u32,
    ) {
        #[cfg(test)]
        self.represented_loot_roll_criteria_events.push(
            crate::session::RepresentedLootRollCriteriaEvent::RollAnyGreed {
                player_guid: _player_guid,
                quantity: _quantity,
            },
        );
    }

    fn record_represented_roll_need_criteria_like_cpp(
        &mut self,
        _player_guid: ObjectGuid,
        _item_id: u32,
        _roll_number: u8,
    ) {
        #[cfg(test)]
        self.represented_loot_roll_criteria_events.push(
            crate::session::RepresentedLootRollCriteriaEvent::RollNeed {
                player_guid: _player_guid,
                item_id: _item_id,
                roll_number: _roll_number,
            },
        );
    }

    fn record_represented_roll_greed_criteria_like_cpp(
        &mut self,
        _player_guid: ObjectGuid,
        _item_id: u32,
        _roll_number: u8,
    ) {
        #[cfg(test)]
        self.represented_loot_roll_criteria_events.push(
            crate::session::RepresentedLootRollCriteriaEvent::RollGreed {
                player_guid: _player_guid,
                item_id: _item_id,
                roll_number: _roll_number,
            },
        );
    }

    fn record_represented_disenchant_criteria_like_cpp(
        &mut self,
        _player_guid: ObjectGuid,
        _spell_id: u32,
    ) {
        #[cfg(test)]
        self.represented_loot_roll_criteria_events.push(
            crate::session::RepresentedLootRollCriteriaEvent::Disenchant {
                player_guid: _player_guid,
                spell_id: _spell_id,
            },
        );
    }

    fn send_represented_loot_roll_final_values_like_cpp(
        &self,
        loot_obj: ObjectGuid,
        entry: &LootEntry,
        winner_guid: ObjectGuid,
        state: &RepresentedLootRollState,
        dungeon_encounter_id: i32,
    ) {
        for (player_guid, vote) in &state.voters {
            let (roll, roll_type) = match vote.vote {
                ROLL_VOTE_PASS_LIKE_CPP => continue,
                ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP | ROLL_VOTE_NOT_VALID_LIKE_CPP => {
                    (0, ROLL_VOTE_PASS_LIKE_CPP)
                }
                ROLL_VOTE_NEED_LIKE_CPP
                | ROLL_VOTE_GREED_LIKE_CPP
                | ROLL_VOTE_DISENCHANT_LIKE_CPP => (i32::from(vote.roll_number), vote.vote),
                _ => continue,
            };

            let ongoing = LootRollBroadcast {
                loot_obj,
                player: *player_guid,
                roll,
                roll_type,
                item: loot_roll_broadcast_item_like_cpp(
                    entry,
                    LOOT_SLOT_TYPE_ROLL_ONGOING_LIKE_CPP,
                ),
                autopassed: false,
                off_spec: false,
                dungeon_encounter_id,
            };

            self.broadcast_represented_loot_roll_packet_to_voters_like_cpp(
                &ongoing,
                state,
                Some(winner_guid),
            );

            let allow = LootRollBroadcast {
                item: loot_roll_broadcast_item_like_cpp(entry, LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP),
                ..ongoing
            };
            self.send_represented_loot_roll_packet_to_player_like_cpp(&allow, winner_guid);
        }
    }

    fn send_represented_loot_roll_packet_to_player_like_cpp<P: ServerPacket>(
        &self,
        packet: &P,
        target: ObjectGuid,
    ) {
        if self.player_guid() == Some(target) {
            self.send_packet(packet);
            return;
        }

        let Some(registry) = self.player_registry() else {
            return;
        };
        let Some(player) = registry.get(&target) else {
            return;
        };
        if player.map_id != self.player_map_id_like_cpp() {
            return;
        }

        let _ = player.send_tx.send(packet.to_bytes());
    }

    fn broadcast_represented_loot_roll_packet_like_cpp<P: ServerPacket>(
        &self,
        packet: &P,
        entry: &LootEntry,
        except: Option<ObjectGuid>,
    ) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };

        let bytes = packet.to_bytes();
        for looter in &entry.allowed_looters {
            if Some(*looter) == except {
                continue;
            }

            if *looter == player_guid {
                self.send_packet(packet);
                continue;
            }

            let Some(registry) = self.player_registry() else {
                continue;
            };
            let Some(player) = registry.get(looter) else {
                continue;
            };
            if player.map_id != self.player_map_id_like_cpp() {
                continue;
            }

            let _ = player.send_tx.send(bytes.clone());
        }
    }

    fn broadcast_represented_loot_roll_packet_to_voters_like_cpp<P: ServerPacket>(
        &self,
        packet: &P,
        state: &RepresentedLootRollState,
        except: Option<ObjectGuid>,
    ) {
        let bytes = packet.to_bytes();
        for (player_guid, vote) in &state.voters {
            if vote.vote == ROLL_VOTE_NOT_VALID_LIKE_CPP {
                continue;
            }
            if Some(*player_guid) == except {
                continue;
            }

            if self.player_guid() == Some(*player_guid) {
                self.send_packet(packet);
                continue;
            }

            let Some(registry) = self.player_registry() else {
                continue;
            };
            let Some(player) = registry.get(player_guid) else {
                continue;
            };
            if player.map_id != self.player_map_id_like_cpp() {
                continue;
            }

            let _ = player.send_tx.send(bytes.clone());
        }
    }

    /// CMSG_MASTER_LOOT_ITEM — master looter assigns loot to a target.
    ///
    /// C++ first rejects players that are not in a group or are not the group's
    /// master looter with `LOOT_ERROR_DIDNT_KILL`. Current Rust group state has
    /// loot method `MASTER_LOOT` and the stored master-looter GUID matching the
    /// current player.
    pub async fn handle_master_loot_item(&mut self, master_loot_item: MasterLootItem) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };

        let is_represented_master_looter =
            if let (Some(group_guid), Some(registry)) = (self.group_guid, self.group_registry()) {
                registry.get(&group_guid).is_some_and(|group| {
                    group.loot_method == LOOT_METHOD_MASTER_LIKE_CPP
                        && group.master_looter_guid == player_guid
                })
            } else {
                false
            };

        if !is_represented_master_looter {
            self.send_loot_error_like_cpp(
                ObjectGuid::EMPTY,
                ObjectGuid::EMPTY,
                LOOT_ERROR_DIDNT_KILL_LIKE_CPP,
            );
            return;
        }

        if !self.represented_master_loot_target_exists_like_cpp(master_loot_item.target) {
            self.send_loot_error_like_cpp(
                ObjectGuid::EMPTY,
                ObjectGuid::EMPTY,
                LOOT_ERROR_PLAYER_NOT_FOUND_LIKE_CPP,
            );
            return;
        }

        let mut current_session_assignments = 0_u32;

        for req in &master_loot_item.loot {
            let Some(owner_guid) = self.active_loot_owner_for_loot_object_like_cpp(req.object)
            else {
                return;
            };

            if !self.represented_master_loot_target_eligible_like_cpp(master_loot_item.target) {
                self.send_loot_error_like_cpp(
                    req.object,
                    owner_guid,
                    LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
                );
                return;
            }

            let Some(loot) = self.loot_table.get(&owner_guid) else {
                return;
            };
            let dungeon_encounter_id = loot.dungeon_encounter_id;

            if loot.loot_method != LOOT_METHOD_MASTER_LIKE_CPP {
                return;
            }

            if !loot.allowed_looters.contains(&master_loot_item.target) {
                self.send_loot_error_like_cpp(
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
                self.send_loot_error_like_cpp(
                    req.object,
                    owner_guid,
                    LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
                );
                return;
            }

            if let Some(error) = self.represented_master_loot_can_store_error_like_cpp(
                master_loot_item.target,
                item.item_id,
                item.quantity,
            ) {
                self.send_loot_error_like_cpp(req.object, owner_guid, error);
                return;
            }

            let entry = item.clone();
            if master_loot_item.target == player_guid {
                if !self
                    .store_direct_loot_item_like_cpp(&entry, dungeon_encounter_id)
                    .await
                {
                    return;
                }
                self.mark_represented_master_loot_item_removed_like_cpp(
                    owner_guid,
                    req.object,
                    req.loot_list_id,
                    master_loot_item.target,
                );
                current_session_assignments = current_session_assignments.saturating_add(1);
            } else {
                match self
                    .request_represented_remote_master_loot_give_like_cpp(
                        master_loot_item.target,
                        owner_guid,
                        req.object,
                        req.loot_list_id,
                        dungeon_encounter_id,
                        entry,
                    )
                    .await
                {
                    MasterLootGiveResult::Stored => {
                        self.mark_represented_master_loot_item_removed_like_cpp(
                            owner_guid,
                            req.object,
                            req.loot_list_id,
                            master_loot_item.target,
                        );
                    }
                    MasterLootGiveResult::StoreFailed(error) => {
                        self.send_loot_error_like_cpp(req.object, owner_guid, error);
                        return;
                    }
                    MasterLootGiveResult::TargetMismatch => {
                        self.send_loot_error_like_cpp(
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
            account = self.account_id,
            target = ?master_loot_item.target,
            request_count = master_loot_item.loot.len(),
            current_session_assignments,
            "CMSG_MASTER_LOOT_ITEM accepted; represented self and connected remote target assignments route through target session state"
        );
    }

    async fn request_represented_remote_master_loot_give_like_cpp(
        &self,
        target: ObjectGuid,
        owner_guid: ObjectGuid,
        loot_obj: ObjectGuid,
        loot_list_id: u8,
        dungeon_encounter_id: u32,
        entry: LootEntry,
    ) -> MasterLootGiveResult {
        let Some(player_guid) = self.player_guid() else {
            return MasterLootGiveResult::TargetMismatch;
        };
        let Some(registry) = self.player_registry() else {
            return MasterLootGiveResult::TargetMismatch;
        };
        let Some(target_info) = registry.get(&target) else {
            return MasterLootGiveResult::TargetMismatch;
        };

        let command_tx = target_info.command_tx.clone();
        drop(target_info);

        let (result_tx, result_rx) = flume::bounded(1);
        let command = SessionCommand::MasterLootGive(MasterLootGiveCommand {
            master_guid: player_guid,
            loot_owner: owner_guid,
            loot_obj,
            loot_list_id,
            dungeon_encounter_id,
            entry,
            result_tx,
        });

        if command_tx.try_send(command).is_err() {
            return MasterLootGiveResult::TargetMismatch;
        }

        timeout(REMOTE_MASTER_LOOT_COMMAND_TIMEOUT, result_rx.recv_async())
            .await
            .ok()
            .and_then(Result::ok)
            .unwrap_or(MasterLootGiveResult::TargetMismatch)
    }

    async fn store_represented_loot_roll_winner_item_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        loot_obj: ObjectGuid,
        loot_list_id: u8,
        entry: &LootEntry,
        winner_guid: ObjectGuid,
        winner_vote: RepresentedLootRollVote,
    ) {
        let dungeon_encounter_id = self
            .loot_table
            .get(&owner_guid)
            .map(|loot| loot.dungeon_encounter_id)
            .unwrap_or(0);
        if winner_vote.vote == ROLL_VOTE_DISENCHANT_LIKE_CPP {
            if self
                .store_represented_disenchant_loot_winner_like_cpp(
                    owner_guid,
                    loot_obj,
                    loot_list_id,
                    entry,
                    winner_guid,
                    dungeon_encounter_id,
                )
                .await
            {
                self.mark_represented_master_loot_item_removed_like_cpp(
                    owner_guid,
                    loot_obj,
                    loot_list_id,
                    winner_guid,
                );
            }
            return;
        }

        if self.char_db().is_none() {
            return;
        }

        let mut store_entry = self
            .loot_table
            .get(&owner_guid)
            .and_then(|loot| {
                loot.items
                    .iter()
                    .find(|loot_entry| loot_entry.loot_list_id == loot_list_id)
                    .cloned()
            })
            .unwrap_or_else(|| entry.clone());
        store_entry.roll_winner = winner_guid;

        if self.player_guid() == Some(winner_guid) {
            if self
                .store_direct_loot_item_like_cpp(&store_entry, dungeon_encounter_id)
                .await
            {
                self.mark_represented_master_loot_item_removed_like_cpp(
                    owner_guid,
                    loot_obj,
                    loot_list_id,
                    winner_guid,
                );
            }
            return;
        }

        match self
            .request_represented_remote_loot_roll_winner_store_like_cpp(
                winner_guid,
                owner_guid,
                loot_obj,
                loot_list_id,
                dungeon_encounter_id,
                store_entry,
            )
            .await
        {
            MasterLootGiveResult::Stored => {
                self.mark_represented_master_loot_item_removed_like_cpp(
                    owner_guid,
                    loot_obj,
                    loot_list_id,
                    winner_guid,
                );
            }
            MasterLootGiveResult::StoreFailed(error) => {
                debug!(
                    account = self.account_id,
                    winner = ?winner_guid,
                    loot_obj = ?loot_obj,
                    loot_list_id,
                    error,
                    "represented loot-roll winner store failed in target session"
                );
            }
            MasterLootGiveResult::TargetMismatch => {
                debug!(
                    account = self.account_id,
                    winner = ?winner_guid,
                    loot_obj = ?loot_obj,
                    loot_list_id,
                    "represented loot-roll winner store target was not connected"
                );
            }
        }
    }

    async fn store_represented_disenchant_loot_winner_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        loot_obj: ObjectGuid,
        loot_list_id: u8,
        entry: &LootEntry,
        winner_guid: ObjectGuid,
        dungeon_encounter_id: u32,
    ) -> bool {
        let Some(template) = self
            .item_stats_store()
            .and_then(|store| store.random_property_template(entry.item_id))
        else {
            return false;
        };
        let Some((disenchant_id, _)) = self.item_disenchant_loot_like_cpp(
            entry.item_id,
            template.quality as u32,
            u32::from(template.item_level),
            true,
        ) else {
            return false;
        };

        let disenchant_entries = self
            .generate_represented_disenchant_loot_template_entries_like_cpp(
                disenchant_id,
                winner_guid,
            )
            .await;
        if disenchant_entries.is_empty() {
            return false;
        }

        if self.player_guid() == Some(winner_guid) {
            for disenchant_entry in &disenchant_entries {
                if !self
                    .store_direct_loot_item_like_cpp(disenchant_entry, dungeon_encounter_id)
                    .await
                {
                    return false;
                }
            }
            return true;
        }

        for disenchant_entry in disenchant_entries {
            match self
                .request_represented_remote_loot_roll_winner_store_like_cpp(
                    winner_guid,
                    owner_guid,
                    loot_obj,
                    loot_list_id,
                    dungeon_encounter_id,
                    disenchant_entry,
                )
                .await
            {
                MasterLootGiveResult::Stored => {}
                MasterLootGiveResult::StoreFailed(error) => {
                    debug!(
                        account = self.account_id,
                        winner = ?winner_guid,
                        loot_obj = ?loot_obj,
                        loot_list_id,
                        error,
                        "represented disenchant loot winner store failed in target session"
                    );
                    return false;
                }
                MasterLootGiveResult::TargetMismatch => {
                    debug!(
                        account = self.account_id,
                        winner = ?winner_guid,
                        loot_obj = ?loot_obj,
                        loot_list_id,
                        "represented disenchant loot winner target was not connected"
                    );
                    return false;
                }
            }
        }

        true
    }

    async fn generate_represented_disenchant_loot_template_entries_like_cpp(
        &self,
        disenchant_id: u32,
        winner_guid: ObjectGuid,
    ) -> Vec<LootEntry> {
        let mut loot_items = Vec::new();
        let mut frames = vec![disenchant_loot_template_frame_like_cpp(
            self.load_represented_disenchant_loot_template_rows_like_cpp(
                DisenchantLootTemplateTable::Disenchant,
                disenchant_id,
            )
            .await,
            0,
        )];

        let mut processed_frames = 0u32;
        while let Some(mut frame) = frames.pop() {
            if frame.requested_group_id > 0 {
                let group_index = usize::from(frame.requested_group_id - 1);
                if let Some(group) = frame.template.groups().get(group_index) {
                    let mut rng = rand::thread_rng();
                    if let Some(row) =
                        group.roll_like_cpp(LOOT_MODE_DEFAULT_LIKE_CPP, &mut rng, |item| {
                            self.item_storage_template(item.item_id).is_some()
                        })
                    {
                        let count =
                            rng.gen_range(u32::from(row.min_count)..=u32::from(row.max_count));
                        add_loot_item_stacks_like_cpp(
                            &mut loot_items,
                            row.item_id,
                            count,
                            self.item_storage_template(row.item_id)
                                .map(|template| template.max_stack_size)
                                .unwrap_or(1)
                                .max(1),
                            LootEntryFlags {
                                follow_loot_rules: true,
                                ..Default::default()
                            },
                        );
                    }
                }
                continue;
            }

            if frame.entry_index >= frame.template.entries().len() {
                if frame.group_index >= frame.template.groups().len() {
                    continue;
                }

                let group_index = frame.group_index;
                frame.group_index += 1;
                frames.push(frame.clone());

                let mut rng = rand::thread_rng();
                if let Some(row) = frame.template.groups()[group_index].roll_like_cpp(
                    LOOT_MODE_DEFAULT_LIKE_CPP,
                    &mut rng,
                    |item| self.item_storage_template(item.item_id).is_some(),
                ) {
                    let count = rng.gen_range(u32::from(row.min_count)..=u32::from(row.max_count));
                    add_loot_item_stacks_like_cpp(
                        &mut loot_items,
                        row.item_id,
                        count,
                        self.item_storage_template(row.item_id)
                            .map(|template| template.max_stack_size)
                            .unwrap_or(1)
                            .max(1),
                        LootEntryFlags {
                            follow_loot_rules: true,
                            ..Default::default()
                        },
                    );
                }
                continue;
            }

            let row = frame.template.entries()[frame.entry_index];
            frame.entry_index += 1;
            frames.push(frame);

            if row.reference > 0 {
                if !represented_disenchant_loot_reference_row_can_roll_like_cpp(&row) {
                    continue;
                }
                if row.chance < 100.0
                    && !roll_chance_with_rate_like_cpp(
                        row.chance,
                        self.loot_drop_rates_like_cpp().item_referenced,
                        &mut rand::thread_rng(),
                    )
                {
                    continue;
                }

                let reference_rows = self
                    .load_represented_disenchant_loot_template_rows_like_cpp(
                        DisenchantLootTemplateTable::Reference,
                        row.reference,
                    )
                    .await;
                let max_count = referenced_loot_max_count_like_cpp(
                    row.max_count,
                    self.loot_drop_rates_like_cpp().item_referenced_amount,
                );
                for _ in 0..max_count {
                    frames.push(disenchant_loot_template_frame_like_cpp(
                        reference_rows.clone(),
                        row.group_id,
                    ));
                }
                processed_frames = processed_frames.saturating_add(1);
                if processed_frames > MAX_LOOT_REFERENCE_FRAMES_LIKE_CPP {
                    warn!(
                        disenchant_id,
                        reference = row.reference,
                        "stopped represented disenchant loot reference processing after safety cap"
                    );
                    break;
                }
                continue;
            }

            if !represented_disenchant_loot_plain_row_can_roll_like_cpp(
                &row,
                self.item_storage_template(row.item_id).is_some(),
            ) {
                continue;
            }
            if row.chance < 100.0
                && !roll_chance_with_rate_like_cpp(
                    row.chance,
                    self.item_drop_rate_like_cpp(row.item_id),
                    &mut rand::thread_rng(),
                )
            {
                continue;
            }

            let mut rng = rand::thread_rng();
            let count = rng.gen_range(u32::from(row.min_count)..=u32::from(row.max_count));
            add_loot_item_stacks_like_cpp(
                &mut loot_items,
                row.item_id,
                count,
                self.item_storage_template(row.item_id)
                    .map(|template| template.max_stack_size)
                    .unwrap_or(1)
                    .max(1),
                LootEntryFlags {
                    follow_loot_rules: true,
                    ..Default::default()
                },
            );
        }

        for (index, loot_entry) in loot_items.iter_mut().enumerate() {
            loot_entry.loot_list_id = index as u8;
            loot_entry.allowed_looters = vec![winner_guid];
            loot_entry.roll_winner = winner_guid;
        }

        loot_items
    }

    async fn load_represented_disenchant_loot_template_rows_like_cpp(
        &self,
        table: DisenchantLootTemplateTable,
        entry: u32,
    ) -> Vec<LootStoreItem> {
        let Some(world_db) = self.world_db() else {
            return Vec::new();
        };

        let statement = match table {
            DisenchantLootTemplateTable::Disenchant => {
                WorldStatements::SEL_DISENCHANT_LOOT_TEMPLATE_ROWS
            }
            DisenchantLootTemplateTable::Reference => {
                WorldStatements::SEL_REFERENCE_LOOT_TEMPLATE_ROWS
            }
        };
        let mut stmt = world_db.prepare(statement);
        stmt.set_u32(0, entry);

        let mut result = match world_db.query(&stmt).await {
            Ok(result) => result,
            Err(err) => {
                warn!(
                    entry,
                    table = table.name(),
                    error = %err,
                    "failed to load represented disenchant loot template rows"
                );
                return Vec::new();
            }
        };

        let mut rows = Vec::new();
        if result.is_empty() {
            return rows;
        }

        loop {
            rows.push(LootStoreItem {
                item_id: result.try_read::<u32>(0).unwrap_or(0),
                reference: result.try_read::<u32>(1).unwrap_or(0),
                chance: result.try_read::<f32>(2).unwrap_or(0.0),
                needs_quest: false,
                loot_mode: result.try_read::<u16>(4).unwrap_or(0),
                group_id: result.try_read::<u8>(5).unwrap_or(0),
                min_count: result.try_read::<u8>(6).unwrap_or(0),
                max_count: result.try_read::<u8>(7).unwrap_or(0),
            });

            if !result.next_row() {
                break;
            }
        }

        rows
    }

    async fn request_represented_remote_loot_roll_winner_store_like_cpp(
        &self,
        target: ObjectGuid,
        owner_guid: ObjectGuid,
        loot_obj: ObjectGuid,
        loot_list_id: u8,
        dungeon_encounter_id: u32,
        entry: LootEntry,
    ) -> MasterLootGiveResult {
        let Some(registry) = self.player_registry() else {
            return MasterLootGiveResult::TargetMismatch;
        };
        let Some(target_info) = registry.get(&target) else {
            return MasterLootGiveResult::TargetMismatch;
        };

        let command_tx = target_info.command_tx.clone();
        drop(target_info);

        let (result_tx, result_rx) = flume::bounded(1);
        let command = SessionCommand::LootRollStoreWinner(LootRollStoreWinnerCommand {
            loot_owner: owner_guid,
            loot_obj,
            loot_list_id,
            dungeon_encounter_id,
            entry,
            result_tx,
        });

        if command_tx.try_send(command).is_err() {
            return MasterLootGiveResult::TargetMismatch;
        }

        timeout(REMOTE_MASTER_LOOT_COMMAND_TIMEOUT, result_rx.recv_async())
            .await
            .ok()
            .and_then(Result::ok)
            .unwrap_or(MasterLootGiveResult::TargetMismatch)
    }

    pub(crate) async fn process_represented_session_commands_like_cpp(&mut self) {
        let commands = self.drain_session_commands();
        for command in commands {
            match command {
                SessionCommand::ApplyCreatureMeleeDamageLikeCpp(command) => {
                    self.handle_apply_creature_melee_damage_like_cpp_command_like_cpp(command);
                }
                SessionCommand::CreatureAttackStartLikeCpp(command) => {
                    self.handle_creature_attack_start_like_cpp_command_like_cpp(command);
                }
                SessionCommand::MasterLootGive(command) => {
                    self.handle_represented_master_loot_give_command_like_cpp(command)
                        .await;
                }
                SessionCommand::LootRollStoreWinner(command) => {
                    self.handle_represented_loot_roll_store_winner_command_like_cpp(command)
                        .await;
                }
                SessionCommand::LootRollVote(command) => {
                    self.handle_represented_loot_roll_vote_command_like_cpp(command)
                        .await;
                }
                SessionCommand::ResetSeasonalQuestStatus(command) => {
                    let _ = self.reset_seasonal_quest_status_like_cpp(
                        command.event_id,
                        command.event_start_time,
                    );
                }
                SessionCommand::SendVisibleObjectValuesUpdate(command) => {
                    self.handle_send_visible_object_values_update_command_like_cpp(command);
                }
                SessionCommand::RefreshVisibleWorldCreaturesLikeCpp(command) => {
                    self.handle_refresh_visible_world_creatures_like_cpp_command_like_cpp(command)
                        .await;
                }
                SessionCommand::RefreshVisibleGameobjectsOrSpellClicksLikeCpp => {
                    let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
                }
                SessionCommand::SyncGatheringNodeGameobjectStateAndRefreshLikeCpp(command) => {
                    self.handle_sync_gathering_node_gameobject_state_and_refresh_like_cpp(command);
                }
                SessionCommand::SyncChestGameobjectStateAndRefreshLikeCpp(command) => {
                    self.handle_sync_chest_gameobject_state_and_refresh_like_cpp(command);
                }
                SessionCommand::SyncGooberGameobjectStateAndRefreshLikeCpp(command) => {
                    self.handle_sync_goober_gameobject_state_and_refresh_like_cpp(command);
                }
                SessionCommand::SetQuestSharingInfoAndSendDetails(command) => {
                    self.handle_set_quest_sharing_info_and_send_details_command_like_cpp(command);
                }
                SessionCommand::SendRepeatableTurnInRequestItemsLikeCpp(command) => {
                    self.handle_send_repeatable_turn_in_request_items_command_like_cpp(command);
                }
                SessionCommand::SendIfVisibleLikeCpp(command) => {
                    self.handle_send_if_visible_like_cpp_command_like_cpp(command);
                }
            }
        }
    }

    /// Mirrors the small gathering-node state subset that C++ keeps on the
    /// shared GameObject before asking this session to recompute its visible
    /// GameObject dynamic-flag deltas.
    fn handle_sync_gathering_node_gameobject_state_and_refresh_like_cpp(
        &mut self,
        command: SyncGatheringNodeGameobjectStateAndRefreshLikeCppCommand,
    ) {
        if self.state() != SessionState::LoggedIn {
            return;
        }
        if command.map_id != self.player_map_id_like_cpp() {
            return;
        }
        let current_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        if command.instance_id != current_instance_id {
            return;
        }
        if u32::from(command.go_type) != GAMEOBJECT_TYPE_GATHERING_NODE {
            return;
        }
        let loot_state = match command.loot_state {
            Some(0) => Some(LootState::NotReady),
            Some(1) => Some(LootState::Ready),
            Some(2) => Some(LootState::Activated),
            Some(3) => Some(LootState::JustDeactivated),
            Some(_) => return,
            None => None,
        };
        let go_state = match command.go_state {
            Some(0) => Some(GoState::Active),
            Some(1) => Some(GoState::Ready),
            Some(2) => Some(GoState::Destroyed),
            Some(24) => Some(GoState::TransportActive),
            Some(25) => Some(GoState::TransportStopped),
            Some(_) => return,
            None => None,
        };

        {
            let state = self
                .represented_gameobject_use_states
                .entry(command.gameobject_guid)
                .or_default();
            state.map_id = Some(command.map_id);
            state.go_type = Some(command.go_type);
            state.loot_state = loot_state;
            state.loot_state_unit_guid = command.loot_state_unit_guid;
            state.go_state = go_state;
            state.dynamic_flags = command.dynamic_flags;
            state.gathering_node_loot_id = command.gathering_node_loot_id;
            state.personal_loot_uses = command.personal_loot_uses;
            state.linked_trap_entry = command.linked_trap_entry;
            state.linked_trap_guid = command.linked_trap_guid;
        }

        let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
    }

    /// Mirrors the small chest state subset that C++ keeps on the shared
    /// GameObject before asking this session to recompute visible GameObject
    /// dynamic-flag deltas.
    fn handle_sync_chest_gameobject_state_and_refresh_like_cpp(
        &mut self,
        command: SyncChestGameobjectStateAndRefreshLikeCppCommand,
    ) {
        if self.state() != SessionState::LoggedIn {
            return;
        }
        if command.map_id != self.player_map_id_like_cpp() {
            return;
        }
        let current_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        if command.instance_id != current_instance_id {
            return;
        }
        if u32::from(command.go_type) != GAMEOBJECT_TYPE_CHEST {
            return;
        }
        let loot_state = match command.loot_state {
            Some(0) => Some(LootState::NotReady),
            Some(1) => Some(LootState::Ready),
            Some(2) => Some(LootState::Activated),
            Some(3) => Some(LootState::JustDeactivated),
            Some(_) => return,
            None => None,
        };

        {
            let state = self
                .represented_gameobject_use_states
                .entry(command.gameobject_guid)
                .or_default();
            state.map_id = Some(command.map_id);
            state.go_type = Some(command.go_type);
            state.loot_state = loot_state;
            state.loot_state_unit_guid = command.loot_state_unit_guid;
            state.chest_loot_source = Some(GameObjectLootSource {
                loot_id: command.chest_loot_id,
                use_group_loot_rules: false,
                dungeon_encounter_id: 0,
                personal_loot_id: command.chest_personal_loot_id,
                push_loot_id: command.chest_push_loot_id,
                triggered_event_id: 0,
                linked_trap_entry: command.linked_trap_entry.unwrap_or_default(),
                chest_restock_time_secs: command.chest_restock_time_secs,
                chest_consumable: command.chest_consumable,
                chest_quest_id: command.chest_quest_id,
            });
            state.chest_restock_time_secs = Some(command.chest_restock_time_secs);
            state.chest_consumable = Some(command.chest_consumable);
            state.chest_personal_loot_id = Some(command.chest_personal_loot_id);
            state.linked_trap_entry = command.linked_trap_entry;
            state.linked_trap_guid = command.linked_trap_guid;
        }

        let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
    }

    /// Mirrors the small shared goober state subset that C++ keeps on the
    /// shared GameObject before asking this session to recompute visible
    /// GameObject dynamic-flag deltas. This intentionally does not import the
    /// cooldown/source ownership fields; the map-owned close/despawn path is a
    /// later runtime slice.
    fn handle_sync_goober_gameobject_state_and_refresh_like_cpp(
        &mut self,
        command: SyncGooberGameobjectStateAndRefreshLikeCppCommand,
    ) {
        if self.state() != SessionState::LoggedIn {
            return;
        }
        if command.map_id != self.player_map_id_like_cpp() {
            return;
        }
        let current_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        if command.instance_id != current_instance_id {
            return;
        }
        if u32::from(command.go_type) != GAMEOBJECT_TYPE_GOOBER {
            return;
        }
        let loot_state = match command.loot_state {
            Some(0) => Some(LootState::NotReady),
            Some(1) => Some(LootState::Ready),
            Some(2) => Some(LootState::Activated),
            Some(3) => Some(LootState::JustDeactivated),
            Some(_) => return,
            None => None,
        };
        let go_state = match command.go_state {
            Some(0) => Some(GoState::Active),
            Some(1) => Some(GoState::Ready),
            Some(2) => Some(GoState::Destroyed),
            Some(24) => Some(GoState::TransportActive),
            Some(25) => Some(GoState::TransportStopped),
            Some(_) => return,
            None => None,
        };

        {
            let state = self
                .represented_gameobject_use_states
                .entry(command.gameobject_guid)
                .or_default();
            state.map_id = Some(command.map_id);
            state.go_type = Some(command.go_type);
            state.gameobject_flags = command.gameobject_flags;
            state.loot_state = loot_state;
            state.loot_state_unit_guid = command.loot_state_unit_guid;
            state.go_state = go_state;
            state.dynamic_flags = command.dynamic_flags;
            state.linked_trap_entry = command.linked_trap_entry;
            state.linked_trap_guid = command.linked_trap_guid;
        }

        let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
    }

    /// Apply a map-owned creature melee hit to this player session.
    ///
    /// C++ contrast: `Creature::Update` calls `DoMeleeAttackIfReady()`, which
    /// eventually emits `AttackerStateUpdate` from the map update tick and
    /// then applies damage to the victim. The global creature combat driver
    /// owns the swing timer/damage/canonical health mutation once; this
    /// command is only the victim-session delivery rail. It sets represented
    /// player health to the final value instead of subtracting damage, so
    /// retries cannot double apply the hit and delayed command delivery cannot
    /// stale-overwrite newer canonical health.
    fn handle_apply_creature_melee_damage_like_cpp_command_like_cpp(
        &mut self,
        command: ApplyCreatureMeleeDamageLikeCppCommand,
    ) {
        if self.state() != crate::session::SessionState::LoggedIn {
            return;
        }
        if self.player_guid() != Some(command.victim_guid) {
            return;
        }
        if !self.player_is_alive_like_cpp() {
            return;
        }
        if self.player_map_id_like_cpp() != command.map_id {
            return;
        }
        let session_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|k| k.instance_id)
            .unwrap_or(0);
        if session_instance_id != command.instance_id {
            return;
        }
        if !self
            .client_visible_guids_like_cpp
            .contains(&command.attacker_guid)
        {
            return;
        }

        let health_after = command.victim_health_after;
        self.set_player_health_after_runtime_damage_like_cpp(health_after);

        use wow_packet::packets::combat::{
            AttackerStateUpdate, HIT_INFO_NORMAL_SWING, HealthUpdate, VICTIM_STATE_HIT,
        };
        self.send_packet(&AttackerStateUpdate {
            attacker: command.attacker_guid,
            victim: command.victim_guid,
            hit_info: HIT_INFO_NORMAL_SWING,
            damage: command.damage.min(i32::MAX as u32) as i32,
            over_damage: command.over_damage,
            victim_state: VICTIM_STATE_HIT,
            school_mask: 1,
            target_level: command.target_level,
            expansion: 2,
        });
        self.send_packet(&HealthUpdate {
            guid: command.victim_guid,
            health: command.victim_health_after.min(i64::MAX as u64) as i64,
        });
    }

    /// Mirror one map-owned creature aggro transition into this victim session.
    ///
    /// C++ contrast: `CreatureAI::MoveInLineOfSight` calls
    /// `Creature::CanStartAttack` and then engages the target; the combat start
    /// is visible to the client through `Unit::SendMeleeAttackStart`. The map
    /// runtime owns the aggro decision; this handler only gates the victim
    /// session and sends one `AttackStart` packet.
    fn handle_creature_attack_start_like_cpp_command_like_cpp(
        &mut self,
        command: CreatureAttackStartLikeCppCommand,
    ) {
        if self.state() != crate::session::SessionState::LoggedIn {
            return;
        }
        if self.player_guid() != Some(command.victim_guid) {
            return;
        }
        if !self.player_is_alive_like_cpp() {
            return;
        }
        if self.player_map_id_like_cpp() != command.map_id {
            return;
        }
        let session_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|k| k.instance_id)
            .unwrap_or(0);
        if session_instance_id != command.instance_id {
            return;
        }
        if !self
            .client_visible_guids_like_cpp
            .contains(&command.attacker_guid)
        {
            return;
        }

        self.combat_target = Some(command.attacker_guid);
        self.in_combat = true;

        use wow_packet::packets::combat::AttackStart;
        self.send_packet(&AttackStart {
            attacker: command.attacker_guid,
            victim: command.victim_guid,
        });
    }

    fn handle_send_visible_object_values_update_command_like_cpp(
        &mut self,
        command: wow_network::player_registry::SendVisibleObjectValuesUpdateCommand,
    ) {
        if self.state() != crate::session::SessionState::LoggedIn {
            return;
        }
        if self.player_map_id_like_cpp() != command.map_id {
            return;
        }
        if !self
            .client_visible_guids_like_cpp
            .contains(&command.object_guid)
        {
            return;
        }

        if let Some(unit_values_update) = command.unit_values_update {
            let update = self.represented_unit_packet_update_to_update_object_like_cpp(
                command.object_guid,
                command.map_id,
                unit_values_update,
            );
            self.send_packet(&update);
        } else {
            self.send_raw_packet(&command.packet_bytes);
        }
    }

    /// Per-session gate for [`SessionCommand::SendIfVisibleLikeCpp`].
    ///
    /// Mirrors C++ `GridNotifiers.h : MessageDistDeliverer::SendPacket` and
    /// `GridNotifiersImpl.h : MessageDistDeliverer::Visit(PlayerMapType&)`:
    /// HaveAtClient (`client_visible_guids_like_cpp`) is the final gate after
    /// map/instance filtering.  No visibility or phase logic is duplicated here.
    fn handle_send_if_visible_like_cpp_command_like_cpp(
        &mut self,
        command: SendIfVisibleLikeCppCommand,
    ) {
        // Gate 1: session must be fully logged in (player object loaded).
        if self.state() != crate::session::SessionState::LoggedIn {
            return;
        }
        // Gate 2: map must match.
        if self.player_map_id_like_cpp() != command.map_id {
            return;
        }
        // Gate 3: instance must match.
        let session_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|k| k.instance_id)
            .unwrap_or(0);
        if session_instance_id != command.instance_id {
            return;
        }
        // Gate 4: source GUID must be in client's visible set (HaveAtClient).
        if !self
            .client_visible_guids_like_cpp
            .contains(&command.source_guid)
        {
            return;
        }
        // All gates passed — deliver the already-serialised packet as-is.
        self.send_raw_packet(&command.packet_bytes);
    }

    /// Recompute this session's map-owned creature visibility.
    ///
    /// This is the session-local side of future global creature CREATE/DESTROY
    /// work. C++ performs creature create/out-of-range decisions in
    /// `Player::UpdateVisibilityOf`; this command reuses Rust's represented
    /// `update_visibility` pass instead of sending raw bytes that cannot update
    /// `client_visible_guids_like_cpp`.
    async fn handle_refresh_visible_world_creatures_like_cpp_command_like_cpp(
        &mut self,
        command: RefreshVisibleWorldCreaturesLikeCppCommand,
    ) {
        if self.state() != crate::session::SessionState::LoggedIn {
            return;
        }
        if self.player_map_id_like_cpp() != command.map_id {
            return;
        }
        let session_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|k| k.instance_id)
            .unwrap_or(0);
        if session_instance_id != command.instance_id {
            return;
        }
        self.force_update_visibility_like_cpp().await;
    }

    fn handle_send_repeatable_turn_in_request_items_command_like_cpp(
        &mut self,
        command: SendRepeatableTurnInRequestItemsLikeCppCommand,
    ) {
        self.send_repeatable_turn_in_request_items_like_cpp(command.sender_guid, &command.quest);
    }

    fn handle_set_quest_sharing_info_and_send_details_command_like_cpp(
        &mut self,
        command: SetQuestSharingInfoAndSendDetailsCommand,
    ) {
        let Some(receiver_guid) = self.player_guid() else {
            return;
        };

        self.set_represented_pending_quest_sharing_like_cpp(command.sender_guid, command.quest.id);
        self.send_represented_quest_giver_quest_details_like_cpp(receiver_guid, &command.quest);
    }

    async fn handle_represented_loot_roll_vote_command_like_cpp(
        &mut self,
        command: LootRollVoteCommand,
    ) {
        let roll = LootRoll {
            loot_obj: command.loot_obj,
            loot_list_id: command.loot_list_id,
            roll_type: command.roll_type,
        };

        let _ = self
            .represented_player_vote_on_loot_roll_with_pass_state_like_cpp(
                &roll,
                command.voter_guid,
                command.pass_on_group_loot,
            )
            .await;
    }

    async fn handle_represented_master_loot_give_command_like_cpp(
        &mut self,
        command: MasterLootGiveCommand,
    ) {
        let Some(player_guid) = self.player_guid() else {
            let _ = command.result_tx.send(MasterLootGiveResult::TargetMismatch);
            return;
        };

        if command.entry.allowed_looters.is_empty()
            || !command.entry.allowed_looters.contains(&player_guid)
        {
            let _ = command.result_tx.send(MasterLootGiveResult::StoreFailed(
                LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
            ));
            return;
        }

        if let Some(error) = self.represented_master_loot_can_store_error_like_cpp(
            player_guid,
            command.entry.item_id,
            command.entry.quantity,
        ) {
            let _ = command
                .result_tx
                .send(MasterLootGiveResult::StoreFailed(error));
            return;
        }

        let result = if self
            .store_direct_loot_item_like_cpp(&command.entry, command.dungeon_encounter_id)
            .await
        {
            MasterLootGiveResult::Stored
        } else {
            MasterLootGiveResult::StoreFailed(LOOT_ERROR_MASTER_OTHER_LIKE_CPP)
        };

        debug!(
            account = self.account_id,
            master = ?command.master_guid,
            owner = ?command.loot_owner,
            loot_obj = ?command.loot_obj,
            loot_list_id = command.loot_list_id,
            ?result,
            "processed represented remote master-loot give command"
        );

        let _ = command.result_tx.send(result);
    }

    async fn handle_represented_loot_roll_store_winner_command_like_cpp(
        &mut self,
        command: LootRollStoreWinnerCommand,
    ) {
        let Some(player_guid) = self.player_guid() else {
            let _ = command.result_tx.send(MasterLootGiveResult::TargetMismatch);
            return;
        };

        if command.entry.allowed_looters.is_empty()
            || !command.entry.allowed_looters.contains(&player_guid)
            || !command.entry.roll_winner_allows_like_cpp(player_guid)
        {
            let _ = command.result_tx.send(MasterLootGiveResult::StoreFailed(
                LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
            ));
            return;
        }

        if let Some(error) = self.represented_master_loot_can_store_error_like_cpp(
            player_guid,
            command.entry.item_id,
            command.entry.quantity,
        ) {
            let _ = command
                .result_tx
                .send(MasterLootGiveResult::StoreFailed(error));
            return;
        }

        let result = if self
            .store_direct_loot_item_like_cpp(&command.entry, command.dungeon_encounter_id)
            .await
        {
            MasterLootGiveResult::Stored
        } else {
            MasterLootGiveResult::StoreFailed(LOOT_ERROR_MASTER_OTHER_LIKE_CPP)
        };

        debug!(
            account = self.account_id,
            owner = ?command.loot_owner,
            loot_obj = ?command.loot_obj,
            loot_list_id = command.loot_list_id,
            ?result,
            "processed represented remote loot-roll winner store command"
        );

        let _ = command.result_tx.send(result);
    }

    fn mark_represented_master_loot_item_removed_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        loot_obj: ObjectGuid,
        loot_list_id: u8,
        target: ObjectGuid,
    ) {
        let Some(loot) = self.loot_table.get_mut(&owner_guid) else {
            return;
        };

        let Some(entry) = loot.items.get_mut(loot_list_id as usize) else {
            return;
        };

        let was_unlooted = !entry.is_looted_for_player_like_cpp(target);
        if !was_unlooted {
            return;
        }

        entry.quantity = 0;
        entry.mark_looted_for_player_like_cpp(target);
        loot.unlooted_count = loot.unlooted_count.saturating_sub(1);
        self.send_packet(&LootRemoved {
            owner: owner_guid,
            loot_obj,
            loot_list_id,
        });
    }

    /// CMSG_SET_LOOT_SPECIALIZATION — select or clear the loot specialization.
    ///
    /// C++ accepts non-zero values only when `sChrSpecializationStore` has the
    /// row and its `ClassID` matches the player's class; `SpecID == 0` clears.
    pub async fn handle_set_loot_specialization(&mut self, packet: SetLootSpecialization) {
        if self.player_guid().is_none() {
            return;
        }

        if packet.spec_id == 0 {
            self.set_loot_specialization_id_like_cpp(0);
            return;
        }

        let Some(store) = self.chr_specialization_store() else {
            return;
        };
        let Some(spec) = store.get(packet.spec_id) else {
            return;
        };
        if spec.class_id != self.player_class_like_cpp() {
            return;
        }

        self.set_loot_specialization_id_like_cpp(packet.spec_id);
    }

    fn represented_master_loot_target_exists_like_cpp(&self, target: ObjectGuid) -> bool {
        if self.player_guid() == Some(target) {
            return true;
        }

        self.player_registry()
            .and_then(|registry| registry.get(&target))
            .is_some_and(|target_info| target_info.map_id == self.player_map_id_like_cpp())
    }

    fn represented_master_loot_target_eligible_like_cpp(&self, target: ObjectGuid) -> bool {
        let Some(group_guid) = self.group_guid else {
            return false;
        };

        let Some(group_registry) = self.group_registry() else {
            return false;
        };

        group_registry
            .get(&group_guid)
            .is_some_and(|group| group.members.contains(&target))
    }

    fn represented_master_loot_can_store_error_like_cpp(
        &self,
        target: ObjectGuid,
        item_id: u32,
        count: u32,
    ) -> Option<u8> {
        if self.player_guid() != Some(target) {
            return None;
        }

        let Some((result, _, _)) = self.plan_store_new_direct_inventory_item(item_id, count) else {
            return Some(LOOT_ERROR_MASTER_OTHER_LIKE_CPP);
        };

        master_loot_error_for_inventory_result_like_cpp(result)
    }

    async fn represented_ae_loot_creature_targets_like_cpp(
        &mut self,
        main_loot_target: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> Vec<ObjectGuid> {
        let Some(player_position) = self.player_position_like_cpp() else {
            return Vec::new();
        };

        let mut candidates: Vec<ObjectGuid> = self
            .world_creature_guids()
            .into_iter()
            .filter(|guid| {
                if *guid == main_loot_target || !guid.is_creature_or_vehicle() {
                    return false;
                }
                self.represented_creature_loot_state_like_cpp(*guid)
                    .is_some_and(|creature| {
                        !creature.is_alive
                            && player_position.is_within_dist(&creature.position, 30.0)
                    })
            })
            .collect();
        candidates.sort_by_key(|guid| (guid.high_value(), guid.low_value()));

        let mut result = Vec::new();
        for owner_guid in candidates {
            let Some(creature) = self.represented_creature_loot_state_like_cpp(owner_guid) else {
                continue;
            };
            if !creature.tappers.is_empty() && !creature.tappers.contains(&player_guid) {
                continue;
            }
            let loot_owner_guid = creature.tappers.first().copied().unwrap_or(player_guid);

            self.ensure_represented_creature_loot_like_cpp(
                owner_guid,
                loot_owner_guid,
                creature.level,
                creature.entry,
                creature.loot_id,
                creature.gold_min,
                creature.gold_max,
                creature.dungeon_encounter_id,
            )
            .await;
            if let Some(loot) = self.loot_table.get_mut(&owner_guid) {
                if creature.tappers.is_empty() {
                    if loot.allowed_looters.contains(&player_guid) {
                        mark_loot_allowed_for_player_like_cpp(loot, player_guid);
                    }
                } else {
                    for tapper in &creature.tappers {
                        mark_loot_allowed_for_player_like_cpp(loot, *tapper);
                    }
                }
            }

            if self.loot_table.get(&owner_guid).is_some_and(|loot| {
                self.represented_loot_can_be_opened_by_player_like_cpp(
                    owner_guid,
                    loot,
                    player_guid,
                )
            }) {
                result.push(owner_guid);
            }
        }

        result
    }

    async fn represented_loot_response_for_owner_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
        ae_looting: bool,
    ) -> Option<LootResponse> {
        let creature = self.represented_creature_loot_state_like_cpp(owner_guid)?;
        if !creature.tappers.is_empty() && !creature.tappers.contains(&player_guid) {
            return None;
        }
        let loot_owner_guid = creature.tappers.first().copied().unwrap_or(player_guid);
        self.ensure_represented_creature_loot_like_cpp(
            owner_guid,
            loot_owner_guid,
            creature.level,
            creature.entry,
            creature.loot_id,
            creature.gold_min,
            creature.gold_max,
            creature.dungeon_encounter_id,
        )
        .await;

        if let Some(loot) = self.loot_table.get_mut(&owner_guid) {
            if creature.tappers.is_empty() {
                if loot.allowed_looters.contains(&player_guid) {
                    mark_loot_allowed_for_player_like_cpp(loot, player_guid);
                }
            } else {
                for tapper in &creature.tappers {
                    mark_loot_allowed_for_player_like_cpp(loot, *tapper);
                }
            }
        }

        let loot = self.loot_table.get(&owner_guid)?;
        if !self.represented_loot_can_be_opened_by_player_like_cpp(owner_guid, loot, player_guid) {
            return None;
        }

        Some(LootResponse {
            owner: owner_guid,
            loot_obj: loot.loot_guid,
            failure_reason: 0,
            acquire_reason: loot_type_for_client_like_cpp(loot.loot_type),
            loot_method: loot.loot_method,
            threshold: 2,
            coins: self.represented_loot_money_for_player_like_cpp(owner_guid, loot, player_guid),
            items: represented_loot_response_items_like_cpp(loot, player_guid),
            currencies: vec![],
            acquired: true,
            ae_looting,
        })
    }

    pub(crate) async fn ensure_represented_creature_kill_loot_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
    ) {
        let Some(creature) = self.represented_creature_loot_state_like_cpp(creature_guid) else {
            return;
        };
        let Some(loot_owner_guid) = creature.tappers.first().copied() else {
            return;
        };

        self.ensure_represented_creature_loot_like_cpp(
            creature_guid,
            loot_owner_guid,
            creature.level,
            creature.entry,
            creature.loot_id,
            creature.gold_min,
            creature.gold_max,
            creature.dungeon_encounter_id,
        )
        .await;

        if let Some(loot) = self.loot_table.get_mut(&creature_guid) {
            for tapper in creature.tappers {
                mark_loot_allowed_for_player_like_cpp(loot, tapper);
            }
        }
    }

    fn represented_on_loot_opened_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) {
        self.ensure_represented_player_looting_like_cpp(owner_guid, player_guid);

        self.represented_notify_loot_list_like_cpp(owner_guid);

        let first_open = match self.loot_table.get_mut(&owner_guid) {
            Some(loot) if !loot.looted_by_player => {
                loot.looted_by_player = true;
                true
            }
            _ => false,
        };
        if !first_open {
            return;
        }

        let loot_method = self
            .loot_table
            .get(&owner_guid)
            .map(|loot| loot.loot_method)
            .unwrap_or_default();

        match loot_method {
            LOOT_METHOD_GROUP_LIKE_CPP | LOOT_METHOD_NEED_BEFORE_GREED_LIKE_CPP => {
                self.represented_start_group_loot_rolls_on_first_open_like_cpp(
                    owner_guid,
                    player_guid,
                );
            }
            LOOT_METHOD_MASTER_LIKE_CPP => {
                if let Some(packet) =
                    self.represented_master_loot_candidate_list_like_cpp(owner_guid, player_guid)
                {
                    self.send_packet(&packet);
                }
            }
            _ => {}
        }
    }

    fn represented_notify_loot_list_like_cpp(&self, owner_guid: ObjectGuid) {
        if self.group_guid.is_none() {
            return;
        }

        let Some(loot) = self.loot_table.get(&owner_guid) else {
            return;
        };

        let master = if loot.loot_method == LOOT_METHOD_MASTER_LIKE_CPP
            && loot_has_over_threshold_item_like_cpp(loot)
        {
            (!loot.loot_master.is_empty()).then_some(loot.loot_master)
        } else {
            None
        };

        let packet = LootList {
            owner: owner_guid,
            loot_obj: loot.loot_guid,
            master,
            round_robin_winner: (!loot.round_robin_player.is_empty())
                .then_some(loot.round_robin_player),
        };
        let bytes = packet.to_bytes();

        for allowed_looter in &loot.allowed_looters {
            if Some(*allowed_looter) == self.player_guid() {
                self.send_packet(&packet);
                continue;
            }

            let Some(registry) = self.player_registry() else {
                continue;
            };
            let Some(player) = registry.get(allowed_looter) else {
                continue;
            };
            if player.map_id != self.player_map_id_like_cpp() {
                continue;
            }

            let _ = player.send_tx.send(bytes.clone());
        }
    }

    fn ensure_represented_player_looting_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) {
        if let Some(loot) = self.loot_table.get_mut(&owner_guid)
            && !loot.players_looting.contains(&player_guid)
        {
            loot.players_looting.push(player_guid);
        }
    }

    fn represented_notify_loot_item_removed_like_cpp(
        &self,
        owner_guid: ObjectGuid,
        loot_list_id: u8,
    ) {
        let Some(loot) = self.loot_table.get(&owner_guid) else {
            return;
        };
        let Some(entry) = loot
            .items
            .iter()
            .find(|entry| entry.loot_list_id == loot_list_id)
        else {
            return;
        };

        let packet = LootRemoved {
            owner: owner_guid,
            loot_obj: loot.loot_guid,
            loot_list_id,
        };
        let bytes = packet.to_bytes();

        for looter in &loot.players_looting {
            if !entry.allowed_looters.contains(looter) {
                continue;
            }

            if Some(*looter) == self.player_guid() {
                self.send_packet(&packet);
                continue;
            }

            let Some(registry) = self.player_registry() else {
                continue;
            };
            let Some(player) = registry.get(looter) else {
                continue;
            };
            if player.map_id != self.player_map_id_like_cpp() {
                continue;
            }

            let _ = player.send_tx.send(bytes.clone());
        }
    }

    fn represented_notify_money_removed_like_cpp(&self, owner_guid: ObjectGuid) {
        let Some(loot) = self.loot_table.get(&owner_guid) else {
            return;
        };

        let packet = CoinRemoved {
            loot_obj: loot.loot_guid,
        };
        let bytes = packet.to_bytes();

        for looter in &loot.players_looting {
            if Some(*looter) == self.player_guid() {
                self.send_packet(&packet);
                continue;
            }

            let Some(registry) = self.player_registry() else {
                continue;
            };
            let Some(player) = registry.get(looter) else {
                continue;
            };
            if player.map_id != self.player_map_id_like_cpp() {
                continue;
            }

            let _ = player.send_tx.send(bytes.clone());
        }
    }

    fn represented_start_group_loot_rolls_on_first_open_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) {
        let current_map_id = self.player_map_id_like_cpp();
        let player_registry = self.player_registry().cloned();
        let mut packets = Vec::new();
        let mut auto_pass_packets = Vec::new();
        let mut pending_rolls = Vec::new();
        let item_flags2_by_item_id: HashMap<u32, (Option<u32>, Option<u16>)> = self
            .loot_table
            .get(&owner_guid)
            .map(|loot| {
                loot.items
                    .iter()
                    .map(|entry| {
                        (
                            entry.item_id,
                            (
                                self.item_template_flags2(entry.item_id),
                                self.represented_loot_roll_disenchant_skill_required_like_cpp(
                                    entry.item_id,
                                ),
                            ),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();

        if let Some(loot) = self.loot_table.get_mut(&owner_guid) {
            for entry in &mut loot.items {
                if !entry.flags.blocked {
                    continue;
                }

                let eligible_looters = connected_roll_looters_like_cpp(
                    entry,
                    player_guid,
                    current_map_id,
                    player_registry.as_deref(),
                );

                if eligible_looters.len() <= 1 {
                    entry.flags.under_threshold = true;
                    entry.flags.blocked = false;
                    continue;
                }

                let mut voters = HashMap::new();
                for looter in &entry.allowed_looters {
                    let vote = if *looter == player_guid {
                        if self.pass_on_group_loot {
                            ROLL_VOTE_PASS_LIKE_CPP
                        } else {
                            ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP
                        }
                    } else {
                        match player_registry
                            .as_deref()
                            .and_then(|registry| registry.get(looter))
                        {
                            Some(player) if player.map_id == current_map_id => {
                                if player.pass_on_group_loot {
                                    ROLL_VOTE_PASS_LIKE_CPP
                                } else {
                                    ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP
                                }
                            }
                            _ => ROLL_VOTE_NOT_VALID_LIKE_CPP,
                        }
                    };
                    voters.insert(
                        *looter,
                        RepresentedLootRollVote {
                            vote,
                            roll_number: 0,
                        },
                    );
                }
                let state = RepresentedLootRollState {
                    loot_obj: loot.loot_guid,
                    loot_list_id: entry.loot_list_id,
                    end_time: Instant::now()
                        + Duration::from_millis(u64::from(LOOT_ROLL_TIMEOUT_MS_LIKE_CPP)),
                    voters,
                };
                let max_enchanting_skill = represented_max_enchanting_skill_like_cpp(
                    &eligible_looters,
                    player_guid,
                    self.represented_enchanting_skill,
                    player_registry.as_deref(),
                );
                let (item_flags2, disenchant_skill_required) = item_flags2_by_item_id
                    .get(&entry.item_id)
                    .copied()
                    .unwrap_or((None, None));
                let valid_rolls = Self::represented_loot_roll_valid_rolls_like_cpp(
                    item_flags2,
                    disenchant_skill_required,
                    max_enchanting_skill,
                );

                for (looter, vote) in &state.voters {
                    if vote.vote != ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP {
                        continue;
                    }

                    packets.push((
                        *looter,
                        start_loot_roll_packet_like_cpp(
                            loot.loot_guid,
                            current_map_id,
                            loot.loot_method,
                            entry,
                            valid_rolls,
                            loot.dungeon_encounter_id as i32,
                        ),
                    ));
                }

                for (looter, vote) in &state.voters {
                    if vote.vote != ROLL_VOTE_PASS_LIKE_CPP {
                        continue;
                    }

                    auto_pass_packets.push((
                        LootRollBroadcast {
                            loot_obj: loot.loot_guid,
                            player: *looter,
                            roll: -1,
                            roll_type: ROLL_VOTE_PASS_LIKE_CPP,
                            item: loot_roll_broadcast_item_like_cpp(
                                entry,
                                LOOT_SLOT_TYPE_ROLL_ONGOING_LIKE_CPP,
                            ),
                            autopassed: false,
                            off_spec: false,
                            dungeon_encounter_id: loot.dungeon_encounter_id as i32,
                        },
                        state.clone(),
                    ));
                }

                pending_rolls.push(state);
            }
        }

        for roll in pending_rolls {
            self.represented_loot_rolls
                .insert((roll.loot_obj, roll.loot_list_id), roll);
        }
        self.publish_represented_loot_roll_ownership_like_cpp();

        for (looter, packet) in packets {
            if looter == player_guid {
                self.send_packet(&packet);
                continue;
            }

            let Some(registry) = self.player_registry() else {
                continue;
            };
            let Some(player) = registry.get(&looter) else {
                continue;
            };
            if player.map_id != self.player_map_id_like_cpp() {
                continue;
            }

            let _ = player.send_tx.send(packet.to_bytes());
        }

        for (packet, state) in auto_pass_packets {
            self.broadcast_represented_loot_roll_packet_to_voters_like_cpp(&packet, &state, None);
        }
    }

    fn publish_represented_loot_roll_ownership_like_cpp(&self) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };
        let Some(registry) = self.player_registry() else {
            return;
        };
        let Some(mut info) = registry.get_mut(&player_guid) else {
            return;
        };

        info.active_loot_rolls = self
            .represented_loot_rolls
            .keys()
            .map(|key| (key.0, key.1))
            .collect();
    }

    pub(crate) async fn tick_represented_loot_rolls_like_cpp(&mut self) {
        let now = Instant::now();
        let expired: Vec<(ObjectGuid, u8)> = self
            .represented_loot_rolls
            .iter()
            .filter_map(|(key, state)| (state.end_time <= now).then_some(*key))
            .collect();

        for (loot_obj, loot_list_id) in expired {
            let Some(state) = self
                .represented_loot_rolls
                .get(&(loot_obj, loot_list_id))
                .cloned()
            else {
                continue;
            };
            let Some(owner_guid) = self.active_loot_owner_for_loot_object_like_cpp(loot_obj) else {
                self.represented_loot_rolls
                    .remove(&(loot_obj, loot_list_id));
                self.publish_represented_loot_roll_ownership_like_cpp();
                continue;
            };
            let Some(entry) = self.loot_table.get(&owner_guid).and_then(|loot| {
                loot.items
                    .iter()
                    .find(|entry| entry.loot_list_id == loot_list_id)
                    .cloned()
            }) else {
                self.represented_loot_rolls
                    .remove(&(loot_obj, loot_list_id));
                self.publish_represented_loot_roll_ownership_like_cpp();
                continue;
            };

            let winner = represented_loot_roll_current_winner_like_cpp(&state);
            self.finish_represented_loot_roll_like_cpp(
                loot_obj,
                loot_list_id,
                &entry,
                winner,
                Some(&state),
            )
            .await;
        }
    }

    fn represented_loot_roll_valid_rolls_like_cpp(
        item_flags2: Option<u32>,
        disenchant_skill_required: Option<u16>,
        max_enchanting_skill: u16,
    ) -> u8 {
        let mut valid_rolls = ROLL_ALL_TYPE_MASK_LIKE_CPP;
        if item_flags2.is_some_and(|flags| (flags & ItemFlags2::CanOnlyRollGreed as u32) != 0) {
            valid_rolls &= !ROLL_FLAG_TYPE_NEED_LIKE_CPP;
        }
        if disenchant_skill_required
            .is_none_or(|skill_required| skill_required > max_enchanting_skill)
        {
            valid_rolls &= !ROLL_FLAG_TYPE_DISENCHANT_LIKE_CPP;
        }

        valid_rolls
    }

    fn represented_loot_roll_disenchant_skill_required_like_cpp(
        &self,
        item_id: u32,
    ) -> Option<u16> {
        let template = self
            .item_stats_store()
            .and_then(|store| store.random_property_template(item_id))?;
        self.item_disenchant_loot_like_cpp(
            item_id,
            template.quality as u32,
            u32::from(template.item_level),
            true,
        )
        .map(|(_, skill_required)| skill_required)
    }

    fn represented_master_loot_candidate_list_like_cpp(
        &self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> Option<MasterLootCandidateList> {
        let is_master_looter =
            if let (Some(group_guid), Some(registry)) = (self.group_guid, self.group_registry()) {
                registry.get(&group_guid).is_some_and(|group| {
                    group.loot_method == LOOT_METHOD_MASTER_LIKE_CPP
                        && group.master_looter_guid == player_guid
                })
            } else {
                false
            };

        let loot = self.loot_table.get(&owner_guid)?;
        if loot.loot_method != LOOT_METHOD_MASTER_LIKE_CPP || !is_master_looter {
            return None;
        }

        Some(MasterLootCandidateList {
            loot_obj: loot.loot_guid,
            players: loot.allowed_looters.clone(),
        })
    }

    async fn ensure_represented_creature_loot_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
        loot_owner_guid: ObjectGuid,
        level: u8,
        entry: u32,
        loot_id: u32,
        gold_min: u32,
        gold_max: u32,
        dungeon_encounter_id: u32,
    ) {
        if !self.loot_table.contains_key(&creature_guid) {
            let loot = self
                .generate_represented_creature_loot_like_cpp(
                    creature_guid,
                    loot_owner_guid,
                    level,
                    entry,
                    loot_id,
                    gold_min,
                    gold_max,
                    dungeon_encounter_id,
                )
                .await;
            self.loot_table.insert(creature_guid, loot);
        }
    }

    async fn ensure_represented_gameobject_chest_loot_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        source: GameObjectLootSource,
    ) {
        if !self.loot_table.contains_key(&gameobject_guid) {
            let loot = self
                .generate_represented_gameobject_chest_loot_like_cpp(
                    gameobject_guid,
                    player_guid,
                    source,
                )
                .await;
            self.loot_table.insert(gameobject_guid, loot);
        }
    }

    async fn generate_represented_gameobject_chest_loot_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        source: GameObjectLootSource,
    ) -> CreatureLoot {
        let (loot_method, loot_master, round_robin_player) = self
            .represented_gameobject_chest_group_state_like_cpp(
                source.use_group_loot_rules,
                player_guid,
            );
        let loot_id = source.open_loot_id_like_cpp();
        let personal_encounter = source.is_personal_encounter_loot_like_cpp();
        let items = if personal_encounter {
            Vec::new()
        } else {
            self.generate_represented_gameobject_loot_items_like_cpp(loot_id)
                .await
                .unwrap_or_else(|| {
                    if loot_id != 0 {
                        debug!(
                            loot_id,
                            gameobject = ?gameobject_guid,
                            "gameobject loot template unavailable for represented chest"
                        );
                    }
                    Vec::new()
                })
        };
        let (min_money, max_money) = self
            .load_gameobject_template_addon_money_loot_like_cpp(gameobject_guid.entry())
            .await;
        let coins = generate_money_loot_with_rate_like_cpp(
            min_money,
            max_money,
            self.loot_drop_rates_like_cpp().money,
            &mut rand::thread_rng(),
        );

        let mut loot = CreatureLoot {
            loot_guid: represented_loot_object_guid_like_cpp(gameobject_guid),
            coins,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
            dungeon_encounter_id: source.dungeon_encounter_id,
            loot_method,
            loot_master,
            round_robin_player,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items,
            looted_by_player: false,
        };

        if personal_encounter {
            loot.coins = 0;
            self.represented_personal_loot_owners
                .insert(gameobject_guid);
            self.represented_personal_loot_money
                .retain(|(owner, _), _| *owner != gameobject_guid);
            let represented_tappers = self
                .represented_gameobject_personal_encounter_tappers_like_cpp(
                    gameobject_guid,
                    player_guid,
                    source.dungeon_encounter_id,
                );
            for tapper in &represented_tappers {
                if !loot.allowed_looters.contains(tapper) {
                    loot.allowed_looters.push(*tapper);
                }
                let tapper_money = generate_money_loot_with_rate_like_cpp(
                    min_money,
                    max_money,
                    self.loot_drop_rates_like_cpp().money,
                    &mut rand::thread_rng(),
                );
                self.represented_personal_loot_money
                    .insert((gameobject_guid, *tapper), tapper_money);
            }
            loot.items = self
                .generate_represented_gameobject_personal_loot_items_like_cpp(
                    loot_id,
                    &represented_tappers,
                )
                .await
                .unwrap_or_else(|| {
                    if loot_id != 0 {
                        debug!(
                            loot_id,
                            gameobject = ?gameobject_guid,
                            "gameobject personal loot template unavailable for represented chest"
                        );
                    }
                    Vec::new()
                });
            rebuild_represented_personal_loot_counts_like_cpp(&mut loot);
            if represented_tappers.is_empty() {
                self.represented_personal_loot_owners
                    .remove(&gameobject_guid);
            }
        }

        loot
    }

    fn represented_gameobject_personal_encounter_tappers_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        dungeon_encounter_id: u32,
    ) -> Vec<ObjectGuid> {
        let Some(tappers) = self.represented_gameobject_tap_lists.get(&gameobject_guid) else {
            return self
                .represented_player_unlocked_for_dungeon_encounter_like_cpp(
                    player_guid,
                    dungeon_encounter_id,
                )
                .into_iter()
                .collect();
        };
        let mut represented_tappers = tappers
            .iter()
            .copied()
            .filter(|guid| guid.is_player())
            .collect::<Vec<_>>();
        represented_tappers.sort_unstable_by_key(|guid| (guid.high_value(), guid.low_value()));
        represented_tappers.dedup();
        if represented_tappers.is_empty() {
            represented_tappers.push(player_guid);
        }
        represented_tappers.retain(|guid| {
            self.represented_player_is_unlocked_for_dungeon_encounter_like_cpp(
                *guid,
                dungeon_encounter_id,
            )
        });
        represented_tappers
    }

    fn represented_player_unlocked_for_dungeon_encounter_like_cpp(
        &self,
        player_guid: ObjectGuid,
        dungeon_encounter_id: u32,
    ) -> Option<ObjectGuid> {
        self.represented_player_is_unlocked_for_dungeon_encounter_like_cpp(
            player_guid,
            dungeon_encounter_id,
        )
        .then_some(player_guid)
    }

    fn represented_player_is_unlocked_for_dungeon_encounter_like_cpp(
        &self,
        player_guid: ObjectGuid,
        dungeon_encounter_id: u32,
    ) -> bool {
        !self
            .represented_locked_dungeon_encounters
            .contains(&(player_guid, dungeon_encounter_id))
    }

    fn represented_gameobject_chest_group_state_like_cpp(
        &self,
        use_group_loot_rules: bool,
        player_guid: ObjectGuid,
    ) -> (u8, ObjectGuid, ObjectGuid) {
        if !use_group_loot_rules {
            return (0, ObjectGuid::EMPTY, ObjectGuid::EMPTY);
        }
        let Some(group_guid) = self.group_guid else {
            return (0, ObjectGuid::EMPTY, ObjectGuid::EMPTY);
        };
        let Some(registry) = self.group_registry() else {
            return (0, ObjectGuid::EMPTY, ObjectGuid::EMPTY);
        };
        let Some(group) = registry.get(&group_guid) else {
            return (0, ObjectGuid::EMPTY, ObjectGuid::EMPTY);
        };

        (group.loot_method, group.master_looter_guid, player_guid)
    }

    async fn generate_represented_gameobject_loot_items_like_cpp(
        &self,
        loot_id: u32,
    ) -> Option<Vec<LootEntry>> {
        self.generate_represented_gameobject_loot_items_for_store_like_cpp(
            loot_id,
            LootStoreKind::Gameobject,
            LOOT_MODE_DEFAULT_LIKE_CPP,
        )
        .await
    }

    async fn generate_represented_gameobject_loot_items_for_store_like_cpp(
        &self,
        loot_id: u32,
        store_kind: LootStoreKind,
        loot_mode: u16,
    ) -> Option<Vec<LootEntry>> {
        if loot_id == 0 {
            return Some(Vec::new());
        }

        let stores = self.loot_stores()?;
        let store = stores.get(&store_kind)?;
        let rates = self.loot_drop_rates_like_cpp();
        let condition_ids = store.condition_ids_for_fill_like_cpp(loot_id, store_kind, stores);
        let condition_rows = self
            .load_represented_creature_loot_condition_rows_like_cpp(&condition_ids)
            .await;
        let condition_references = self
            .load_represented_creature_loot_condition_reference_rows_like_cpp(&condition_rows)
            .await;
        let addon_metadata = self
            .load_item_template_addon_loot_metadata_for_item_ids_like_cpp(
                condition_ids.iter().map(|id| id.source_entry),
            )
            .await;
        let generated = {
            let mut rng = rand::thread_rng();
            match store.fill_loot_with_context_like_cpp(
                loot_id,
                store_kind,
                stores,
                LootFillOptions {
                    loot_mode,
                    rates_allowed: true,
                    referenced_amount_rate: rates.item_referenced_amount,
                    item_context: ItemContext::None as u8,
                },
                &mut rng,
                |item_id| {
                    self.item_storage_template(item_id)
                        .map(|template| LootItemTemplateMetadata {
                            max_stack: template.max_stack_size.max(1),
                            has_multi_drop_flag: template.flags.contains(ItemFlags::MULTI_DROP),
                            has_follow_loot_rules_flag: false,
                        })
                },
                |item| self.item_drop_rate_like_cpp(item.item_id),
                |context| {
                    self.represented_creature_loot_item_allowed_like_cpp(
                        context,
                        &condition_rows,
                        &condition_references,
                        &addon_metadata,
                    )
                },
                |item_id| {
                    let random_properties =
                        self.generate_loot_store_random_properties_like_cpp(item_id);
                    LootItemRandomProperties {
                        id: random_properties.id,
                        seed: random_properties.seed,
                    }
                },
            ) {
                Ok(generated) => generated,
                Err(LootFillError::MissingLootTemplate { .. }) => Vec::new(),
            }
        };

        Some(
            generated
                .into_iter()
                .map(|item| {
                    let metadata = addon_metadata
                        .get(&item.item_id)
                        .copied()
                        .unwrap_or_default();
                    generated_creature_loot_item_to_entry_like_cpp(item, metadata)
                })
                .collect(),
        )
    }

    async fn generate_represented_fishing_loot_items_like_cpp(
        &self,
        area_id: u32,
        loot_mode: u16,
    ) -> Option<Vec<LootEntry>> {
        let mut current_area_id = area_id;
        while current_area_id != 0 {
            let items = self
                .generate_represented_gameobject_loot_items_for_store_like_cpp(
                    current_area_id,
                    LootStoreKind::Fishing,
                    loot_mode,
                )
                .await?;
            if !items.is_empty() {
                return Some(items);
            }
            let Some(parent_area_id) = self
                .area_table_store()
                .and_then(|store| store.get(current_area_id))
                .map(|entry| u32::from(entry.parent_area_id))
            else {
                break;
            };
            current_area_id = parent_area_id;
        }

        self.generate_represented_gameobject_loot_items_for_store_like_cpp(
            1,
            LootStoreKind::Fishing,
            loot_mode,
        )
        .await
    }

    async fn generate_represented_gameobject_personal_loot_items_like_cpp(
        &self,
        loot_id: u32,
        tappers: &[ObjectGuid],
    ) -> Option<Vec<LootEntry>> {
        if loot_id == 0 || tappers.is_empty() {
            return Some(Vec::new());
        }

        let stores = self.loot_stores()?;
        let store = stores.get(&LootStoreKind::Gameobject)?;
        let rates = self.loot_drop_rates_like_cpp();
        let condition_ids =
            store.condition_ids_for_fill_like_cpp(loot_id, LootStoreKind::Gameobject, stores);
        let condition_rows = self
            .load_represented_creature_loot_condition_rows_like_cpp(&condition_ids)
            .await;
        let condition_references = self
            .load_represented_creature_loot_condition_reference_rows_like_cpp(&condition_rows)
            .await;
        let addon_metadata = self
            .load_item_template_addon_loot_metadata_for_item_ids_like_cpp(
                condition_ids.iter().map(|id| id.source_entry),
            )
            .await;
        let generated = {
            let mut rng = rand::thread_rng();
            store
                .fill_personal_loot_with_context_like_cpp(
                    loot_id,
                    LootStoreKind::Gameobject,
                    stores,
                    LootFillOptions {
                        loot_mode: LOOT_MODE_DEFAULT_LIKE_CPP,
                        rates_allowed: true,
                        referenced_amount_rate: rates.item_referenced_amount,
                        item_context: ItemContext::None as u8,
                    },
                    tappers,
                    &mut rng,
                    |item_id| {
                        self.item_storage_template(item_id).map(|template| {
                            LootItemTemplateMetadata {
                                max_stack: template.max_stack_size.max(1),
                                has_multi_drop_flag: template.flags.contains(ItemFlags::MULTI_DROP),
                                has_follow_loot_rules_flag: false,
                            }
                        })
                    },
                    |item| self.item_drop_rate_like_cpp(item.item_id),
                    |context, looter| {
                        self.represented_creature_loot_item_allowed_for_player_like_cpp(
                            context,
                            looter,
                            &condition_rows,
                            &condition_references,
                            &addon_metadata,
                        )
                    },
                    |item_id| {
                        let random_properties =
                            self.generate_loot_store_random_properties_like_cpp(item_id);
                        LootItemRandomProperties {
                            id: random_properties.id,
                            seed: random_properties.seed,
                        }
                    },
                )
                .ok()?
        };

        Some(
            generated
                .into_iter()
                .map(|personal_item| {
                    let metadata = addon_metadata
                        .get(&personal_item.item.item_id)
                        .copied()
                        .unwrap_or_default();
                    let mut entry = generated_creature_loot_item_to_entry_like_cpp(
                        personal_item.item,
                        metadata,
                    );
                    entry.add_allowed_looter_like_cpp(personal_item.looter);
                    entry
                })
                .collect(),
        )
    }

    async fn autostore_represented_gameobject_chest_push_loot_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        source: GameObjectLootSource,
    ) -> bool {
        if !source.should_autostore_push_loot_like_cpp() {
            return true;
        }

        let items = self
            .generate_represented_gameobject_loot_items_like_cpp(source.push_loot_id)
            .await
            .unwrap_or_else(|| {
                debug!(
                    loot_id = source.push_loot_id,
                    gameobject = ?gameobject_guid,
                    "gameobject push loot template unavailable for represented chest"
                );
                Vec::new()
            });

        let mut all_stored = true;
        for entry in items {
            if !self
                .store_direct_loot_item_like_cpp(&entry, source.dungeon_encounter_id)
                .await
            {
                all_stored = false;
            }
        }

        all_stored
    }

    async fn load_gameobject_template_addon_money_loot_like_cpp(
        &self,
        gameobject_entry: u32,
    ) -> (u32, u32) {
        let Some(world_db) = self.world_db() else {
            return (0, 0);
        };

        let mut stmt = world_db.prepare(WorldStatements::SEL_GAMEOBJECT_TEMPLATE_ADDON_MONEY_LOOT);
        stmt.set_u32(0, gameobject_entry);

        match world_db.query(&stmt).await {
            Ok(result) if !result.is_empty() => {
                let min_money = result.try_read::<u32>(0).unwrap_or(0);
                let max_money = result.try_read::<u32>(1).unwrap_or(0);
                (min_money, max_money)
            }
            Ok(_) => (0, 0),
            Err(err) => {
                warn!(
                    gameobject_entry,
                    "failed to load gameobject_template_addon money loot: {err}"
                );
                (0, 0)
            }
        }
    }

    async fn generate_represented_creature_loot_like_cpp(
        &self,
        creature_guid: ObjectGuid,
        loot_owner_guid: ObjectGuid,
        level: u8,
        entry: u32,
        loot_id: u32,
        gold_min: u32,
        gold_max: u32,
        dungeon_encounter_id: u32,
    ) -> CreatureLoot {
        let (loot_method, loot_master, round_robin_player) =
            self.represented_creature_loot_group_state_like_cpp(loot_owner_guid);
        let coins = if gold_max > 0 {
            generate_money_loot_with_rate_like_cpp(
                gold_min,
                gold_max,
                self.loot_drop_rates_like_cpp().money,
                &mut rand::thread_rng(),
            )
        } else {
            generate_legacy_creature_coin_fallback_like_cpp(creature_guid, level)
        };

        let items = self
            .generate_represented_creature_loot_items_like_cpp(loot_id)
            .await
            .unwrap_or_else(|| {
                if loot_id != 0 {
                    debug!(
                        entry,
                        loot_id, "creature loot template unavailable for represented corpse"
                    );
                }
                Vec::new()
            });

        CreatureLoot {
            loot_guid: represented_loot_object_guid_like_cpp(creature_guid),
            coins,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id,
            loot_method,
            loot_master,
            round_robin_player,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items,
            looted_by_player: false,
        }
    }

    fn represented_creature_loot_group_state_like_cpp(
        &self,
        loot_owner_guid: ObjectGuid,
    ) -> (u8, ObjectGuid, ObjectGuid) {
        let Some(group_guid) = self.group_guid else {
            return (0, ObjectGuid::EMPTY, ObjectGuid::EMPTY);
        };
        let Some(registry) = self.group_registry() else {
            return (0, ObjectGuid::EMPTY, ObjectGuid::EMPTY);
        };
        let Some(group) = registry.get(&group_guid) else {
            return (0, ObjectGuid::EMPTY, ObjectGuid::EMPTY);
        };

        (group.loot_method, group.master_looter_guid, loot_owner_guid)
    }

    async fn generate_represented_creature_loot_items_like_cpp(
        &self,
        loot_id: u32,
    ) -> Option<Vec<LootEntry>> {
        if loot_id == 0 {
            return Some(Vec::new());
        }

        let stores = self.loot_stores()?;
        let store = stores.get(&LootStoreKind::Creature)?;
        let rates = self.loot_drop_rates_like_cpp();
        let condition_ids =
            store.condition_ids_for_fill_like_cpp(loot_id, LootStoreKind::Creature, stores);
        let condition_rows = self
            .load_represented_creature_loot_condition_rows_like_cpp(&condition_ids)
            .await;
        let condition_references = self
            .load_represented_creature_loot_condition_reference_rows_like_cpp(&condition_rows)
            .await;
        let addon_metadata = self
            .load_item_template_addon_loot_metadata_for_item_ids_like_cpp(
                condition_ids.iter().map(|id| id.source_entry),
            )
            .await;
        let generated = {
            let mut rng = rand::thread_rng();
            store
                .fill_loot_with_context_like_cpp(
                    loot_id,
                    LootStoreKind::Creature,
                    stores,
                    LootFillOptions {
                        loot_mode: LOOT_MODE_DEFAULT_LIKE_CPP,
                        rates_allowed: true,
                        referenced_amount_rate: rates.item_referenced_amount,
                        item_context: ItemContext::None as u8,
                    },
                    &mut rng,
                    |item_id| {
                        self.item_storage_template(item_id).map(|template| {
                            LootItemTemplateMetadata {
                                max_stack: template.max_stack_size.max(1),
                                has_multi_drop_flag: template.flags.contains(ItemFlags::MULTI_DROP),
                                has_follow_loot_rules_flag: false,
                            }
                        })
                    },
                    |item| self.item_drop_rate_like_cpp(item.item_id),
                    |context| {
                        self.represented_creature_loot_item_allowed_like_cpp(
                            context,
                            &condition_rows,
                            &condition_references,
                            &addon_metadata,
                        )
                    },
                    |item_id| {
                        let random_properties =
                            self.generate_loot_store_random_properties_like_cpp(item_id);
                        LootItemRandomProperties {
                            id: random_properties.id,
                            seed: random_properties.seed,
                        }
                    },
                )
                .ok()?
        };

        Some(
            generated
                .into_iter()
                .map(|item| {
                    let metadata = addon_metadata
                        .get(&item.item_id)
                        .copied()
                        .unwrap_or_default();
                    generated_creature_loot_item_to_entry_like_cpp(item, metadata)
                })
                .collect(),
        )
    }

    async fn load_represented_creature_loot_condition_rows_like_cpp(
        &self,
        condition_ids: &[LootConditionId],
    ) -> HashMap<LootConditionId, Vec<LootConditionRowLikeCpp>> {
        let mut rows_by_id = HashMap::new();
        for &condition_id in condition_ids {
            let rows = self
                .load_represented_creature_loot_condition_rows_for_id_like_cpp(condition_id)
                .await;
            if !rows.is_empty() {
                rows_by_id.insert(condition_id, rows);
            }
        }
        rows_by_id
    }

    async fn load_represented_creature_loot_condition_reference_rows_like_cpp(
        &self,
        condition_rows: &HashMap<LootConditionId, Vec<LootConditionRowLikeCpp>>,
    ) -> HashMap<u32, Vec<LootConditionRowLikeCpp>> {
        let mut references = HashMap::new();
        let mut pending = Vec::new();
        for rows in condition_rows.values() {
            pending.extend(loot_condition_reference_ids_like_cpp(rows));
        }

        while let Some(reference_id) = pending.pop() {
            if references.contains_key(&reference_id) {
                continue;
            }

            let rows = self
                .load_represented_creature_loot_condition_reference_rows_for_id_like_cpp(
                    reference_id,
                )
                .await;
            for nested_reference_id in loot_condition_reference_ids_like_cpp(&rows) {
                if !references.contains_key(&nested_reference_id) {
                    pending.push(nested_reference_id);
                }
            }
            references.insert(reference_id, rows);
        }

        references
    }

    async fn load_represented_creature_loot_condition_reference_rows_for_id_like_cpp(
        &self,
        reference_id: u32,
    ) -> Vec<LootConditionRowLikeCpp> {
        let Ok(reference_source_type) = i32::try_from(reference_id).map(|id| -id) else {
            return Vec::new();
        };

        self.load_represented_creature_loot_condition_rows_for_id_like_cpp(LootConditionId {
            source_type: reference_source_type,
            source_group: 0,
            source_entry: 0,
        })
        .await
    }

    async fn load_represented_creature_loot_condition_rows_for_id_like_cpp(
        &self,
        condition_id: LootConditionId,
    ) -> Vec<LootConditionRowLikeCpp> {
        let Some(world_db) = self.world_db() else {
            return Vec::new();
        };

        let mut stmt = world_db.prepare(WorldStatements::SEL_LOOT_TEMPLATE_CONDITION_ROWS);
        stmt.set_i32(0, condition_id.source_type);
        stmt.set_u32(1, condition_id.source_group);
        stmt.set_u32(2, condition_id.source_entry);

        let mut result = match world_db.query(&stmt).await {
            Ok(result) => result,
            Err(err) => {
                warn!(
                    source_type = condition_id.source_type,
                    source_group = condition_id.source_group,
                    source_entry = condition_id.source_entry,
                    error = %err,
                    "failed to load represented creature loot conditions"
                );
                return Vec::new();
            }
        };

        let mut conditions = Vec::new();
        if result.is_empty() {
            return conditions;
        }

        loop {
            let condition = LootConditionRowLikeCpp {
                else_group: result.try_read::<u32>(0).unwrap_or(0),
                condition_type_or_reference: result.try_read::<i32>(1).unwrap_or(0),
                condition_target: result.try_read::<u8>(2).unwrap_or(0),
                value1: result.try_read::<u32>(3).unwrap_or(0),
                value2: result.try_read::<u32>(4).unwrap_or(0),
                value3: result.try_read::<u32>(5).unwrap_or(0),
                string_value1: result.try_read::<String>(6).unwrap_or_default(),
                negative: result.try_read::<bool>(7).unwrap_or(false),
                script_name: result.try_read::<String>(8).unwrap_or_default(),
            };
            if !loot_condition_reference_self_references_like_cpp(
                condition_id.source_type,
                condition.condition_type_or_reference,
            ) {
                if let Some(condition) =
                    loot_condition_row_normalize_without_external_stores_like_cpp(condition)
                {
                    conditions.push(condition);
                }
            }

            if !result.next_row() {
                break;
            }
        }

        conditions
    }

    fn represented_creature_loot_item_allowed_like_cpp(
        &self,
        context: LootStoreItemContext,
        condition_rows: &HashMap<LootConditionId, Vec<LootConditionRowLikeCpp>>,
        condition_references: &HashMap<u32, Vec<LootConditionRowLikeCpp>>,
        addon_metadata: &HashMap<u32, ItemTemplateAddonLootMetadataLikeCpp>,
    ) -> bool {
        self.represented_creature_loot_item_allowed_for_player_like_cpp(
            context,
            self.player_guid().unwrap_or(ObjectGuid::EMPTY),
            condition_rows,
            condition_references,
            addon_metadata,
        )
    }

    fn represented_creature_loot_item_allowed_for_player_like_cpp(
        &self,
        context: LootStoreItemContext,
        player_guid: ObjectGuid,
        condition_rows: &HashMap<LootConditionId, Vec<LootConditionRowLikeCpp>>,
        condition_references: &HashMap<u32, Vec<LootConditionRowLikeCpp>>,
        addon_metadata: &HashMap<u32, ItemTemplateAddonLootMetadataLikeCpp>,
    ) -> bool {
        let Some(template) = self.item_storage_template(context.item.item_id) else {
            return false;
        };
        let Some(player_context) = self.represented_loot_player_context_like_cpp(player_guid)
        else {
            return false;
        };

        let flags2 = self.item_template_flags2_like_cpp(context.item.item_id);
        if represented_item_faction_flags_block_player_like_cpp(flags2, player_context.race) {
            return false;
        }

        let condition_id = LootConditionId {
            source_type: wow_loot::condition_source_type_for_loot_store_kind_like_cpp(
                context.store_kind,
            ),
            source_group: context.entry,
            source_entry: context.item.item_id,
        };
        if !loot_conditions_allow_player_with_references_like_cpp_representable(
            condition_rows
                .get(&condition_id)
                .map(Vec::as_slice)
                .unwrap_or(&[]),
            condition_references,
            |condition| {
                self.evaluate_creature_loot_condition_for_player_like_cpp_representable(
                    condition,
                    &player_context,
                )
            },
        ) {
            return false;
        }

        let addon = addon_metadata
            .get(&context.item.item_id)
            .copied()
            .unwrap_or_default();
        self.item_loot_quest_status_allows_for_player_like_cpp(
            context.item.item_id,
            context.item.needs_quest,
            addon,
            &player_context,
        ) && template.max_stack_size != 0
    }

    fn item_loot_quest_status_allows_for_player_like_cpp(
        &self,
        item_id: u32,
        needs_quest: bool,
        addon_metadata: ItemTemplateAddonLootMetadataLikeCpp,
        player_context: &RepresentedLootPlayerContext,
    ) -> bool {
        if player_context.is_current {
            return self.item_loot_quest_status_allows_like_cpp(
                item_id,
                needs_quest,
                addon_metadata,
            );
        }

        if addon_metadata.ignores_quest_status() {
            return true;
        }

        let start_quest_id = self.item_template_start_quest_id(item_id).unwrap_or(0);
        let has_non_none_start_quest_status = u32::try_from(start_quest_id)
            .ok()
            .is_some_and(|quest_id| quest_id != 0 && player_context.quest_status(quest_id) != 0);
        let has_quest_for_item =
            self.represented_has_quest_for_item_like_cpp(item_id, addon_metadata, player_context);

        (!needs_quest && !has_non_none_start_quest_status) || has_quest_for_item
    }

    fn represented_loot_player_context_like_cpp(
        &self,
        player_guid: ObjectGuid,
    ) -> Option<RepresentedLootPlayerContext> {
        if Some(player_guid) == self.player_guid() {
            return Some(RepresentedLootPlayerContext {
                race: self.player_race_like_cpp(),
                class: self.player_class_like_cpp(),
                gender: self.player_gender_like_cpp(),
                level: self.player_level_like_cpp(),
                known_spells: self.known_spells_like_cpp().to_vec(),
                active_quest_statuses: self
                    .player_quests
                    .iter()
                    .map(|(quest_id, status)| (*quest_id, status.status))
                    .collect(),
                active_quest_objective_counts: self
                    .player_quests
                    .iter()
                    .map(|(quest_id, status)| (*quest_id, status.objective_counts.clone()))
                    .collect(),
                rewarded_quests: self.rewarded_quests.clone(),
                inventory_item_counts: self.represented_inventory_item_counts_like_cpp(),
                is_current: true,
            });
        }

        let registry = self.player_registry()?;
        let player = registry.get(&player_guid)?;
        Some(RepresentedLootPlayerContext {
            race: player.race,
            class: player.class,
            gender: player.sex,
            level: player.level,
            known_spells: player.known_spells.clone(),
            active_quest_statuses: player.active_quest_statuses.clone(),
            active_quest_objective_counts: player.active_quest_objective_counts.clone(),
            rewarded_quests: player.rewarded_quests.clone(),
            inventory_item_counts: player.inventory_item_counts.clone(),
            is_current: false,
        })
    }

    fn item_template_flags2_like_cpp(&self, item_id: u32) -> Option<u32> {
        self.item_stats_store()
            .and_then(|store| store.sparse_template(item_id))
            .map(|template| template.flags[1])
    }

    fn item_loot_quest_status_allows_like_cpp(
        &self,
        item_id: u32,
        needs_quest: bool,
        addon_metadata: ItemTemplateAddonLootMetadataLikeCpp,
    ) -> bool {
        let start_quest_id = self.item_template_start_quest_id(item_id).unwrap_or(0);
        let has_non_none_start_quest_status =
            u32::try_from(start_quest_id).ok().is_some_and(|quest_id| {
                quest_id != 0
                    && (self.player_quests.contains_key(&quest_id)
                        || self.rewarded_quests.contains(&quest_id))
            });
        let has_quest_for_item = self.has_incomplete_quest_objective_for_item_like_cpp(item_id)
            || (addon_metadata.quest_log_item_id != 0
                && self.has_incomplete_quest_objective_for_object_id_like_cpp(
                    addon_metadata.quest_log_item_id,
                ))
            || self.has_incomplete_quest_item_drop_for_item_like_cpp(item_id);

        addon_metadata.ignores_quest_status()
            || ((!needs_quest && !has_non_none_start_quest_status) || has_quest_for_item)
    }

    fn has_incomplete_quest_objective_for_item_like_cpp(&self, item_id: u32) -> bool {
        let Ok(item_object_id) = i32::try_from(item_id) else {
            return false;
        };
        self.has_incomplete_quest_objective_for_object_id_like_cpp(item_object_id)
    }

    fn has_incomplete_quest_objective_for_object_id_like_cpp(&self, item_object_id: i32) -> bool {
        let Some(quest_store) = &self.quest_store else {
            return false;
        };

        self.player_quests.values().any(|status| {
            if status.status != 1 {
                return false;
            }

            let Some(quest) = quest_store.get(status.quest_id) else {
                return false;
            };

            quest
                .objectives
                .iter()
                .enumerate()
                .any(|(fallback_index, objective)| {
                    if objective.obj_type != 1 || objective.object_id != item_object_id {
                        return false;
                    }

                    let storage_index = usize::try_from(objective.storage_index)
                        .ok()
                        .unwrap_or(fallback_index);
                    let current = status
                        .objective_counts
                        .get(storage_index)
                        .copied()
                        .unwrap_or(0);
                    current < objective.amount.max(1)
                })
        })
    }

    fn has_incomplete_quest_item_drop_for_item_like_cpp(&self, item_id: u32) -> bool {
        let Some(quest_store) = &self.quest_store else {
            return false;
        };

        self.player_quests.values().any(|status| {
            if status.status != 1 {
                return false;
            }

            let Some(quest) = quest_store.get(status.quest_id) else {
                return false;
            };

            quest
                .item_drop
                .iter()
                .enumerate()
                .any(|(index, drop_item_id)| {
                    if *drop_item_id != item_id {
                        return false;
                    }

                    let Some(template) = self.item_storage_template(item_id) else {
                        return false;
                    };

                    let quantity = quest.item_drop_quantity[index];
                    let mut max_allowed_count = if quantity != 0 {
                        quantity
                    } else {
                        template.max_stack_size
                    };
                    if template.max_count > 0 {
                        max_allowed_count = max_allowed_count.min(template.max_count as u32);
                    }

                    self.direct_inventory_item_count_like_cpp(item_id) < max_allowed_count
                })
        })
    }

    fn represented_has_quest_for_item_like_cpp(
        &self,
        item_id: u32,
        addon_metadata: ItemTemplateAddonLootMetadataLikeCpp,
        player_context: &RepresentedLootPlayerContext,
    ) -> bool {
        if player_context.is_current {
            return self.has_incomplete_quest_objective_for_item_like_cpp(item_id)
                || (addon_metadata.quest_log_item_id != 0
                    && self.has_incomplete_quest_objective_for_object_id_like_cpp(
                        addon_metadata.quest_log_item_id,
                    ))
                || self.has_incomplete_quest_item_drop_for_item_like_cpp(item_id);
        }

        let Ok(item_object_id) = i32::try_from(item_id) else {
            return false;
        };
        self.remote_has_incomplete_quest_objective_for_object_id_like_cpp(
            item_object_id,
            player_context,
        ) || (addon_metadata.quest_log_item_id != 0
            && self.remote_has_incomplete_quest_objective_for_object_id_like_cpp(
                addon_metadata.quest_log_item_id,
                player_context,
            ))
            || self.remote_has_incomplete_quest_item_drop_for_item_like_cpp(item_id, player_context)
    }

    fn remote_has_incomplete_quest_objective_for_object_id_like_cpp(
        &self,
        item_object_id: i32,
        player_context: &RepresentedLootPlayerContext,
    ) -> bool {
        let Some(quest_store) = &self.quest_store else {
            return false;
        };

        player_context
            .active_quest_objective_counts
            .iter()
            .any(|(quest_id, objective_counts)| {
                if player_context.quest_status(*quest_id) != 1 {
                    return false;
                }

                let Some(quest) = quest_store.get(*quest_id) else {
                    return false;
                };

                quest
                    .objectives
                    .iter()
                    .enumerate()
                    .any(|(fallback_index, objective)| {
                        if objective.obj_type != 1 || objective.object_id != item_object_id {
                            return false;
                        }

                        let storage_index = usize::try_from(objective.storage_index)
                            .ok()
                            .unwrap_or(fallback_index);
                        let current = objective_counts.get(storage_index).copied().unwrap_or(0);
                        current < objective.amount.max(1)
                    })
            })
    }

    fn remote_has_incomplete_quest_item_drop_for_item_like_cpp(
        &self,
        item_id: u32,
        player_context: &RepresentedLootPlayerContext,
    ) -> bool {
        let Some(quest_store) = &self.quest_store else {
            return false;
        };

        player_context
            .active_quest_statuses
            .iter()
            .any(|(quest_id, status)| {
                if *status != 1 {
                    return false;
                }

                let Some(quest) = quest_store.get(*quest_id) else {
                    return false;
                };

                quest
                    .item_drop
                    .iter()
                    .enumerate()
                    .any(|(index, drop_item_id)| {
                        if *drop_item_id != item_id {
                            return false;
                        }

                        let Some(template) = self.item_storage_template(item_id) else {
                            return false;
                        };

                        let quantity = quest.item_drop_quantity[index];
                        let mut max_allowed_count = if quantity != 0 {
                            quantity
                        } else {
                            template.max_stack_size
                        };
                        if template.max_count > 0 {
                            max_allowed_count = max_allowed_count.min(template.max_count as u32);
                        }

                        player_context.inventory_item_count(item_id) < max_allowed_count
                    })
            })
    }

    fn direct_inventory_item_count_like_cpp(&self, item_id: u32) -> u32 {
        self.represented_inventory_item_counts_like_cpp()
            .get(&item_id)
            .copied()
            .unwrap_or(0)
    }

    fn evaluate_creature_loot_condition_for_player_like_cpp_representable(
        &self,
        condition: &LootConditionRowLikeCpp,
        player_context: &RepresentedLootPlayerContext,
    ) -> Option<bool> {
        match condition.condition_type_or_reference {
            0 => Some(true),
            2 => {
                if condition.value3 != 0 {
                    return None;
                }
                let item_count = if player_context.is_current {
                    self.direct_inventory_item_count_like_cpp(condition.value1)
                } else {
                    player_context.inventory_item_count(condition.value1)
                };
                Some(item_count >= condition.value2)
            }
            6 => Some(
                player_team_for_race_cpp_representable(player_context.race) == condition.value1,
            ),
            8 => Some(player_context.rewarded_quests.contains(&condition.value1)),
            9 => Some(player_context.quest_status(condition.value1) == 1),
            14 => Some(player_context.quest_status(condition.value1) == 0),
            15 => Some(
                player_class_mask_like_cpp(player_context.class)
                    .is_some_and(|mask| mask & condition.value1 != 0),
            ),
            16 => Some(
                player_race_mask_like_cpp(player_context.race)
                    .is_some_and(|mask| mask & condition.value1 != 0),
            ),
            20 => Some(u32::from(player_context.gender) == condition.value1),
            25 => i32::try_from(condition.value1)
                .ok()
                .map(|spell_id| player_context.known_spells.contains(&spell_id)),
            27 => condition_compare_values_like_cpp(
                condition.value2,
                u32::from(player_context.level),
                condition.value1,
            ),
            28 => Some(
                player_context.quest_status(condition.value1) == 2
                    && !player_context.rewarded_quests.contains(&condition.value1),
            ),
            47 => Some(
                player_quest_status_mask_like_cpp(
                    player_context
                        .active_quest_statuses
                        .get(&condition.value1)
                        .copied(),
                    player_context.rewarded_quests.contains(&condition.value1),
                ) & condition.value2
                    != 0,
            ),
            48 => {
                let progress = if player_context.is_current {
                    self.player_quest_objective_progress_like_cpp(condition.value1)
                } else {
                    self.remote_player_quest_objective_progress_like_cpp(
                        condition.value1,
                        player_context,
                    )
                };
                Some(progress == Some(condition.value3 as i32))
            }
            CONDITION_OBJECT_ENTRY_GUID_LIKE_CPP => {
                Some(condition.value1 == TYPEID_PLAYER_LIKE_CPP)
            }
            CONDITION_TYPE_MASK_LIKE_CPP => Some(condition.value1 & PLAYER_TYPE_MASK_LIKE_CPP != 0),
            _ => None,
        }
    }

    fn player_quest_objective_progress_like_cpp(&self, objective_id: u32) -> Option<i32> {
        let quest_store = self.quest_store.as_ref()?;

        for status in self.player_quests.values() {
            let Some(quest) = quest_store.get(status.quest_id) else {
                continue;
            };
            let Some((_, objective)) = quest
                .objectives
                .iter()
                .enumerate()
                .find(|(_, objective)| objective.id == objective_id)
            else {
                continue;
            };
            let objective_index = objective.storage_index.max(0) as usize;
            return Some(
                status
                    .objective_counts
                    .get(objective_index)
                    .copied()
                    .unwrap_or(0),
            );
        }

        None
    }

    fn remote_player_quest_objective_progress_like_cpp(
        &self,
        objective_id: u32,
        player_context: &RepresentedLootPlayerContext,
    ) -> Option<i32> {
        let quest_store = self.quest_store.as_ref()?;

        for (quest_id, objective_counts) in &player_context.active_quest_objective_counts {
            let Some(quest) = quest_store.get(*quest_id) else {
                continue;
            };
            let Some((_, objective)) = quest
                .objectives
                .iter()
                .enumerate()
                .find(|(_, objective)| objective.id == objective_id)
            else {
                continue;
            };
            let objective_index = objective.storage_index.max(0) as usize;
            return Some(objective_counts.get(objective_index).copied().unwrap_or(0));
        }

        None
    }

    async fn load_item_template_addon_loot_metadata_for_item_ids_like_cpp<I>(
        &self,
        item_ids: I,
    ) -> HashMap<u32, ItemTemplateAddonLootMetadataLikeCpp>
    where
        I: IntoIterator<Item = u32>,
    {
        let mut item_ids: Vec<u32> = item_ids.into_iter().collect();
        item_ids.sort_unstable();
        item_ids.dedup();

        let mut metadata = HashMap::with_capacity(item_ids.len());
        for item_id in item_ids {
            metadata.insert(
                item_id,
                self.load_creature_item_template_addon_loot_metadata_like_cpp(item_id)
                    .await,
            );
        }
        metadata
    }

    async fn load_creature_item_template_addon_loot_metadata_like_cpp(
        &self,
        item_id: u32,
    ) -> ItemTemplateAddonLootMetadataLikeCpp {
        let Some(world_db) = self.world_db() else {
            return ItemTemplateAddonLootMetadataLikeCpp::default();
        };

        let mut stmt = world_db.prepare(WorldStatements::SEL_ITEM_TEMPLATE_ADDON_LOOT_METADATA);
        stmt.set_u32(0, item_id);

        match world_db.query(&stmt).await {
            Ok(result) if !result.is_empty() => ItemTemplateAddonLootMetadataLikeCpp {
                flags_cu: result.try_read::<u32>(0).unwrap_or(0),
                quest_log_item_id: result.try_read::<i32>(1).unwrap_or(0),
            },
            Ok(_) => ItemTemplateAddonLootMetadataLikeCpp::default(),
            Err(err) => {
                warn!(
                    item_id,
                    error = %err,
                    "failed to load item_template_addon loot metadata for creature loot"
                );
                ItemTemplateAddonLootMetadataLikeCpp::default()
            }
        }
    }

    fn active_loot_owner_for_loot_object_like_cpp(
        &self,
        loot_object: ObjectGuid,
    ) -> Option<ObjectGuid> {
        let active_owners: Vec<ObjectGuid> = if self.active_loot_view_owners.is_empty() {
            vec![self.active_loot_guid]
        } else {
            self.active_loot_view_owners.iter().copied().collect()
        };

        active_owners.into_iter().find(|owner_guid| {
            !owner_guid.is_empty()
                && self
                    .loot_table
                    .get(owner_guid)
                    .is_some_and(|loot| loot.loot_guid == loot_object)
        })
    }

    fn canonical_map_object_position_for_loot_like_cpp(
        &self,
        guid: ObjectGuid,
        allowed: &[AccessorObjectKind],
    ) -> Option<wow_core::Position> {
        let manager = self.canonical_map_manager.as_ref()?;
        let manager = manager.lock().ok()?;
        let map = manager
            .find_map(u32::from(self.player_map_id_like_cpp()), 0)?
            .map();
        map.map_object_by_kind(guid, allowed)
            .map(|object| object.position())
    }

    fn canonical_gameobject_owner_for_loot_like_cpp(&self, guid: ObjectGuid) -> Option<ObjectGuid> {
        let manager = self.canonical_map_manager.as_ref()?;
        let manager = manager.lock().ok()?;
        let map = manager
            .find_map(u32::from(self.player_map_id_like_cpp()), 0)?
            .map();
        let owner_guid = map.get_typed_game_object(guid)?.owner_guid();
        (!owner_guid.is_empty()).then_some(owner_guid)
    }

    fn represented_creature_loot_state_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> Option<RepresentedCreatureLootStateLikeCpp> {
        self.mutate_world_creature(guid, |creature| RepresentedCreatureLootStateLikeCpp {
            is_alive: creature.is_alive(),
            position: creature.position(),
            level: creature.level(),
            entry: creature.entry(),
            loot_id: creature.loot_id(),
            gold_min: creature.gold_min(),
            gold_max: creature.gold_max(),
            dungeon_encounter_id: creature.dungeon_encounter_id(),
            tappers: creature.creature.tap_list().to_vec(),
        })
    }

    fn represented_creature_position_for_loot_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) -> Option<wow_core::Position> {
        if let Some(position) = self
            .canonical_map_object_position_for_loot_like_cpp(guid, &[AccessorObjectKind::Creature])
        {
            return Some(position);
        }

        self.represented_creature_loot_state_like_cpp(guid)
            .map(|creature| creature.position)
    }

    fn represented_gameobject_loot_state_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<RepresentedGameObjectLootStateLikeCpp> {
        if !guid.is_game_object() {
            return None;
        }

        let canonical_position = self.canonical_map_object_position_for_loot_like_cpp(
            guid,
            &[
                AccessorObjectKind::GameObject,
                AccessorObjectKind::Transport,
            ],
        );
        let canonical_owner = self.canonical_gameobject_owner_for_loot_like_cpp(guid);
        let represented_state = self.represented_gameobject_use_states.get(&guid);
        if canonical_position.is_none()
            && represented_state.and_then(|state| state.position).is_none()
            && !self.client_visible_guids_like_cpp.contains(&guid)
        {
            return None;
        }

        Some(RepresentedGameObjectLootStateLikeCpp {
            position: canonical_position
                .or_else(|| represented_state.and_then(|state| state.position)),
            display_id: represented_state.and_then(|state| state.display_id),
            scale: represented_state.map(|state| state.scale).unwrap_or(1.0),
            rotation: represented_state
                .map(|state| state.rotation)
                .unwrap_or([0.0, 0.0, 0.0, 1.0]),
            go_type: represented_state.and_then(|state| state.go_type),
            interact_radius_override: represented_state
                .and_then(|state| state.interact_radius_override),
            lock_id: represented_state.and_then(|state| state.lock_id),
            owner_guid: canonical_owner
                .or_else(|| represented_state.and_then(|state| state.owner_guid)),
        })
    }

    fn represented_gameobject_exists_for_loot_like_cpp(&self, guid: ObjectGuid) -> bool {
        self.represented_gameobject_loot_state_like_cpp(guid)
            .is_some()
    }

    fn represented_spell_max_range_like_cpp(&self, spell_id: i32) -> Option<f32> {
        let spell_store = self.spell_store()?;
        let spell_misc_store = self.spell_misc_store()?;
        let spell_range_store = self.spell_range_store()?;
        spell_store.get(spell_id)?;
        let spell_id = u32::try_from(spell_id).ok()?;
        let range_index = spell_misc_store.get(spell_id)?.range_index;
        let range = spell_range_store.get(u32::from(range_index))?;
        Some(range.range_max[1].max(range.range_max[0]))
    }

    fn represented_gameobject_spell_lock_range_like_cpp(
        &self,
        lock_id: Option<u32>,
    ) -> Option<f32> {
        let lock_id = lock_id?;
        let lock = self.lock_store()?.get(lock_id)?;
        for i in 0..wow_data::lock::MAX_LOCK_CASE {
            let lock_type = lock.lock_type[i];
            if lock_type == 0 {
                continue;
            }

            if lock_type == LOCK_KEY_SPELL_LIKE_CPP {
                if let Some(range) = self.represented_spell_max_range_like_cpp(lock.index[i]) {
                    return Some(range);
                }
            }

            if lock_type != LOCK_KEY_SKILL_LIKE_CPP {
                break;
            }

            for spell_id in self.known_spells_like_cpp() {
                let Some(spell) = self.spell_store().and_then(|store| store.get(*spell_id)) else {
                    continue;
                };
                let can_open_lock = spell.effects().iter().any(|effect| {
                    effect.effect == SPELL_EFFECT_OPEN_LOCK_LIKE_CPP
                        && effect.effect_misc_value_1 == lock.index[i]
                        && effect.effect_base_points >= i32::from(lock.skill[i])
                });
                if can_open_lock {
                    if let Some(range) = self.represented_spell_max_range_like_cpp(*spell_id) {
                        return Some(range);
                    }
                }
            }
        }

        None
    }

    fn represented_gameobject_can_autostore_loot_item_like_cpp(
        &self,
        guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> bool {
        let Some(state) = self.represented_gameobject_loot_state_like_cpp(guid) else {
            return false;
        };

        // C++ ref: LootHandler.cpp HandleAutostoreLootItemOpcode skips distance
        // for owned GameObjects and GAMEOBJECT_TYPE_FISHINGHOLE. DB spawns do
        // not carry CreatedBy; apply the owner exception only when runtime GO
        // state explicitly recorded GetOwnerGUID.
        if state.owner_guid == Some(player_guid)
            || state.go_type == Some(GAMEOBJECT_TYPE_FISHING_HOLE as u8)
        {
            return true;
        }

        match (self.player_position_like_cpp(), state.position) {
            (Some(player), Some(position)) => {
                let radius = represented_gameobject_interaction_distance_like_cpp(
                    state.go_type,
                    state.interact_radius_override,
                );
                let radius = self
                    .represented_gameobject_spell_lock_range_like_cpp(state.lock_id)
                    .unwrap_or(radius);
                if let Some(display_info) = self.gameobject_display_info_store().and_then(|store| {
                    state
                        .display_id
                        .and_then(|display_id| store.get(display_id))
                }) {
                    represented_gameobject_display_box_contains_like_cpp(
                        position,
                        player,
                        display_info,
                        state.scale,
                        state.rotation,
                        radius,
                    )
                } else {
                    player.is_within_dist(&position, radius)
                }
            }
            _ => true,
        }
    }

    fn apply_represented_gameobject_loot_release_like_cpp(
        &mut self,
        guid: ObjectGuid,
        player_guid: ObjectGuid,
        fully_looted: bool,
    ) {
        let go_type = self
            .represented_gameobject_use_states
            .get(&guid)
            .and_then(|state| state.go_type)
            .map(u32::from);
        let represented_chest_restock_time_secs = self
            .represented_gameobject_use_states
            .get(&guid)
            .and_then(|state| state.chest_restock_time_secs)
            .unwrap_or_default();
        let represented_personal_loot_uses_after_release = self
            .represented_gameobject_use_states
            .get(&guid)
            .map(|state| state.personal_loot_uses.saturating_add(1))
            .unwrap_or(1);
        // C++ `FishingHole.MaxOpens` is still template evidence from the represented
        // GO value; the use counter source-of-truth is canonical `GameObject::use_times`
        // when the canonical GameObject can be mutated.
        let represented_fishing_hole_max_opens = self
            .represented_gameobject_use_states
            .get(&guid)
            .and_then(|state| state.fishing_hole_max_opens);
        let canonical_fishing_hole_use_count_after_release = (go_type
            == Some(GAMEOBJECT_TYPE_FISHING_HOLE))
        .then(|| self.add_use_and_get_canonical_gameobject_use_count_like_cpp(guid))
        .flatten();

        let canonical_loot_state_request = match go_type {
            Some(GAMEOBJECT_TYPE_FISHING_NODE) => Some((LootState::JustDeactivated, None, false)),
            Some(GAMEOBJECT_TYPE_FISHING_HOLE) => {
                let use_count_after_release = canonical_fishing_hole_use_count_after_release
                    .unwrap_or(represented_personal_loot_uses_after_release);
                let state = if represented_fishing_hole_max_opens
                    .is_some_and(|max_opens| use_count_after_release >= max_opens)
                {
                    LootState::JustDeactivated
                } else {
                    LootState::Ready
                };
                Some((state, None, false))
            }
            Some(GAMEOBJECT_TYPE_GATHERING_NODE) if fully_looted => None,
            _ if fully_looted => Some((LootState::JustDeactivated, None, false)),
            _ => Some((LootState::Activated, Some(player_guid), true)),
        };
        let canonical_loot_state_updated = canonical_loot_state_request.is_some_and(
            |(loot_state, unit_guid, shared_loot_is_changed_like_cpp)| {
                self.set_canonical_gameobject_loot_state_like_cpp(
                    guid,
                    loot_state,
                    unit_guid,
                    represented_chest_restock_time_secs,
                    shared_loot_is_changed_like_cpp,
                )
                .is_some_and(|outcome| {
                    outcome.status == wow_map::map::GameObjectSetLootStateStatusLikeCpp::Updated
                })
            },
        );

        let state = self
            .represented_gameobject_use_states
            .entry(guid)
            .or_default();
        if canonical_loot_state_updated {
            if let Some((loot_state, unit_guid, _)) = canonical_loot_state_request {
                state.loot_state = Some(loot_state);
                state.loot_state_unit_guid = unit_guid.unwrap_or(ObjectGuid::EMPTY);
                if loot_state == LootState::Activated
                    && go_type == Some(GAMEOBJECT_TYPE_CHEST)
                    && state.chest_consumable == Some(false)
                    && state.chest_restock_until.is_none()
                    && state
                        .chest_restock_time_secs
                        .is_some_and(|restock_time| restock_time != 0)
                {
                    let restock_secs = state.chest_restock_time_secs.unwrap_or_default();
                    state.chest_restock_until =
                        Some(Instant::now() + Duration::from_secs(u64::from(restock_secs)));
                }
            }
        } else {
            match go_type {
                Some(GAMEOBJECT_TYPE_FISHING_NODE) => {
                    state.loot_state = Some(LootState::JustDeactivated);
                    state.loot_state_unit_guid = ObjectGuid::EMPTY;
                }
                Some(GAMEOBJECT_TYPE_FISHING_HOLE) => {
                    state.personal_loot_uses = state.personal_loot_uses.saturating_add(1);
                    state.loot_state = if state
                        .fishing_hole_max_opens
                        .is_some_and(|max_opens| state.personal_loot_uses >= max_opens)
                    {
                        Some(LootState::JustDeactivated)
                    } else {
                        Some(LootState::Ready)
                    };
                    state.loot_state_unit_guid = ObjectGuid::EMPTY;
                }
                Some(GAMEOBJECT_TYPE_GATHERING_NODE) if fully_looted => {}
                Some(GAMEOBJECT_TYPE_CHEST)
                    if fully_looted
                        && state.chest_consumable == Some(false)
                        && state
                            .chest_personal_loot_id
                            .is_none_or(|loot_id| loot_id == 0)
                        && state
                            .chest_restock_time_secs
                            .is_some_and(|restock_time| restock_time != 0) =>
                {
                    let restock_secs = state.chest_restock_time_secs.unwrap_or_default();
                    state.loot_state = Some(LootState::NotReady);
                    state.loot_state_unit_guid = ObjectGuid::EMPTY;
                    state.chest_restock_until =
                        Some(Instant::now() + Duration::from_secs(u64::from(restock_secs)));
                }
                _ if fully_looted => {
                    state.loot_state = Some(LootState::JustDeactivated);
                    state.loot_state_unit_guid = ObjectGuid::EMPTY;
                }
                _ => {
                    state.loot_state = Some(LootState::Activated);
                    state.loot_state_unit_guid = player_guid;
                    if go_type == Some(GAMEOBJECT_TYPE_CHEST)
                        && state.chest_consumable == Some(false)
                        && state.chest_restock_until.is_none()
                        && state
                            .chest_restock_time_secs
                            .is_some_and(|restock_time| restock_time != 0)
                    {
                        let restock_secs = state.chest_restock_time_secs.unwrap_or_default();
                        state.chest_restock_until =
                            Some(Instant::now() + Duration::from_secs(u64::from(restock_secs)));
                    }
                }
            }
        }
        if canonical_loot_state_updated && go_type == Some(GAMEOBJECT_TYPE_FISHING_HOLE) {
            state.personal_loot_uses = canonical_fishing_hole_use_count_after_release
                .unwrap_or(represented_personal_loot_uses_after_release);
        }
        if go_type == Some(GAMEOBJECT_TYPE_GATHERING_NODE) {
            state.go_state = Some(GoState::Active);
        }
        if go_type == Some(GAMEOBJECT_TYPE_CHEST)
            && state.chest_consumable == Some(false)
            && state
                .chest_personal_loot_id
                .is_some_and(|loot_id| loot_id != 0)
        {
            let delay_secs = state
                .chest_restock_time_secs
                .filter(|restock_time| *restock_time != 0)
                .unwrap_or(wow_entities::DEFAULT_GAMEOBJECT_RESPAWN_DELAY_SECS);
            state.per_player_despawn_secs = Some(delay_secs);
            state.per_player_despawn_until =
                Some(Instant::now() + Duration::from_secs(u64::from(delay_secs)));
            state.per_player_state_player_guid = Some(player_guid);
        }
    }

    fn hide_represented_gameobject_for_player_after_loot_release_like_cpp(
        &mut self,
        guid: ObjectGuid,
    ) {
        let Some(map_id) = self
            .represented_gameobject_use_states
            .get(&guid)
            .and_then(|state| state.per_player_despawn_until.map(|_| state.map_id))
            .flatten()
        else {
            return;
        };
        if !self.client_visible_guids_like_cpp.remove(&guid) {
            return;
        }
        self.send_packet(&UpdateObject::out_of_range_objects(vec![guid], map_id));
    }

    fn send_gathering_node_loot_release_dynamic_flags_update_like_cpp(&self, guid: ObjectGuid) {
        if !self.client_visible_guids_like_cpp.contains(&guid) {
            return;
        }
        let Some(access) = self.canonical_gameobject_access_like_cpp(guid) else {
            return;
        };
        let Some(state) = self.represented_gameobject_use_states.get(&guid) else {
            return;
        };
        if state.go_type.map(u32::from) != Some(GAMEOBJECT_TYPE_GATHERING_NODE) {
            return;
        }
        let dynamic_flags =
            self.represented_gameobject_dynamic_flags_for_player_like_cpp(access.entry, state);
        let packet_update = wow_packet::packets::update::GameObjectDataValuesUpdate {
            changed_object_type_mask: 1 << wow_entities::TYPEID_OBJECT,
            object_data: Some(wow_packet::packets::update::ObjectDataValuesUpdate {
                changed_object_type_mask: 1 << wow_entities::TYPEID_OBJECT,
                object_data_mask: 0x05,
                entry_id: 0,
                dynamic_flags,
                scale: 0.0,
            }),
            game_object_data_mask: 0,
            state_world_effect_ids: Vec::new(),
            enable_doodad_sets: Vec::new(),
            enable_doodad_sets_update_mask: None,
            world_effects: Vec::new(),
            world_effects_update_mask: None,
            display_id: 0,
            spell_visual_id: 0,
            state_spell_visual_id: 0,
            spawn_tracking_state_anim_id: 0,
            spawn_tracking_state_anim_kit_id: 0,
            created_by: ObjectGuid::EMPTY,
            guild_guid: ObjectGuid::EMPTY,
            flags: 0,
            parent_rotation: [0.0; 4],
            faction_template: 0,
            level: 0,
            state: 0,
            type_id: 0,
            percent_health: 0,
            art_kit: 0,
            custom_param: 0,
        };
        let update = UpdateObject::game_object_values_update(
            guid,
            self.player_map_id_like_cpp(),
            packet_update,
        );
        self.send_packet(&update);
    }

    fn send_loot_error_like_cpp(&self, loot_obj: ObjectGuid, owner: ObjectGuid, error: u8) {
        self.send_packet(&LootResponse {
            owner,
            loot_obj,
            failure_reason: error,
            acquire_reason: 0,
            loot_method: 0,
            threshold: 0,
            coins: 0,
            items: vec![],
            currencies: vec![],
            acquired: false,
            ae_looting: false,
        });
    }

    pub(crate) async fn do_loot_release_all_like_cpp(&mut self, player_guid: ObjectGuid) {
        let mut active_owners: Vec<ObjectGuid> =
            self.active_loot_view_owners.iter().copied().collect();
        if active_owners.is_empty() && !self.active_loot_guid.is_empty() {
            active_owners.push(self.active_loot_guid);
        }
        active_owners.sort_by_key(|guid| (guid.high_value(), guid.low_value()));

        for owner_guid in active_owners {
            self.do_loot_release_owner_like_cpp(owner_guid, player_guid)
                .await;
        }
    }

    pub(crate) fn close_active_loot_windows_like_cpp(&mut self, player_guid: ObjectGuid) {
        let mut active_owners: Vec<ObjectGuid> =
            self.active_loot_view_owners.iter().copied().collect();
        if active_owners.is_empty() && !self.active_loot_guid.is_empty() {
            active_owners.push(self.active_loot_guid);
        }
        active_owners.sort_by_key(|guid| (guid.high_value(), guid.low_value()));

        for owner_guid in active_owners {
            if let Some(loot) = self.loot_table.get_mut(&owner_guid) {
                loot.players_looting.retain(|looter| *looter != player_guid);
            }
            self.send_packet(&SLootRelease {
                loot_obj: owner_guid,
                owner: player_guid,
            });
            self.clear_active_loot_guid_if(owner_guid);
        }
    }

    async fn do_loot_release_owner_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> bool {
        if !self.active_loot_view_owners.contains(&owner_guid)
            && !self.is_active_loot_guid(owner_guid)
        {
            return false;
        }

        // Check if loot is fully taken (all items picked up).
        // Coins are auto-consumed when the loot window opens (sent in LootResponse),
        // so we only check items here.
        // C# ref: `loot.IsLooted()` → no more non-taken items.
        let Some(loot) = self.loot_table.get(&owner_guid) else {
            return false;
        };
        let represented_fully_looted = loot_is_looted_like_cpp(loot);
        let fully_looted = if owner_guid.is_game_object() {
            self.canonical_gameobject_fully_looted_after_represented_sync_like_cpp(
                owner_guid,
                player_guid,
                represented_fully_looted,
            )
        } else {
            represented_fully_looted
        };

        if let Some(loot) = self.loot_table.get_mut(&owner_guid) {
            loot.players_looting.retain(|looter| *looter != player_guid);
        }

        // Acknowledge the release to the client.
        let release = SLootRelease {
            loot_obj: owner_guid,
            owner: player_guid,
        };
        self.send_packet(&release);

        if owner_guid.is_game_object() {
            self.clear_active_loot_guid_if(owner_guid);
            if !self
                .represented_gameobject_can_autostore_loot_item_like_cpp(owner_guid, player_guid)
            {
                return true;
            }
            self.apply_represented_gameobject_loot_release_like_cpp(
                owner_guid,
                player_guid,
                fully_looted,
            );
            let _ = self.queue_chest_gameobject_state_refresh_for_same_map_like_cpp(owner_guid);
            if !fully_looted {
                return true;
            }

            self.hide_represented_gameobject_for_player_after_loot_release_like_cpp(owner_guid);
            let go_type = self
                .represented_gameobject_use_states
                .get(&owner_guid)
                .and_then(|state| state.go_type)
                .map(u32::from);
            if go_type == Some(GAMEOBJECT_TYPE_GATHERING_NODE) {
                self.send_gathering_node_loot_release_dynamic_flags_update_like_cpp(owner_guid);
            }
            self.loot_table.remove(&owner_guid);
            return true;
        }

        if owner_guid.is_item() && !fully_looted {
            self.clear_active_loot_guid_if(owner_guid);
            return true;
        }

        self.clear_active_loot_guid_if(owner_guid);

        if !fully_looted {
            let round_robin_released = self.loot_table.get_mut(&owner_guid).is_some_and(|loot| {
                if loot.round_robin_player == player_guid {
                    loot.round_robin_player = ObjectGuid::EMPTY;
                    true
                } else {
                    false
                }
            });
            if round_robin_released {
                self.represented_notify_loot_list_like_cpp(owner_guid);
            }
            return true;
        }

        // Remove loot entry from memory once the represented loot is consumed.
        self.loot_table.remove(&owner_guid);

        if owner_guid.is_item() && fully_looted {
            self.destroy_fully_looted_direct_item(owner_guid).await;
            return true;
        }

        // Start corpse despawn timer if fully looted.
        // C# uses `RateCorpseDecayLooted` config × `m_corpseDelay` (default 60s).
        // We use a simple 30s fixed decay.
        let marked = self
            .mutate_world_creature(owner_guid, |creature| {
                if !creature.is_alive() && creature.corpse_despawn_at().is_none() {
                    const CORPSE_DECAY_SECS: u64 = 30;
                    creature.set_corpse_despawn_at(Some(
                        Instant::now() + Duration::from_secs(CORPSE_DECAY_SECS),
                    ));
                    Some((creature.entry(), CORPSE_DECAY_SECS))
                } else {
                    None
                }
            })
            .flatten();

        if let Some((entry, corpse_decay_secs)) = marked {
            info!(
                "Creature {:?} (entry {}) fully looted — despawning in {}s",
                owner_guid, entry, corpse_decay_secs
            );
        }

        true
    }

    async fn delete_stored_item_money_like_cpp(&self, item_guid: ObjectGuid) {
        let Some(char_db) = self.char_db().map(Arc::clone) else {
            return;
        };

        let mut stmt = char_db.prepare(CharStatements::DEL_ITEMCONTAINER_MONEY);
        stmt.set_u64(0, item_guid.counter() as u64);
        if let Err(e) = char_db.execute(&stmt).await {
            warn!(
                item_guid = item_guid.counter(),
                error = %e,
                "failed to delete stored item loot money"
            );
        }
    }

    async fn delete_stored_item_loot_item_like_cpp(
        &self,
        item_guid: ObjectGuid,
        item_id: u32,
        count: u32,
        loot_list_id: u8,
    ) {
        let Some(char_db) = self.char_db().map(Arc::clone) else {
            return;
        };

        let mut stmt = char_db.prepare(CharStatements::DEL_ITEMCONTAINER_ITEM);
        stmt.set_u64(0, item_guid.counter() as u64);
        stmt.set_u32(1, item_id);
        stmt.set_u32(2, count);
        stmt.set_u32(3, u32::from(loot_list_id));
        if let Err(e) = char_db.execute(&stmt).await {
            warn!(
                item_guid = item_guid.counter(),
                item_id,
                count,
                loot_list_id,
                error = %e,
                "failed to delete stored item loot row"
            );
        }
    }

    async fn store_direct_loot_item_like_cpp(
        &mut self,
        loot_entry: &LootEntry,
        dungeon_encounter_id: u32,
    ) -> bool {
        let item_id = loot_entry.item_id;
        let count = loot_entry.quantity;
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        let Some(char_db) = self.char_db().map(Arc::clone) else {
            return false;
        };
        let Some((store_result, mut store_dest, _)) =
            self.plan_store_new_direct_inventory_item(item_id, count)
        else {
            self.send_equip_error(InventoryResult::ItemNotFound, None, None, 0, 0);
            return false;
        };
        if store_result != InventoryResult::Ok {
            self.send_equip_error(store_result, None, None, 0, 0);
            return false;
        }

        let store_random_properties = self.generate_loot_store_random_properties_like_cpp(item_id);

        if store_dest.iter().any(|dest| {
            let bag = (dest.pos >> 8) as u8;
            let slot = (dest.pos & 0x00FF) as u8;
            bag == u8::from(INVENTORY_SLOT_BAG_0)
                && self
                    .inventory_items_like_cpp()
                    .get(&slot)
                    .is_some_and(|existing| {
                        self.inventory_item_objects_like_cpp()
                            .get(&existing.guid)
                            .is_some_and(|item| {
                                !loot_store_data_can_stack_with_item(
                                    loot_entry,
                                    store_random_properties,
                                    item,
                                )
                            })
                    })
        }) {
            let Some(compatible_dest) = self.plan_direct_loot_item_preserving_cpp_store_metadata(
                loot_entry,
                store_random_properties,
            ) else {
                self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                return false;
            };
            store_dest = compatible_dest;
        }

        let mut planned_existing_counts = Vec::<(u8, ObjectGuid, u64, u32, u32)>::new();
        let mut planned_new_stacks = Vec::<PlannedLootNewStack>::new();

        for dest in store_dest {
            let bag = (dest.pos >> 8) as u8;
            let slot = (dest.pos & 0x00FF) as u8;
            if bag != u8::from(INVENTORY_SLOT_BAG_0) {
                self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                return false;
            }

            let max_stack = self
                .item_storage_template(item_id)
                .map(|template| template.max_stack_size)
                .unwrap_or(1)
                .max(1);

            if let Some(existing) = self.inventory_items_like_cpp().get(&slot) {
                let Some(existing_object) =
                    self.inventory_item_objects_like_cpp().get(&existing.guid)
                else {
                    self.send_equip_error(InventoryResult::ItemNotFound, None, None, 0, 0);
                    return false;
                };
                let base_count = planned_existing_counts
                    .iter()
                    .find(|(planned_slot, ..)| *planned_slot == slot)
                    .map(|(_, _, _, new_count, _)| *new_count)
                    .unwrap_or_else(|| existing_object.count());
                let new_count = base_count.saturating_add(dest.count);
                if existing.entry_id != item_id
                    || new_count > max_stack
                    || !loot_store_data_can_stack_with_item(
                        loot_entry,
                        store_random_properties,
                        existing_object,
                    )
                {
                    self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                    return false;
                }
                if let Some(existing_plan) = planned_existing_counts
                    .iter_mut()
                    .find(|(planned_slot, ..)| *planned_slot == slot)
                {
                    existing_plan.3 = new_count;
                    existing_plan.4 = existing_plan.4.saturating_add(dest.count);
                } else {
                    planned_existing_counts.push((
                        slot,
                        existing.guid,
                        existing.db_guid,
                        new_count,
                        dest.count,
                    ));
                }
                continue;
            }

            if let Some(stack) = planned_new_stacks
                .iter_mut()
                .find(|stack| stack.slot == slot)
            {
                if stack.entry_id == item_id
                    && stack.random_properties_id == store_random_properties.id
                    && stack.random_properties_seed == store_random_properties.seed
                    && stack.item_context == loot_entry.item_context
                    && stack.count.saturating_add(dest.count) <= max_stack
                {
                    stack.count = stack.count.saturating_add(dest.count);
                    continue;
                }
                self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                return false;
            }

            planned_new_stacks.push(PlannedLootNewStack {
                slot,
                entry_id: item_id,
                count: dest.count,
                max_durability: self.item_template_max_durability(item_id),
                random_properties_id: store_random_properties.id,
                random_properties_seed: store_random_properties.seed,
                item_context: loot_entry.item_context,
            });
        }

        let mut tx = SqlTransaction::new();
        for &(_, _, db_guid, new_count, _) in &planned_existing_counts {
            let mut upd_count = char_db.prepare(CharStatements::UPD_ITEM_INSTANCE_COUNT);
            upd_count.set_u32(0, new_count);
            upd_count.set_u64(1, db_guid);
            tx.append(upd_count);
        }

        let realm_id = self.realm_id();
        let mut created_new_stacks = Vec::new();
        if !planned_new_stacks.is_empty() {
            let max_guid_stmt = char_db.prepare(CharStatements::SEL_MAX_ITEM_GUID);
            let mut next_item_guid = match char_db.query(&max_guid_stmt).await {
                Ok(r) => r.try_read::<u64>(0).unwrap_or(0) + 1,
                Err(_) => 1,
            };

            for stack in &planned_new_stacks {
                let db_guid = next_item_guid;
                next_item_guid += 1;
                let item_guid = ObjectGuid::create_item(realm_id, db_guid as i64);

                let mut ins_item =
                    char_db.prepare(CharStatements::INS_ITEM_INSTANCE_WITH_RANDOM_CONTEXT);
                ins_item.set_u64(0, db_guid);
                ins_item.set_u32(1, stack.entry_id);
                ins_item.set_u64(2, player_guid.counter() as u64);
                ins_item.set_u32(3, stack.count);
                ins_item.set_u32(4, stack.max_durability);
                ins_item.set_i32(5, stack.random_properties_id);
                ins_item.set_i32(6, stack.random_properties_seed);
                ins_item.set_u8(7, stack.item_context);
                tx.append(ins_item);

                let mut ins_inv = char_db.prepare(CharStatements::INS_CHAR_INVENTORY);
                ins_inv.set_u64(0, player_guid.counter() as u64);
                ins_inv.set_u8(1, stack.slot);
                ins_inv.set_u64(2, db_guid);
                tx.append(ins_inv);

                created_new_stacks.push((stack.clone(), db_guid, item_guid));
            }
        }

        if let Err(e) = char_db.commit_transaction(tx).await {
            warn!("LootItem: store transaction failed: {e}");
            self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
            return false;
        }

        for &(_, item_guid, _, new_count, _) in &planned_existing_counts {
            self.update_inventory_item_object_like_cpp(item_guid, |item| {
                item.set_count(new_count);
            });
        }

        let mut collection_updates = Vec::new();
        for (stack, db_guid, item_guid) in &created_new_stacks {
            self.insert_inventory_item_like_cpp(
                stack.slot,
                InventoryItem {
                    guid: *item_guid,
                    entry_id: stack.entry_id,
                    db_guid: *db_guid,
                    inventory_type: self.item_template_inventory_type(stack.entry_id),
                },
            );
            let mut item_object = self.make_inventory_item_object(
                *item_guid,
                stack.entry_id,
                player_guid,
                stack.count,
                stack.max_durability,
                loot_item_context(stack.item_context),
                stack.slot,
            );
            if stack.random_properties_id != 0 {
                item_object.set_random_properties_id(stack.random_properties_id);
            }
            if stack.random_properties_seed != 0 {
                item_object.set_property_seed(stack.random_properties_seed);
            }
            collection_updates.extend(self.on_item_added_to_collection_like_cpp(&item_object));
            self.insert_inventory_item_object(item_object);
        }
        self.sync_object_accessor_player();

        let map_id = self.player_map_id_like_cpp();
        if !created_new_stacks.is_empty() {
            let item_creates = created_new_stacks
                .iter()
                .map(|(stack, _, item_guid)| ItemCreateData {
                    item_guid: *item_guid,
                    entry_id: stack.entry_id as i32,
                    owner_guid: player_guid,
                    contained_in: player_guid,
                    stack_count: stack.count,
                    dynamic_flags: 0,
                    durability: stack.max_durability,
                    max_durability: stack.max_durability,
                    random_properties_seed: stack.random_properties_seed,
                    random_properties_id: stack.random_properties_id,
                    context: stack.item_context,
                })
                .collect();
            self.send_packet(&UpdateObject::create_items(item_creates, map_id));
        }

        for &(slot, item_guid, _, new_count, added_count) in &planned_existing_counts {
            self.send_packet(&UpdateObject::item_stack_count_update(
                item_guid, map_id, new_count,
            ));
            self.send_loot_item_push_result(
                player_guid,
                item_guid,
                loot_entry,
                store_random_properties.id,
                store_random_properties.seed,
                slot,
                added_count,
                new_count,
                false,
                dungeon_encounter_id,
            );
        }

        for (stack, _, item_guid) in &created_new_stacks {
            self.send_loot_item_push_result(
                player_guid,
                *item_guid,
                loot_entry,
                stack.random_properties_id,
                stack.random_properties_seed,
                stack.slot,
                stack.count,
                stack.count,
                false,
                dungeon_encounter_id,
            );
        }

        if !created_new_stacks.is_empty() {
            let changed_slots: Vec<_> = created_new_stacks
                .iter()
                .map(|(stack, _, item_guid)| (stack.slot, *item_guid))
                .collect();
            self.send_player_values_update_from_entity_bridge(&changed_slots, &[], &[], &[], None);
        }
        for update in &collection_updates {
            self.send_player_values_update_like_cpp(update);
        }

        self.sync_player_registry_state_like_cpp();
        true
    }

    fn plan_direct_loot_item_preserving_cpp_store_metadata(
        &self,
        loot_entry: &LootEntry,
        random_properties: LootStoreRandomProperties,
    ) -> Option<Vec<ItemPosCount>> {
        let max_stack = self
            .item_storage_template(loot_entry.item_id)
            .map(|template| template.max_stack_size)
            .unwrap_or(1)
            .max(1);
        let mut remaining = loot_entry.quantity;
        let mut dest = Vec::new();

        let mut existing_slots: Vec<u8> = self.inventory_items_like_cpp().keys().copied().collect();
        existing_slots.sort_unstable();
        for slot in existing_slots {
            if remaining == 0 {
                break;
            }
            let Some(existing) = self.inventory_items_like_cpp().get(&slot) else {
                continue;
            };
            let Some(existing_object) = self.inventory_item_objects_like_cpp().get(&existing.guid)
            else {
                continue;
            };
            if existing.entry_id != loot_entry.item_id
                || !loot_store_data_can_stack_with_item(
                    loot_entry,
                    random_properties,
                    existing_object,
                )
                || existing_object.count() >= max_stack
            {
                continue;
            }
            let can_add = max_stack
                .saturating_sub(existing_object.count())
                .min(remaining);
            if can_add > 0 {
                dest.push(ItemPosCount::new(
                    make_item_pos(INVENTORY_SLOT_BAG_0, slot),
                    can_add,
                ));
                remaining = remaining.saturating_sub(can_add);
            }
        }

        let backpack_end = INVENTORY_SLOT_ITEM_START
            .saturating_add(INVENTORY_DEFAULT_SIZE)
            .min(INVENTORY_SLOT_ITEM_END);
        for slot in INVENTORY_SLOT_ITEM_START..backpack_end {
            if remaining == 0 {
                break;
            }
            if self.inventory_items_like_cpp().contains_key(&slot) {
                continue;
            }
            let quantity = max_stack.min(remaining);
            dest.push(ItemPosCount::new(
                make_item_pos(INVENTORY_SLOT_BAG_0, slot),
                quantity,
            ));
            remaining = remaining.saturating_sub(quantity);
        }

        (remaining == 0).then_some(dest)
    }

    fn send_loot_item_push_result(
        &self,
        player_guid: ObjectGuid,
        item_guid: ObjectGuid,
        loot_entry: &LootEntry,
        random_properties_id: i32,
        random_properties_seed: i32,
        slot: u8,
        quantity: u32,
        quantity_in_inventory: u32,
        created: bool,
        dungeon_encounter_id: u32,
    ) {
        let is_encounter_loot = dungeon_encounter_id != 0;
        self.send_packet(&ItemPushResult {
            player_guid,
            slot: u8::from(INVENTORY_SLOT_BAG_0),
            slot_in_bag: i32::from(slot),
            item: ItemInstance {
                item_id: loot_entry.item_id as i32,
                random_properties_seed,
                random_properties_id,
                item_bonus: None,
                modifications: ItemModList { values: Vec::new() },
            },
            quest_log_item_id: 0,
            quantity: quantity as i32,
            quantity_in_inventory: quantity_in_inventory as i32,
            dungeon_encounter_id: dungeon_encounter_id as i32,
            battle_pet_species_id: 0,
            battle_pet_breed_id: 0,
            battle_pet_breed_quality: 0,
            battle_pet_level: 0,
            item_guid,
            pushed: false,
            display_text: if is_encounter_loot {
                ItemPushResultDisplayType::EncounterLoot
            } else {
                ItemPushResultDisplayType::Normal
            },
            created,
            is_bonus_roll: false,
            is_encounter_loot,
        });
    }

    async fn destroy_fully_looted_direct_item(&mut self, item_guid: ObjectGuid) {
        let player_guid = match self.player_guid() {
            Some(guid) => guid,
            None => return,
        };

        let runtime_item = self
            .inventory_item_objects_like_cpp()
            .get(&item_guid)
            .cloned();
        let (bag, slot) = match runtime_item.as_ref() {
            Some(item) => (item.bag_slot(), item.slot()),
            None => return,
        };

        let Some(item) = self.get_inventory_item_by_pos(bag, slot) else {
            return;
        };

        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };

        let mut tx = SqlTransaction::new();
        let should_expire_refund = runtime_item
            .as_ref()
            .is_some_and(|item_object| item_object.is_refundable());
        if should_expire_refund {
            let mut del_refund = char_db.prepare(CharStatements::DEL_ITEM_REFUND_INSTANCE);
            del_refund.set_u64(0, item.db_guid);
            tx.append(del_refund);
        }

        let mut del_inv = char_db.prepare(CharStatements::DEL_CHAR_INVENTORY_ITEM);
        del_inv.set_u64(0, player_guid.counter() as u64);
        del_inv.set_u64(1, item.db_guid);
        tx.append(del_inv);

        let mut del_item = char_db.prepare(CharStatements::DEL_ITEM_INSTANCE);
        del_item.set_u64(0, item.db_guid);
        tx.append(del_item);

        if let Err(e) = char_db.commit_transaction(tx).await {
            warn!("LootRelease: delete fully looted item failed: {e}");
            return;
        }

        self.remove_fully_looted_runtime_item(bag, slot, item.guid);

        if should_expire_refund {
            self.send_packet(&ItemExpirePurchaseRefund {
                item_guid: item.guid,
            });
        }

        // Player-values update and stat refresh only apply to top-level slots.
        if bag == INVENTORY_SLOT_BAG_0 {
            let mut visible_item_changes = Vec::new();
            let mut virtual_item_changes = Vec::new();
            if (slot as usize) < 19 {
                visible_item_changes.push((slot, 0i32, 0u16, 0u16));
            }
            if slot >= 15 && slot <= 17 {
                virtual_item_changes.push((slot - 15, 0i32, 0u16, 0u16));
            }

            self.send_player_values_update_from_entity_bridge(
                &[(slot, ObjectGuid::EMPTY)],
                &visible_item_changes,
                &virtual_item_changes,
                &[],
                None,
            );

            if slot < 19 {
                self.send_stat_update();
            }
        }
    }
}

fn master_loot_error_for_inventory_result_like_cpp(result: InventoryResult) -> Option<u8> {
    match result {
        InventoryResult::Ok => None,
        InventoryResult::ItemMaxCount => Some(LOOT_ERROR_MASTER_UNIQUE_ITEM_LIKE_CPP),
        InventoryResult::InvFull => Some(LOOT_ERROR_MASTER_INV_FULL_LIKE_CPP),
        _ => Some(LOOT_ERROR_MASTER_OTHER_LIKE_CPP),
    }
}

// ── Loot generation ───────────────────────────────────────────────

fn generate_legacy_creature_coin_fallback_like_cpp(creature_guid: ObjectGuid, level: u8) -> u32 {
    let base = level as u32;
    let seed = creature_guid.counter() as u32;
    base * 200 + (seed % (base * 300 + 1))
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ItemTemplateAddonLootMetadataLikeCpp {
    flags_cu: u32,
    quest_log_item_id: i32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RepresentedLootPlayerContext {
    race: u8,
    class: u8,
    gender: u8,
    level: u8,
    known_spells: Vec<i32>,
    active_quest_statuses: HashMap<u32, u8>,
    active_quest_objective_counts: HashMap<u32, Vec<i32>>,
    rewarded_quests: HashSet<u32>,
    inventory_item_counts: HashMap<u32, u32>,
    is_current: bool,
}

impl RepresentedLootPlayerContext {
    fn quest_status(&self, quest_id: u32) -> u8 {
        if self.rewarded_quests.contains(&quest_id) {
            return 3;
        }
        self.active_quest_statuses
            .get(&quest_id)
            .copied()
            .unwrap_or(0)
    }

    fn inventory_item_count(&self, item_id: u32) -> u32 {
        self.inventory_item_counts
            .get(&item_id)
            .copied()
            .unwrap_or(0)
    }
}

impl ItemTemplateAddonLootMetadataLikeCpp {
    fn ignores_quest_status(self) -> bool {
        self.flags_cu & ITEM_FLAGS_CU_IGNORE_QUEST_STATUS_LIKE_CPP != 0
    }

    fn follows_loot_rules(self) -> bool {
        self.flags_cu & ITEM_FLAGS_CU_FOLLOW_LOOT_RULES_LIKE_CPP != 0
    }
}

fn player_class_mask_like_cpp(class_id: u8) -> Option<u32> {
    if (1..=13).contains(&class_id) {
        Some(1_u32 << (class_id - 1))
    } else {
        None
    }
}

fn player_race_mask_like_cpp(race_id: u8) -> Option<u32> {
    let bit = match race_id {
        1..=11 => race_id - 1,
        22 => 21,
        24..=32 => race_id - 1,
        34 => 11,
        35 => 12,
        36 => 13,
        37 => 14,
        52 => 16,
        70 => 15,
        _ => return None,
    };
    Some(1_u32 << bit)
}

fn player_team_for_race_cpp_representable(race: u8) -> u32 {
    match race {
        2 | 5 | 6 | 8 | 9 | 10 | 26 | 27 | 28 | 31 | 35 | 36 | 70 => 67,
        _ => 469,
    }
}

fn represented_item_faction_flags_block_player_like_cpp(flags2: Option<u32>, race: u8) -> bool {
    let Some(flags2) = flags2 else {
        return false;
    };

    let team = player_team_for_race_cpp_representable(race);
    ((flags2 & ItemFlags2::FactionHorde as u32) != 0 && team != 67)
        || ((flags2 & ItemFlags2::FactionAlliance as u32) != 0 && team != 469)
}

fn player_quest_status_mask_like_cpp(status: Option<u8>, rewarded: bool) -> u32 {
    if rewarded {
        return 0x40;
    }

    match status {
        None => 0x01,
        Some(2) => 0x02,
        Some(1) => 0x08,
        Some(3) => 0x20,
        _ => 0,
    }
}

fn generated_creature_loot_item_to_entry_like_cpp(
    item: GeneratedLootItem,
    addon_metadata: ItemTemplateAddonLootMetadataLikeCpp,
) -> LootEntry {
    LootEntry {
        loot_list_id: item.loot_list_id as u8,
        item_id: item.item_id,
        quantity: item.count,
        random_properties_id: item.random_properties_id,
        random_properties_seed: item.random_properties_seed,
        item_context: item.context,
        flags: LootEntryFlags {
            follow_loot_rules: !item.needs_quest || addon_metadata.follows_loot_rules(),
            freeforall: item.free_for_all,
            blocked: item.is_blocked,
            counted: item.is_counted,
            under_threshold: item.is_under_threshold,
            needs_quest: item.needs_quest,
        },
        allowed_looters: Vec::new(),
        roll_winner: ObjectGuid::EMPTY,
        ffa_looted_by: Vec::new(),
        taken: item.is_looted,
    }
}

#[derive(Debug, Clone)]
struct RepresentedCreatureLootStateLikeCpp {
    is_alive: bool,
    position: wow_core::Position,
    level: u8,
    entry: u32,
    loot_id: u32,
    gold_min: u32,
    gold_max: u32,
    dungeon_encounter_id: u32,
    tappers: Vec<ObjectGuid>,
}

#[derive(Debug, Clone, Copy)]
struct RepresentedGameObjectLootStateLikeCpp {
    position: Option<wow_core::Position>,
    display_id: Option<u32>,
    scale: f32,
    rotation: [f32; 4],
    go_type: Option<u8>,
    interact_radius_override: Option<u32>,
    lock_id: Option<u32>,
    owner_guid: Option<ObjectGuid>,
}

pub(crate) fn represented_gameobject_interaction_distance_like_cpp(
    go_type: Option<u8>,
    interact_radius_override: Option<u32>,
) -> f32 {
    // C++ ref: GameObject.cpp GetInteractionDistance().
    // Spell-lock range remains with the typed GameObject/SpellInfo port.
    if let Some(override_hundredths) = interact_radius_override.filter(|value| *value != 0) {
        return override_hundredths as f32 / 100.0;
    }

    match go_type.map(u32::from) {
        Some(GAMEOBJECT_TYPE_AREADAMAGE) => 0.0,
        Some(GAMEOBJECT_TYPE_QUESTGIVER)
        | Some(GAMEOBJECT_TYPE_TEXT)
        | Some(GAMEOBJECT_TYPE_FLAGSTAND)
        | Some(GAMEOBJECT_TYPE_FLAGDROP)
        | Some(GAMEOBJECT_TYPE_MINI_GAME) => 5.5555553,
        Some(GAMEOBJECT_TYPE_BINDER) => 10.0,
        Some(GAMEOBJECT_TYPE_CHAIR) | Some(GAMEOBJECT_TYPE_BARBER_CHAIR) => 3.0,
        Some(GAMEOBJECT_TYPE_FISHING_NODE) => 100.0,
        Some(GAMEOBJECT_TYPE_FISHING_HOLE) => 20.0 + wow_movement::CONTACT_DISTANCE_LIKE_CPP,
        Some(GAMEOBJECT_TYPE_CAMERA)
        | Some(GAMEOBJECT_TYPE_MAP_OBJECT)
        | Some(GAMEOBJECT_TYPE_DUNGEON_DIFFICULTY)
        | Some(GAMEOBJECT_TYPE_DESTRUCTIBLE_BUILDING)
        | Some(GAMEOBJECT_TYPE_DOOR) => 5.0,
        Some(GAMEOBJECT_TYPE_GUILD_BANK) | Some(GAMEOBJECT_TYPE_MAILBOX) => 10.0,
        _ => 5.0,
    }
}

fn represented_gameobject_display_box_contains_like_cpp(
    go_position: wow_core::Position,
    player_position: wow_core::Position,
    display_info: &wow_data::GameObjectDisplayInfoEntry,
    scale: f32,
    rotation: [f32; 4],
    radius: f32,
) -> bool {
    let min_x = display_info.geo_box_min.x * scale - radius;
    let min_y = display_info.geo_box_min.y * scale - radius;
    let min_z = display_info.geo_box_min.z * scale - radius;
    let max_x = display_info.geo_box_max.x * scale + radius;
    let max_y = display_info.geo_box_max.y * scale + radius;
    let max_z = display_info.geo_box_max.z * scale + radius;

    let dx = player_position.x - go_position.x;
    let dy = player_position.y - go_position.y;
    let dz = player_position.z - go_position.z;
    let [qx, qy, qz, qw] = rotation;
    let iqx = -qx;
    let iqy = -qy;
    let iqz = -qz;

    let tx = 2.0 * (iqy * dz - iqz * dy);
    let ty = 2.0 * (iqz * dx - iqx * dz);
    let tz = 2.0 * (iqx * dy - iqy * dx);
    let local_x = dx + qw * tx + (iqy * tz - iqz * ty);
    let local_y = dy + qw * ty + (iqz * tx - iqx * tz);
    let local_z = dz + qw * tz + (iqx * ty - iqy * tx);

    local_x >= min_x
        && local_x <= max_x
        && local_y >= min_y
        && local_y <= max_y
        && local_z >= min_z
        && local_z <= max_z
}

fn represented_loot_object_guid_like_cpp(owner: ObjectGuid) -> ObjectGuid {
    if owner.is_empty() {
        return ObjectGuid::EMPTY;
    }

    ObjectGuid::create_world_object(
        HighGuid::LootObject,
        0,
        owner.realm_id(),
        owner.map_id(),
        owner.server_id(),
        0,
        owner.counter(),
    )
}

fn loot_type_for_client_like_cpp(loot_type: u8) -> u8 {
    match loot_type {
        LOOT_TYPE_PROSPECTING_LIKE_CPP | LOOT_TYPE_MILLING_LIKE_CPP => {
            LOOT_TYPE_DISENCHANTING_LIKE_CPP
        }
        LOOT_TYPE_INSIGNIA_LIKE_CPP => LOOT_TYPE_SKINNING_LIKE_CPP,
        LOOT_TYPE_FISHINGHOLE_LIKE_CPP | LOOT_TYPE_FISHING_JUNK_LIKE_CPP => {
            LOOT_TYPE_FISHING_LIKE_CPP
        }
        _ => loot_type,
    }
}

fn loot_is_looted_like_cpp(loot: &CreatureLoot) -> bool {
    loot.coins == 0 && loot.unlooted_count == 0
}

fn mark_loot_allowed_for_player_like_cpp(loot: &mut CreatureLoot, player_guid: ObjectGuid) {
    if !player_guid.is_empty() && !loot.allowed_looters.contains(&player_guid) {
        loot.allowed_looters.push(player_guid);
    }

    for entry in &mut loot.items {
        if entry.allowed_looters.is_empty() || entry.flags.freeforall {
            entry.add_allowed_looter_like_cpp(player_guid);
        }
    }

    let existing_ffa_item_ids: Vec<u8> = loot
        .player_ffa_items
        .iter()
        .find(|(player, _)| *player == player_guid)
        .map(|(_, items)| items.iter().map(|item| item.loot_list_id).collect())
        .unwrap_or_default();
    let mut ffa_items = Vec::new();
    for entry in &mut loot.items {
        if entry.flags.freeforall
            && entry.has_allowed_looter_like_cpp(player_guid)
            && !existing_ffa_item_ids.contains(&entry.loot_list_id)
        {
            ffa_items.push(NotNormalLootItem {
                loot_list_id: entry.loot_list_id,
                is_looted: false,
            });
            loot.unlooted_count = loot.unlooted_count.saturating_add(1);
        } else if !entry.flags.freeforall
            && entry.has_allowed_looter_like_cpp(player_guid)
            && !entry.flags.counted
        {
            entry.flags.counted = true;
            loot.unlooted_count = loot.unlooted_count.saturating_add(1);
        }
    }

    if !ffa_items.is_empty() {
        match loot
            .player_ffa_items
            .iter_mut()
            .find(|(player, _)| *player == player_guid)
        {
            Some((_, existing)) => existing.extend(ffa_items),
            None => loot.player_ffa_items.push((player_guid, ffa_items)),
        }
    }
}

#[cfg(test)]
fn assign_represented_personal_loot_items_like_cpp<R: Rng + ?Sized>(
    loot: &mut CreatureLoot,
    tappers: &[ObjectGuid],
    rng: &mut R,
) {
    if tappers.is_empty() {
        return;
    }

    loot.unlooted_count = 0;
    loot.player_ffa_items.clear();

    for entry in &mut loot.items {
        entry.allowed_looters.clear();
        entry.flags.counted = false;

        let chosen_tapper = tappers[rng.gen_range(0..tappers.len())];
        entry.add_allowed_looter_like_cpp(chosen_tapper);
    }

    rebuild_represented_personal_loot_counts_like_cpp(loot);
}

fn rebuild_represented_personal_loot_counts_like_cpp(loot: &mut CreatureLoot) {
    loot.unlooted_count = 0;
    loot.player_ffa_items.clear();

    for entry in &mut loot.items {
        entry.ffa_looted_by.clear();
        entry.flags.counted = false;

        if entry.flags.freeforall {
            for looter in &entry.allowed_looters {
                match loot
                    .player_ffa_items
                    .iter_mut()
                    .find(|(player, _)| player == looter)
                {
                    Some((_, existing)) => existing.push(NotNormalLootItem {
                        loot_list_id: entry.loot_list_id,
                        is_looted: false,
                    }),
                    None => loot.player_ffa_items.push((
                        *looter,
                        vec![NotNormalLootItem {
                            loot_list_id: entry.loot_list_id,
                            is_looted: false,
                        }],
                    )),
                }
                loot.unlooted_count = loot.unlooted_count.saturating_add(1);
            }
        } else if !entry.allowed_looters.is_empty() {
            entry.flags.counted = true;
            loot.unlooted_count = loot.unlooted_count.saturating_add(1);
        }
    }
}

fn loot_player_has_unlooted_ffa_item_like_cpp(
    loot: &CreatureLoot,
    player_guid: ObjectGuid,
    loot_list_id: u8,
) -> bool {
    loot.player_ffa_items
        .iter()
        .find(|(player, _)| *player == player_guid)
        .is_some_and(|(_, items)| {
            items
                .iter()
                .any(|item| item.loot_list_id == loot_list_id && !item.is_looted)
        })
}

fn loot_item_is_looted_for_player_like_cpp(
    loot: &CreatureLoot,
    entry: &LootEntry,
    player_guid: ObjectGuid,
) -> bool {
    if entry.flags.freeforall {
        !loot_player_has_unlooted_ffa_item_like_cpp(loot, player_guid, entry.loot_list_id)
    } else {
        entry.taken
    }
}

fn mark_loot_item_looted_for_player_like_cpp(
    loot: &mut CreatureLoot,
    loot_list_id: u8,
    player_guid: ObjectGuid,
) {
    let should_decrement = loot
        .items
        .iter()
        .find(|entry| entry.loot_list_id == loot_list_id)
        .is_some_and(|entry| !loot_item_is_looted_for_player_like_cpp(loot, entry, player_guid));

    if let Some(entry) = loot
        .items
        .iter_mut()
        .find(|entry| entry.loot_list_id == loot_list_id)
    {
        entry.mark_looted_for_player_like_cpp(player_guid);
        if entry.flags.freeforall {
            if let Some((_, items)) = loot
                .player_ffa_items
                .iter_mut()
                .find(|(player, _)| *player == player_guid)
                && let Some(item) = items
                    .iter_mut()
                    .find(|item| item.loot_list_id == loot_list_id)
            {
                item.is_looted = true;
            }
        }
        if should_decrement {
            loot.unlooted_count = loot.unlooted_count.saturating_sub(1);
        }
    }
}

fn represented_loot_response_items_like_cpp(
    loot: &CreatureLoot,
    player_guid: ObjectGuid,
) -> Vec<LootItemData> {
    loot.items
        .iter()
        .filter_map(|entry| {
            let ui_type = loot_item_ui_type_for_player_like_cpp(
                player_guid,
                &entry.allowed_looters,
                loot_item_is_looted_for_player_like_cpp(loot, entry, player_guid),
                entry.flags.freeforall,
                loot_player_has_unlooted_ffa_item_like_cpp(loot, player_guid, entry.loot_list_id),
                entry.flags.needs_quest,
                entry.flags.follow_loot_rules,
                loot.loot_method,
                loot.round_robin_player,
                loot.loot_master,
                entry.flags.under_threshold,
                entry.flags.blocked,
                entry.roll_winner,
            )?;

            Some(LootItemData {
                item_type: 0,
                ui_type,
                can_trade_to_tap_list: false,
                loot: ItemInstance {
                    item_id: entry.item_id as i32,
                    ..ItemInstance::default()
                },
                loot_list_id: entry.loot_list_id,
                quantity: entry.quantity,
                loot_item_type: 0,
            })
        })
        .collect()
}

fn loot_can_be_opened_by_player_like_cpp(loot: &CreatureLoot, player_guid: ObjectGuid) -> bool {
    if loot_is_looted_like_cpp(loot) {
        return false;
    }

    loot_has_item_for_all_like_cpp(loot, player_guid)
        || loot_has_item_for_player_like_cpp(loot, player_guid)
}

fn loot_has_over_threshold_item_like_cpp(loot: &CreatureLoot) -> bool {
    loot.items
        .iter()
        .any(|entry| !entry.taken && entry.is_over_threshold_like_cpp())
}

fn connected_roll_looters_like_cpp(
    entry: &LootEntry,
    player_guid: ObjectGuid,
    current_map_id: u16,
    player_registry: Option<&PlayerRegistry>,
) -> Vec<ObjectGuid> {
    let mut looters = Vec::new();

    for looter in &entry.allowed_looters {
        if *looter == player_guid {
            looters.push(*looter);
            continue;
        }

        let Some(registry) = player_registry else {
            continue;
        };
        let Some(player) = registry.get(looter) else {
            continue;
        };
        if player.map_id == current_map_id {
            looters.push(*looter);
        }
    }

    looters.sort_by_key(|guid| (guid.high_value(), guid.low_value()));
    looters.dedup();
    looters
}

fn represented_max_enchanting_skill_like_cpp(
    looters: &[ObjectGuid],
    current_player_guid: ObjectGuid,
    current_player_enchanting_skill: u16,
    player_registry: Option<&PlayerRegistry>,
) -> u16 {
    looters.iter().fold(0, |max_skill, looter| {
        if *looter == current_player_guid {
            max_skill.max(current_player_enchanting_skill)
        } else {
            max_skill.max(
                player_registry
                    .and_then(|registry| registry.get(looter))
                    .map(|player| player.enchanting_skill)
                    .unwrap_or(0),
            )
        }
    })
}

fn start_loot_roll_packet_like_cpp(
    loot_obj: ObjectGuid,
    map_id: u16,
    loot_method: u8,
    entry: &LootEntry,
    valid_rolls: u8,
    dungeon_encounter_id: i32,
) -> StartLootRoll {
    StartLootRoll {
        loot_obj,
        map_id: map_id as i32,
        roll_time_ms: LOOT_ROLL_TIMEOUT_MS_LIKE_CPP,
        method: loot_method,
        valid_rolls,
        loot_roll_ineligible_reason: [0; 4],
        item: LootItemData {
            item_type: 0,
            ui_type: LOOT_SLOT_TYPE_ROLL_ONGOING_LIKE_CPP,
            can_trade_to_tap_list: entry.allowed_looters.len() > 1,
            loot: ItemInstance {
                item_id: entry.item_id as i32,
                random_properties_id: entry.random_properties_id,
                random_properties_seed: entry.random_properties_seed,
                ..ItemInstance::default()
            },
            loot_list_id: entry.loot_list_id,
            quantity: entry.quantity,
            loot_item_type: 0,
        },
        dungeon_encounter_id,
    }
}

fn loot_roll_broadcast_item_like_cpp(entry: &LootEntry, ui_type: u8) -> LootItemData {
    LootItemData {
        item_type: 0,
        ui_type,
        can_trade_to_tap_list: entry.allowed_looters.len() > 1,
        loot: ItemInstance {
            item_id: entry.item_id as i32,
            random_properties_id: entry.random_properties_id,
            random_properties_seed: entry.random_properties_seed,
            ..ItemInstance::default()
        },
        loot_list_id: entry.loot_list_id,
        quantity: entry.quantity,
        loot_item_type: 0,
    }
}

fn represented_roll_number_like_cpp() -> u8 {
    rand::thread_rng().gen_range(1..=100)
}

fn roll_chance_with_rate_like_cpp<R: Rng + ?Sized>(chance: f32, rate: f32, rng: &mut R) -> bool {
    if chance >= 100.0 {
        return true;
    }
    rng.gen_range(0.0f32..100.0f32) < chance * rate
}

fn referenced_loot_max_count_like_cpp(max_count: u8, rate: f32) -> u32 {
    ((max_count as f32) * rate) as u32
}

fn represented_disenchant_loot_plain_row_can_roll_like_cpp(
    row: &LootStoreItem,
    item_exists: bool,
) -> bool {
    row.can_roll_as_plain_entry_like_cpp(item_exists, LOOT_MODE_DEFAULT_LIKE_CPP)
}

fn represented_disenchant_loot_reference_row_can_roll_like_cpp(row: &LootStoreItem) -> bool {
    row.can_roll_as_reference_entry_like_cpp(LOOT_MODE_DEFAULT_LIKE_CPP)
}

fn add_loot_item_stacks_like_cpp(
    loot_items: &mut Vec<LootEntry>,
    item_id: u32,
    mut count: u32,
    max_stack_size: u32,
    flags: LootEntryFlags,
) {
    while count > 0 && loot_items.len() < MAX_NR_LOOT_ITEMS_LIKE_CPP {
        let quantity = count.min(max_stack_size);
        loot_items.push(LootEntry {
            loot_list_id: loot_items.len() as u8,
            item_id,
            quantity,
            random_properties_id: 0,
            random_properties_seed: 0,
            item_context: 0,
            flags,
            allowed_looters: Vec::new(),
            roll_winner: ObjectGuid::EMPTY,
            ffa_looted_by: Vec::new(),
            taken: false,
        });
        count = count.saturating_sub(max_stack_size);
    }
}

#[derive(Debug, Clone)]
struct DisenchantLootTemplateFrame {
    template: LootTemplate,
    entry_index: usize,
    group_index: usize,
    requested_group_id: u8,
}

fn disenchant_loot_template_frame_like_cpp(
    rows: Vec<LootStoreItem>,
    requested_group_id: u8,
) -> DisenchantLootTemplateFrame {
    let mut template = LootTemplate::default();
    for row in rows {
        template.add_entry_like_cpp(row);
    }

    DisenchantLootTemplateFrame {
        template,
        entry_index: 0,
        group_index: 0,
        requested_group_id,
    }
}

#[derive(Debug, Clone, Copy)]
enum DisenchantLootTemplateTable {
    Disenchant,
    Reference,
}

impl DisenchantLootTemplateTable {
    fn name(self) -> &'static str {
        match self {
            Self::Disenchant => "disenchant_loot_template",
            Self::Reference => "reference_loot_template",
        }
    }
}

fn represented_loot_roll_finish_winner_like_cpp(
    state: &RepresentedLootRollState,
) -> Option<Option<(ObjectGuid, RepresentedLootRollVote)>> {
    let mut winner = None;
    let mut has_need = false;

    for (player_guid, vote) in &state.voters {
        match vote.vote {
            ROLL_VOTE_NEED_LIKE_CPP => {
                if !has_need
                    || winner.is_none_or(|(_, current): (ObjectGuid, RepresentedLootRollVote)| {
                        vote.roll_number > current.roll_number
                    })
                {
                    has_need = true;
                    winner = Some((*player_guid, *vote));
                }
            }
            ROLL_VOTE_GREED_LIKE_CPP | ROLL_VOTE_DISENCHANT_LIKE_CPP => {
                if !has_need
                    && winner.is_none_or(|(_, current): (ObjectGuid, RepresentedLootRollVote)| {
                        vote.roll_number > current.roll_number
                    })
                {
                    winner = Some((*player_guid, *vote));
                }
            }
            ROLL_VOTE_PASS_LIKE_CPP | ROLL_VOTE_NOT_VALID_LIKE_CPP => {}
            ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP => return None,
            _ => {}
        }
    }

    Some(winner)
}

fn represented_loot_roll_current_winner_like_cpp(
    state: &RepresentedLootRollState,
) -> Option<(ObjectGuid, RepresentedLootRollVote)> {
    let mut winner = None;
    let mut has_need = false;

    for (player_guid, vote) in &state.voters {
        match vote.vote {
            ROLL_VOTE_NEED_LIKE_CPP => {
                if !has_need
                    || winner.is_none_or(|(_, current): (ObjectGuid, RepresentedLootRollVote)| {
                        vote.roll_number > current.roll_number
                    })
                {
                    has_need = true;
                    winner = Some((*player_guid, *vote));
                }
            }
            ROLL_VOTE_GREED_LIKE_CPP | ROLL_VOTE_DISENCHANT_LIKE_CPP => {
                if !has_need
                    && winner.is_none_or(|(_, current): (ObjectGuid, RepresentedLootRollVote)| {
                        vote.roll_number > current.roll_number
                    })
                {
                    winner = Some((*player_guid, *vote));
                }
            }
            ROLL_VOTE_PASS_LIKE_CPP
            | ROLL_VOTE_NOT_VALID_LIKE_CPP
            | ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP => {}
            _ => {}
        }
    }

    winner
}

fn loot_has_item_for_all_like_cpp(loot: &CreatureLoot, player_guid: ObjectGuid) -> bool {
    if loot.coins > 0 {
        return true;
    }

    loot.items.iter().any(|entry| {
        !entry.taken
            && entry.flags.follow_loot_rules
            && !entry.flags.freeforall
            && entry.has_allowed_looter_like_cpp(player_guid)
    })
}

fn loot_has_item_for_player_like_cpp(loot: &CreatureLoot, player_guid: ObjectGuid) -> bool {
    loot.items.iter().any(|entry| {
        !loot_item_is_looted_for_player_like_cpp(loot, entry, player_guid)
            && entry.has_allowed_looter_like_cpp(player_guid)
            && (!entry.flags.follow_loot_rules || entry.flags.freeforall)
    })
}

fn loot_item_context(context: u8) -> ItemContext {
    <ItemContext as num_traits::FromPrimitive>::from_u8(context).unwrap_or(ItemContext::None)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LootStoreRandomProperties {
    id: i32,
    seed: i32,
}

fn loot_store_data_can_stack_with_item(
    loot_entry: &LootEntry,
    random_properties: LootStoreRandomProperties,
    item: &Item,
) -> bool {
    let data = item.data();
    data.random_properties_id == random_properties.id
        && data.property_seed == random_properties.seed
        && u8::try_from(data.context).unwrap_or(0) == loot_entry.item_context
}

impl WorldSession {
    fn generate_loot_store_random_properties_like_cpp(
        &self,
        item_id: u32,
    ) -> LootStoreRandomProperties {
        let mut rng = rand::thread_rng();
        self.generate_loot_store_random_properties_with_rng_like_cpp(item_id, &mut rng)
    }

    fn generate_loot_store_random_properties_with_rng_like_cpp<R: Rng + ?Sized>(
        &self,
        item_id: u32,
        rng: &mut R,
    ) -> LootStoreRandomProperties {
        // C++ Player::StoreLootItem calls ItemEnchantmentMgr::GenerateRandomProperties(itemid).
        let random_select = self.item_template_random_select(item_id);
        let random_suffix = self.item_template_random_suffix_group_id(item_id);
        if random_select == 0 && random_suffix == 0 {
            return LootStoreRandomProperties { id: 0, seed: 0 };
        }

        if random_select != 0 {
            let Some(random_properties_id) =
                self.select_random_enchantment_from_group_like_cpp(u32::from(random_select), rng)
            else {
                return LootStoreRandomProperties { id: 0, seed: 0 };
            };

            if self
                .item_random_properties_store()
                .and_then(|store| store.get(random_properties_id))
                .is_none()
            {
                return LootStoreRandomProperties { id: 0, seed: 0 };
            }

            return LootStoreRandomProperties {
                id: i32::try_from(random_properties_id).unwrap_or(0),
                seed: 0,
            };
        }

        let Some(random_suffix_id) =
            self.select_random_enchantment_from_group_like_cpp(u32::from(random_suffix), rng)
        else {
            return LootStoreRandomProperties { id: 0, seed: 0 };
        };

        if self
            .item_random_suffix_store()
            .and_then(|store| store.get(random_suffix_id))
            .is_none()
        {
            return LootStoreRandomProperties { id: 0, seed: 0 };
        }

        let seed = self
            .item_random_property_template(item_id)
            .map(|template| self.random_property_points_like_cpp(template))
            .unwrap_or(0);

        LootStoreRandomProperties {
            id: -i32::try_from(random_suffix_id).unwrap_or(0),
            seed,
        }
    }

    fn select_random_enchantment_from_group_like_cpp<R: Rng + ?Sized>(
        &self,
        group_id: u32,
        rng: &mut R,
    ) -> Option<u32> {
        let group = self
            .item_random_enchantment_template_store()
            .and_then(|store| store.group(group_id))?;
        select_weighted_random_enchantment_like_cpp(group, rng)
    }

    fn random_property_points_like_cpp(&self, template: ItemRandomPropertyTemplateEntry) -> i32 {
        let prop_index =
            match <InventoryType as num_traits::FromPrimitive>::from_i8(template.inventory_type) {
                Some(InventoryType::NonEquip)
                | Some(InventoryType::Bag)
                | Some(InventoryType::Tabard)
                | Some(InventoryType::Ammo)
                | Some(InventoryType::Quiver)
                | Some(InventoryType::Relic)
                | None => return 0,
                Some(InventoryType::Head)
                | Some(InventoryType::Body)
                | Some(InventoryType::Chest)
                | Some(InventoryType::Legs)
                | Some(InventoryType::Weapon2Hand)
                | Some(InventoryType::Robe) => 0,
                Some(InventoryType::Shoulders)
                | Some(InventoryType::Waist)
                | Some(InventoryType::Feet)
                | Some(InventoryType::Hands)
                | Some(InventoryType::Trinket) => 1,
                Some(InventoryType::Neck)
                | Some(InventoryType::Wrists)
                | Some(InventoryType::Finger)
                | Some(InventoryType::Shield)
                | Some(InventoryType::Cloak)
                | Some(InventoryType::Holdable) => 2,
                Some(InventoryType::Weapon)
                | Some(InventoryType::WeaponMainhand)
                | Some(InventoryType::WeaponOffhand) => 3,
                Some(InventoryType::Ranged)
                | Some(InventoryType::Thrown)
                | Some(InventoryType::RangedRight) => 4,
                _ => return 0,
            };

        let Some(points) = self
            .rand_prop_points_store()
            .and_then(|store| store.get(u32::from(template.item_level)))
        else {
            return 0;
        };

        match <ItemQuality as num_traits::FromPrimitive>::from_i8(template.quality) {
            Some(ItemQuality::Uncommon) => points.good[prop_index] as i32,
            Some(ItemQuality::Rare) | Some(ItemQuality::Heirloom) => {
                points.superior[prop_index] as i32
            }
            Some(ItemQuality::Epic)
            | Some(ItemQuality::Legendary)
            | Some(ItemQuality::Artifact) => points.epic[prop_index] as i32,
            _ => 0,
        }
    }
}

fn select_weighted_random_enchantment_like_cpp<R: Rng + ?Sized>(
    group: &[ItemRandomEnchantmentTemplateEntry],
    rng: &mut R,
) -> Option<u32> {
    let valid_rows = group
        .iter()
        .filter(|row| (0.000001..=100.0).contains(&row.chance))
        .collect::<Vec<_>>();
    let weights = valid_rows.iter().map(|row| row.chance).collect::<Vec<_>>();
    let distribution = WeightedIndex::new(weights).ok()?;
    Some(valid_rows[distribution.sample(rng)].enchantment_id)
}

#[derive(Debug, Clone)]
struct PlannedLootNewStack {
    slot: u8,
    entry_id: u32,
    count: u32,
    max_durability: u32,
    random_properties_id: i32,
    random_properties_seed: i32,
    item_context: u8,
}

#[cfg(test)]
mod tests {
    use super::{
        GAMEOBJECT_TYPE_AREADAMAGE, GAMEOBJECT_TYPE_BINDER, GAMEOBJECT_TYPE_CHAIR,
        GAMEOBJECT_TYPE_DOOR, GAMEOBJECT_TYPE_GUILD_BANK, GAMEOBJECT_TYPE_QUESTGIVER,
        INVENTORY_SLOT_BAG_0, ITEM_FLAGS_CU_FOLLOW_LOOT_RULES_LIKE_CPP,
        ItemTemplateAddonLootMetadataLikeCpp, LOCK_KEY_SKILL_LIKE_CPP, LOCK_KEY_SPELL_LIKE_CPP,
        LOOT_METHOD_GROUP_LIKE_CPP, LOOT_METHOD_MASTER_LIKE_CPP, LOOT_MODE_DEFAULT_LIKE_CPP,
        LOOT_MODE_JUNK_FISH_LIKE_CPP, LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP,
        LOOT_SLOT_TYPE_ROLL_ONGOING_LIKE_CPP, LootStoreRandomProperties,
        ROLL_ALL_TYPE_NO_DISENCHANT_LIKE_CPP, ROLL_FLAG_TYPE_NEED_LIKE_CPP,
        ROLL_VOTE_GREED_LIKE_CPP, ROLL_VOTE_NEED_LIKE_CPP, ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP,
        ROLL_VOTE_NOT_VALID_LIKE_CPP, ROLL_VOTE_PASS_LIKE_CPP, RepresentedLootPlayerContext,
        SPELL_EFFECT_OPEN_LOCK_LIKE_CPP, SyncChestGameobjectStateAndRefreshLikeCppCommand,
        SyncGatheringNodeGameobjectStateAndRefreshLikeCppCommand,
        SyncGooberGameobjectStateAndRefreshLikeCppCommand,
        assign_represented_personal_loot_items_like_cpp,
        generated_creature_loot_item_to_entry_like_cpp, loot_is_looted_like_cpp, loot_item_context,
        loot_store_data_can_stack_with_item, loot_type_for_client_like_cpp,
        mark_loot_allowed_for_player_like_cpp, mark_loot_item_looted_for_player_like_cpp,
        represented_gameobject_display_box_contains_like_cpp,
        represented_gameobject_interaction_distance_like_cpp,
        represented_loot_object_guid_like_cpp, represented_loot_response_items_like_cpp,
        select_weighted_random_enchantment_like_cpp, start_loot_roll_packet_like_cpp,
    };
    use crate::session::{
        RepresentedGameObjectSpellCaster, RepresentedGameObjectUseEffect,
        RepresentedLootRollCriteriaEvent, SessionState,
    };
    use rand::{SeedableRng, rngs::StdRng};
    use std::time::{Duration, Instant};
    use std::{
        collections::{HashMap, HashSet},
        sync::{Arc, Mutex, RwLock},
    };
    use wow_ai::CreatureAI;
    use wow_constants::{
        InventoryResult, InventoryType, ItemBondingType, ItemClass, ItemContext, ItemFlags2,
        ItemQuality, TypeId, TypeMask,
    };
    use wow_core::{ObjectGuid, Position, guid::HighGuid};
    use wow_data::quest::{
        QUEST_REWARD_REPUTATIONS_COUNT, QuestObjective, QuestStore, QuestTemplate,
    };
    use wow_data::{
        AreaTableEntry, AreaTableStore, ChrSpecializationEntry, ChrSpecializationStore,
        ItemDisenchantLootEntry, ItemDisenchantLootStore, ItemRandomEnchantmentTemplateEntry,
        ItemRandomEnchantmentTemplateStore, ItemRandomPropertiesEntry, ItemRandomPropertiesStore,
        ItemRandomPropertyTemplateEntry, ItemRandomSuffixEntry, ItemRandomSuffixStore, ItemRecord,
        ItemSparseTemplateEntry, ItemStatsStore, ItemStore, RandPropPointsEntry,
        RandPropPointsStore, SpellEffectInfo, SpellInfo, SpellMiscEntry, SpellMiscStore,
        SpellRangeEntry, SpellRangeStore, SpellStore,
    };
    use wow_database::{CharStatements, StatementDef};
    use wow_entities::{
        AccessorObjectKind, GAMEOBJECT_TYPE_CHEST, GAMEOBJECT_TYPE_FISHING_HOLE,
        GAMEOBJECT_TYPE_FISHING_NODE, GAMEOBJECT_TYPE_GATHERING_NODE, GAMEOBJECT_TYPE_GOOBER,
        GO_DYNFLAG_LO_NO_INTERACT, GameObject, GameObjectLootSource, GameObjectOwnedLoot,
        GatheringNodeUseSource, GoState, Item, ItemCreateInfo, LootState, MAX_ITEM_SPELLS,
        WorldObject,
    };
    use wow_loot::{
        GeneratedLootItem, LOOT_SLOT_TYPE_OWNER_LIKE_CPP, LootConditionRowLikeCpp, LootStore,
        LootStoreItem, LootStoreKind, LootStores, LootTemplateRow,
    };
    use wow_network::{
        GroupInfo, GroupRegistry, LootDropRatesLikeCpp, LootRollVoteCommand, PendingInvites,
        PlayerBroadcastInfo, PlayerRegistry, SessionCommand,
    };
    use wow_packet::packets::loot::{
        CreatureLoot, LOOT_ERROR_MASTER_OTHER_LIKE_CPP, LOOT_ERROR_MASTER_UNIQUE_ITEM_LIKE_CPP,
        LOOT_ERROR_NO_LOOT_LIKE_CPP, LOOT_ERROR_PLAYER_NOT_FOUND_LIKE_CPP,
        LOOT_ERROR_TOO_FAR_LIKE_CPP, LOOT_TYPE_CHEST_LIKE_CPP, LOOT_TYPE_CORPSE_LIKE_CPP,
        LOOT_TYPE_CORPSE_PERSONAL_LIKE_CPP, LOOT_TYPE_DISENCHANTING_LIKE_CPP,
        LOOT_TYPE_FISHING_JUNK_LIKE_CPP, LOOT_TYPE_FISHING_LIKE_CPP,
        LOOT_TYPE_FISHINGHOLE_LIKE_CPP, LOOT_TYPE_GATHERING_NODE_LIKE_CPP,
        LOOT_TYPE_INSIGNIA_LIKE_CPP, LOOT_TYPE_ITEM_LIKE_CPP, LOOT_TYPE_MILLING_LIKE_CPP,
        LOOT_TYPE_NONE_LIKE_CPP, LOOT_TYPE_PROSPECTING_LIKE_CPP, LOOT_TYPE_SKINNING_LIKE_CPP,
        LootEntry, LootEntryFlags, LootRoll, MasterLootItem, SetLootSpecialization,
    };
    use wow_packet::packets::update::CreatureCreateData;
    use wow_packet::{ServerPacket, WorldPacket};

    use crate::session::{
        AuraApplication, InventoryItem, SPELL_AURA_INTERRUPT_FLAG_LOOTING_LIKE_CPP, SpellCastState,
        WorldSession,
    };

    fn make_session_with_send_capacity(
        capacity: usize,
    ) -> (WorldSession, flume::Receiver<Vec<u8>>) {
        let (_pkt_tx, pkt_rx) = flume::bounded::<WorldPacket>(1);
        let (send_tx, send_rx) = flume::bounded::<Vec<u8>>(capacity);
        (
            WorldSession::new(
                1,
                "TestAccount".into(),
                0,
                2,
                9,
                54261,
                vec![0u8; 40],
                "esES".into(),
                pkt_rx,
                send_tx,
            ),
            send_rx,
        )
    }

    fn make_session_with_send() -> (WorldSession, flume::Receiver<Vec<u8>>) {
        make_session_with_send_capacity(1)
    }

    fn make_session() -> WorldSession {
        make_session_with_send().0
    }

    fn canonical_world_object(guid: ObjectGuid, map_id: u32, position: Position) -> WorldObject {
        let (type_id, type_mask) = if guid.is_game_object() {
            (TypeId::GameObject, TypeMask::GAME_OBJECT)
        } else {
            (TypeId::Unit, TypeMask::UNIT)
        };
        let mut object = WorldObject::new(false, type_id, type_mask);
        object.object_mut().create(guid);
        object.set_map(map_id, 0).unwrap();
        object.relocate(position);
        object.object_mut().add_to_world();
        object
    }

    fn attach_canonical_map_object(
        session: &mut WorldSession,
        kind: AccessorObjectKind,
        object: WorldObject,
    ) {
        let map_id = object.map_id();
        let instance_id = object.instance_id();
        let manager = Arc::new(Mutex::new(wow_map::MapManager::default()));
        {
            let mut manager = manager.lock().unwrap();
            manager
                .create_world_map(map_id, instance_id)
                .map_mut()
                .add_to_map_like_cpp(kind, object)
                .unwrap();
        }
        session.set_canonical_map_manager(manager);
    }

    fn attach_canonical_gameobject(session: &mut WorldSession, game_object: GameObject) {
        let map_id = game_object.world().map_id();
        let instance_id = game_object.world().instance_id();
        let manager = Arc::new(Mutex::new(wow_map::MapManager::default()));
        {
            let mut manager = manager.lock().unwrap();
            manager
                .create_world_map(map_id, instance_id)
                .map_mut()
                .insert_map_object_record(
                    wow_entities::MapObjectRecord::new_game_object(game_object).unwrap(),
                )
                .unwrap();
        }
        session.set_canonical_map_manager(manager);
    }

    fn make_canonical_gameobject_for_session(
        session: &WorldSession,
        guid: ObjectGuid,
        go_type: u8,
    ) -> GameObject {
        let mut game_object = GameObject::new();
        game_object.world_mut().object_mut().create(guid);
        game_object
            .world_mut()
            .set_map(u32::from(session.player_map_id_like_cpp()), 0)
            .unwrap();
        game_object.world_mut().relocate(Position::ZERO);
        game_object.world_mut().object_mut().add_to_world();
        game_object.set_go_type(go_type);
        game_object
    }

    fn canonical_gameobject_snapshot(
        session: &WorldSession,
        guid: ObjectGuid,
    ) -> Option<GameObject> {
        let manager = session.canonical_map_manager.as_ref()?;
        let manager = manager.lock().ok()?;
        let map = manager.find_map(u32::from(session.player_map_id_like_cpp()), 0)?;
        map.map().get_typed_game_object(guid).cloned()
    }

    fn loot_item_packet(object: ObjectGuid, loot_list_id: u8) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_uint32(1);
        pkt.write_packed_guid(&object);
        pkt.write_uint8(loot_list_id);
        pkt.write_bit(false);
        pkt.flush_bits();
        pkt.reset_read();
        pkt
    }

    fn loot_unit_packet(object: ObjectGuid) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&object);
        pkt.reset_read();
        pkt
    }

    fn represented_loot_entry(
        loot_list_id: u8,
        item_id: u32,
        player_guid: ObjectGuid,
    ) -> LootEntry {
        LootEntry {
            loot_list_id,
            item_id,
            quantity: 1,
            random_properties_id: 0,
            random_properties_seed: 0,
            item_context: 0,
            flags: LootEntryFlags {
                follow_loot_rules: true,
                freeforall: false,
                blocked: false,
                counted: false,
                under_threshold: false,
                needs_quest: false,
            },
            allowed_looters: vec![player_guid],
            roll_winner: ObjectGuid::EMPTY,
            ffa_looted_by: Vec::new(),
            taken: false,
        }
    }

    #[test]
    fn represented_loot_type_for_client_matches_cpp_aliases() {
        assert_eq!(
            loot_type_for_client_like_cpp(LOOT_TYPE_NONE_LIKE_CPP),
            LOOT_TYPE_NONE_LIKE_CPP
        );
        assert_eq!(
            loot_type_for_client_like_cpp(LOOT_TYPE_CORPSE_LIKE_CPP),
            LOOT_TYPE_CORPSE_LIKE_CPP
        );
        assert_eq!(
            loot_type_for_client_like_cpp(LOOT_TYPE_ITEM_LIKE_CPP),
            LOOT_TYPE_ITEM_LIKE_CPP
        );
        assert_eq!(
            loot_type_for_client_like_cpp(LOOT_TYPE_GATHERING_NODE_LIKE_CPP),
            LOOT_TYPE_GATHERING_NODE_LIKE_CPP
        );
        assert_eq!(
            loot_type_for_client_like_cpp(LOOT_TYPE_CHEST_LIKE_CPP),
            LOOT_TYPE_CHEST_LIKE_CPP
        );
        assert_eq!(
            loot_type_for_client_like_cpp(LOOT_TYPE_CORPSE_PERSONAL_LIKE_CPP),
            LOOT_TYPE_CORPSE_PERSONAL_LIKE_CPP
        );
        assert_eq!(
            loot_type_for_client_like_cpp(LOOT_TYPE_PROSPECTING_LIKE_CPP),
            LOOT_TYPE_DISENCHANTING_LIKE_CPP
        );
        assert_eq!(
            loot_type_for_client_like_cpp(LOOT_TYPE_MILLING_LIKE_CPP),
            LOOT_TYPE_DISENCHANTING_LIKE_CPP
        );
        assert_eq!(
            loot_type_for_client_like_cpp(LOOT_TYPE_INSIGNIA_LIKE_CPP),
            LOOT_TYPE_SKINNING_LIKE_CPP
        );
        assert_eq!(
            loot_type_for_client_like_cpp(LOOT_TYPE_FISHINGHOLE_LIKE_CPP),
            LOOT_TYPE_FISHING_LIKE_CPP
        );
        assert_eq!(
            loot_type_for_client_like_cpp(LOOT_TYPE_FISHING_JUNK_LIKE_CPP),
            LOOT_TYPE_FISHING_LIKE_CPP
        );
    }

    #[tokio::test]
    async fn represented_loot_response_acquire_reason_uses_cpp_loot_type_mapping() {
        let mut session = make_session();
        let player_guid = ObjectGuid::create_player(1, 42);
        let owner_guid = test_creature_guid(19_096);
        let loot_guid = represented_loot_object_guid_like_cpp(owner_guid);
        let entry = represented_loot_entry(0, 25, player_guid);
        register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
        session.loot_table.insert(
            owner_guid,
            CreatureLoot {
                loot_guid,
                coins: 0,
                unlooted_count: 1,
                loot_type: LOOT_TYPE_PROSPECTING_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: vec![player_guid],
                items: vec![entry],
                looted_by_player: false,
            },
        );

        let response = session
            .represented_loot_response_for_owner_like_cpp(owner_guid, player_guid, false)
            .await
            .unwrap();

        assert_eq!(response.acquire_reason, LOOT_TYPE_DISENCHANTING_LIKE_CPP);
    }

    #[test]
    fn represented_start_loot_roll_carries_cpp_dungeon_encounter_id() {
        let player_guid = ObjectGuid::create_player(1, 42);
        let loot_obj = ObjectGuid::create_world_object(HighGuid::LootObject, 0, 1, 0, 0, 1, 900);
        let entry = represented_loot_entry(0, 25, player_guid);

        let packet = start_loot_roll_packet_like_cpp(
            loot_obj,
            571,
            LOOT_METHOD_GROUP_LIKE_CPP,
            &entry,
            ROLL_ALL_TYPE_NO_DISENCHANT_LIKE_CPP,
            615,
        );

        assert_eq!(packet.dungeon_encounter_id, 615);
    }

    #[test]
    fn represented_loot_item_push_result_carries_cpp_encounter_loot_fields() {
        let (session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let item_guid = ObjectGuid::create_item(1, 700);
        let entry = represented_loot_entry(0, 25, player_guid);

        session.send_loot_item_push_result(
            player_guid,
            item_guid,
            &entry,
            0,
            0,
            0,
            1,
            1,
            false,
            615,
        );

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::ItemPushResult as u16
        );
        assert_eq!(sent.read_packed_guid().unwrap(), player_guid);
        assert_eq!(sent.read_uint8().unwrap(), u8::from(INVENTORY_SLOT_BAG_0));
        assert_eq!(sent.read_int32().unwrap(), 0);
        assert_eq!(sent.read_int32().unwrap(), 0);
        assert_eq!(sent.read_int32().unwrap(), 1);
        assert_eq!(sent.read_int32().unwrap(), 1);
        assert_eq!(sent.read_int32().unwrap(), 615);
        assert_eq!(sent.read_int32().unwrap(), 0);
        assert_eq!(sent.read_int32().unwrap(), 0);
        assert_eq!(sent.read_uint32().unwrap(), 0);
        assert_eq!(sent.read_int32().unwrap(), 0);
        assert_eq!(sent.read_packed_guid().unwrap(), item_guid);
        assert!(!sent.read_bit().unwrap());
        assert!(!sent.read_bit().unwrap());
        assert_eq!(sent.read_bits(3).unwrap(), 2);
        assert!(!sent.read_bit().unwrap());
        assert!(sent.read_bit().unwrap());
        assert_eq!(sent.read_int32().unwrap(), 25);
    }

    #[test]
    fn creature_generated_loot_entry_uses_item_template_addon_follow_loot_rules_like_cpp() {
        let generated = GeneratedLootItem {
            item_id: 25,
            count: 1,
            loot_list_id: 7,
            random_properties_id: -77,
            random_properties_seed: 456,
            context: ItemContext::DungeonNormal as u8,
            free_for_all: false,
            follow_loot_rules: false,
            needs_quest: true,
            is_looted: false,
            is_blocked: false,
            is_under_threshold: false,
            is_counted: false,
        };

        let default_entry = generated_creature_loot_item_to_entry_like_cpp(
            generated,
            ItemTemplateAddonLootMetadataLikeCpp::default(),
        );
        assert!(!default_entry.flags.follow_loot_rules);
        assert_eq!(default_entry.loot_list_id, 7);
        assert_eq!(default_entry.random_properties_id, -77);
        assert_eq!(default_entry.random_properties_seed, 456);
        assert_eq!(default_entry.item_context, ItemContext::DungeonNormal as u8);

        let follow_entry = generated_creature_loot_item_to_entry_like_cpp(
            generated,
            ItemTemplateAddonLootMetadataLikeCpp {
                flags_cu: ITEM_FLAGS_CU_FOLLOW_LOOT_RULES_LIKE_CPP,
                quest_log_item_id: 0,
            },
        );
        assert!(follow_entry.flags.follow_loot_rules);
        assert!(follow_entry.flags.needs_quest);
    }

    #[test]
    fn represented_loot_response_items_use_cpp_ui_type_decision_tree() {
        let player_guid = ObjectGuid::create_player(1, 42);
        let other_guid = ObjectGuid::create_player(1, 77);
        let loot_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 100);

        let mut rolling_entry = represented_loot_entry(0, 25, player_guid);
        rolling_entry.flags.blocked = true;

        let mut won_entry = represented_loot_entry(1, 26, player_guid);
        won_entry.roll_winner = player_guid;

        let mut hidden_entry = represented_loot_entry(2, 27, player_guid);
        hidden_entry.roll_winner = other_guid;

        let mut allowed_entry = represented_loot_entry(3, 28, player_guid);
        allowed_entry.flags.under_threshold = true;

        let loot = CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![player_guid],
            items: vec![rolling_entry, won_entry, hidden_entry, allowed_entry],
            looted_by_player: false,
        };

        let items = represented_loot_response_items_like_cpp(&loot, player_guid);

        assert_eq!(items.len(), 3);
        assert_eq!(items[0].loot_list_id, 0);
        assert_eq!(items[0].ui_type, LOOT_SLOT_TYPE_ROLL_ONGOING_LIKE_CPP);
        assert_eq!(items[1].loot_list_id, 1);
        assert_eq!(items[1].ui_type, LOOT_SLOT_TYPE_OWNER_LIKE_CPP);
        assert_eq!(items[2].loot_list_id, 3);
        assert_eq!(items[2].ui_type, LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP);
    }

    #[test]
    fn represented_ffa_loot_uses_player_ffa_items_like_cpp() {
        let player_guid = ObjectGuid::create_player(1, 42);
        let other_guid = ObjectGuid::create_player(1, 77);
        let loot_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 101);

        let mut ffa_entry = represented_loot_entry(0, 25, player_guid);
        ffa_entry.flags.freeforall = true;
        ffa_entry.allowed_looters.clear();

        let mut loot = CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: vec![ffa_entry],
            looted_by_player: false,
        };

        mark_loot_allowed_for_player_like_cpp(&mut loot, player_guid);
        mark_loot_allowed_for_player_like_cpp(&mut loot, other_guid);
        assert_eq!(loot.unlooted_count, 2);

        let player_items = represented_loot_response_items_like_cpp(&loot, player_guid);
        let other_items = represented_loot_response_items_like_cpp(&loot, other_guid);
        assert_eq!(player_items.len(), 1);
        assert_eq!(other_items.len(), 1);
        assert_eq!(player_items[0].ui_type, LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP);
        assert_eq!(other_items[0].ui_type, LOOT_SLOT_TYPE_ALLOW_LOOT_LIKE_CPP);

        mark_loot_item_looted_for_player_like_cpp(&mut loot, 0, player_guid);
        assert_eq!(loot.unlooted_count, 1);

        let player_ffa = loot
            .player_ffa_items
            .iter()
            .find(|(player, _)| *player == player_guid)
            .and_then(|(_, items)| items.iter().find(|item| item.loot_list_id == 0))
            .unwrap();
        let other_ffa = loot
            .player_ffa_items
            .iter()
            .find(|(player, _)| *player == other_guid)
            .and_then(|(_, items)| items.iter().find(|item| item.loot_list_id == 0))
            .unwrap();

        assert!(player_ffa.is_looted);
        assert!(!other_ffa.is_looted);
        assert!(represented_loot_response_items_like_cpp(&loot, player_guid).is_empty());
        assert_eq!(
            represented_loot_response_items_like_cpp(&loot, other_guid).len(),
            1
        );
    }

    #[test]
    fn represented_unlooted_count_counts_shared_items_once_like_cpp() {
        let player_guid = ObjectGuid::create_player(1, 42);
        let other_guid = ObjectGuid::create_player(1, 77);
        let loot_guid = ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, 102);

        let mut entry = represented_loot_entry(0, 25, player_guid);
        entry.allowed_looters.clear();

        let mut loot = CreatureLoot {
            loot_guid,
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: vec![entry],
            looted_by_player: false,
        };

        mark_loot_allowed_for_player_like_cpp(&mut loot, player_guid);
        assert_eq!(loot.unlooted_count, 1);
        assert!(loot.items[0].flags.counted);

        mark_loot_allowed_for_player_like_cpp(&mut loot, other_guid);
        assert_eq!(loot.unlooted_count, 1);

        mark_loot_item_looted_for_player_like_cpp(&mut loot, 0, player_guid);
        assert_eq!(loot.unlooted_count, 0);
        mark_loot_item_looted_for_player_like_cpp(&mut loot, 0, player_guid);
        assert_eq!(loot.unlooted_count, 0);
    }

    #[test]
    fn represented_loot_removed_uses_players_looting_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let open_guid = ObjectGuid::create_player(1, 77);
        let closed_guid = ObjectGuid::create_player(1, 88);
        let owner_guid = test_creature_guid(19_095);
        let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
        let (open_tx, open_rx) = flume::bounded::<Vec<u8>>(1);
        let (closed_tx, closed_rx) = flume::bounded::<Vec<u8>>(1);
        let player_registry = Arc::new(PlayerRegistry::default());
        player_registry.insert(open_guid, broadcast_info(open_guid, open_tx));
        player_registry.insert(closed_guid, broadcast_info(closed_guid, closed_tx));
        session.set_player_registry(player_registry);
        session.set_player_guid(Some(player_guid));
        session.loot_table.insert(
            owner_guid,
            CreatureLoot {
                loot_guid: loot_object,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: vec![player_guid, open_guid],
                allowed_looters: vec![player_guid, open_guid, closed_guid],
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags::default(),
                    allowed_looters: vec![player_guid, open_guid, closed_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        session.represented_notify_loot_item_removed_like_cpp(owner_guid, 0);

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootRemoved as u16
        );
        assert_eq!(sent.read_packed_guid().unwrap(), owner_guid);
        assert_eq!(sent.read_packed_guid().unwrap(), loot_object);
        assert_eq!(sent.read_uint8().unwrap(), 0);

        let sent = open_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootRemoved as u16
        );
        assert!(closed_rx.try_recv().is_err());
    }

    fn loot_release_packet(object: ObjectGuid) -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_packed_guid(&object);
        pkt.reset_read();
        pkt
    }

    fn loot_money_packet() -> WorldPacket {
        let mut pkt = WorldPacket::new_empty();
        pkt.write_bit(false);
        pkt.flush_bits();
        pkt.reset_read();
        pkt
    }

    fn recv_packet_with_opcode(
        rx: &flume::Receiver<Vec<u8>>,
        opcode: wow_constants::ServerOpcodes,
    ) -> WorldPacket {
        for _ in 0..8 {
            let sent = rx.try_recv().unwrap();
            let mut packet = WorldPacket::from_bytes(&sent);
            if packet.read_uint16().unwrap() == opcode as u16 {
                return packet;
            }
        }
        panic!("expected packet opcode {:?}", opcode);
    }

    fn test_creature(guid: ObjectGuid, is_alive: bool) -> CreatureAI {
        let mut creature = CreatureAI::new(
            guid,
            1,
            Position::ZERO,
            100,
            1,
            1,
            2,
            0.0,
            1,
            35,
            0,
            0,
            0,
            0,
            0,
            None,
            0,
        );
        creature.is_alive = is_alive;
        creature
    }

    fn register_test_creature_like_cpp(session: &mut WorldSession, creature: CreatureAI) {
        if session.map_manager.is_none() {
            session.set_map_manager(Arc::new(RwLock::new(crate::map_manager::MapManager::new())));
        }

        let create_data = CreatureCreateData {
            guid: creature.guid,
            entry: creature.entry,
            display_id: creature.display_id,
            native_display_id: creature.display_id,
            health: i64::from(creature.hp.max(1)),
            max_health: i64::from(creature.max_hp.max(1)),
            level: creature.level,
            faction_template: creature.faction as i32,
            npc_flags: u64::from(creature.npc_flags),
            unit_flags: creature.unit_flags,
            unit_flags2: 0,
            unit_flags3: 0,
            damage_school: wow_constants::spell::SpellSchools::Normal as u8,
            scale: 1.0,
            unit_class: 1,
            base_attack_time: 2000,
            ranged_attack_time: 0,
            zone_id: 0,
            speed_walk_rate: 1.0,
            speed_run_rate: 1.14286,
            ai_anim_kit_id: 0,
            movement_anim_kit_id: 0,
            melee_anim_kit_id: 0,
        };
        let guid = creature.guid;
        let is_alive = creature.is_alive;
        session.register_world_creature(
            session.player_map_id_like_cpp(),
            creature.current_pos,
            create_data,
            creature.min_dmg,
            creature.max_dmg,
            creature.aggro_radius,
            creature.loot_id,
            0,
            creature.gold_min,
            creature.gold_max,
            creature.boss_id,
            creature.dungeon_encounter_id,
            0,
            0,
            0,
            -1,
        );
        if !is_alive {
            let _ = session.mutate_world_creature(guid, |world_creature| {
                world_creature.creature.mark_ai_dead(0);
            });
        }
    }

    fn insert_allowed_coin_loot_like_cpp(
        session: &mut WorldSession,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
        coins: u32,
    ) {
        session.loot_table.insert(
            owner_guid,
            CreatureLoot {
                loot_guid: represented_loot_object_guid_like_cpp(owner_guid),
                coins,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: vec![player_guid],
                items: Vec::new(),
                looted_by_player: false,
            },
        );
    }

    fn tap_test_creature_like_cpp(
        session: &mut WorldSession,
        creature_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) {
        let _ = session.mutate_world_creature(creature_guid, |world_creature| {
            world_creature
                .creature
                .set_tapped_by_player(player_guid, &[]);
        });
    }

    fn loot_response_failure_reason(sent: &[u8]) -> u8 {
        let mut pkt = WorldPacket::from_bytes(&sent[2..]);
        let _owner = pkt.read_packed_guid().unwrap();
        let _loot_obj = pkt.read_packed_guid().unwrap();
        pkt.read_uint8().unwrap()
    }

    fn test_creature_guid(counter: i64) -> ObjectGuid {
        ObjectGuid::create_world_object(HighGuid::Creature, 0, 1, 0, 0, 1, counter)
    }

    fn broadcast_info(guid: ObjectGuid, send_tx: flume::Sender<Vec<u8>>) -> PlayerBroadcastInfo {
        let (command_tx, _command_rx) = flume::bounded(1);
        PlayerBroadcastInfo {
            map_id: 0,
            instance_id: 0,
            position: Position::ZERO,
            combat_reach: 0.0,
            liquid_status: 0,
            is_in_world: true,
            send_tx,
            command_tx,
            active_loot_rolls: Vec::new(),
            pass_on_group_loot: false,
            enchanting_skill: 0,
            is_alive: true,
            unit_flags: 0,
            unit_flags2: 0,
            unit_state: 0,
            is_game_master: false,
            is_contested_pvp: false,
            active_expansion: 2,
            pending_quest_sharing: None,
            known_spells: Vec::new(),
            active_quest_statuses: Default::default(),
            active_quest_objective_counts: Default::default(),
            rewarded_quests: Default::default(),
            daily_quests_completed: Default::default(),
            df_quests: Default::default(),
            faction_template_id: 0,
            reputation_standings: Vec::new(),
            reputation_state_flags: Vec::new(),
            forced_reputation_ranks: Vec::new(),
            forced_reputation_faction_ids: Vec::new(),
            inventory_item_counts: Default::default(),
            party_member_phase_states: Default::default(),
            player_name: format!("Player{}", guid.counter()),
            account_id: guid.counter() as u32,
            recruiter_id: 0,
            race: 1,
            class: 1,
            sex: 0,
            level: 1,
            gray_level: 0,
            display_id: 49,
            visible_items: [(0, 0, 0); 19],
        }
    }

    fn loot_condition(
        condition_type_or_reference: i32,
        value1: u32,
        value2: u32,
        value3: u32,
    ) -> LootConditionRowLikeCpp {
        LootConditionRowLikeCpp {
            else_group: 0,
            condition_type_or_reference,
            condition_target: 0,
            value1,
            value2,
            value3,
            string_value1: String::new(),
            negative: false,
            script_name: String::new(),
        }
    }

    fn test_quest_template(id: u32) -> QuestTemplate {
        QuestTemplate {
            id,
            quest_type: 0,
            quest_level: 1,
            quest_max_scaling_level: 0,
            quest_package_id: 0,
            min_level: 1,
            quest_sort_id: 0,
            quest_info_id: 0,
            suggested_group_num: 0,
            reward_next_quest: 0,
            reward_xp_difficulty: 0,
            reward_xp_multiplier: 1.0,
            reward_money_difficulty: 0,
            reward_money_multiplier: 1.0,
            reward_bonus_money: 0,
            reward_display_spell: [0; 3],
            reward_spell: 0,
            reward_honor: 0,
            reward_title_id: 0,
            reward_skill_line_id: 0,
            reward_skill_points: 0,
            reward_mail_template_id: 0,
            reward_mail_delay_secs: 0,
            reward_mail_sender_entry: 0,
            reward_faction_ids: [0; QUEST_REWARD_REPUTATIONS_COUNT],
            reward_faction_values: [0; QUEST_REWARD_REPUTATIONS_COUNT],
            reward_faction_overrides: [0; QUEST_REWARD_REPUTATIONS_COUNT],
            reward_faction_cap_in: [0; QUEST_REWARD_REPUTATIONS_COUNT],
            reward_faction_flags: 0,
            source_item_id: 0,
            source_item_count: 0,
            source_spell_id: 0,
            limit_time_secs: 0,
            expansion: 0,
            flags: 0,
            flags_ex: 0,
            flags_ex2: 0,
            special_flags: 0,
            event_id_for_quest: 0,
            reward_items: [0; 4],
            reward_amounts: [0; 4],
            reward_currencies: [0; 4],
            reward_currency_amounts: [0; 4],
            item_drop: [0; 4],
            item_drop_quantity: [0; 4],
            log_title: String::new(),
            log_description: String::new(),
            quest_description: String::new(),
            area_description: String::new(),
            quest_completion_log: String::new(),
            objectives: Vec::new(),
            allowable_races: 0,
            allowable_classes: 0,
            max_level: 0,
            prev_quest_id: 0,
            next_quest_id: 0,
            exclusive_group: 0,
            breadcrumb_for_quest_id: 0,
            dependent_previous_quests: Vec::new(),
            dependent_breadcrumb_quests: Vec::new(),
            required_min_rep_faction: 0,
            required_min_rep_value: 0,
            required_max_rep_faction: 0,
            required_max_rep_value: 0,
            required_skill_id: 0,
            required_skill_points: 0,
            reward_choice_items: [(0, 0); 6],
            reward_choice_item_types: [0; 6],
        }
    }

    #[test]
    fn represented_personal_loot_remote_context_uses_registry_fields_like_cpp() {
        let (session, _) = make_session_with_send_capacity(1);
        let remote_context = RepresentedLootPlayerContext {
            race: 1,
            class: 1,
            gender: 0,
            level: 80,
            known_spells: Vec::new(),
            active_quest_statuses: HashMap::new(),
            active_quest_objective_counts: HashMap::new(),
            rewarded_quests: HashSet::new(),
            inventory_item_counts: HashMap::new(),
            is_current: false,
        };

        assert_eq!(
            session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
                &loot_condition(6, 469, 0, 0),
                &remote_context,
            ),
            Some(true)
        );
        assert_eq!(
            session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
                &loot_condition(15, 1, 0, 0),
                &remote_context,
            ),
            Some(true)
        );
        assert_eq!(
            session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
                &loot_condition(16, 1, 0, 0),
                &remote_context,
            ),
            Some(true)
        );
        assert_eq!(
            session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
                &loot_condition(20, 0, 0, 0),
                &remote_context,
            ),
            Some(true)
        );
        assert_eq!(
            session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
                &loot_condition(27, 70, 3, 0),
                &remote_context,
            ),
            Some(true)
        );
    }

    #[test]
    fn represented_personal_loot_remote_quest_and_spell_conditions_use_registry_like_cpp() {
        let (session, _) = make_session_with_send_capacity(1);
        let mut active_quest_statuses = HashMap::new();
        active_quest_statuses.insert(100, 1);
        active_quest_statuses.insert(200, 2);
        let mut rewarded_quests = HashSet::new();
        rewarded_quests.insert(300);
        let remote_context = RepresentedLootPlayerContext {
            race: 1,
            class: 1,
            gender: 0,
            level: 80,
            known_spells: vec![12_345],
            active_quest_statuses,
            active_quest_objective_counts: HashMap::new(),
            rewarded_quests,
            inventory_item_counts: HashMap::new(),
            is_current: false,
        };

        assert_eq!(
            session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
                &loot_condition(9, 100, 0, 0),
                &remote_context,
            ),
            Some(true)
        );
        assert_eq!(
            session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
                &loot_condition(28, 200, 0, 0),
                &remote_context,
            ),
            Some(true)
        );
        assert_eq!(
            session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
                &loot_condition(8, 300, 0, 0),
                &remote_context,
            ),
            Some(true)
        );
        assert_eq!(
            session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
                &loot_condition(14, 400, 0, 0),
                &remote_context,
            ),
            Some(true)
        );
        assert_eq!(
            session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
                &loot_condition(25, 12_345, 0, 0),
                &remote_context,
            ),
            Some(true)
        );
        assert_eq!(
            session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
                &loot_condition(47, 100, 0x08, 0),
                &remote_context,
            ),
            Some(true)
        );
    }

    #[test]
    fn represented_personal_loot_remote_inventory_and_objective_conditions_use_registry_like_cpp() {
        let (mut session, _) = make_session_with_send_capacity(1);
        let mut quest_store = QuestStore::new();
        let mut quest = test_quest_template(100);
        quest.objectives.push(QuestObjective {
            id: 11,
            quest_id: 100,
            obj_type: 1,
            order: 0,
            storage_index: 0,
            object_id: 7001,
            amount: 7,
            flags: 0,
            flags2: 0,
            progress_bar_weight: 0.0,
            description: String::new(),
        });
        quest_store.quests.insert(100, quest);
        session.set_quest_store(Arc::new(quest_store));
        let mut active_quest_statuses = HashMap::new();
        active_quest_statuses.insert(100, 1);
        let mut active_quest_objective_counts = HashMap::new();
        active_quest_objective_counts.insert(100, vec![5]);
        let mut inventory_item_counts = HashMap::new();
        inventory_item_counts.insert(9001, 2);
        let remote_context = RepresentedLootPlayerContext {
            race: 1,
            class: 1,
            gender: 0,
            level: 80,
            known_spells: Vec::new(),
            active_quest_statuses,
            active_quest_objective_counts,
            rewarded_quests: HashSet::new(),
            inventory_item_counts,
            is_current: false,
        };

        assert_eq!(
            session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
                &loot_condition(2, 9001, 2, 0),
                &remote_context,
            ),
            Some(true)
        );
        assert_eq!(
            session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
                &loot_condition(2, 9001, 3, 0),
                &remote_context,
            ),
            Some(false)
        );
        assert_eq!(
            session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
                &loot_condition(2, 9001, 2, 1),
                &remote_context,
            ),
            None
        );
        assert_eq!(
            session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
                &loot_condition(48, 11, 0, 5),
                &remote_context,
            ),
            Some(true)
        );
        assert_eq!(
            session.evaluate_creature_loot_condition_for_player_like_cpp_representable(
                &loot_condition(48, 11, 0, 4),
                &remote_context,
            ),
            Some(false)
        );
    }

    #[test]
    fn represented_personal_loot_remote_has_quest_for_item_objective_like_cpp() {
        let (mut session, _) = make_session_with_send_capacity(1);
        install_limited_test_item_template(&mut session, 7001, 0);
        let mut quest_store = QuestStore::new();
        let mut quest = test_quest_template(100);
        quest.objectives.push(QuestObjective {
            id: 1,
            quest_id: 100,
            obj_type: 1,
            order: 0,
            storage_index: 0,
            object_id: 7001,
            amount: 3,
            flags: 0,
            flags2: 0,
            progress_bar_weight: 0.0,
            description: String::new(),
        });
        quest_store.quests.insert(100, quest);
        session.set_quest_store(Arc::new(quest_store));

        let mut active_quest_statuses = HashMap::new();
        active_quest_statuses.insert(100, 1);
        let mut active_quest_objective_counts = HashMap::new();
        active_quest_objective_counts.insert(100, vec![2]);
        let mut remote_context = RepresentedLootPlayerContext {
            race: 1,
            class: 1,
            gender: 0,
            level: 80,
            known_spells: Vec::new(),
            active_quest_statuses,
            active_quest_objective_counts,
            rewarded_quests: HashSet::new(),
            inventory_item_counts: HashMap::new(),
            is_current: false,
        };

        assert!(session.item_loot_quest_status_allows_for_player_like_cpp(
            7001,
            true,
            ItemTemplateAddonLootMetadataLikeCpp::default(),
            &remote_context,
        ));

        remote_context
            .active_quest_objective_counts
            .insert(100, vec![3]);
        assert!(!session.item_loot_quest_status_allows_for_player_like_cpp(
            7001,
            true,
            ItemTemplateAddonLootMetadataLikeCpp::default(),
            &remote_context,
        ));
    }

    #[test]
    fn represented_personal_loot_remote_has_quest_for_item_drop_like_cpp() {
        let (mut session, _) = make_session_with_send_capacity(1);
        install_limited_test_item_template(&mut session, 7002, 0);
        let mut quest_store = QuestStore::new();
        let mut quest = test_quest_template(200);
        quest.item_drop[0] = 7002;
        quest.item_drop_quantity[0] = 4;
        quest_store.quests.insert(200, quest);
        session.set_quest_store(Arc::new(quest_store));

        let mut active_quest_statuses = HashMap::new();
        active_quest_statuses.insert(200, 1);
        let mut inventory_item_counts = HashMap::new();
        inventory_item_counts.insert(7002, 3);
        let mut remote_context = RepresentedLootPlayerContext {
            race: 1,
            class: 1,
            gender: 0,
            level: 80,
            known_spells: Vec::new(),
            active_quest_statuses,
            active_quest_objective_counts: HashMap::new(),
            rewarded_quests: HashSet::new(),
            inventory_item_counts,
            is_current: false,
        };

        assert!(session.item_loot_quest_status_allows_for_player_like_cpp(
            7002,
            true,
            ItemTemplateAddonLootMetadataLikeCpp::default(),
            &remote_context,
        ));

        remote_context.inventory_item_counts.insert(7002, 4);
        assert!(!session.item_loot_quest_status_allows_for_player_like_cpp(
            7002,
            true,
            ItemTemplateAddonLootMetadataLikeCpp::default(),
            &remote_context,
        ));
    }

    fn install_master_loot_group(
        session: &mut WorldSession,
        master_guid: ObjectGuid,
        candidate_guid: ObjectGuid,
    ) {
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(master_guid);
        group.add_member(candidate_guid);
        group.loot_method = LOOT_METHOD_MASTER_LIKE_CPP;
        group.master_looter_guid = master_guid;
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);
        session.group_guid = Some(group_guid);
        session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    }

    fn install_group_loot_group(
        session: &mut WorldSession,
        leader_guid: ObjectGuid,
        candidate_guid: ObjectGuid,
    ) {
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(leader_guid);
        group.add_member(candidate_guid);
        group.loot_method = LOOT_METHOD_GROUP_LIKE_CPP;
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);
        session.group_guid = Some(group_guid);
        session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
    }

    fn install_limited_test_item_template(session: &mut WorldSession, entry: u32, max_count: i32) {
        install_limited_test_item_template_with_flags2(session, entry, max_count, 0);
    }

    fn install_limited_test_item_template_with_flags2(
        session: &mut WorldSession,
        entry: u32,
        max_count: i32,
        flags2: u32,
    ) {
        session.set_item_store(Arc::new(ItemStore::from_records([ItemRecord {
            id: entry,
            class_id: ItemClass::Consumable as u8,
            subclass_id: 0,
            material: 0,
            inventory_type: InventoryType::NonEquip as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
        }])));
        session.set_item_stats_store(Arc::new(ItemStatsStore::from_sparse_templates([(
            entry,
            ItemSparseTemplateEntry {
                flags: [0, flags2, 0, 0],
                bag_family: 0,
                start_quest_id: 0,
                stackable: 20,
                max_count,
                lock_id: 0,
                required_reputation_rank: 0,
                sell_price: 0,
                buy_price: 0,
                vendor_stack_count: 1,
                price_variance: 1.0,
                price_random_value: 1.0,
                max_durability: 0,
                limit_category: 0,
                instance_bound: 0,
                zone_bound: [0, 0],
                required_reputation_faction: 0,
                allowable_class: -1,
                required_expansion: 0,
                bonding: ItemBondingType::None as u8,
                container_slots: 0,
                inventory_type: InventoryType::NonEquip as i8,
            },
        )])));
    }

    fn install_disenchantable_test_item_template(session: &mut WorldSession, entry: u32) {
        session.set_item_store(Arc::new(ItemStore::from_records([ItemRecord {
            id: entry,
            class_id: ItemClass::Armor as u8,
            subclass_id: 0,
            material: 0,
            inventory_type: InventoryType::Chest as i8,
            sheathe_type: 0,
            random_select: 0,
            random_suffix_group_id: 0,
        }])));
        session.set_item_stats_store(Arc::new(
            ItemStatsStore::from_sparse_and_random_property_templates(
                [(
                    entry,
                    ItemSparseTemplateEntry {
                        flags: [0, 0, 0, 0],
                        bag_family: 0,
                        start_quest_id: 0,
                        stackable: 1,
                        max_count: 0,
                        lock_id: 0,
                        required_reputation_rank: 0,
                        sell_price: 1,
                        buy_price: 0,
                        vendor_stack_count: 1,
                        price_variance: 1.0,
                        price_random_value: 1.0,
                        max_durability: 0,
                        limit_category: 0,
                        instance_bound: 0,
                        zone_bound: [0, 0],
                        required_reputation_faction: 0,
                        allowable_class: -1,
                        required_expansion: 0,
                        bonding: ItemBondingType::None as u8,
                        container_slots: 0,
                        inventory_type: InventoryType::Chest as i8,
                    },
                )],
                [(
                    entry,
                    ItemRandomPropertyTemplateEntry {
                        item_level: 10,
                        quality: ItemQuality::Rare as i8,
                        inventory_type: InventoryType::Chest as i8,
                    },
                )],
            ),
        ));
        session.set_item_disenchant_loot_store(Arc::new(ItemDisenchantLootStore::from_entries([
            ItemDisenchantLootEntry {
                id: 901,
                subclass: 0,
                quality: ItemQuality::Rare as u8,
                min_level: 1,
                max_level: 20,
                skill_required: 175,
                expansion_id: -2,
                class_id: ItemClass::Armor as u32,
            },
        ])));
    }

    fn install_active_spell_cast(session: &mut WorldSession, player_guid: ObjectGuid) {
        session.active_spell_cast = Some(SpellCastState {
            spell_id: 133,
            target_guid: player_guid,
            target_data: wow_packet::packets::spell::SpellTargetData {
                flags: 0x2,
                unit: player_guid,
                ..Default::default()
            },
            cast_id: ObjectGuid::create_world_object(HighGuid::Cast, 0, 1, 0, 0, 1, 7),
            cast_start_time: std::time::Instant::now(),
            cast_time_ms: 30_000,
            spell_visual: wow_packet::packets::spell::SpellCastVisual {
                spell_visual_id: 1,
                script_visual_id: 0,
            },
            metadata: crate::session::SpellCastMetadata::default(),
        });
    }

    fn install_visible_aura_with_interrupt_flags(
        session: &mut WorldSession,
        slot: u8,
        spell_id: i32,
        caster_guid: ObjectGuid,
        aura_interrupt_flags: u32,
    ) {
        session.visible_auras.insert(
            slot,
            AuraApplication {
                spell_id,
                caster_guid,
                slot,
                duration_total: 30_000,
                duration_remaining: 30_000,
                stack_count: 1,
                aura_flags: 0x0000_0001,
                aura_interrupt_flags,
                aura_interrupt_flags2: 0,
                represented_effect: None,
                represented_amount: 0,
                represented_misc_value: None,
                represented_multiplier: 1.0,
                applied_at: std::time::Instant::now(),
            },
        );
    }

    #[tokio::test]
    async fn represented_creature_money_uses_cpp_money_drop_rate() {
        let mut session = make_session();
        session.set_loot_drop_rates_like_cpp(LootDropRatesLikeCpp {
            money: 2.5,
            ..LootDropRatesLikeCpp::default()
        });

        let loot = session
            .generate_represented_creature_loot_like_cpp(
                test_creature_guid(1),
                ObjectGuid::create_player(1, 42),
                10,
                25,
                0,
                100,
                100,
                0,
            )
            .await;

        assert_eq!(loot.coins, 250);
        assert!(loot.items.is_empty());
    }

    #[tokio::test]
    async fn represented_creature_loot_generation_carries_cpp_dungeon_encounter_id() {
        let session = make_session();

        let loot = session
            .generate_represented_creature_loot_like_cpp(
                test_creature_guid(19_097),
                ObjectGuid::create_player(1, 42),
                10,
                25,
                0,
                0,
                0,
                615,
            )
            .await;

        assert_eq!(loot.dungeon_encounter_id, 615);
    }

    #[tokio::test]
    async fn represented_gameobject_chest_loot_carries_cpp_source_metadata() {
        let mut session = make_session();
        let source = GameObjectLootSource {
            loot_id: 0,
            use_group_loot_rules: false,
            dungeon_encounter_id: 733,
            personal_loot_id: 10_001,
            push_loot_id: 0,
            triggered_event_id: 0,
            linked_trap_entry: 0,
            ..Default::default()
        };

        let loot = session
            .generate_represented_gameobject_chest_loot_like_cpp(
                test_gameobject_guid(91_001),
                ObjectGuid::create_player(1, 42),
                source,
            )
            .await;

        assert_eq!(loot.loot_type, LOOT_TYPE_CHEST_LIKE_CPP);
        assert_eq!(loot.dungeon_encounter_id, 733);
        assert_eq!(loot.loot_method, 0);
    }

    #[tokio::test]
    async fn represented_gameobject_personal_encounter_loot_uses_current_player_when_no_tap_list_like_cpp()
     {
        let mut session = make_session();
        let player_guid = ObjectGuid::create_player(1, 42);
        let gameobject_guid = test_gameobject_guid(91_008);
        let source = GameObjectLootSource {
            loot_id: 0,
            use_group_loot_rules: false,
            dungeon_encounter_id: 733,
            personal_loot_id: 10_001,
            push_loot_id: 0,
            triggered_event_id: 0,
            linked_trap_entry: 0,
            ..Default::default()
        };

        let loot = session
            .generate_represented_gameobject_chest_loot_like_cpp(
                gameobject_guid,
                player_guid,
                source,
            )
            .await;

        assert_eq!(loot.allowed_looters, vec![player_guid]);
        assert!(
            loot.items
                .iter()
                .all(|entry| entry.allowed_looters == vec![player_guid])
        );
        assert_eq!(loot.coins, 0);
        assert!(
            session
                .represented_personal_loot_owners
                .contains(&gameobject_guid)
        );
        assert!(
            session
                .represented_personal_loot_money
                .contains_key(&(gameobject_guid, player_guid))
        );
    }

    #[tokio::test]
    async fn represented_gameobject_personal_encounter_loot_uses_tap_list_like_cpp() {
        let mut session = make_session();
        let first_tapper = ObjectGuid::create_player(1, 42);
        let second_tapper = ObjectGuid::create_player(1, 77);
        let non_player_tapper = ObjectGuid::create_item(1, 900);
        let gameobject_guid = test_gameobject_guid(91_009);
        session.represented_gameobject_tap_lists.insert(
            gameobject_guid,
            vec![
                second_tapper,
                non_player_tapper,
                first_tapper,
                second_tapper,
            ],
        );
        let source = GameObjectLootSource {
            loot_id: 0,
            use_group_loot_rules: false,
            dungeon_encounter_id: 733,
            personal_loot_id: 10_001,
            push_loot_id: 0,
            triggered_event_id: 0,
            linked_trap_entry: 0,
            ..Default::default()
        };

        let loot = session
            .generate_represented_gameobject_chest_loot_like_cpp(
                gameobject_guid,
                first_tapper,
                source,
            )
            .await;

        assert_eq!(loot.allowed_looters, vec![first_tapper, second_tapper]);
        assert!(
            loot.items
                .iter()
                .all(|entry| entry.allowed_looters == vec![first_tapper, second_tapper])
        );
        assert_eq!(loot.coins, 0);
        assert!(
            session
                .represented_personal_loot_owners
                .contains(&gameobject_guid)
        );
        assert!(
            session
                .represented_personal_loot_money
                .contains_key(&(gameobject_guid, first_tapper))
        );
        assert!(
            session
                .represented_personal_loot_money
                .contains_key(&(gameobject_guid, second_tapper))
        );
    }

    #[tokio::test]
    async fn represented_gameobject_personal_encounter_loot_skips_locked_tappers_like_cpp() {
        let mut session = make_session();
        let locked_tapper = ObjectGuid::create_player(1, 42);
        let open_tapper = ObjectGuid::create_player(1, 77);
        let gameobject_guid = test_gameobject_guid(91_010);
        session
            .represented_gameobject_tap_lists
            .insert(gameobject_guid, vec![locked_tapper, open_tapper]);
        session
            .represented_locked_dungeon_encounters
            .insert((locked_tapper, 733));
        let source = GameObjectLootSource {
            loot_id: 0,
            use_group_loot_rules: false,
            dungeon_encounter_id: 733,
            personal_loot_id: 10_001,
            push_loot_id: 0,
            triggered_event_id: 0,
            linked_trap_entry: 0,
            ..Default::default()
        };

        let loot = session
            .generate_represented_gameobject_chest_loot_like_cpp(
                gameobject_guid,
                locked_tapper,
                source,
            )
            .await;

        assert_eq!(loot.allowed_looters, vec![open_tapper]);
        assert!(
            loot.items
                .iter()
                .all(|entry| entry.allowed_looters == vec![open_tapper])
        );
    }

    #[tokio::test]
    async fn represented_gameobject_personal_encounter_open_does_not_auto_allow_non_tapper_like_cpp()
     {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let other_tapper = ObjectGuid::create_player(1, 77);
        let gameobject_guid = test_gameobject_guid(91_011);
        session.set_player_guid(Some(player_guid));
        session
            .client_visible_guids_like_cpp
            .insert(gameobject_guid);
        session
            .represented_gameobject_tap_lists
            .insert(gameobject_guid, vec![other_tapper]);
        let source = GameObjectLootSource {
            loot_id: 0,
            use_group_loot_rules: false,
            dungeon_encounter_id: 733,
            personal_loot_id: 10_001,
            push_loot_id: 0,
            triggered_event_id: 0,
            linked_trap_entry: 0,
            ..Default::default()
        };

        session
            .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
            .await;

        assert!(send_rx.try_recv().is_err());
        assert!(!session.is_active_loot_guid(gameobject_guid));
        assert_eq!(
            session
                .loot_table
                .get(&gameobject_guid)
                .unwrap()
                .allowed_looters,
            vec![other_tapper]
        );
    }

    #[tokio::test]
    async fn represented_gameobject_personal_encounter_open_reads_player_money_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(4);
        let player_guid = ObjectGuid::create_player(1, 42);
        let gameobject_guid = test_gameobject_guid(91_012);
        let loot_object = represented_loot_object_guid_like_cpp(gameobject_guid);
        session.set_player_guid(Some(player_guid));
        session
            .client_visible_guids_like_cpp
            .insert(gameobject_guid);
        session.loot_table.insert(
            gameobject_guid,
            CreatureLoot {
                loot_guid: loot_object,
                coins: 999,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
                dungeon_encounter_id: 733,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: vec![player_guid],
                items: Vec::new(),
                looted_by_player: false,
            },
        );
        session
            .represented_personal_loot_owners
            .insert(gameobject_guid);
        session
            .represented_personal_loot_money
            .insert((gameobject_guid, player_guid), 123);
        let source = GameObjectLootSource {
            loot_id: 0,
            use_group_loot_rules: false,
            dungeon_encounter_id: 733,
            personal_loot_id: 10_001,
            push_loot_id: 0,
            triggered_event_id: 0,
            linked_trap_entry: 0,
            ..Default::default()
        };

        session
            .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
            .await;

        let mut response =
            recv_packet_with_opcode(&send_rx, wow_constants::ServerOpcodes::LootResponse);
        assert_eq!(response.read_packed_guid().unwrap(), gameobject_guid);
        assert_eq!(response.read_packed_guid().unwrap(), loot_object);
        assert_eq!(response.read_uint8().unwrap(), 0);
        assert_eq!(response.read_uint8().unwrap(), LOOT_TYPE_CHEST_LIKE_CPP);
        assert_eq!(response.read_uint8().unwrap(), 0);
        assert_eq!(response.read_uint8().unwrap(), 2);
        assert_eq!(response.read_uint32().unwrap(), 123);
    }

    #[tokio::test]
    async fn represented_gameobject_personal_encounter_money_pickup_consumes_only_player_like_cpp()
    {
        let (mut session, send_rx) = make_session_with_send_capacity(4);
        let player_guid = ObjectGuid::create_player(1, 42);
        let other_tapper = ObjectGuid::create_player(1, 77);
        let gameobject_guid = test_gameobject_guid(91_013);
        let loot_object = represented_loot_object_guid_like_cpp(gameobject_guid);
        session.set_player_guid(Some(player_guid));
        session.set_active_loot_guid(gameobject_guid);
        session.loot_table.insert(
            gameobject_guid,
            CreatureLoot {
                loot_guid: loot_object,
                coins: 999,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
                dungeon_encounter_id: 733,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: vec![player_guid, other_tapper],
                items: Vec::new(),
                looted_by_player: false,
            },
        );
        session
            .represented_personal_loot_owners
            .insert(gameobject_guid);
        session
            .represented_personal_loot_money
            .insert((gameobject_guid, player_guid), 123);
        session
            .represented_personal_loot_money
            .insert((gameobject_guid, other_tapper), 456);

        session.handle_loot_money(loot_money_packet()).await;

        let mut notify =
            recv_packet_with_opcode(&send_rx, wow_constants::ServerOpcodes::LootMoneyNotify);
        assert_eq!(notify.read_uint64().unwrap(), 123);
        assert_eq!(
            session
                .represented_personal_loot_money
                .get(&(gameobject_guid, player_guid)),
            Some(&0)
        );
        assert_eq!(
            session
                .represented_personal_loot_money
                .get(&(gameobject_guid, other_tapper)),
            Some(&456)
        );
        assert_eq!(session.loot_table.get(&gameobject_guid).unwrap().coins, 999);
    }

    #[test]
    fn represented_gameobject_personal_encounter_items_are_single_tapper_like_cpp() {
        let first_tapper = ObjectGuid::create_player(1, 42);
        let second_tapper = ObjectGuid::create_player(1, 77);
        let mut loot = CreatureLoot {
            loot_guid: represented_loot_object_guid_like_cpp(test_gameobject_guid(91_014)),
            coins: 0,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
            dungeon_encounter_id: 733,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: vec![first_tapper, second_tapper],
            items: vec![
                LootEntry {
                    loot_list_id: 0,
                    item_id: 1_001,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags::default(),
                    allowed_looters: vec![first_tapper, second_tapper],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                },
                LootEntry {
                    loot_list_id: 1,
                    item_id: 1_002,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags {
                        freeforall: true,
                        ..LootEntryFlags::default()
                    },
                    allowed_looters: vec![first_tapper, second_tapper],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: vec![first_tapper],
                    taken: false,
                },
            ],
            looted_by_player: false,
        };
        let mut rng = StdRng::seed_from_u64(7);

        assign_represented_personal_loot_items_like_cpp(
            &mut loot,
            &[first_tapper, second_tapper],
            &mut rng,
        );

        assert_eq!(loot.unlooted_count, 2);
        assert_eq!(loot.items[0].allowed_looters.len(), 1);
        assert_eq!(loot.items[1].allowed_looters.len(), 1);
        assert!([first_tapper, second_tapper].contains(&loot.items[0].allowed_looters[0]));
        assert!([first_tapper, second_tapper].contains(&loot.items[1].allowed_looters[0]));
        assert!(loot.items[0].flags.counted);
        assert!(!loot.items[1].flags.counted);
        assert_eq!(loot.items[1].ffa_looted_by, Vec::<ObjectGuid>::new());
        assert_eq!(loot.player_ffa_items.len(), 1);
        assert_eq!(loot.player_ffa_items[0].1[0].loot_list_id, 1);
    }

    #[tokio::test]
    async fn represented_gameobject_chest_first_generation_records_use_effects_like_cpp() {
        let mut session = make_session();
        let player_guid = ObjectGuid::create_player(1, 42);
        let gameobject_guid = test_gameobject_guid(91_002);
        session.set_player_guid(Some(player_guid));
        session
            .client_visible_guids_like_cpp
            .insert(gameobject_guid);

        let source = GameObjectLootSource {
            loot_id: 55,
            use_group_loot_rules: false,
            dungeon_encounter_id: 0,
            personal_loot_id: 0,
            push_loot_id: 0,
            triggered_event_id: 777,
            linked_trap_entry: 888,
            ..Default::default()
        };

        session
            .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
            .await;
        session
            .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
            .await;

        assert_eq!(
            session.represented_gameobject_use_effects,
            vec![
                RepresentedGameObjectUseEffect::TriggerGameEvent {
                    gameobject_guid,
                    player_guid,
                    event_id: 777,
                },
                RepresentedGameObjectUseEffect::TriggerLinkedTrap {
                    gameobject_guid,
                    player_guid,
                    trap_entry: 888,
                },
            ]
        );
    }

    #[tokio::test]
    async fn represented_gameobject_chest_push_unique_use_records_effects_like_cpp() {
        let mut session = make_session();
        let player_guid = ObjectGuid::create_player(1, 42);
        let gameobject_guid = test_gameobject_guid(91_004);
        session.set_player_guid(Some(player_guid));
        session
            .client_visible_guids_like_cpp
            .insert(gameobject_guid);

        let source = GameObjectLootSource {
            loot_id: 0,
            use_group_loot_rules: false,
            dungeon_encounter_id: 0,
            personal_loot_id: 0,
            push_loot_id: 99,
            triggered_event_id: 321,
            linked_trap_entry: 654,
            ..Default::default()
        };

        session
            .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
            .await;
        session
            .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
            .await;

        assert_eq!(
            session.represented_gameobject_use_effects,
            vec![
                RepresentedGameObjectUseEffect::TriggerGameEvent {
                    gameobject_guid,
                    player_guid,
                    event_id: 321,
                },
                RepresentedGameObjectUseEffect::TriggerLinkedTrap {
                    gameobject_guid,
                    player_guid,
                    trap_entry: 654,
                },
            ]
        );
    }

    #[tokio::test]
    async fn represented_gameobject_chest_no_loot_unique_use_records_effects_like_cpp() {
        let mut session = make_session();
        let player_guid = ObjectGuid::create_player(1, 42);
        let gameobject_guid = test_gameobject_guid(91_005);
        session.set_player_guid(Some(player_guid));
        session
            .client_visible_guids_like_cpp
            .insert(gameobject_guid);

        let source = GameObjectLootSource {
            loot_id: 0,
            use_group_loot_rules: false,
            dungeon_encounter_id: 0,
            personal_loot_id: 0,
            push_loot_id: 0,
            triggered_event_id: 901,
            linked_trap_entry: 902,
            ..Default::default()
        };

        session
            .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
            .await;
        session
            .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
            .await;

        assert_eq!(
            session.represented_gameobject_use_effects,
            vec![
                RepresentedGameObjectUseEffect::TriggerGameEvent {
                    gameobject_guid,
                    player_guid,
                    event_id: 901,
                },
                RepresentedGameObjectUseEffect::TriggerLinkedTrap {
                    gameobject_guid,
                    player_guid,
                    trap_entry: 902,
                },
            ]
        );
    }

    #[tokio::test]
    async fn represented_gameobject_chest_use_sets_activated_loot_state_like_cpp() {
        let mut session = make_session();
        let player_guid = ObjectGuid::create_player(1, 42);
        let gameobject_guid = test_gameobject_guid(91_006);
        session.set_player_guid(Some(player_guid));
        session
            .client_visible_guids_like_cpp
            .insert(gameobject_guid);

        session
            .open_represented_gameobject_chest_like_cpp(
                gameobject_guid,
                GameObjectLootSource::default(),
            )
            .await;

        let state = session
            .represented_gameobject_use_states
            .get(&gameobject_guid)
            .expect("represented chest use records GO loot state");
        assert_eq!(state.loot_state, Some(LootState::Activated));
        assert_eq!(state.loot_state_unit_guid, player_guid);
    }

    #[tokio::test]
    async fn represented_chest_use_syncs_state_to_same_map_viewers_like_cpp() {
        let mut session = make_session();
        let player_guid = ObjectGuid::create_player(1, 42);
        let same_map_guid = ObjectGuid::create_player(1, 77);
        let other_map_guid = ObjectGuid::create_player(1, 88);
        let gameobject_guid = test_gameobject_guid(91_010);
        let (same_command_tx, same_command_rx) = flume::bounded(2);
        let (other_command_tx, other_command_rx) = flume::bounded(2);
        let (same_send_tx, _same_send_rx) = flume::bounded::<Vec<u8>>(1);
        let (other_send_tx, _other_send_rx) = flume::bounded::<Vec<u8>>(1);
        let player_registry = Arc::new(PlayerRegistry::default());
        let mut same_info = broadcast_info(same_map_guid, same_send_tx);
        same_info.map_id = 571;
        same_info.command_tx = same_command_tx;
        player_registry.insert(same_map_guid, same_info);
        let mut other_info = broadcast_info(other_map_guid, other_send_tx);
        other_info.map_id = 1;
        other_info.command_tx = other_command_tx;
        player_registry.insert(other_map_guid, other_info);
        let source = GameObjectLootSource {
            loot_id: 190_010,
            personal_loot_id: 190_011,
            push_loot_id: 190_012,
            chest_restock_time_secs: 30,
            chest_consumable: false,
            chest_quest_id: 777,
            linked_trap_entry: 190_013,
            ..Default::default()
        };

        session.set_player_guid(Some(player_guid));
        session.set_player_map_position_like_cpp(571, Position::ZERO);
        session.set_player_registry(player_registry);
        session
            .client_visible_guids_like_cpp
            .insert(gameobject_guid);

        session
            .open_represented_gameobject_chest_like_cpp(gameobject_guid, source)
            .await;

        let command = match same_command_rx.try_recv() {
            Ok(SessionCommand::SyncChestGameobjectStateAndRefreshLikeCpp(command)) => command,
            other => panic!("expected chest sync command, got {other:?}"),
        };
        assert_eq!(command.gameobject_guid, gameobject_guid);
        assert_eq!(command.map_id, 571);
        assert_eq!(command.go_type, wow_entities::GAMEOBJECT_TYPE_CHEST as u8);
        assert_eq!(
            command.loot_state,
            Some(wow_entities::LootState::Activated as u8)
        );
        assert_eq!(command.chest_loot_id, 190_010);
        assert_eq!(command.chest_personal_loot_id, 190_011);
        assert_eq!(command.chest_push_loot_id, 190_012);
        assert_eq!(command.chest_quest_id, 777);
        assert_eq!(command.chest_restock_time_secs, 30);
        assert!(!command.chest_consumable);
        assert_eq!(command.linked_trap_entry, Some(190_013));
        assert_eq!(command.linked_trap_guid, None);
        assert!(other_command_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn chest_state_sync_command_updates_receiver_before_refresh_like_cpp() {
        let mut session = make_session();
        let player_guid = ObjectGuid::create_player(1, 77);
        let gameobject_guid = test_gameobject_guid(91_011);
        session.set_state(SessionState::LoggedIn);
        session.set_player_guid(Some(player_guid));
        session.set_player_map_position_like_cpp(571, Position::ZERO);

        session
            .session_command_tx()
            .try_send(SessionCommand::SyncChestGameobjectStateAndRefreshLikeCpp(
                SyncChestGameobjectStateAndRefreshLikeCppCommand {
                    gameobject_guid,
                    map_id: 571,
                    instance_id: 0,
                    go_type: wow_entities::GAMEOBJECT_TYPE_CHEST as u8,
                    loot_state: Some(wow_entities::LootState::Activated as u8),
                    loot_state_unit_guid: ObjectGuid::create_player(1, 42),
                    chest_loot_id: 190_011,
                    chest_personal_loot_id: 190_012,
                    chest_push_loot_id: 190_013,
                    chest_quest_id: 778,
                    chest_restock_time_secs: 45,
                    chest_consumable: false,
                    linked_trap_entry: Some(190_014),
                    linked_trap_guid: Some(test_gameobject_guid(91_014)),
                },
            ))
            .expect("command queued");

        session
            .process_represented_session_commands_like_cpp()
            .await;

        let state = session
            .represented_gameobject_use_states
            .get(&gameobject_guid)
            .expect("synced chest state");
        assert_eq!(
            state.go_type,
            Some(wow_entities::GAMEOBJECT_TYPE_CHEST as u8)
        );
        assert_eq!(state.loot_state, Some(wow_entities::LootState::Activated));
        assert_eq!(state.chest_restock_time_secs, Some(45));
        assert_eq!(state.chest_consumable, Some(false));
        assert_eq!(state.chest_personal_loot_id, Some(190_012));
        assert_eq!(state.linked_trap_entry, Some(190_014));
        assert_eq!(state.linked_trap_guid, Some(test_gameobject_guid(91_014)));
        let source = state.chest_loot_source.expect("synced chest source");
        assert_eq!(source.loot_id, 190_011);
        assert_eq!(source.personal_loot_id, 190_012);
        assert_eq!(source.push_loot_id, 190_013);
        assert_eq!(source.chest_quest_id, 778);
    }

    #[test]
    fn represented_goober_use_syncs_shared_state_to_same_map_viewers_like_cpp() {
        let mut session = make_session();
        let player_guid = ObjectGuid::create_player(1, 42);
        let same_map_guid = ObjectGuid::create_player(1, 77);
        let other_map_guid = ObjectGuid::create_player(1, 88);
        let gameobject_guid = test_gameobject_guid(91_012);
        let (same_command_tx, same_command_rx) = flume::bounded(2);
        let (other_command_tx, other_command_rx) = flume::bounded(2);
        let (same_send_tx, _same_send_rx) = flume::bounded::<Vec<u8>>(1);
        let (other_send_tx, _other_send_rx) = flume::bounded::<Vec<u8>>(1);
        let player_registry = Arc::new(PlayerRegistry::default());
        let mut same_info = broadcast_info(same_map_guid, same_send_tx);
        same_info.map_id = 571;
        same_info.command_tx = same_command_tx;
        player_registry.insert(same_map_guid, same_info);
        let mut other_info = broadcast_info(other_map_guid, other_send_tx);
        other_info.map_id = 1;
        other_info.command_tx = other_command_tx;
        player_registry.insert(other_map_guid, other_info);

        session.set_player_guid(Some(player_guid));
        session.set_player_map_position_like_cpp(571, Position::ZERO);
        session.set_player_registry(player_registry);
        session
            .represented_gameobject_use_states
            .entry(gameobject_guid)
            .or_default()
            .linked_trap_entry = Some(190_015);

        assert!(session.use_represented_gameobject_goober_state_like_cpp(
            gameobject_guid,
            player_guid,
            777,
            wow_entities::GooberUseSource {
                auto_close_ms: 3_000,
                linked_trap_entry: 190_015,
                ..Default::default()
            },
        ));

        let command = match same_command_rx.try_recv() {
            Ok(SessionCommand::SyncGooberGameobjectStateAndRefreshLikeCpp(command)) => command,
            other => panic!("expected goober sync command, got {other:?}"),
        };
        assert_eq!(command.gameobject_guid, gameobject_guid);
        assert_eq!(command.map_id, 571);
        assert_eq!(command.go_type, GAMEOBJECT_TYPE_GOOBER as u8);
        assert_eq!(command.gameobject_flags & wow_entities::GO_FLAG_IN_USE, 1);
        assert_eq!(
            command.loot_state,
            Some(wow_entities::LootState::Activated as u8)
        );
        assert_eq!(command.loot_state_unit_guid, player_guid);
        assert_eq!(command.go_state, Some(wow_entities::GoState::Active as i8));
        assert_eq!(command.linked_trap_entry, Some(190_015));
        assert!(other_command_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn goober_state_sync_command_updates_receiver_before_refresh_like_cpp() {
        let mut session = make_session();
        let player_guid = ObjectGuid::create_player(1, 77);
        let owner_guid = ObjectGuid::create_player(1, 42);
        let gameobject_guid = test_gameobject_guid(91_013);
        session.set_state(SessionState::LoggedIn);
        session.set_player_guid(Some(player_guid));
        session.set_player_map_position_like_cpp(571, Position::ZERO);

        session
            .session_command_tx()
            .try_send(SessionCommand::SyncGooberGameobjectStateAndRefreshLikeCpp(
                SyncGooberGameobjectStateAndRefreshLikeCppCommand {
                    gameobject_guid,
                    map_id: 571,
                    instance_id: 0,
                    go_type: GAMEOBJECT_TYPE_GOOBER as u8,
                    gameobject_flags: wow_entities::GO_FLAG_IN_USE,
                    loot_state: Some(wow_entities::LootState::Activated as u8),
                    loot_state_unit_guid: owner_guid,
                    go_state: Some(wow_entities::GoState::Active as i8),
                    dynamic_flags: wow_entities::GO_DYNFLAG_LO_NO_INTERACT,
                    linked_trap_entry: Some(190_016),
                    linked_trap_guid: Some(test_gameobject_guid(91_016)),
                },
            ))
            .expect("command queued");

        session
            .process_represented_session_commands_like_cpp()
            .await;

        let state = session
            .represented_gameobject_use_states
            .get(&gameobject_guid)
            .expect("synced goober state");
        assert_eq!(state.go_type, Some(GAMEOBJECT_TYPE_GOOBER as u8));
        assert_eq!(state.gameobject_flags & wow_entities::GO_FLAG_IN_USE, 1);
        assert_eq!(state.loot_state, Some(wow_entities::LootState::Activated));
        assert_eq!(state.loot_state_unit_guid, owner_guid);
        assert_eq!(state.go_state, Some(wow_entities::GoState::Active));
        assert_eq!(
            state.dynamic_flags & wow_entities::GO_DYNFLAG_LO_NO_INTERACT,
            wow_entities::GO_DYNFLAG_LO_NO_INTERACT
        );
        assert_eq!(state.linked_trap_entry, Some(190_016));
        assert_eq!(state.linked_trap_guid, Some(test_gameobject_guid(91_016)));
        assert!(state.cooldown_until.is_none());
        assert!(state.goober_use_source.is_none());
    }

    #[tokio::test]
    async fn represented_gathering_node_first_use_records_effects_like_cpp() {
        let mut session = make_session();
        let player_guid = ObjectGuid::create_player(1, 42);
        let gameobject_guid = test_gameobject_guid(91_003);
        session.set_player_guid(Some(player_guid));
        session
            .client_visible_guids_like_cpp
            .insert(gameobject_guid);

        let source = GatheringNodeUseSource {
            loot_id: 0,
            despawn_delay_secs: 0,
            triggered_event_id: 123,
            xp_difficulty: 0,
            spell_id: 0,
            max_loots: 10,
            linked_trap_entry: 456,
        };

        session
            .open_represented_gathering_node_like_cpp(gameobject_guid, 190_003, source)
            .await;
        session
            .open_represented_gathering_node_like_cpp(gameobject_guid, 190_003, source)
            .await;

        assert_eq!(
            session.represented_gameobject_use_effects,
            vec![
                RepresentedGameObjectUseEffect::TriggerGameEvent {
                    gameobject_guid,
                    player_guid,
                    event_id: 123,
                },
                RepresentedGameObjectUseEffect::TriggerLinkedTrap {
                    gameobject_guid,
                    player_guid,
                    trap_entry: 456,
                },
            ]
        );
    }

    #[tokio::test]
    async fn represented_fishing_hole_updates_catch_criteria_like_cpp() {
        let mut session = make_session();
        let player_guid = ObjectGuid::create_player(1, 42);
        let gameobject_guid = test_gameobject_guid(91_005);
        session.set_player_guid(Some(player_guid));
        session
            .client_visible_guids_like_cpp
            .insert(gameobject_guid);

        session
            .open_represented_fishing_hole_like_cpp(gameobject_guid, 190_000, 123)
            .await;

        assert_eq!(
            session.represented_gameobject_use_effects,
            vec![
                RepresentedGameObjectUseEffect::FishingHoleCatchCriteriaUpdated {
                    gameobject_guid,
                    player_guid,
                    gameobject_entry: 190_000,
                }
            ]
        );
    }

    #[tokio::test]
    async fn represented_fishing_node_loot_walks_parent_area_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let gameobject_guid = test_gameobject_guid(91_051);
        let item_id = 80_001;
        session.set_player_guid(Some(player_guid));
        session
            .client_visible_guids_like_cpp
            .insert(gameobject_guid);
        session.set_area_table_store(Arc::new(AreaTableStore::from_entries([
            AreaTableEntry {
                id: 77,
                continent_id: 0,
                parent_area_id: 10,
                mount_flags: 0,
                flags: 0,
            },
            AreaTableEntry {
                id: 10,
                continent_id: 0,
                parent_area_id: 0,
                mount_flags: 0,
                flags: 0,
            },
        ])));
        install_limited_test_item_template(&mut session, item_id, 0);
        let mut fishing_store = LootStore::for_kind_like_cpp(LootStoreKind::Fishing);
        fishing_store
            .load_rows_like_cpp(
                [LootTemplateRow {
                    entry: 10,
                    item: LootStoreItem {
                        item_id,
                        reference: 0,
                        chance: 100.0,
                        needs_quest: false,
                        loot_mode: LOOT_MODE_DEFAULT_LIKE_CPP,
                        group_id: 0,
                        min_count: 1,
                        max_count: 1,
                    },
                }],
                |_| true,
            )
            .unwrap();
        let mut stores = LootStores::new();
        stores.insert(LootStoreKind::Fishing, fishing_store);
        session.set_loot_stores(Arc::new(stores));

        session
            .open_represented_fishing_node_loot_like_cpp(gameobject_guid, 77, false)
            .await;

        let loot = session.loot_table.get(&gameobject_guid).unwrap();
        assert_eq!(loot.loot_type, LOOT_TYPE_FISHING_LIKE_CPP);
        assert_eq!(loot.items.len(), 1);
        assert_eq!(loot.items[0].item_id, item_id);
        assert!(session.is_active_loot_guid(gameobject_guid));
    }

    #[tokio::test]
    async fn represented_fishing_node_junk_loot_uses_default_zone_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let gameobject_guid = test_gameobject_guid(91_052);
        let item_id = 80_002;
        session.set_player_guid(Some(player_guid));
        session
            .client_visible_guids_like_cpp
            .insert(gameobject_guid);
        install_limited_test_item_template(&mut session, item_id, 0);
        let mut fishing_store = LootStore::for_kind_like_cpp(LootStoreKind::Fishing);
        fishing_store
            .load_rows_like_cpp(
                [LootTemplateRow {
                    entry: 1,
                    item: LootStoreItem {
                        item_id,
                        reference: 0,
                        chance: 100.0,
                        needs_quest: false,
                        loot_mode: LOOT_MODE_JUNK_FISH_LIKE_CPP,
                        group_id: 0,
                        min_count: 1,
                        max_count: 1,
                    },
                }],
                |_| true,
            )
            .unwrap();
        let mut stores = LootStores::new();
        stores.insert(LootStoreKind::Fishing, fishing_store);
        session.set_loot_stores(Arc::new(stores));

        session
            .open_represented_fishing_node_loot_like_cpp(gameobject_guid, 77, true)
            .await;

        let loot = session.loot_table.get(&gameobject_guid).unwrap();
        assert_eq!(loot.loot_type, LOOT_TYPE_FISHING_JUNK_LIKE_CPP);
        assert_eq!(loot.items.len(), 1);
        assert_eq!(loot.items[0].item_id, item_id);
        assert!(session.is_active_loot_guid(gameobject_guid));
    }

    #[tokio::test]
    async fn represented_gathering_node_runtime_state_matches_cpp_side_effects() {
        let mut session = make_session();
        let player_guid = ObjectGuid::create_player(1, 42);
        let gameobject_guid = test_gameobject_guid(91_007);
        session.set_player_guid(Some(player_guid));
        session
            .client_visible_guids_like_cpp
            .insert(gameobject_guid);

        let source = GatheringNodeUseSource {
            loot_id: 0,
            despawn_delay_secs: 15,
            triggered_event_id: 0,
            xp_difficulty: 0,
            spell_id: 777,
            max_loots: 1,
            linked_trap_entry: 0,
        };

        session
            .open_represented_gathering_node_like_cpp(gameobject_guid, 190_007, source)
            .await;
        session
            .open_represented_gathering_node_like_cpp(gameobject_guid, 190_007, source)
            .await;

        let state = session
            .represented_gameobject_use_states
            .get(&gameobject_guid)
            .expect("represented gathering use records GO state");
        assert_eq!(state.personal_loot_uses, 1);
        assert_eq!(state.go_state, Some(GoState::Active));
        assert_eq!(
            state.dynamic_flags & GO_DYNFLAG_LO_NO_INTERACT,
            GO_DYNFLAG_LO_NO_INTERACT
        );
        assert_eq!(state.loot_state, Some(LootState::Activated));
        assert_eq!(state.loot_state_unit_guid, player_guid);
        assert_eq!(state.despawn_delay_secs, Some(15));
        assert!(state.despawn_delay_until.is_some());
        assert_eq!(
            session.represented_gameobject_use_effects,
            vec![
                RepresentedGameObjectUseEffect::OutdoorPvpCustomSpellRequested {
                    gameobject_guid,
                    player_guid,
                    gameobject_entry: 190_007,
                    spell_id: 777,
                    go_type: GAMEOBJECT_TYPE_GATHERING_NODE,
                    spell_lookup_difficulty_id: 0,
                    spell_info_missing: false,
                },
                RepresentedGameObjectUseEffect::GameObjectPostUseSpellCast {
                    gameobject_guid,
                    target_guid: player_guid,
                    caster_guid: player_guid,
                    spell_id: 777,
                    triggered: false,
                    caster: RepresentedGameObjectSpellCaster::User,
                    spell_lookup_difficulty_id: 0,
                },
            ]
        );
    }

    #[tokio::test]
    async fn represented_gathering_node_use_refreshes_same_map_gameobject_viewers_like_cpp() {
        let mut session = make_session();
        let player_guid = ObjectGuid::create_player(1, 42);
        let same_map_guid = ObjectGuid::create_player(1, 77);
        let other_map_guid = ObjectGuid::create_player(1, 88);
        let gameobject_guid = test_gameobject_guid(91_008);
        let (same_command_tx, same_command_rx) = flume::bounded(2);
        let (other_command_tx, other_command_rx) = flume::bounded(2);
        let (same_send_tx, _same_send_rx) = flume::bounded::<Vec<u8>>(1);
        let (other_send_tx, _other_send_rx) = flume::bounded::<Vec<u8>>(1);
        let player_registry = Arc::new(PlayerRegistry::default());
        let mut same_info = broadcast_info(same_map_guid, same_send_tx);
        same_info.map_id = 571;
        same_info.command_tx = same_command_tx;
        player_registry.insert(same_map_guid, same_info);
        let mut other_info = broadcast_info(other_map_guid, other_send_tx);
        other_info.map_id = 1;
        other_info.command_tx = other_command_tx;
        player_registry.insert(other_map_guid, other_info);

        session.set_player_guid(Some(player_guid));
        session.set_player_map_position_like_cpp(571, Position::ZERO);
        session.set_player_registry(player_registry);
        session
            .client_visible_guids_like_cpp
            .insert(gameobject_guid);

        let source = GatheringNodeUseSource {
            loot_id: 0,
            despawn_delay_secs: 0,
            triggered_event_id: 0,
            xp_difficulty: 0,
            spell_id: 0,
            max_loots: 1,
            linked_trap_entry: 0,
        };

        session
            .open_represented_gathering_node_like_cpp(gameobject_guid, 190_008, source)
            .await;

        let command = match same_command_rx.try_recv() {
            Ok(SessionCommand::SyncGatheringNodeGameobjectStateAndRefreshLikeCpp(command)) => {
                command
            }
            other => panic!("expected gathering-node sync command, got {other:?}"),
        };
        assert_eq!(command.gameobject_guid, gameobject_guid);
        assert_eq!(command.map_id, 571);
        assert_eq!(
            command.go_type,
            wow_entities::GAMEOBJECT_TYPE_GATHERING_NODE as u8
        );
        assert_eq!(
            command.loot_state,
            Some(wow_entities::LootState::Activated as u8)
        );
        assert_eq!(command.go_state, Some(wow_entities::GoState::Active as i8));
        assert_eq!(command.gathering_node_loot_id, Some(0));
        assert_eq!(
            command.dynamic_flags & wow_entities::GO_DYNFLAG_LO_NO_INTERACT,
            wow_entities::GO_DYNFLAG_LO_NO_INTERACT
        );
        assert!(other_command_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn gathering_node_state_sync_command_updates_receiver_before_refresh_like_cpp() {
        let mut session = make_session();
        let player_guid = ObjectGuid::create_player(1, 77);
        let gameobject_guid = test_gameobject_guid(91_009);
        session.set_state(SessionState::LoggedIn);
        session.set_player_guid(Some(player_guid));
        session.set_player_map_position_like_cpp(571, Position::ZERO);

        session
            .session_command_tx()
            .try_send(
                SessionCommand::SyncGatheringNodeGameobjectStateAndRefreshLikeCpp(
                    SyncGatheringNodeGameobjectStateAndRefreshLikeCppCommand {
                        gameobject_guid,
                        map_id: 571,
                        instance_id: 0,
                        go_type: wow_entities::GAMEOBJECT_TYPE_GATHERING_NODE as u8,
                        loot_state: Some(wow_entities::LootState::Activated as u8),
                        loot_state_unit_guid: ObjectGuid::create_player(1, 42),
                        go_state: Some(wow_entities::GoState::Active as i8),
                        dynamic_flags: wow_entities::GO_DYNFLAG_LO_NO_INTERACT,
                        gathering_node_loot_id: Some(190_009),
                        personal_loot_uses: 1,
                        linked_trap_entry: Some(191_009),
                        linked_trap_guid: Some(test_gameobject_guid(91_010)),
                    },
                ),
            )
            .expect("command queued");

        session
            .process_represented_session_commands_like_cpp()
            .await;

        let state = session
            .represented_gameobject_use_states
            .get(&gameobject_guid)
            .expect("synced gathering node state");
        assert_eq!(
            state.go_type,
            Some(wow_entities::GAMEOBJECT_TYPE_GATHERING_NODE as u8)
        );
        assert_eq!(state.loot_state, Some(wow_entities::LootState::Activated));
        assert_eq!(state.go_state, Some(wow_entities::GoState::Active));
        assert_eq!(
            state.dynamic_flags & wow_entities::GO_DYNFLAG_LO_NO_INTERACT,
            wow_entities::GO_DYNFLAG_LO_NO_INTERACT
        );
        assert_eq!(state.gathering_node_loot_id, Some(190_009));
        assert_eq!(state.personal_loot_uses, 1);
        assert_eq!(state.linked_trap_entry, Some(191_009));
        assert_eq!(state.linked_trap_guid, Some(test_gameobject_guid(91_010)));
    }

    #[tokio::test]
    async fn represented_creature_loot_captures_group_method_master_and_round_robin_like_cpp() {
        let mut session = make_session();
        let master_guid = ObjectGuid::create_player(1, 42);
        let candidate_guid = ObjectGuid::create_player(1, 77);
        let owner_guid = test_creature_guid(19_049);
        install_master_loot_group(&mut session, master_guid, candidate_guid);

        let loot = session
            .generate_represented_creature_loot_like_cpp(
                owner_guid,
                master_guid,
                10,
                25,
                0,
                0,
                0,
                0,
            )
            .await;

        assert_eq!(loot.loot_method, LOOT_METHOD_MASTER_LIKE_CPP);
        assert_eq!(loot.loot_master, master_guid);
        assert_eq!(loot.round_robin_player, master_guid);
    }

    fn test_gameobject_guid(counter: i64) -> ObjectGuid {
        ObjectGuid::create_world_object(HighGuid::GameObject, 0, 1, 0, 0, 1, counter)
    }

    fn test_item_record(
        item_id: u32,
        random_select: u16,
        random_suffix_group_id: u16,
    ) -> ItemRecord {
        ItemRecord {
            id: item_id,
            class_id: 2,
            subclass_id: 7,
            material: 0,
            inventory_type: InventoryType::Chest as i8,
            sheathe_type: 0,
            random_select,
            random_suffix_group_id,
        }
    }

    #[test]
    fn loot_item_random_context_runtime_and_persistence_fields_match_entry() {
        let sql = CharStatements::INS_ITEM_INSTANCE_WITH_RANDOM_CONTEXT.sql();
        assert!(sql.contains("randomPropertiesId"));
        assert!(sql.contains("randomPropertiesSeed"));
        assert!(sql.contains("context"));

        let item_guid = ObjectGuid::create_item(1, 902);
        let owner_guid = ObjectGuid::create_player(1, 42);
        let mut item = Item::new(0);
        item.initialize_created_state(ItemCreateInfo {
            guid: item_guid,
            item_id: 25,
            context: loot_item_context(2),
            owner: Some(owner_guid),
            max_durability: 0,
            expiration: 0,
            spell_charges: [0; MAX_ITEM_SPELLS],
        });
        item.set_random_properties_id(-77);
        item.set_property_seed(456);

        let data = item.data();
        assert_eq!(data.random_properties_id, -77);
        assert_eq!(data.property_seed, 456);
        assert_eq!(u8::try_from(data.context).unwrap_or(0), 2);
    }

    #[test]
    fn loot_item_random_context_stack_compatibility_uses_cpp_store_metadata() {
        let item_guid = ObjectGuid::create_item(1, 901);
        let owner_guid = ObjectGuid::create_player(1, 42);
        let mut item = Item::new(0);
        item.initialize_created_state(ItemCreateInfo {
            guid: item_guid,
            item_id: 25,
            context: ItemContext::DungeonHeroic,
            owner: Some(owner_guid),
            max_durability: 0,
            expiration: 0,
            spell_charges: [0; MAX_ITEM_SPELLS],
        });
        item.set_random_properties_id(-77);
        item.set_property_seed(456);

        let matching = LootEntry {
            loot_list_id: 0,
            item_id: 25,
            quantity: 1,
            random_properties_id: -77,
            random_properties_seed: 456,
            item_context: 2,
            flags: LootEntryFlags::default(),
            allowed_looters: Vec::new(),
            roll_winner: ObjectGuid::EMPTY,
            ffa_looted_by: Vec::new(),
            taken: false,
        };
        assert!(loot_store_data_can_stack_with_item(
            &matching,
            LootStoreRandomProperties { id: -77, seed: 456 },
            &item
        ));

        let different_random = LootEntry {
            random_properties_id: -78,
            ..matching.clone()
        };
        assert!(loot_store_data_can_stack_with_item(
            &different_random,
            LootStoreRandomProperties { id: -77, seed: 456 },
            &item
        ));
        assert!(!loot_store_data_can_stack_with_item(
            &matching,
            LootStoreRandomProperties { id: 0, seed: 0 },
            &item
        ));
    }

    #[test]
    fn loot_item_store_random_properties_are_generated_from_cpp_random_select() {
        let entry = LootEntry {
            loot_list_id: 0,
            item_id: 25,
            quantity: 1,
            random_properties_id: -77,
            random_properties_seed: 456,
            item_context: 2,
            flags: LootEntryFlags::default(),
            allowed_looters: Vec::new(),
            roll_winner: ObjectGuid::EMPTY,
            ffa_looted_by: Vec::new(),
            taken: false,
        };
        let mut session = make_session();
        session.set_item_store(Arc::new(ItemStore::from_records([test_item_record(
            entry.item_id,
            77,
            0,
        )])));
        session.set_item_random_enchantment_template_store(Arc::new(
            ItemRandomEnchantmentTemplateStore::from_entries([
                ItemRandomEnchantmentTemplateEntry {
                    group_id: 77,
                    enchantment_id: 9001,
                    chance: 100.0,
                },
            ]),
        ));
        session.set_item_random_properties_store(Arc::new(
            ItemRandomPropertiesStore::from_entries([ItemRandomPropertiesEntry {
                id: 9001,
                enchantments: [1, 2, 3, 0, 0],
            }]),
        ));

        let generated = session.generate_loot_store_random_properties_with_rng_like_cpp(
            entry.item_id,
            &mut StdRng::seed_from_u64(1),
        );
        assert_eq!(generated, LootStoreRandomProperties { id: 9001, seed: 0 });
        assert_ne!(generated.id, entry.random_properties_id);
        assert_ne!(generated.seed, entry.random_properties_seed);
    }

    #[test]
    fn loot_item_store_random_suffix_uses_cpp_property_points_seed() {
        let mut session = make_session();
        session.set_item_store(Arc::new(ItemStore::from_records([test_item_record(
            25, 0, 88,
        )])));
        session.set_item_stats_store(Arc::new(ItemStatsStore::from_random_property_templates([
            (
                25,
                ItemRandomPropertyTemplateEntry {
                    item_level: 11,
                    quality: ItemQuality::Uncommon as i8,
                    inventory_type: InventoryType::Chest as i8,
                },
            ),
        ])));
        session.set_item_random_enchantment_template_store(Arc::new(
            ItemRandomEnchantmentTemplateStore::from_entries([
                ItemRandomEnchantmentTemplateEntry {
                    group_id: 88,
                    enchantment_id: 7001,
                    chance: 100.0,
                },
            ]),
        ));
        session.set_item_random_suffix_store(Arc::new(ItemRandomSuffixStore::from_entries([
            ItemRandomSuffixEntry {
                id: 7001,
                enchantments: [10, 0, 0, 0, 0],
                allocation_pct: [10000, 0, 0, 0, 0],
            },
        ])));
        session.set_rand_prop_points_store(Arc::new(RandPropPointsStore::from_entries([
            RandPropPointsEntry {
                id: 11,
                damage_replace_stat: 0,
                epic: [900, 0, 0, 0, 0],
                superior: [500, 0, 0, 0, 0],
                good: [123, 0, 0, 0, 0],
            },
        ])));

        let generated = session.generate_loot_store_random_properties_with_rng_like_cpp(
            25,
            &mut StdRng::seed_from_u64(1),
        );
        assert_eq!(
            generated,
            LootStoreRandomProperties {
                id: -7001,
                seed: 123
            }
        );
    }

    #[test]
    fn random_enchantment_selection_uses_cpp_weighted_chances() {
        let group = [
            ItemRandomEnchantmentTemplateEntry {
                group_id: 1,
                enchantment_id: 10,
                chance: 0.0,
            },
            ItemRandomEnchantmentTemplateEntry {
                group_id: 1,
                enchantment_id: 11,
                chance: 100.0,
            },
        ];
        assert_eq!(
            select_weighted_random_enchantment_like_cpp(&group, &mut StdRng::seed_from_u64(5)),
            Some(11)
        );
    }

    #[test]
    fn gameobject_interaction_distance_uses_cpp_type_branches() {
        assert_eq!(
            represented_gameobject_interaction_distance_like_cpp(
                Some(GAMEOBJECT_TYPE_CHEST as u8),
                Some(725)
            ),
            7.25
        );
        assert_eq!(
            represented_gameobject_interaction_distance_like_cpp(
                Some(GAMEOBJECT_TYPE_AREADAMAGE as u8),
                None
            ),
            0.0
        );
        assert_eq!(
            represented_gameobject_interaction_distance_like_cpp(
                Some(GAMEOBJECT_TYPE_QUESTGIVER as u8),
                None
            ),
            5.5555553
        );
        assert_eq!(
            represented_gameobject_interaction_distance_like_cpp(
                Some(GAMEOBJECT_TYPE_BINDER as u8),
                None
            ),
            10.0
        );
        assert_eq!(
            represented_gameobject_interaction_distance_like_cpp(
                Some(GAMEOBJECT_TYPE_CHAIR as u8),
                None
            ),
            3.0
        );
        assert_eq!(
            represented_gameobject_interaction_distance_like_cpp(
                Some(GAMEOBJECT_TYPE_FISHING_NODE as u8),
                None
            ),
            100.0
        );
        assert_eq!(
            represented_gameobject_interaction_distance_like_cpp(
                Some(GAMEOBJECT_TYPE_FISHING_HOLE as u8),
                None
            ),
            20.0 + wow_movement::CONTACT_DISTANCE_LIKE_CPP
        );
        assert_eq!(
            represented_gameobject_interaction_distance_like_cpp(
                Some(GAMEOBJECT_TYPE_DOOR as u8),
                None
            ),
            5.0
        );
        assert_eq!(
            represented_gameobject_interaction_distance_like_cpp(
                Some(GAMEOBJECT_TYPE_GUILD_BANK as u8),
                None
            ),
            10.0
        );
        assert_eq!(
            represented_gameobject_interaction_distance_like_cpp(None, None),
            5.0
        );
    }

    #[test]
    fn gameobject_display_box_interaction_matches_cpp_contains_branch() {
        let display_info = wow_data::GameObjectDisplayInfoEntry {
            id: 77,
            model_name: "test".to_string(),
            geo_box_min: wow_data::Db2Pos3 {
                x: -2.0,
                y: -1.0,
                z: -0.5,
            },
            geo_box_max: wow_data::Db2Pos3 {
                x: 2.0,
                y: 1.0,
                z: 0.5,
            },
            file_data_id: 0,
            object_effect_package_id: 0,
            override_loot_effect_scale: 0.0,
            override_name_scale: 0.0,
        };
        let go_position = Position::ZERO;

        assert!(represented_gameobject_display_box_contains_like_cpp(
            go_position,
            Position::xyz(6.9, 0.0, 0.0),
            &display_info,
            1.0,
            [0.0, 0.0, 0.0, 1.0],
            5.0,
        ));
        assert!(!represented_gameobject_display_box_contains_like_cpp(
            go_position,
            Position::xyz(7.1, 0.0, 0.0),
            &display_info,
            1.0,
            [0.0, 0.0, 0.0, 1.0],
            5.0,
        ));
    }

    #[test]
    fn gameobject_loot_distance_uses_display_box_when_db2_exists_like_cpp() {
        let mut session = make_session();
        let player_guid = ObjectGuid::create_player(1, 42);
        let gameobject_guid = test_gameobject_guid(19_041);
        session.set_player_guid(Some(player_guid));
        session.record_represented_gameobject_runtime_state_like_cpp(
            0,
            gameobject_guid,
            gameobject_guid.entry(),
            Position::ZERO,
            GAMEOBJECT_TYPE_CHEST as u8,
        );
        session.record_represented_gameobject_display_model_like_cpp(
            gameobject_guid,
            77,
            1.0,
            [0.0, 0.0, 0.0, 1.0],
        );
        session.set_gameobject_display_info_store(Arc::new(
            wow_data::GameObjectDisplayInfoStore::from_entries([
                wow_data::GameObjectDisplayInfoEntry {
                    id: 77,
                    model_name: "test".to_string(),
                    geo_box_min: wow_data::Db2Pos3 {
                        x: -2.0,
                        y: -1.0,
                        z: -0.5,
                    },
                    geo_box_max: wow_data::Db2Pos3 {
                        x: 2.0,
                        y: 1.0,
                        z: 0.5,
                    },
                    file_data_id: 0,
                    object_effect_package_id: 0,
                    override_loot_effect_scale: 0.0,
                    override_name_scale: 0.0,
                },
            ]),
        ));

        session.set_player_position_like_cpp(Position::xyz(6.9, 0.0, 0.0));
        assert!(
            session.represented_gameobject_can_autostore_loot_item_like_cpp(
                gameobject_guid,
                player_guid
            )
        );

        session.set_player_position_like_cpp(Position::xyz(7.1, 0.0, 0.0));
        assert!(
            !session.represented_gameobject_can_autostore_loot_item_like_cpp(
                gameobject_guid,
                player_guid
            )
        );
    }

    #[test]
    fn gameobject_loot_distance_uses_spell_lock_range_like_cpp() {
        let mut session = make_session();
        let player_guid = ObjectGuid::create_player(1, 42);
        let gameobject_guid = test_gameobject_guid(19_042);
        session.set_player_guid(Some(player_guid));
        session.set_player_position_like_cpp(Position::xyz(11.0, 0.0, 0.0));
        session.record_represented_gameobject_runtime_state_like_cpp(
            0,
            gameobject_guid,
            gameobject_guid.entry(),
            Position::ZERO,
            GAMEOBJECT_TYPE_CHEST as u8,
        );
        session.record_represented_gameobject_lock_id_like_cpp(gameobject_guid, 501);
        session.set_lock_store(Arc::new(wow_data::LockStore::from_entries([
            wow_data::LockEntry {
                id: 501,
                index: [7001, 0, 0, 0, 0, 0, 0, 0],
                skill: [0; wow_data::lock::MAX_LOCK_CASE],
                lock_type: [LOCK_KEY_SPELL_LIKE_CPP, 0, 0, 0, 0, 0, 0, 0],
                action: [0; wow_data::lock::MAX_LOCK_CASE],
            },
        ])));
        let mut spell_store = SpellStore::new();
        spell_store.insert(
            7001,
            SpellInfo {
                spell_id: 7001,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: 0,
                effect_base_points: 0,
                effect_bonus_coefficient: 0.0,
                aura_type: None,
                display_flags: 0,
                requires_spell_focus: 0,
                effects: Vec::new(),
            },
        );
        session.set_spell_store(Arc::new(spell_store));
        session.set_spell_misc_store(Arc::new(SpellMiscStore::from_entries([SpellMiscEntry {
            id: 7001,
            attributes: [0; 15],
            difficulty_id: 0,
            casting_time_index: 0,
            duration_index: 0,
            range_index: 77,
            school_mask: 0,
            speed: 0.0,
            launch_delay: 0.0,
            min_duration: 0.0,
            spell_icon_file_data_id: 0,
            active_icon_file_data_id: 0,
            content_tuning_id: 0,
            show_future_spell_player_condition_id: 0,
            spell_id: 7001,
        }])));
        session.set_spell_range_store(Arc::new(SpellRangeStore::from_entries([SpellRangeEntry {
            id: 77,
            display_name: "lock".to_string(),
            display_name_short: "lock".to_string(),
            flags: 0,
            range_min: [0.0, 0.0],
            range_max: [12.0, 12.0],
        }])));

        assert!(
            session.represented_gameobject_can_autostore_loot_item_like_cpp(
                gameobject_guid,
                player_guid
            )
        );

        session.set_player_position_like_cpp(Position::xyz(12.1, 0.0, 0.0));
        assert!(
            !session.represented_gameobject_can_autostore_loot_item_like_cpp(
                gameobject_guid,
                player_guid
            )
        );
    }

    #[test]
    fn gameobject_loot_distance_uses_known_open_lock_skill_spell_like_cpp() {
        let mut session = make_session();
        let player_guid = ObjectGuid::create_player(1, 42);
        let gameobject_guid = test_gameobject_guid(19_043);
        session.set_player_guid(Some(player_guid));
        session.set_player_position_like_cpp(Position::xyz(8.0, 0.0, 0.0));
        session.set_known_spells_like_cpp(vec![8001]);
        session.record_represented_gameobject_runtime_state_like_cpp(
            0,
            gameobject_guid,
            gameobject_guid.entry(),
            Position::ZERO,
            GAMEOBJECT_TYPE_CHEST as u8,
        );
        session.record_represented_gameobject_lock_id_like_cpp(gameobject_guid, 502);
        session.set_lock_store(Arc::new(wow_data::LockStore::from_entries([
            wow_data::LockEntry {
                id: 502,
                index: [333, 0, 0, 0, 0, 0, 0, 0],
                skill: [50, 0, 0, 0, 0, 0, 0, 0],
                lock_type: [LOCK_KEY_SKILL_LIKE_CPP, 0, 0, 0, 0, 0, 0, 0],
                action: [0; wow_data::lock::MAX_LOCK_CASE],
            },
        ])));
        let mut spell_store = SpellStore::new();
        spell_store.insert(
            8001,
            SpellInfo {
                spell_id: 8001,
                cast_time_ms: 0,
                cooldown_ms: 0,
                recovery_time_ms: 0,
                effect_type: SPELL_EFFECT_OPEN_LOCK_LIKE_CPP,
                effect_base_points: 75,
                effect_bonus_coefficient: 0.0,
                aura_type: None,
                display_flags: 0,
                requires_spell_focus: 0,
                effects: vec![SpellEffectInfo {
                    effect_index: 0,
                    effect: SPELL_EFFECT_OPEN_LOCK_LIKE_CPP,
                    effect_aura: 0,
                    effect_base_points: 75,
                    effect_misc_value_1: 333,
                    effect_misc_value_2: 0,
                    effect_radius_index_1: 0,
                    chain_targets: 0,
                    implicit_target_1: 0,
                    implicit_target_2: 0,
                    ..Default::default()
                }],
            },
        );
        session.set_spell_store(Arc::new(spell_store));
        session.set_spell_misc_store(Arc::new(SpellMiscStore::from_entries([SpellMiscEntry {
            id: 8001,
            attributes: [0; 15],
            difficulty_id: 0,
            casting_time_index: 0,
            duration_index: 0,
            range_index: 88,
            school_mask: 0,
            speed: 0.0,
            launch_delay: 0.0,
            min_duration: 0.0,
            spell_icon_file_data_id: 0,
            active_icon_file_data_id: 0,
            content_tuning_id: 0,
            show_future_spell_player_condition_id: 0,
            spell_id: 8001,
        }])));
        session.set_spell_range_store(Arc::new(SpellRangeStore::from_entries([SpellRangeEntry {
            id: 88,
            display_name: "skill".to_string(),
            display_name_short: "skill".to_string(),
            flags: 0,
            range_min: [0.0, 0.0],
            range_max: [9.0, 9.0],
        }])));

        assert!(
            session.represented_gameobject_can_autostore_loot_item_like_cpp(
                gameobject_guid,
                player_guid
            )
        );
    }

    #[test]
    fn loot_is_looted_requires_no_money_and_no_unlooted_items_like_cpp() {
        let mut loot = CreatureLoot {
            loot_guid: ObjectGuid::EMPTY,
            coins: 1,
            unlooted_count: 0,
            loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
            dungeon_encounter_id: 0,
            loot_method: 0,
            loot_master: ObjectGuid::EMPTY,
            round_robin_player: ObjectGuid::EMPTY,
            player_ffa_items: Vec::new(),
            players_looting: Vec::new(),
            allowed_looters: Vec::new(),
            items: vec![],
            looted_by_player: false,
        };
        assert!(!loot_is_looted_like_cpp(&loot));

        loot.coins = 0;
        loot.items.push(LootEntry {
            loot_list_id: 0,
            item_id: 25,
            quantity: 1,
            random_properties_id: 0,
            random_properties_seed: 0,
            item_context: 0,
            flags: LootEntryFlags::default(),
            allowed_looters: Vec::new(),
            roll_winner: ObjectGuid::EMPTY,
            ffa_looted_by: Vec::new(),
            taken: false,
        });
        loot.unlooted_count = 1;
        assert!(!loot_is_looted_like_cpp(&loot));

        loot.items[0].taken = true;
        assert!(!loot_is_looted_like_cpp(&loot));

        loot.unlooted_count = 0;
        assert!(loot_is_looted_like_cpp(&loot));
    }

    #[tokio::test]
    async fn loot_unit_live_creature_returns_silently_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let loot_guid = test_creature_guid(19_006);
        session.set_player_guid(Some(player_guid));
        register_test_creature_like_cpp(&mut session, test_creature(loot_guid, true));

        session.handle_loot_unit(loot_unit_packet(loot_guid)).await;

        assert!(send_rx.try_recv().is_err());
        assert!(!session.is_active_loot_guid(loot_guid));
    }

    #[tokio::test]
    async fn loot_unit_dead_player_returns_silently_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let loot_guid = test_creature_guid(19_033);
        session.set_player_guid(Some(player_guid));
        session.set_player_alive_like_cpp(false);
        install_active_spell_cast(&mut session, player_guid);
        install_visible_aura_with_interrupt_flags(
            &mut session,
            3,
            777,
            player_guid,
            SPELL_AURA_INTERRUPT_FLAG_LOOTING_LIKE_CPP,
        );
        register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));

        session.handle_loot_unit(loot_unit_packet(loot_guid)).await;

        assert!(send_rx.try_recv().is_err());
        assert!(!session.is_active_loot_guid(loot_guid));
        assert!(!session.loot_table.contains_key(&loot_guid));
        assert!(session.active_spell_cast.is_some());
        assert!(session.visible_auras.contains_key(&3));
    }

    #[tokio::test]
    async fn loot_unit_valid_target_interrupts_active_cast_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let loot_guid = test_creature_guid(19_034);
        session.set_player_guid(Some(player_guid));
        install_active_spell_cast(&mut session, player_guid);
        register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));

        session.handle_loot_unit(loot_unit_packet(loot_guid)).await;

        assert!(session.active_spell_cast.is_none());
    }

    #[tokio::test]
    async fn loot_unit_valid_target_removes_looting_interrupt_auras_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send_capacity(2);
        let player_guid = ObjectGuid::create_player(1, 42);
        let loot_guid = test_creature_guid(19_035);
        session.set_player_guid(Some(player_guid));
        install_visible_aura_with_interrupt_flags(
            &mut session,
            3,
            777,
            player_guid,
            SPELL_AURA_INTERRUPT_FLAG_LOOTING_LIKE_CPP,
        );
        install_visible_aura_with_interrupt_flags(&mut session, 4, 778, player_guid, 0);
        register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));

        session.handle_loot_unit(loot_unit_packet(loot_guid)).await;

        assert!(!session.visible_auras.contains_key(&3));
        assert!(session.visible_auras.contains_key(&4));
    }

    #[tokio::test]
    async fn loot_unit_master_looter_first_open_sends_candidate_list_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(3);
        let master_guid = ObjectGuid::create_player(1, 42);
        let candidate_guid = ObjectGuid::create_player(1, 77);
        let owner_guid = test_creature_guid(19_046);
        let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
        session.set_player_guid(Some(master_guid));
        install_master_loot_group(&mut session, master_guid, candidate_guid);
        register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
        session.loot_table.insert(
            owner_guid,
            CreatureLoot {
                loot_guid: loot_object,
                coins: 1,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: LOOT_METHOD_MASTER_LIKE_CPP,
                loot_master: master_guid,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: vec![master_guid, candidate_guid],
                items: Vec::new(),
                looted_by_player: false,
            },
        );

        session.handle_loot_unit(loot_unit_packet(owner_guid)).await;

        let response = send_rx.try_recv().unwrap();
        let mut response = WorldPacket::from_bytes(&response);
        assert_eq!(
            response.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootResponse as u16
        );

        let loot_list = send_rx.try_recv().unwrap();
        let mut loot_list = WorldPacket::from_bytes(&loot_list);
        assert_eq!(
            loot_list.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootList as u16
        );
        assert_eq!(loot_list.read_packed_guid().unwrap(), owner_guid);
        assert_eq!(loot_list.read_packed_guid().unwrap(), loot_object);
        assert!(!loot_list.read_bit().unwrap());
        assert!(!loot_list.read_bit().unwrap());

        let candidate_list = send_rx.try_recv().unwrap();
        let mut candidate_list = WorldPacket::from_bytes(&candidate_list);
        assert_eq!(
            candidate_list.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::MasterLootCandidateList as u16
        );
        assert_eq!(candidate_list.read_packed_guid().unwrap(), loot_object);
        assert_eq!(candidate_list.read_uint32().unwrap(), 2);
        assert_eq!(candidate_list.read_packed_guid().unwrap(), master_guid);
        assert_eq!(candidate_list.read_packed_guid().unwrap(), candidate_guid);
        assert!(send_rx.try_recv().is_err());
        assert!(
            session
                .loot_table
                .get(&owner_guid)
                .is_some_and(|loot| loot.looted_by_player)
        );
    }

    #[tokio::test]
    async fn loot_unit_master_looter_candidate_list_is_first_open_only_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(8);
        let master_guid = ObjectGuid::create_player(1, 42);
        let candidate_guid = ObjectGuid::create_player(1, 77);
        let owner_guid = test_creature_guid(19_047);
        session.set_player_guid(Some(master_guid));
        install_master_loot_group(&mut session, master_guid, candidate_guid);
        register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
        session.loot_table.insert(
            owner_guid,
            CreatureLoot {
                loot_guid: represented_loot_object_guid_like_cpp(owner_guid),
                coins: 1,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: LOOT_METHOD_MASTER_LIKE_CPP,
                loot_master: master_guid,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: vec![master_guid, candidate_guid],
                items: Vec::new(),
                looted_by_player: false,
            },
        );

        session.handle_loot_unit(loot_unit_packet(owner_guid)).await;
        session.handle_loot_unit(loot_unit_packet(owner_guid)).await;

        let mut master_candidate_lists = 0;
        while let Ok(sent) = send_rx.try_recv() {
            let mut sent = WorldPacket::from_bytes(&sent);
            if sent.read_uint16().unwrap()
                == wow_constants::ServerOpcodes::MasterLootCandidateList as u16
            {
                master_candidate_lists += 1;
            }
        }

        assert_eq!(master_candidate_lists, 1);
    }

    #[tokio::test]
    async fn loot_unit_master_loot_notify_list_fans_out_to_allowed_looters_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(4);
        let master_guid = ObjectGuid::create_player(1, 42);
        let candidate_guid = ObjectGuid::create_player(1, 77);
        let owner_guid = test_creature_guid(19_048);
        let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
        let (candidate_tx, candidate_rx) = flume::bounded::<Vec<u8>>(2);
        let player_registry = Arc::new(PlayerRegistry::default());
        player_registry.insert(candidate_guid, broadcast_info(candidate_guid, candidate_tx));
        session.set_player_registry(player_registry);
        session.set_player_guid(Some(master_guid));
        install_master_loot_group(&mut session, master_guid, candidate_guid);
        register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
        session.loot_table.insert(
            owner_guid,
            CreatureLoot {
                loot_guid: loot_object,
                coins: 0,
                unlooted_count: 1,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: LOOT_METHOD_MASTER_LIKE_CPP,
                loot_master: master_guid,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: vec![master_guid, candidate_guid],
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags {
                        follow_loot_rules: true,
                        ..Default::default()
                    },
                    allowed_looters: vec![master_guid, candidate_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        session.handle_loot_unit(loot_unit_packet(owner_guid)).await;

        let _response = send_rx.try_recv().unwrap();
        let local_loot_list = send_rx.try_recv().unwrap();
        let mut local_loot_list = WorldPacket::from_bytes(&local_loot_list);
        assert_eq!(
            local_loot_list.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootList as u16
        );
        assert_eq!(local_loot_list.read_packed_guid().unwrap(), owner_guid);
        assert_eq!(local_loot_list.read_packed_guid().unwrap(), loot_object);
        assert!(local_loot_list.read_bit().unwrap());
        assert!(!local_loot_list.read_bit().unwrap());
        local_loot_list.reset_bits();
        assert_eq!(local_loot_list.read_packed_guid().unwrap(), master_guid);

        let remote_loot_list = candidate_rx.try_recv().unwrap();
        let mut remote_loot_list = WorldPacket::from_bytes(&remote_loot_list);
        assert_eq!(
            remote_loot_list.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootList as u16
        );
        assert_eq!(remote_loot_list.read_packed_guid().unwrap(), owner_guid);
        assert_eq!(remote_loot_list.read_packed_guid().unwrap(), loot_object);
        assert!(remote_loot_list.read_bit().unwrap());
        assert!(!remote_loot_list.read_bit().unwrap());
        remote_loot_list.reset_bits();
        assert_eq!(remote_loot_list.read_packed_guid().unwrap(), master_guid);
    }

    #[tokio::test]
    async fn loot_unit_group_loot_first_open_starts_roll_for_blocked_item_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(4);
        let player_guid = ObjectGuid::create_player(1, 42);
        let candidate_guid = ObjectGuid::create_player(1, 77);
        let disconnected_guid = ObjectGuid::create_player(1, 88);
        let owner_guid = test_creature_guid(19_049);
        let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
        let (candidate_tx, candidate_rx) = flume::bounded::<Vec<u8>>(4);
        let player_registry = Arc::new(PlayerRegistry::default());
        player_registry.insert(candidate_guid, broadcast_info(candidate_guid, candidate_tx));
        session.set_player_registry(player_registry);
        session.set_player_guid(Some(player_guid));
        install_group_loot_group(&mut session, player_guid, candidate_guid);
        register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
        session.loot_table.insert(
            owner_guid,
            CreatureLoot {
                loot_guid: loot_object,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: vec![player_guid, candidate_guid],
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags {
                        follow_loot_rules: true,
                        blocked: true,
                        ..Default::default()
                    },
                    allowed_looters: vec![player_guid, candidate_guid, disconnected_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        session.handle_loot_unit(loot_unit_packet(owner_guid)).await;

        let response = send_rx.try_recv().unwrap();
        let mut response = WorldPacket::from_bytes(&response);
        assert_eq!(
            response.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootResponse as u16
        );
        let loot_list = send_rx.try_recv().unwrap();
        let mut loot_list = WorldPacket::from_bytes(&loot_list);
        assert_eq!(
            loot_list.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootList as u16
        );

        let start_roll = send_rx.try_recv().unwrap();
        let mut start_roll = WorldPacket::from_bytes(&start_roll);
        assert_eq!(
            start_roll.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::StartLootRoll as u16
        );
        assert_eq!(start_roll.read_packed_guid().unwrap(), loot_object);
        assert_eq!(start_roll.read_int32().unwrap(), 0);
        assert_eq!(start_roll.read_uint32().unwrap(), 60_000);
        assert_eq!(start_roll.read_uint8().unwrap(), 0x07);
        assert_eq!(start_roll.read_uint32().unwrap(), 0);
        assert_eq!(start_roll.read_uint32().unwrap(), 0);
        assert_eq!(start_roll.read_uint32().unwrap(), 0);
        assert_eq!(start_roll.read_uint32().unwrap(), 0);
        assert_eq!(start_roll.read_uint8().unwrap(), LOOT_METHOD_GROUP_LIKE_CPP);
        assert_eq!(start_roll.read_int32().unwrap(), 0);
        assert_eq!(start_roll.read_bits(2).unwrap(), 0);
        assert_eq!(start_roll.read_bits(3).unwrap(), 1);
        assert!(send_rx.try_recv().is_err());

        let remote_loot_list = candidate_rx.try_recv().unwrap();
        let mut remote_loot_list = WorldPacket::from_bytes(&remote_loot_list);
        assert_eq!(
            remote_loot_list.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootList as u16
        );
        let remote_start_roll = candidate_rx.try_recv().unwrap();
        let mut remote_start_roll = WorldPacket::from_bytes(&remote_start_roll);
        assert_eq!(
            remote_start_roll.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::StartLootRoll as u16
        );
        assert_eq!(remote_start_roll.read_packed_guid().unwrap(), loot_object);

        let state = session
            .represented_loot_rolls
            .get(&(loot_object, 0))
            .unwrap();
        assert_eq!(
            state.voters.get(&player_guid).unwrap().vote,
            ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP
        );
        assert_eq!(
            state.voters.get(&candidate_guid).unwrap().vote,
            ROLL_VOTE_NOT_EMITTED_YET_LIKE_CPP
        );
        assert_eq!(
            state.voters.get(&disconnected_guid).unwrap().vote,
            ROLL_VOTE_NOT_VALID_LIKE_CPP
        );

        let entry = &session.loot_table.get(&owner_guid).unwrap().items[0];
        assert!(entry.flags.blocked);
        assert!(!entry.flags.under_threshold);
        assert!(
            session
                .loot_table
                .get(&owner_guid)
                .unwrap()
                .looted_by_player
        );
    }

    #[tokio::test]
    async fn loot_unit_group_loot_can_only_roll_greed_removes_need_from_start_mask_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(4);
        let player_guid = ObjectGuid::create_player(1, 42);
        let candidate_guid = ObjectGuid::create_player(1, 77);
        let owner_guid = test_creature_guid(19_058);
        let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
        let (candidate_tx, _candidate_rx) = flume::bounded::<Vec<u8>>(4);
        let player_registry = Arc::new(PlayerRegistry::default());
        player_registry.insert(candidate_guid, broadcast_info(candidate_guid, candidate_tx));
        session.set_player_registry(player_registry);
        session.set_player_guid(Some(player_guid));
        install_group_loot_group(&mut session, player_guid, candidate_guid);
        install_limited_test_item_template_with_flags2(
            &mut session,
            25,
            0,
            ItemFlags2::CanOnlyRollGreed as u32,
        );
        register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
        session.loot_table.insert(
            owner_guid,
            CreatureLoot {
                loot_guid: loot_object,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: vec![player_guid, candidate_guid],
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags {
                        follow_loot_rules: true,
                        blocked: true,
                        ..Default::default()
                    },
                    allowed_looters: vec![player_guid, candidate_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        session.handle_loot_unit(loot_unit_packet(owner_guid)).await;

        let _response = send_rx.try_recv().unwrap();
        let _loot_list = send_rx.try_recv().unwrap();
        let start_roll = send_rx.try_recv().unwrap();
        let mut start_roll = WorldPacket::from_bytes(&start_roll);
        assert_eq!(
            start_roll.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::StartLootRoll as u16
        );
        assert_eq!(start_roll.read_packed_guid().unwrap(), loot_object);
        assert_eq!(start_roll.read_int32().unwrap(), 0);
        assert_eq!(start_roll.read_uint32().unwrap(), 60_000);
        assert_eq!(
            start_roll.read_uint8().unwrap(),
            ROLL_ALL_TYPE_NO_DISENCHANT_LIKE_CPP & !ROLL_FLAG_TYPE_NEED_LIKE_CPP
        );
    }

    #[tokio::test]
    async fn loot_unit_group_loot_disenchant_mask_uses_cpp_skill_required_gate() {
        let (mut session, send_rx) = make_session_with_send_capacity(4);
        let player_guid = ObjectGuid::create_player(1, 42);
        let candidate_guid = ObjectGuid::create_player(1, 77);
        let owner_guid = test_creature_guid(19_059);
        let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
        let (candidate_tx, _candidate_rx) = flume::bounded::<Vec<u8>>(4);
        let player_registry = Arc::new(PlayerRegistry::default());
        let mut candidate_info = broadcast_info(candidate_guid, candidate_tx);
        candidate_info.enchanting_skill = 175;
        player_registry.insert(candidate_guid, candidate_info);
        session.set_player_registry(player_registry);
        session.set_player_guid(Some(player_guid));
        install_group_loot_group(&mut session, player_guid, candidate_guid);
        install_disenchantable_test_item_template(&mut session, 25);
        register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
        session.loot_table.insert(
            owner_guid,
            CreatureLoot {
                loot_guid: loot_object,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: vec![player_guid, candidate_guid],
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags {
                        follow_loot_rules: true,
                        blocked: true,
                        ..Default::default()
                    },
                    allowed_looters: vec![player_guid, candidate_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        session.handle_loot_unit(loot_unit_packet(owner_guid)).await;

        let _response = send_rx.try_recv().unwrap();
        let _loot_list = send_rx.try_recv().unwrap();
        let start_roll = send_rx.try_recv().unwrap();
        let mut start_roll = WorldPacket::from_bytes(&start_roll);
        assert_eq!(
            start_roll.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::StartLootRoll as u16
        );
        assert_eq!(start_roll.read_packed_guid().unwrap(), loot_object);
        assert_eq!(start_roll.read_int32().unwrap(), 0);
        assert_eq!(start_roll.read_uint32().unwrap(), 60_000);
        assert_eq!(start_roll.read_uint8().unwrap(), 0x0F);
    }

    #[test]
    fn represented_disenchant_loot_template_row_guards_match_cpp_shape() {
        let valid = LootStoreItem {
            item_id: 10940,
            reference: 0,
            chance: 100.0,
            needs_quest: false,
            loot_mode: super::LOOT_MODE_DEFAULT_LIKE_CPP,
            group_id: 0,
            min_count: 1,
            max_count: 2,
        };
        assert!(super::represented_disenchant_loot_plain_row_can_roll_like_cpp(&valid, true));

        let mut missing_item = valid;
        missing_item.item_id = 0;
        assert!(
            !super::represented_disenchant_loot_plain_row_can_roll_like_cpp(&missing_item, true)
        );

        let mut bad_count = valid;
        bad_count.max_count = 0;
        assert!(!super::represented_disenchant_loot_plain_row_can_roll_like_cpp(&bad_count, true));

        let reference = LootStoreItem {
            item_id: 0,
            reference: 700,
            chance: 100.0,
            needs_quest: false,
            loot_mode: super::LOOT_MODE_DEFAULT_LIKE_CPP,
            group_id: 0,
            min_count: 1,
            max_count: 1,
        };
        assert!(super::represented_disenchant_loot_reference_row_can_roll_like_cpp(&reference));
    }

    #[test]
    fn represented_disenchant_loot_template_frame_splits_group_rows_like_cpp() {
        let rows = vec![
            LootStoreItem {
                item_id: 10940,
                reference: 0,
                chance: 100.0,
                needs_quest: false,
                loot_mode: super::LOOT_MODE_DEFAULT_LIKE_CPP,
                group_id: 0,
                min_count: 1,
                max_count: 1,
            },
            LootStoreItem {
                item_id: 10978,
                reference: 0,
                chance: 0.0,
                needs_quest: false,
                loot_mode: super::LOOT_MODE_DEFAULT_LIKE_CPP,
                group_id: 2,
                min_count: 1,
                max_count: 1,
            },
            LootStoreItem {
                item_id: 0,
                reference: 700,
                chance: 100.0,
                needs_quest: false,
                loot_mode: super::LOOT_MODE_DEFAULT_LIKE_CPP,
                group_id: 2,
                min_count: 1,
                max_count: 1,
            },
        ];

        let frame = super::disenchant_loot_template_frame_like_cpp(rows, 0);

        assert_eq!(frame.template.entries().len(), 2);
        assert_eq!(frame.template.groups().len(), 2);
        assert_eq!(frame.template.groups()[1].equal_chanced().len(), 1);
        assert_eq!(frame.template.entries()[1].reference, 700);
        assert_eq!(frame.template.entries()[1].group_id, 2);
    }

    #[tokio::test]
    async fn loot_unit_group_loot_single_candidate_unblocks_under_threshold_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(4);
        let player_guid = ObjectGuid::create_player(1, 42);
        let candidate_guid = ObjectGuid::create_player(1, 77);
        let owner_guid = test_creature_guid(19_050);
        let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
        session.set_player_guid(Some(player_guid));
        install_group_loot_group(&mut session, player_guid, candidate_guid);
        register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
        session.loot_table.insert(
            owner_guid,
            CreatureLoot {
                loot_guid: loot_object,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: vec![player_guid],
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags {
                        follow_loot_rules: true,
                        blocked: true,
                        ..Default::default()
                    },
                    allowed_looters: vec![player_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        session.handle_loot_unit(loot_unit_packet(owner_guid)).await;

        let _response = send_rx.try_recv().unwrap();
        let _loot_list = send_rx.try_recv().unwrap();
        assert!(send_rx.try_recv().is_err());

        let entry = &session.loot_table.get(&owner_guid).unwrap().items[0];
        assert!(!entry.flags.blocked);
        assert!(entry.flags.under_threshold);
    }

    #[tokio::test]
    async fn loot_unit_group_loot_pass_on_loot_suppresses_current_prompt_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(4);
        let player_guid = ObjectGuid::create_player(1, 42);
        let candidate_guid = ObjectGuid::create_player(1, 77);
        let owner_guid = test_creature_guid(19_051);
        let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
        let (candidate_tx, candidate_rx) = flume::bounded::<Vec<u8>>(4);
        let player_registry = Arc::new(PlayerRegistry::default());
        player_registry.insert(candidate_guid, broadcast_info(candidate_guid, candidate_tx));
        session.set_player_registry(player_registry);
        session.set_player_guid(Some(player_guid));
        session.pass_on_group_loot = true;
        install_group_loot_group(&mut session, player_guid, candidate_guid);
        register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
        session.loot_table.insert(
            owner_guid,
            CreatureLoot {
                loot_guid: loot_object,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: vec![player_guid, candidate_guid],
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags {
                        follow_loot_rules: true,
                        blocked: true,
                        ..Default::default()
                    },
                    allowed_looters: vec![player_guid, candidate_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        session.handle_loot_unit(loot_unit_packet(owner_guid)).await;

        let _response = send_rx.try_recv().unwrap();
        let _loot_list = send_rx.try_recv().unwrap();
        let local_auto_pass = send_rx.try_recv().unwrap();
        let mut local_auto_pass = WorldPacket::from_bytes(&local_auto_pass);
        assert_eq!(
            local_auto_pass.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootRoll as u16
        );
        assert_eq!(local_auto_pass.read_packed_guid().unwrap(), loot_object);
        assert_eq!(local_auto_pass.read_packed_guid().unwrap(), player_guid);
        assert_eq!(local_auto_pass.read_int32().unwrap(), -1);
        assert_eq!(
            local_auto_pass.read_uint8().unwrap(),
            ROLL_VOTE_PASS_LIKE_CPP
        );
        assert!(send_rx.try_recv().is_err());

        let _remote_loot_list = candidate_rx.try_recv().unwrap();
        let remote_start_roll = candidate_rx.try_recv().unwrap();
        let mut remote_start_roll = WorldPacket::from_bytes(&remote_start_roll);
        assert_eq!(
            remote_start_roll.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::StartLootRoll as u16
        );
        let remote_auto_pass = candidate_rx.try_recv().unwrap();
        let mut remote_auto_pass = WorldPacket::from_bytes(&remote_auto_pass);
        assert_eq!(
            remote_auto_pass.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootRoll as u16
        );
        assert_eq!(remote_auto_pass.read_packed_guid().unwrap(), loot_object);
        assert_eq!(remote_auto_pass.read_packed_guid().unwrap(), player_guid);
        assert_eq!(remote_auto_pass.read_int32().unwrap(), -1);
        assert_eq!(
            remote_auto_pass.read_uint8().unwrap(),
            ROLL_VOTE_PASS_LIKE_CPP
        );

        let entry = &session.loot_table.get(&owner_guid).unwrap().items[0];
        assert!(entry.flags.blocked);
        assert!(!entry.flags.under_threshold);
    }

    #[tokio::test]
    async fn loot_roll_need_vote_broadcasts_immediate_roll_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(5);
        let player_guid = ObjectGuid::create_player(1, 42);
        let candidate_guid = ObjectGuid::create_player(1, 77);
        let owner_guid = test_creature_guid(19_052);
        let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
        let (candidate_tx, candidate_rx) = flume::bounded::<Vec<u8>>(5);
        let player_registry = Arc::new(PlayerRegistry::default());
        player_registry.insert(candidate_guid, broadcast_info(candidate_guid, candidate_tx));
        session.set_player_registry(player_registry);
        session.set_player_guid(Some(player_guid));
        install_group_loot_group(&mut session, player_guid, candidate_guid);
        register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
        session.loot_table.insert(
            owner_guid,
            CreatureLoot {
                loot_guid: loot_object,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: vec![player_guid, candidate_guid],
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags {
                        follow_loot_rules: true,
                        blocked: true,
                        ..Default::default()
                    },
                    allowed_looters: vec![player_guid, candidate_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        session.handle_loot_unit(loot_unit_packet(owner_guid)).await;
        let _response = send_rx.try_recv().unwrap();
        let _loot_list = send_rx.try_recv().unwrap();
        let _start_roll = send_rx.try_recv().unwrap();
        let _remote_loot_list = candidate_rx.try_recv().unwrap();
        let _remote_start_roll = candidate_rx.try_recv().unwrap();

        session
            .handle_loot_roll(LootRoll {
                loot_obj: loot_object,
                loot_list_id: 0,
                roll_type: ROLL_VOTE_NEED_LIKE_CPP,
            })
            .await;

        let local_roll = send_rx.try_recv().unwrap();
        let mut local_roll = WorldPacket::from_bytes(&local_roll);
        assert_eq!(
            local_roll.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootRoll as u16
        );
        assert_eq!(local_roll.read_packed_guid().unwrap(), loot_object);
        assert_eq!(local_roll.read_packed_guid().unwrap(), player_guid);
        assert_eq!(local_roll.read_int32().unwrap(), 0);
        assert_eq!(local_roll.read_uint8().unwrap(), ROLL_VOTE_NEED_LIKE_CPP);
        assert_eq!(local_roll.read_int32().unwrap(), 0);
        assert_eq!(local_roll.read_bits(2).unwrap(), 0);
        assert_eq!(local_roll.read_bits(3).unwrap(), 1);

        let remote_roll = candidate_rx.try_recv().unwrap();
        let mut remote_roll = WorldPacket::from_bytes(&remote_roll);
        assert_eq!(
            remote_roll.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootRoll as u16
        );
        assert_eq!(remote_roll.read_packed_guid().unwrap(), loot_object);
        assert_eq!(remote_roll.read_packed_guid().unwrap(), player_guid);
    }

    #[tokio::test]
    async fn loot_roll_all_voted_finishes_need_winner_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(8);
        let player_guid = ObjectGuid::create_player(1, 42);
        let candidate_guid = ObjectGuid::create_player(1, 77);
        let owner_guid = test_creature_guid(19_053);
        let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
        let (player_tx, player_rx) = flume::bounded::<Vec<u8>>(8);
        let (candidate_tx, candidate_rx) = flume::bounded::<Vec<u8>>(8);
        let player_registry = Arc::new(PlayerRegistry::default());
        player_registry.insert(player_guid, broadcast_info(player_guid, player_tx));
        player_registry.insert(candidate_guid, broadcast_info(candidate_guid, candidate_tx));
        session.set_player_registry(player_registry);
        session.set_player_guid(Some(player_guid));
        install_group_loot_group(&mut session, player_guid, candidate_guid);
        register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
        session.loot_table.insert(
            owner_guid,
            CreatureLoot {
                loot_guid: loot_object,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: vec![player_guid, candidate_guid],
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags {
                        follow_loot_rules: true,
                        blocked: true,
                        ..Default::default()
                    },
                    allowed_looters: vec![player_guid, candidate_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        session.handle_loot_unit(loot_unit_packet(owner_guid)).await;
        let _response = send_rx.try_recv().unwrap();
        let _loot_list = send_rx.try_recv().unwrap();
        let _start_roll = send_rx.try_recv().unwrap();
        let _remote_loot_list = candidate_rx.try_recv().unwrap();
        let _remote_start_roll = candidate_rx.try_recv().unwrap();

        session
            .handle_loot_roll(LootRoll {
                loot_obj: loot_object,
                loot_list_id: 0,
                roll_type: ROLL_VOTE_NEED_LIKE_CPP,
            })
            .await;
        let _local_need_roll = send_rx.try_recv().unwrap();
        let _remote_need_roll = candidate_rx.try_recv().unwrap();

        session.set_player_guid(Some(candidate_guid));
        session
            .handle_loot_roll(LootRoll {
                loot_obj: loot_object,
                loot_list_id: 0,
                roll_type: ROLL_VOTE_GREED_LIKE_CPP,
            })
            .await;

        let local_greed_roll = send_rx.try_recv().unwrap();
        let mut local_greed_roll = WorldPacket::from_bytes(&local_greed_roll);
        assert_eq!(
            local_greed_roll.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootRoll as u16
        );
        assert_eq!(local_greed_roll.read_packed_guid().unwrap(), loot_object);
        assert_eq!(local_greed_roll.read_packed_guid().unwrap(), candidate_guid);

        let mut local_won_locked =
            recv_packet_with_opcode(&send_rx, wow_constants::ServerOpcodes::LootRollWon);
        assert_eq!(local_won_locked.read_packed_guid().unwrap(), loot_object);
        assert_eq!(local_won_locked.read_packed_guid().unwrap(), player_guid);
        let winner_roll = local_won_locked.read_int32().unwrap();
        assert!((1..=100).contains(&winner_roll));
        assert_eq!(
            local_won_locked.read_uint8().unwrap(),
            ROLL_VOTE_NEED_LIKE_CPP
        );
        assert_eq!(local_won_locked.read_int32().unwrap(), 0);
        assert_eq!(local_won_locked.read_bits(2).unwrap(), 0);
        assert_eq!(local_won_locked.read_bits(3).unwrap(), 2);

        let mut original_greed_roll =
            recv_packet_with_opcode(&player_rx, wow_constants::ServerOpcodes::LootRoll);
        assert_eq!(original_greed_roll.read_packed_guid().unwrap(), loot_object);
        assert_eq!(
            original_greed_roll.read_packed_guid().unwrap(),
            candidate_guid
        );

        let final_replay_to_winner =
            recv_packet_with_opcode(&player_rx, wow_constants::ServerOpcodes::LootRoll);
        let mut final_replay_to_winner = final_replay_to_winner;
        assert!(matches!(
            final_replay_to_winner.read_packed_guid().unwrap(),
            guid if guid == loot_object
        ));
        let _replay_player = final_replay_to_winner.read_packed_guid().unwrap();
        let replay_roll = final_replay_to_winner.read_int32().unwrap();
        assert!((0..=100).contains(&replay_roll));

        let mut original_won_allow =
            recv_packet_with_opcode(&player_rx, wow_constants::ServerOpcodes::LootRollWon);
        assert_eq!(original_won_allow.read_packed_guid().unwrap(), loot_object);
        assert_eq!(original_won_allow.read_packed_guid().unwrap(), player_guid);
        let _roll = original_won_allow.read_int32().unwrap();
        assert_eq!(
            original_won_allow.read_uint8().unwrap(),
            ROLL_VOTE_NEED_LIKE_CPP
        );
        assert_eq!(original_won_allow.read_int32().unwrap(), 0);
        assert_eq!(original_won_allow.read_bits(2).unwrap(), 0);
        assert_eq!(original_won_allow.read_bits(3).unwrap(), 0);

        let entry = &session.loot_table.get(&owner_guid).unwrap().items[0];
        assert!(!entry.flags.blocked);
        assert_eq!(entry.roll_winner, player_guid);
        assert!(
            !session
                .represented_loot_rolls
                .contains_key(&(loot_object, 0))
        );
        assert_eq!(
            session.represented_loot_roll_criteria_events[0],
            RepresentedLootRollCriteriaEvent::RollAnyNeed {
                player_guid,
                quantity: 1
            }
        );
        assert_eq!(
            session.represented_loot_roll_criteria_events[1],
            RepresentedLootRollCriteriaEvent::RollAnyGreed {
                player_guid: candidate_guid,
                quantity: 1
            }
        );
        match session.represented_loot_roll_criteria_events[2] {
            RepresentedLootRollCriteriaEvent::RollNeed {
                player_guid: criteria_player,
                item_id,
                roll_number,
            } => {
                assert_eq!(criteria_player, player_guid);
                assert_eq!(item_id, 25);
                assert!((1..=100).contains(&roll_number));
            }
            other => panic!("unexpected criteria event: {other:?}"),
        }
    }

    #[tokio::test]
    async fn loot_roll_timer_expiry_finishes_current_winner_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(8);
        let player_guid = ObjectGuid::create_player(1, 42);
        let candidate_guid = ObjectGuid::create_player(1, 77);
        let owner_guid = test_creature_guid(19_057);
        let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
        let (candidate_tx, candidate_rx) = flume::bounded::<Vec<u8>>(8);
        let player_registry = Arc::new(PlayerRegistry::default());
        player_registry.insert(candidate_guid, broadcast_info(candidate_guid, candidate_tx));
        session.set_player_registry(player_registry);
        session.set_player_guid(Some(player_guid));
        install_group_loot_group(&mut session, player_guid, candidate_guid);
        register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
        session.loot_table.insert(
            owner_guid,
            CreatureLoot {
                loot_guid: loot_object,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: vec![player_guid, candidate_guid],
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags {
                        follow_loot_rules: true,
                        blocked: true,
                        ..Default::default()
                    },
                    allowed_looters: vec![player_guid, candidate_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        session.handle_loot_unit(loot_unit_packet(owner_guid)).await;
        let _response = send_rx.try_recv().unwrap();
        let _loot_list = send_rx.try_recv().unwrap();
        let _start_roll = send_rx.try_recv().unwrap();
        let _remote_loot_list = candidate_rx.try_recv().unwrap();
        let _remote_start_roll = candidate_rx.try_recv().unwrap();

        session
            .handle_loot_roll(LootRoll {
                loot_obj: loot_object,
                loot_list_id: 0,
                roll_type: ROLL_VOTE_GREED_LIKE_CPP,
            })
            .await;
        let _local_greed_roll = send_rx.try_recv().unwrap();
        let _remote_greed_roll = candidate_rx.try_recv().unwrap();

        session
            .represented_loot_rolls
            .get_mut(&(loot_object, 0))
            .unwrap()
            .end_time = Instant::now() - Duration::from_millis(1);
        session.tick_represented_loot_rolls_like_cpp().await;

        let mut local_final_replay =
            recv_packet_with_opcode(&send_rx, wow_constants::ServerOpcodes::LootRoll);
        assert_eq!(local_final_replay.read_packed_guid().unwrap(), loot_object);
        let _replay_player = local_final_replay.read_packed_guid().unwrap();

        let mut local_won_allow =
            recv_packet_with_opcode(&send_rx, wow_constants::ServerOpcodes::LootRollWon);
        assert_eq!(local_won_allow.read_packed_guid().unwrap(), loot_object);
        assert_eq!(local_won_allow.read_packed_guid().unwrap(), player_guid);
        assert!((1..=100).contains(&local_won_allow.read_int32().unwrap()));
        assert_eq!(
            local_won_allow.read_uint8().unwrap(),
            ROLL_VOTE_GREED_LIKE_CPP
        );

        let mut remote_final_replay =
            recv_packet_with_opcode(&candidate_rx, wow_constants::ServerOpcodes::LootRoll);
        assert_eq!(remote_final_replay.read_packed_guid().unwrap(), loot_object);
        let _remote_replay_player = remote_final_replay.read_packed_guid().unwrap();

        let mut remote_won_locked =
            recv_packet_with_opcode(&candidate_rx, wow_constants::ServerOpcodes::LootRollWon);
        assert_eq!(remote_won_locked.read_packed_guid().unwrap(), loot_object);
        assert_eq!(remote_won_locked.read_packed_guid().unwrap(), player_guid);
        assert!((1..=100).contains(&remote_won_locked.read_int32().unwrap()));
        assert_eq!(
            remote_won_locked.read_uint8().unwrap(),
            ROLL_VOTE_GREED_LIKE_CPP
        );

        let entry = &session.loot_table.get(&owner_guid).unwrap().items[0];
        assert!(!entry.flags.blocked);
        assert_eq!(entry.roll_winner, player_guid);
        assert!(
            !session
                .represented_loot_rolls
                .contains_key(&(loot_object, 0))
        );
        assert_eq!(
            session.represented_loot_roll_criteria_events[0],
            RepresentedLootRollCriteriaEvent::RollAnyGreed {
                player_guid,
                quantity: 1
            }
        );
        match session.represented_loot_roll_criteria_events[1] {
            RepresentedLootRollCriteriaEvent::RollGreed {
                player_guid: criteria_player,
                item_id,
                roll_number,
            } => {
                assert_eq!(criteria_player, player_guid);
                assert_eq!(item_id, 25);
                assert!((1..=100).contains(&roll_number));
            }
            other => panic!("unexpected criteria event: {other:?}"),
        }
    }

    #[tokio::test]
    async fn loot_roll_all_passed_unblocks_without_all_passed_to_valid_voters_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(8);
        let player_guid = ObjectGuid::create_player(1, 42);
        let candidate_guid = ObjectGuid::create_player(1, 77);
        let owner_guid = test_creature_guid(19_054);
        let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
        let (player_tx, player_rx) = flume::bounded::<Vec<u8>>(8);
        let (candidate_tx, candidate_rx) = flume::bounded::<Vec<u8>>(8);
        let player_registry = Arc::new(PlayerRegistry::default());
        player_registry.insert(player_guid, broadcast_info(player_guid, player_tx));
        player_registry.insert(candidate_guid, broadcast_info(candidate_guid, candidate_tx));
        session.set_player_registry(player_registry);
        session.set_player_guid(Some(player_guid));
        install_group_loot_group(&mut session, player_guid, candidate_guid);
        register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
        session.loot_table.insert(
            owner_guid,
            CreatureLoot {
                loot_guid: loot_object,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: vec![player_guid, candidate_guid],
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags {
                        follow_loot_rules: true,
                        blocked: true,
                        ..Default::default()
                    },
                    allowed_looters: vec![player_guid, candidate_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        session.handle_loot_unit(loot_unit_packet(owner_guid)).await;
        let _response = send_rx.try_recv().unwrap();
        let _loot_list = send_rx.try_recv().unwrap();
        let _start_roll = send_rx.try_recv().unwrap();
        let _remote_loot_list = candidate_rx.try_recv().unwrap();
        let _remote_start_roll = candidate_rx.try_recv().unwrap();

        session
            .handle_loot_roll(LootRoll {
                loot_obj: loot_object,
                loot_list_id: 0,
                roll_type: ROLL_VOTE_PASS_LIKE_CPP,
            })
            .await;
        let _local_pass_roll = send_rx.try_recv().unwrap();
        let _remote_pass_roll = candidate_rx.try_recv().unwrap();

        session.set_player_guid(Some(candidate_guid));
        session
            .handle_loot_roll(LootRoll {
                loot_obj: loot_object,
                loot_list_id: 0,
                roll_type: ROLL_VOTE_PASS_LIKE_CPP,
            })
            .await;

        let candidate_pass_roll = send_rx.try_recv().unwrap();
        let mut candidate_pass_roll = WorldPacket::from_bytes(&candidate_pass_roll);
        assert_eq!(
            candidate_pass_roll.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootRoll as u16
        );
        assert_eq!(candidate_pass_roll.read_packed_guid().unwrap(), loot_object);
        assert_eq!(
            candidate_pass_roll.read_packed_guid().unwrap(),
            candidate_guid
        );
        assert_eq!(candidate_pass_roll.read_int32().unwrap(), -1);
        assert_eq!(
            candidate_pass_roll.read_uint8().unwrap(),
            ROLL_VOTE_PASS_LIKE_CPP
        );

        let original_pass_roll = player_rx.try_recv().unwrap();
        let mut original_pass_roll = WorldPacket::from_bytes(&original_pass_roll);
        assert_eq!(
            original_pass_roll.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootRoll as u16
        );
        assert!(send_rx.try_recv().is_err());
        assert!(player_rx.try_recv().is_err());
        assert!(candidate_rx.try_recv().is_err());

        let entry = &session.loot_table.get(&owner_guid).unwrap().items[0];
        assert!(!entry.flags.blocked);
        assert!(entry.roll_winner.is_empty());
        assert!(
            !session
                .represented_loot_rolls
                .contains_key(&(loot_object, 0))
        );
    }

    #[tokio::test]
    async fn loot_roll_vote_command_updates_owner_session_roll_state_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(8);
        let player_guid = ObjectGuid::create_player(1, 42);
        let candidate_guid = ObjectGuid::create_player(1, 77);
        let owner_guid = test_creature_guid(19_055);
        let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
        let (candidate_tx, candidate_rx) = flume::bounded::<Vec<u8>>(8);
        let player_registry = Arc::new(PlayerRegistry::default());
        player_registry.insert(candidate_guid, broadcast_info(candidate_guid, candidate_tx));
        session.set_player_registry(player_registry);
        session.set_player_guid(Some(player_guid));
        install_group_loot_group(&mut session, player_guid, candidate_guid);
        register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
        session.loot_table.insert(
            owner_guid,
            CreatureLoot {
                loot_guid: loot_object,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: vec![player_guid, candidate_guid],
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags {
                        follow_loot_rules: true,
                        blocked: true,
                        ..Default::default()
                    },
                    allowed_looters: vec![player_guid, candidate_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        session.handle_loot_unit(loot_unit_packet(owner_guid)).await;
        let _response = send_rx.try_recv().unwrap();
        let _loot_list = send_rx.try_recv().unwrap();
        let _start_roll = send_rx.try_recv().unwrap();
        let _remote_loot_list = candidate_rx.try_recv().unwrap();
        let _remote_start_roll = candidate_rx.try_recv().unwrap();

        session
            .session_command_tx()
            .send(SessionCommand::LootRollVote(LootRollVoteCommand {
                voter_guid: candidate_guid,
                loot_obj: loot_object,
                loot_list_id: 0,
                roll_type: ROLL_VOTE_GREED_LIKE_CPP,
                pass_on_group_loot: false,
            }))
            .unwrap();
        session
            .process_represented_session_commands_like_cpp()
            .await;

        let local_roll = send_rx.try_recv().unwrap();
        let mut local_roll = WorldPacket::from_bytes(&local_roll);
        assert_eq!(
            local_roll.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootRoll as u16
        );
        assert_eq!(local_roll.read_packed_guid().unwrap(), loot_object);
        assert_eq!(local_roll.read_packed_guid().unwrap(), candidate_guid);
        assert_eq!(local_roll.read_int32().unwrap(), -1);
        assert_eq!(local_roll.read_uint8().unwrap(), ROLL_VOTE_GREED_LIKE_CPP);

        let remote_roll = candidate_rx.try_recv().unwrap();
        let mut remote_roll = WorldPacket::from_bytes(&remote_roll);
        assert_eq!(
            remote_roll.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootRoll as u16
        );
        assert_eq!(remote_roll.read_packed_guid().unwrap(), loot_object);
        assert_eq!(remote_roll.read_packed_guid().unwrap(), candidate_guid);

        let state = session
            .represented_loot_rolls
            .get(&(loot_object, 0))
            .unwrap();
        assert_eq!(
            state.voters.get(&candidate_guid).unwrap().vote,
            ROLL_VOTE_GREED_LIKE_CPP
        );
    }

    #[tokio::test]
    async fn loot_roll_remote_session_routes_vote_to_owner_session_like_cpp() {
        let (mut owner_session, owner_rx) = make_session_with_send_capacity(8);
        let (mut remote_session, _remote_rx) = make_session_with_send_capacity(2);
        let player_guid = ObjectGuid::create_player(1, 42);
        let candidate_guid = ObjectGuid::create_player(1, 77);
        let owner_guid = test_creature_guid(19_056);
        let loot_object = represented_loot_object_guid_like_cpp(owner_guid);
        let (candidate_tx, candidate_rx) = flume::bounded::<Vec<u8>>(8);
        let (owner_registry_tx, _owner_registry_rx) = flume::bounded::<Vec<u8>>(8);
        let player_registry = Arc::new(PlayerRegistry::default());

        let mut owner_info = broadcast_info(player_guid, owner_registry_tx);
        owner_info.command_tx = owner_session.session_command_tx();
        player_registry.insert(player_guid, owner_info);
        player_registry.insert(candidate_guid, broadcast_info(candidate_guid, candidate_tx));

        owner_session.set_player_registry(Arc::clone(&player_registry));
        owner_session.set_player_guid(Some(player_guid));
        remote_session.set_player_registry(Arc::clone(&player_registry));
        remote_session.set_player_guid(Some(candidate_guid));
        install_group_loot_group(&mut owner_session, player_guid, candidate_guid);
        register_test_creature_like_cpp(&mut owner_session, test_creature(owner_guid, false));
        owner_session.loot_table.insert(
            owner_guid,
            CreatureLoot {
                loot_guid: loot_object,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: LOOT_METHOD_GROUP_LIKE_CPP,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: vec![player_guid, candidate_guid],
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags {
                        follow_loot_rules: true,
                        blocked: true,
                        ..Default::default()
                    },
                    allowed_looters: vec![player_guid, candidate_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        owner_session
            .handle_loot_unit(loot_unit_packet(owner_guid))
            .await;
        let _response = owner_rx.try_recv().unwrap();
        let _loot_list = owner_rx.try_recv().unwrap();
        let _start_roll = owner_rx.try_recv().unwrap();
        let _remote_loot_list = candidate_rx.try_recv().unwrap();
        let _remote_start_roll = candidate_rx.try_recv().unwrap();

        assert!(
            player_registry
                .get(&player_guid)
                .unwrap()
                .active_loot_rolls
                .contains(&(loot_object, 0))
        );

        remote_session
            .handle_loot_roll(LootRoll {
                loot_obj: loot_object,
                loot_list_id: 0,
                roll_type: ROLL_VOTE_GREED_LIKE_CPP,
            })
            .await;
        owner_session
            .process_represented_session_commands_like_cpp()
            .await;

        let local_roll = owner_rx.try_recv().unwrap();
        let mut local_roll = WorldPacket::from_bytes(&local_roll);
        assert_eq!(
            local_roll.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootRoll as u16
        );
        assert_eq!(local_roll.read_packed_guid().unwrap(), loot_object);
        assert_eq!(local_roll.read_packed_guid().unwrap(), candidate_guid);
        assert_eq!(local_roll.read_int32().unwrap(), -1);
        assert_eq!(local_roll.read_uint8().unwrap(), ROLL_VOTE_GREED_LIKE_CPP);

        let remote_roll = candidate_rx.try_recv().unwrap();
        let mut remote_roll = WorldPacket::from_bytes(&remote_roll);
        assert_eq!(
            remote_roll.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootRoll as u16
        );
        assert_eq!(remote_roll.read_packed_guid().unwrap(), loot_object);
        assert_eq!(remote_roll.read_packed_guid().unwrap(), candidate_guid);
    }

    #[tokio::test]
    async fn loot_unit_new_main_target_releases_existing_view_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(2);
        let player_guid = ObjectGuid::create_player(1, 42);
        let old_guid = test_creature_guid(19_036);
        let new_guid = test_creature_guid(19_037);
        session.set_player_guid(Some(player_guid));
        session.set_active_loot_guid(old_guid);
        insert_allowed_coin_loot_like_cpp(&mut session, old_guid, player_guid, 7);
        register_test_creature_like_cpp(&mut session, test_creature(new_guid, false));
        insert_allowed_coin_loot_like_cpp(&mut session, new_guid, player_guid, 7);

        session.handle_loot_unit(loot_unit_packet(new_guid)).await;

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootRelease as u16
        );
        assert_eq!(sent.read_packed_guid().unwrap(), old_guid);
        assert_eq!(sent.read_packed_guid().unwrap(), player_guid);

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootResponse as u16
        );
        assert!(!session.is_active_loot_guid(old_guid));
        assert!(session.is_active_loot_guid(new_guid));
        assert!(session.loot_table.contains_key(&old_guid));
    }

    #[tokio::test]
    async fn loot_unit_non_creature_guid_returns_silently_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let loot_guid = test_gameobject_guid(19_019);
        session.set_player_guid(Some(player_guid));

        session.handle_loot_unit(loot_unit_packet(loot_guid)).await;

        assert!(send_rx.try_recv().is_err());
        assert!(!session.is_active_loot_guid(loot_guid));
        assert!(!session.loot_table.contains_key(&loot_guid));
    }

    #[tokio::test]
    async fn loot_unit_creature_too_far_returns_silently_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let loot_guid = test_creature_guid(19_016);
        session.set_player_guid(Some(player_guid));
        session.set_player_position_like_cpp(Position::ZERO);
        let mut creature = test_creature(loot_guid, false);
        creature.current_pos = Position::new(31.0, 0.0, 0.0, 0.0);
        register_test_creature_like_cpp(&mut session, creature);

        session.handle_loot_unit(loot_unit_packet(loot_guid)).await;

        assert!(send_rx.try_recv().is_err());
        assert!(!session.is_active_loot_guid(loot_guid));
        assert!(!session.loot_table.contains_key(&loot_guid));
    }

    #[tokio::test]
    async fn loot_unit_response_uses_loot_owner_not_player_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let owner_guid = test_creature_guid(19_022);
        session.set_player_guid(Some(player_guid));
        register_test_creature_like_cpp(&mut session, test_creature(owner_guid, false));
        insert_allowed_coin_loot_like_cpp(&mut session, owner_guid, player_guid, 7);

        session.handle_loot_unit(loot_unit_packet(owner_guid)).await;

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootResponse as u16
        );
        let response_owner = sent.read_packed_guid().unwrap();
        let response_loot_obj = sent.read_packed_guid().unwrap();
        assert_eq!(response_owner, owner_guid);
        assert_eq!(response_loot_obj.high_type(), HighGuid::LootObject);
        assert_ne!(response_loot_obj, owner_guid);
        assert_ne!(owner_guid, player_guid);
        assert_eq!(
            session.loot_table.get(&owner_guid).unwrap().loot_guid,
            response_loot_obj
        );
        assert!(session.is_active_loot_guid(owner_guid));
    }

    #[tokio::test]
    async fn loot_unit_ae_loot_sends_targets_and_secondary_ack_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(5);
        let player_guid = ObjectGuid::create_player(1, 42);
        let main_guid = test_creature_guid(19_031);
        let secondary_guid = test_creature_guid(19_032);
        session.set_player_guid(Some(player_guid));
        session.set_enable_ae_loot_like_cpp(true);
        session.set_player_position_like_cpp(Position::ZERO);
        register_test_creature_like_cpp(&mut session, test_creature(main_guid, false));
        register_test_creature_like_cpp(&mut session, test_creature(secondary_guid, false));
        insert_allowed_coin_loot_like_cpp(&mut session, main_guid, player_guid, 7);
        insert_allowed_coin_loot_like_cpp(&mut session, secondary_guid, player_guid, 7);

        session.handle_loot_unit(loot_unit_packet(main_guid)).await;

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::AeLootTargets as u16
        );
        assert_eq!(sent.read_uint32().unwrap(), 2);

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootResponse as u16
        );
        assert_eq!(sent.read_packed_guid().unwrap(), main_guid);
        let main_loot_object = sent.read_packed_guid().unwrap();
        assert_eq!(main_loot_object.high_type(), HighGuid::LootObject);
        sent.read_uint8().unwrap();
        sent.read_uint8().unwrap();
        sent.read_uint8().unwrap();
        sent.read_uint8().unwrap();
        sent.read_uint32().unwrap();
        sent.read_int32().unwrap();
        sent.read_int32().unwrap();
        assert!(sent.read_bit().unwrap());
        assert!(!sent.read_bit().unwrap());

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::AeLootTargetAck as u16
        );
        assert!(sent.read_uint8().is_err());

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootResponse as u16
        );
        assert_eq!(sent.read_packed_guid().unwrap(), secondary_guid);
        let secondary_loot_object = sent.read_packed_guid().unwrap();
        assert_eq!(secondary_loot_object.high_type(), HighGuid::LootObject);
        sent.read_uint8().unwrap();
        sent.read_uint8().unwrap();
        sent.read_uint8().unwrap();
        sent.read_uint8().unwrap();
        sent.read_uint32().unwrap();
        sent.read_int32().unwrap();
        sent.read_int32().unwrap();
        assert!(sent.read_bit().unwrap());
        assert!(sent.read_bit().unwrap());

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::AeLootTargetAck as u16
        );
        assert!(session.is_active_loot_guid(main_guid));
        assert!(session.active_loot_view_owners.contains(&main_guid));
        assert!(session.active_loot_view_owners.contains(&secondary_guid));
    }

    #[tokio::test]
    async fn loot_unit_empty_visible_loot_returns_silently_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let loot_guid = test_creature_guid(19_007);
        session.set_player_guid(Some(player_guid));
        register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
        session.loot_table.insert(
            loot_guid,
            CreatureLoot {
                loot_guid,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: vec![player_guid],
                items: vec![],
                looted_by_player: false,
            },
        );

        session.handle_loot_unit(loot_unit_packet(loot_guid)).await;

        assert!(send_rx.try_recv().is_err());
        assert!(!session.is_active_loot_guid(loot_guid));
    }

    #[tokio::test]
    async fn loot_unit_fully_looted_existing_loot_returns_silently_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let loot_guid = test_creature_guid(19_017);
        session.set_player_guid(Some(player_guid));
        register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
        session.loot_table.insert(
            loot_guid,
            CreatureLoot {
                loot_guid,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: vec![player_guid],
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags::default(),
                    allowed_looters: vec![player_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: true,
                }],
                looted_by_player: false,
            },
        );

        session.handle_loot_unit(loot_unit_packet(loot_guid)).await;

        assert!(send_rx.try_recv().is_err());
        assert!(!session.is_active_loot_guid(loot_guid));
    }

    #[tokio::test]
    async fn loot_unit_without_allowed_loot_for_player_returns_silently_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let other_guid = ObjectGuid::create_player(1, 43);
        let loot_guid = test_creature_guid(19_018);
        session.set_player_guid(Some(player_guid));
        register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
        session.loot_table.insert(
            loot_guid,
            CreatureLoot {
                loot_guid,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: vec![player_guid],
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags::default(),
                    allowed_looters: vec![other_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        session.handle_loot_unit(loot_unit_packet(loot_guid)).await;

        assert!(send_rx.try_recv().is_err());
        assert_eq!(
            session.loot_table.get(&loot_guid).unwrap().items[0].allowed_looters,
            vec![other_guid]
        );
        assert!(!session.is_active_loot_guid(loot_guid));
    }

    #[tokio::test]
    async fn loot_unit_non_tapper_existing_tap_list_returns_silently_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let other_guid = ObjectGuid::create_player(1, 43);
        let loot_guid = test_creature_guid(19_098);
        session.set_player_guid(Some(player_guid));
        register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
        tap_test_creature_like_cpp(&mut session, loot_guid, other_guid);

        session.handle_loot_unit(loot_unit_packet(loot_guid)).await;

        assert!(send_rx.try_recv().is_err());
        assert!(!session.is_active_loot_guid(loot_guid));
        assert!(!session.loot_table.contains_key(&loot_guid));
    }

    #[tokio::test]
    async fn loot_unit_existing_coin_loot_without_allowed_looter_returns_silently_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let other_guid = ObjectGuid::create_player(1, 43);
        let loot_guid = test_creature_guid(19_099);
        session.set_player_guid(Some(player_guid));
        register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
        insert_allowed_coin_loot_like_cpp(&mut session, loot_guid, other_guid, 7);

        session.handle_loot_unit(loot_unit_packet(loot_guid)).await;

        assert!(send_rx.try_recv().is_err());
        assert!(!session.is_active_loot_guid(loot_guid));
        assert_eq!(session.loot_table.get(&loot_guid).unwrap().coins, 7);
        assert_eq!(
            session.loot_table.get(&loot_guid).unwrap().allowed_looters,
            vec![other_guid]
        );
    }

    #[tokio::test]
    async fn loot_money_non_allowed_active_creature_does_not_take_coins_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let other_guid = ObjectGuid::create_player(1, 43);
        let loot_guid = test_creature_guid(19_100);
        session.set_player_guid(Some(player_guid));
        session.set_active_loot_guid(loot_guid);
        insert_allowed_coin_loot_like_cpp(&mut session, loot_guid, other_guid, 7);

        session.handle_loot_money(loot_money_packet()).await;

        assert!(send_rx.try_recv().is_err());
        assert_eq!(session.player_gold_like_cpp(), 0);
        assert_eq!(session.loot_table.get(&loot_guid).unwrap().coins, 7);
    }

    #[tokio::test]
    async fn loot_item_uses_active_loot_view_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let active_guid = test_creature_guid(19_001);
        let inactive_guid = test_creature_guid(19_002);
        session.set_player_guid(Some(player_guid));
        session.set_active_loot_guid(active_guid);
        session.loot_table.insert(
            inactive_guid,
            CreatureLoot {
                loot_guid: inactive_guid,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: vec![player_guid],
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags::default(),
                    allowed_looters: Vec::new(),
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        session
            .handle_loot_item(loot_item_packet(inactive_guid, 0))
            .await;

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootRelease as u16
        );
        assert_eq!(sent.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
        assert_eq!(sent.read_packed_guid().unwrap(), player_guid);
        assert!(!session.loot_table.get(&inactive_guid).unwrap().items[0].taken);
    }

    #[tokio::test]
    async fn loot_money_stale_active_without_loot_view_returns_silently_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let loot_guid = test_creature_guid(19_020);
        session.set_player_guid(Some(player_guid));
        session.set_active_loot_guid(loot_guid);

        session.handle_loot_money(loot_money_packet()).await;

        assert!(session.is_active_loot_guid(loot_guid));
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn loot_money_zero_money_still_notifies_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(2);
        let player_guid = ObjectGuid::create_player(1, 42);
        let loot_guid = test_creature_guid(19_021);
        session.set_player_guid(Some(player_guid));
        session.set_active_loot_guid(loot_guid);
        session.loot_table.insert(
            loot_guid,
            CreatureLoot {
                loot_guid,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: vec![player_guid],
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags::default(),
                    allowed_looters: vec![player_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        session.handle_loot_money(loot_money_packet()).await;

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::CoinRemoved as u16
        );
        assert_eq!(sent.read_packed_guid().unwrap(), loot_guid);

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootMoneyNotify as u16
        );
        assert_eq!(sent.read_uint64().unwrap(), 0);
        assert_eq!(sent.read_uint64().unwrap(), 0);
        assert!(sent.read_bit().unwrap());
        assert_eq!(session.player_gold_like_cpp(), 0);
        assert!(session.is_active_loot_guid(loot_guid));
    }

    #[tokio::test]
    async fn loot_money_coin_removed_uses_loot_object_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(2);
        let player_guid = ObjectGuid::create_player(1, 42);
        let owner_guid = test_creature_guid(19_024);
        let loot_object_guid = represented_loot_object_guid_like_cpp(owner_guid);
        session.set_player_guid(Some(player_guid));
        session.set_active_loot_guid(owner_guid);
        session.loot_table.insert(
            owner_guid,
            CreatureLoot {
                loot_guid: loot_object_guid,
                coins: 3,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: vec![player_guid],
                items: vec![],
                looted_by_player: false,
            },
        );

        session.handle_loot_money(loot_money_packet()).await;

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::CoinRemoved as u16
        );
        assert_eq!(sent.read_packed_guid().unwrap(), loot_object_guid);
        assert!(session.is_active_loot_guid(owner_guid));
    }

    #[tokio::test]
    async fn loot_money_consumes_all_active_loot_views_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(4);
        let player_guid = ObjectGuid::create_player(1, 42);
        let owner_one = test_creature_guid(19_025);
        let owner_two = test_creature_guid(19_026);
        let loot_object_one = represented_loot_object_guid_like_cpp(owner_one);
        let loot_object_two = represented_loot_object_guid_like_cpp(owner_two);
        session.set_player_guid(Some(player_guid));
        session.set_active_loot_guid(owner_one);
        session.add_active_loot_view_owner_like_cpp(owner_two);
        session.loot_table.insert(
            owner_one,
            CreatureLoot {
                loot_guid: loot_object_one,
                coins: 3,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: vec![player_guid],
                items: vec![],
                looted_by_player: false,
            },
        );
        session.loot_table.insert(
            owner_two,
            CreatureLoot {
                loot_guid: loot_object_two,
                coins: 7,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: vec![player_guid],
                items: vec![],
                looted_by_player: false,
            },
        );

        session.handle_loot_money(loot_money_packet()).await;

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::CoinRemoved as u16
        );
        assert_eq!(sent.read_packed_guid().unwrap(), loot_object_one);

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootMoneyNotify as u16
        );
        assert_eq!(sent.read_uint64().unwrap(), 3);

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::CoinRemoved as u16
        );
        assert_eq!(sent.read_packed_guid().unwrap(), loot_object_two);

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootMoneyNotify as u16
        );
        assert_eq!(sent.read_uint64().unwrap(), 7);
        assert_eq!(session.player_gold_like_cpp(), 10);
        assert_eq!(session.loot_table.get(&owner_one).unwrap().coins, 0);
        assert_eq!(session.loot_table.get(&owner_two).unwrap().coins, 0);
        assert!(session.active_loot_view_owners.contains(&owner_one));
        assert!(session.active_loot_view_owners.contains(&owner_two));
    }

    #[tokio::test]
    async fn loot_money_gain_completes_money_tracking_event_objective_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(4);
        let player_guid = ObjectGuid::create_player(1, 42);
        let loot_guid = test_creature_guid(19_029);
        let quest_id = 12_530;
        let mut quest = test_quest_template(quest_id);
        quest.flags |= 0x0000_0400; // C++ QUEST_FLAGS_TRACKING_EVENT.
        quest.objectives.push(wow_data::quest::QuestObjective {
            id: quest_id * 10,
            quest_id,
            obj_type: 8, // C++ QUEST_OBJECTIVE_MONEY.
            order: 0,
            storage_index: -1,
            object_id: 0,
            amount: 7,
            flags: 0,
            flags2: 0,
            progress_bar_weight: 0.0,
            description: String::new(),
        });
        session.set_player_guid(Some(player_guid));
        session.set_active_loot_guid(loot_guid);
        session.set_quest_store(Arc::new(wow_data::quest::QuestStore::from_quests_like_cpp(
            [quest],
        )));
        session.player_quests.insert(
            quest_id,
            crate::handlers::quest::PlayerQuestStatus {
                quest_id,
                status: crate::conditions::QUEST_STATUS_INCOMPLETE_LIKE_CPP,
                explored: false,
                accept_time_secs: 0,
                end_time_secs: 0,
                objective_counts: Vec::new(),
                slot: 0,
            },
        );
        insert_allowed_coin_loot_like_cpp(&mut session, loot_guid, player_guid, 7);

        session.handle_loot_money(loot_money_packet()).await;

        assert_eq!(session.player_gold_like_cpp(), 7);
        assert!(!session.player_quests.contains_key(&quest_id));
        assert!(session.rewarded_quests.contains(&quest_id));

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::CoinRemoved as u16
        );
        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootMoneyNotify as u16
        );
        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::QuestGiverQuestComplete as u16
        );
        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::QuestUpdateComplete as u16
        );
    }

    #[tokio::test]
    async fn loot_money_splits_corpse_gold_to_near_group_members_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(2);
        let player_guid = ObjectGuid::create_player(1, 42);
        let other_guid = ObjectGuid::create_player(1, 43);
        let loot_guid = test_creature_guid(19_027);
        let (other_tx, other_rx) = flume::bounded::<Vec<u8>>(2);
        let player_registry = Arc::new(PlayerRegistry::default());
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(player_guid);
        group.add_member(other_guid);
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);
        player_registry.insert(other_guid, broadcast_info(other_guid, other_tx));

        session.set_player_guid(Some(player_guid));
        session.set_player_position_like_cpp(Position::ZERO);
        session.group_guid = Some(group_guid);
        session.set_player_registry(player_registry);
        session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
        session.set_active_loot_guid(loot_guid);
        session.loot_table.insert(
            loot_guid,
            CreatureLoot {
                loot_guid,
                coins: 9,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: vec![player_guid, other_guid],
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags::default(),
                    allowed_looters: vec![player_guid, other_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        session.handle_loot_money(loot_money_packet()).await;

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::CoinRemoved as u16
        );
        assert_eq!(sent.read_packed_guid().unwrap(), loot_guid);

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootMoneyNotify as u16
        );
        assert_eq!(sent.read_uint64().unwrap(), 4);
        assert_eq!(sent.read_uint64().unwrap(), 0);
        assert!(!sent.read_bit().unwrap());

        let sent = other_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootMoneyNotify as u16
        );
        assert_eq!(sent.read_uint64().unwrap(), 4);
        assert_eq!(sent.read_uint64().unwrap(), 0);
        assert!(!sent.read_bit().unwrap());
        assert_eq!(session.player_gold_like_cpp(), 4);
        assert_eq!(session.loot_table.get(&loot_guid).unwrap().coins, 0);
    }

    #[tokio::test]
    async fn loot_item_releases_blocked_item_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let loot_guid = test_creature_guid(19_003);
        session.set_player_guid(Some(player_guid));
        session.set_active_loot_guid(loot_guid);
        session.set_player_position_like_cpp(Position::ZERO);
        register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
        session.loot_table.insert(
            loot_guid,
            CreatureLoot {
                loot_guid,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: Vec::new(),
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags {
                        blocked: true,
                        ..Default::default()
                    },
                    allowed_looters: vec![player_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        session
            .handle_loot_item(loot_item_packet(loot_guid, 0))
            .await;

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootReleaseAll as u16
        );
        assert_eq!(sent.remaining(), 0);
        assert!(session.is_active_loot_guid(loot_guid));
        assert!(!session.loot_table.get(&loot_guid).unwrap().items[0].taken);
    }

    #[tokio::test]
    async fn loot_item_releases_when_player_is_not_allowed_looter_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let other_guid = ObjectGuid::create_player(1, 43);
        let loot_guid = test_creature_guid(19_004);
        session.set_player_guid(Some(player_guid));
        session.set_active_loot_guid(loot_guid);
        session.set_player_position_like_cpp(Position::ZERO);
        register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
        session.loot_table.insert(
            loot_guid,
            CreatureLoot {
                loot_guid,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: Vec::new(),
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags::default(),
                    allowed_looters: vec![other_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        session
            .handle_loot_item(loot_item_packet(loot_guid, 0))
            .await;

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootReleaseAll as u16
        );
        assert_eq!(sent.remaining(), 0);
        assert!(session.is_active_loot_guid(loot_guid));
        assert!(!session.loot_table.get(&loot_guid).unwrap().items[0].taken);
    }

    #[tokio::test]
    async fn loot_item_releases_when_roll_winner_is_different_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let winner_guid = ObjectGuid::create_player(1, 43);
        let loot_guid = test_creature_guid(19_005);
        session.set_player_guid(Some(player_guid));
        session.set_active_loot_guid(loot_guid);
        session.set_player_position_like_cpp(Position::ZERO);
        register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
        session.loot_table.insert(
            loot_guid,
            CreatureLoot {
                loot_guid,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: Vec::new(),
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags::default(),
                    allowed_looters: vec![player_guid],
                    roll_winner: winner_guid,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        session
            .handle_loot_item(loot_item_packet(loot_guid, 0))
            .await;

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootReleaseAll as u16
        );
        assert_eq!(sent.remaining(), 0);
        assert!(session.is_active_loot_guid(loot_guid));
        assert!(!session.loot_table.get(&loot_guid).unwrap().items[0].taken);
    }

    #[tokio::test]
    async fn loot_roll_without_canonical_roll_state_returns_silently_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));

        session
            .handle_loot_roll(LootRoll {
                loot_obj: test_creature_guid(19_006),
                loot_list_id: 0,
                roll_type: 1,
            })
            .await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn set_loot_specialization_matches_cpp_class_validation() {
        let (mut session, send_rx) = make_session_with_send();
        session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));
        session.set_player_class_like_cpp(2);
        session.set_chr_specialization_store(Arc::new(ChrSpecializationStore::from_entries([
            ChrSpecializationEntry {
                id: 65,
                class_id: 2,
                order_index: 0,
                role: 0,
            },
            ChrSpecializationEntry {
                id: 71,
                class_id: 1,
                order_index: 0,
                role: 0,
            },
        ])));

        session
            .handle_set_loot_specialization(SetLootSpecialization { spec_id: 65 })
            .await;
        assert_eq!(session.loot_specialization_id_like_cpp(), 65);

        session
            .handle_set_loot_specialization(SetLootSpecialization { spec_id: 71 })
            .await;
        assert_eq!(session.loot_specialization_id_like_cpp(), 65);

        session
            .handle_set_loot_specialization(SetLootSpecialization { spec_id: 999 })
            .await;
        assert_eq!(session.loot_specialization_id_like_cpp(), 65);

        session
            .handle_set_loot_specialization(SetLootSpecialization { spec_id: 0 })
            .await;
        assert_eq!(session.loot_specialization_id_like_cpp(), 0);
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn set_loot_specialization_without_loaded_player_is_ignored_like_cpp_status_guard() {
        let (mut session, _send_rx) = make_session_with_send();
        session.set_player_class_like_cpp(2);
        session.set_chr_specialization_store(Arc::new(ChrSpecializationStore::from_entries([
            ChrSpecializationEntry {
                id: 65,
                class_id: 2,
                order_index: 0,
                role: 0,
            },
        ])));

        session
            .handle_set_loot_specialization(SetLootSpecialization { spec_id: 65 })
            .await;

        assert_eq!(session.loot_specialization_id_like_cpp(), 0);
    }

    #[tokio::test]
    async fn master_loot_item_without_group_sends_didnt_kill_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        session.set_player_guid(Some(ObjectGuid::create_player(1, 42)));

        session
            .handle_master_loot_item(MasterLootItem {
                target: ObjectGuid::create_player(1, 77),
                loot: Vec::new(),
            })
            .await;

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootResponse as u16
        );
        assert_eq!(sent.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
        assert_eq!(sent.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
        assert_eq!(
            sent.read_uint8().unwrap(),
            wow_packet::packets::loot::LOOT_ERROR_DIDNT_KILL_LIKE_CPP
        );
    }

    #[tokio::test]
    async fn master_loot_item_uses_group_master_looter_guid_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let leader_guid = ObjectGuid::create_player(1, 42);
        let master_guid = ObjectGuid::create_player(1, 43);
        let (leader_tx, _leader_rx) = flume::bounded::<Vec<u8>>(2);
        let player_registry = Arc::new(PlayerRegistry::default());
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(leader_guid);
        group.add_member(master_guid);
        group.loot_method = LOOT_METHOD_MASTER_LIKE_CPP;
        group.master_looter_guid = master_guid;
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);
        player_registry.insert(leader_guid, broadcast_info(leader_guid, leader_tx));
        session.group_guid = Some(group_guid);
        session.set_player_registry(player_registry);
        session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
        session.set_player_guid(Some(leader_guid));

        session
            .handle_master_loot_item(MasterLootItem {
                target: master_guid,
                loot: Vec::new(),
            })
            .await;

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootResponse as u16
        );
        assert_eq!(sent.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
        assert_eq!(sent.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
        assert_eq!(
            sent.read_uint8().unwrap(),
            wow_packet::packets::loot::LOOT_ERROR_DIDNT_KILL_LIKE_CPP
        );

        session.set_player_guid(Some(master_guid));
        session
            .handle_master_loot_item(MasterLootItem {
                target: leader_guid,
                loot: Vec::new(),
            })
            .await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn master_loot_item_missing_target_sends_player_not_found_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let master_guid = ObjectGuid::create_player(1, 42);
        let missing_target = ObjectGuid::create_player(1, 77);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(master_guid);
        group.loot_method = LOOT_METHOD_MASTER_LIKE_CPP;
        group.master_looter_guid = master_guid;
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);
        session.group_guid = Some(group_guid);
        session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
        session.set_player_guid(Some(master_guid));

        session
            .handle_master_loot_item(MasterLootItem {
                target: missing_target,
                loot: Vec::new(),
            })
            .await;

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootResponse as u16
        );
        assert_eq!(sent.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
        assert_eq!(sent.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
        assert_eq!(
            sent.read_uint8().unwrap(),
            LOOT_ERROR_PLAYER_NOT_FOUND_LIKE_CPP
        );
    }

    #[tokio::test]
    async fn master_loot_item_non_master_loot_view_returns_silently_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let master_guid = ObjectGuid::create_player(1, 42);
        let loot_owner = test_creature_guid(19_082);
        let loot_object = represented_loot_object_guid_like_cpp(loot_owner);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(master_guid);
        group.loot_method = LOOT_METHOD_MASTER_LIKE_CPP;
        group.master_looter_guid = master_guid;
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        session.group_guid = Some(group_guid);
        session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
        session.set_player_guid(Some(master_guid));
        session.set_active_loot_guid(loot_owner);
        session.loot_table.insert(
            loot_owner,
            CreatureLoot {
                loot_guid: loot_object,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: Vec::new(),
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags::default(),
                    allowed_looters: vec![master_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        session
            .handle_master_loot_item(MasterLootItem {
                target: master_guid,
                loot: vec![wow_packet::packets::loot::LootItemRequest {
                    object: loot_object,
                    loot_list_id: 0,
                }],
            })
            .await;

        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn master_loot_item_ineligible_target_sends_master_other_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let master_guid = ObjectGuid::create_player(1, 42);
        let target_guid = ObjectGuid::create_player(1, 77);
        let loot_owner = test_creature_guid(19_080);
        let loot_object = represented_loot_object_guid_like_cpp(loot_owner);
        let (target_tx, _target_rx) = flume::bounded::<Vec<u8>>(2);
        let player_registry = Arc::new(PlayerRegistry::default());
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(master_guid);
        group.loot_method = LOOT_METHOD_MASTER_LIKE_CPP;
        group.master_looter_guid = master_guid;
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);
        player_registry.insert(target_guid, broadcast_info(target_guid, target_tx));
        session.group_guid = Some(group_guid);
        session.set_player_registry(player_registry);
        session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
        session.set_player_guid(Some(master_guid));
        session.set_active_loot_guid(loot_owner);
        session.loot_table.insert(
            loot_owner,
            CreatureLoot {
                loot_guid: loot_object,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: LOOT_METHOD_MASTER_LIKE_CPP,
                loot_master: master_guid,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: Vec::new(),
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags::default(),
                    allowed_looters: vec![master_guid, target_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        session
            .handle_master_loot_item(MasterLootItem {
                target: target_guid,
                loot: vec![wow_packet::packets::loot::LootItemRequest {
                    object: loot_object,
                    loot_list_id: 0,
                }],
            })
            .await;

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootResponse as u16
        );
        assert_eq!(sent.read_packed_guid().unwrap(), loot_owner);
        assert_eq!(sent.read_packed_guid().unwrap(), loot_object);
        assert_eq!(sent.read_uint8().unwrap(), LOOT_ERROR_MASTER_OTHER_LIKE_CPP);
    }

    #[tokio::test]
    async fn master_loot_item_target_not_allowed_for_loot_sends_master_other_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let master_guid = ObjectGuid::create_player(1, 42);
        let loot_owner = test_creature_guid(19_083);
        let loot_object = represented_loot_object_guid_like_cpp(loot_owner);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(master_guid);
        group.loot_method = LOOT_METHOD_MASTER_LIKE_CPP;
        group.master_looter_guid = master_guid;
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        session.group_guid = Some(group_guid);
        session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
        session.set_player_guid(Some(master_guid));
        session.set_active_loot_guid(loot_owner);
        session.loot_table.insert(
            loot_owner,
            CreatureLoot {
                loot_guid: loot_object,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: LOOT_METHOD_MASTER_LIKE_CPP,
                loot_master: master_guid,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: Vec::new(),
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags::default(),
                    allowed_looters: vec![master_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        session
            .handle_master_loot_item(MasterLootItem {
                target: master_guid,
                loot: vec![wow_packet::packets::loot::LootItemRequest {
                    object: loot_object,
                    loot_list_id: 0,
                }],
            })
            .await;

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootResponse as u16
        );
        assert_eq!(sent.read_packed_guid().unwrap(), loot_owner);
        assert_eq!(sent.read_packed_guid().unwrap(), loot_object);
        assert_eq!(sent.read_uint8().unwrap(), LOOT_ERROR_MASTER_OTHER_LIKE_CPP);
    }

    #[test]
    fn master_loot_inventory_result_mapping_matches_cpp_errors() {
        assert_eq!(
            super::master_loot_error_for_inventory_result_like_cpp(InventoryResult::Ok),
            None
        );
        assert_eq!(
            super::master_loot_error_for_inventory_result_like_cpp(InventoryResult::ItemMaxCount),
            Some(LOOT_ERROR_MASTER_UNIQUE_ITEM_LIKE_CPP)
        );
        assert_eq!(
            super::master_loot_error_for_inventory_result_like_cpp(InventoryResult::InvFull),
            Some(wow_packet::packets::loot::LOOT_ERROR_MASTER_INV_FULL_LIKE_CPP)
        );
        assert_eq!(
            super::master_loot_error_for_inventory_result_like_cpp(InventoryResult::CantEquipEver),
            Some(LOOT_ERROR_MASTER_OTHER_LIKE_CPP)
        );
    }

    #[tokio::test]
    async fn master_loot_item_self_target_can_store_maps_unique_error_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let master_guid = ObjectGuid::create_player(1, 42);
        let item_guid = ObjectGuid::create_item(1, 700);
        let loot_owner = test_creature_guid(19_081);
        let loot_object = represented_loot_object_guid_like_cpp(loot_owner);
        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(master_guid);
        group.loot_method = LOOT_METHOD_MASTER_LIKE_CPP;
        group.master_looter_guid = master_guid;
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        session.group_guid = Some(group_guid);
        session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
        session.set_player_guid(Some(master_guid));
        session.set_active_loot_guid(loot_owner);
        install_limited_test_item_template(&mut session, 700, 1);
        session.insert_inventory_item_like_cpp(
            35,
            InventoryItem {
                guid: item_guid,
                entry_id: 700,
                db_guid: 700,
                inventory_type: None,
            },
        );
        let item = session.make_inventory_item_object(
            item_guid,
            700,
            master_guid,
            1,
            0,
            ItemContext::None,
            35,
        );
        session.insert_inventory_item_object(item);
        session.loot_table.insert(
            loot_owner,
            CreatureLoot {
                loot_guid: loot_object,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: LOOT_METHOD_MASTER_LIKE_CPP,
                loot_master: master_guid,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: vec![master_guid],
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 700,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags::default(),
                    allowed_looters: vec![master_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        session
            .handle_master_loot_item(MasterLootItem {
                target: master_guid,
                loot: vec![wow_packet::packets::loot::LootItemRequest {
                    object: loot_object,
                    loot_list_id: 0,
                }],
            })
            .await;

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootResponse as u16
        );
        assert_eq!(sent.read_packed_guid().unwrap(), loot_owner);
        assert_eq!(sent.read_packed_guid().unwrap(), loot_object);
        assert_eq!(
            sent.read_uint8().unwrap(),
            LOOT_ERROR_MASTER_UNIQUE_ITEM_LIKE_CPP
        );
    }

    #[tokio::test]
    async fn master_loot_item_self_target_success_marks_removed_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let master_guid = ObjectGuid::create_player(1, 42);
        let loot_owner = test_creature_guid(19_082);
        let loot_object = represented_loot_object_guid_like_cpp(loot_owner);

        session.loot_table.insert(
            loot_owner,
            CreatureLoot {
                loot_guid: loot_object,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: LOOT_METHOD_MASTER_LIKE_CPP,
                loot_master: master_guid,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: vec![master_guid],
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 701,
                    quantity: 3,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags::default(),
                    allowed_looters: vec![master_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        session.mark_represented_master_loot_item_removed_like_cpp(
            loot_owner,
            loot_object,
            0,
            master_guid,
        );

        let loot = session.loot_table.get(&loot_owner).unwrap();
        assert_eq!(loot.items[0].quantity, 0);
        assert!(loot.items[0].is_looted_for_player_like_cpp(master_guid));
        assert_eq!(loot.unlooted_count, 0);

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootRemoved as u16
        );
        assert_eq!(sent.read_packed_guid().unwrap(), loot_owner);
        assert_eq!(sent.read_packed_guid().unwrap(), loot_object);
        assert_eq!(sent.read_uint8().unwrap(), 0);
    }

    #[tokio::test]
    async fn master_loot_item_remote_target_can_store_error_is_reported_by_target_session_like_cpp()
    {
        let (mut master_session, master_rx) = make_session_with_send();
        let (mut target_session, _target_rx) = make_session_with_send();
        let master_guid = ObjectGuid::create_player(1, 42);
        let target_guid = ObjectGuid::create_player(1, 77);
        let existing_item_guid = ObjectGuid::create_item(1, 701);
        let loot_owner = test_creature_guid(19_083);
        let loot_object = represented_loot_object_guid_like_cpp(loot_owner);

        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(master_guid);
        group.loot_method = LOOT_METHOD_MASTER_LIKE_CPP;
        group.master_looter_guid = master_guid;
        group.members.push(target_guid);
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        let player_registry = Arc::new(PlayerRegistry::default());
        let (target_send_tx, _target_send_rx) = flume::bounded::<Vec<u8>>(2);
        let mut target_info = broadcast_info(target_guid, target_send_tx);
        target_info.command_tx = target_session.session_command_tx();
        player_registry.insert(target_guid, target_info);

        master_session.group_guid = Some(group_guid);
        master_session.set_group_registry(
            Arc::clone(&group_registry),
            Arc::new(PendingInvites::default()),
        );
        master_session.set_player_registry(Arc::clone(&player_registry));
        master_session.set_player_guid(Some(master_guid));
        master_session.set_active_loot_guid(loot_owner);
        master_session.loot_table.insert(
            loot_owner,
            CreatureLoot {
                loot_guid: loot_object,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: LOOT_METHOD_MASTER_LIKE_CPP,
                loot_master: master_guid,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: vec![target_guid],
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 701,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags::default(),
                    allowed_looters: vec![target_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        target_session.set_player_guid(Some(target_guid));
        install_limited_test_item_template(&mut target_session, 701, 1);
        target_session.insert_inventory_item_like_cpp(
            35,
            InventoryItem {
                guid: existing_item_guid,
                entry_id: 701,
                db_guid: 701,
                inventory_type: None,
            },
        );
        let item = target_session.make_inventory_item_object(
            existing_item_guid,
            701,
            target_guid,
            1,
            0,
            ItemContext::None,
            35,
        );
        target_session.insert_inventory_item_object(item);

        let master_future = master_session.handle_master_loot_item(MasterLootItem {
            target: target_guid,
            loot: vec![wow_packet::packets::loot::LootItemRequest {
                object: loot_object,
                loot_list_id: 0,
            }],
        });
        let target_future = async {
            for _ in 0..8 {
                target_session.process_pending().await;
                tokio::task::yield_now().await;
            }
        };
        tokio::join!(master_future, target_future);

        let sent = master_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootResponse as u16
        );
        assert_eq!(sent.read_packed_guid().unwrap(), loot_owner);
        assert_eq!(sent.read_packed_guid().unwrap(), loot_object);
        assert_eq!(
            sent.read_uint8().unwrap(),
            LOOT_ERROR_MASTER_UNIQUE_ITEM_LIKE_CPP
        );
        assert!(!master_session.loot_table.get(&loot_owner).unwrap().items[0].taken);
    }

    #[tokio::test]
    async fn master_loot_item_remote_target_unavailable_command_reports_player_not_found_like_cpp()
    {
        let (mut master_session, master_rx) = make_session_with_send();
        let master_guid = ObjectGuid::create_player(1, 42);
        let target_guid = ObjectGuid::create_player(1, 77);
        let loot_owner = test_creature_guid(19_084);
        let loot_object = represented_loot_object_guid_like_cpp(loot_owner);

        let group_registry = Arc::new(GroupRegistry::default());
        let mut group = GroupInfo::new(master_guid);
        group.loot_method = LOOT_METHOD_MASTER_LIKE_CPP;
        group.master_looter_guid = master_guid;
        group.members.push(target_guid);
        let group_guid = group.group_guid;
        group_registry.insert(group_guid, group);

        let player_registry = Arc::new(PlayerRegistry::default());
        let (target_send_tx, _target_send_rx) = flume::bounded::<Vec<u8>>(2);
        let (command_tx, _command_rx) = flume::bounded(0);
        let mut target_info = broadcast_info(target_guid, target_send_tx);
        target_info.command_tx = command_tx;
        player_registry.insert(target_guid, target_info);

        master_session.group_guid = Some(group_guid);
        master_session.set_group_registry(group_registry, Arc::new(PendingInvites::default()));
        master_session.set_player_registry(player_registry);
        master_session.set_player_guid(Some(master_guid));
        master_session.set_active_loot_guid(loot_owner);
        master_session.loot_table.insert(
            loot_owner,
            CreatureLoot {
                loot_guid: loot_object,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: LOOT_METHOD_MASTER_LIKE_CPP,
                loot_master: master_guid,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: vec![target_guid],
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 702,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags::default(),
                    allowed_looters: vec![target_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        master_session
            .handle_master_loot_item(MasterLootItem {
                target: target_guid,
                loot: vec![wow_packet::packets::loot::LootItemRequest {
                    object: loot_object,
                    loot_list_id: 0,
                }],
            })
            .await;

        let sent = master_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootResponse as u16
        );
        assert_eq!(sent.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
        assert_eq!(sent.read_packed_guid().unwrap(), ObjectGuid::EMPTY);
        assert_eq!(
            sent.read_uint8().unwrap(),
            LOOT_ERROR_PLAYER_NOT_FOUND_LIKE_CPP
        );
    }

    #[tokio::test]
    async fn loot_item_creature_too_far_uses_cpp_error() {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let loot_guid = test_creature_guid(19_008);
        session.set_player_guid(Some(player_guid));
        session.set_player_position_like_cpp(Position::ZERO);
        session.set_active_loot_guid(loot_guid);

        let mut creature = test_creature(loot_guid, false);
        creature.current_pos = Position::new(31.0, 0.0, 0.0, 0.0);
        register_test_creature_like_cpp(&mut session, creature);
        session.loot_table.insert(
            loot_guid,
            CreatureLoot {
                loot_guid,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: Vec::new(),
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags::default(),
                    allowed_looters: vec![player_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        session
            .handle_loot_item(loot_item_packet(loot_guid, 0))
            .await;

        let sent = send_rx.try_recv().unwrap();
        assert_eq!(
            loot_response_failure_reason(&sent),
            LOOT_ERROR_TOO_FAR_LIKE_CPP
        );
        assert!(!session.loot_table.get(&loot_guid).unwrap().items[0].taken);
        assert!(session.is_active_loot_guid(loot_guid));
    }

    #[tokio::test]
    async fn loot_item_creature_distance_can_use_canonical_map_object_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let loot_guid = test_creature_guid(19_018);
        session.set_player_guid(Some(player_guid));
        session.set_player_position_like_cpp(Position::ZERO);
        session.set_active_loot_guid(loot_guid);
        attach_canonical_map_object(
            &mut session,
            AccessorObjectKind::Creature,
            canonical_world_object(loot_guid, 0, Position::new(31.0, 0.0, 0.0, 0.0)),
        );
        session.loot_table.insert(
            loot_guid,
            CreatureLoot {
                loot_guid,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: Vec::new(),
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags::default(),
                    allowed_looters: vec![player_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        session
            .handle_loot_item(loot_item_packet(loot_guid, 0))
            .await;

        let sent = send_rx.try_recv().unwrap();
        assert_eq!(
            loot_response_failure_reason(&sent),
            LOOT_ERROR_TOO_FAR_LIKE_CPP
        );
        assert!(!session.loot_table.get(&loot_guid).unwrap().items[0].taken);
        assert!(session.is_active_loot_guid(loot_guid));
    }

    #[tokio::test]
    async fn loot_item_missing_creature_uses_cpp_no_loot_error() {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let loot_guid = test_creature_guid(19_009);
        session.set_player_guid(Some(player_guid));
        session.set_active_loot_guid(loot_guid);
        session.loot_table.insert(
            loot_guid,
            CreatureLoot {
                loot_guid,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: Vec::new(),
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags::default(),
                    allowed_looters: vec![player_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        session
            .handle_loot_item(loot_item_packet(loot_guid, 0))
            .await;

        let sent = send_rx.try_recv().unwrap();
        assert_eq!(
            loot_response_failure_reason(&sent),
            LOOT_ERROR_NO_LOOT_LIKE_CPP
        );
        assert!(!session.loot_table.get(&loot_guid).unwrap().items[0].taken);
        assert!(session.is_active_loot_guid(loot_guid));
    }

    #[tokio::test]
    async fn loot_item_request_uses_loot_object_to_find_active_owner_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let owner_guid = test_creature_guid(19_023);
        let loot_object_guid = represented_loot_object_guid_like_cpp(owner_guid);
        session.set_player_guid(Some(player_guid));
        session.set_active_loot_guid(owner_guid);
        session.loot_table.insert(
            owner_guid,
            CreatureLoot {
                loot_guid: loot_object_guid,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: Vec::new(),
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags::default(),
                    allowed_looters: vec![player_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        session
            .handle_loot_item(loot_item_packet(loot_object_guid, 0))
            .await;

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootResponse as u16
        );
        assert_eq!(sent.read_packed_guid().unwrap(), owner_guid);
        assert_eq!(sent.read_packed_guid().unwrap(), loot_object_guid);
        assert_eq!(sent.read_uint8().unwrap(), LOOT_ERROR_NO_LOOT_LIKE_CPP);
        assert!(!session.loot_table.get(&owner_guid).unwrap().items[0].taken);
        assert!(session.is_active_loot_guid(owner_guid));
    }

    #[tokio::test]
    async fn loot_item_request_can_use_secondary_active_loot_object_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let primary_owner = test_creature_guid(19_027);
        let secondary_owner = test_creature_guid(19_028);
        let secondary_loot_object = represented_loot_object_guid_like_cpp(secondary_owner);
        session.set_player_guid(Some(player_guid));
        session.set_active_loot_guid(primary_owner);
        session.add_active_loot_view_owner_like_cpp(secondary_owner);
        session.loot_table.insert(
            secondary_owner,
            CreatureLoot {
                loot_guid: secondary_loot_object,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: Vec::new(),
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags::default(),
                    allowed_looters: vec![player_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        session
            .handle_loot_item(loot_item_packet(secondary_loot_object, 0))
            .await;

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootResponse as u16
        );
        assert_eq!(sent.read_packed_guid().unwrap(), secondary_owner);
        assert_eq!(sent.read_packed_guid().unwrap(), secondary_loot_object);
        assert_eq!(sent.read_uint8().unwrap(), LOOT_ERROR_NO_LOOT_LIKE_CPP);
        assert!(!session.loot_table.get(&secondary_owner).unwrap().items[0].taken);
        assert!(session.active_loot_view_owners.contains(&primary_owner));
        assert!(session.active_loot_view_owners.contains(&secondary_owner));
    }

    #[tokio::test]
    async fn loot_item_missing_gameobject_uses_cpp_release() {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let loot_guid = test_gameobject_guid(19_010);
        session.set_player_guid(Some(player_guid));
        session.set_active_loot_guid(loot_guid);
        session.loot_table.insert(
            loot_guid,
            CreatureLoot {
                loot_guid,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: Vec::new(),
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags::default(),
                    allowed_looters: vec![player_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        session
            .handle_loot_item(loot_item_packet(loot_guid, 0))
            .await;

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootRelease as u16
        );
        assert_eq!(sent.read_packed_guid().unwrap(), loot_guid);
        assert_eq!(sent.read_packed_guid().unwrap(), player_guid);
        assert!(!session.loot_table.get(&loot_guid).unwrap().items[0].taken);
        assert!(session.is_active_loot_guid(loot_guid));
    }

    #[tokio::test]
    async fn loot_item_gameobject_too_far_uses_cpp_release() {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let loot_guid = test_gameobject_guid(19_029);
        let go_position = Position::new(6.0, 0.0, 0.0, 0.0);
        session.set_player_guid(Some(player_guid));
        session.set_player_position_like_cpp(Position::ZERO);
        session.set_active_loot_guid(loot_guid);
        attach_canonical_map_object(
            &mut session,
            AccessorObjectKind::GameObject,
            canonical_world_object(loot_guid, 0, go_position),
        );
        session.record_represented_gameobject_runtime_state_like_cpp(
            0,
            loot_guid,
            loot_guid.entry(),
            go_position,
            GAMEOBJECT_TYPE_CHEST as u8,
        );
        session.loot_table.insert(
            loot_guid,
            CreatureLoot {
                loot_guid,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: Vec::new(),
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags::default(),
                    allowed_looters: vec![player_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        session
            .handle_loot_item(loot_item_packet(loot_guid, 0))
            .await;

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootRelease as u16
        );
        assert_eq!(sent.read_packed_guid().unwrap(), loot_guid);
        assert_eq!(sent.read_packed_guid().unwrap(), player_guid);
        assert!(!session.loot_table.get(&loot_guid).unwrap().items[0].taken);
        assert!(session.is_active_loot_guid(loot_guid));
    }

    #[tokio::test]
    async fn loot_item_fishing_hole_skips_gameobject_distance_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let loot_guid = test_gameobject_guid(19_030);
        let go_position = Position::new(100.0, 0.0, 0.0, 0.0);
        session.set_player_guid(Some(player_guid));
        session.set_player_position_like_cpp(Position::ZERO);
        session.set_active_loot_guid(loot_guid);
        attach_canonical_map_object(
            &mut session,
            AccessorObjectKind::GameObject,
            canonical_world_object(loot_guid, 0, go_position),
        );
        session.record_represented_gameobject_runtime_state_like_cpp(
            0,
            loot_guid,
            loot_guid.entry(),
            go_position,
            GAMEOBJECT_TYPE_FISHING_HOLE as u8,
        );
        session.loot_table.insert(
            loot_guid,
            CreatureLoot {
                loot_guid,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_FISHINGHOLE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: Vec::new(),
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags {
                        blocked: true,
                        ..Default::default()
                    },
                    allowed_looters: vec![player_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        session
            .handle_loot_item(loot_item_packet(loot_guid, 0))
            .await;

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootReleaseAll as u16
        );
        assert_eq!(sent.remaining(), 0);
        assert!(!session.loot_table.get(&loot_guid).unwrap().items[0].taken);
        assert!(session.is_active_loot_guid(loot_guid));
    }

    #[tokio::test]
    async fn loot_item_owned_gameobject_skips_distance_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let loot_guid = test_gameobject_guid(19_035);
        let go_position = Position::new(100.0, 0.0, 0.0, 0.0);
        session.set_player_guid(Some(player_guid));
        session.set_player_position_like_cpp(Position::ZERO);
        session.set_active_loot_guid(loot_guid);
        session.record_represented_gameobject_runtime_state_like_cpp(
            0,
            loot_guid,
            loot_guid.entry(),
            go_position,
            GAMEOBJECT_TYPE_CHEST as u8,
        );
        session.record_represented_gameobject_owner_guid_like_cpp(loot_guid, player_guid);
        session.loot_table.insert(
            loot_guid,
            CreatureLoot {
                loot_guid,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: Vec::new(),
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags {
                        blocked: true,
                        ..Default::default()
                    },
                    allowed_looters: vec![player_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        session
            .handle_loot_item(loot_item_packet(loot_guid, 0))
            .await;

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootReleaseAll as u16
        );
        assert_eq!(sent.remaining(), 0);
        assert!(!session.loot_table.get(&loot_guid).unwrap().items[0].taken);
        assert!(session.is_active_loot_guid(loot_guid));
    }

    #[tokio::test]
    async fn loot_item_owned_gameobject_skips_distance_from_canonical_created_by_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let loot_guid = test_gameobject_guid(19_036);
        let mut game_object = GameObject::new();
        game_object.world_mut().object_mut().create(loot_guid);
        game_object
            .world_mut()
            .set_map(u32::from(session.player_map_id_like_cpp()), 0)
            .unwrap();
        game_object
            .world_mut()
            .relocate(Position::new(100.0, 0.0, 0.0, 0.0));
        game_object.world_mut().object_mut().add_to_world();
        game_object.set_created_by(player_guid);

        session.set_player_guid(Some(player_guid));
        session.set_player_position_like_cpp(Position::ZERO);
        session.set_active_loot_guid(loot_guid);
        attach_canonical_gameobject(&mut session, game_object);
        session.loot_table.insert(
            loot_guid,
            CreatureLoot {
                loot_guid,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: Vec::new(),
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags {
                        blocked: true,
                        ..Default::default()
                    },
                    allowed_looters: vec![player_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        session
            .handle_loot_item(loot_item_packet(loot_guid, 0))
            .await;

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootReleaseAll as u16
        );
        assert_eq!(sent.remaining(), 0);
        assert!(!session.loot_table.get(&loot_guid).unwrap().items[0].taken);
        assert!(session.is_active_loot_guid(loot_guid));
    }

    #[tokio::test]
    async fn loot_release_ignores_guid_outside_active_view_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let active_guid = test_creature_guid(19_011);
        let spoofed_guid = test_creature_guid(19_012);
        session.set_player_guid(Some(player_guid));
        session.set_active_loot_guid(active_guid);
        session.loot_table.insert(
            spoofed_guid,
            CreatureLoot {
                loot_guid: spoofed_guid,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: Vec::new(),
                items: Vec::new(),
                looted_by_player: false,
            },
        );

        session
            .handle_loot_release(loot_release_packet(spoofed_guid))
            .await;

        assert!(session.is_active_loot_guid(active_guid));
        assert!(session.loot_table.contains_key(&spoofed_guid));
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn loot_release_ignores_active_guid_without_represented_loot_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let active_guid = test_creature_guid(19_015);
        session.set_player_guid(Some(player_guid));
        session.set_active_loot_guid(active_guid);

        session
            .handle_loot_release(loot_release_packet(active_guid))
            .await;

        assert!(session.is_active_loot_guid(active_guid));
        assert!(send_rx.try_recv().is_err());
    }

    #[tokio::test]
    async fn loot_release_accepts_secondary_active_owner_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let primary_guid = test_creature_guid(19_029);
        let secondary_guid = test_creature_guid(19_030);
        session.set_player_guid(Some(player_guid));
        session.set_active_loot_guid(primary_guid);
        session.add_active_loot_view_owner_like_cpp(secondary_guid);
        session.loot_table.insert(
            secondary_guid,
            CreatureLoot {
                loot_guid: represented_loot_object_guid_like_cpp(secondary_guid),
                coins: 5,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: Vec::new(),
                items: Vec::new(),
                looted_by_player: false,
            },
        );

        session
            .handle_loot_release(loot_release_packet(secondary_guid))
            .await;

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootRelease as u16
        );
        assert_eq!(sent.read_packed_guid().unwrap(), secondary_guid);
        assert_eq!(sent.read_packed_guid().unwrap(), player_guid);
        assert!(session.is_active_loot_guid(primary_guid));
        assert!(session.active_loot_view_owners.contains(&primary_guid));
        assert!(!session.active_loot_view_owners.contains(&secondary_guid));
        assert!(session.loot_table.contains_key(&secondary_guid));
    }

    #[tokio::test]
    async fn loot_release_keeps_unlooted_creature_loot_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let other_guid = ObjectGuid::create_player(1, 77);
        let loot_guid = test_creature_guid(19_013);
        session.set_player_guid(Some(player_guid));
        session.set_active_loot_guid(loot_guid);
        register_test_creature_like_cpp(&mut session, test_creature(loot_guid, false));
        session.loot_table.insert(
            loot_guid,
            CreatureLoot {
                loot_guid,
                coins: 7,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: vec![player_guid, other_guid],
                allowed_looters: Vec::new(),
                items: Vec::new(),
                looted_by_player: false,
            },
        );

        session
            .handle_loot_release(loot_release_packet(loot_guid))
            .await;

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootRelease as u16
        );
        assert_eq!(sent.read_packed_guid().unwrap(), loot_guid);
        assert_eq!(sent.read_packed_guid().unwrap(), player_guid);
        assert!(!session.is_active_loot_guid(loot_guid));
        assert!(session.loot_table.contains_key(&loot_guid));
        assert_eq!(
            session.loot_table.get(&loot_guid).unwrap().players_looting,
            vec![other_guid]
        );
        assert!(
            session
                .mutate_world_creature(loot_guid, |creature| creature.corpse_despawn_at())
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn loot_release_keeps_unlooted_gameobject_loot_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let loot_guid = test_gameobject_guid(19_014);
        session.set_player_guid(Some(player_guid));
        session.set_active_loot_guid(loot_guid);
        session.client_visible_guids_like_cpp.insert(loot_guid);
        session.record_represented_gameobject_runtime_state_like_cpp(
            0,
            loot_guid,
            loot_guid.entry(),
            Position::ZERO,
            GAMEOBJECT_TYPE_CHEST as u8,
        );
        session.loot_table.insert(
            loot_guid,
            CreatureLoot {
                loot_guid,
                coins: 0,
                unlooted_count: 1,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: Vec::new(),
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags::default(),
                    allowed_looters: vec![player_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        session
            .handle_loot_release(loot_release_packet(loot_guid))
            .await;

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootRelease as u16
        );
        assert_eq!(sent.read_packed_guid().unwrap(), loot_guid);
        assert_eq!(sent.read_packed_guid().unwrap(), player_guid);
        assert!(!session.is_active_loot_guid(loot_guid));
        assert!(!session.loot_table.get(&loot_guid).unwrap().items[0].taken);
        let state = session
            .represented_gameobject_use_states
            .get(&loot_guid)
            .unwrap();
        assert_eq!(state.loot_state, Some(LootState::Activated));
        assert_eq!(state.loot_state_unit_guid, player_guid);
    }

    #[tokio::test]
    async fn loot_release_gameobject_too_far_keeps_state_and_loot_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let loot_guid = test_gameobject_guid(19_031);
        let go_position = Position::new(6.0, 0.0, 0.0, 0.0);
        session.set_player_guid(Some(player_guid));
        session.set_player_position_like_cpp(Position::ZERO);
        session.set_active_loot_guid(loot_guid);
        session.record_represented_gameobject_runtime_state_like_cpp(
            0,
            loot_guid,
            loot_guid.entry(),
            go_position,
            GAMEOBJECT_TYPE_CHEST as u8,
        );
        session.loot_table.insert(
            loot_guid,
            CreatureLoot {
                loot_guid,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: Vec::new(),
                items: Vec::new(),
                looted_by_player: false,
            },
        );

        session
            .handle_loot_release(loot_release_packet(loot_guid))
            .await;

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootRelease as u16
        );
        assert_eq!(sent.read_packed_guid().unwrap(), loot_guid);
        assert_eq!(sent.read_packed_guid().unwrap(), player_guid);
        assert!(!session.is_active_loot_guid(loot_guid));
        assert!(session.loot_table.contains_key(&loot_guid));
        assert_eq!(
            session
                .represented_gameobject_use_states
                .get(&loot_guid)
                .unwrap()
                .loot_state,
            None
        );
    }

    #[tokio::test]
    async fn loot_release_owned_gameobject_skips_distance_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let loot_guid = test_gameobject_guid(19_036);
        let go_position = Position::new(100.0, 0.0, 0.0, 0.0);
        session.set_player_guid(Some(player_guid));
        session.set_player_position_like_cpp(Position::ZERO);
        session.set_active_loot_guid(loot_guid);
        session.record_represented_gameobject_runtime_state_like_cpp(
            0,
            loot_guid,
            loot_guid.entry(),
            go_position,
            GAMEOBJECT_TYPE_CHEST as u8,
        );
        session.record_represented_gameobject_owner_guid_like_cpp(loot_guid, player_guid);
        session.loot_table.insert(
            loot_guid,
            CreatureLoot {
                loot_guid,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: Vec::new(),
                items: Vec::new(),
                looted_by_player: false,
            },
        );

        session
            .handle_loot_release(loot_release_packet(loot_guid))
            .await;

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootRelease as u16
        );
        assert!(!session.loot_table.contains_key(&loot_guid));
        assert_eq!(
            session
                .represented_gameobject_use_states
                .get(&loot_guid)
                .unwrap()
                .loot_state,
            Some(LootState::JustDeactivated)
        );
    }

    #[tokio::test]
    async fn loot_release_fully_looted_gameobject_just_deactivates_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let loot_guid = test_gameobject_guid(19_032);
        session.set_player_guid(Some(player_guid));
        session.set_player_position_like_cpp(Position::ZERO);
        session.set_active_loot_guid(loot_guid);
        session.record_represented_gameobject_runtime_state_like_cpp(
            0,
            loot_guid,
            loot_guid.entry(),
            Position::ZERO,
            GAMEOBJECT_TYPE_CHEST as u8,
        );
        session.loot_table.insert(
            loot_guid,
            CreatureLoot {
                loot_guid,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: Vec::new(),
                items: Vec::new(),
                looted_by_player: false,
            },
        );

        session
            .handle_loot_release(loot_release_packet(loot_guid))
            .await;

        let sent = send_rx.try_recv().unwrap();
        let mut sent = WorldPacket::from_bytes(&sent);
        assert_eq!(
            sent.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootRelease as u16
        );
        assert!(!session.loot_table.contains_key(&loot_guid));
        assert_eq!(
            session
                .represented_gameobject_use_states
                .get(&loot_guid)
                .unwrap()
                .loot_state,
            Some(LootState::JustDeactivated)
        );
    }

    #[tokio::test]
    async fn gameobject_owned_loot_release_partial_chest_uses_canonical_is_fully_looted_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let loot_guid = test_gameobject_guid(19_132);
        let game_object =
            make_canonical_gameobject_for_session(&session, loot_guid, GAMEOBJECT_TYPE_CHEST as u8);
        attach_canonical_gameobject(&mut session, game_object);
        session.set_player_guid(Some(player_guid));
        session.set_player_position_like_cpp(Position::ZERO);
        session.set_active_loot_guid(loot_guid);
        session.record_represented_gameobject_runtime_state_like_cpp(
            0,
            loot_guid,
            loot_guid.entry(),
            Position::ZERO,
            GAMEOBJECT_TYPE_CHEST as u8,
        );
        session.record_represented_gameobject_chest_release_metadata_like_cpp(
            loot_guid,
            GameObjectLootSource {
                chest_consumable: false,
                chest_restock_time_secs: 7,
                ..Default::default()
            },
        );
        session.loot_table.insert(
            loot_guid,
            CreatureLoot {
                loot_guid: represented_loot_object_guid_like_cpp(loot_guid),
                coins: 0,
                unlooted_count: 1,
                loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: vec![player_guid],
                allowed_looters: vec![player_guid],
                items: vec![represented_loot_entry(0, 25, player_guid)],
                looted_by_player: false,
            },
        );

        session
            .handle_loot_release(loot_release_packet(loot_guid))
            .await;

        assert!(send_rx.try_recv().is_ok());
        let canonical = canonical_gameobject_snapshot(&session, loot_guid).unwrap();
        assert_eq!(
            canonical.shared_loot_like_cpp(),
            Some(&GameObjectOwnedLoot::new(0, 1))
        );
        assert!(!canonical.is_fully_looted_like_cpp());
        assert_eq!(canonical.loot_state(), LootState::Activated);
        assert_eq!(canonical.loot_state_unit_guid(), player_guid);
        assert!(canonical.restock_time() > 0);
        assert!(session.loot_table.contains_key(&loot_guid));
        assert_eq!(
            session
                .represented_gameobject_use_states
                .get(&loot_guid)
                .unwrap()
                .loot_state,
            Some(LootState::Activated)
        );
    }

    #[tokio::test]
    async fn loot_release_partial_chest_syncs_state_to_same_map_viewers_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let same_map_guid = ObjectGuid::create_player(1, 77);
        let loot_guid = test_gameobject_guid(19_138);
        let (same_command_tx, same_command_rx) = flume::bounded(2);
        let (same_send_tx, _same_send_rx) = flume::bounded::<Vec<u8>>(1);
        let player_registry = Arc::new(PlayerRegistry::default());
        let mut same_info = broadcast_info(same_map_guid, same_send_tx);
        same_info.map_id = 571;
        same_info.command_tx = same_command_tx;
        player_registry.insert(same_map_guid, same_info);

        session.set_player_registry(player_registry);
        session.set_player_guid(Some(player_guid));
        session.set_player_position_like_cpp(Position::ZERO);
        session.set_player_map_position_like_cpp(571, Position::ZERO);
        session.set_active_loot_guid(loot_guid);
        session.record_represented_gameobject_runtime_state_like_cpp(
            0,
            loot_guid,
            loot_guid.entry(),
            Position::ZERO,
            GAMEOBJECT_TYPE_CHEST as u8,
        );
        session.record_represented_gameobject_chest_release_metadata_like_cpp(
            loot_guid,
            GameObjectLootSource {
                loot_id: 7_001,
                chest_restock_time_secs: 45,
                chest_consumable: false,
                ..Default::default()
            },
        );
        session.loot_table.insert(
            loot_guid,
            CreatureLoot {
                loot_guid: represented_loot_object_guid_like_cpp(loot_guid),
                coins: 0,
                unlooted_count: 1,
                loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: vec![player_guid],
                allowed_looters: Vec::new(),
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags::default(),
                    allowed_looters: Vec::new(),
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        session
            .handle_loot_release(loot_release_packet(loot_guid))
            .await;

        let release_bytes = send_rx.try_recv().unwrap();
        let mut release = WorldPacket::from_bytes(&release_bytes);
        assert_eq!(
            release.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootRelease as u16
        );
        let command = match same_command_rx.try_recv() {
            Ok(SessionCommand::SyncChestGameobjectStateAndRefreshLikeCpp(command)) => command,
            other => panic!("expected chest release sync command, got {other:?}"),
        };
        assert_eq!(command.gameobject_guid, loot_guid);
        assert_eq!(command.map_id, 571);
        assert_eq!(
            command.loot_state,
            Some(wow_entities::LootState::Activated as u8)
        );
        assert_eq!(command.loot_state_unit_guid, player_guid);
        assert_eq!(command.chest_loot_id, 7_001);
        assert_eq!(command.chest_restock_time_secs, 45);
    }

    #[tokio::test]
    async fn gameobject_owned_loot_release_fully_consumed_chest_uses_canonical_is_fully_looted_like_cpp()
     {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let loot_guid = test_gameobject_guid(19_133);
        let mut game_object =
            make_canonical_gameobject_for_session(&session, loot_guid, GAMEOBJECT_TYPE_CHEST as u8);
        game_object.set_shared_loot_like_cpp(GameObjectOwnedLoot::new(0, 1));
        attach_canonical_gameobject(&mut session, game_object);
        session.set_player_guid(Some(player_guid));
        session.set_player_position_like_cpp(Position::ZERO);
        session.set_active_loot_guid(loot_guid);
        session.record_represented_gameobject_runtime_state_like_cpp(
            0,
            loot_guid,
            loot_guid.entry(),
            Position::ZERO,
            GAMEOBJECT_TYPE_CHEST as u8,
        );
        session.record_represented_gameobject_chest_release_metadata_like_cpp(
            loot_guid,
            GameObjectLootSource {
                chest_consumable: false,
                chest_restock_time_secs: 7,
                ..Default::default()
            },
        );
        session.loot_table.insert(
            loot_guid,
            CreatureLoot {
                loot_guid: represented_loot_object_guid_like_cpp(loot_guid),
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: vec![player_guid],
                allowed_looters: vec![player_guid],
                items: Vec::new(),
                looted_by_player: false,
            },
        );

        session
            .handle_loot_release(loot_release_packet(loot_guid))
            .await;

        assert!(send_rx.try_recv().is_ok());
        let canonical = canonical_gameobject_snapshot(&session, loot_guid).unwrap();
        assert_eq!(
            canonical.shared_loot_like_cpp(),
            Some(&GameObjectOwnedLoot::default())
        );
        assert!(canonical.is_fully_looted_like_cpp());
        assert_eq!(canonical.loot_state(), LootState::JustDeactivated);
        assert_eq!(canonical.loot_state_unit_guid(), ObjectGuid::EMPTY);
        assert_eq!(canonical.restock_time(), 0);
        assert!(!session.loot_table.contains_key(&loot_guid));
        assert_eq!(
            session
                .represented_gameobject_use_states
                .get(&loot_guid)
                .unwrap()
                .loot_state,
            Some(LootState::JustDeactivated)
        );
    }

    #[test]
    fn gameobject_loot_release_without_canonical_manager_keeps_represented_restock_fallback_like_cpp()
     {
        let mut session = make_session();
        let player_guid = ObjectGuid::create_player(1, 42);
        let loot_guid = test_gameobject_guid(19_135);
        session.record_represented_gameobject_runtime_state_like_cpp(
            0,
            loot_guid,
            loot_guid.entry(),
            Position::ZERO,
            GAMEOBJECT_TYPE_CHEST as u8,
        );
        session.record_represented_gameobject_chest_release_metadata_like_cpp(
            loot_guid,
            GameObjectLootSource {
                chest_consumable: false,
                chest_restock_time_secs: 7,
                ..Default::default()
            },
        );

        session.apply_represented_gameobject_loot_release_like_cpp(loot_guid, player_guid, true);

        let state = session
            .represented_gameobject_use_states
            .get(&loot_guid)
            .unwrap();
        assert_eq!(state.loot_state, Some(LootState::NotReady));
        assert_eq!(state.loot_state_unit_guid, ObjectGuid::EMPTY);
        assert!(state.chest_restock_until.is_some());
    }

    #[tokio::test]
    async fn gameobject_owned_loot_release_personal_chest_syncs_current_player_and_despawns_like_cpp()
     {
        let (mut session, send_rx) = make_session_with_send_capacity(2);
        let player_guid = ObjectGuid::create_player(1, 42);
        let loot_guid = test_gameobject_guid(19_134);
        let game_object =
            make_canonical_gameobject_for_session(&session, loot_guid, GAMEOBJECT_TYPE_CHEST as u8);
        attach_canonical_gameobject(&mut session, game_object);
        session.set_player_guid(Some(player_guid));
        session.set_player_position_like_cpp(Position::ZERO);
        session.set_active_loot_guid(loot_guid);
        session.record_represented_gameobject_runtime_state_like_cpp(
            0,
            loot_guid,
            loot_guid.entry(),
            Position::ZERO,
            GAMEOBJECT_TYPE_CHEST as u8,
        );
        session.record_represented_gameobject_chest_release_metadata_like_cpp(
            loot_guid,
            GameObjectLootSource {
                personal_loot_id: 55,
                chest_consumable: false,
                chest_restock_time_secs: 7,
                ..Default::default()
            },
        );
        session.represented_personal_loot_owners.insert(loot_guid);
        session
            .represented_personal_loot_money
            .insert((loot_guid, player_guid), 0);
        session.loot_table.insert(
            loot_guid,
            CreatureLoot {
                loot_guid: represented_loot_object_guid_like_cpp(loot_guid),
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
                dungeon_encounter_id: 1,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: vec![player_guid],
                allowed_looters: vec![player_guid],
                items: vec![LootEntry {
                    taken: true,
                    ..represented_loot_entry(0, 25, player_guid)
                }],
                looted_by_player: false,
            },
        );

        session
            .handle_loot_release(loot_release_packet(loot_guid))
            .await;

        assert!(send_rx.try_recv().is_ok());
        let canonical = canonical_gameobject_snapshot(&session, loot_guid).unwrap();
        assert_eq!(canonical.shared_loot_like_cpp(), None);
        assert_eq!(
            canonical.personal_loot_like_cpp(player_guid),
            Some(&GameObjectOwnedLoot::default())
        );
        let state = session
            .represented_gameobject_use_states
            .get(&loot_guid)
            .unwrap();
        assert_eq!(state.per_player_state_player_guid, Some(player_guid));
        assert_eq!(state.per_player_despawn_secs, Some(7));
        assert!(state.per_player_despawn_until.is_some());
    }

    #[tokio::test]
    async fn loot_release_fishing_gameobjects_follow_cpp_state_branches() {
        let (mut session, send_rx) = make_session_with_send_capacity(2);
        let player_guid = ObjectGuid::create_player(1, 42);
        let fishing_node = test_gameobject_guid(19_033);
        let fishing_hole = test_gameobject_guid(19_034);
        session.set_player_guid(Some(player_guid));
        session.set_player_position_like_cpp(Position::ZERO);
        session.set_active_loot_guid(fishing_node);
        session.add_active_loot_view_owner_like_cpp(fishing_hole);
        for (guid, go_type, loot_type) in [
            (
                fishing_node,
                GAMEOBJECT_TYPE_FISHING_NODE as u8,
                LOOT_TYPE_FISHING_LIKE_CPP,
            ),
            (
                fishing_hole,
                GAMEOBJECT_TYPE_FISHING_HOLE as u8,
                LOOT_TYPE_FISHINGHOLE_LIKE_CPP,
            ),
        ] {
            session.record_represented_gameobject_runtime_state_like_cpp(
                0,
                guid,
                guid.entry(),
                Position::ZERO,
                go_type,
            );
            session.loot_table.insert(
                guid,
                CreatureLoot {
                    loot_guid: guid,
                    coins: 0,
                    unlooted_count: 1,
                    loot_type,
                    dungeon_encounter_id: 0,
                    loot_method: 0,
                    loot_master: ObjectGuid::EMPTY,
                    round_robin_player: ObjectGuid::EMPTY,
                    player_ffa_items: Vec::new(),
                    players_looting: Vec::new(),
                    allowed_looters: Vec::new(),
                    items: vec![LootEntry {
                        loot_list_id: 0,
                        item_id: 25,
                        quantity: 1,
                        random_properties_id: 0,
                        random_properties_seed: 0,
                        item_context: 0,
                        flags: LootEntryFlags::default(),
                        allowed_looters: vec![player_guid],
                        roll_winner: ObjectGuid::EMPTY,
                        ffa_looted_by: Vec::new(),
                        taken: false,
                    }],
                    looted_by_player: false,
                },
            );
        }

        session
            .handle_loot_release(loot_release_packet(fishing_node))
            .await;
        session
            .handle_loot_release(loot_release_packet(fishing_hole))
            .await;

        assert!(send_rx.try_recv().is_ok());
        assert!(send_rx.try_recv().is_ok());
        assert_eq!(
            session
                .represented_gameobject_use_states
                .get(&fishing_node)
                .unwrap()
                .loot_state,
            Some(LootState::JustDeactivated)
        );
        let hole_state = session
            .represented_gameobject_use_states
            .get(&fishing_hole)
            .unwrap();
        assert_eq!(hole_state.loot_state, Some(LootState::Ready));
        assert_eq!(hole_state.personal_loot_uses, 1);
    }

    #[tokio::test]
    async fn loot_release_fishing_hole_just_deactivates_at_max_opens_like_cpp() {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let fishing_hole = test_gameobject_guid(19_037);
        session.set_player_guid(Some(player_guid));
        session.set_player_position_like_cpp(Position::ZERO);
        session.set_active_loot_guid(fishing_hole);
        session.record_represented_gameobject_runtime_state_like_cpp(
            0,
            fishing_hole,
            fishing_hole.entry(),
            Position::ZERO,
            GAMEOBJECT_TYPE_FISHING_HOLE as u8,
        );
        session.record_represented_fishing_hole_max_opens_like_cpp(fishing_hole, 1);
        session.loot_table.insert(
            fishing_hole,
            CreatureLoot {
                loot_guid: fishing_hole,
                coins: 0,
                unlooted_count: 1,
                loot_type: LOOT_TYPE_FISHINGHOLE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: Vec::new(),
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags::default(),
                    allowed_looters: vec![player_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        session
            .handle_loot_release(loot_release_packet(fishing_hole))
            .await;

        assert!(send_rx.try_recv().is_ok());
        let hole_state = session
            .represented_gameobject_use_states
            .get(&fishing_hole)
            .unwrap();
        assert_eq!(hole_state.personal_loot_uses, 1);
        assert_eq!(hole_state.loot_state, Some(LootState::JustDeactivated));
    }

    #[tokio::test]
    async fn gameobject_loot_release_fishing_hole_uses_canonical_use_count_when_represented_stale_like_cpp()
     {
        let (mut session, send_rx) = make_session_with_send();
        let player_guid = ObjectGuid::create_player(1, 42);
        let fishing_hole = test_gameobject_guid(19_136);
        let mut game_object = make_canonical_gameobject_for_session(
            &session,
            fishing_hole,
            GAMEOBJECT_TYPE_FISHING_HOLE as u8,
        );
        game_object.add_use_like_cpp();
        attach_canonical_gameobject(&mut session, game_object);
        session.set_player_guid(Some(player_guid));
        session.set_player_position_like_cpp(Position::ZERO);
        session.set_active_loot_guid(fishing_hole);
        session.record_represented_gameobject_runtime_state_like_cpp(
            0,
            fishing_hole,
            fishing_hole.entry(),
            Position::ZERO,
            GAMEOBJECT_TYPE_FISHING_HOLE as u8,
        );
        session.record_represented_fishing_hole_max_opens_like_cpp(fishing_hole, 2);
        session.loot_table.insert(
            fishing_hole,
            CreatureLoot {
                loot_guid: fishing_hole,
                coins: 0,
                unlooted_count: 1,
                loot_type: LOOT_TYPE_FISHINGHOLE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: Vec::new(),
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags::default(),
                    allowed_looters: vec![player_guid],
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );

        session
            .handle_loot_release(loot_release_packet(fishing_hole))
            .await;

        assert!(send_rx.try_recv().is_ok());
        let canonical = canonical_gameobject_snapshot(&session, fishing_hole).unwrap();
        assert_eq!(canonical.use_times(), 2);
        assert_eq!(canonical.loot_state(), LootState::JustDeactivated);
        let hole_state = session
            .represented_gameobject_use_states
            .get(&fishing_hole)
            .unwrap();
        assert_eq!(hole_state.personal_loot_uses, 2);
        assert_eq!(hole_state.loot_state, Some(LootState::JustDeactivated));
    }

    #[tokio::test]
    async fn loot_release_gathering_node_sets_local_active_state_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(2);
        let player_guid = ObjectGuid::create_player(1, 42);
        let gathering_node = test_gameobject_guid(19_038);
        session.set_player_guid(Some(player_guid));
        session.set_player_position_like_cpp(Position::ZERO);
        session.set_player_map_position_like_cpp(571, Position::ZERO);
        let game_object = make_canonical_gameobject_for_session(
            &session,
            gathering_node,
            GAMEOBJECT_TYPE_GATHERING_NODE as u8,
        );
        attach_canonical_gameobject(&mut session, game_object);
        session.set_active_loot_guid(gathering_node);
        session.client_visible_guids_like_cpp.insert(gathering_node);
        session.record_represented_gameobject_runtime_state_like_cpp(
            0,
            gathering_node,
            gathering_node.entry(),
            Position::ZERO,
            GAMEOBJECT_TYPE_GATHERING_NODE as u8,
        );
        session.loot_table.insert(
            gathering_node,
            CreatureLoot {
                loot_guid: gathering_node,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_GATHERING_NODE_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: Vec::new(),
                items: Vec::new(),
                looted_by_player: false,
            },
        );

        session
            .handle_loot_release(loot_release_packet(gathering_node))
            .await;

        let release_bytes = send_rx.try_recv().unwrap();
        let mut release = WorldPacket::from_bytes(&release_bytes);
        assert_eq!(
            release.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootRelease as u16
        );
        let state = session
            .represented_gameobject_use_states
            .get(&gathering_node)
            .unwrap();
        assert_eq!(state.go_state, Some(GoState::Active));
        assert_eq!(state.loot_state, None);
        let expected = wow_packet::packets::update::UpdateObject::game_object_values_update(
            gathering_node,
            571,
            wow_packet::packets::update::GameObjectDataValuesUpdate {
                changed_object_type_mask: 1 << wow_entities::TYPEID_OBJECT,
                object_data: Some(wow_packet::packets::update::ObjectDataValuesUpdate {
                    changed_object_type_mask: 1 << wow_entities::TYPEID_OBJECT,
                    object_data_mask: 0x05,
                    entry_id: 0,
                    dynamic_flags: wow_entities::GO_DYNFLAG_LO_DEPLETED,
                    scale: 0.0,
                }),
                game_object_data_mask: 0,
                state_world_effect_ids: Vec::new(),
                enable_doodad_sets: Vec::new(),
                enable_doodad_sets_update_mask: None,
                world_effects: Vec::new(),
                world_effects_update_mask: None,
                display_id: 0,
                spell_visual_id: 0,
                state_spell_visual_id: 0,
                spawn_tracking_state_anim_id: 0,
                spawn_tracking_state_anim_kit_id: 0,
                created_by: ObjectGuid::EMPTY,
                guild_guid: ObjectGuid::EMPTY,
                flags: 0,
                parent_rotation: [0.0; 4],
                faction_template: 0,
                level: 0,
                state: 0,
                type_id: 0,
                percent_health: 0,
                art_kit: 0,
                custom_param: 0,
            },
        )
        .to_bytes();
        assert_eq!(send_rx.try_recv().unwrap(), expected);
    }

    #[tokio::test]
    async fn loot_release_personal_chest_records_per_player_despawn_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(4);
        let player_guid = ObjectGuid::create_player(1, 42);
        let restocked_chest = test_gameobject_guid(19_039);
        let fallback_chest = test_gameobject_guid(19_040);
        session.set_player_guid(Some(player_guid));
        session.set_player_position_like_cpp(Position::ZERO);
        session.set_active_loot_guid(restocked_chest);
        session.add_active_loot_view_owner_like_cpp(fallback_chest);
        session
            .client_visible_guids_like_cpp
            .insert(restocked_chest);
        session.client_visible_guids_like_cpp.insert(fallback_chest);

        for (guid, restock_time) in [(restocked_chest, 45), (fallback_chest, 0)] {
            session.record_represented_gameobject_runtime_state_like_cpp(
                0,
                guid,
                guid.entry(),
                Position::ZERO,
                GAMEOBJECT_TYPE_CHEST as u8,
            );
            session.record_represented_gameobject_chest_release_metadata_like_cpp(
                guid,
                GameObjectLootSource {
                    personal_loot_id: 7_001,
                    chest_restock_time_secs: restock_time,
                    chest_consumable: false,
                    ..Default::default()
                },
            );
            session.loot_table.insert(
                guid,
                CreatureLoot {
                    loot_guid: guid,
                    coins: 0,
                    unlooted_count: 0,
                    loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
                    dungeon_encounter_id: 0,
                    loot_method: 0,
                    loot_master: ObjectGuid::EMPTY,
                    round_robin_player: ObjectGuid::EMPTY,
                    player_ffa_items: Vec::new(),
                    players_looting: Vec::new(),
                    allowed_looters: Vec::new(),
                    items: Vec::new(),
                    looted_by_player: false,
                },
            );
        }

        session
            .handle_loot_release(loot_release_packet(restocked_chest))
            .await;
        session
            .handle_loot_release(loot_release_packet(fallback_chest))
            .await;

        assert!(send_rx.try_recv().is_ok());
        assert!(send_rx.try_recv().is_ok());
        assert!(send_rx.try_recv().is_ok());
        assert!(send_rx.try_recv().is_ok());
        assert!(
            !session
                .client_visible_guids_like_cpp
                .contains(&restocked_chest)
        );
        assert!(
            !session
                .client_visible_guids_like_cpp
                .contains(&fallback_chest)
        );
        assert_eq!(
            session
                .represented_gameobject_use_states
                .get(&restocked_chest)
                .unwrap()
                .per_player_despawn_secs,
            Some(45)
        );
        assert_eq!(
            session
                .represented_gameobject_use_states
                .get(&fallback_chest)
                .unwrap()
                .per_player_despawn_secs,
            Some(wow_entities::DEFAULT_GAMEOBJECT_RESPAWN_DELAY_SECS)
        );
        assert!(
            session
                .represented_gameobject_use_states
                .get(&restocked_chest)
                .unwrap()
                .per_player_despawn_until
                .is_some()
        );
        assert!(session.represented_gameobject_is_per_player_despawned_like_cpp(restocked_chest));
    }

    #[tokio::test]
    async fn loot_release_personal_chest_without_have_at_client_sends_no_out_of_range_like_cpp() {
        let (mut session, send_rx) = make_session_with_send_capacity(2);
        let player_guid = ObjectGuid::create_player(1, 42);
        let chest_guid = test_gameobject_guid(19_137);
        session.set_player_guid(Some(player_guid));
        session.set_player_position_like_cpp(Position::ZERO);
        session.set_active_loot_guid(chest_guid);
        session.record_represented_gameobject_runtime_state_like_cpp(
            0,
            chest_guid,
            chest_guid.entry(),
            Position::ZERO,
            GAMEOBJECT_TYPE_CHEST as u8,
        );
        session.record_represented_gameobject_chest_release_metadata_like_cpp(
            chest_guid,
            GameObjectLootSource {
                personal_loot_id: 7_001,
                chest_restock_time_secs: 45,
                chest_consumable: false,
                ..Default::default()
            },
        );
        session.loot_table.insert(
            chest_guid,
            CreatureLoot {
                loot_guid: chest_guid,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: Vec::new(),
                items: Vec::new(),
                looted_by_player: false,
            },
        );

        session
            .handle_loot_release(loot_release_packet(chest_guid))
            .await;

        let release_bytes = send_rx.try_recv().unwrap();
        let mut release = WorldPacket::from_bytes(&release_bytes);
        assert_eq!(
            release.read_uint16().unwrap(),
            wow_constants::ServerOpcodes::LootRelease as u16
        );
        assert!(send_rx.try_recv().is_err());
        let state = session
            .represented_gameobject_use_states
            .get(&chest_guid)
            .unwrap();
        assert_eq!(state.per_player_despawn_secs, Some(45));
        assert!(state.per_player_despawn_until.is_some());
        assert_eq!(state.per_player_state_player_guid, Some(player_guid));
    }

    #[tokio::test]
    async fn loot_release_shared_chest_restock_starts_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send_capacity(4);
        let player_guid = ObjectGuid::create_player(1, 42);
        let partial_chest = test_gameobject_guid(19_041);
        let full_chest = test_gameobject_guid(19_042);
        session.set_player_guid(Some(player_guid));
        session.set_active_loot_guid(partial_chest);
        session.add_active_loot_view_owner_like_cpp(full_chest);

        for guid in [partial_chest, full_chest] {
            session.record_represented_gameobject_runtime_state_like_cpp(
                0,
                guid,
                guid.entry(),
                Position::ZERO,
                GAMEOBJECT_TYPE_CHEST as u8,
            );
            session.record_represented_gameobject_chest_release_metadata_like_cpp(
                guid,
                GameObjectLootSource {
                    loot_id: 7_001,
                    chest_restock_time_secs: 45,
                    chest_consumable: false,
                    ..Default::default()
                },
            );
        }
        session.loot_table.insert(
            partial_chest,
            CreatureLoot {
                loot_guid: partial_chest,
                coins: 0,
                unlooted_count: 1,
                loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: vec![player_guid],
                allowed_looters: Vec::new(),
                items: vec![LootEntry {
                    loot_list_id: 0,
                    item_id: 25,
                    quantity: 1,
                    random_properties_id: 0,
                    random_properties_seed: 0,
                    item_context: 0,
                    flags: LootEntryFlags::default(),
                    allowed_looters: Vec::new(),
                    roll_winner: ObjectGuid::EMPTY,
                    ffa_looted_by: Vec::new(),
                    taken: false,
                }],
                looted_by_player: false,
            },
        );
        session.loot_table.insert(
            full_chest,
            CreatureLoot {
                loot_guid: full_chest,
                coins: 0,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: vec![player_guid],
                allowed_looters: Vec::new(),
                items: Vec::new(),
                looted_by_player: false,
            },
        );

        session
            .handle_loot_release(loot_release_packet(partial_chest))
            .await;
        session
            .handle_loot_release(loot_release_packet(full_chest))
            .await;

        let partial_state = session
            .represented_gameobject_use_states
            .get(&partial_chest)
            .unwrap();
        assert_eq!(partial_state.loot_state, Some(LootState::Activated));
        assert!(partial_state.chest_restock_until.is_some());
        assert!(session.loot_table.contains_key(&partial_chest));

        let full_state = session
            .represented_gameobject_use_states
            .get(&full_chest)
            .unwrap();
        assert_eq!(full_state.loot_state, Some(LootState::NotReady));
        assert!(full_state.chest_restock_until.is_some());
        assert!(!session.loot_table.contains_key(&full_chest));
    }

    #[tokio::test]
    async fn process_pending_shared_chest_restock_clears_loot_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send_capacity(4);
        let chest_guid = test_gameobject_guid(19_043);
        session.set_state(SessionState::LoggedIn);
        session.record_represented_gameobject_runtime_state_like_cpp(
            0,
            chest_guid,
            chest_guid.entry(),
            Position::ZERO,
            GAMEOBJECT_TYPE_CHEST as u8,
        );
        {
            let state = session
                .represented_gameobject_use_states
                .get_mut(&chest_guid)
                .unwrap();
            state.loot_state = Some(LootState::Activated);
            state.chest_restock_until = Some(Instant::now() - Duration::from_secs(1));
        }
        session.loot_table.insert(
            chest_guid,
            CreatureLoot {
                loot_guid: chest_guid,
                coins: 7,
                unlooted_count: 1,
                loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: Vec::new(),
                items: Vec::new(),
                looted_by_player: false,
            },
        );

        session.process_pending().await;

        let state = session
            .represented_gameobject_use_states
            .get(&chest_guid)
            .unwrap();
        assert_eq!(state.loot_state, Some(LootState::Ready));
        assert!(state.chest_restock_until.is_none());
        assert!(!session.loot_table.contains_key(&chest_guid));
    }

    #[tokio::test]
    async fn process_pending_shared_chest_restock_syncs_state_to_same_map_viewers_like_cpp() {
        let (mut session, _send_rx) = make_session_with_send_capacity(4);
        let player_guid = ObjectGuid::create_player(1, 42);
        let same_map_guid = ObjectGuid::create_player(1, 77);
        let chest_guid = test_gameobject_guid(19_044);
        let (same_command_tx, same_command_rx) = flume::bounded(2);
        let (same_send_tx, _same_send_rx) = flume::bounded::<Vec<u8>>(1);
        let player_registry = Arc::new(PlayerRegistry::default());
        let mut same_info = broadcast_info(same_map_guid, same_send_tx);
        same_info.map_id = 571;
        same_info.command_tx = same_command_tx;
        player_registry.insert(same_map_guid, same_info);

        session.set_state(SessionState::LoggedIn);
        session.set_player_guid(Some(player_guid));
        session.set_player_map_position_like_cpp(571, Position::ZERO);
        session.set_player_registry(player_registry);
        session.record_represented_gameobject_runtime_state_like_cpp(
            0,
            chest_guid,
            chest_guid.entry(),
            Position::ZERO,
            GAMEOBJECT_TYPE_CHEST as u8,
        );
        session.record_represented_gameobject_chest_release_metadata_like_cpp(
            chest_guid,
            GameObjectLootSource {
                loot_id: 7_002,
                chest_restock_time_secs: 45,
                chest_consumable: false,
                ..Default::default()
            },
        );
        {
            let state = session
                .represented_gameobject_use_states
                .get_mut(&chest_guid)
                .unwrap();
            state.loot_state = Some(LootState::NotReady);
            state.chest_restock_until = Some(Instant::now() - Duration::from_secs(1));
        }
        session.loot_table.insert(
            chest_guid,
            CreatureLoot {
                loot_guid: chest_guid,
                coins: 7,
                unlooted_count: 1,
                loot_type: LOOT_TYPE_CHEST_LIKE_CPP,
                dungeon_encounter_id: 0,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: Vec::new(),
                items: Vec::new(),
                looted_by_player: false,
            },
        );

        session.process_pending().await;

        let command = match same_command_rx.try_recv() {
            Ok(SessionCommand::SyncChestGameobjectStateAndRefreshLikeCpp(command)) => command,
            other => panic!("expected restock chest sync command, got {other:?}"),
        };
        assert_eq!(command.gameobject_guid, chest_guid);
        assert_eq!(command.map_id, 571);
        assert_eq!(command.loot_state, Some(LootState::Ready as u8));
        assert_eq!(command.loot_state_unit_guid, ObjectGuid::EMPTY);
        assert_eq!(command.chest_loot_id, 7_002);
        assert_eq!(command.chest_restock_time_secs, 45);
    }
}
