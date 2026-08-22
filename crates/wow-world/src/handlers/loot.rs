// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Loot packet handlers — CMSG_LOOT_UNIT, CMSG_LOOT_ITEM, CMSG_LOOT_RELEASE.
//!
//! References: C++ `WorldSession::HandleLoot*`/`DoLootRelease` in
//! `src/server/game/Handlers/LootHandler.cpp` and `Loot` in
//! `src/server/game/Loot/Loot.cpp`.

use std::collections::{HashMap, HashSet};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
};
use std::time::{Duration, Instant};

use rand::{
    Rng,
    distributions::{Distribution, WeightedIndex},
};
use tokio::time::timeout;
use tracing::{debug, info, warn};

use crate::session::directory::{PlayerRegistry, PrepareLootMoneyApplicationLikeCpp};
use crate::session::mailbox::{
    ApplyCreatureMeleeDamageLikeCppCommand, ApplyGroupJoinLikeCppCommand,
    ApplyGroupRemovalLikeCppCommand, ApplyLootMoneyLikeCppCommand, ApplyLootMoneyResultLikeCpp,
    CancelRepresentedTradeLikeCppCommand, CreatureAttackStartLikeCppCommand,
    CreatureAttackStopLikeCppCommand, KickLikeCppCommand, LootRollCommandIdentityLikeCpp,
    LootRollStoreWinnerCommand, LootRollVoteCommand, MasterLootGiveCommand, MasterLootGiveResult,
    NotifyLootMoneyRemovedLikeCppCommand, ReconcilePvpCombatExpiryLikeCppCommand,
    RefreshVisibleWorldCreaturesLikeCppCommand, SendAddonIfRegisteredLikeCppCommand,
    SendCreatureLootReleaseValuesUpdateLikeCppCommand,
    SendCreatureSpellCastIfVisibleLikeCppCommand, SendIfVisibleLikeCppCommand,
    SendPartyUpdateLikeCppCommand, SendRepeatableTurnInRequestItemsLikeCppCommand,
    SendRepresentedDuelCountdownLikeCppCommand, SendRepresentedDuelRequestedLikeCppCommand,
    SendRepresentedTradeStatusLikeCppCommand, SessionCommand,
    SetQuestSharingInfoAndSendDetailsCommand, SyncChestGameobjectStateAndRefreshLikeCppCommand,
    SyncGatheringNodeGameobjectStateAndRefreshLikeCppCommand,
    SyncGooberGameobjectStateAndRefreshLikeCppCommand, UnacceptRepresentedTradeLikeCppCommand,
    WorldSessionShutdownFlushResultLikeCpp,
};
use wow_constants::{
    ClientOpcodes, InventoryResult, InventoryType, ItemContext, ItemFieldFlags, ItemFlags,
    ItemFlags2, ItemQuality, UnitDynFlags,
};
use wow_core::{ObjectGuid, guid::HighGuid};
use wow_data::{ItemRandomEnchantmentTemplateEntry, ItemRandomPropertyTemplateEntry};
use wow_database::{
    CharStatements, CharacterDatabase, DatabaseError, SqlTransaction, SqlTransactionCommitError,
    StatementDef, WorldStatements, is_database_deadlock_like_cpp,
    retry_deadlocked_operation_like_cpp,
};
use wow_entities::{
    AccessorObjectKind, CORPSE_DYNFLAG_LOOTABLE, GAMEOBJECT_TYPE_AREADAMAGE,
    GAMEOBJECT_TYPE_BARBER_CHAIR, GAMEOBJECT_TYPE_BINDER, GAMEOBJECT_TYPE_CAMERA,
    GAMEOBJECT_TYPE_CHAIR, GAMEOBJECT_TYPE_CHEST, GAMEOBJECT_TYPE_DESTRUCTIBLE_BUILDING,
    GAMEOBJECT_TYPE_DOOR, GAMEOBJECT_TYPE_DUNGEON_DIFFICULTY, GAMEOBJECT_TYPE_FISHING_HOLE,
    GAMEOBJECT_TYPE_FISHING_NODE, GAMEOBJECT_TYPE_FLAGDROP, GAMEOBJECT_TYPE_FLAGSTAND,
    GAMEOBJECT_TYPE_GATHERING_NODE, GAMEOBJECT_TYPE_GOOBER, GAMEOBJECT_TYPE_GUILD_BANK,
    GAMEOBJECT_TYPE_MAILBOX, GAMEOBJECT_TYPE_MAP_OBJECT, GAMEOBJECT_TYPE_MINI_GAME,
    GAMEOBJECT_TYPE_QUESTGIVER, GAMEOBJECT_TYPE_TEXT, GO_DYNFLAG_LO_NO_INTERACT,
    GameObjectLootSource, GatheringNodeUseSource, GoState, INVENTORY_DEFAULT_SIZE,
    INVENTORY_SLOT_BAG_0, INVENTORY_SLOT_ITEM_END, INVENTORY_SLOT_ITEM_START, Item, ItemPosCount,
    LootState, MAX_MONEY_AMOUNT, is_bag_pos, make_item_pos,
};
use wow_handler::{PacketHandlerEntry, PacketProcessing, SessionStatus};
use wow_loot::{
    GeneratedLootItem, LootClaimCommitError, LootClaimError, LootClaimLease, LootClaimPayload,
    LootConditionId, LootConditionRowLikeCpp, LootFillError, LootFillOptions,
    LootItemRandomProperties, LootItemTemplateMetadata, LootStoreItem, LootStoreItemContext,
    LootStoreKind, LootTemplate, OwnedLootAuthority, OwnedLootAuthorityLifecycle, OwnedLootScope,
    OwnedLootSnapshot, condition_compare_values_like_cpp, loot_condition_reference_ids_like_cpp,
    loot_condition_reference_self_references_like_cpp,
    loot_condition_row_normalize_without_external_stores_like_cpp,
    loot_conditions_allow_player_with_references_like_cpp_representable,
    loot_item_ui_type_for_player_like_cpp,
};
use wow_packet::packets::item::{
    ItemExpirePurchaseRefund, ItemInstance, ItemModList, ItemPushResult, ItemPushResultDisplayType,
};
use wow_packet::packets::loot::{
    AELootTargets, AELootTargetsAck, CoinRemoved, CreatureLoot, LOOT_ERROR_DIDNT_KILL_LIKE_CPP,
    LOOT_ERROR_MASTER_INV_FULL_LIKE_CPP, LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
    LOOT_ERROR_MASTER_UNIQUE_ITEM_LIKE_CPP, LOOT_ERROR_NO_LOOT_LIKE_CPP,
    LOOT_ERROR_PLAYER_NOT_FOUND_LIKE_CPP, LOOT_ERROR_TOO_FAR_LIKE_CPP,
    LOOT_RESPONSE_DEFAULT_FAILURE_REASON_LIKE_CPP, LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP,
    LOOT_TYPE_CHEST_LIKE_CPP, LOOT_TYPE_CORPSE_LIKE_CPP, LOOT_TYPE_DISENCHANTING_LIKE_CPP,
    LOOT_TYPE_FISHING_JUNK_LIKE_CPP, LOOT_TYPE_FISHING_LIKE_CPP, LOOT_TYPE_FISHINGHOLE_LIKE_CPP,
    LOOT_TYPE_INSIGNIA_LIKE_CPP, LOOT_TYPE_MILLING_LIKE_CPP, LOOT_TYPE_PROSPECTING_LIKE_CPP,
    LOOT_TYPE_SKINNING_LIKE_CPP, LootAllPassed, LootEntry, LootEntryFlags, LootItemData,
    LootItemPkt, LootList, LootMoney, LootMoneyNotify, LootRelease, LootReleaseAll, LootRemoved,
    LootResponse, LootRoll, LootRollBroadcast, LootRollWon, LootUnit, MasterLootCandidateList,
    MasterLootItem, NotNormalLootItem, SLootRelease, SetLootSpecialization, StartLootRoll,
};
use wow_packet::packets::update::{ItemCreateData, ItemEnchantmentValuesUpdate, UpdateObject};
use wow_packet::{ClientPacket, ServerPacket};

use crate::conditions::{
    QUEST_STATUS_COMPLETE_LIKE_CPP, QUEST_STATUS_FAILED_LIKE_CPP, QUEST_STATUS_INCOMPLETE_LIKE_CPP,
    QUEST_STATUS_NONE_LIKE_CPP, QUEST_STATUS_REWARDED_LIKE_CPP,
};
use crate::session::{
    DurableItemLootCompletionLikeCpp, DurableItemLootPersistenceGuardLikeCpp,
    DurableLootItemFanoutLikeCpp, InventoryItem, LootMoneyDeliveryAddressLikeCpp,
    LootMoneyPersistenceErrorLikeCpp, LootMoneyViewerFanoutLikeCpp,
    RepresentedGameObjectSpellCaster, RepresentedGameObjectUseEffect, RepresentedLootRollState,
    RepresentedLootRollVote, RepresentedQuestObjectiveProgressEventLikeCpp, SessionState,
    WorldSession, loot_money_durable_outcome_like_cpp,
};

const LOOT_METHOD_FREE_FOR_ALL_LIKE_CPP: u8 = 0;
const LOOT_METHOD_ROUND_ROBIN_LIKE_CPP: u8 = 1;
const LOOT_METHOD_MASTER_LIKE_CPP: u8 = 2;
const LOOT_METHOD_GROUP_LIKE_CPP: u8 = 3;
const LOOT_METHOD_NEED_BEFORE_GREED_LIKE_CPP: u8 = 4;
const LOOT_METHOD_PERSONAL_LIKE_CPP: u8 = 5;
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

#[derive(Clone)]
struct AuthoritativeLootReleaseLikeCpp {
    authority: OwnedLootAuthority,
    selected_generation: u64,
    loot: CreatureLoot,
    whole_object_fully_looted: bool,
    whole_object_fully_skinned: bool,
    object_generation: u64,
    lifecycle_revision: u64,
    require_no_viewers: bool,
}

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
        if self.has_active_non_item_loot_views_like_cpp() {
            self.do_loot_release_all_like_cpp(player_guid).await;
        }
        self.set_active_loot_guid(req.unit);
        self.represented_on_loot_opened_like_cpp(req.unit, player_guid, response);

        if !ae_owner_guids.is_empty() {
            self.send_packet(&AELootTargetsAck);

            for owner_guid in ae_owner_guids {
                if let Some(response) = self
                    .represented_loot_response_for_owner_like_cpp(owner_guid, player_guid, true)
                    .await
                {
                    self.add_active_loot_view_owner_like_cpp(owner_guid);
                    self.represented_on_loot_opened_like_cpp(owner_guid, player_guid, response);
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
        let allowed_looters = if source.is_personal_encounter_loot_like_cpp() {
            Vec::new()
        } else if source.uses_personal_loot_like_cpp() {
            // C++ creates only `m_personalLoot[player]` for a personal chest
            // without a DungeonEncounter; group loot rules never widen it.
            vec![player_guid]
        } else if source.use_group_loot_rules {
            self.represented_group_looters_at_reward_distance_like_cpp(player_guid)
        } else {
            vec![player_guid]
        };
        self.ensure_represented_gameobject_chest_loot_like_cpp(
            gameobject_guid,
            player_guid,
            source,
            &allowed_looters,
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

        if self
            .sync_represented_gameobject_loot_to_canonical_like_cpp(gameobject_guid, player_guid)
            .is_none()
        {
            self.loot_table.remove(&gameobject_guid);
            return;
        }

        let Some(loot) = self.loot_table.get(&gameobject_guid) else {
            return;
        };
        // C++ keeps and sends an empty non-encounter
        // `m_personalLoot[player]`. Encounter generation instead discards
        // empty pools in `GenerateDungeonEncounterPersonalLoot`, so only the
        // former bypasses the generic item/money availability gate.
        let empty_non_encounter_personal_pool = source.uses_personal_loot_like_cpp()
            && !source.is_personal_encounter_loot_like_cpp()
            && loot.allowed_looters.contains(&player_guid);
        if !empty_non_encounter_personal_pool
            && !self.represented_loot_can_be_opened_by_player_like_cpp(
                gameobject_guid,
                loot,
                player_guid,
            )
        {
            return;
        }

        let response = LootResponse {
            owner: gameobject_guid,
            loot_obj: loot.loot_guid,
            failure_reason: LOOT_RESPONSE_DEFAULT_FAILURE_REASON_LIKE_CPP,
            acquire_reason: loot_type_for_client_like_cpp(loot.loot_type),
            loot_method: loot.loot_method,
            threshold: LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP,
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

        if self.has_active_non_item_loot_views_like_cpp() {
            self.do_loot_release_all_like_cpp(player_guid).await;
        }
        self.set_active_loot_guid(gameobject_guid);
        self.represented_on_loot_opened_like_cpp(gameobject_guid, player_guid, response);
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
        let install_observation =
            self.represented_gameobject_loot_install_observation_like_cpp(gameobject_guid);
        if install_observation.is_none() && !represented_local_loot_fixture_allowed_like_cpp() {
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

        let Some(loot_guid) = self.next_represented_loot_object_guid_like_cpp(gameobject_guid)
        else {
            return;
        };
        self.loot_table.insert(
            gameobject_guid,
            CreatureLoot {
                loot_guid,
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
        let upserted = self
            .loot_table
            .get(&gameobject_guid)
            .cloned()
            .and_then(|loot| {
                install_observation.as_ref().and_then(|observation| {
                    self.upsert_represented_personal_gameobject_loot_authority_if_observed_like_cpp(
                        gameobject_guid,
                        player_guid,
                        loot,
                        false,
                        observation,
                    )
                })
            });
        if upserted.is_none() && !represented_local_loot_fixture_allowed_like_cpp() {
            self.loot_table.remove(&gameobject_guid);
            return;
        }

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
            failure_reason: LOOT_RESPONSE_DEFAULT_FAILURE_REASON_LIKE_CPP,
            acquire_reason: loot_type_for_client_like_cpp(loot.loot_type),
            loot_method: loot.loot_method,
            threshold: LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP,
            coins: loot.coins,
            items: represented_loot_response_items_like_cpp(loot, player_guid),
            currencies: vec![],
            acquired: true,
            ae_looting: false,
        };

        if self.has_active_non_item_loot_views_like_cpp() {
            self.do_loot_release_all_like_cpp(player_guid).await;
        }
        self.set_active_loot_guid(gameobject_guid);
        self.represented_on_loot_opened_like_cpp(gameobject_guid, player_guid, response);
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
                self.give_xp(xp, ObjectGuid::EMPTY, 1.0).await;
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

        for registration in
            registry.same_map_loot_recipients(player_guid, current_map_id, current_instance_id)
        {
            if registry
                .try_send_current_command(
                    registration,
                    SessionCommand::SyncChestGameobjectStateAndRefreshLikeCpp(command.clone()),
                )
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

        for registration in
            registry.same_map_loot_recipients(player_guid, current_map_id, current_instance_id)
        {
            if registry
                .try_send_current_command(
                    registration,
                    SessionCommand::SyncGooberGameobjectStateAndRefreshLikeCpp(command.clone()),
                )
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

        for registration in
            registry.same_map_loot_recipients(player_guid, current_map_id, current_instance_id)
        {
            if registry
                .try_send_current_command(
                    registration,
                    SessionCommand::SendIfVisibleLikeCpp(SendIfVisibleLikeCppCommand {
                        queued_at: Instant::now(),
                        source_guid: gameobject_guid,
                        map_id: current_map_id,
                        instance_id: current_instance_id,
                        packet_bytes: packet_bytes.clone(),
                    }),
                )
                .is_ok()
            {
                queued += 1;
            }
        }

        queued
    }

    fn represented_creature_is_dead_for_loot_visibility_like_cpp(
        &self,
        creature_guid: ObjectGuid,
    ) -> bool {
        let (map_id, instance_id) = self.current_legacy_runtime_map_key_like_cpp();
        if let Some(manager) = self.map_manager.as_ref()
            && let Some(creature) = manager
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .find_creature(map_id, instance_id, creature_guid)
        {
            return !creature.is_alive();
        }

        let Some(map_key) =
            self.canonical_object_lookup_map_key_like_cpp(u32::from(self.player_map_id_like_cpp()))
        else {
            return false;
        };
        let Some(manager) = self.canonical_map_manager.as_ref() else {
            return false;
        };
        let Ok(manager) = manager.lock() else {
            return false;
        };
        manager
            .find_map(map_key.map_id, map_key.instance_id)
            .and_then(|map| map.map().get_typed_creature(creature_guid))
            .is_some_and(|creature| !creature.is_alive())
    }

    fn creature_loot_release_values_for_viewer_like_cpp(
        &self,
        creature_guid: ObjectGuid,
        viewer_guid: ObjectGuid,
        viewer_has_pending_bind: bool,
        authority: Option<&OwnedLootAuthority>,
        mut update: wow_packet::packets::update::UnitDataValuesDeltaUpdate,
    ) -> wow_packet::packets::update::UnitDataValuesDeltaUpdate {
        let Some(object_data) = update.object_data.as_mut() else {
            return update;
        };
        if object_data.dynamic_flags & UnitDynFlags::Lootable as u32 == 0 {
            return update;
        }
        let Some(authority) = authority else {
            // The authority-less path exists only for bounded unit fixtures.
            // Preserve the canonical flag rather than inventing per-viewer
            // ownership without `Creature::GetLootForPlayer` evidence.
            return update;
        };

        // C++ `ViewerDependentValue<ObjectData::DynamicFlags>` removes
        // UNIT_DYNFLAG_LOOTABLE when the complete `Player::isAllowedToLoot`
        // predicate is false. The object-owned authority is the Rust
        // equivalent of `Creature::GetLootForPlayer`; one exhausted personal
        // pool must not hide a different player's still-live pool.
        let creature_is_dead =
            self.represented_creature_is_dead_for_loot_visibility_like_cpp(creature_guid);
        let viewer_can_still_loot = authority
            .snapshot_for_player_like_cpp(viewer_guid)
            .is_some_and(|snapshot| {
                creature_loot_is_allowed_to_player_like_cpp(
                    creature_is_dead,
                    viewer_has_pending_bind,
                    &snapshot.loot,
                    viewer_guid,
                )
            });
        if !viewer_can_still_loot {
            object_data.dynamic_flags &= !(UnitDynFlags::Lootable as u32);
        }
        update
    }

    /// Publishes the dirty DynamicFlags field created by C++
    /// `WorldSession::DoLootRelease` to every same-map session that currently
    /// has the creature at the client. The canonical object mutation alone is
    /// insufficient until the global `Map::SendObjectUpdates` bridge owns
    /// normal VALUES fanout.
    fn send_creature_loot_release_dynamic_flags_update_like_cpp(
        &self,
        creature_guid: ObjectGuid,
        values_update: &wow_entities::UnitValuesUpdate,
        authority: Option<&OwnedLootAuthority>,
    ) -> usize {
        let Some(player_guid) = self.player_guid() else {
            return 0;
        };
        let Some(packet_update) =
            crate::entity_update_bridge::unit_values_update_to_packet(values_update)
        else {
            return 0;
        };
        let map_id = self.player_map_id_like_cpp();
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let mut sent = 0;

        if self.client_visible_guids_like_cpp.contains(&creature_guid) {
            let source_update = self.creature_loot_release_values_for_viewer_like_cpp(
                creature_guid,
                player_guid,
                self.pending_bind.is_some(),
                authority,
                packet_update.clone(),
            );
            self.send_packet(&UpdateObject::unit_values_update(
                creature_guid,
                map_id,
                source_update,
            ));
            sent += 1;
        }

        let Some(registry) = self.player_registry() else {
            return sent;
        };
        let recipients = registry.same_map_loot_recipients(player_guid, map_id, instance_id);
        for registration in recipients {
            // C++'s dirty-field pass cannot silently lose this forced update.
            // Do not retain a DashMap guard (or any map/authority lock) while
            // queueing the bounded target-session command rail.
            if registry.queue_current_command_reliably(
                registration,
                SessionCommand::SendCreatureLootReleaseValuesUpdateLikeCpp(
                    SendCreatureLootReleaseValuesUpdateLikeCppCommand {
                        creature_guid,
                        map_id,
                        instance_id,
                        unit_values_update: packet_update.clone(),
                        authority: authority.cloned(),
                    },
                ),
            ) != crate::session::directory::PlayerDirectoryReliableSendOutcome::StaleOrDisconnected
            {
                sent += 1;
            }
        }

        sent
    }

    /// Receiver-owned half of the loot-release VALUES fanout. Applying
    /// `Player::isAllowedToLoot` here preserves session-local pending-bind
    /// state and avoids serialising one player's dynamic flags for another.
    pub(crate) fn handle_send_creature_loot_release_values_update_command_like_cpp(
        &mut self,
        command: SendCreatureLootReleaseValuesUpdateLikeCppCommand,
    ) {
        if self.state() != crate::session::SessionState::LoggedIn
            || self.player_map_id_like_cpp() != command.map_id
        {
            return;
        }
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        if instance_id != command.instance_id
            || !self
                .client_visible_guids_like_cpp
                .contains(&command.creature_guid)
        {
            return;
        }
        if self.represented_can_receive_creature_message_to_set_by_guid_like_cpp(
            command.creature_guid,
            command.map_id,
            command.instance_id,
            false,
        ) != Some(true)
        {
            return;
        }
        let Some(expected_authority) = command.authority.as_ref() else {
            return;
        };
        let Some(current_authority) =
            self.represented_owned_loot_authority_like_cpp(command.creature_guid)
        else {
            return;
        };
        if !current_authority.shares_storage_like_cpp(expected_authority) {
            // The queued update belongs to an older corpse generation. C++
            // publishes synchronously before respawn; Rust must not apply the
            // delayed VALUES delta to a replacement creature with the same GUID.
            return;
        }
        let Some(viewer_guid) = self.player_guid() else {
            return;
        };
        let viewer_update = self.creature_loot_release_values_for_viewer_like_cpp(
            command.creature_guid,
            viewer_guid,
            self.pending_bind.is_some(),
            Some(expected_authority),
            command.unit_values_update,
        );
        self.send_packet(&UpdateObject::unit_values_update(
            command.creature_guid,
            command.map_id,
            viewer_update,
        ));
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

        for registration in
            registry.same_map_loot_recipients(player_guid, current_map_id, current_instance_id)
        {
            if registry
                .try_send_current_command(
                    registration,
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

        // Fishing holes replace this player's personal `Loot` in place. Close
        // the old C++ view before the upsert so its release cannot detach or
        // apply lifecycle state to the freshly generated pool.
        if replace_existing && self.has_active_non_item_loot_views_like_cpp() {
            self.do_loot_release_all_like_cpp(player_guid).await;
        }

        // C++ serializes template generation and `ClearLoot` on the map
        // thread. Rust awaits database-backed template generation, so retain
        // the exact object lifetime and authority tombstone across that await.
        let install_observation =
            self.represented_gameobject_loot_install_observation_like_cpp(gameobject_guid);
        if install_observation.is_none() && !represented_local_loot_fixture_allowed_like_cpp() {
            return;
        }

        if !replace_existing
            && let Some(snapshot) = self
                .represented_owned_loot_authority_like_cpp(gameobject_guid)
                .and_then(|authority| authority.snapshot_for_player_like_cpp(player_guid))
        {
            self.loot_table.insert(gameobject_guid, snapshot.loot);
            self.represented_loot_cache_generations_like_cpp
                .insert(gameobject_guid, snapshot.generation);
        }

        if replace_existing || !self.loot_table.contains_key(&gameobject_guid) {
            let items = self
                .generate_represented_gameobject_loot_items_for_store_like_cpp(
                    loot_id,
                    LootStoreKind::Gameobject,
                    LOOT_MODE_DEFAULT_LIKE_CPP,
                    None,
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
            let Some(loot_guid) = self.next_represented_loot_object_guid_like_cpp(gameobject_guid)
            else {
                return;
            };
            self.loot_table.insert(
                gameobject_guid,
                CreatureLoot {
                    loot_guid,
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
        self.represented_personal_loot_owners
            .insert(gameobject_guid);
        if let Some(loot) = self.loot_table.get(&gameobject_guid).cloned() {
            let upserted = install_observation.as_ref().and_then(|observation| {
                self.upsert_represented_personal_gameobject_loot_authority_if_observed_like_cpp(
                    gameobject_guid,
                    player_guid,
                    loot,
                    replace_existing,
                    observation,
                )
            });
            if upserted.is_none() && !represented_local_loot_fixture_allowed_like_cpp() {
                self.loot_table.remove(&gameobject_guid);
                self.represented_personal_loot_owners
                    .remove(&gameobject_guid);
                return;
            }
        }

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
            failure_reason: LOOT_RESPONSE_DEFAULT_FAILURE_REASON_LIKE_CPP,
            acquire_reason: loot_type_for_client_like_cpp(loot.loot_type),
            loot_method: loot.loot_method,
            threshold: LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP,
            coins: loot.coins,
            items: represented_loot_response_items_like_cpp(loot, player_guid),
            currencies: vec![],
            acquired: true,
            ae_looting: false,
        };

        if !replace_existing && self.has_active_non_item_loot_views_like_cpp() {
            self.do_loot_release_all_like_cpp(player_guid).await;
        }
        self.set_active_loot_guid(gameobject_guid);
        self.represented_on_loot_opened_like_cpp(gameobject_guid, player_guid, response);
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
        let mut canonical_loot_sync: Vec<ObjectGuid> = Vec::new();

        for loot_req in &req.requests {
            let Some(owner_guid) = self.active_loot_owner_for_loot_object_like_cpp(loot_req.object)
            else {
                self.send_packet(&SLootRelease {
                    loot_obj: ObjectGuid::EMPTY,
                    owner: player_guid,
                });
                continue;
            };

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

            let owned_authority = self
                .prepare_owned_loot_authority_for_active_request_like_cpp(owner_guid, player_guid);
            let authority = owned_authority
                .as_ref()
                .filter(|authority| {
                    authority
                        .snapshot_for_player_like_cpp(player_guid)
                        .is_some()
                })
                .cloned();
            if authority.is_none()
                && (owner_guid.is_creature_or_vehicle() || owner_guid.is_game_object())
                && (owned_authority.is_some() || !represented_local_loot_fixture_allowed_like_cpp())
            {
                self.send_equip_error(InventoryResult::LootGone, None, None, 0, 0);
                continue;
            }
            if let Some(authority) = authority.as_ref() {
                if !self.represented_active_loot_generation_matches_like_cpp(owner_guid, authority)
                {
                    self.send_equip_error(InventoryResult::LootGone, None, None, 0, 0);
                    continue;
                }
                let _ = self.reconcile_represented_loot_cache_like_cpp(owner_guid, player_guid);
            }
            self.ensure_represented_player_looting_like_cpp(owner_guid, player_guid);

            let Some((cached_entry, dungeon_encounter_id)) =
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

            if !cached_entry.has_allowed_looter_like_cpp(player_guid) {
                self.send_packet(&LootReleaseAll);
                continue;
            }

            if cached_entry.flags.blocked {
                self.send_packet(&LootReleaseAll);
                continue;
            }

            if !cached_entry.roll_winner_allows_like_cpp(player_guid) {
                self.send_packet(&LootReleaseAll);
                continue;
            }

            let (entry, claim) = if let Some(authority) = authority {
                let Some(expected_generation) = self
                    .active_loot_view_generations_like_cpp
                    .get(&owner_guid)
                    .copied()
                else {
                    self.send_equip_error(InventoryResult::LootGone, None, None, 0, 0);
                    continue;
                };
                let claim = match authority
                    .reserve_item_for_generation_like_cpp(
                        player_guid,
                        loot_req.loot_list_id,
                        expected_generation,
                    )
                    .await
                {
                    Ok(claim) => claim,
                    Err(_) => {
                        let _ =
                            self.reconcile_represented_loot_cache_like_cpp(owner_guid, player_guid);
                        self.send_equip_error(InventoryResult::LootGone, None, None, 0, 0);
                        continue;
                    }
                };
                if !self
                    .represented_active_loot_claim_generation_matches_like_cpp(owner_guid, &claim)
                {
                    claim.rollback_like_cpp();
                    self.send_equip_error(InventoryResult::LootGone, None, None, 0, 0);
                    continue;
                }
                let LootClaimPayload::Item(entry) = claim.payload_like_cpp() else {
                    claim.rollback_like_cpp();
                    self.send_equip_error(InventoryResult::LootGone, None, None, 0, 0);
                    continue;
                };
                (entry.clone(), Some(claim))
            } else {
                (cached_entry, None)
            };

            let stored = if let Some(claim) = claim.as_ref() {
                self.store_claimed_direct_loot_item_from_owner_like_cpp(
                    &entry,
                    dungeon_encounter_id,
                    owner_guid,
                    loot_req.object,
                    claim,
                )
                .await
            } else {
                self.store_direct_loot_item_from_owner_like_cpp(
                    &entry,
                    dungeon_encounter_id,
                    owner_guid,
                )
                .await
            };
            if !stored {
                continue;
            }

            if owner_guid.is_item() {
                // The detached worker published this exact durable removal to
                // the session tracker before its JoinHandle completed. Apply
                // it here on the normal path; logout/disconnect and the
                // session tick drain the same completion after cancellation.
                self.apply_pending_durable_item_loot_completions_like_cpp()
                    .await;
                debug!(
                    account = self.account_id,
                    item = entry.item_id,
                    quantity = entry.quantity,
                    "Looted item"
                );
                continue;
            }

            if claim.is_some() {
                debug!(
                    account = self.account_id,
                    item = entry.item_id,
                    quantity = entry.quantity,
                    "Looted item"
                );
                continue;
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
                    canonical_loot_sync.push(owner_guid);
                }
            }
        }

        canonical_loot_sync.sort_by_key(|guid| (guid.high_value(), guid.low_value()));
        canonical_loot_sync.dedup();
        for owner_guid in canonical_loot_sync {
            self.refresh_represented_loot_owner_canonical_summary_like_cpp(owner_guid, player_guid);
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
                // C++ only places loot in Player::GetAELootView after the
                // player passed the source's loot-eligibility gate. Keep the
                // same invariant at this represented boundary so a stale or
                // forged local view cannot take another player's money.
                if !loot.allowed_looters.contains(&player_guid) {
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
        let mut legacy_money_processed = false;

        for (loot_guid, loot_obj, money) in &money_by_loot {
            let owned_authority = self
                .prepare_owned_loot_authority_for_active_request_like_cpp(*loot_guid, player_guid);
            let authority = owned_authority
                .as_ref()
                .filter(|authority| {
                    authority
                        .snapshot_for_player_like_cpp(player_guid)
                        .is_some()
                })
                .cloned();
            if authority.is_none()
                && (loot_guid.is_creature_or_vehicle() || loot_guid.is_game_object())
                && (owned_authority.is_some() || !represented_local_loot_fixture_allowed_like_cpp())
            {
                debug!(
                    owner = ?loot_guid,
                    "world-object loot money has no shared authority; refusing session-local fallback"
                );
                continue;
            }
            if let Some(authority) = authority {
                if !self.represented_active_loot_generation_matches_like_cpp(*loot_guid, &authority)
                {
                    debug!(
                        owner = ?loot_guid,
                        "delayed loot-money request does not belong to the active object generation"
                    );
                    continue;
                }
                let _ = self.reconcile_represented_loot_cache_like_cpp(*loot_guid, player_guid);
                self.ensure_represented_player_looting_like_cpp(*loot_guid, player_guid);

                let Some(expected_generation) = self
                    .active_loot_view_generations_like_cpp
                    .get(loot_guid)
                    .copied()
                else {
                    continue;
                };
                let claim = match authority
                    .reserve_money_for_generation_like_cpp(player_guid, expected_generation)
                    .await
                {
                    Ok(claim) => claim,
                    Err(_) => {
                        let _ =
                            self.reconcile_represented_loot_cache_like_cpp(*loot_guid, player_guid);
                        continue;
                    }
                };
                if !self
                    .represented_active_loot_claim_generation_matches_like_cpp(*loot_guid, &claim)
                {
                    claim.rollback_like_cpp();
                    continue;
                }
                let LootClaimPayload::Money(reserved_money) = claim.payload_like_cpp() else {
                    claim.rollback_like_cpp();
                    continue;
                };
                let authority_generation = claim.generation_like_cpp();
                let mut recipients = self.represented_loot_money_recipients_like_cpp(*loot_guid);
                recipients.sort_unstable_by_key(|guid| (guid.high_value(), guid.low_value()));
                recipients.dedup();
                if recipients.is_empty() {
                    recipients.push(player_guid);
                }
                let money_per_player = u64::from(*reserved_money) / recipients.len() as u64;
                let sole_looter = recipients.len() <= 1;
                let payouts = recipients
                    .iter()
                    .copied()
                    .map(|recipient| (recipient, money_per_player))
                    .collect::<Vec<_>>();
                let authority_committed = Arc::new(AtomicBool::new(false));
                let mut deliveries = Vec::with_capacity(recipients.len());
                let mut local_application = None;
                for recipient in recipients.iter().copied() {
                    let durable_applied_amount = Arc::new(AtomicU64::new(0));
                    let send_coin_removed = Arc::new(AtomicBool::new(false));
                    let applied = Arc::new(AtomicBool::new(false));
                    let published = Arc::new(AtomicBool::new(false));
                    let (delivery, application) = if recipient == player_guid {
                        let application = ApplyLootMoneyLikeCppCommand {
                            recipient,
                            loot_owner: *loot_guid,
                            loot_obj: *loot_obj,
                            amount: money_per_player,
                            durable_applied_amount,
                            durable_persistence_tracker: self
                                .durable_loot_money_persistence_tracker_like_cpp(),
                            sole_looter,
                            authority: authority.clone(),
                            authority_generation,
                            authority_committed: Arc::clone(&authority_committed),
                            send_coin_removed,
                            applied,
                            published,
                        };
                        local_application = Some(application.clone());
                        (
                            LootMoneyDeliveryAddressLikeCpp::Source(self.session_command_tx()),
                            application,
                        )
                    } else {
                        let Some(registry) = self.player_registry().cloned() else {
                            deliveries.clear();
                            break;
                        };
                        let Some(prepared) = registry.prepare_loot_money_application(
                            PrepareLootMoneyApplicationLikeCpp {
                                recipient,
                                loot_owner: *loot_guid,
                                loot_obj: *loot_obj,
                                amount: money_per_player,
                                durable_applied_amount,
                                sole_looter,
                                authority: authority.clone(),
                                authority_generation,
                                authority_committed: Arc::clone(&authority_committed),
                                send_coin_removed,
                                applied,
                                published,
                            },
                        ) else {
                            deliveries.clear();
                            break;
                        };
                        (
                            LootMoneyDeliveryAddressLikeCpp::Directory {
                                registry,
                                registration: prepared.registration,
                            },
                            prepared.command,
                        )
                    };
                    deliveries.push((delivery, SessionCommand::ApplyLootMoneyLikeCpp(application)));
                }

                // Eligibility is chosen once, before persistence. If a
                // connected eligible member cannot be admitted, retry the
                // original pool instead of silently changing the divisor.
                if deliveries.len() != recipients.len() || deliveries.is_empty() {
                    claim.rollback_like_cpp();
                    let _ = self.reconcile_represented_loot_cache_like_cpp(*loot_guid, player_guid);
                    continue;
                }

                let current_map = self.player_map_id_like_cpp();
                let current_instance = self
                    .current_canonical_player_map_key_like_cpp()
                    .map(|key| key.instance_id)
                    .unwrap_or(0);
                let viewer_fanout = LootMoneyViewerFanoutLikeCpp {
                    scope_player: player_guid,
                    source_player: player_guid,
                    source_command_tx: self.session_command_tx(),
                    player_registry: self.player_registry().cloned(),
                    map_id: current_map,
                    instance_id: current_instance,
                    loot_owner: *loot_guid,
                    loot_obj: *loot_obj,
                    authority: authority.clone(),
                    authority_generation,
                    payout_recipients: recipients.iter().copied().collect(),
                };

                let persistence = match self.spawn_group_loot_money_persistence_like_cpp(
                    payouts,
                    claim,
                    deliveries,
                    authority_committed,
                    viewer_fanout,
                ) {
                    Ok(persistence) => persistence,
                    Err(error) => {
                        warn!(
                            owner = ?loot_guid,
                            recipients = recipients.len(),
                            amount = money_per_player,
                            %error,
                            "atomic loot-money fanout could not start; pool remains available"
                        );
                        let _ =
                            self.reconcile_represented_loot_cache_like_cpp(*loot_guid, player_guid);
                        continue;
                    }
                };

                if let Err(error) = persistence.await.unwrap_or_else(|join_error| {
                    warn!(
                        owner = ?loot_guid,
                        ?join_error,
                        "atomic loot-money persistence worker terminated"
                    );
                    Err(crate::session::LootMoneyPersistenceErrorLikeCpp::WorkerTerminated)
                }) {
                    warn!(
                        owner = ?loot_guid,
                        recipients = recipients.len(),
                        amount = money_per_player,
                        %error,
                        "atomic loot-money fanout persistence failed; pool remains available"
                    );
                    let _ = self.reconcile_represented_loot_cache_like_cpp(*loot_guid, player_guid);
                    continue;
                }
                if let Some(application) = local_application {
                    self.handle_apply_loot_money_like_cpp_command(application)
                        .await;
                }
                // The detached worker has already committed the authority and
                // queued the durable runtime applications.  Returning to the
                // session loop lets this session drain its own command too.
                continue;
            }

            self.ensure_represented_player_looting_like_cpp(*loot_guid, player_guid);

            if loot_guid.is_item() {
                let cached_amount = u64::from(*money);
                let Some((balance_applied, publication_applied, applied_delta, notified_amount)) =
                    self.persist_and_consume_stored_item_money_like_cpp(*loot_guid, cached_amount)
                        .await
                else {
                    continue;
                };
                let apply_balance = balance_applied
                    .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok();
                let publish = publication_applied
                    .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                    .is_ok();
                if !apply_balance && !publish {
                    continue;
                }

                if apply_balance {
                    let old_money = self.player_gold_like_cpp();
                    let new_money = old_money
                        .checked_add(applied_delta)
                        .filter(|money| *money <= MAX_MONEY_AMOUNT)
                        .unwrap_or(old_money);
                    self.set_player_gold_like_cpp(new_money);
                    if applied_delta != 0 {
                        self.enqueue_represented_quest_objective_progress_like_cpp(
                            RepresentedQuestObjectiveProgressEventLikeCpp::MoneyChanged {
                                old_money,
                                new_money,
                            },
                        );
                    }
                }
                if publish {
                    self.represented_notify_money_removed_like_cpp(*loot_guid);
                    self.send_packet(&LootMoneyNotify {
                        money: notified_amount,
                        money_mod: 0,
                        sole_looter: true,
                    });
                    if let Some(loot) = self.loot_table.get_mut(loot_guid) {
                        loot.coins = 0;
                        if loot_is_looted_like_cpp(loot) {
                            item_release.push(*loot_guid);
                        }
                    }
                }
                if apply_balance || publish {
                    self.drain_represented_quest_objective_progress_like_cpp()
                        .await;
                }
                continue;
            }

            // Every live Creature/Vehicle/GameObject source must use its
            // object-owned authority, and stored Item money has the atomic
            // character/source-row transaction above. The remaining local
            // cache path exists only for pre-authority unit fixtures. Refuse
            // it in production so an unknown future owner type cannot publish
            // CoinRemoved or clear its pool before durable money succeeds.
            if !represented_local_loot_fixture_allowed_like_cpp() {
                debug!(
                    owner = ?loot_guid,
                    "non-authoritative loot-money fallback is disabled in production"
                );
                continue;
            }

            legacy_money_processed = true;
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
                    if let Some(member) = registry.loot_presence(recipient) {
                        let _ =
                            registry.send_current_packet(member.registration, notify.to_bytes());
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

        if legacy_money_processed {
            if let Some((old_money, new_money)) = self
                .mutate_and_persist_player_gold_exclusive_like_cpp(|old_money| {
                    crate::session::loot_money_durable_outcome_like_cpp(
                        old_money,
                        player_money_delta,
                    )
                    .0
                })
                .await
            {
                if old_money != new_money {
                    self.enqueue_represented_quest_objective_progress_like_cpp(
                        RepresentedQuestObjectiveProgressEventLikeCpp::MoneyChanged {
                            old_money,
                            new_money,
                        },
                    );
                }
            }
            self.drain_represented_quest_objective_progress_like_cpp()
                .await;
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

        let Some(loot) = self.loot_table.get(&loot_guid) else {
            return vec![player_guid];
        };
        // C++ shares only LOOT_CORPSE. Pickpocket money is creature-owned but
        // personal; vehicle corpses still share even though their HighGuid is
        // not Creature (`LootHandler.cpp::HandleLootMoneyOpcode`).
        if loot.loot_type != LOOT_TYPE_CORPSE_LIKE_CPP {
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
        let source_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let mut recipients = Vec::new();

        for member_guid in &group.members {
            if !loot.allowed_looters.contains(member_guid) {
                continue;
            }

            if *member_guid == player_guid {
                recipients.push(*member_guid);
                continue;
            }

            let Some(member) = player_registry.loot_presence(*member_guid) else {
                continue;
            };

            if !member.is_in_world
                || member.map_id != self.player_map_id_like_cpp()
                || member.instance_id != source_instance_id
            {
                continue;
            }

            if self.current_map_is_dungeon_like_cpp()
                || source_position.is_within_dist(&member.position, 74.0)
            {
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

    /// Clone the object-owned authority while the map/entity lock is held,
    /// then release that lock before any reservation can await.
    fn represented_owned_loot_authority_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
    ) -> Option<OwnedLootAuthority> {
        if owner_guid.is_creature_or_vehicle() {
            // The legacy and canonical maps deliberately use separate locks.
            // Reconcile optimistically with object-local compare/exchange;
            // blind rebinding can otherwise clobber a newer respawn between
            // the read and write phases.
            for _ in 0..8 {
                let canonical_player_map_key = self.current_canonical_player_map_key_like_cpp();
                let map_key = canonical_player_map_key
                    .or_else(|| {
                        self.canonical_object_lookup_map_key_like_cpp(u32::from(
                            self.player_map_id_like_cpp(),
                        ))
                    })
                    .unwrap_or_else(|| {
                        let (map_id, instance_id) = self.current_legacy_runtime_map_key_like_cpp();
                        wow_map::MapKey::new(u32::from(map_id), instance_id)
                    });
                let map_key_still_valid = |session: &Self| {
                    session.loot_reconciliation_map_key_still_valid_like_cpp(
                        map_key,
                        canonical_player_map_key.is_some(),
                    )
                };
                let legacy =
                    self.read_legacy_creature_loot_authority_on_map_like_cpp(owner_guid, map_key);
                let canonical = self
                    .read_canonical_creature_loot_authority_on_map_like_cpp(owner_guid, map_key);
                let (legacy, canonical) = match (legacy, canonical) {
                    (Some(legacy), Some(canonical)) => (legacy, canonical),
                    (None, None) => return None,
                    (Some(authority), None) | (None, Some(authority)) => {
                        if !map_key_still_valid(self) {
                            continue;
                        }
                        return Some(authority);
                    }
                };
                if !map_key_still_valid(self) {
                    continue;
                }

                let legacy_stamp = legacy.stamp_like_cpp();
                let canonical_stamp = canonical.stamp_like_cpp();
                let selected = crate::session::reconcile_creature_loot_authority_mirrors_like_cpp(
                    &canonical,
                    canonical_stamp,
                    &legacy,
                    legacy_stamp,
                );
                if !map_key_still_valid(self) {
                    continue;
                }
                if self
                    .rebind_canonical_creature_loot_authority_on_map_like_cpp(
                        owner_guid,
                        map_key,
                        &canonical,
                        canonical_stamp,
                        selected.clone(),
                    )
                    .is_none()
                {
                    continue;
                }
                if !map_key_still_valid(self) {
                    continue;
                }
                if self
                    .rebind_legacy_creature_loot_authority_on_map_like_cpp(
                        owner_guid,
                        map_key,
                        &legacy,
                        legacy_stamp,
                        selected.clone(),
                    )
                    .is_none()
                {
                    continue;
                }

                if !map_key_still_valid(self) {
                    continue;
                }
                let converged_legacy = self
                    .read_legacy_creature_loot_authority_on_map_like_cpp(owner_guid, map_key)
                    .is_some_and(|authority| authority.shares_storage_like_cpp(&selected));
                let converged_canonical = self
                    .read_canonical_creature_loot_authority_on_map_like_cpp(owner_guid, map_key)
                    .is_some_and(|authority| authority.shares_storage_like_cpp(&selected));
                if converged_legacy && converged_canonical {
                    return Some(selected);
                }
            }

            // Continuous concurrent replacement is safer as a failed request
            // than as an overwrite of the newest mirror.
            return None;
        }

        if owner_guid.is_game_object() {
            let canonical_player_map_key = self.current_canonical_player_map_key_like_cpp();
            let map_key = canonical_player_map_key.or_else(|| {
                self.canonical_object_lookup_map_key_like_cpp(u32::from(
                    self.player_map_id_like_cpp(),
                ))
            })?;
            let authority =
                self.read_canonical_gameobject_loot_authority_on_map_like_cpp(owner_guid, map_key)?;
            let still_valid = self.loot_reconciliation_map_key_still_valid_like_cpp(
                map_key,
                canonical_player_map_key.is_some(),
            );
            return still_valid.then_some(authority);
        }

        None
    }

    /// Bridge pre-authority represented fixtures (and the equivalent first
    /// live generation) into the object-owned source of truth exactly once.
    /// A retired non-zero generation is never reinstalled from session cache.
    fn prepare_owned_loot_authority_for_active_request_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        scope_player: ObjectGuid,
    ) -> Option<OwnedLootAuthority> {
        let authority = self.represented_owned_loot_authority_like_cpp(owner_guid)?;
        let can_install_first_generation = represented_local_loot_fixture_allowed_like_cpp()
            && authority.is_retired_like_cpp()
            && authority.generation_like_cpp() == 0
            && self.loot_table.contains_key(&owner_guid)
            && (self.active_loot_view_owners.contains(&owner_guid)
                || self.is_active_loot_guid(owner_guid));
        if !can_install_first_generation {
            return Some(authority);
        }

        if owner_guid.is_game_object() {
            let _ = self
                .sync_represented_gameobject_loot_to_canonical_like_cpp(owner_guid, scope_player);
        } else if owner_guid.is_creature_or_vehicle() {
            let _ =
                self.sync_represented_creature_loot_to_canonical_like_cpp(owner_guid, scope_player);
        }

        let authority = self.represented_owned_loot_authority_like_cpp(owner_guid)?;
        if let Some(snapshot) = authority.snapshot_for_player_like_cpp(scope_player) {
            self.active_loot_view_generations_like_cpp
                .entry(owner_guid)
                .or_insert(snapshot.generation);
            self.active_loot_view_authorities_like_cpp
                .entry(owner_guid)
                .or_insert_with(|| authority.clone());
        }
        Some(authority)
    }

    /// Refresh the session-local window from the object-owned source of truth.
    /// The local table remains a packet-building cache only.
    fn reconcile_represented_loot_cache_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> bool {
        let Some(authority) = self.represented_owned_loot_authority_like_cpp(owner_guid) else {
            return false;
        };
        let Some(snapshot) = authority.snapshot_for_player_like_cpp(player_guid) else {
            self.discard_represented_personal_loot_cache_for_player_like_cpp(
                owner_guid,
                player_guid,
            );
            return false;
        };
        self.cache_represented_owned_loot_snapshot_like_cpp(owner_guid, player_guid, snapshot);
        true
    }

    /// Rebuild every session-local field derived from one authoritative
    /// snapshot. In particular, a reopened personal creature view must restore
    /// its personal-owner marker and per-player money mirror; restoring only
    /// `loot_table` would make the same pool behave as shared loot.
    fn cache_represented_owned_loot_snapshot_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        _requested_player_guid: ObjectGuid,
        snapshot: OwnedLootSnapshot,
    ) {
        let OwnedLootSnapshot {
            generation,
            scope,
            loot,
        } = snapshot;
        // One WorldSession caches exactly one selected pool for an owner.
        // Generation scratch may have populated money entries for every
        // encounter tapper, but those peer pools now live in the authority;
        // retaining their session-local markers can misclassify a later
        // shared snapshot as personal loot.
        self.represented_personal_loot_money
            .retain(|(owner, _), _| *owner != owner_guid);
        self.represented_personal_loot_owners.remove(&owner_guid);
        match scope {
            OwnedLootScope::Personal(scope_player_guid) => {
                self.represented_personal_loot_owners.insert(owner_guid);
                self.represented_personal_loot_money
                    .insert((owner_guid, scope_player_guid), loot.coins);
            }
            OwnedLootScope::Shared => {}
        }
        self.loot_table.insert(owner_guid, loot);
        self.represented_loot_cache_generations_like_cpp
            .insert(owner_guid, generation);
    }

    /// Drops only this session/player's packet-building mirror. The canonical
    /// object-owned authority remains the source of truth and rehydrates a
    /// later open. C++ has no session-owned `Loot` clone after a window closes.
    fn discard_represented_personal_loot_cache_for_player_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        _player_guid: ObjectGuid,
    ) {
        self.loot_table.remove(&owner_guid);
        self.represented_loot_cache_generations_like_cpp
            .remove(&owner_guid);
        self.represented_personal_loot_money
            .retain(|(owner, _), _| *owner != owner_guid);
        self.represented_personal_loot_owners.remove(&owner_guid);
    }

    fn refresh_owned_loot_summary_like_cpp(&mut self, owner_guid: ObjectGuid) {
        if owner_guid.is_creature_or_vehicle() {
            if let Some(authority) = self.represented_owned_loot_authority_like_cpp(owner_guid) {
                let _ = self.rebind_legacy_creature_loot_authority_like_cpp(
                    owner_guid,
                    &authority,
                    authority.stamp_like_cpp(),
                    authority.clone(),
                );
                let authority_stamp = authority.stamp_like_cpp();
                let _ = self.rebind_canonical_creature_loot_authority_like_cpp(
                    owner_guid,
                    &authority,
                    authority_stamp,
                    authority.clone(),
                );
            }
        } else if owner_guid.is_game_object() {
            if let Some(authority) = self.represented_owned_loot_authority_like_cpp(owner_guid) {
                let _ = self.rebind_canonical_gameobject_loot_authority_like_cpp(
                    owner_guid,
                    &authority,
                    authority.stamp_like_cpp(),
                    authority.clone(),
                );
            }
        }
    }

    fn represented_loot_authority_pools_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
        loot: CreatureLoot,
        personal: bool,
    ) -> Option<(Option<CreatureLoot>, HashMap<ObjectGuid, CreatureLoot>)> {
        if !personal {
            return Some((Some(loot), HashMap::new()));
        }

        let mut looters = loot.allowed_looters.clone();
        if looters.is_empty() && !player_guid.is_empty() {
            looters.push(player_guid);
        }
        looters.sort_unstable_by_key(|guid| (guid.high_value(), guid.low_value()));
        looters.dedup();

        let mut personal_loot = HashMap::new();
        for (index, looter) in looters.into_iter().enumerate() {
            let mut pool = loot.clone();
            if index != 0 {
                pool.loot_guid = self.next_represented_loot_object_guid_like_cpp(owner_guid)?;
            }
            pool.coins = self
                .represented_personal_loot_money
                .get(&(owner_guid, looter))
                .copied()
                .unwrap_or(0);
            pool.allowed_looters = vec![looter];
            pool.players_looting.retain(|viewer| *viewer == looter);
            pool.items.retain(|entry| {
                entry.allowed_looters.is_empty() || entry.allowed_looters.contains(&looter)
            });
            for entry in &mut pool.items {
                entry.allowed_looters = vec![looter];
            }
            rebuild_represented_personal_loot_counts_preserving_consumed_like_cpp(&mut pool);
            personal_loot.insert(looter, pool);
        }

        Some((None, personal_loot))
    }

    /// Runtime-facing allocator. Production has no fallback: the `cfg(test)`
    /// branch exists only so older packet-cache fixtures can retain their
    /// deterministic owner-derived identity while they are migrated to typed
    /// canonical map objects.
    fn next_represented_loot_object_guid_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
    ) -> Option<ObjectGuid> {
        let canonical = self.next_canonical_loot_object_guid_like_cpp(owner_guid);
        #[cfg(test)]
        {
            canonical.or_else(|| {
                (!owner_guid.is_empty()).then(|| represented_loot_object_guid_like_cpp(owner_guid))
            })
        }
        #[cfg(not(test))]
        {
            canonical
        }
    }

    /// Mirrors `Loot::Loot(Map*)`: every concrete pool receives a fresh
    /// map-owned `HighGuid::LootObject` low GUID. This strict helper always
    /// fails closed when the owner's exact canonical map is unavailable.
    fn next_canonical_loot_object_guid_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
    ) -> Option<ObjectGuid> {
        (|| {
            // Map 0 (Eastern Kingdoms) is a real map, not an unspecified
            // sentinel. C++ allocates from the owner's exact `GetMap()`.
            let owner_map_id = u32::from(owner_guid.map_id());
            let key = self.canonical_object_lookup_map_key_like_cpp(owner_map_id)?;
            if key.map_id != owner_map_id {
                return None;
            }
            let manager = self.canonical_map_manager.as_ref()?;
            let mut manager = manager.lock().ok()?;
            let map = manager.find_map_mut(key.map_id, key.instance_id)?.map_mut();
            let counter = map.generate_low_guid_like_cpp(HighGuid::LootObject).ok()?;
            let map_id = u16::try_from(key.map_id).ok()?;
            // C++ passes realm id 0 to ObjectGuidFactory, where
            // `GetRealmIdForObjectGuid(0)` substitutes the active realm.
            // Rust's factory is explicit, so pass the session realm here.
            Some(ObjectGuid::create_world_object(
                HighGuid::LootObject,
                0,
                self.realm_id(),
                map_id,
                0,
                0,
                counter,
            ))
        })()
    }

    fn sync_represented_gameobject_loot_to_canonical_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> Option<()> {
        let Some(authority) = self.represented_owned_loot_authority_like_cpp(gameobject_guid)
        else {
            return (represented_local_loot_fixture_allowed_like_cpp()
                && self.loot_table.contains_key(&gameobject_guid))
            .then_some(());
        };
        let loot = self.loot_table.get(&gameobject_guid)?.clone();
        let is_personal = self
            .represented_personal_loot_owners
            .contains(&gameobject_guid);
        let (shared, personal) = self.represented_loot_authority_pools_like_cpp(
            gameobject_guid,
            player_guid,
            loot,
            is_personal,
        )?;
        let installed = authority
            .initialize_pristine_like_cpp(shared, personal)
            .installed();
        if !installed
            && authority
                .snapshot_for_player_like_cpp(player_guid)
                .is_none()
        {
            self.loot_table.remove(&gameobject_guid);
            self.represented_loot_cache_generations_like_cpp
                .remove(&gameobject_guid);
            return None;
        }
        self.refresh_owned_loot_summary_like_cpp(gameobject_guid);
        let _ = self.reconcile_represented_loot_cache_like_cpp(gameobject_guid, player_guid);
        Some(())
    }

    fn upsert_represented_personal_gameobject_loot_authority_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        loot: CreatureLoot,
        replace: bool,
    ) -> Option<()> {
        let observation =
            self.represented_gameobject_loot_install_observation_like_cpp(gameobject_guid)?;
        self.upsert_represented_personal_gameobject_loot_authority_if_observed_like_cpp(
            gameobject_guid,
            player_guid,
            loot,
            replace,
            &observation,
        )
    }

    fn represented_gameobject_loot_install_observation_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
    ) -> Option<RepresentedGameObjectLootInstallObservationLikeCpp> {
        self.represented_gameobject_loot_install_observation_result_like_cpp(gameobject_guid)?
    }

    /// Preserves the distinction between a missing canonical owner (`None`)
    /// and an owner whose current lifecycle rejects generation (`Some(None)`).
    /// Test-only packet fixtures may fall back only for the former.
    fn represented_gameobject_loot_install_observation_result_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
    ) -> Option<Option<RepresentedGameObjectLootInstallObservationLikeCpp>> {
        self.mutate_canonical_gameobject_by_guid_like_cpp(gameobject_guid, |gameobject| {
            (gameobject.loot_state() != LootState::JustDeactivated).then(|| {
                let authority = gameobject.loot_authority_like_cpp().clone();
                RepresentedGameObjectLootInstallObservationLikeCpp {
                    object_generation: authority.generation_like_cpp(),
                    authority,
                    loot_lifecycle_revision: gameobject.loot_lifecycle_revision_like_cpp(),
                }
            })
        })
    }

    fn upsert_represented_personal_gameobject_loot_authority_if_observed_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        loot: CreatureLoot,
        replace: bool,
        observation: &RepresentedGameObjectLootInstallObservationLikeCpp,
    ) -> Option<()> {
        self.upsert_represented_personal_gameobject_loot_authority_if_observed_with_empty_policy_like_cpp(
            gameobject_guid,
            player_guid,
            loot,
            replace,
            false,
            observation,
        )
    }

    fn upsert_represented_personal_gameobject_loot_authority_if_observed_with_empty_policy_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        loot: CreatureLoot,
        replace: bool,
        discard_empty_pool: bool,
        observation: &RepresentedGameObjectLootInstallObservationLikeCpp,
    ) -> Option<()> {
        let (_, mut personal) = self.represented_loot_authority_pools_like_cpp(
            gameobject_guid,
            player_guid,
            loot,
            true,
        )?;
        let pool = personal.remove(&player_guid)?;
        if discard_empty_pool && loot_is_looted_like_cpp(&pool) {
            self.discard_represented_personal_loot_cache_for_player_like_cpp(
                gameobject_guid,
                player_guid,
            );
            return None;
        }
        let installed =
            self.mutate_canonical_gameobject_by_guid_like_cpp(gameobject_guid, move |gameobject| {
                gameobject.install_personal_loot_if_lifecycle_like_cpp(
                    &observation.authority,
                    observation.object_generation,
                    observation.loot_lifecycle_revision,
                    player_guid,
                    pool,
                    replace,
                )
            });
        if installed != Some(true) {
            self.discard_represented_personal_loot_cache_for_player_like_cpp(
                gameobject_guid,
                player_guid,
            );
            return None;
        }
        if !self.reconcile_represented_loot_cache_like_cpp(gameobject_guid, player_guid) {
            return None;
        }
        Some(())
    }

    fn sync_represented_creature_loot_to_canonical_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
        _player_guid: ObjectGuid,
    ) -> Option<()> {
        let Some(authority) = self.represented_owned_loot_authority_like_cpp(creature_guid) else {
            return (represented_local_loot_fixture_allowed_like_cpp()
                && self.loot_table.contains_key(&creature_guid))
            .then_some(());
        };
        let loot = self.loot_table.get(&creature_guid)?.clone();
        let is_personal = self
            .represented_personal_loot_owners
            .contains(&creature_guid);
        let (shared, personal) = self.represented_loot_authority_pools_like_cpp(
            creature_guid,
            _player_guid,
            loot,
            is_personal,
        )?;
        let installed = authority
            .initialize_pristine_like_cpp(shared, personal)
            .installed();
        if !installed
            && authority
                .snapshot_for_player_like_cpp(_player_guid)
                .is_none()
        {
            self.loot_table.remove(&creature_guid);
            self.represented_loot_cache_generations_like_cpp
                .remove(&creature_guid);
            return None;
        }
        self.refresh_owned_loot_summary_like_cpp(creature_guid);
        let _ = self.reconcile_represented_loot_cache_like_cpp(creature_guid, _player_guid);
        Some(())
    }

    fn refresh_represented_loot_owner_canonical_summary_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) {
        if owner_guid.is_game_object() {
            let _ = self
                .sync_represented_gameobject_loot_to_canonical_like_cpp(owner_guid, player_guid);
        } else if owner_guid.is_creature_or_vehicle() {
            if self
                .sync_represented_creature_loot_to_canonical_like_cpp(owner_guid, player_guid)
                .is_none()
            {
                self.loot_table.remove(&owner_guid);
                return;
            }
        }
    }

    fn canonical_creature_fully_looted_after_represented_sync_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
        player_guid: ObjectGuid,
        fallback_fully_looted: bool,
    ) -> bool {
        if self
            .sync_represented_creature_loot_to_canonical_like_cpp(creature_guid, player_guid)
            .is_some()
        {
            return self
                .mutate_canonical_creature_by_guid_like_cpp(creature_guid, |creature| {
                    creature.is_fully_looted_like_cpp()
                })
                .unwrap_or(fallback_fully_looted);
        }

        fallback_fully_looted
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

    /// CMSG_LOOT_RELEASE — player closes the loot window.
    ///
    /// C++ `WorldSession::DoLootRelease` creature branch:
    /// `loot->isLooted() && creature->IsFullyLooted()` removes the lootable
    /// dynamic flag and calls `Creature::AllLootRemovedFromCorpse` for a corpse.
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

        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let Some((registration, roll_identity)) = registry.loot_roll_owner(
            player_guid,
            self.player_map_id_like_cpp(),
            instance_id,
            roll.loot_obj,
            roll.loot_list_id,
        ) else {
            return false;
        };

        registry
            .try_send_current_command(
                registration,
                SessionCommand::LootRollVote(LootRollVoteCommand {
                    voter_guid: player_guid,
                    loot_obj: roll.loot_obj,
                    loot_list_id: roll.loot_list_id,
                    roll_type: roll.roll_type,
                    pass_on_group_loot: self.pass_on_group_loot,
                    roll_identity,
                }),
            )
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
        let roll_key = (roll.loot_obj, roll.loot_list_id);
        let Some(roll_state) = self.represented_loot_rolls.get(&roll_key).cloned() else {
            return false;
        };
        if self
            .represented_current_loot_roll_authority_like_cpp(&roll_state)
            .is_none()
        {
            self.cancel_represented_loot_roll_generation_mismatch_like_cpp(roll_key, &roll_state);
            return true;
        }

        if pass_on_group_loot {
            return false;
        }

        let owner_guid = roll_state.owner_guid;

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

        let (roll_number, stored_roll_number) = match roll.roll_type {
            ROLL_VOTE_PASS_LIKE_CPP => (-1, None),
            ROLL_VOTE_NEED_LIKE_CPP => (0, Some(self.represented_urand_u32_like_cpp(1, 100) as u8)),
            ROLL_VOTE_GREED_LIKE_CPP | ROLL_VOTE_DISENCHANT_LIKE_CPP => {
                (-1, Some(self.represented_urand_u32_like_cpp(1, 100) as u8))
            }
            _ => return false,
        };

        let Some(state) = self
            .represented_loot_rolls
            .get_mut(&(loot_guid, roll.loot_list_id))
        else {
            return false;
        };
        let Some(voter) = state.voters.get_mut(&player_guid) else {
            return false;
        };
        voter.vote = roll.roll_type;
        if let Some(stored_roll_number) = stored_roll_number {
            voter.roll_number = stored_roll_number;
        }

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

    /// A represented roll is scoped to one lifetime of the object-owned Loot.
    ///
    /// C++ destroys `LootRoll` together with its owning `Loot`. Rust keeps the
    /// packet-facing roll state in the session, so a recycled object GUID must
    /// not let that stale state unblock or award an item from a later lifetime.
    fn represented_current_loot_roll_authority_like_cpp(
        &mut self,
        state: &RepresentedLootRollState,
    ) -> Option<OwnedLootAuthority> {
        let Some(authority) = self.represented_owned_loot_authority_like_cpp(state.owner_guid)
        else {
            return None;
        };

        if !authority.shares_storage_like_cpp(&state.authority) {
            return None;
        }
        let player_guid = match state.authority_scope {
            wow_loot::OwnedLootScope::Shared => self.player_guid()?,
            wow_loot::OwnedLootScope::Personal(player_guid) => player_guid,
        };
        authority
            .snapshot_for_player_like_cpp(player_guid)
            .is_some_and(|snapshot| {
                snapshot.scope == state.authority_scope
                    && snapshot.generation == state.authority_generation
                    && snapshot.loot.loot_guid == state.loot_obj
            })
            .then_some(authority)
    }

    fn cancel_represented_loot_roll_generation_mismatch_like_cpp(
        &mut self,
        key: (ObjectGuid, u8),
        state: &RepresentedLootRollState,
    ) {
        debug!(
            owner = ?state.owner_guid,
            loot_obj = ?state.loot_obj,
            loot_list_id = state.loot_list_id,
            authority_generation = state.authority_generation,
            "represented loot roll cancelled after owner loot generation changed"
        );
        self.represented_loot_rolls.remove(&key);
        self.publish_represented_loot_roll_ownership_like_cpp();
    }

    async fn finish_represented_loot_roll_like_cpp(
        &mut self,
        loot_obj: ObjectGuid,
        loot_list_id: u8,
        entry: &LootEntry,
        winner: Option<(ObjectGuid, RepresentedLootRollVote)>,
        finished_state: Option<&RepresentedLootRollState>,
    ) {
        let Some(state) = finished_state else {
            return;
        };
        let roll_key = (loot_obj, loot_list_id);
        if state.loot_obj != loot_obj || state.loot_list_id != loot_list_id {
            self.cancel_represented_loot_roll_generation_mismatch_like_cpp(roll_key, state);
            return;
        }
        let Some(authority) = self.represented_current_loot_roll_authority_like_cpp(state) else {
            self.cancel_represented_loot_roll_generation_mismatch_like_cpp(roll_key, state);
            return;
        };
        let owner_guid = state.owner_guid;
        let dungeon_encounter_id = self
            .loot_table
            .get(&owner_guid)
            .map(|loot| loot.dungeon_encounter_id as i32)
            .unwrap_or(0);

        let winner_guid = winner.as_ref().map(|(guid, _)| *guid);
        let scope_player = winner_guid
            .or_else(|| self.player_guid())
            .unwrap_or(ObjectGuid::EMPTY);
        let claim = if let Some(winner_guid) = winner_guid {
            match authority.finish_item_roll_and_reserve_award_like_cpp(
                scope_player,
                state.authority_generation,
                loot_list_id,
                winner_guid,
            ) {
                Ok(claim) => Some(claim),
                Err(_) => {
                    self.cancel_represented_loot_roll_generation_mismatch_like_cpp(roll_key, state);
                    return;
                }
            }
        } else {
            if authority
                .finish_item_roll_like_cpp(
                    scope_player,
                    state.authority_generation,
                    loot_list_id,
                    false,
                    None,
                )
                .is_err()
            {
                self.cancel_represented_loot_roll_generation_mismatch_like_cpp(roll_key, state);
                return;
            }
            None
        };
        let _ = self.reconcile_represented_loot_cache_like_cpp(owner_guid, scope_player);

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
            claim,
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
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let Some(registration) =
            registry.loot_delivery_recipient(target, self.player_map_id_like_cpp(), instance_id)
        else {
            return;
        };

        let _ = registry.send_current_packet(registration, packet.to_bytes());
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
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
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
            let Some(registration) = registry.loot_delivery_recipient(
                *looter,
                self.player_map_id_like_cpp(),
                instance_id,
            ) else {
                continue;
            };

            let _ = registry.send_current_packet(registration, bytes.clone());
        }
    }

    fn broadcast_represented_loot_roll_packet_to_voters_like_cpp<P: ServerPacket>(
        &self,
        packet: &P,
        state: &RepresentedLootRollState,
        except: Option<ObjectGuid>,
    ) {
        let bytes = packet.to_bytes();
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
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
            let Some(registration) = registry.loot_delivery_recipient(
                *player_guid,
                self.player_map_id_like_cpp(),
                instance_id,
            ) else {
                continue;
            };

            let _ = registry.send_current_packet(registration, bytes.clone());
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

            let owned_authority = self
                .prepare_owned_loot_authority_for_active_request_like_cpp(owner_guid, player_guid);
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
                && (owned_authority.is_some() || !represented_local_loot_fixture_allowed_like_cpp())
            {
                self.send_loot_error_like_cpp(
                    req.object,
                    owner_guid,
                    LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
                );
                return;
            }
            if let Some(authority) = authority.as_ref() {
                if !self.represented_active_loot_generation_matches_like_cpp(owner_guid, authority)
                {
                    self.send_loot_error_like_cpp(
                        req.object,
                        owner_guid,
                        LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
                    );
                    return;
                }
                let _ = self
                    .reconcile_represented_loot_cache_like_cpp(owner_guid, master_loot_item.target);
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

            let mut entry = item.clone();
            let claim = if let Some(authority) = authority {
                let Some(expected_generation) = self
                    .active_loot_view_generations_like_cpp
                    .get(&owner_guid)
                    .copied()
                else {
                    self.send_loot_error_like_cpp(
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
                        self.send_loot_error_like_cpp(
                            req.object,
                            owner_guid,
                            LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
                        );
                        return;
                    }
                };
                if !self
                    .represented_active_loot_claim_generation_matches_like_cpp(owner_guid, &claim)
                {
                    claim.rollback_like_cpp();
                    self.send_loot_error_like_cpp(
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
                    self.store_claimed_direct_loot_item_from_owner_like_cpp(
                        &entry,
                        dungeon_encounter_id,
                        owner_guid,
                        req.object,
                        claim,
                    )
                    .await
                } else {
                    self.store_direct_loot_item_from_owner_like_cpp(
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
                    self.mark_represented_master_loot_item_removed_like_cpp(
                        owner_guid,
                        req.object,
                        req.loot_list_id,
                        master_loot_item.target,
                    );
                }
                current_session_assignments = current_session_assignments.saturating_add(1);
            } else {
                let authoritative_claim = claim.is_some();
                match self
                    .request_represented_remote_master_loot_give_like_cpp(
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
                        self.mark_represented_master_loot_item_removed_like_cpp(
                            owner_guid,
                            req.object,
                            req.loot_list_id,
                            master_loot_item.target,
                        );
                    }
                    MasterLootGiveResult::Stored => {}
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
        claim: Option<LootClaimLease>,
    ) -> MasterLootGiveResult {
        let Some(player_guid) = self.player_guid() else {
            return MasterLootGiveResult::TargetMismatch;
        };
        let Some(registry) = self.player_registry() else {
            return MasterLootGiveResult::TargetMismatch;
        };
        let Some(command_address) = registry.control_address(target) else {
            return MasterLootGiveResult::TargetMismatch;
        };

        let (result_tx, result_rx) = flume::bounded(1);
        let command = SessionCommand::MasterLootGive(MasterLootGiveCommand {
            master_guid: player_guid,
            loot_owner: owner_guid,
            loot_obj,
            loot_list_id,
            dungeon_encounter_id,
            entry,
            claim,
            result_tx,
        });

        if command_address.try_send(command).is_err() {
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
        claim: Option<LootClaimLease>,
    ) {
        let dungeon_encounter_id = self
            .loot_table
            .get(&owner_guid)
            .map(|loot| loot.dungeon_encounter_id)
            .unwrap_or(0);
        if winner_vote.vote == ROLL_VOTE_DISENCHANT_LIKE_CPP {
            let reserved_entry = claim
                .as_ref()
                .and_then(|claim| match claim.payload_like_cpp() {
                    LootClaimPayload::Item(entry) => Some(entry),
                    LootClaimPayload::Money(_) => None,
                })
                .unwrap_or(entry);
            if self
                .store_represented_disenchant_loot_winner_like_cpp(
                    owner_guid,
                    loot_obj,
                    loot_list_id,
                    reserved_entry,
                    winner_guid,
                    dungeon_encounter_id,
                    claim.as_ref(),
                )
                .await
            {
                if self.player_guid() == Some(winner_guid) {
                    if claim.is_none() {
                        self.mark_represented_master_loot_item_removed_like_cpp(
                            owner_guid,
                            loot_obj,
                            loot_list_id,
                            winner_guid,
                        );
                    }
                } else if claim.is_none() {
                    // Object-owned claims are committed and fanned out by the
                    // remote target session.  The legacy cache-only fallback
                    // still has to be retired by the source session.
                    self.mark_represented_master_loot_item_removed_like_cpp(
                        owner_guid,
                        loot_obj,
                        loot_list_id,
                        winner_guid,
                    );
                }
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
        if let Some(claim) = claim.as_ref()
            && let LootClaimPayload::Item(reserved_entry) = claim.payload_like_cpp()
        {
            store_entry = reserved_entry.clone();
        }
        store_entry.roll_winner = winner_guid;

        if self.player_guid() == Some(winner_guid) {
            let stored = if let Some(claim) = claim.as_ref() {
                self.store_claimed_direct_loot_item_from_owner_like_cpp(
                    &store_entry,
                    dungeon_encounter_id,
                    owner_guid,
                    loot_obj,
                    claim,
                )
                .await
            } else {
                self.store_direct_loot_item_from_owner_like_cpp(
                    &store_entry,
                    dungeon_encounter_id,
                    owner_guid,
                )
                .await
            };
            if stored {
                if claim.is_none() {
                    self.mark_represented_master_loot_item_removed_like_cpp(
                        owner_guid,
                        loot_obj,
                        loot_list_id,
                        winner_guid,
                    );
                }
            }
            return;
        }

        let authoritative_claim = claim.is_some();
        match self
            .request_represented_remote_loot_roll_winner_store_like_cpp(
                winner_guid,
                owner_guid,
                loot_obj,
                loot_list_id,
                dungeon_encounter_id,
                vec![store_entry],
                false,
                claim,
            )
            .await
        {
            MasterLootGiveResult::Stored if !authoritative_claim => {
                self.mark_represented_master_loot_item_removed_like_cpp(
                    owner_guid,
                    loot_obj,
                    loot_list_id,
                    winner_guid,
                );
            }
            MasterLootGiveResult::Stored => {}
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
        claim: Option<&LootClaimLease>,
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
            return self
                .store_direct_disenchant_batch_like_cpp(
                    &disenchant_entries,
                    dungeon_encounter_id,
                    claim,
                    claim.map(|_| LootItemClaimCommitContextLikeCpp {
                        owner_guid,
                        loot_obj,
                        loot_list_id,
                        player_guid: winner_guid,
                        free_for_all: entry.flags.freeforall,
                    }),
                )
                .await;
        }

        match self
            .request_represented_remote_loot_roll_winner_store_like_cpp(
                winner_guid,
                owner_guid,
                loot_obj,
                loot_list_id,
                dungeon_encounter_id,
                disenchant_entries,
                true,
                claim.cloned(),
            )
            .await
        {
            MasterLootGiveResult::Stored => true,
            MasterLootGiveResult::StoreFailed(error) => {
                debug!(
                    account = self.account_id,
                    winner = ?winner_guid,
                    loot_obj = ?loot_obj,
                    loot_list_id,
                    error,
                    "represented disenchant loot winner batch failed in target session"
                );
                false
            }
            MasterLootGiveResult::TargetMismatch => {
                debug!(
                    account = self.account_id,
                    winner = ?winner_guid,
                    loot_obj = ?loot_obj,
                    loot_list_id,
                    "represented disenchant loot winner target was not connected"
                );
                false
            }
        }
    }

    async fn generate_represented_disenchant_loot_template_entries_like_cpp(
        &mut self,
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

        let mut rng = self.represented_runtime_subrng_like_cpp();
        let mut processed_frames = 0u32;
        while let Some(mut frame) = frames.pop() {
            if frame.requested_group_id > 0 {
                let group_index = usize::from(frame.requested_group_id - 1);
                if let Some(group) = frame.template.groups().get(group_index) {
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
                        &mut rng,
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
                    &mut rng,
                )
            {
                continue;
            }

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
        entries: Vec<LootEntry>,
        is_disenchant: bool,
        claim: Option<LootClaimLease>,
    ) -> MasterLootGiveResult {
        let Some(registry) = self.player_registry() else {
            return MasterLootGiveResult::TargetMismatch;
        };
        let Some(command_address) = registry.control_address(target) else {
            return MasterLootGiveResult::TargetMismatch;
        };

        let (result_tx, result_rx) = flume::bounded(1);
        let command = SessionCommand::LootRollStoreWinner(LootRollStoreWinnerCommand {
            loot_owner: owner_guid,
            loot_obj,
            loot_list_id,
            dungeon_encounter_id,
            entries,
            is_disenchant,
            claim,
            result_tx,
        });

        if command_address.try_send(command).is_err() {
            return MasterLootGiveResult::TargetMismatch;
        }

        timeout(REMOTE_MASTER_LOOT_COMMAND_TIMEOUT, result_rx.recv_async())
            .await
            .ok()
            .and_then(Result::ok)
            .unwrap_or(MasterLootGiveResult::TargetMismatch)
    }

    pub(crate) fn handle_apply_group_removal_command_like_cpp(
        &mut self,
        command: ApplyGroupRemovalLikeCppCommand,
    ) {
        if self.state() != SessionState::LoggedIn {
            return;
        }
        if self.group_guid != Some(command.group_guid) {
            return;
        }

        self.group_guid = None;
        self.clear_represented_group_subgroup_like_cpp();
        self.send_player_party_type_update_like_cpp(command.category, command.party_type);
        self.sync_player_registry_state_like_cpp();

        if command.refresh_visible_gameobjects_or_spellclicks {
            let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
        }
        if command.send_group_destroyed {
            self.send_packet_realm(&wow_packet::packets::party::GroupDestroyed);
        }
        if command.send_group_uninvite {
            self.send_packet_realm(&wow_packet::packets::party::GroupUninvite);
        }
        // C++ `Group::RemoveMember` (`Group.cpp:654-655`) and `Group::Disband`
        // (`Group.cpp:746`) both finish by sending the removed player the
        // destroyed `PartyUpdate` so its client tears down the party frames.
        if command.send_group_destroyed || command.send_group_uninvite {
            self.send_destroyed_group_party_update_like_cpp(command.group_guid, command.category);
        }
    }

    pub(crate) fn handle_apply_group_join_command_like_cpp(
        &mut self,
        command: ApplyGroupJoinLikeCppCommand,
    ) {
        if self.state() != SessionState::LoggedIn {
            return;
        }

        self.apply_group_join_like_cpp(command.group_guid, command.subgroup);
        self.send_player_party_type_update_like_cpp(command.category, command.party_type);

        if command.refresh_visible_gameobjects_or_spellclicks {
            let _ = self.update_visible_gameobjects_or_spell_clicks_like_cpp();
        }
    }

    pub(crate) fn handle_send_party_update_command_like_cpp(
        &mut self,
        mut command: SendPartyUpdateLikeCppCommand,
    ) {
        if self.state() != SessionState::LoggedIn {
            return;
        }
        if self.player_guid() != Some(command.recipient) {
            return;
        }

        command.party_update.sequence_num =
            self.next_group_update_sequence_number_like_cpp(command.party_update.party_index);
        // `SMSG_PARTY_UPDATE` and `SMSG_PARTY_MEMBER_FULL_STATE` are both
        // CONNECTION_TYPE_REALM in legacy C++ Opcodes.cpp:1829/1832.
        self.send_packet_realm(&command.party_update);
        for packet in command.member_full_state_packets {
            self.send_raw_packet_realm(&packet);
        }
    }

    pub(crate) fn handle_apply_group_difficulty_command_like_cpp(
        &mut self,
        command: crate::session::mailbox::ApplyGroupDifficultyLikeCppCommand,
    ) {
        if self.state() != SessionState::LoggedIn {
            return;
        }
        self.apply_group_difficulty_like_cpp(
            command.group_guid,
            command.difficulty_id,
            command.kind,
        );
    }

    pub(crate) fn handle_apply_group_subgroup_command_like_cpp(
        &mut self,
        command: crate::session::mailbox::ApplyGroupSubgroupLikeCppCommand,
    ) {
        if self.state() != SessionState::LoggedIn {
            return;
        }
        self.apply_group_subgroup_like_cpp(command.group_guid, command.subgroup);
    }

    /// Mirrors the small gathering-node state subset that C++ keeps on the
    /// shared GameObject before asking this session to recompute its visible
    /// GameObject dynamic-flag deltas.
    pub(crate) fn handle_sync_gathering_node_gameobject_state_and_refresh_like_cpp(
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
    pub(crate) fn handle_sync_chest_gameobject_state_and_refresh_like_cpp(
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
    pub(crate) fn handle_sync_goober_gameobject_state_and_refresh_like_cpp(
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

    /// Apply a transitional map-owned creature melee compatibility hit to this
    /// player session.
    ///
    /// C++ contrast: `Creature::Update` calls `DoMeleeAttackIfReady()`, which
    /// eventually emits `AttackerStateUpdate` from the map update tick and
    /// then applies damage to the victim. This driver preserves the earlier
    /// normal-hit bridge; it does not claim full `CalculateMeleeDamage` parity.
    /// It owns the swing timer/damage/canonical health mutation once, and this
    /// command is only the victim-session delivery rail. Delivery rereads the
    /// current canonical health/death tuple and advances a presentation-only
    /// revision, so neither retries nor a delayed command can write an older
    /// value over a newer heal, hit, death, or resurrection.
    pub(crate) fn handle_apply_creature_melee_damage_like_cpp_command_like_cpp(
        &mut self,
        command: ApplyCreatureMeleeDamageLikeCppCommand,
    ) {
        if self.state() != crate::session::SessionState::LoggedIn {
            return;
        }
        if self.player_guid() != Some(command.victim_guid) {
            return;
        }
        if self.player_map_id_like_cpp() != command.map_id {
            return;
        }
        let session_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        if session_instance_id != command.instance_id {
            return;
        }
        let Some(canonical_health) = self.present_committed_creature_melee_health_like_cpp(
            command.victim_health_state_revision_after,
        ) else {
            return;
        };

        use wow_packet::packets::combat::{
            AttackerStateUpdate, HIT_INFO_NORMAL_SWING, HealthUpdate, VICTIM_STATE_HIT,
        };
        // Visibility can change after the map-owned swing commits. It gates
        // only the attacker-facing combat packet, never authoritative victim
        // health/death reconciliation.
        if self
            .client_visible_guids_like_cpp
            .contains(&command.attacker_guid)
        {
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
        }
        self.send_packet(&HealthUpdate {
            guid: command.victim_guid,
            health: canonical_health.min(i64::MAX as u64) as i64,
        });
    }

    /// Mirror one map-owned creature aggro transition into this victim session.
    ///
    /// C++ contrast: `CreatureAI::MoveInLineOfSight` calls
    /// `Creature::CanStartAttack` and then engages the target; the combat start
    /// is visible to the client through `Unit::SendMeleeAttackStart`. The map
    /// runtime owns the aggro decision; this handler only gates the victim
    /// session and sends one `AttackStart` packet.
    pub(crate) fn handle_creature_attack_start_like_cpp_command_like_cpp(
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
            .map(|key| key.instance_id)
            .unwrap_or(0);
        if session_instance_id != command.instance_id {
            return;
        }
        let attacker_is_visible = self
            .client_visible_guids_like_cpp
            .contains(&command.attacker_guid);

        if let Some(manager) = self.canonical_map_manager.as_ref().cloned()
            && let Ok(mut manager) = manager.lock()
            && let Some(managed) =
                manager.find_map_mut(u32::from(command.map_id), command.instance_id)
        {
            let map = managed.map_mut();
            if let Some(previous_victim) = command.previous_victim_guid {
                if let Some(player) = map.get_typed_player_mut(previous_victim) {
                    player
                        .unit_mut()
                        .remove_attacker_like_cpp(command.attacker_guid);
                } else if let Some(creature) = map.get_typed_creature_mut(previous_victim) {
                    creature
                        .unit_mut()
                        .remove_attacker_like_cpp(command.attacker_guid);
                }
            }
            if let Some(player) = map.get_typed_player_mut(command.victim_guid) {
                player
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .set_in_combat_with(command.attacker_guid, false, false);
                player
                    .unit_mut()
                    .add_attacker_like_cpp(command.attacker_guid);
            }
            if let Some(creature) = map.get_typed_creature_mut(command.attacker_guid) {
                let combat = &mut creature.unit_mut().subsystems_mut().combat;
                combat.set_in_combat_with(command.victim_guid, false, false);
                if combat.threat_ref(command.victim_guid).is_none() {
                    combat.set_threat(command.victim_guid, 0.0);
                }
                let threat_ref = combat.threat_ref(command.victim_guid).copied();
                if let Some(threat_ref) = threat_ref
                    && let Some(player) = map.get_typed_player_mut(command.victim_guid)
                {
                    player
                        .unit_mut()
                        .subsystems_mut()
                        .combat
                        .put_threatened_by_me_ref(command.attacker_guid, threat_ref);
                }
            }
        }

        // Incoming attackers do not become the player's own melee target.
        // C++ keeps that direction solely in `m_attackers`/combat references.
        self.set_in_combat_like_cpp(true);

        if attacker_is_visible && !command.packet_already_broadcast {
            use wow_packet::packets::combat::AttackStart;
            self.send_packet(&AttackStart {
                attacker: command.attacker_guid,
                victim: command.victim_guid,
            });
        }
    }

    pub(crate) fn handle_creature_attack_stop_like_cpp_command_like_cpp(
        &mut self,
        command: CreatureAttackStopLikeCppCommand,
    ) {
        // This cleanup command is emitted only by the full
        // `LegacyCreatureThreatUpdateLikeCpp::Evade` path. Ordinary victim
        // switches fan out `SMSG_ATTACKSTOP` directly but deliberately do not
        // enqueue this command, matching C++ `Unit::AttackStop()` preserving
        // threat and combat references.
        if self.state() != crate::session::SessionState::LoggedIn
            || self.player_guid() != Some(command.victim_guid)
            || self.player_map_id_like_cpp() != command.map_id
        {
            return;
        }
        let Some(map_key) = self.current_canonical_player_map_key_like_cpp() else {
            return;
        };
        if map_key.instance_id != command.instance_id {
            return;
        }

        let Some(manager) = self.canonical_map_manager.as_ref().cloned() else {
            return;
        };
        let Ok(mut manager) = manager.lock() else {
            return;
        };
        let Some(managed) = manager.find_map_mut(map_key.map_id, map_key.instance_id) else {
            return;
        };
        let map = managed.map_mut();
        let still_in_combat = if let Some(player) = map.get_typed_player_mut(command.victim_guid) {
            player
                .unit_mut()
                .subsystems_mut()
                .combat
                .purge_combat_ref_like_cpp(command.attacker_guid);
            player
                .unit_mut()
                .subsystems_mut()
                .combat
                .purge_threatened_by_me_ref(command.attacker_guid);
            player
                .unit_mut()
                .remove_attacker_like_cpp(command.attacker_guid);
            player.unit().subsystems().combat.has_combat()
        } else {
            false
        };
        if let Some(creature) = map.get_typed_creature_mut(command.attacker_guid) {
            creature
                .unit_mut()
                .subsystems_mut()
                .combat
                .purge_combat_ref_like_cpp(command.victim_guid);
            creature
                .unit_mut()
                .remove_attacker_like_cpp(command.victim_guid);
        }
        if self.combat_target == Some(command.attacker_guid) {
            self.combat_target = None;
        }
        self.set_in_combat_like_cpp(still_in_combat);
    }

    pub(crate) fn handle_reconcile_pvp_combat_expiry_like_cpp(
        &mut self,
        command: ReconcilePvpCombatExpiryLikeCppCommand,
    ) {
        if self.state() != crate::session::SessionState::LoggedIn
            || self.player_guid() != Some(command.player_guid)
            || self.player_map_id_like_cpp() != command.map_id
        {
            return;
        }
        let Some(map_key) = self.current_canonical_player_map_key_like_cpp() else {
            return;
        };
        if map_key.instance_id != command.instance_id {
            return;
        }
        let still_in_combat = self
            .canonical_map_manager
            .as_ref()
            .and_then(|manager| manager.lock().ok())
            .and_then(|manager| {
                manager
                    .find_map(map_key.map_id, map_key.instance_id)
                    .and_then(|managed| managed.map().get_typed_player(command.player_guid))
                    .map(|player| player.unit().subsystems().combat.has_combat())
            })
            .unwrap_or(false);
        self.set_in_combat_like_cpp(still_in_combat);
    }

    pub(crate) fn handle_send_visible_object_values_update_command_like_cpp(
        &mut self,
        command: crate::session::mailbox::SendVisibleObjectValuesUpdateCommand,
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

    /// Shared per-session visibility gate for one or more packet frames.
    ///
    /// Mirrors C++ `GridNotifiers.h : MessageDistDeliverer::SendPacket` and
    /// `GridNotifiersImpl.h : MessageDistDeliverer::Visit(PlayerMapType&)`:
    /// `MessageDistDeliverer::Visit` rechecks phase/distance against the
    /// current source object, then `SendPacket` applies HaveAtClient.
    fn send_if_visible_like_cpp_gate_passes_like_cpp(
        &mut self,
        queued_at: Instant,
        source_guid: ObjectGuid,
        map_id: u16,
        instance_id: u32,
        representative_packet_bytes: &[u8],
        allow_legacy_creature_source: bool,
    ) -> bool {
        let is_monster_move = representative_packet_bytes
            .get(0..2)
            .and_then(|bytes| bytes.try_into().ok())
            .map(u16::from_le_bytes)
            == Some(wow_constants::ServerOpcodes::OnMonsterMove as u16);
        // Gate 1: session must be fully logged in (player object loaded).
        if self.state() != crate::session::SessionState::LoggedIn {
            if is_monster_move {
                tracing::info!(
                    account = self.account_id,
                    source_guid = ?source_guid,
                    "RUST_MONSTER_MOVE_DELIVERY rejected: session not logged in"
                );
            }
            return false;
        }
        // Gate 1b: C++ does not deliver SMSG_ON_MONSTER_MOVE during the
        // initial enter-world packet burst. Rust queues fan-out commands from
        // a sessionless world tick, so drop only movement commands that were
        // queued before the login burst completed.
        if is_monster_move {
            if let Some(cutoff) = self.suppress_creature_movement_queued_at_or_before_like_cpp {
                if queued_at <= cutoff {
                    tracing::info!(
                        account = self.account_id,
                        source_guid = ?source_guid,
                        queued_before_cutoff_ms =
                            cutoff.saturating_duration_since(queued_at).as_millis(),
                        "RUST_MONSTER_MOVE_DELIVERY rejected: queued before enter-world movement cutoff"
                    );
                    return false;
                }
            }
        }
        // Gate 2: map must match.
        if self.player_map_id_like_cpp() != map_id {
            if is_monster_move {
                tracing::info!(
                    account = self.account_id,
                    source_guid = ?source_guid,
                    player_map = self.player_map_id_like_cpp(),
                    command_map = map_id,
                    "RUST_MONSTER_MOVE_DELIVERY rejected: wrong map"
                );
            }
            return false;
        }
        // Gate 3: instance must match.
        let session_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|k| k.instance_id)
            .unwrap_or(0);
        if session_instance_id != instance_id {
            if is_monster_move {
                tracing::info!(
                    account = self.account_id,
                    source_guid = ?source_guid,
                    session_instance_id,
                    command_instance_id = instance_id,
                    "RUST_MONSTER_MOVE_DELIVERY rejected: wrong instance"
                );
            }
            return false;
        }
        // Gate 4: source GUID must be in client's visible set (HaveAtClient).
        if !self.client_visible_guids_like_cpp.contains(&source_guid) {
            if is_monster_move {
                tracing::info!(
                    account = self.account_id,
                    source_guid = ?source_guid,
                    visible_count = self.client_visible_guids_like_cpp.len(),
                    "RUST_MONSTER_MOVE_DELIVERY rejected: source not visible"
                );
            }
            return false;
        }
        // Gate 5: for creature-backed MessageDistDeliverer packets, re-read
        // the current source object and apply C++ Visit(PlayerMapType&): same
        // phase and exact 2D visibility range before SendPacket.
        if source_guid.is_creature() {
            match self
                .represented_can_receive_creature_message_to_set_by_guid_with_legacy_fallback_like_cpp(
                    source_guid,
                    map_id,
                    instance_id,
                    false,
                    allow_legacy_creature_source,
                )
            {
                Some(true) => {}
                Some(false) => {
                    if is_monster_move {
                        tracing::info!(
                            account = self.account_id,
                            source_guid = ?source_guid,
                            visible_count = self.client_visible_guids_like_cpp.len(),
                            "RUST_MONSTER_MOVE_DELIVERY rejected: source failed current creature phase/range gate"
                        );
                    }
                    return false;
                }
                None => {
                    if is_monster_move {
                        tracing::info!(
                            account = self.account_id,
                            source_guid = ?source_guid,
                            visible_count = self.client_visible_guids_like_cpp.len(),
                            "RUST_MONSTER_MOVE_DELIVERY rejected: source creature missing"
                        );
                    }
                    return false;
                }
            }
        }
        true
    }

    pub(crate) fn handle_send_if_visible_like_cpp_command_like_cpp(
        &mut self,
        command: SendIfVisibleLikeCppCommand,
        realm_connection: bool,
        allow_legacy_creature_source: bool,
    ) {
        if !self.send_if_visible_like_cpp_gate_passes_like_cpp(
            command.queued_at,
            command.source_guid,
            command.map_id,
            command.instance_id,
            &command.packet_bytes,
            allow_legacy_creature_source,
        ) {
            return;
        }
        // All gates passed — deliver the already-serialised packet as-is.
        if command
            .packet_bytes
            .get(0..2)
            .and_then(|bytes| bytes.try_into().ok())
            .map(u16::from_le_bytes)
            == Some(wow_constants::ServerOpcodes::OnMonsterMove as u16)
        {
            tracing::info!(
                account = self.account_id,
                source_guid = ?command.source_guid,
                "RUST_MONSTER_MOVE_DELIVERY sent"
            );
        }
        if realm_connection {
            self.send_raw_packet_realm(&command.packet_bytes);
        } else {
            self.send_raw_packet(&command.packet_bytes);
        }
    }

    /// Deliver one map-owned creature START+GO pair after one visibility gate.
    ///
    /// C++ `WorldObject::SendCombatLogMessage` selects the committed full GO
    /// frame for advanced-combat-log viewers and the basic frame otherwise.
    /// Both viewers receive START and their selected GO consecutively with no
    /// command drain or visibility revalidation between them. The two
    /// frame-oriented socket sends are not transactional against other cloned
    /// producers or a receiver closing after START; absolute writer adjacency
    /// needs a future batch-aware socket envelope.
    pub(crate) fn handle_send_creature_spell_cast_if_visible_like_cpp_command_like_cpp(
        &mut self,
        command: SendCreatureSpellCastIfVisibleLikeCppCommand,
    ) {
        let opcode = |packet_bytes: &[u8]| {
            packet_bytes
                .get(0..2)
                .and_then(|bytes| bytes.try_into().ok())
                .map(u16::from_le_bytes)
        };
        if opcode(&command.start_packet_bytes)
            != Some(wow_constants::ServerOpcodes::SpellStart as u16)
            || opcode(&command.go_packet_bytes)
                != Some(wow_constants::ServerOpcodes::SpellGo as u16)
        {
            return;
        }
        // Recipient selection already happened where C++ performs it: inside the
        // synchronous `SendSpellGo` fan-out, against this session's
        // `HaveAtClient` set. Re-deriving it here from the drain-time set would
        // drop a correctly committed pair after a visibility exit and deliver a
        // stale cast to a viewer that only became visible afterwards. Validate
        // that the command belongs to this session incarnation and that the
        // session is still on the map it was committed for, then honor it.
        if !self
            .client_visible_guids_like_cpp
            .shares_storage_like_cpp(&command.committed_visibility_like_cpp)
        {
            return;
        }
        if self.state() != crate::session::SessionState::LoggedIn {
            return;
        }
        if self.player_map_id_like_cpp() != command.map_id {
            return;
        }
        let session_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        if session_instance_id != command.instance_id {
            return;
        }

        // The basic/full combat-log representation was already chosen for this
        // recipient when the cast resolved, so a preference the client toggled
        // since then cannot retroactively change an earlier cast's frame.
        self.send_raw_packet(&command.start_packet_bytes);
        self.send_raw_packet(&command.go_packet_bytes);
    }

    /// Per-session gate for addon chat delivery.
    ///
    /// Mirrors C++ `WorldSession::IsAddonRegistered(prefix)`: when
    /// `_filterAddonMessages` is false, all prefixes are accepted; otherwise
    /// the prefix must be in the session-local registered list.
    pub(crate) fn handle_send_addon_if_registered_like_cpp_command_like_cpp(
        &mut self,
        command: SendAddonIfRegisteredLikeCppCommand,
    ) {
        if self.state() != crate::session::SessionState::LoggedIn {
            return;
        }
        if self.is_addon_registered_like_cpp(&command.prefix) {
            self.send_raw_packet(&command.packet_bytes);
        }
    }

    pub(crate) fn handle_cancel_represented_trade_command_like_cpp(
        &mut self,
        command: CancelRepresentedTradeLikeCppCommand,
    ) {
        if self.represented_active_trade_partner_like_cpp().is_none() {
            return;
        }

        self.record_represented_trade_cancel_like_cpp(command.status);
        self.clear_represented_active_trade_partner_like_cpp();
        self.send_raw_packet(&command.packet_bytes);
    }

    pub(crate) fn handle_send_represented_trade_status_command_like_cpp(
        &mut self,
        command: SendRepresentedTradeStatusLikeCppCommand,
    ) {
        if self.represented_active_trade_partner_like_cpp().is_none() {
            return;
        }

        self.send_raw_packet(&command.packet_bytes);
    }

    pub(crate) fn handle_unaccept_represented_trade_command_like_cpp(
        &mut self,
        command: UnacceptRepresentedTradeLikeCppCommand,
    ) {
        if self.represented_active_trade_partner_like_cpp().is_none() {
            return;
        }

        self.set_represented_trade_accepted_like_cpp_for_command(false);
        self.send_raw_packet(&command.packet_bytes);
    }

    pub(crate) fn handle_send_represented_duel_countdown_command_like_cpp(
        &mut self,
        command: SendRepresentedDuelCountdownLikeCppCommand,
    ) {
        self.send_raw_packet(&command.packet_bytes);
    }

    pub(crate) fn handle_send_represented_duel_requested_command_like_cpp(
        &mut self,
        command: SendRepresentedDuelRequestedLikeCppCommand,
    ) {
        self.set_represented_duel_arbiter_guid_like_cpp(Some(command.arbiter_guid));
        self.send_raw_packet(&command.packet_bytes);
    }

    /// Recompute this session's map-owned creature visibility.
    ///
    /// This is the session-local side of future global creature CREATE/DESTROY
    /// work. C++ performs creature create/out-of-range decisions in
    /// `Player::UpdateVisibilityOf`; this command reuses Rust's represented
    /// `update_visibility` pass instead of sending raw bytes that cannot update
    /// `client_visible_guids_like_cpp`.
    pub(crate) async fn handle_refresh_visible_world_creatures_like_cpp_command_like_cpp(
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
        self.clear_pending_visibility_refresh_like_cpp();
        self.force_update_visibility_like_cpp().await;
    }

    pub(crate) fn handle_send_repeatable_turn_in_request_items_command_like_cpp(
        &mut self,
        command: SendRepeatableTurnInRequestItemsLikeCppCommand,
    ) {
        self.send_repeatable_turn_in_request_items_like_cpp(command.sender_guid, &command.quest);
    }

    pub(crate) fn handle_set_quest_sharing_info_and_send_details_command_like_cpp(
        &mut self,
        command: SetQuestSharingInfoAndSendDetailsCommand,
    ) {
        let Some(receiver_guid) = self.player_guid() else {
            return;
        };

        self.set_represented_pending_quest_sharing_like_cpp(command.sender_guid, command.quest.id);
        self.send_represented_quest_giver_quest_details_like_cpp(
            receiver_guid,
            &command.quest,
            false,
        );
    }

    pub(crate) async fn handle_represented_loot_roll_vote_command_like_cpp(
        &mut self,
        command: LootRollVoteCommand,
    ) {
        let roll_key = (command.loot_obj, command.loot_list_id);
        let Some(current_roll) = self.represented_loot_rolls.get(&roll_key) else {
            return;
        };
        if !Self::represented_loot_roll_vote_command_targets_identity_like_cpp(
            &command,
            &current_roll.command_identity,
        ) {
            return;
        }

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

    fn represented_loot_roll_vote_command_targets_identity_like_cpp(
        command: &LootRollVoteCommand,
        current_identity: &LootRollCommandIdentityLikeCpp,
    ) -> bool {
        current_identity.matches_key_like_cpp(command.loot_obj, command.loot_list_id)
            && current_identity.is_exact_roll_like_cpp(&command.roll_identity)
    }

    pub(crate) async fn handle_apply_loot_money_like_cpp_command(
        &mut self,
        command: ApplyLootMoneyLikeCppCommand,
    ) {
        if self.player_guid() != Some(command.recipient) {
            return;
        }
        let apply_money = !command.applied.swap(true, Ordering::SeqCst);
        let publish = !command.published.swap(true, Ordering::SeqCst);
        if !apply_money && !publish {
            return;
        }

        if publish
            && command.send_coin_removed.load(Ordering::Acquire)
            && command.authority_committed.load(Ordering::Acquire)
            && self.represented_loot_money_command_targets_active_generation_like_cpp(
                command.loot_owner,
                &command.authority,
                command.authority_generation,
            )
        {
            self.send_packet(&CoinRemoved {
                loot_obj: command.loot_obj,
            });
            self.refresh_owned_loot_summary_like_cpp(command.loot_owner);
            if let Some(player_guid) = self.player_guid() {
                let _ =
                    self.reconcile_represented_loot_cache_like_cpp(command.loot_owner, player_guid);
            }
        }
        let durable_applied_amount = command.durable_applied_amount.load(Ordering::Acquire);
        let _ = self
            .apply_durable_represented_loot_money_payout_like_cpp(
                command.amount,
                durable_applied_amount,
                command.sole_looter,
                apply_money,
                publish,
            )
            .await;
    }

    pub(crate) fn handle_notify_loot_money_removed_like_cpp_command(
        &mut self,
        command: NotifyLootMoneyRemovedLikeCppCommand,
    ) {
        if self.player_guid() != Some(command.recipient)
            || !command.authority_committed.load(Ordering::Acquire)
            || !self.represented_loot_money_command_targets_active_generation_like_cpp(
                command.loot_owner,
                &command.authority,
                command.authority_generation,
            )
        {
            return;
        }

        self.send_packet(&CoinRemoved {
            loot_obj: command.loot_obj,
        });
        self.refresh_owned_loot_summary_like_cpp(command.loot_owner);
        if let Some(player_guid) = self.player_guid() {
            let _ = self.reconcile_represented_loot_cache_like_cpp(command.loot_owner, player_guid);
        }
    }

    fn represented_loot_money_command_targets_active_generation_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        expected_authority: &OwnedLootAuthority,
        authority_generation: u64,
    ) -> bool {
        if !self
            .active_loot_view_authorities_like_cpp
            .get(&owner_guid)
            .is_some_and(|active| active.shares_storage_like_cpp(expected_authority))
            || !self
                .active_loot_view_generations_like_cpp
                .get(&owner_guid)
                .is_some_and(|active| *active == authority_generation)
        {
            return false;
        }
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        self.represented_owned_loot_authority_like_cpp(owner_guid)
            .is_some_and(|authority| {
                authority.shares_storage_like_cpp(expected_authority)
                    && authority
                        .snapshot_for_player_like_cpp(player_guid)
                        .is_some_and(|snapshot| snapshot.generation == authority_generation)
            })
    }

    async fn apply_durable_represented_loot_money_payout_like_cpp(
        &mut self,
        notified_amount: u64,
        durable_applied_amount: u64,
        sole_looter: bool,
        apply_money: bool,
        publish: bool,
    ) -> ApplyLootMoneyResultLikeCpp {
        if self.player_guid().is_none() {
            return ApplyLootMoneyResultLikeCpp::TargetMismatch;
        }
        let old_money = self.player_gold_like_cpp();
        let new_money = if apply_money {
            old_money
                .checked_add(durable_applied_amount)
                .filter(|money| *money <= MAX_MONEY_AMOUNT)
                .unwrap_or(old_money)
        } else {
            old_money
        };

        if apply_money && durable_applied_amount != 0 {
            self.enqueue_represented_quest_objective_progress_like_cpp(
                RepresentedQuestObjectiveProgressEventLikeCpp::MoneyChanged {
                    old_money,
                    new_money,
                },
            );
        }
        if apply_money {
            self.set_player_gold_like_cpp(new_money);
        }
        if publish {
            self.send_packet(&LootMoneyNotify {
                money: notified_amount,
                money_mod: 0,
                sole_looter,
            });
        }
        if apply_money || publish {
            self.drain_represented_quest_objective_progress_like_cpp()
                .await;
        }

        ApplyLootMoneyResultLikeCpp::Applied
    }

    pub(crate) async fn handle_represented_master_loot_give_command_like_cpp(
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

        let stored = if let Some(claim) = command.claim.as_ref() {
            self.store_claimed_direct_loot_item_from_owner_like_cpp(
                &command.entry,
                command.dungeon_encounter_id,
                command.loot_owner,
                command.loot_obj,
                claim,
            )
            .await
        } else {
            self.store_direct_loot_item_from_owner_like_cpp(
                &command.entry,
                command.dungeon_encounter_id,
                command.loot_owner,
            )
            .await
        };
        let result = if stored {
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

    pub(crate) async fn handle_represented_loot_roll_store_winner_command_like_cpp(
        &mut self,
        command: LootRollStoreWinnerCommand,
    ) {
        let Some(player_guid) = self.player_guid() else {
            let _ = command.result_tx.send(MasterLootGiveResult::TargetMismatch);
            return;
        };

        if command.entries.is_empty()
            || (!command.is_disenchant && command.entries.len() != 1)
            || command.entries.iter().any(|entry| {
                entry.allowed_looters.is_empty()
                    || !entry.allowed_looters.contains(&player_guid)
                    || !entry.roll_winner_allows_like_cpp(player_guid)
            })
        {
            let _ = command.result_tx.send(MasterLootGiveResult::StoreFailed(
                LOOT_ERROR_MASTER_OTHER_LIKE_CPP,
            ));
            return;
        }

        if let Some(error) = command.entries.iter().find_map(|entry| {
            self.represented_master_loot_can_store_error_like_cpp(
                player_guid,
                entry.item_id,
                entry.quantity,
            )
        }) {
            let _ = command
                .result_tx
                .send(MasterLootGiveResult::StoreFailed(error));
            return;
        }

        let stored = if command.is_disenchant {
            self.store_direct_disenchant_batch_like_cpp(
                &command.entries,
                command.dungeon_encounter_id,
                command.claim.as_ref(),
                command
                    .claim
                    .as_ref()
                    .map(|claim| LootItemClaimCommitContextLikeCpp {
                        owner_guid: command.loot_owner,
                        loot_obj: command.loot_obj,
                        loot_list_id: command.loot_list_id,
                        player_guid,
                        free_for_all: match claim.payload_like_cpp() {
                            LootClaimPayload::Item(entry) => entry.flags.freeforall,
                            LootClaimPayload::Money(_) => false,
                        },
                    }),
            )
            .await
        } else if let Some(claim) = command.claim.as_ref() {
            self.store_claimed_direct_loot_item_from_owner_like_cpp(
                &command.entries[0],
                command.dungeon_encounter_id,
                command.loot_owner,
                command.loot_obj,
                claim,
            )
            .await
        } else {
            self.store_direct_loot_item_from_owner_like_cpp(
                &command.entries[0],
                command.dungeon_encounter_id,
                command.loot_owner,
            )
            .await
        };
        let result = if stored {
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

    fn prepare_durable_loot_item_fanout_like_cpp(
        &mut self,
        claim: &LootClaimLease,
        context: LootItemClaimCommitContextLikeCpp,
    ) -> Option<DurableLootItemFanoutLikeCpp> {
        let authority = self
            .represented_owned_loot_authority_like_cpp(context.owner_guid)
            .filter(|authority| claim.shares_authority_like_cpp(authority))?;
        let precommit_snapshot = authority
            .snapshot_for_player_like_cpp(context.player_guid)
            .filter(|snapshot| {
                snapshot.generation == claim.generation_like_cpp()
                    && snapshot.loot.loot_guid == context.loot_obj
                    && snapshot
                        .loot
                        .items
                        .iter()
                        .any(|entry| entry.loot_list_id == context.loot_list_id)
            })?;
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        Some(DurableLootItemFanoutLikeCpp {
            owner_guid: context.owner_guid,
            loot_obj: context.loot_obj,
            loot_list_id: context.loot_list_id,
            player_guid: context.player_guid,
            free_for_all: context.free_for_all,
            authority,
            authority_generation: claim.generation_like_cpp(),
            precommit_snapshot,
            committed_snapshot: Arc::new(std::sync::OnceLock::new()),
            source_send_tx: self.send_tx().clone(),
            player_registry: self.player_registry().cloned(),
            map_id: self.player_map_id_like_cpp(),
            instance_id,
            published: Arc::new(AtomicBool::new(false)),
        })
    }

    fn publish_durable_loot_item_fanout_like_cpp(
        &mut self,
        route: &DurableLootItemFanoutLikeCpp,
    ) -> bool {
        let Some(committed_snapshot) = route.committed_snapshot.get().filter(|snapshot| {
            snapshot.generation == route.authority_generation
                && snapshot.loot.loot_guid == route.loot_obj
        }) else {
            // Never replace the serialization cut with a later authority
            // sample. The latter may include a viewer that opened after the
            // item commit and already received a response without this slot.
            return false;
        };
        if route
            .published
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return true;
        }

        // C++ serializes StoreLootItem before a later LootRelease and notifies
        // synchronously. Preserve that already ordered cohort across Rust's
        // SQL wait, then add only viewers captured by the item mutation. A
        // post-COMMIT opener is excluded because it saw the removed slot.
        let viewers = durable_loot_item_fanout_viewers_like_cpp(
            &route.precommit_snapshot.loot.players_looting,
            &committed_snapshot.loot.players_looting,
        );
        let Some(entry) = committed_snapshot
            .loot
            .items
            .iter()
            .find(|entry| entry.loot_list_id == route.loot_list_id)
        else {
            return false;
        };
        let allowed_looters = entry
            .allowed_looters
            .iter()
            .copied()
            .collect::<HashSet<_>>();
        let packet = LootRemoved {
            owner: route.owner_guid,
            loot_obj: route.loot_obj,
            loot_list_id: route.loot_list_id,
        };
        let bytes = packet.to_bytes();
        let mut stale_viewers = Vec::new();

        if route.free_for_all {
            let _ = route.source_send_tx.send(bytes);
        } else {
            for viewer in viewers {
                if !allowed_looters.contains(&viewer) {
                    continue;
                }
                if viewer == route.player_guid {
                    let _ = route.source_send_tx.send(bytes.clone());
                    continue;
                }
                let Some(registry) = route.player_registry.as_ref() else {
                    stale_viewers.push(viewer);
                    continue;
                };
                let Some(registration) =
                    registry.loot_delivery_recipient(viewer, route.map_id, route.instance_id)
                else {
                    stale_viewers.push(viewer);
                    continue;
                };
                if registry
                    .send_current_packet(registration, bytes.clone())
                    .is_err()
                {
                    stale_viewers.push(viewer);
                }
            }
        }

        for viewer in stale_viewers {
            let _ = route
                .authority
                .remove_viewer_if_generation_like_cpp(route.authority_generation, viewer);
        }

        self.refresh_owned_loot_summary_like_cpp(route.owner_guid);
        if self.player_guid() == Some(route.player_guid) {
            let _ =
                self.reconcile_represented_loot_cache_like_cpp(route.owner_guid, route.player_guid);
        }
        self.finalize_unviewed_durable_loot_owner_like_cpp(route);
        true
    }

    fn finalize_unviewed_durable_loot_owner_like_cpp(
        &mut self,
        route: &DurableLootItemFanoutLikeCpp,
    ) {
        let same_view_still_open = self
            .active_loot_view_authorities_like_cpp
            .get(&route.owner_guid)
            .is_some_and(|authority| authority.shares_storage_like_cpp(&route.authority))
            && self
                .active_loot_view_generations_like_cpp
                .get(&route.owner_guid)
                .is_some_and(|generation| *generation == route.authority_generation);
        if same_view_still_open {
            return;
        }
        if !self
            .represented_owned_loot_authority_like_cpp(route.owner_guid)
            .is_some_and(|authority| authority.shares_storage_like_cpp(&route.authority))
        {
            return;
        }
        let Some(observation) = route
            .authority
            .fully_looted_unviewed_lifecycle_observation_like_cpp()
        else {
            return;
        };
        let Some(snapshot) = route
            .authority
            .snapshot_for_player_like_cpp(route.player_guid)
            .filter(|snapshot| snapshot.generation == route.authority_generation)
        else {
            return;
        };

        self.loot_table
            .insert(route.owner_guid, snapshot.loot.clone());
        self.represented_loot_cache_generations_like_cpp
            .insert(route.owner_guid, snapshot.generation);

        if route.owner_guid.is_game_object() {
            let release = AuthoritativeLootReleaseLikeCpp {
                authority: route.authority.clone(),
                selected_generation: route.authority_generation,
                loot: snapshot.loot,
                whole_object_fully_looted: true,
                whole_object_fully_skinned: observation.whole_object_fully_skinned,
                object_generation: observation.object_generation,
                lifecycle_revision: observation.lifecycle_revision,
                require_no_viewers: true,
            };
            self.apply_represented_gameobject_loot_release_like_cpp(
                route.owner_guid,
                route.player_guid,
                true,
                true,
                Some(&release),
            );
            let _ =
                self.queue_chest_gameobject_state_refresh_for_same_map_like_cpp(route.owner_guid);
            self.hide_represented_gameobject_for_player_after_loot_release_like_cpp(
                route.owner_guid,
            );
            if self
                .represented_gameobject_use_states
                .get(&route.owner_guid)
                .and_then(|state| state.go_type)
                .map(u32::from)
                == Some(GAMEOBJECT_TYPE_GATHERING_NODE)
            {
                self.send_gathering_node_loot_release_dynamic_flags_update_like_cpp(
                    route.owner_guid,
                );
            }
            self.loot_table.remove(&route.owner_guid);
            return;
        }

        if route.owner_guid.is_corpse() {
            self.remove_canonical_corpse_lootable_dynamic_flag_if_unviewed_fully_looted_observation_like_cpp(
                route.owner_guid,
                &route.authority,
                observation.object_generation,
                observation.lifecycle_revision,
            );
            self.loot_table.remove(&route.owner_guid);
            return;
        }

        if !route.owner_guid.is_creature_or_vehicle() {
            return;
        }

        let corpse_decay_looted_rate = self.loot_drop_rates_like_cpp().corpse_decay_looted;
        let whole_object_fully_skinned = observation.whole_object_fully_skinned;
        let lifecycle_update = self
            .mutate_world_creature_if_unviewed_fully_looted_observation_like_cpp(
                route.owner_guid,
                &route.authority,
                observation.object_generation,
                observation.lifecycle_revision,
                |creature| {
                    creature.force_dynamic_flags_update_like_cpp();
                    creature.remove_lootable_dynamic_flag_like_cpp();
                    let marked = if creature.is_alive() {
                        None
                    } else {
                        let corpse_decay_secs = looted_corpse_decay_secs_like_cpp(
                            whole_object_fully_skinned,
                            creature.corpse_delay_secs_like_cpp(),
                            creature.ignore_corpse_decay_ratio_like_cpp(),
                            corpse_decay_looted_rate,
                        );
                        creature
                            .all_loot_removed_from_corpse_like_cpp(
                                corpse_decay_looted_rate,
                                whole_object_fully_skinned,
                            )
                            .then_some((creature.entry(), corpse_decay_secs))
                    };
                    (marked, creature.creature.unit().values_update())
                },
            );
        self.loot_table.remove(&route.owner_guid);
        if let Some((_, values_update)) = lifecycle_update.as_ref() {
            self.send_creature_loot_release_dynamic_flags_update_like_cpp(
                route.owner_guid,
                values_update,
                Some(&route.authority),
            );
        }
        let marked = lifecycle_update.and_then(|(marked, _)| marked);
        if let Some((entry, corpse_decay_secs)) = marked {
            info!(
                "Creature {:?} (entry {}) fully looted after durable claim — despawning in {}s",
                route.owner_guid, entry, corpse_decay_secs
            );
        }
    }

    fn commit_represented_loot_item_claim_like_cpp(
        &mut self,
        claim: &LootClaimLease,
        context: LootItemClaimCommitContextLikeCpp,
        fanout: Option<&DurableLootItemFanoutLikeCpp>,
    ) -> bool {
        if !claim.is_committed_like_cpp() {
            match claim.commit_with_snapshot_like_cpp() {
                Ok((_, Some(snapshot))) => {
                    if let Some(fanout) = fanout {
                        let _ = fanout.committed_snapshot.set(snapshot);
                    }
                }
                Ok((_, None)) => {
                    warn!(
                        owner = ?context.owner_guid,
                        loot_list_id = context.loot_list_id,
                        "durable loot item committed without an exact authority snapshot"
                    );
                    return false;
                }
                Err(error) => {
                    warn!(
                        owner = ?context.owner_guid,
                        loot_list_id = context.loot_list_id,
                        ?error,
                        "durable loot item could not commit its object-owned claim"
                    );
                    return false;
                }
            }
        }
        let Some(fanout) = fanout.filter(|fanout| {
            fanout.owner_guid == context.owner_guid
                && fanout.loot_obj == context.loot_obj
                && fanout.loot_list_id == context.loot_list_id
                && fanout.player_guid == context.player_guid
                && fanout.free_for_all == context.free_for_all
                && claim.shares_authority_like_cpp(&fanout.authority)
                && claim.generation_like_cpp() == fanout.authority_generation
        }) else {
            warn!(
                owner = ?context.owner_guid,
                loot_list_id = context.loot_list_id,
                "durable loot item committed without its retained fanout route"
            );
            return false;
        };
        self.publish_durable_loot_item_fanout_like_cpp(fanout)
    }

    fn publish_persisted_loot_item_removal_like_cpp(
        &mut self,
        claim: Option<&LootClaimLease>,
        context: Option<LootItemClaimCommitContextLikeCpp>,
        fanout: Option<&DurableLootItemFanoutLikeCpp>,
    ) -> bool {
        match (claim, context, fanout) {
            (None, None, None) => true,
            (Some(claim), Some(context), fanout) => {
                self.commit_represented_loot_item_claim_like_cpp(claim, context, fanout)
            }
            _ => {
                warn!("durable loot item claim/context mismatch before removal publication");
                false
            }
        }
    }

    fn mark_represented_master_loot_item_removed_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        loot_obj: ObjectGuid,
        loot_list_id: u8,
        target: ObjectGuid,
    ) {
        {
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
        }

        self.refresh_represented_loot_owner_canonical_summary_like_cpp(owner_guid, target);
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

        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        self.player_registry()
            .and_then(|registry| {
                registry.loot_delivery_recipient(target, self.player_map_id_like_cpp(), instance_id)
            })
            .is_some()
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
            // C++ `CMSG_LOOT_UNIT` only reads the Loot created by
            // `Unit::Kill`; it never regenerates a corpse pool. Reconcile the
            // active object-owned generation and fail closed if kill-time
            // generation is absent or the corpse lifetime was retired.
            if !self.reconcile_represented_loot_cache_like_cpp(owner_guid, player_guid) {
                self.loot_table.remove(&owner_guid);
                continue;
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
        // `Player::isAllowedToLoot` reads `Creature::GetLootForPlayer`; the
        // client request is not a generation trigger. A retired/missing
        // authority therefore means there is no loot response.
        if !self.reconcile_represented_loot_cache_like_cpp(owner_guid, player_guid) {
            self.loot_table.remove(&owner_guid);
            return None;
        }

        let loot = self.loot_table.get(&owner_guid)?;
        if !self.represented_loot_can_be_opened_by_player_like_cpp(owner_guid, loot, player_guid) {
            return None;
        }

        Some(LootResponse {
            owner: owner_guid,
            loot_obj: loot.loot_guid,
            failure_reason: LOOT_RESPONSE_DEFAULT_FAILURE_REASON_LIKE_CPP,
            acquire_reason: loot_type_for_client_like_cpp(loot.loot_type),
            loot_method: loot.loot_method,
            threshold: LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP,
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
        let loot_scope_player_guid = if self.current_map_dungeon_state_like_cpp() == Some(false) {
            let connected_tappers =
                self.represented_connected_creature_tappers_like_cpp(&creature.tappers);
            self.player_guid()
                .filter(|player_guid| connected_tappers.contains(player_guid))
                .or_else(|| connected_tappers.first().copied())
                .unwrap_or(loot_owner_guid)
        } else {
            loot_owner_guid
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
            &creature.tappers,
            creature.loot_lifecycle_revision,
        )
        .await;
        if self
            .sync_represented_creature_loot_to_canonical_like_cpp(
                creature_guid,
                loot_scope_player_guid,
            )
            .is_none()
        {
            self.loot_table.remove(&creature_guid);
        }
    }

    fn represented_on_loot_opened_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
        mut response: LootResponse,
    ) {
        let authority = self
            .prepare_owned_loot_authority_for_active_request_like_cpp(owner_guid, player_guid)
            .filter(|authority| {
                authority
                    .snapshot_for_player_like_cpp(player_guid)
                    .is_some()
            });
        let authoritative_open = if let Some(authority) = authority.as_ref() {
            match authority.try_open_view_with_snapshot_like_cpp(
                player_guid,
                |snapshot, outcome| {
                    // Enqueue the response while the authority mutex still
                    // excludes item/money commit. Any later commit therefore
                    // observes this viewer and its removal packet is ordered
                    // after the response on this session's send queue.
                    response.loot_obj = snapshot.loot.loot_guid;
                    response.acquire_reason =
                        loot_type_for_client_like_cpp(snapshot.loot.loot_type);
                    response.loot_method = snapshot.loot.loot_method;
                    response.coins = snapshot.loot.coins;
                    response.items =
                        represented_loot_response_items_like_cpp(&snapshot.loot, player_guid);

                    // `flume::Sender::send` may wait indefinitely while this
                    // lock is held. Reject a saturated/disconnected socket
                    // queue immediately; the authority method rolls back its
                    // tentative viewer and first-open mutations before unlock.
                    if !self.try_send_packet(&response) {
                        return None;
                    }

                    // Session mirrors become observable only after the client
                    // response was accepted by its ordered send queue.
                    self.loot_table.insert(owner_guid, snapshot.loot.clone());
                    self.represented_loot_cache_generations_like_cpp
                        .insert(owner_guid, snapshot.generation);
                    self.active_loot_view_generations_like_cpp
                        .insert(owner_guid, outcome.generation);
                    self.active_loot_view_authorities_like_cpp
                        .insert(owner_guid, authority.clone());
                    Some(())
                },
            ) {
                Ok((outcome, ())) => Some(outcome),
                Err(LootClaimError::ResponseEnqueueFailed) => {
                    // Do not attempt a blocking release on the same saturated
                    // queue. The client never observed this view, so dropping
                    // every local mirror is the closed state.
                    self.discard_represented_personal_loot_cache_for_player_like_cpp(
                        owner_guid,
                        player_guid,
                    );
                    self.clear_active_loot_guid_if(owner_guid);
                    return;
                }
                Err(_) => None,
            }
        } else {
            None
        };
        if authoritative_open.is_none() {
            if (owner_guid.is_creature_or_vehicle() || owner_guid.is_game_object())
                && !represented_local_loot_fixture_allowed_like_cpp()
            {
                self.close_stale_active_loot_view_like_cpp(owner_guid, player_guid);
                return;
            }
            self.send_packet(&response);
            self.ensure_represented_player_looting_like_cpp(owner_guid, player_guid);
        } else if let Some(authority) = authority.as_ref() {
            if !self
                .active_loot_view_authorities_like_cpp
                .get(&owner_guid)
                .is_some_and(|opened| opened.shares_storage_like_cpp(authority))
            {
                self.active_loot_view_authorities_like_cpp
                    .insert(owner_guid, authority.clone());
            }
        }

        self.represented_notify_loot_list_like_cpp(owner_guid);

        let first_open = match authoritative_open {
            Some(outcome) => outcome.first_viewer,
            None => match self.loot_table.get_mut(&owner_guid) {
                Some(loot) if !loot.looted_by_player => {
                    loot.looted_by_player = true;
                    true
                }
                _ => false,
            },
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

    /// True only while a request still belongs to the exact object lifetime
    /// whose loot window this session opened.
    fn represented_active_loot_generation_matches_like_cpp(
        &self,
        owner_guid: ObjectGuid,
        authority: &OwnedLootAuthority,
    ) -> bool {
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        let current_generation = authority
            .snapshot_for_player_like_cpp(player_guid)
            .map(|snapshot| snapshot.generation);
        self.active_loot_view_authorities_like_cpp
            .get(&owner_guid)
            .is_some_and(|opened| opened.shares_storage_like_cpp(authority))
            && self
                .active_loot_view_generations_like_cpp
                .get(&owner_guid)
                .is_some_and(|opened| Some(*opened) == current_generation)
    }

    fn represented_active_loot_claim_generation_matches_like_cpp(
        &self,
        owner_guid: ObjectGuid,
        claim: &LootClaimLease,
    ) -> bool {
        self.active_loot_view_authorities_like_cpp
            .get(&owner_guid)
            .is_some_and(|opened| claim.shares_authority_like_cpp(opened))
            && self
                .active_loot_view_generations_like_cpp
                .get(&owner_guid)
                .is_some_and(|opened| *opened == claim.generation_like_cpp())
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
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);

        for allowed_looter in &loot.allowed_looters {
            if Some(*allowed_looter) == self.player_guid() {
                self.send_packet(&packet);
                continue;
            }

            let Some(registry) = self.player_registry() else {
                continue;
            };
            let Some(registration) = registry.loot_delivery_recipient(
                *allowed_looter,
                self.player_map_id_like_cpp(),
                instance_id,
            ) else {
                continue;
            };

            let _ = registry.send_current_packet(registration, bytes.clone());
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
        &mut self,
        owner_guid: ObjectGuid,
        loot_list_id: u8,
    ) {
        let Some(loot) = self.loot_table.get(&owner_guid).cloned() else {
            return;
        };
        let snapshot = wow_loot::OwnedLootSnapshot {
            generation: self
                .represented_loot_cache_generations_like_cpp
                .get(&owner_guid)
                .copied()
                .unwrap_or(0),
            scope: wow_loot::OwnedLootScope::Shared,
            loot,
        };
        self.represented_notify_loot_item_removed_from_snapshot_like_cpp(
            owner_guid,
            None,
            &snapshot,
            loot_list_id,
        );
    }

    fn represented_notify_loot_item_removed_from_snapshot_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        authority: Option<&OwnedLootAuthority>,
        snapshot: &wow_loot::OwnedLootSnapshot,
        loot_list_id: u8,
    ) {
        let loot = &snapshot.loot;
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
        let players_looting = loot.players_looting.clone();
        let allowed_looters = entry.allowed_looters.clone();
        let current_player = self.player_guid();
        let current_map = self.player_map_id_like_cpp();
        let current_instance = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let registry = self.player_registry().cloned();
        let mut stale_looters = Vec::new();

        for looter in &players_looting {
            if !allowed_looters.contains(looter) {
                continue;
            }

            if Some(*looter) == current_player {
                self.send_packet(&packet);
                continue;
            }

            let Some(registry) = registry.as_ref() else {
                stale_looters.push(*looter);
                continue;
            };
            let Some(registration) =
                registry.loot_delivery_recipient(*looter, current_map, current_instance)
            else {
                stale_looters.push(*looter);
                continue;
            };
            if registry
                .send_current_packet(registration, bytes.clone())
                .is_err()
            {
                stale_looters.push(*looter);
            }
        }

        if !stale_looters.is_empty()
            && let Some(loot) = self.loot_table.get_mut(&owner_guid)
        {
            loot.players_looting
                .retain(|looter| !stale_looters.contains(looter));
        }
        if !stale_looters.is_empty() {
            if let Some(authority) = authority {
                for looter in stale_looters {
                    let _ =
                        authority.remove_viewer_if_generation_like_cpp(snapshot.generation, looter);
                }
            }
        }
    }

    fn represented_notify_money_removed_like_cpp(&mut self, owner_guid: ObjectGuid) {
        if let Some(player_guid) = self.player_guid() {
            let _ = self.reconcile_represented_loot_cache_like_cpp(owner_guid, player_guid);
        }
        let Some(loot) = self.loot_table.get(&owner_guid) else {
            return;
        };

        let packet = CoinRemoved {
            loot_obj: loot.loot_guid,
        };
        let bytes = packet.to_bytes();
        let players_looting = loot.players_looting.clone();
        let current_player = self.player_guid();
        let current_map = self.player_map_id_like_cpp();
        let current_instance = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let registry = self.player_registry().cloned();
        let mut stale_looters = Vec::new();

        for looter in &players_looting {
            if Some(*looter) == current_player {
                self.send_packet(&packet);
                continue;
            }

            let Some(registry) = registry.as_ref() else {
                stale_looters.push(*looter);
                continue;
            };
            let Some(registration) =
                registry.loot_delivery_recipient(*looter, current_map, current_instance)
            else {
                stale_looters.push(*looter);
                continue;
            };
            if registry
                .send_current_packet(registration, bytes.clone())
                .is_err()
            {
                stale_looters.push(*looter);
            }
        }

        if !stale_looters.is_empty()
            && let Some(loot) = self.loot_table.get_mut(&owner_guid)
        {
            loot.players_looting
                .retain(|looter| !stale_looters.contains(looter));
        }
        if !stale_looters.is_empty()
            && let Some(authority) = self.represented_owned_loot_authority_like_cpp(owner_guid)
        {
            for looter in stale_looters {
                authority.remove_viewer_like_cpp(looter);
            }
            if let Some(player_guid) = current_player {
                let _ = self.reconcile_represented_loot_cache_like_cpp(owner_guid, player_guid);
            }
        }
    }

    fn represented_start_group_loot_rolls_on_first_open_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) {
        let Some(authority) = self.represented_owned_loot_authority_like_cpp(owner_guid) else {
            return;
        };
        let Some(authority_snapshot) = authority.snapshot_for_player_like_cpp(player_guid) else {
            return;
        };
        let authority_generation = authority_snapshot.generation;
        let authority_scope = authority_snapshot.scope;
        let current_map_id = self.player_map_id_like_cpp();
        let current_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let player_registry = self.player_registry().cloned();
        let mut packets = Vec::new();
        let mut auto_pass_packets = Vec::new();
        let mut pending_rolls = Vec::new();
        let mut unblocked_without_roll = Vec::new();
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
                    current_instance_id,
                    player_registry.as_deref(),
                );
                if eligible_looters.len() <= 1 {
                    entry.flags.under_threshold = true;
                    entry.flags.blocked = false;
                    unblocked_without_roll.push(entry.loot_list_id);
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
                        match player_registry.as_deref().and_then(|registry| {
                            registry.loot_pass_on_group_loot(
                                *looter,
                                current_map_id,
                                current_instance_id,
                            )
                        }) {
                            Some(pass_on_group_loot) => {
                                if pass_on_group_loot {
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
                let command_identity = LootRollCommandIdentityLikeCpp::new_like_cpp(
                    loot.loot_guid,
                    entry.loot_list_id,
                    authority.clone(),
                    authority_generation,
                );
                let state = RepresentedLootRollState {
                    owner_guid,
                    loot_obj: loot.loot_guid,
                    loot_list_id: entry.loot_list_id,
                    authority: authority.clone(),
                    authority_generation,
                    authority_scope,
                    command_identity,
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

        if !unblocked_without_roll.is_empty()
            && let Some(authority) = self.represented_owned_loot_authority_like_cpp(owner_guid)
        {
            for loot_list_id in unblocked_without_roll {
                let _ = authority.finish_item_roll_like_cpp(
                    player_guid,
                    authority_generation,
                    loot_list_id,
                    true,
                    None,
                );
            }
            let _ = self.reconcile_represented_loot_cache_like_cpp(owner_guid, player_guid);
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
            let Some(registration) = registry.loot_delivery_recipient(
                looter,
                self.player_map_id_like_cpp(),
                current_instance_id,
            ) else {
                continue;
            };

            let _ = registry.send_current_packet(registration, packet.to_bytes());
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
        let identities = self
            .represented_loot_rolls
            .values()
            .map(|state| state.command_identity.clone())
            .collect();
        let _ = registry.replace_loot_rolls_for_control_channel(
            player_guid,
            &self.session_command_tx(),
            identities,
        );
    }

    pub(crate) async fn tick_represented_loot_rolls_like_cpp(&mut self) {
        let now = Instant::now();
        let roll_keys: Vec<(ObjectGuid, u8)> =
            self.represented_loot_rolls.keys().copied().collect();

        for (loot_obj, loot_list_id) in roll_keys {
            let Some(state) = self
                .represented_loot_rolls
                .get(&(loot_obj, loot_list_id))
                .cloned()
            else {
                continue;
            };
            if self
                .represented_current_loot_roll_authority_like_cpp(&state)
                .is_none()
            {
                self.cancel_represented_loot_roll_generation_mismatch_like_cpp(
                    (loot_obj, loot_list_id),
                    &state,
                );
                continue;
            }
            if state.end_time > now {
                continue;
            }

            let owner_guid = state.owner_guid;
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

    /// Install kill-time pools only while the exact creature death lifetime
    /// observed before async template generation is still current. C++ runs
    /// `Unit::Kill` and loot creation on one map thread; this lock-scoped CAS
    /// is the Rust equivalent and prevents corpse-removal/respawn ABA.
    fn install_represented_creature_kill_loot_if_current_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
        expected_authority: &OwnedLootAuthority,
        expected_object_generation: u64,
        expected_loot_lifecycle_revision: u64,
        shared: Option<CreatureLoot>,
        personal: HashMap<ObjectGuid, CreatureLoot>,
    ) -> bool {
        let expected_authority = expected_authority.clone();
        self.mutate_world_creature(creature_guid, move |world_creature| {
            if world_creature.is_alive()
                || world_creature.creature.loot_lifecycle_revision_like_cpp()
                    != expected_loot_lifecycle_revision
                || !world_creature
                    .creature
                    .loot_authority_like_cpp()
                    .shares_storage_like_cpp(&expected_authority)
                || !expected_authority.is_retired_like_cpp()
                || expected_authority.generation_like_cpp() != expected_object_generation
            {
                return false;
            }

            let installed = if expected_object_generation == 0 {
                expected_authority
                    .initialize_pristine_like_cpp(shared, personal)
                    .installed()
            } else {
                expected_authority
                    .replace_retired_generation_like_cpp(
                        expected_object_generation,
                        shared,
                        personal,
                    )
                    .is_some()
            };
            if installed {
                world_creature
                    .creature
                    .sync_loot_summaries_from_authority_like_cpp();
            }
            installed
        })
        .unwrap_or(false)
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
        allowed_looters: &[ObjectGuid],
        expected_loot_lifecycle_revision: u64,
    ) {
        let authority = self.represented_owned_loot_authority_like_cpp(creature_guid);
        if authority.is_none() && !represented_local_loot_fixture_allowed_like_cpp() {
            self.loot_table.remove(&creature_guid);
            return;
        }
        let mut retired_object_generation = None;
        if let Some(authority) = authority.as_ref() {
            #[cfg(test)]
            if authority.is_pristine_like_cpp() && self.loot_table.contains_key(&creature_guid) {
                if !self
                    .represented_personal_loot_owners
                    .contains(&creature_guid)
                    && let Some(loot) = self.loot_table.get_mut(&creature_guid)
                {
                    prepare_represented_shared_creature_loot_generation_like_cpp(
                        loot,
                        allowed_looters,
                    );
                }
                if self
                    .sync_represented_creature_loot_to_canonical_like_cpp(
                        creature_guid,
                        loot_owner_guid,
                    )
                    .is_some()
                {
                    // Legacy packet fixtures pre-populate the former session
                    // cache. Install that value once into the typed object-owned
                    // authority instead of silently replacing it with generated
                    // empty loot. This branch does not exist in production.
                    return;
                }
            }
            let snapshot = self
                .player_guid()
                .and_then(|player_guid| authority.snapshot_for_player_like_cpp(player_guid))
                .or_else(|| authority.snapshot_for_player_like_cpp(loot_owner_guid));
            if let Some(snapshot) = snapshot {
                let cache_player = match snapshot.scope {
                    OwnedLootScope::Personal(player_guid) => player_guid,
                    OwnedLootScope::Shared => loot_owner_guid,
                };
                self.cache_represented_owned_loot_snapshot_like_cpp(
                    creature_guid,
                    cache_player,
                    snapshot,
                );
                return;
            }
            if !authority.is_retired_like_cpp() {
                self.loot_table.remove(&creature_guid);
                return;
            }
            retired_object_generation = Some(authority.generation_like_cpp());
            self.loot_table.remove(&creature_guid);
            self.represented_loot_cache_generations_like_cpp
                .remove(&creature_guid);
        }

        let map_is_dungeon = self.current_map_dungeon_state_like_cpp();
        let connected_tappers =
            self.represented_connected_creature_tappers_like_cpp(allowed_looters);

        // C++ `Unit::Kill` has three distinct ownership shapes:
        // - overworld: one independently generated personal pool per tapper;
        // - dungeon encounter/boss: one independent, lockout-filtered pool per
        //   tapper (`GenerateDungeonEncounterPersonalLoot`);
        // - dungeon trash: exactly one personal pool, keyed by the group's
        //   selected looter (or the first tapper without a group).
        if map_is_dungeon == Some(false)
            || (map_is_dungeon == Some(true) && dungeon_encounter_id != 0)
        {
            let personal_tappers = connected_tappers
                .into_iter()
                .filter(|tapper| {
                    dungeon_encounter_id == 0
                        || !self
                            .represented_locked_dungeon_encounters
                            .contains(&(*tapper, dungeon_encounter_id))
                })
                .collect::<Vec<_>>();
            if personal_tappers.is_empty() {
                self.loot_table.remove(&creature_guid);
                return;
            }

            let Some(personal) = self
                .generate_represented_creature_personal_loot_like_cpp(
                    creature_guid,
                    level,
                    entry,
                    loot_id,
                    gold_min,
                    gold_max,
                    dungeon_encounter_id,
                    &personal_tappers,
                )
                .await
            else {
                return;
            };
            let cache_player = self
                .player_guid()
                .filter(|player_guid| personal.contains_key(player_guid))
                .unwrap_or(personal_tappers[0]);

            if let (Some(authority), Some(expected_generation)) =
                (authority.as_ref(), retired_object_generation)
            {
                if self.install_represented_creature_kill_loot_if_current_like_cpp(
                    creature_guid,
                    authority,
                    expected_generation,
                    expected_loot_lifecycle_revision,
                    None,
                    personal,
                ) {
                    let _ =
                        self.reconcile_represented_loot_cache_like_cpp(creature_guid, cache_player);
                } else {
                    self.loot_table.remove(&creature_guid);
                }
            } else if represented_local_loot_fixture_allowed_like_cpp()
                && let Some(pool) = personal.get(&cache_player).cloned()
            {
                self.loot_table.insert(creature_guid, pool);
            }
            return;
        }

        if map_is_dungeon == Some(true) {
            if connected_tappers.is_empty() {
                self.loot_table.remove(&creature_guid);
                return;
            }
            let selected_looter =
                self.represented_dungeon_trash_looter_like_cpp(&connected_tappers);
            let Some(personal) = self
                .generate_represented_creature_personal_loot_like_cpp(
                    creature_guid,
                    level,
                    entry,
                    loot_id,
                    gold_min,
                    gold_max,
                    0,
                    &[selected_looter],
                )
                .await
            else {
                return;
            };
            let has_loot = personal
                .get(&selected_looter)
                .is_some_and(|loot| !loot_is_looted_like_cpp(loot));

            if let (Some(authority), Some(expected_generation)) =
                (authority.as_ref(), retired_object_generation)
            {
                if self.install_represented_creature_kill_loot_if_current_like_cpp(
                    creature_guid,
                    authority,
                    expected_generation,
                    expected_loot_lifecycle_revision,
                    None,
                    personal,
                ) {
                    let _ = self
                        .reconcile_represented_loot_cache_like_cpp(creature_guid, selected_looter);
                    if has_loot {
                        self.advance_represented_dungeon_trash_looter_like_cpp(&connected_tappers);
                    }
                } else {
                    self.loot_table.remove(&creature_guid);
                }
            } else if represented_local_loot_fixture_allowed_like_cpp()
                && let Some(pool) = personal.get(&selected_looter).cloned()
            {
                self.loot_table.insert(creature_guid, pool);
            }
            return;
        }

        // Missing Map.db2 metadata is not proof of either overworld or
        // dungeon. Preserve the represented shared fallback for legacy test
        // fixtures, but still bind its async install to the exact death token.
        if !self.loot_table.contains_key(&creature_guid) {
            let Some(mut loot) = self
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
                .await
            else {
                return;
            };
            prepare_represented_shared_creature_loot_generation_like_cpp(
                &mut loot,
                allowed_looters,
            );
            if let (Some(authority), Some(expected_generation)) =
                (authority.as_ref(), retired_object_generation)
            {
                if self.install_represented_creature_kill_loot_if_current_like_cpp(
                    creature_guid,
                    authority,
                    expected_generation,
                    expected_loot_lifecycle_revision,
                    Some(loot),
                    HashMap::new(),
                ) {
                    let _ = self
                        .reconcile_represented_loot_cache_like_cpp(creature_guid, loot_owner_guid);
                } else {
                    self.loot_table.remove(&creature_guid);
                }
            } else if represented_local_loot_fixture_allowed_like_cpp() {
                self.loot_table.insert(creature_guid, loot);
            }
        }
    }

    async fn ensure_represented_gameobject_chest_loot_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        source: GameObjectLootSource,
        allowed_looters: &[ObjectGuid],
    ) {
        // C++ creates `m_loot` synchronously in `GameObject::Use`
        // (`GameObject.cpp:2559-2575`). Capture the exact map-owned lifetime
        // before async template work, then revalidate it under the map lock at
        // install time so `ClearLoot`/restock cannot be crossed.
        let install_observation = match self
            .represented_gameobject_loot_install_observation_result_like_cpp(gameobject_guid)
        {
            Some(Some(observation)) => Some(observation),
            Some(None) => {
                self.loot_table.remove(&gameobject_guid);
                self.represented_loot_cache_generations_like_cpp
                    .remove(&gameobject_guid);
                return;
            }
            None if !represented_local_loot_fixture_allowed_like_cpp() => {
                self.loot_table.remove(&gameobject_guid);
                self.represented_loot_cache_generations_like_cpp
                    .remove(&gameobject_guid);
                return;
            }
            None => None,
        };
        let authority = install_observation
            .as_ref()
            .map(|observation| observation.authority.clone());
        let mut install_single_personal_pool = false;
        if let Some(authority) = authority.as_ref() {
            #[cfg(test)]
            if authority.is_pristine_like_cpp() && self.loot_table.contains_key(&gameobject_guid) {
                if !self
                    .represented_personal_loot_owners
                    .contains(&gameobject_guid)
                    && let Some(loot) = self.loot_table.get_mut(&gameobject_guid)
                {
                    prepare_represented_shared_loot_generation_like_cpp(loot, allowed_looters);
                }
                if self
                    .sync_represented_gameobject_loot_to_canonical_like_cpp(
                        gameobject_guid,
                        player_guid,
                    )
                    .is_some()
                {
                    // Test-only bridge for legacy pre-authority packet fixtures;
                    // live gameobjects still require their canonical map owner.
                    return;
                }
            }
            if let Some(snapshot) = authority.snapshot_for_player_like_cpp(player_guid) {
                self.cache_represented_owned_loot_snapshot_like_cpp(
                    gameobject_guid,
                    player_guid,
                    snapshot,
                );
                return;
            }
            let active_authority =
                authority.stamp_like_cpp().lifecycle == OwnedLootAuthorityLifecycle::Active;
            let can_add_personal_pool = active_authority
                && source.uses_personal_loot_like_cpp()
                && !source.is_personal_encounter_loot_like_cpp();
            if can_add_personal_pool {
                // The non-encounter C++ branch adds exactly this opener's
                // `m_personalLoot[player]` pool. Encounter loot is different:
                // it regenerates one topology from GameObject::GetTapList and
                // assigns the whole map. Rust does not yet have that canonical
                // script-owned tap list, so an encounter opener absent from
                // the installed topology must fail closed rather than receive
                // a fabricated singleton pool.
                install_single_personal_pool = true;
            }
            if !authority.is_retired_like_cpp() && !can_add_personal_pool {
                self.loot_table.remove(&gameobject_guid);
                self.represented_loot_cache_generations_like_cpp
                    .remove(&gameobject_guid);
                return;
            }
            self.loot_table.remove(&gameobject_guid);
            self.represented_loot_cache_generations_like_cpp
                .remove(&gameobject_guid);
        }

        if !self.loot_table.contains_key(&gameobject_guid) {
            let single_personal_looter = install_single_personal_pool.then_some([player_guid]);
            let generation_allowed_looters = single_personal_looter
                .as_ref()
                .map_or(allowed_looters, |looters| looters.as_slice());
            let Some(mut loot) = self
                .generate_represented_gameobject_chest_loot_like_cpp(
                    gameobject_guid,
                    player_guid,
                    source,
                    generation_allowed_looters,
                )
                .await
            else {
                return;
            };
            let personal = self
                .represented_personal_loot_owners
                .contains(&gameobject_guid);
            if !personal {
                prepare_represented_shared_loot_generation_like_cpp(&mut loot, allowed_looters);
            }
            if let Some(observation) = install_observation {
                if source.uses_personal_loot_like_cpp()
                    && (!source.is_personal_encounter_loot_like_cpp()
                        || install_single_personal_pool)
                {
                    if self
                        .upsert_represented_personal_gameobject_loot_authority_if_observed_with_empty_policy_like_cpp(
                            gameobject_guid,
                            player_guid,
                            loot,
                            false,
                            source.is_personal_encounter_loot_like_cpp(),
                            &observation,
                        )
                        .is_none()
                    {
                        self.loot_table.remove(&gameobject_guid);
                        self.represented_loot_cache_generations_like_cpp
                            .remove(&gameobject_guid);
                    }
                    return;
                }
                let Some((shared, mut personal)) = self.represented_loot_authority_pools_like_cpp(
                    gameobject_guid,
                    player_guid,
                    loot,
                    personal,
                ) else {
                    self.loot_table.remove(&gameobject_guid);
                    self.represented_loot_cache_generations_like_cpp
                        .remove(&gameobject_guid);
                    return;
                };
                if source.is_personal_encounter_loot_like_cpp() {
                    // `GenerateDungeonEncounterPersonalLoot` drops each
                    // per-player `Loot` that is already empty after money,
                    // personal-template and not-normal processing
                    // (`LootMgr.cpp:933-941`).  The non-encounter
                    // `chestPersonalLoot` branch deliberately keeps its empty
                    // `m_personalLoot[player]`, so this filter belongs only to
                    // the encounter topology.
                    personal.retain(|_, pool| !loot_is_looted_like_cpp(pool));
                    self.represented_personal_loot_money
                        .retain(|(owner, player), _| {
                            *owner != gameobject_guid || personal.contains_key(player)
                        });
                    if personal.is_empty() {
                        // C++ assigns an empty `m_personalLoot` map and sends no
                        // loot window.  Keep the authority pristine/retired so
                        // a later `Use` may generate again instead of leaving
                        // an active owner with no selectable pool.
                        self.represented_personal_loot_owners
                            .remove(&gameobject_guid);
                        self.loot_table.remove(&gameobject_guid);
                        self.represented_loot_cache_generations_like_cpp
                            .remove(&gameobject_guid);
                        return;
                    }
                }
                let installed = self
                    .mutate_canonical_gameobject_by_guid_like_cpp(
                        gameobject_guid,
                        move |gameobject| {
                            gameobject.install_loot_authority_if_lifecycle_like_cpp(
                                &observation.authority,
                                observation.object_generation,
                                observation.loot_lifecycle_revision,
                                shared,
                                personal,
                            )
                        },
                    )
                    .unwrap_or(false);
                if !installed {
                    self.loot_table.remove(&gameobject_guid);
                    self.represented_loot_cache_generations_like_cpp
                        .remove(&gameobject_guid);
                    return;
                }
                let _ =
                    self.reconcile_represented_loot_cache_like_cpp(gameobject_guid, player_guid);
            } else if represented_local_loot_fixture_allowed_like_cpp() {
                self.loot_table.insert(gameobject_guid, loot);
            }
        }
    }

    async fn generate_represented_gameobject_chest_loot_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        source: GameObjectLootSource,
        allowed_looters: &[ObjectGuid],
    ) -> Option<CreatureLoot> {
        let personal_loot = source.uses_personal_loot_like_cpp();
        let personal_encounter = source.is_personal_encounter_loot_like_cpp();
        let (loot_method, loot_master, round_robin_player) = self
            .represented_gameobject_chest_group_state_like_cpp(
                source.use_group_loot_rules && !personal_loot,
                player_guid,
            );
        let loot_id = source.open_loot_id_like_cpp();
        let items = if personal_encounter {
            Vec::new()
        } else {
            self.generate_represented_shared_gameobject_loot_items_like_cpp(
                loot_id,
                allowed_looters,
            )
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
        let coins = self.represented_money_loot_with_rate_like_cpp(
            min_money,
            max_money,
            self.loot_drop_rates_like_cpp().money,
        );

        let loot_guid = self.next_represented_loot_object_guid_like_cpp(gameobject_guid)?;
        let mut loot = CreatureLoot {
            loot_guid,
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

        if personal_loot {
            loot.coins = 0;
            self.represented_personal_loot_owners
                .insert(gameobject_guid);
            self.represented_personal_loot_money
                .retain(|(owner, _), _| *owner != gameobject_guid);
            let represented_tappers = if personal_encounter && !allowed_looters.is_empty() {
                let mut tappers = allowed_looters
                    .iter()
                    .copied()
                    .filter(|guid| {
                        guid.is_player()
                            && self.represented_player_is_unlocked_for_dungeon_encounter_like_cpp(
                                *guid,
                                source.dungeon_encounter_id,
                            )
                    })
                    .collect::<Vec<_>>();
                tappers.sort_unstable_by_key(|guid| (guid.high_value(), guid.low_value()));
                tappers.dedup();
                tappers
            } else if personal_encounter {
                self.represented_gameobject_personal_encounter_tappers_like_cpp(
                    gameobject_guid,
                    player_guid,
                    source.dungeon_encounter_id,
                )
            } else {
                vec![player_guid]
            };
            for tapper in &represented_tappers {
                if !loot.allowed_looters.contains(tapper) {
                    loot.allowed_looters.push(*tapper);
                }
                let tapper_money = self.represented_money_loot_with_rate_like_cpp(
                    min_money,
                    max_money,
                    self.loot_drop_rates_like_cpp().money,
                );
                self.represented_personal_loot_money
                    .insert((gameobject_guid, *tapper), tapper_money);
            }
            if personal_encounter {
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
            }
            rebuild_represented_personal_loot_counts_like_cpp(&mut loot);
            if represented_tappers.is_empty() {
                self.represented_personal_loot_owners
                    .remove(&gameobject_guid);
            }
        }

        Some(loot)
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
        _player_guid: ObjectGuid,
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

        // C++ `Loot::FillLoot` assigns round robin only for `LOOT_CORPSE`.
        (
            group.loot_method,
            group.master_looter_guid,
            ObjectGuid::EMPTY,
        )
    }

    /// C++ `Loot::FillLoot` calls `FillNotNormalLootFor` for every connected
    /// group member at reward distance from the opening player before the
    /// chest's shared `Loot` becomes visible.
    fn represented_group_looters_at_reward_distance_like_cpp(
        &self,
        player_guid: ObjectGuid,
    ) -> Vec<ObjectGuid> {
        let Some(group_guid) = self.group_guid else {
            return vec![player_guid];
        };
        let Some(group_registry) = self.group_registry() else {
            return vec![player_guid];
        };
        let Some(group) = group_registry.get(&group_guid) else {
            return vec![player_guid];
        };
        let source_position = self.player_position_like_cpp().unwrap_or_default();
        let map_id = self.player_map_id_like_cpp();
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let registry = self.player_registry();
        let mut looters = Vec::new();

        for member_guid in &group.members {
            if *member_guid == player_guid {
                looters.push(*member_guid);
                continue;
            }
            let Some(member) = registry.and_then(|registry| registry.loot_presence(*member_guid))
            else {
                continue;
            };
            if member.is_in_world
                && member.map_id == map_id
                && member.instance_id == instance_id
                && (self.current_map_is_dungeon_like_cpp()
                    || source_position.is_within_dist(&member.position, 74.0))
            {
                looters.push(*member_guid);
            }
        }

        if looters.is_empty() {
            looters.push(player_guid);
        }
        looters.sort_unstable_by_key(|guid| (guid.high_value(), guid.low_value()));
        looters.dedup();
        looters
    }

    async fn generate_represented_gameobject_loot_items_like_cpp(
        &mut self,
        loot_id: u32,
    ) -> Option<Vec<LootEntry>> {
        self.generate_represented_gameobject_loot_items_for_store_like_cpp(
            loot_id,
            LootStoreKind::Gameobject,
            LOOT_MODE_DEFAULT_LIKE_CPP,
            None,
        )
        .await
    }

    async fn generate_represented_shared_gameobject_loot_items_like_cpp(
        &mut self,
        loot_id: u32,
        allowed_looters: &[ObjectGuid],
    ) -> Option<Vec<LootEntry>> {
        self.generate_represented_gameobject_loot_items_for_store_like_cpp(
            loot_id,
            LootStoreKind::Gameobject,
            LOOT_MODE_DEFAULT_LIKE_CPP,
            Some(allowed_looters),
        )
        .await
    }

    async fn generate_represented_gameobject_loot_items_for_store_like_cpp(
        &mut self,
        loot_id: u32,
        store_kind: LootStoreKind,
        loot_mode: u16,
        shared_allowed_looters: Option<&[ObjectGuid]>,
    ) -> Option<Vec<LootEntry>> {
        if loot_id == 0 {
            return Some(Vec::new());
        }

        let mut rng = self.represented_runtime_subrng_like_cpp();
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
        let defer_eligibility_until_after_roll = shared_allowed_looters.is_some();
        let generated = {
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
                    defer_eligibility_until_after_roll
                        || self.represented_creature_loot_item_allowed_like_cpp(
                            context,
                            &condition_rows,
                            &condition_references,
                            &addon_metadata,
                        )
                },
                |item_id, rng| {
                    let random_properties =
                        self.generate_loot_store_random_properties_with_rng_like_cpp(item_id, rng);
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
                    if let Some(allowed_looters) = shared_allowed_looters {
                        generated_shared_gameobject_loot_item_to_entry_like_cpp(
                            item,
                            metadata,
                            allowed_looters,
                            |context, looter| {
                                self.represented_creature_loot_item_allowed_for_player_like_cpp(
                                    context,
                                    looter,
                                    &condition_rows,
                                    &condition_references,
                                    &addon_metadata,
                                )
                            },
                        )
                    } else {
                        generated_creature_loot_item_to_entry_like_cpp(item, metadata)
                    }
                })
                .collect(),
        )
    }

    async fn generate_represented_fishing_loot_items_like_cpp(
        &mut self,
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
                    None,
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
            None,
        )
        .await
    }

    async fn generate_represented_gameobject_personal_loot_items_like_cpp(
        &mut self,
        loot_id: u32,
        tappers: &[ObjectGuid],
    ) -> Option<Vec<LootEntry>> {
        if loot_id == 0 || tappers.is_empty() {
            return Some(Vec::new());
        }

        let mut rng = self.represented_runtime_subrng_like_cpp();
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
                    |item_id, rng| {
                        let random_properties = self
                            .generate_loot_store_random_properties_with_rng_like_cpp(item_id, rng);
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
                match (result.try_read::<u32>(0), result.try_read::<u32>(1)) {
                    (Some(min_money), Some(max_money)) => (min_money, max_money),
                    _ => {
                        warn!(
                            gameobject_entry,
                            "failed to decode gameobject_template_addon money loot as C++ uint32 columns"
                        );
                        (0, 0)
                    }
                }
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
        &mut self,
        creature_guid: ObjectGuid,
        loot_owner_guid: ObjectGuid,
        _level: u8,
        entry: u32,
        loot_id: u32,
        gold_min: u32,
        gold_max: u32,
        dungeon_encounter_id: u32,
    ) -> Option<CreatureLoot> {
        let (loot_method, loot_master, round_robin_player) =
            self.represented_creature_loot_group_state_like_cpp(loot_owner_guid);
        let coins = self.represented_money_loot_with_rate_like_cpp(
            gold_min,
            gold_max,
            self.loot_drop_rates_like_cpp().money,
        );

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

        let loot_guid = self.next_represented_loot_object_guid_like_cpp(creature_guid)?;
        Some(CreatureLoot {
            loot_guid,
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
        })
    }

    /// C++ `Unit::Kill` first resolves every tap-list GUID through
    /// `ObjectAccessor::GetPlayer(*creature, guid)`. Only connected players in
    /// the creature's exact map instance receive an overworld personal pool.
    fn represented_connected_creature_tappers_like_cpp(
        &self,
        tappers: &[ObjectGuid],
    ) -> Vec<ObjectGuid> {
        let current_player = self.player_guid();
        let map_id = self.player_map_id_like_cpp();
        let instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let registry = self.player_registry();
        let mut connected = tappers
            .iter()
            .copied()
            .filter(|tapper| {
                if !tapper.is_player() {
                    return false;
                }
                if Some(*tapper) == current_player {
                    return true;
                }
                registry
                    .and_then(|registry| registry.loot_presence(*tapper))
                    .is_some_and(|player| {
                        player.is_in_world
                            && player.map_id == map_id
                            && player.instance_id == instance_id
                    })
            })
            .collect::<Vec<_>>();
        connected.sort_unstable_by_key(|guid| (guid.high_value(), guid.low_value()));
        connected.dedup();
        connected
    }

    fn represented_dungeon_trash_looter_like_cpp(
        &self,
        connected_tappers: &[ObjectGuid],
    ) -> ObjectGuid {
        let selected =
            if let (Some(group_guid), Some(registry)) = (self.group_guid, self.group_registry()) {
                registry
                    .get(&group_guid)
                    .map(|group| group.looter_guid_like_cpp())
                    .filter(|looter| connected_tappers.contains(looter))
            } else {
                None
            };
        selected.unwrap_or(connected_tappers[0])
    }

    fn advance_represented_dungeon_trash_looter_like_cpp(&self, connected_tappers: &[ObjectGuid]) {
        let (Some(group_guid), Some(registry)) = (self.group_guid, self.group_registry()) else {
            return;
        };
        let _ = registry
            .advance_looter_transition_like_cpp(group_guid, connected_tappers.iter().copied());
    }

    /// Generate one independently rolled C++ personal `Loot` per supplied
    /// player. The caller chooses the ownership set for overworld tappers,
    /// encounter-eligible dungeon tappers, or the single dungeon-trash
    /// selected looter. Every pool is constructed without a Group and remains
    /// an object-owned per-view source of truth.
    #[allow(clippy::too_many_arguments)]
    async fn generate_represented_creature_personal_loot_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
        _level: u8,
        entry: u32,
        loot_id: u32,
        gold_min: u32,
        gold_max: u32,
        dungeon_encounter_id: u32,
        tappers: &[ObjectGuid],
    ) -> Option<HashMap<ObjectGuid, CreatureLoot>> {
        let mut personal = HashMap::with_capacity(tappers.len());
        for tapper in tappers {
            let coins = self.represented_money_loot_with_rate_like_cpp(
                gold_min,
                gold_max,
                self.loot_drop_rates_like_cpp().money,
            );
            let items = self
                .generate_represented_creature_loot_items_for_player_like_cpp(loot_id, *tapper)
                .await
                .unwrap_or_else(|| {
                    if loot_id != 0 {
                        debug!(
                            entry,
                            loot_id,
                            tapper = ?tapper,
                            "creature personal loot template unavailable for represented overworld corpse"
                        );
                    }
                    Vec::new()
                });
            let loot_guid = self.next_represented_loot_object_guid_like_cpp(creature_guid)?;
            let mut loot = CreatureLoot {
                loot_guid,
                coins,
                unlooted_count: 0,
                loot_type: LOOT_TYPE_CORPSE_LIKE_CPP,
                dungeon_encounter_id,
                loot_method: 0,
                loot_master: ObjectGuid::EMPTY,
                round_robin_player: ObjectGuid::EMPTY,
                player_ffa_items: Vec::new(),
                players_looting: Vec::new(),
                allowed_looters: Vec::new(),
                items,
                looted_by_player: false,
            };
            mark_loot_allowed_for_player_like_cpp(&mut loot, *tapper);
            rebuild_represented_personal_loot_counts_preserving_consumed_like_cpp(&mut loot);
            personal.insert(*tapper, loot);
        }
        Some(personal)
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
        &mut self,
        loot_id: u32,
    ) -> Option<Vec<LootEntry>> {
        let player_guid = self.player_guid().unwrap_or(ObjectGuid::EMPTY);
        self.generate_represented_creature_loot_items_for_player_like_cpp(loot_id, player_guid)
            .await
    }

    async fn generate_represented_creature_loot_items_for_player_like_cpp(
        &mut self,
        loot_id: u32,
        player_guid: ObjectGuid,
    ) -> Option<Vec<LootEntry>> {
        if loot_id == 0 {
            return Some(Vec::new());
        }

        let mut rng = self.represented_runtime_subrng_like_cpp();
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
                        self.represented_creature_loot_item_allowed_for_player_like_cpp(
                            context,
                            player_guid,
                            &condition_rows,
                            &condition_references,
                            &addon_metadata,
                        )
                    },
                    |item_id, rng| {
                        let random_properties = self
                            .generate_loot_store_random_properties_with_rng_like_cpp(item_id, rng);
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

        let player = self.player_registry()?.loot_player_context(player_guid)?;
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
            if status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
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
            if status.status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
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
                if player_context.quest_status(*quest_id) != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
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
                if *status != QUEST_STATUS_INCOMPLETE_LIKE_CPP {
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
            9 => Some(
                player_context.quest_status(condition.value1) == QUEST_STATUS_INCOMPLETE_LIKE_CPP,
            ),
            14 => Some(player_context.quest_status(condition.value1) == QUEST_STATUS_NONE_LIKE_CPP),
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
                player_context.quest_status(condition.value1) == QUEST_STATUS_COMPLETE_LIKE_CPP
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
        let map_key = self
            .canonical_object_lookup_map_key_like_cpp(u32::from(self.player_map_id_like_cpp()))?;
        let manager = self.canonical_map_manager.as_ref()?;
        let manager = manager.lock().ok()?;
        let map = manager.find_map(map_key.map_id, map_key.instance_id)?.map();
        map.map_object_by_kind(guid, allowed)
            .map(|object| object.position())
    }

    fn canonical_gameobject_owner_for_loot_like_cpp(&self, guid: ObjectGuid) -> Option<ObjectGuid> {
        let map_key = self
            .canonical_object_lookup_map_key_like_cpp(u32::from(self.player_map_id_like_cpp()))?;
        let manager = self.canonical_map_manager.as_ref()?;
        let manager = manager.lock().ok()?;
        let map = manager.find_map(map_key.map_id, map_key.instance_id)?.map();
        let owner_guid = map.get_typed_game_object(guid)?.owner_guid();
        (!owner_guid.is_empty()).then_some(owner_guid)
    }

    fn remove_canonical_corpse_lootable_dynamic_flag_like_cpp(
        &mut self,
        corpse_guid: ObjectGuid,
    ) -> bool {
        let Some(map_key) =
            self.canonical_object_lookup_map_key_like_cpp(u32::from(self.player_map_id_like_cpp()))
        else {
            return false;
        };
        let Some(manager) = self.canonical_map_manager.as_ref().cloned() else {
            return false;
        };
        let Ok(mut manager) = manager.lock() else {
            return false;
        };
        let Some(map) = manager.find_map_mut(map_key.map_id, map_key.instance_id) else {
            return false;
        };
        let Some(corpse) = map.map_mut().get_typed_corpse_mut(corpse_guid) else {
            return false;
        };

        corpse.remove_corpse_dynamic_flag(CORPSE_DYNFLAG_LOOTABLE);
        true
    }

    fn remove_canonical_corpse_lootable_dynamic_flag_if_unviewed_fully_looted_observation_like_cpp(
        &mut self,
        corpse_guid: ObjectGuid,
        authority: &OwnedLootAuthority,
        object_generation: u64,
        lifecycle_revision: u64,
    ) -> bool {
        let Some(map_key) =
            self.canonical_object_lookup_map_key_like_cpp(u32::from(self.player_map_id_like_cpp()))
        else {
            return false;
        };
        let Some(manager) = self.canonical_map_manager.as_ref().cloned() else {
            return false;
        };
        let Ok(mut manager) = manager.lock() else {
            return false;
        };
        let Some(map) = manager.find_map_mut(map_key.map_id, map_key.instance_id) else {
            return false;
        };
        let Some(corpse) = map.map_mut().get_typed_corpse_mut(corpse_guid) else {
            return false;
        };

        authority
            .with_unviewed_fully_looted_lifecycle_observation_like_cpp(
                object_generation,
                lifecycle_revision,
                || corpse.remove_corpse_dynamic_flag(CORPSE_DYNFLAG_LOOTABLE),
            )
            .is_some()
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
            loot_lifecycle_revision: creature.creature.loot_lifecycle_revision_like_cpp(),
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
        selected_pool_looted: bool,
        mut whole_object_fully_looted: bool,
        authoritative_release: Option<&AuthoritativeLootReleaseLikeCpp>,
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
        let canonical_fishing_hole_release = (go_type == Some(GAMEOBJECT_TYPE_FISHING_HOLE))
            .then(|| {
                self.release_canonical_fishing_hole_like_cpp(
                    guid,
                    represented_fishing_hole_max_opens,
                )
            })
            .flatten();
        let canonical_fishing_hole_use_count_after_release = canonical_fishing_hole_release
            .as_ref()
            .map(|(use_count, _, _)| *use_count);

        let guarded_global_transition_attempted = selected_pool_looted
            && whole_object_fully_looted
            && !matches!(
                go_type,
                Some(GAMEOBJECT_TYPE_FISHING_NODE)
                    | Some(GAMEOBJECT_TYPE_FISHING_HOLE)
                    | Some(GAMEOBJECT_TYPE_GATHERING_NODE)
            )
            && authoritative_release.is_some();
        let guarded_global_transition = authoritative_release
            .filter(|_| guarded_global_transition_attempted)
            .and_then(|release| {
                if release.require_no_viewers {
                    self.set_canonical_gameobject_loot_state_if_unviewed_fully_looted_observation_like_cpp(
                        guid,
                        &release.authority,
                        release.object_generation,
                        release.lifecycle_revision,
                        LootState::JustDeactivated,
                        None,
                        represented_chest_restock_time_secs,
                        false,
                    )
                } else {
                    self.set_canonical_gameobject_loot_state_if_fully_looted_observation_like_cpp(
                        guid,
                        &release.authority,
                        release.object_generation,
                        release.lifecycle_revision,
                        LootState::JustDeactivated,
                        None,
                        represented_chest_restock_time_secs,
                        false,
                    )
                }
            });
        if guarded_global_transition_attempted && guarded_global_transition.is_none() {
            // An upsert/install/replacement won the serialization point after
            // close. Its new pool must keep the object globally active.
            whole_object_fully_looted = false;
        }

        let canonical_loot_state_request = match go_type {
            Some(GAMEOBJECT_TYPE_FISHING_NODE) => Some((LootState::JustDeactivated, None, false)),
            Some(GAMEOBJECT_TYPE_FISHING_HOLE) if canonical_fishing_hole_release.is_some() => None,
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
            Some(GAMEOBJECT_TYPE_GATHERING_NODE) if selected_pool_looted => None,
            _ if guarded_global_transition_attempted => None,
            _ if selected_pool_looted && whole_object_fully_looted => {
                Some((LootState::JustDeactivated, None, false))
            }
            _ if selected_pool_looted => None,
            _ => Some((LootState::Activated, Some(player_guid), true)),
        };
        let requested_loot_state_outcome = canonical_loot_state_request.and_then(
            |(loot_state, unit_guid, shared_loot_is_changed_like_cpp)| {
                self.set_canonical_gameobject_loot_state_like_cpp(
                    guid,
                    loot_state,
                    unit_guid,
                    represented_chest_restock_time_secs,
                    shared_loot_is_changed_like_cpp,
                )
            },
        );
        let canonical_applied_loot_state = if guarded_global_transition.is_some() {
            Some((LootState::JustDeactivated, None))
        } else if let Some((_, state, _)) = canonical_fishing_hole_release.as_ref() {
            Some((*state, None))
        } else {
            canonical_loot_state_request.map(|(state, unit_guid, _)| (state, unit_guid))
        };
        let canonical_loot_state_updated = guarded_global_transition
            .as_ref()
            .or_else(|| {
                canonical_fishing_hole_release
                    .as_ref()
                    .map(|(_, _, outcome)| outcome)
            })
            .or(requested_loot_state_outcome.as_ref())
            .is_some_and(|outcome| {
                outcome.status == wow_map::map::GameObjectSetLootStateStatusLikeCpp::Updated
            });

        let state = self
            .represented_gameobject_use_states
            .entry(guid)
            .or_default();
        if canonical_loot_state_updated {
            if let Some((loot_state, unit_guid)) = canonical_applied_loot_state {
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
                Some(GAMEOBJECT_TYPE_GATHERING_NODE) if selected_pool_looted => {}
                Some(GAMEOBJECT_TYPE_CHEST)
                    if selected_pool_looted
                        && whole_object_fully_looted
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
                _ if selected_pool_looted && whole_object_fully_looted => {
                    state.loot_state = Some(LootState::JustDeactivated);
                    state.loot_state_unit_guid = ObjectGuid::EMPTY;
                }
                _ if selected_pool_looted => {}
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
        if go_type == Some(GAMEOBJECT_TYPE_GATHERING_NODE) && selected_pool_looted {
            state.go_state = Some(GoState::Active);
        }
        if go_type == Some(GAMEOBJECT_TYPE_CHEST)
            && selected_pool_looted
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
            threshold: LOOT_RESPONSE_DEFAULT_THRESHOLD_LIKE_CPP,
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

    /// Publishes successful durable loot transactions that outlived their
    /// packet waiter. This replays committed Item-owned money or item grants
    /// into runtime state before disconnect can persist a stale snapshot. C++
    /// auto-releases only when `Loot::isLooted()` (zero coins and no visible
    /// items) and the owner GUID is an Item.
    pub(crate) async fn apply_pending_durable_item_loot_completions_like_cpp(&mut self) {
        self.apply_pending_durable_item_loot_completions_with_objective_drain_like_cpp(true)
            .await;
    }

    pub(crate) async fn apply_pending_durable_item_loot_completions_with_objective_drain_like_cpp(
        &mut self,
        drain_money_objectives: bool,
    ) {
        let completions = self.take_durable_item_loot_completions_like_cpp();
        for completion in completions {
            if let Some(fanout) = completion.item_fanout.as_ref() {
                let _ = self.publish_durable_loot_item_fanout_like_cpp(fanout);
            }
            let requires_runtime_recovery =
                !completion.runtime_inventory_applied.load(Ordering::Acquire);
            let targets_current_player = self.player_guid() == Some(completion.player_guid);

            if let Some(applied_delta) = completion.durable_item_money_applied_amount {
                let apply_balance = targets_current_player
                    && completion
                        .durable_item_money_balance_applied
                        .as_ref()
                        .is_some_and(|applied| {
                            applied
                                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                                .is_ok()
                        });
                let publish = targets_current_player
                    && completion
                        .runtime_inventory_applied
                        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                        .is_ok();
                if apply_balance {
                    let old_money = self.player_gold_like_cpp();
                    let new_money = old_money
                        .checked_add(applied_delta)
                        .filter(|money| *money <= MAX_MONEY_AMOUNT)
                        .unwrap_or(old_money);
                    self.set_player_gold_like_cpp(new_money);
                    if old_money != new_money {
                        self.enqueue_represented_quest_objective_progress_like_cpp(
                            RepresentedQuestObjectiveProgressEventLikeCpp::MoneyChanged {
                                old_money,
                                new_money,
                            },
                        );
                    }
                }
                if publish {
                    self.represented_notify_money_removed_like_cpp(completion.owner_guid);
                    self.send_packet(&LootMoneyNotify {
                        money: completion
                            .durable_item_money_notified_amount
                            .expect("durable Item money completion retains notification amount"),
                        money_mod: 0,
                        sole_looter: true,
                    });

                    let fully_looted =
                        self.loot_table
                            .get_mut(&completion.owner_guid)
                            .is_some_and(|loot| {
                                loot.coins = 0;
                                loot_is_looted_like_cpp(loot)
                            });
                    if fully_looted {
                        // Source Item destruction remains owned by the normal
                        // C++ release phase; the completion only makes the
                        // already-durable money mutation visible first.
                        self.do_loot_release_owner_like_cpp(
                            completion.owner_guid,
                            completion.player_guid,
                        )
                        .await;
                    }
                }
                if drain_money_objectives && (apply_balance || publish) {
                    self.drain_represented_quest_objective_progress_like_cpp()
                        .await;
                }
                continue;
            }

            if targets_current_player && completion.item_owner_auto_release {
                debug_assert!(completion.owner_guid.is_item());
                let removal = self
                    .loot_table
                    .get_mut(&completion.owner_guid)
                    .and_then(|loot| {
                        let entry = loot
                            .items
                            .iter()
                            .find(|entry| entry.loot_list_id == completion.loot_list_id)?;
                        let free_for_all = entry.flags.freeforall;
                        let newly_removed = !loot_item_is_looted_for_player_like_cpp(
                            loot,
                            entry,
                            completion.player_guid,
                        );
                        let loot_obj = loot.loot_guid;
                        mark_loot_item_looted_for_player_like_cpp(
                            loot,
                            completion.loot_list_id,
                            completion.player_guid,
                        );
                        Some((
                            loot_obj,
                            free_for_all,
                            newly_removed,
                            loot_is_looted_like_cpp(loot),
                        ))
                    });

                if let Some((loot_obj, free_for_all, newly_removed, fully_looted)) = removal {
                    if newly_removed {
                        if free_for_all {
                            self.send_packet(&LootRemoved {
                                owner: completion.owner_guid,
                                loot_obj,
                                loot_list_id: completion.loot_list_id,
                            });
                        } else {
                            self.represented_notify_loot_item_removed_like_cpp(
                                completion.owner_guid,
                                completion.loot_list_id,
                            );
                        }
                    }

                    if fully_looted {
                        self.do_loot_release_owner_like_cpp(
                            completion.owner_guid,
                            completion.player_guid,
                        )
                        .await;
                    }
                }
            } else if targets_current_player && requires_runtime_recovery {
                // The detached worker already committed the authority. Refresh
                // the packet cache so disconnect's DoLootReleaseAll observes
                // the consumed claim rather than the pre-commit session copy.
                self.refresh_owned_loot_summary_like_cpp(completion.owner_guid);
                let _ = self.reconcile_represented_loot_cache_like_cpp(
                    completion.owner_guid,
                    completion.player_guid,
                );
            }

            if requires_runtime_recovery {
                // SQL committed after the packet waiter disappeared, before
                // its synchronous runtime inventory publication. Do not let
                // the player operate on a stale slot; the persisted grant and
                // source consumption are reconstructed on the next login.
                self.kick("durable loot item completed after handler cancellation; relog required");
            }
        }
    }

    pub(crate) async fn wait_for_active_loot_persistence_like_cpp(&mut self) {
        let mut authorities = Vec::<OwnedLootAuthority>::new();
        for authority in self.active_loot_view_authorities_like_cpp.values() {
            if authorities
                .iter()
                .any(|existing| existing.shares_storage_like_cpp(authority))
            {
                continue;
            }
            authorities.push(authority.clone());
        }
        for authority in authorities {
            authority.wait_for_persisting_claims_like_cpp().await;
        }
        self.wait_for_durable_item_loot_persistence_like_cpp().await;
        self.apply_pending_durable_item_loot_completions_like_cpp()
            .await;
    }

    /// Mirrors the observable side of C++ `Loot::~Loot`: once the exact
    /// object-owned allocation behind an open view is retired, detached, or
    /// replaced, the next session tick releases that stale client window.
    /// Each session owns its socket, so global object destruction is fanned
    /// out cooperatively without holding a map lock across network work.
    pub(crate) fn close_retired_active_loot_windows_like_cpp(&mut self, player_guid: ObjectGuid) {
        let mut stale_owners = self
            .active_loot_view_authorities_like_cpp
            .iter()
            .filter_map(|(owner_guid, authority)| {
                let generation = self
                    .active_loot_view_generations_like_cpp
                    .get(owner_guid)
                    .copied();
                let still_open = generation.is_some_and(|generation| {
                    authority
                        .snapshot_for_player_like_cpp(player_guid)
                        .is_some_and(|snapshot| snapshot.generation == generation)
                });
                (!still_open).then_some(*owner_guid)
            })
            .collect::<Vec<_>>();
        stale_owners.sort_unstable_by_key(|guid| (guid.high_value(), guid.low_value()));

        for owner_guid in stale_owners {
            self.close_stale_active_loot_view_like_cpp(owner_guid, player_guid);
        }
    }

    fn close_stale_active_loot_view_like_cpp(
        &mut self,
        owner_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) {
        self.discard_represented_personal_loot_cache_for_player_like_cpp(owner_guid, player_guid);
        self.send_packet(&SLootRelease {
            loot_obj: owner_guid,
            owner: player_guid,
        });
        self.clear_active_loot_guid_if(owner_guid);
    }

    /// Retire the detached Rust representation of Loot owned by an Item that
    /// a durable transaction has committed to destroy. C++ gets the same
    /// window teardown from destroying the Item and its owned `Loot`; this is
    /// deliberately narrower than `DoLootReleaseAll` and cannot consume or
    /// otherwise mutate an unrelated active loot owner.
    pub(crate) fn retire_committed_destroyed_item_loot_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) {
        if self.active_loot_view_owners.contains(&item_guid) || self.is_active_loot_guid(item_guid)
        {
            self.close_stale_active_loot_view_like_cpp(item_guid, player_guid);
        }
        self.loot_table.remove(&item_guid);
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

        let authoritative_release = if let Some(authority) =
            self.prepare_owned_loot_authority_for_active_request_like_cpp(owner_guid, player_guid)
        {
            if !self
                .active_loot_view_authorities_like_cpp
                .get(&owner_guid)
                .is_some_and(|opened| opened.shares_storage_like_cpp(&authority))
            {
                self.close_stale_active_loot_view_like_cpp(owner_guid, player_guid);
                return true;
            }
            let Some(active_generation) = self
                .active_loot_view_generations_like_cpp
                .get(&owner_guid)
                .copied()
            else {
                self.close_stale_active_loot_view_like_cpp(owner_guid, player_guid);
                return true;
            };
            let Some(close) =
                authority.close_viewer_if_generation_like_cpp(active_generation, player_guid)
            else {
                self.close_stale_active_loot_view_like_cpp(owner_guid, player_guid);
                return true;
            };
            Some(AuthoritativeLootReleaseLikeCpp {
                authority,
                selected_generation: active_generation,
                loot: close.snapshot.loot,
                whole_object_fully_looted: close.whole_object_fully_looted,
                whole_object_fully_skinned: close.whole_object_fully_skinned,
                object_generation: close.object_generation,
                lifecycle_revision: close.lifecycle_revision,
                require_no_viewers: false,
            })
        } else {
            None
        };

        if authoritative_release.is_none()
            && (owner_guid.is_creature_or_vehicle() || owner_guid.is_game_object())
            && !represented_local_loot_fixture_allowed_like_cpp()
        {
            self.close_stale_active_loot_view_like_cpp(owner_guid, player_guid);
            return true;
        }

        // C++ `Loot::isLooted()` requires both zero gold and zero remaining
        // player-visible item count.
        let Some(loot) = authoritative_release
            .as_ref()
            .map(|release| &release.loot)
            .or_else(|| self.loot_table.get(&owner_guid))
        else {
            return false;
        };
        let selected_pool_looted = loot_is_looted_like_cpp(loot);
        let represented_loot_type = loot.loot_type;
        let whole_object_fully_looted = if let Some(release) = authoritative_release.as_ref() {
            release.whole_object_fully_looted
        } else if owner_guid.is_game_object() {
            self.canonical_gameobject_fully_looted_after_represented_sync_like_cpp(
                owner_guid,
                player_guid,
                selected_pool_looted,
            )
        } else if owner_guid.is_creature_or_vehicle() {
            self.canonical_creature_fully_looted_after_represented_sync_like_cpp(
                owner_guid,
                player_guid,
                selected_pool_looted,
            )
        } else {
            selected_pool_looted
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
                if authoritative_release.is_some() {
                    self.discard_represented_personal_loot_cache_for_player_like_cpp(
                        owner_guid,
                        player_guid,
                    );
                }
                return true;
            }
            self.apply_represented_gameobject_loot_release_like_cpp(
                owner_guid,
                player_guid,
                selected_pool_looted,
                whole_object_fully_looted,
                authoritative_release.as_ref(),
            );
            let _ = self.queue_chest_gameobject_state_refresh_for_same_map_like_cpp(owner_guid);
            let go_type = self
                .represented_gameobject_use_states
                .get(&owner_guid)
                .and_then(|state| state.go_type)
                .map(u32::from);
            let selected_release_branch = selected_pool_looted
                || matches!(
                    go_type,
                    Some(GAMEOBJECT_TYPE_FISHING_NODE) | Some(GAMEOBJECT_TYPE_FISHING_HOLE)
                );
            if !selected_release_branch {
                if authoritative_release.is_some() {
                    self.discard_represented_personal_loot_cache_for_player_like_cpp(
                        owner_guid,
                        player_guid,
                    );
                }
                return true;
            }

            self.hide_represented_gameobject_for_player_after_loot_release_like_cpp(owner_guid);
            if go_type == Some(GAMEOBJECT_TYPE_GATHERING_NODE) {
                self.send_gathering_node_loot_release_dynamic_flags_update_like_cpp(owner_guid);
            }
            if authoritative_release.is_some() {
                self.discard_represented_personal_loot_cache_for_player_like_cpp(
                    owner_guid,
                    player_guid,
                );
            } else {
                self.loot_table.remove(&owner_guid);
            }
            return true;
        }

        if owner_guid.is_item()
            && matches!(
                represented_loot_type,
                LOOT_TYPE_PROSPECTING_LIKE_CPP | LOOT_TYPE_MILLING_LIKE_CPP
            )
        {
            // C++ always clears the generated Loot and consumes at most five
            // source items for prospecting/milling, even if the window closes
            // before every generated entry was taken.
            self.clear_active_loot_guid_if(owner_guid);
            self.loot_table.remove(&owner_guid);
            self.update_inventory_item_object_like_cpp(owner_guid, |item| {
                item.set_loot_generated(false);
            });
            self.destroy_direct_item_count_after_loot_release_like_cpp(owner_guid, Some(5))
                .await;
            return true;
        }

        if owner_guid.is_item() && !selected_pool_looted {
            self.clear_active_loot_guid_if(owner_guid);
            let item_has_loot_flag = self
                .inventory_items_like_cpp()
                .values()
                .find(|item| item.guid == owner_guid)
                .and_then(|item| self.item_template_flags(item.entry_id))
                .map(|flags| flags.contains(wow_constants::ItemFlags::HAS_LOOT));
            if item_has_loot_flag == Some(false) {
                self.destroy_fully_looted_direct_item(owner_guid).await;
            }
            return true;
        }

        self.clear_active_loot_guid_if(owner_guid);

        if !selected_pool_looted {
            let round_robin_released = if let Some(release) = authoritative_release.as_ref() {
                release
                    .authority
                    .clear_round_robin_if_generation_like_cpp(
                        release.selected_generation,
                        player_guid,
                    )
                    .is_some_and(|outcome| {
                        self.loot_table.insert(owner_guid, outcome.snapshot.loot);
                        outcome.cleared
                    })
            } else {
                self.loot_table.get_mut(&owner_guid).is_some_and(|loot| {
                    if loot.round_robin_player == player_guid {
                        loot.round_robin_player = ObjectGuid::EMPTY;
                        true
                    } else {
                        false
                    }
                })
            };
            if round_robin_released {
                self.represented_notify_loot_list_like_cpp(owner_guid);
            }
            if owner_guid.is_creature_or_vehicle() {
                let values_update = self.mutate_world_creature(owner_guid, |creature| {
                    creature.force_dynamic_flags_update_like_cpp();
                    creature.creature.unit().values_update()
                });
                if let Some(values_update) = values_update.as_ref() {
                    self.send_creature_loot_release_dynamic_flags_update_like_cpp(
                        owner_guid,
                        values_update,
                        authoritative_release
                            .as_ref()
                            .map(|release| &release.authority),
                    );
                }
            }
            if authoritative_release.is_some() {
                self.discard_represented_personal_loot_cache_for_player_like_cpp(
                    owner_guid,
                    player_guid,
                );
            }
            return true;
        }

        // Remove loot entry from memory once the represented loot is consumed.
        self.loot_table.remove(&owner_guid);

        if owner_guid.is_item() && selected_pool_looted {
            self.destroy_fully_looted_direct_item(owner_guid).await;
            return true;
        }

        if owner_guid.is_corpse() {
            self.remove_canonical_corpse_lootable_dynamic_flag_like_cpp(owner_guid);
            return true;
        }

        // C++ forces the viewer-dependent DynamicFlags field after every
        // creature release, including a selected personal pool that completed
        // while another pool remains.
        let forced_values_update = self.mutate_world_creature(owner_guid, |creature| {
            creature.force_dynamic_flags_update_like_cpp();
            creature.creature.unit().values_update()
        });

        if !whole_object_fully_looted {
            if let Some(values_update) = forced_values_update.as_ref() {
                self.send_creature_loot_release_dynamic_flags_update_like_cpp(
                    owner_guid,
                    values_update,
                    authoritative_release
                        .as_ref()
                        .map(|release| &release.authority),
                );
            }
            if authoritative_release.is_some() {
                self.discard_represented_personal_loot_cache_for_player_like_cpp(
                    owner_guid,
                    player_guid,
                );
            }
            return true;
        }

        let corpse_decay_looted_rate = self.loot_drop_rates_like_cpp().corpse_decay_looted;

        // Start corpse despawn timer if fully looted.
        let whole_object_fully_skinned = authoritative_release.as_ref().map_or(
            represented_loot_type == LOOT_TYPE_SKINNING_LIKE_CPP,
            |release| release.whole_object_fully_skinned,
        );
        let apply_lifecycle = |creature: &mut crate::map_manager::WorldCreature| {
            creature.remove_lootable_dynamic_flag_like_cpp();
            let marked = if !creature.is_alive() {
                let corpse_decay_secs = looted_corpse_decay_secs_like_cpp(
                    whole_object_fully_skinned,
                    creature.corpse_delay_secs_like_cpp(),
                    creature.ignore_corpse_decay_ratio_like_cpp(),
                    corpse_decay_looted_rate,
                );
                if !creature.all_loot_removed_from_corpse_like_cpp(
                    corpse_decay_looted_rate,
                    whole_object_fully_skinned,
                ) {
                    // C++ returns without resetting an already-expired
                    // corpse. The lifecycle mirror must remain expired too.
                    None
                } else {
                    Some((creature.entry(), corpse_decay_secs))
                }
            } else {
                None
            };
            (marked, creature.creature.unit().values_update())
        };
        let lifecycle_update = if let Some(release) = authoritative_release.as_ref() {
            self.mutate_world_creature_if_fully_looted_observation_like_cpp(
                owner_guid,
                &release.authority,
                release.object_generation,
                release.lifecycle_revision,
                apply_lifecycle,
            )
        } else {
            self.mutate_world_creature(owner_guid, apply_lifecycle)
        };

        if let Some((_, values_update)) = lifecycle_update.as_ref() {
            self.send_creature_loot_release_dynamic_flags_update_like_cpp(
                owner_guid,
                values_update,
                authoritative_release
                    .as_ref()
                    .map(|release| &release.authority),
            );
        }
        let marked = lifecycle_update.and_then(|(marked, _)| marked);

        if let Some((entry, corpse_decay_secs)) = marked {
            info!(
                "Creature {:?} (entry {}) fully looted — despawning in {}s",
                owner_guid, entry, corpse_decay_secs
            );
        }

        if authoritative_release.is_some() {
            self.discard_represented_personal_loot_cache_for_player_like_cpp(
                owner_guid,
                player_guid,
            );
        }

        true
    }

    async fn persist_and_consume_stored_item_money_like_cpp(
        &self,
        item_guid: ObjectGuid,
        cached_notified_amount: u64,
    ) -> Option<(Arc<AtomicBool>, Arc<AtomicBool>, u64, u64)> {
        let (worker, balance_applied, publication_applied) = self
            .spawn_stored_item_money_persistence_worker_like_cpp(
                item_guid,
                cached_notified_amount,
            )?;
        match worker.await {
            Ok(Ok((applied_delta, notified_amount))) => Some((
                balance_applied,
                publication_applied,
                applied_delta,
                notified_amount,
            )),
            Ok(Err(error)) => {
                warn!(
                    item_guid = item_guid.counter(),
                    ?error,
                    "failed to atomically persist and consume stored item loot money"
                );
                None
            }
            Err(error) => {
                warn!(
                    item_guid = item_guid.counter(),
                    ?error,
                    "stored item loot-money persistence worker terminated"
                );
                None
            }
        }
    }

    fn spawn_stored_item_money_persistence_worker_like_cpp(
        &self,
        item_guid: ObjectGuid,
        cached_notified_amount: u64,
    ) -> Option<(
        tokio::task::JoinHandle<Result<(u64, u64), LootMoneyPersistenceErrorLikeCpp>>,
        Arc<AtomicBool>,
        Arc<AtomicBool>,
    )> {
        let Some(player_guid) = self.player_guid() else {
            return None;
        };
        let test_result = self.loot_money_persistence_test_result_for_worker_like_cpp();
        let char_db = if test_result.is_some() {
            None
        } else {
            Some(self.char_db().map(Arc::clone)?)
        };
        let test_current_money = self.player_gold_like_cpp();
        let balance_applied = Arc::new(AtomicBool::new(false));
        let publication_applied = Arc::new(AtomicBool::new(false));
        let mut item_persistence_guard = self.begin_durable_item_loot_persistence_like_cpp();
        let money_persistence_tracker = self.durable_loot_money_persistence_tracker_like_cpp();
        let mut money_persistence_guard = money_persistence_tracker.begin_like_cpp().ok()?;
        let command_tx = self.session_command_tx();
        let worker_balance_applied = Arc::clone(&balance_applied);
        let worker_publication_applied = Arc::clone(&publication_applied);
        let worker = tokio::spawn(async move {
            let _money_mutation_lock = money_persistence_tracker
                .lock_money_mutation_like_cpp()
                .await;
            let (before, after, applied_delta, notified_amount) = if let Some(success) = test_result
            {
                tokio::task::yield_now().await;
                if !success {
                    return Err(LootMoneyPersistenceErrorLikeCpp::MissingCharacterDatabase);
                }
                let (after, applied_delta) =
                    loot_money_durable_outcome_like_cpp(test_current_money, cached_notified_amount);
                (
                    test_current_money,
                    after,
                    applied_delta,
                    cached_notified_amount,
                )
            } else {
                let char_db = char_db.expect("production stored-money worker has a database");
                let attempt = retry_deadlocked_operation_like_cpp(
                    || {
                        attempt_stored_item_money_transaction_like_cpp(
                            char_db.as_ref(),
                            player_guid,
                            item_guid,
                            cached_notified_amount,
                        )
                    },
                    stored_item_money_attempt_is_deadlock_like_cpp,
                )
                .await;
                let outcome = match attempt {
                    Ok(outcome) => outcome,
                    Err(StoredItemMoneyAttemptErrorLikeCpp::DefinitelyRolledBack(error)) => {
                        return Err(error);
                    }
                    Err(StoredItemMoneyAttemptErrorLikeCpp::CommitOutcomeUnknown {
                        error,
                        outcome,
                    }) => match reconcile_stored_item_money_commit_like_cpp(
                        char_db.as_ref(),
                        player_guid,
                        item_guid,
                        outcome,
                    )
                    .await
                    {
                        Ok(StoredItemMoneyCommitReconciliationLikeCpp::Committed) => outcome,
                        Ok(StoredItemMoneyCommitReconciliationLikeCpp::RolledBack) => {
                            return Err(LootMoneyPersistenceErrorLikeCpp::Database(
                                DatabaseError::Transaction(
                                    "stored Item money COMMIT was reconciled as rolled back"
                                        .to_string(),
                                ),
                            ));
                        }
                        Ok(StoredItemMoneyCommitReconciliationLikeCpp::Indeterminate) | Err(_) => {
                            money_persistence_guard.mark_indeterminate_like_cpp();
                            queue_stored_item_money_indeterminate_kick_like_cpp(&command_tx);
                            return Err(LootMoneyPersistenceErrorLikeCpp::CommitOutcomeUnknown(
                                error,
                            ));
                        }
                    },
                };
                (
                    outcome.before,
                    outcome.after,
                    outcome.applied_delta,
                    outcome.notified_amount,
                )
            };

            money_persistence_guard.commit_like_cpp(
                crate::session::mailbox::DurableLootMoneyCompletionLikeCpp {
                    durable_money_before: before,
                    durable_money_after: after,
                    durable_applied_amount: applied_delta,
                    applied: Arc::clone(&worker_balance_applied),
                },
            );
            item_persistence_guard.mark_committed_like_cpp(DurableItemLootCompletionLikeCpp {
                owner_guid: item_guid,
                loot_list_id: 0,
                player_guid,
                item_owner_auto_release: false,
                durable_item_money_applied_amount: Some(applied_delta),
                durable_item_money_notified_amount: Some(notified_amount),
                durable_item_money_balance_applied: Some(Arc::clone(&worker_balance_applied)),
                item_fanout: None,
                runtime_inventory_applied: Arc::clone(&worker_publication_applied),
            });
            Ok((applied_delta, notified_amount))
        });
        Some((worker, balance_applied, publication_applied))
    }

    async fn store_direct_loot_item_like_cpp(
        &mut self,
        loot_entry: &LootEntry,
        dungeon_encounter_id: u32,
    ) -> bool {
        self.store_direct_loot_item_with_source_like_cpp(
            loot_entry,
            dungeon_encounter_id,
            None,
            None,
            None,
        )
        .await
    }

    /// Apply the item state established by C++ `Player::StoreNewItem` and
    /// `_StoreItem` before the item is persisted or sent to the client.
    fn apply_stored_new_item_flags_like_cpp(&self, item_id: u32, slot: u8, item: &mut Item) {
        if let Some(template) = self.item_storage_template(item_id) {
            item.set_bonding(template.bonding);
        }
        item.set_item_flag(ItemFieldFlags::NEW_ITEM);
        item.bind_if_stored(is_bag_pos(make_item_pos(INVENTORY_SLOT_BAG_0, slot)));
    }

    fn stored_new_item_dynamic_flags_like_cpp(&self, item_id: u32, slot: u8) -> u32 {
        let mut item = Item::new(0);
        self.apply_stored_new_item_flags_like_cpp(item_id, slot, &mut item);
        item.item_flags_bits()
    }

    /// C++ `_StoreItem` binds the destination object before incrementing an
    /// existing stack. Unlike `StoreNewItem`, that historical object must not
    /// acquire `ITEM_FIELD_FLAG_NEW_ITEM` merely because more items arrived.
    fn stored_existing_item_dynamic_flags_like_cpp(
        &self,
        item_id: u32,
        slot: u8,
        existing: &Item,
    ) -> u32 {
        let mut planned = existing.clone();
        if let Some(template) = self.item_storage_template(item_id) {
            planned.set_bonding(template.bonding);
        }
        planned.bind_if_stored(is_bag_pos(make_item_pos(INVENTORY_SLOT_BAG_0, slot)));
        planned.item_flags_bits()
    }

    /// Queue count and any binding transition in one transaction. Runtime
    /// state is deliberately updated only after this transaction commits.
    fn append_existing_loot_stack_persistence_like_cpp(
        char_db: &CharacterDatabase,
        transaction: &mut SqlTransaction,
        db_guid: u64,
        new_count: u32,
        dynamic_flags: Option<u32>,
    ) {
        let mut update_count = char_db.prepare(CharStatements::UPD_ITEM_INSTANCE_COUNT);
        update_count.set_u32(0, new_count);
        update_count.set_u64(1, db_guid);
        transaction.append_expect_rows_affected(update_count, 1);

        if let Some(dynamic_flags) = dynamic_flags {
            let mut update_flags = char_db.prepare(CharStatements::UPD_ITEM_INSTANCE_FLAGS);
            update_flags.set_u32(0, dynamic_flags);
            update_flags.set_u64(1, db_guid);
            transaction.append_expect_rows_affected(update_flags, 1);
        }
    }

    /// Persist the complete result of one group-roll disenchant as a single
    /// durable award.
    ///
    /// C++ `LootRoll::Finish` first materializes a temporary
    /// `LOOT_DISENCHANTING` loot and then calls `Loot::AutoStore`.  The C++
    /// loop stores each generated material independently, which is unsafe for
    /// Rust's concurrently shared object authority: a later failure could
    /// reopen the original roll slot after an earlier material was durable.
    /// This bounded divergence keeps C++ generation/inventory rules but plans
    /// every material before creating one SQL transaction.  The detached
    /// transaction worker owns the original roll claim through COMMIT.
    async fn store_direct_disenchant_batch_like_cpp(
        &mut self,
        loot_entries: &[LootEntry],
        dungeon_encounter_id: u32,
        claim: Option<&LootClaimLease>,
        claim_commit_context: Option<LootItemClaimCommitContextLikeCpp>,
    ) -> bool {
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        if loot_entries.is_empty()
            || loot_entries
                .iter()
                .any(|entry| entry.item_id == 0 || entry.quantity == 0)
        {
            return false;
        }
        let durable_item_fanout = match (claim, claim_commit_context) {
            (Some(claim), Some(context)) => {
                let Some(fanout) = self.prepare_durable_loot_item_fanout_like_cpp(claim, context)
                else {
                    return false;
                };
                Some(fanout)
            }
            (None, None) => None,
            _ => return false,
        };

        #[cfg(test)]
        if let Some(grants) = self.loot_item_store_test_grants_like_cpp.clone() {
            let success = self.loot_item_store_test_success_like_cpp;
            let commit_gate = self.loot_item_store_test_commit_gate_like_cpp.clone();
            let grant_count = loot_entries.len();
            let runtime_inventory_applied =
                claim_commit_context.map(|_| Arc::new(AtomicBool::new(false)));
            let durable_item_completion = claim_commit_context
                .zip(runtime_inventory_applied.as_ref().map(Arc::clone))
                .map(|(context, runtime_inventory_applied)| {
                    (
                        self.begin_durable_item_loot_persistence_like_cpp(),
                        DurableItemLootCompletionLikeCpp {
                            owner_guid: context.owner_guid,
                            loot_list_id: context.loot_list_id,
                            player_guid: context.player_guid,
                            item_owner_auto_release: false,
                            durable_item_money_applied_amount: None,
                            durable_item_money_notified_amount: None,
                            durable_item_money_balance_applied: None,
                            item_fanout: durable_item_fanout.clone(),
                            runtime_inventory_applied,
                        },
                    )
                });
            let Ok(persistence) = spawn_loot_claim_persistence_worker_like_cpp(
                async move {
                    // Model the asynchronous commit boundary so cancellation
                    // regressions exercise the same ownership shape as SQL.
                    tokio::task::yield_now().await;
                    if let Some(gate) = commit_gate {
                        gate.notified().await;
                    }
                    if !success {
                        return Err(());
                    }
                    grants.fetch_add(grant_count, Ordering::SeqCst);
                    Ok(())
                },
                claim.cloned(),
                durable_item_completion,
            ) else {
                return false;
            };
            if !matches!(persistence.await, Ok(Ok(()))) {
                return false;
            }
            for (slot, entry) in loot_entries.iter().enumerate() {
                self.send_loot_item_push_result(
                    player_guid,
                    ObjectGuid::EMPTY,
                    entry,
                    0,
                    0,
                    u8::try_from(slot).unwrap_or(0),
                    entry.quantity,
                    entry.quantity,
                    false,
                    dungeon_encounter_id,
                );
            }
            if !self.publish_persisted_loot_item_removal_like_cpp(
                claim,
                claim_commit_context,
                durable_item_fanout.as_ref(),
            ) {
                return false;
            }
            if let Some(runtime_inventory_applied) = runtime_inventory_applied {
                runtime_inventory_applied.store(true, Ordering::Release);
            }
            return true;
        }

        let Some(char_db) = self.char_db().map(Arc::clone) else {
            return false;
        };

        // `CanStoreNewItem`'s max-count checks must see all generated stacks
        // of the same material, not each temporary LootItem in isolation.
        let mut quantity_by_item = HashMap::<u32, u32>::new();
        for entry in loot_entries {
            let Some(total) = quantity_by_item
                .get(&entry.item_id)
                .copied()
                .unwrap_or(0)
                .checked_add(entry.quantity)
            else {
                self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                return false;
            };
            quantity_by_item.insert(entry.item_id, total);
        }
        for (item_id, count) in quantity_by_item {
            let Some((store_result, _, _)) =
                self.plan_store_new_direct_inventory_item(item_id, count)
            else {
                self.send_equip_error(InventoryResult::ItemNotFound, None, None, 0, 0);
                return false;
            };
            if store_result != InventoryResult::Ok {
                self.send_equip_error(store_result, None, None, 0, 0);
                return false;
            }
        }

        let backpack_end = INVENTORY_SLOT_ITEM_START
            .saturating_add(INVENTORY_DEFAULT_SIZE)
            .min(INVENTORY_SLOT_ITEM_END);
        let mut planned_existing_stacks = Vec::<PlannedDisenchantExistingStack>::new();
        let mut planned_new_stacks = Vec::<PlannedLootNewStack>::new();
        let mut planned_grants = Vec::<PlannedDisenchantGrant>::new();

        for loot_entry in loot_entries {
            let random_properties = {
                let mut rng = self.represented_runtime_subrng_like_cpp();
                self.generate_loot_store_random_properties_with_rng_like_cpp(
                    loot_entry.item_id,
                    &mut rng,
                )
            };
            let max_stack = self
                .item_storage_template(loot_entry.item_id)
                .map(|template| template.max_stack_size)
                .unwrap_or(1)
                .max(1);
            let mut remaining = loot_entry.quantity;
            let mut existing_pushes = Vec::new();
            let mut new_pushes = Vec::new();

            // Existing backpack stacks are consumed first, matching the
            // direct StoreNewItem path represented in this server.
            for slot in INVENTORY_SLOT_ITEM_START..backpack_end {
                if remaining == 0 {
                    break;
                }
                let Some(existing) = self.inventory_items_like_cpp().get(&slot) else {
                    continue;
                };
                if existing.entry_id != loot_entry.item_id {
                    continue;
                }
                let Some(existing_object) =
                    self.inventory_item_objects_like_cpp().get(&existing.guid)
                else {
                    self.send_equip_error(InventoryResult::ItemNotFound, None, None, 0, 0);
                    return false;
                };
                if !loot_store_data_can_stack_with_item(
                    loot_entry,
                    random_properties,
                    existing_object,
                ) {
                    continue;
                }

                let current_count = planned_existing_stacks
                    .iter()
                    .find(|planned| planned.slot == slot)
                    .map(|planned| planned.new_count)
                    .unwrap_or_else(|| existing_object.count());
                let added_count = max_stack.saturating_sub(current_count).min(remaining);
                if added_count == 0 {
                    continue;
                }
                let new_count = current_count.saturating_add(added_count);
                if let Some(planned) = planned_existing_stacks
                    .iter_mut()
                    .find(|planned| planned.slot == slot)
                {
                    planned.new_count = new_count;
                } else {
                    let dynamic_flags = self.stored_existing_item_dynamic_flags_like_cpp(
                        loot_entry.item_id,
                        slot,
                        existing_object,
                    );
                    planned_existing_stacks.push(PlannedDisenchantExistingStack {
                        slot,
                        item_guid: existing.guid,
                        db_guid: existing.db_guid,
                        new_count,
                        dynamic_flags,
                        flags_changed: dynamic_flags != existing_object.item_flags_bits(),
                    });
                }
                existing_pushes.push(PlannedDisenchantExistingPush {
                    slot,
                    item_guid: existing.guid,
                    added_count,
                    new_count,
                });
                remaining = remaining.saturating_sub(added_count);
            }

            // A second generated LootItem for the same material may continue
            // a new stack already planned earlier in this same transaction.
            for (stack_index, stack) in planned_new_stacks.iter_mut().enumerate() {
                if remaining == 0 {
                    break;
                }
                if stack.entry_id != loot_entry.item_id
                    || stack.random_properties_id != random_properties.id
                    || stack.random_properties_seed != random_properties.seed
                    || stack.item_context != loot_entry.item_context
                {
                    continue;
                }
                let added_count = max_stack.saturating_sub(stack.count).min(remaining);
                if added_count == 0 {
                    continue;
                }
                stack.count = stack.count.saturating_add(added_count);
                new_pushes.push(PlannedDisenchantNewPush {
                    stack_index,
                    added_count,
                    new_count: stack.count,
                });
                remaining = remaining.saturating_sub(added_count);
            }

            while remaining > 0 {
                let Some(slot) = (INVENTORY_SLOT_ITEM_START..backpack_end).find(|slot| {
                    !self.inventory_items_like_cpp().contains_key(slot)
                        && !planned_new_stacks.iter().any(|stack| stack.slot == *slot)
                }) else {
                    self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                    return false;
                };
                let count = remaining.min(max_stack);
                let stack_index = planned_new_stacks.len();
                planned_new_stacks.push(PlannedLootNewStack {
                    slot,
                    entry_id: loot_entry.item_id,
                    count,
                    max_durability: self.item_template_max_durability(loot_entry.item_id),
                    dynamic_flags: self
                        .stored_new_item_dynamic_flags_like_cpp(loot_entry.item_id, slot),
                    random_properties_id: random_properties.id,
                    random_properties_seed: random_properties.seed,
                    item_context: loot_entry.item_context,
                });
                new_pushes.push(PlannedDisenchantNewPush {
                    stack_index,
                    added_count: count,
                    new_count: count,
                });
                remaining = remaining.saturating_sub(count);
            }

            planned_grants.push(PlannedDisenchantGrant {
                entry: loot_entry.clone(),
                random_properties,
                existing_pushes,
                new_pushes,
            });
        }

        let mut transaction = SqlTransaction::new();
        for stack in &planned_existing_stacks {
            Self::append_existing_loot_stack_persistence_like_cpp(
                &char_db,
                &mut transaction,
                stack.db_guid,
                stack.new_count,
                stack.flags_changed.then_some(stack.dynamic_flags),
            );
        }

        let mut created_new_stacks = Vec::with_capacity(planned_new_stacks.len());
        if !planned_new_stacks.is_empty() {
            let Some(allocated_guids) =
                self.allocate_item_instance_guids_like_cpp(planned_new_stacks.len())
            else {
                warn!(
                    count = planned_new_stacks.len(),
                    "disenchant item grant has no process-wide item GUID allocator"
                );
                self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                return false;
            };

            for (stack, (db_guid, item_guid)) in planned_new_stacks.iter().zip(allocated_guids) {
                let mut insert_item =
                    char_db.prepare(CharStatements::INS_ITEM_INSTANCE_WITH_RANDOM_CONTEXT);
                insert_item.set_u64(0, db_guid);
                insert_item.set_u32(1, stack.entry_id);
                insert_item.set_u64(2, player_guid.counter() as u64);
                insert_item.set_u32(3, stack.count);
                insert_item.set_u32(4, stack.max_durability);
                insert_item.set_u32(5, stack.dynamic_flags);
                insert_item.set_i32(6, stack.random_properties_id);
                insert_item.set_i32(7, stack.random_properties_seed);
                insert_item.set_u8(8, stack.item_context);
                transaction.append_expect_rows_affected(insert_item, 1);

                let mut insert_inventory = char_db.prepare(CharStatements::INS_CHAR_INVENTORY);
                insert_inventory.set_u64(0, player_guid.counter() as u64);
                insert_inventory.set_u8(1, stack.slot);
                insert_inventory.set_u64(2, db_guid);
                transaction.append_expect_rows_affected(insert_inventory, 1);

                created_new_stacks.push((stack.clone(), db_guid, item_guid));
            }
        }

        let runtime_inventory_applied =
            claim_commit_context.map(|_| Arc::new(AtomicBool::new(false)));
        let durable_item_completion = claim_commit_context
            .zip(runtime_inventory_applied.as_ref().map(Arc::clone))
            .map(|(context, runtime_inventory_applied)| {
                (
                    self.begin_durable_item_loot_persistence_like_cpp(),
                    DurableItemLootCompletionLikeCpp {
                        owner_guid: context.owner_guid,
                        loot_list_id: context.loot_list_id,
                        player_guid: context.player_guid,
                        item_owner_auto_release: false,
                        durable_item_money_applied_amount: None,
                        durable_item_money_notified_amount: None,
                        durable_item_money_balance_applied: None,
                        item_fanout: durable_item_fanout.clone(),
                        runtime_inventory_applied,
                    },
                )
            });
        let persistence_char_db = Arc::clone(&char_db);
        let persistence = match spawn_sql_loot_claim_persistence_worker_like_cpp(
            async move {
                transaction
                    .commit_with_outcome_like_cpp(persistence_char_db.pool())
                    .await
            },
            claim.cloned(),
            durable_item_completion,
            self.session_command_tx(),
        ) {
            Ok(persistence) => persistence,
            Err(error) => {
                warn!(?error, "disenchant claim closed before persistence started");
                return false;
            }
        };
        match persistence.await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                warn!(?error, "disenchant material batch transaction failed");
                self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                return false;
            }
            Err(error) => {
                warn!(?error, "disenchant material batch worker terminated");
                self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                return false;
            }
        }

        for stack in &planned_existing_stacks {
            self.update_inventory_item_object_like_cpp(stack.item_guid, |item| {
                item.set_count(stack.new_count);
                if stack.flags_changed {
                    item.replace_all_item_flags(ItemFieldFlags::from_bits_retain(
                        stack.dynamic_flags,
                    ));
                }
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
            self.apply_stored_new_item_flags_like_cpp(stack.entry_id, stack.slot, &mut item_object);
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
        if let Some(runtime_inventory_applied) = runtime_inventory_applied {
            runtime_inventory_applied.store(true, Ordering::Release);
        }

        for grant in &planned_grants {
            let quest_log_item_id = self
                .load_creature_item_template_addon_loot_metadata_like_cpp(grant.entry.item_id)
                .await
                .quest_log_item_id
                .try_into()
                .unwrap_or(0);
            let mut changed_quest_ids = self
                .apply_quest_source_item_added_non_bound_objective_progress_like_cpp(
                    grant.entry.item_id,
                    quest_log_item_id,
                    grant.entry.quantity,
                )
                .await;
            self.save_changed_represented_quest_statuses_like_cpp(&mut changed_quest_ids)
                .await;
        }

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
                    dynamic_flags: stack.dynamic_flags,
                    durability: stack.max_durability,
                    max_durability: stack.max_durability,
                    random_properties_seed: stack.random_properties_seed,
                    random_properties_id: stack.random_properties_id,
                    enchantments: [ItemEnchantmentValuesUpdate::default(); 13],
                    gems: Vec::new(),
                    context: stack.item_context,
                    container_slots: 0,
                    container_item_guids: [ObjectGuid::EMPTY; 36],
                })
                .collect();
            self.send_packet(&UpdateObject::create_stored_items(item_creates, map_id));
        }
        for stack in &planned_existing_stacks {
            let update = if stack.flags_changed {
                UpdateObject::item_stack_count_and_flags_update(
                    stack.item_guid,
                    map_id,
                    stack.new_count,
                    stack.dynamic_flags,
                )
            } else {
                UpdateObject::item_stack_count_update(stack.item_guid, map_id, stack.new_count)
            };
            self.send_packet(&update);
        }

        // C++ writes each material's item update on the instance connection
        // before `SendNewItem` routes its push result to the realm connection.
        if !self
            .wait_for_instance_send_before_realm_send_like_cpp()
            .await
        {
            let _ = self.publish_persisted_loot_item_removal_like_cpp(
                claim,
                claim_commit_context,
                durable_item_fanout.as_ref(),
            );
            self.sync_player_registry_state_like_cpp();
            self.kick("loot socket ordering fence failed after durable disenchant claim");
            return true;
        }

        for grant in &planned_grants {
            for push in &grant.existing_pushes {
                self.send_loot_item_push_result(
                    player_guid,
                    push.item_guid,
                    &grant.entry,
                    grant.random_properties.id,
                    grant.random_properties.seed,
                    push.slot,
                    push.added_count,
                    push.new_count,
                    false,
                    dungeon_encounter_id,
                );
            }
            for push in &grant.new_pushes {
                let (stack, _, item_guid) = &created_new_stacks[push.stack_index];
                self.send_loot_item_push_result(
                    player_guid,
                    *item_guid,
                    &grant.entry,
                    stack.random_properties_id,
                    stack.random_properties_seed,
                    stack.slot,
                    push.added_count,
                    push.new_count,
                    false,
                    dungeon_encounter_id,
                );
            }
        }

        // `Loot::AutoStore` completes every realm-routed `SendNewItem` before
        // `LootRoll::Finish` emits the original slot removal on the instance
        // connection. Do not publish the later instance packets without the
        // writer acknowledgement; reconnect will reload the durable grant.
        if !self
            .wait_for_realm_send_before_instance_update_like_cpp()
            .await
        {
            self.sync_player_registry_state_like_cpp();
            self.kick("loot socket ordering fence failed after durable disenchant claim");
            return true;
        }

        // C++ `Loot::AutoStore` performs `StoreNewItem` and `SendNewItem` for
        // every generated material. Only after `AutoStore` returns does
        // `LootRoll::Finish` call `NotifyItemRemoved` for the original loot.
        // SQL and the claim were already committed by the detached worker.
        if !self.publish_persisted_loot_item_removal_like_cpp(
            claim,
            claim_commit_context,
            durable_item_fanout.as_ref(),
        ) {
            return false;
        }

        if !created_new_stacks.is_empty() {
            let changed_slots = created_new_stacks
                .iter()
                .map(|(stack, _, item_guid)| (stack.slot, *item_guid))
                .collect::<Vec<_>>();
            self.send_player_values_update_from_entity_bridge(&changed_slots, &[], &[], &[], None);
        }
        for update in &collection_updates {
            self.send_player_values_update_like_cpp(update);
        }
        self.sync_player_registry_state_like_cpp();
        true
    }

    async fn store_direct_loot_item_from_owner_like_cpp(
        &mut self,
        loot_entry: &LootEntry,
        dungeon_encounter_id: u32,
        owner_guid: ObjectGuid,
    ) -> bool {
        self.store_direct_loot_item_with_source_like_cpp(
            loot_entry,
            dungeon_encounter_id,
            owner_guid.is_item().then_some(owner_guid),
            None,
            None,
        )
        .await
    }

    async fn store_claimed_direct_loot_item_from_owner_like_cpp(
        &mut self,
        loot_entry: &LootEntry,
        dungeon_encounter_id: u32,
        owner_guid: ObjectGuid,
        loot_obj: ObjectGuid,
        claim: &LootClaimLease,
    ) -> bool {
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        self.store_direct_loot_item_with_source_like_cpp(
            loot_entry,
            dungeon_encounter_id,
            owner_guid.is_item().then_some(owner_guid),
            Some(claim),
            Some(LootItemClaimCommitContextLikeCpp {
                owner_guid,
                loot_obj,
                loot_list_id: loot_entry.loot_list_id,
                player_guid,
                free_for_all: loot_entry.flags.freeforall,
            }),
        )
        .await
    }

    async fn store_direct_loot_item_with_source_like_cpp(
        &mut self,
        loot_entry: &LootEntry,
        dungeon_encounter_id: u32,
        stored_item_loot_source: Option<ObjectGuid>,
        claim: Option<&LootClaimLease>,
        claim_commit_context: Option<LootItemClaimCommitContextLikeCpp>,
    ) -> bool {
        let item_id = loot_entry.item_id;
        let count = loot_entry.quantity;
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        let durable_item_fanout = match (claim, claim_commit_context) {
            (Some(claim), Some(context)) => {
                let Some(fanout) = self.prepare_durable_loot_item_fanout_like_cpp(claim, context)
                else {
                    return false;
                };
                Some(fanout)
            }
            (None, None) => None,
            _ => return false,
        };
        // C++ Loot::AutoStore validates CanStoreNewItem before StoreNewItem.
        // That ordering still applies when StoreNewItem later converts a
        // quest-bound Item into objective credit and returns nullptr.
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
        let quest_log_item_id = self
            .load_creature_item_template_addon_loot_metadata_like_cpp(item_id)
            .await
            .quest_log_item_id
            .try_into()
            .unwrap_or(0);
        let bound_objective_plan = self
            .plan_quest_source_item_bound_objective_persistence_like_cpp(
                item_id,
                quest_log_item_id,
                count,
            );
        #[cfg(test)]
        if let Some(grants) = self.loot_item_store_test_grants_like_cpp.clone() {
            let success = self.loot_item_store_test_success_like_cpp;
            let commit_gate = self.loot_item_store_test_commit_gate_like_cpp.clone();
            let materializes_inventory_item = bound_objective_plan.is_none();
            let durable_completion_context = stored_item_loot_source
                .map(|owner_guid| (owner_guid, loot_entry.loot_list_id, player_guid, true))
                .or_else(|| {
                    claim_commit_context.map(|context| {
                        (
                            context.owner_guid,
                            context.loot_list_id,
                            context.player_guid,
                            false,
                        )
                    })
                });
            let runtime_inventory_applied =
                durable_completion_context.map(|_| Arc::new(AtomicBool::new(false)));
            let durable_item_completion = durable_completion_context
                .zip(runtime_inventory_applied.as_ref().map(Arc::clone))
                .map(
                    |(
                        (owner_guid, loot_list_id, player_guid, item_owner_auto_release),
                        runtime_inventory_applied,
                    )| {
                        (
                            self.begin_durable_item_loot_persistence_like_cpp(),
                            DurableItemLootCompletionLikeCpp {
                                owner_guid,
                                loot_list_id,
                                player_guid,
                                item_owner_auto_release,
                                durable_item_money_applied_amount: None,
                                durable_item_money_notified_amount: None,
                                durable_item_money_balance_applied: None,
                                item_fanout: durable_item_fanout.clone(),
                                runtime_inventory_applied,
                            },
                        )
                    },
                );
            let Ok(worker) = spawn_loot_claim_persistence_worker_like_cpp(
                async move {
                    tokio::task::yield_now().await;
                    if let Some(gate) = commit_gate {
                        gate.notified().await;
                    }
                    if !success {
                        return Err(());
                    }
                    if materializes_inventory_item {
                        grants.fetch_add(1, Ordering::SeqCst);
                    }
                    Ok(())
                },
                claim.cloned(),
                durable_item_completion,
            ) else {
                return false;
            };
            if !matches!(worker.await, Ok(Ok(()))) {
                return false;
            }
            if let Some(plan) = bound_objective_plan.as_ref() {
                let applied = self
                    .apply_quest_source_item_bound_objective_preflight_like_cpp(
                        item_id,
                        quest_log_item_id,
                        count,
                    )
                    .await;
                debug_assert!(applied.as_ref().is_some_and(|result| result.no_grant));
                debug_assert!(plan.statuses.iter().all(|planned| {
                    self.player_quests
                        .get(&planned.quest_id)
                        .is_some_and(|actual| {
                            actual.status == planned.status
                                && actual.objective_counts == planned.objective_counts
                        })
                }));
            }
            if !self.publish_persisted_loot_item_removal_like_cpp(
                claim,
                claim_commit_context,
                durable_item_fanout.as_ref(),
            ) {
                return false;
            }
            if bound_objective_plan.is_none() {
                self.send_loot_item_push_result(
                    player_guid,
                    ObjectGuid::EMPTY,
                    loot_entry,
                    0,
                    0,
                    0,
                    count,
                    count,
                    false,
                    dungeon_encounter_id,
                );
            }
            if let Some(runtime_inventory_applied) = runtime_inventory_applied {
                runtime_inventory_applied.store(true, Ordering::Release);
            }
            return true;
        }
        let Some(char_db) = self.char_db().map(Arc::clone) else {
            return false;
        };
        if let Some(bound_objective_plan) = bound_objective_plan {
            let mut tx = SqlTransaction::new();
            self.append_planned_quest_statuses_to_transaction_like_cpp(
                &mut tx,
                char_db.as_ref(),
                player_guid.counter() as u64,
                &bound_objective_plan.statuses,
            );
            if let Some(item_guid) = stored_item_loot_source {
                let mut delete_source = char_db.prepare(CharStatements::DEL_ITEMCONTAINER_ITEM);
                delete_source.set_u64(0, item_guid.counter() as u64);
                delete_source.set_u32(1, item_id);
                delete_source.set_u32(2, count);
                delete_source.set_u32(3, u32::from(loot_entry.loot_list_id));
                tx.append_expect_rows_affected(delete_source, 1);
            }

            let durable_completion_context = stored_item_loot_source
                .map(|owner_guid| (owner_guid, loot_entry.loot_list_id, player_guid, true))
                .or_else(|| {
                    claim_commit_context.map(|context| {
                        (
                            context.owner_guid,
                            context.loot_list_id,
                            context.player_guid,
                            false,
                        )
                    })
                });
            let runtime_inventory_applied =
                durable_completion_context.map(|_| Arc::new(AtomicBool::new(false)));
            let durable_item_completion = durable_completion_context
                .zip(runtime_inventory_applied.as_ref().map(Arc::clone))
                .map(
                    |(
                        (owner_guid, loot_list_id, player_guid, item_owner_auto_release),
                        runtime_inventory_applied,
                    )| {
                        (
                            self.begin_durable_item_loot_persistence_like_cpp(),
                            DurableItemLootCompletionLikeCpp {
                                owner_guid,
                                loot_list_id,
                                player_guid,
                                item_owner_auto_release,
                                durable_item_money_applied_amount: None,
                                durable_item_money_notified_amount: None,
                                durable_item_money_balance_applied: None,
                                item_fanout: durable_item_fanout.clone(),
                                runtime_inventory_applied,
                            },
                        )
                    },
                );
            let persistence_char_db = Arc::clone(&char_db);
            let persistence = match spawn_sql_loot_claim_persistence_worker_like_cpp(
                async move {
                    tx.commit_with_outcome_like_cpp(persistence_char_db.pool())
                        .await
                },
                claim.cloned(),
                durable_item_completion,
                self.session_command_tx(),
            ) {
                Ok(persistence) => persistence,
                Err(error) => {
                    warn!(
                        ?error,
                        "LootItem: quest-bound claim closed before persistence started"
                    );
                    self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                    return false;
                }
            };
            match persistence.await {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    warn!(?error, "LootItem: quest-bound objective transaction failed");
                    self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                    return false;
                }
                Err(error) => {
                    warn!(
                        ?error,
                        "LootItem: detached quest-bound transaction worker terminated"
                    );
                    self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                    return false;
                }
            }

            let applied = self
                .apply_quest_source_item_bound_objective_preflight_like_cpp(
                    item_id,
                    quest_log_item_id,
                    count,
                )
                .await;
            if !applied.as_ref().is_some_and(|result| result.no_grant)
                || !bound_objective_plan.statuses.iter().all(|planned| {
                    self.player_quests
                        .get(&planned.quest_id)
                        .is_some_and(|actual| {
                            actual.status == planned.status
                                && actual.objective_counts == planned.objective_counts
                        })
                })
            {
                self.kick("durable quest-bound loot state diverged; relog required");
                return true;
            }
            if let Some(runtime_inventory_applied) = runtime_inventory_applied {
                runtime_inventory_applied.store(true, Ordering::Release);
            }
            if !self.publish_persisted_loot_item_removal_like_cpp(
                claim,
                claim_commit_context,
                durable_item_fanout.as_ref(),
            ) {
                return false;
            }
            self.sync_player_registry_state_like_cpp();
            return true;
        }
        let store_random_properties = {
            let mut rng = self.represented_runtime_subrng_like_cpp();
            self.generate_loot_store_random_properties_with_rng_like_cpp(item_id, &mut rng)
        };

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

        let mut planned_existing_counts = Vec::<PlannedDirectLootExistingStack>::new();
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
                    .find(|planned| planned.slot == slot)
                    .map(|planned| planned.new_count)
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
                    .find(|planned| planned.slot == slot)
                {
                    existing_plan.new_count = new_count;
                    existing_plan.added_count =
                        existing_plan.added_count.saturating_add(dest.count);
                } else {
                    let dynamic_flags = self.stored_existing_item_dynamic_flags_like_cpp(
                        item_id,
                        slot,
                        existing_object,
                    );
                    planned_existing_counts.push(PlannedDirectLootExistingStack {
                        slot,
                        item_guid: existing.guid,
                        db_guid: existing.db_guid,
                        new_count,
                        added_count: dest.count,
                        dynamic_flags,
                        flags_changed: dynamic_flags != existing_object.item_flags_bits(),
                    });
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
                dynamic_flags: self.stored_new_item_dynamic_flags_like_cpp(item_id, slot),
                random_properties_id: store_random_properties.id,
                random_properties_seed: store_random_properties.seed,
                item_context: loot_entry.item_context,
            });
        }

        let mut tx = SqlTransaction::new();
        for stack in &planned_existing_counts {
            Self::append_existing_loot_stack_persistence_like_cpp(
                &char_db,
                &mut tx,
                stack.db_guid,
                stack.new_count,
                stack.flags_changed.then_some(stack.dynamic_flags),
            );
        }

        let mut created_new_stacks = Vec::new();
        if !planned_new_stacks.is_empty() {
            let Some(allocated_guids) =
                self.allocate_item_instance_guids_like_cpp(planned_new_stacks.len())
            else {
                warn!(
                    count = planned_new_stacks.len(),
                    "loot item grant has no process-wide item GUID allocator"
                );
                self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                return false;
            };

            for (stack, (db_guid, item_guid)) in planned_new_stacks.iter().zip(allocated_guids) {
                let mut ins_item =
                    char_db.prepare(CharStatements::INS_ITEM_INSTANCE_WITH_RANDOM_CONTEXT);
                ins_item.set_u64(0, db_guid);
                ins_item.set_u32(1, stack.entry_id);
                ins_item.set_u64(2, player_guid.counter() as u64);
                ins_item.set_u32(3, stack.count);
                ins_item.set_u32(4, stack.max_durability);
                ins_item.set_u32(5, stack.dynamic_flags);
                ins_item.set_i32(6, stack.random_properties_id);
                ins_item.set_i32(7, stack.random_properties_seed);
                ins_item.set_u8(8, stack.item_context);
                tx.append_expect_rows_affected(ins_item, 1);

                let mut ins_inv = char_db.prepare(CharStatements::INS_CHAR_INVENTORY);
                ins_inv.set_u64(0, player_guid.counter() as u64);
                ins_inv.set_u8(1, stack.slot);
                ins_inv.set_u64(2, db_guid);
                tx.append_expect_rows_affected(ins_inv, 1);

                created_new_stacks.push((stack.clone(), db_guid, item_guid));
            }
        }

        // Item-container loot is a move between two durable stores.  Delete
        // the source row in the same transaction that grants the destination
        // stack so a crash cannot duplicate (grant-only) or lose (delete-only)
        // the item.
        if let Some(item_guid) = stored_item_loot_source {
            let mut delete_source = char_db.prepare(CharStatements::DEL_ITEMCONTAINER_ITEM);
            delete_source.set_u64(0, item_guid.counter() as u64);
            delete_source.set_u32(1, item_id);
            delete_source.set_u32(2, count);
            delete_source.set_u32(3, u32::from(loot_entry.loot_list_id));
            tx.append_expect_rows_affected(delete_source, 1);
        }

        let durable_claim = claim.cloned();
        let durable_completion_context = stored_item_loot_source
            .map(|owner_guid| (owner_guid, loot_entry.loot_list_id, player_guid, true))
            .or_else(|| {
                claim_commit_context.map(|context| {
                    (
                        context.owner_guid,
                        context.loot_list_id,
                        context.player_guid,
                        false,
                    )
                })
            });
        let runtime_inventory_applied =
            durable_completion_context.map(|_| Arc::new(AtomicBool::new(false)));
        let durable_item_completion = durable_completion_context
            .zip(runtime_inventory_applied.as_ref().map(Arc::clone))
            .map(
                |(
                    (owner_guid, loot_list_id, player_guid, item_owner_auto_release),
                    runtime_inventory_applied,
                )| {
                    (
                        self.begin_durable_item_loot_persistence_like_cpp(),
                        DurableItemLootCompletionLikeCpp {
                            owner_guid,
                            loot_list_id,
                            player_guid,
                            item_owner_auto_release,
                            durable_item_money_applied_amount: None,
                            durable_item_money_notified_amount: None,
                            durable_item_money_balance_applied: None,
                            item_fanout: durable_item_fanout.clone(),
                            runtime_inventory_applied,
                        },
                    )
                },
            );
        let persistence_char_db = Arc::clone(&char_db);
        let persistence = match spawn_sql_loot_claim_persistence_worker_like_cpp(
            async move {
                tx.commit_with_outcome_like_cpp(persistence_char_db.pool())
                    .await
            },
            durable_claim,
            durable_item_completion,
            self.session_command_tx(),
        ) {
            Ok(persistence) => persistence,
            Err(error) => {
                warn!(?error, "LootItem: claim closed before persistence started");
                self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                return false;
            }
        };
        let persistence_result = match persistence.await {
            Ok(result) => result,
            Err(error) => {
                warn!(
                    ?error,
                    "LootItem: detached store transaction worker terminated"
                );
                self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
                return false;
            }
        };
        if let Err(e) = persistence_result {
            warn!("LootItem: store transaction failed: {e:?}");
            self.send_equip_error(InventoryResult::InvFull, None, None, 0, 0);
            return false;
        }

        for stack in &planned_existing_counts {
            self.update_inventory_item_object_like_cpp(stack.item_guid, |item| {
                item.set_count(stack.new_count);
                if stack.flags_changed {
                    item.replace_all_item_flags(ItemFieldFlags::from_bits_retain(
                        stack.dynamic_flags,
                    ));
                }
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
            self.apply_stored_new_item_flags_like_cpp(stack.entry_id, stack.slot, &mut item_object);
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
        if let Some(runtime_inventory_applied) = runtime_inventory_applied {
            runtime_inventory_applied.store(true, Ordering::Release);
        }

        let mut changed_quest_ids = self
            .apply_quest_source_item_added_non_bound_objective_progress_like_cpp(
                item_id,
                quest_log_item_id,
                count,
            )
            .await;
        self.save_changed_represented_quest_statuses_like_cpp(&mut changed_quest_ids)
            .await;

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
                    dynamic_flags: stack.dynamic_flags,
                    durability: stack.max_durability,
                    max_durability: stack.max_durability,
                    random_properties_seed: stack.random_properties_seed,
                    random_properties_id: stack.random_properties_id,
                    enchantments: [ItemEnchantmentValuesUpdate::default(); 13],
                    gems: Vec::new(),
                    context: stack.item_context,
                    container_slots: 0,
                    container_item_guids: [ObjectGuid::EMPTY; 36],
                })
                .collect();
            self.send_packet(&UpdateObject::create_stored_items(item_creates, map_id));
        }

        for stack in &planned_existing_counts {
            let update = if stack.flags_changed {
                UpdateObject::item_stack_count_and_flags_update(
                    stack.item_guid,
                    map_id,
                    stack.new_count,
                    stack.dynamic_flags,
                )
            } else {
                UpdateObject::item_stack_count_update(stack.item_guid, map_id, stack.new_count)
            };
            self.send_packet(&update);
        }

        // The worker committed SQL and the authority claim before runtime
        // publication. C++ `StoreNewItem` sends the stored item's update,
        // `Player::StoreLootItem` then notifies removal, and only afterwards
        // does `SendNewItem` emit `SMSG_ITEM_PUSH_RESULT`.
        if !self.publish_persisted_loot_item_removal_like_cpp(
            claim,
            claim_commit_context,
            durable_item_fanout.as_ref(),
        ) {
            return false;
        }

        if !self
            .wait_for_instance_send_before_realm_send_like_cpp()
            .await
        {
            self.sync_player_registry_state_like_cpp();
            self.kick("loot socket ordering fence failed after durable item claim");
            return true;
        }

        for stack in &planned_existing_counts {
            self.send_loot_item_push_result(
                player_guid,
                stack.item_guid,
                loot_entry,
                store_random_properties.id,
                store_random_properties.seed,
                stack.slot,
                stack.added_count,
                stack.new_count,
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

        if (!created_new_stacks.is_empty() || !collection_updates.is_empty())
            && !self
                .wait_for_realm_send_before_instance_update_like_cpp()
                .await
        {
            self.sync_player_registry_state_like_cpp();
            self.kick("loot socket ordering fence failed after durable item claim");
            return true;
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
        self.send_packet_realm(&ItemPushResult {
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
        self.destroy_direct_item_count_after_loot_release_like_cpp(item_guid, None)
            .await;
    }

    async fn destroy_direct_item_count_after_loot_release_like_cpp(
        &mut self,
        item_guid: ObjectGuid,
        maximum_destroy_count: Option<u32>,
    ) {
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

        let current_count = runtime_item.as_ref().map_or(1, Item::count);
        let new_count =
            direct_item_count_after_loot_release_like_cpp(current_count, maximum_destroy_count);
        if new_count != 0 {
            let mut update_count = char_db.prepare(CharStatements::UPD_ITEM_INSTANCE_COUNT);
            update_count.set_u32(0, new_count);
            update_count.set_u64(1, item.db_guid);
            if let Err(error) = char_db.execute(&update_count).await {
                warn!(?error, "LootRelease: update partially consumed item failed");
                return;
            }
            self.update_inventory_item_object_like_cpp(item_guid, |item| {
                item.set_count(new_count);
                item.set_loot_generated(false);
            });
            self.sync_object_accessor_player();
            self.send_packet(&UpdateObject::item_stack_count_update(
                item_guid,
                self.player_map_id_like_cpp(),
                new_count,
            ));
            return;
        }

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

fn durable_loot_item_fanout_viewers_like_cpp(
    precommit_viewers: &[ObjectGuid],
    committed_viewers: &[ObjectGuid],
) -> HashSet<ObjectGuid> {
    precommit_viewers
        .iter()
        .chain(committed_viewers)
        .copied()
        .collect()
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
        self.active_quest_statuses
            .get(&quest_id)
            .copied()
            .or_else(|| {
                self.rewarded_quests
                    .contains(&quest_id)
                    .then_some(QUEST_STATUS_REWARDED_LIKE_CPP)
            })
            .unwrap_or(QUEST_STATUS_NONE_LIKE_CPP)
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
        Some(QUEST_STATUS_COMPLETE_LIKE_CPP) => 0x02,
        Some(QUEST_STATUS_INCOMPLETE_LIKE_CPP) => 0x08,
        Some(QUEST_STATUS_FAILED_LIKE_CPP) => 0x20,
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

fn generated_shared_gameobject_loot_item_to_entry_like_cpp<FAllowed>(
    item: GeneratedLootItem,
    addon_metadata: ItemTemplateAddonLootMetadataLikeCpp,
    allowed_looters: &[ObjectGuid],
    mut item_allowed_for_player: FAllowed,
) -> LootEntry
where
    FAllowed: FnMut(LootStoreItemContext, ObjectGuid) -> bool,
{
    let store_item_context = item.store_item_context;
    let mut entry = generated_creature_loot_item_to_entry_like_cpp(item, addon_metadata);
    for looter in allowed_looters {
        if item_allowed_for_player(store_item_context, *looter) {
            entry.add_allowed_looter_like_cpp(*looter);
        }
    }
    entry
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
    loot_lifecycle_revision: u64,
}

#[derive(Debug, Clone)]
struct RepresentedGameObjectLootInstallObservationLikeCpp {
    authority: OwnedLootAuthority,
    object_generation: u64,
    loot_lifecycle_revision: u64,
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

#[cfg(test)]
fn represented_loot_object_guid_like_cpp(owner: ObjectGuid) -> ObjectGuid {
    if owner.is_empty() {
        return ObjectGuid::EMPTY;
    }

    ObjectGuid::create_world_object(
        HighGuid::LootObject,
        0,
        owner.realm_id(),
        owner.map_id(),
        0,
        0,
        owner.counter(),
    )
}

/// Unit fixtures that predate canonical map objects may exercise packet-cache
/// behavior locally. This is a compile-time false branch in production: live
/// Creature/GameObject claims fail closed without their map-owned authority.
const fn represented_local_loot_fixture_allowed_like_cpp() -> bool {
    cfg!(test)
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

fn direct_item_count_after_loot_release_like_cpp(
    current_count: u32,
    maximum_destroy_count: Option<u32>,
) -> u32 {
    let destroy_count = maximum_destroy_count
        .unwrap_or(current_count)
        .min(current_count);
    current_count.saturating_sub(destroy_count)
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

/// Completes the shared C++ `Loot::FillLoot` visibility/count state before the
/// generation is published through the object-owned authority. Applying this
/// only to the session cache after publication is unsafe: reconciliation would
/// immediately restore the older authoritative snapshot and lose the tap list.
fn prepare_represented_shared_loot_generation_like_cpp(
    loot: &mut CreatureLoot,
    allowed_looters: &[ObjectGuid],
) {
    for looter in allowed_looters {
        if !looter.is_empty() && !loot.allowed_looters.contains(looter) {
            loot.allowed_looters.push(*looter);
        }
    }
    rebuild_represented_personal_loot_counts_preserving_consumed_like_cpp(loot);
}

fn prepare_represented_shared_creature_loot_generation_like_cpp(
    loot: &mut CreatureLoot,
    allowed_looters: &[ObjectGuid],
) {
    for looter in allowed_looters {
        mark_loot_allowed_for_player_like_cpp(loot, *looter);
    }
    rebuild_represented_personal_loot_counts_preserving_consumed_like_cpp(loot);
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

/// Rebuild a player-scoped authority pool without resurrecting entries already
/// consumed in the session view. The generation helper above intentionally
/// starts fresh; authority synchronization can also run during release, after
/// `taken`/`ffa_looted_by` have already changed.
fn rebuild_represented_personal_loot_counts_preserving_consumed_like_cpp(loot: &mut CreatureLoot) {
    loot.unlooted_count = 0;
    loot.player_ffa_items.clear();

    for entry in &mut loot.items {
        if entry.flags.freeforall {
            entry.flags.counted = false;
            for looter in &entry.allowed_looters {
                let is_looted = entry.ffa_looted_by.contains(looter);
                let item = NotNormalLootItem {
                    loot_list_id: entry.loot_list_id,
                    is_looted,
                };
                match loot
                    .player_ffa_items
                    .iter_mut()
                    .find(|(player, _)| player == looter)
                {
                    Some((_, items)) => items.push(item),
                    None => loot.player_ffa_items.push((*looter, vec![item])),
                }
                if !is_looted {
                    loot.unlooted_count = loot.unlooted_count.saturating_add(1);
                }
            }
        } else {
            entry.flags.counted = !entry.allowed_looters.is_empty();
            if entry.flags.counted && !entry.taken {
                loot.unlooted_count = loot.unlooted_count.saturating_add(1);
            }
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

fn looted_corpse_decay_secs_like_cpp(
    is_fully_skinned: bool,
    corpse_delay_secs: u32,
    ignore_decay_ratio: bool,
    corpse_decay_looted_rate: f32,
) -> u32 {
    if is_fully_skinned {
        return 0;
    }

    let rate = if ignore_decay_ratio {
        1.0
    } else {
        corpse_decay_looted_rate.max(0.0)
    };
    ((corpse_delay_secs as f32) * rate) as u32
}

fn loot_can_be_opened_by_player_like_cpp(loot: &CreatureLoot, player_guid: ObjectGuid) -> bool {
    if loot_is_looted_like_cpp(loot) {
        return false;
    }

    loot_has_item_for_all_like_cpp(loot, player_guid)
        || loot_has_item_for_player_like_cpp(loot, player_guid)
}

/// Exact represented branch order of C++ `Player::isAllowedToLoot` for a
/// creature and the pool selected by `Creature::GetLootForPlayer`.
fn creature_loot_is_allowed_to_player_like_cpp(
    creature_is_dead: bool,
    player_has_pending_bind: bool,
    loot: &CreatureLoot,
    player_guid: ObjectGuid,
) -> bool {
    if !creature_is_dead || player_has_pending_bind || loot_is_looted_like_cpp(loot) {
        return false;
    }
    if !loot.allowed_looters.contains(&player_guid)
        || (!loot_has_item_for_all_like_cpp(loot, player_guid)
            && !loot_has_item_for_player_like_cpp(loot, player_guid))
    {
        return false;
    }

    match loot.loot_method {
        LOOT_METHOD_FREE_FOR_ALL_LIKE_CPP | LOOT_METHOD_PERSONAL_LIKE_CPP => true,
        LOOT_METHOD_ROUND_ROBIN_LIKE_CPP => {
            loot.round_robin_player.is_empty()
                || loot.round_robin_player == player_guid
                || loot_has_item_for_player_like_cpp(loot, player_guid)
        }
        LOOT_METHOD_MASTER_LIKE_CPP
        | LOOT_METHOD_GROUP_LIKE_CPP
        | LOOT_METHOD_NEED_BEFORE_GREED_LIKE_CPP => {
            loot.round_robin_player.is_empty()
                || loot.round_robin_player == player_guid
                || loot_has_over_threshold_item_like_cpp(loot)
                || loot_has_item_for_player_like_cpp(loot, player_guid)
        }
        _ => false,
    }
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CreatureLootReleaseCommandQueueOutcomeLikeCpp {
    Queued,
    Retrying,
    Disconnected,
}

#[cfg(test)]
fn queue_creature_loot_release_command_reliably_like_cpp(
    command_tx: &flume::Sender<SessionCommand>,
    command: SessionCommand,
) -> CreatureLootReleaseCommandQueueOutcomeLikeCpp {
    match command_tx.try_send(command) {
        Ok(()) => CreatureLootReleaseCommandQueueOutcomeLikeCpp::Queued,
        Err(flume::TrySendError::Disconnected(_)) => {
            CreatureLootReleaseCommandQueueOutcomeLikeCpp::Disconnected
        }
        Err(flume::TrySendError::Full(command)) => {
            let command_tx = command_tx.clone();
            // Never await another session from the source session loop: two
            // full queues could otherwise wait on each other forever. The
            // detached retry retains the exact command until capacity opens;
            // receiver-side authority/lifecycle gates coalesce its meaning to
            // the current corpse generation and reject stale respawn reuse.
            tokio::spawn(async move {
                if command_tx.send_async(command).await.is_err() {
                    tracing::debug!(
                        "loot-release DynamicFlags retry ended after target session disconnected"
                    );
                }
            });
            CreatureLootReleaseCommandQueueOutcomeLikeCpp::Retrying
        }
    }
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
    current_instance_id: u32,
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
        let Some(player) = registry.loot_presence(*looter) else {
            continue;
        };
        if player.map_id == current_map_id && player.instance_id == current_instance_id {
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
                    .and_then(|registry| registry.loot_enchanting_skill(*looter))
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
    dynamic_flags: u32,
    random_properties_id: i32,
    random_properties_seed: i32,
    item_context: u8,
}

/// Everything needed to mirror C++ `Player::StoreLootItem`'s post-store wire
/// boundary. SQL and the object-owned claim are settled by the detached
/// worker; the session publishes the stored-item update before choosing the
/// direct-loot or disenchant-specific removal/`ItemPushResult` order.
#[derive(Debug, Clone, Copy)]
struct LootItemClaimCommitContextLikeCpp {
    owner_guid: ObjectGuid,
    loot_obj: ObjectGuid,
    loot_list_id: u8,
    player_guid: ObjectGuid,
    free_for_all: bool,
}

#[derive(Debug, Clone)]
struct PlannedDisenchantExistingStack {
    slot: u8,
    item_guid: ObjectGuid,
    db_guid: u64,
    new_count: u32,
    dynamic_flags: u32,
    flags_changed: bool,
}

#[derive(Debug, Clone)]
struct PlannedDirectLootExistingStack {
    slot: u8,
    item_guid: ObjectGuid,
    db_guid: u64,
    new_count: u32,
    added_count: u32,
    dynamic_flags: u32,
    flags_changed: bool,
}

#[derive(Debug, Clone)]
struct PlannedDisenchantExistingPush {
    slot: u8,
    item_guid: ObjectGuid,
    added_count: u32,
    new_count: u32,
}

#[derive(Debug, Clone)]
struct PlannedDisenchantNewPush {
    stack_index: usize,
    added_count: u32,
    new_count: u32,
}

#[derive(Debug, Clone)]
struct PlannedDisenchantGrant {
    entry: LootEntry,
    random_properties: LootStoreRandomProperties,
    existing_pushes: Vec<PlannedDisenchantExistingPush>,
    new_pushes: Vec<PlannedDisenchantNewPush>,
}

/// Own a loot lease in the same detached task that crosses the durable
/// persistence boundary.  Tokio does not cancel a spawned task when the
/// caller drops its `JoinHandle`, so packet/session cancellation cannot turn a
/// successful SQL commit back into an available object-owned claim.
enum LootClaimPersistenceWorkerError<E> {
    Persistence(E),
    Claim(LootClaimCommitError),
}

impl<E: std::fmt::Debug> std::fmt::Debug for LootClaimPersistenceWorkerError<E> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Persistence(error) => formatter.debug_tuple("Persistence").field(error).finish(),
            Self::Claim(error) => formatter.debug_tuple("Claim").field(error).finish(),
        }
    }
}

/// The stored-money row is the durable single-winner token. Keeping its
/// deletion in the same transaction as the gold update and requiring exactly
/// one affected row turns retries/concurrent handlers into a database CAS.
const STORED_ITEM_MONEY_SOURCE_ROWS_EXPECTED_LIKE_CPP: u64 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct StoredItemMoneyDbOutcomeLikeCpp {
    before: u64,
    after: u64,
    applied_delta: u64,
    notified_amount: u64,
}

#[derive(Debug)]
enum StoredItemMoneyAttemptErrorLikeCpp {
    DefinitelyRolledBack(LootMoneyPersistenceErrorLikeCpp),
    CommitOutcomeUnknown {
        error: DatabaseError,
        outcome: StoredItemMoneyDbOutcomeLikeCpp,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StoredItemMoneyCommitReconciliationLikeCpp {
    Committed,
    RolledBack,
    Indeterminate,
}

fn classify_stored_item_money_commit_reconciliation_like_cpp(
    outcome: StoredItemMoneyDbOutcomeLikeCpp,
    observed_money: u64,
    observed_source_money: Option<u64>,
) -> StoredItemMoneyCommitReconciliationLikeCpp {
    let all_before =
        observed_money == outcome.before && observed_source_money == Some(outcome.notified_amount);
    let all_after = observed_money == outcome.after && observed_source_money.is_none();
    match (all_before, all_after) {
        (true, false) => StoredItemMoneyCommitReconciliationLikeCpp::RolledBack,
        (false, true) => StoredItemMoneyCommitReconciliationLikeCpp::Committed,
        _ => StoredItemMoneyCommitReconciliationLikeCpp::Indeterminate,
    }
}

fn stored_item_money_zero_without_source_outcome_like_cpp(
    before: u64,
    cached_notified_amount: u64,
) -> Option<StoredItemMoneyDbOutcomeLikeCpp> {
    (cached_notified_amount == 0).then_some(StoredItemMoneyDbOutcomeLikeCpp {
        before,
        after: before,
        applied_delta: 0,
        notified_amount: 0,
    })
}

fn stored_item_money_attempt_is_deadlock_like_cpp(
    error: &StoredItemMoneyAttemptErrorLikeCpp,
) -> bool {
    matches!(
        error,
        StoredItemMoneyAttemptErrorLikeCpp::DefinitelyRolledBack(
            LootMoneyPersistenceErrorLikeCpp::Database(error)
        ) if is_database_deadlock_like_cpp(error)
    )
}

async fn attempt_stored_item_money_transaction_like_cpp(
    char_db: &CharacterDatabase,
    player_guid: ObjectGuid,
    item_guid: ObjectGuid,
    cached_notified_amount: u64,
) -> Result<StoredItemMoneyDbOutcomeLikeCpp, StoredItemMoneyAttemptErrorLikeCpp> {
    let definitely = |error| {
        StoredItemMoneyAttemptErrorLikeCpp::DefinitelyRolledBack(
            LootMoneyPersistenceErrorLikeCpp::Database(DatabaseError::from(error)),
        )
    };
    // `SELECT ... FOR UPDATE` inside the transaction means this cannot be an
    // `SqlTransaction`, so the ambient hook never sees it. Recorded explicitly
    // or the whole durable operation is missing from a trace that still looks
    // complete.
    let mut transaction = match char_db.pool().begin().await {
        Ok(transaction) => transaction,
        Err(error) => {
            // No connection, so nothing was attempted. Recorded rather than
            // returning silently: an empty trace makes a definite
            // non-execution indistinguishable from the workflow never being
            // reached, and only one of those is safe to retry.
            wow_database::persistence_trace::record_batch_not_started(
                wow_database::persistence_trace::LogicalDatabase::Character,
            );
            return Err(definitely(error));
        }
    };
    // Guarded for its whole lifetime: every early return through `?` drops the
    // transaction, SQLx rolls it back, and the guard records that end — including
    // for returns added later, which a per-site hook would miss.
    let mut trace = wow_database::persistence_trace::ExplicitTransactionTrace::open(
        wow_database::persistence_trace::LogicalDatabase::Character,
    );
    // Global order shared with group payouts: character mutation mutex, then
    // character row, then the stored Item source row.
    trace.statement(|| {
        (
            CharStatements::SEL_CHAR_MONEY_FOR_UPDATE.trace_identity(),
            vec![wow_database::persistence_trace::TracedParam::Uint {
                value: player_guid.counter() as u64,
                width_bits: 64,
            }],
        )
    });
    let before = sqlx::query_scalar::<_, u64>(CharStatements::SEL_CHAR_MONEY_FOR_UPDATE.sql())
        .bind(player_guid.counter() as u64)
        .fetch_optional(&mut *transaction)
        .await
        .map_err(definitely)?
        .ok_or_else(|| {
            StoredItemMoneyAttemptErrorLikeCpp::DefinitelyRolledBack(
                LootMoneyPersistenceErrorLikeCpp::MissingPlayer,
            )
        })?;
    trace.statement(|| {
        (
            CharStatements::SEL_ITEMCONTAINER_MONEY_FOR_UPDATE.trace_identity(),
            vec![wow_database::persistence_trace::TracedParam::Uint {
                value: item_guid.counter() as u64,
                width_bits: 64,
            }],
        )
    });
    let source_money =
        sqlx::query_scalar::<_, u64>(CharStatements::SEL_ITEMCONTAINER_MONEY_FOR_UPDATE.sql())
            .bind(item_guid.counter() as u64)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(definitely)?;
    let Some(notified_amount) = source_money else {
        if let Some(outcome) =
            stored_item_money_zero_without_source_outcome_like_cpp(before, cached_notified_amount)
        {
            {
                trace.rolled_back();
                transaction
            }
            .rollback()
            .await
            .map_err(definitely)?;
            return Ok(outcome);
        }
        return Err(StoredItemMoneyAttemptErrorLikeCpp::DefinitelyRolledBack(
            LootMoneyPersistenceErrorLikeCpp::Database(DatabaseError::Transaction(
                "stored Item money source was already consumed".to_string(),
            )),
        ));
    };
    let (after, applied_delta) = loot_money_durable_outcome_like_cpp(before, notified_amount);
    let outcome = StoredItemMoneyDbOutcomeLikeCpp {
        before,
        after,
        applied_delta,
        notified_amount,
    };

    if applied_delta != 0 {
        // The durable credit itself. Recording only the preceding SELECTs left
        // the mutation invisible, so removing or reordering it would not have
        // moved the trace at all.
        trace.statement_expecting(
            || {
                (
                    CharStatements::UPD_CHAR_MONEY.trace_identity(),
                    vec![
                        wow_database::persistence_trace::TracedParam::Uint {
                            value: after,
                            width_bits: 64,
                        },
                        wow_database::persistence_trace::TracedParam::Uint {
                            value: player_guid.counter() as u64,
                            width_bits: 64,
                        },
                    ],
                )
            },
            1,
        );
        let result = sqlx::query(CharStatements::UPD_CHAR_MONEY.sql())
            .bind(after)
            .bind(player_guid.counter() as u64)
            .execute(&mut *transaction)
            .await
            .map_err(definitely)?;
        if result.rows_affected() != 1 {
            return Err(StoredItemMoneyAttemptErrorLikeCpp::DefinitelyRolledBack(
                LootMoneyPersistenceErrorLikeCpp::Database(DatabaseError::Transaction(format!(
                    "stored Item money update affected {} rows; expected exactly 1",
                    result.rows_affected()
                ))),
            ));
        }
    }
    // Consuming the source is the other half of the durable operation.
    trace.statement_expecting(
        || {
            (
                CharStatements::DEL_ITEMCONTAINER_MONEY.trace_identity(),
                vec![wow_database::persistence_trace::TracedParam::Uint {
                    value: item_guid.counter() as u64,
                    width_bits: 64,
                }],
            )
        },
        STORED_ITEM_MONEY_SOURCE_ROWS_EXPECTED_LIKE_CPP,
    );
    let delete = sqlx::query(CharStatements::DEL_ITEMCONTAINER_MONEY.sql())
        .bind(item_guid.counter() as u64)
        .execute(&mut *transaction)
        .await
        .map_err(definitely)?;
    if delete.rows_affected() != STORED_ITEM_MONEY_SOURCE_ROWS_EXPECTED_LIKE_CPP {
        return Err(StoredItemMoneyAttemptErrorLikeCpp::DefinitelyRolledBack(
            LootMoneyPersistenceErrorLikeCpp::Database(DatabaseError::Transaction(format!(
                "stored Item money source delete affected {} rows; expected exactly 1",
                delete.rows_affected()
            ))),
        ));
    }

    // Announced before the await: a cancellation in this window means COMMIT
    // was issued and its answer never came, which is not a rollback.
    trace.committing();
    match transaction.commit().await {
        Ok(()) => {
            trace.committed(wow_database::persistence_trace::CommitOutcome::Committed);
            Ok(outcome)
        }
        Err(error) => {
            let error = DatabaseError::from(error);
            trace.committed(if is_database_deadlock_like_cpp(&error) {
                wow_database::persistence_trace::CommitOutcome::RolledBack
            } else {
                wow_database::persistence_trace::CommitOutcome::Unknown
            });
            if is_database_deadlock_like_cpp(&error) {
                Err(StoredItemMoneyAttemptErrorLikeCpp::DefinitelyRolledBack(
                    LootMoneyPersistenceErrorLikeCpp::Database(error),
                ))
            } else {
                Err(StoredItemMoneyAttemptErrorLikeCpp::CommitOutcomeUnknown { error, outcome })
            }
        }
    }
}

async fn reconcile_stored_item_money_commit_like_cpp(
    char_db: &CharacterDatabase,
    player_guid: ObjectGuid,
    item_guid: ObjectGuid,
    outcome: StoredItemMoneyDbOutcomeLikeCpp,
) -> Result<StoredItemMoneyCommitReconciliationLikeCpp, DatabaseError> {
    // `SELECT ... FOR UPDATE` inside the transaction means this cannot be an
    // `SqlTransaction`, so the ambient hook never sees it. Recorded explicitly
    // or the whole durable operation is missing from a trace that still looks
    // complete.
    let mut transaction = match char_db.pool().begin().await {
        Ok(transaction) => transaction,
        Err(error) => return Err(DatabaseError::from(error)),
    };
    // Guarded for its whole lifetime: every early return through `?` drops the
    // transaction, SQLx rolls it back, and the guard records that end — including
    // for returns added later, which a per-site hook would miss.
    let mut trace = wow_database::persistence_trace::ExplicitTransactionTrace::open(
        wow_database::persistence_trace::LogicalDatabase::Character,
    );
    // Read and lock both facts in the same order as the original mutation.
    // The per-character mutation mutex is still held, so a later local payout
    // cannot manufacture a mixed observation while COMMIT is reconciled.
    trace.statement(|| {
        (
            CharStatements::SEL_CHAR_MONEY_FOR_UPDATE.trace_identity(),
            vec![wow_database::persistence_trace::TracedParam::Uint {
                value: player_guid.counter() as u64,
                width_bits: 64,
            }],
        )
    });
    let observed_money =
        sqlx::query_scalar::<_, u64>(CharStatements::SEL_CHAR_MONEY_FOR_UPDATE.sql())
            .bind(player_guid.counter() as u64)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(DatabaseError::from)?
            .ok_or_else(|| {
                DatabaseError::Transaction("stored-money character vanished".to_string())
            })?;
    trace.statement(|| {
        (
            CharStatements::SEL_ITEMCONTAINER_MONEY_FOR_UPDATE.trace_identity(),
            vec![wow_database::persistence_trace::TracedParam::Uint {
                value: item_guid.counter() as u64,
                width_bits: 64,
            }],
        )
    });
    let observed_source_money =
        sqlx::query_scalar::<_, u64>(CharStatements::SEL_ITEMCONTAINER_MONEY_FOR_UPDATE.sql())
            .bind(item_guid.counter() as u64)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(DatabaseError::from)?;
    let classification = classify_stored_item_money_commit_reconciliation_like_cpp(
        outcome,
        observed_money,
        observed_source_money,
    );
    // A read-only reconciliation ends in a rollback by design, so this is its
    // normal termination rather than a failure.
    trace.rolled_back();
    transaction.rollback().await.map_err(DatabaseError::from)?;
    Ok(classification)
}

fn queue_stored_item_money_indeterminate_kick_like_cpp(command_tx: &flume::Sender<SessionCommand>) {
    let kick = SessionCommand::KickLikeCpp(KickLikeCppCommand {
        reason: "stored Item money COMMIT outcome is unknown; relog required".to_string(),
    });
    if let Err(error) = command_tx.try_send(kick) {
        let kick = error.into_inner();
        let command_tx = command_tx.clone();
        tokio::spawn(async move {
            let _ = command_tx.send_async(kick).await;
        });
    }
}

fn spawn_loot_claim_persistence_worker_like_cpp<F, E>(
    persistence: F,
    claim: Option<LootClaimLease>,
    durable_item_completion: Option<(
        DurableItemLootPersistenceGuardLikeCpp,
        DurableItemLootCompletionLikeCpp,
    )>,
) -> Result<
    tokio::task::JoinHandle<Result<(), LootClaimPersistenceWorkerError<E>>>,
    LootClaimCommitError,
>
where
    F: std::future::Future<Output = Result<(), E>> + Send + 'static,
    E: Send + 'static,
{
    let persistence_guard = claim
        .as_ref()
        .map(LootClaimLease::begin_persistence_guard_like_cpp)
        .transpose()?;
    drop(claim);
    Ok(tokio::spawn(async move {
        let mut durable_item_completion = durable_item_completion;
        persistence
            .await
            .map_err(LootClaimPersistenceWorkerError::Persistence)?;
        if let Some(mut guard) = persistence_guard {
            let (_, committed_snapshot) = guard
                .commit_with_snapshot_like_cpp()
                .map_err(LootClaimPersistenceWorkerError::Claim)?;
            if let (Some(snapshot), Some((_, completion))) =
                (committed_snapshot, durable_item_completion.as_ref())
                && let Some(fanout) = completion.item_fanout.as_ref()
            {
                // Publish the serialization cut before exposing the durable
                // completion to the session. Sampling the authority later can
                // include an opener that already saw the consumed slot.
                let _ = fanout.committed_snapshot.set(snapshot);
            }
        }
        if let Some((guard, completion)) = durable_item_completion.as_mut() {
            guard.mark_committed_like_cpp(completion.clone());
        }
        Ok(())
    }))
}

/// Outcome-aware SQL variant for consume-and-grant item transactions. A
/// transport failure returned by COMMIT cannot be treated as rollback: the
/// old object allocation is quarantined permanently and the player is kicked
/// to reload whichever durable state MySQL ultimately kept.
fn spawn_sql_loot_claim_persistence_worker_like_cpp<F>(
    persistence: F,
    claim: Option<LootClaimLease>,
    durable_item_completion: Option<(
        DurableItemLootPersistenceGuardLikeCpp,
        DurableItemLootCompletionLikeCpp,
    )>,
    command_tx: flume::Sender<SessionCommand>,
) -> Result<
    tokio::task::JoinHandle<Result<(), LootClaimPersistenceWorkerError<SqlTransactionCommitError>>>,
    LootClaimCommitError,
>
where
    F: std::future::Future<Output = Result<(), SqlTransactionCommitError>> + Send + 'static,
{
    let mut persistence_guard = claim
        .as_ref()
        .map(LootClaimLease::begin_persistence_guard_like_cpp)
        .transpose()?;
    drop(claim);
    Ok(tokio::spawn(async move {
        let mut durable_item_completion = durable_item_completion;
        match persistence.await {
            Ok(()) => {}
            Err(error @ SqlTransactionCommitError::DefinitelyRolledBack(_)) => {
                return Err(LootClaimPersistenceWorkerError::Persistence(error));
            }
            Err(error @ SqlTransactionCommitError::CommitOutcomeUnknown(_)) => {
                if let Some(guard) = persistence_guard.as_mut() {
                    let _ = guard.quarantine_commit_unknown_like_cpp();
                }
                let kick = SessionCommand::KickLikeCpp(KickLikeCppCommand {
                    reason: "loot item COMMIT outcome is unknown; relog required".to_string(),
                });
                if let Err(send_error) = command_tx.try_send(kick) {
                    let kick = send_error.into_inner();
                    tokio::spawn(async move {
                        let _ = command_tx.send_async(kick).await;
                    });
                }
                return Err(LootClaimPersistenceWorkerError::Persistence(error));
            }
        }
        if let Some(mut guard) = persistence_guard {
            let (_, committed_snapshot) = guard
                .commit_with_snapshot_like_cpp()
                .map_err(LootClaimPersistenceWorkerError::Claim)?;
            if let (Some(snapshot), Some((_, completion))) =
                (committed_snapshot, durable_item_completion.as_ref())
                && let Some(fanout) = completion.item_fanout.as_ref()
            {
                let _ = fanout.committed_snapshot.set(snapshot);
            }
        }
        if let Some((guard, completion)) = durable_item_completion.as_mut() {
            guard.mark_committed_like_cpp(completion.clone());
        }
        Ok(())
    }))
}

#[cfg(test)]
#[path = "loot_tests.rs"]
mod tests;
