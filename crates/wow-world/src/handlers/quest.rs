// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest system handlers.
//!
//! Implements:
//!   CMSG_QUEST_GIVER_STATUS_QUERY  → SMSG_QUEST_GIVER_STATUS
//!   CMSG_QUEST_GIVER_HELLO         → SMSG_QUEST_GIVER_QUEST_LIST_MESSAGE
//!   CMSG_QUEST_GIVER_QUERY_QUEST   → SMSG_QUEST_GIVER_QUEST_DETAILS
//!   CMSG_QUEST_GIVER_ACCEPT_QUEST  → save to DB + SMSG_QUEST_GIVER_QUEST_COMPLETE
//!   CMSG_QUEST_LOG_REMOVE_QUEST    → remove from DB
//!   CMSG_QUERY_QUEST_INFO          → SMSG_QUERY_QUEST_INFO_RESPONSE
//!
//! C# ref: Game/Handlers/QuestHandler.cs

use std::sync::Arc;
use tracing::{debug, info, warn};
use wow_constants::ClientOpcodes;
use wow_handler::{PacketHandlerEntry, PacketProcessing, SessionStatus};
use wow_packet::packets::quest::{
    quest_giver_status, QuestGiverOfferReward, QuestGiverQuestComplete, QuestGiverQuestDetails,
    QuestGiverQuestList, QuestGiverRequestItems, QuestGiverStatus, QuestListEntry,
    QuestObjectiveInfo, QuestObjectiveSimple, QuestRewardsBlock, QueryQuestInfoResponse,
    QuestUpdateComplete,
};
use wow_packet::ServerPacket;

use crate::session::WorldSession;

// ── Handler registrations ────────────────────────────────────────────────────

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverStatusQuery,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_quest_giver_status_query",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverHello,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_quest_giver_hello",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverQueryQuest,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_quest_giver_query_quest",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverAcceptQuest,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_quest_giver_accept_quest",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestLogRemoveQuest,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_quest_log_remove_quest",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QueryQuestInfo,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_query_quest_info",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverRequestReward,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_quest_giver_request_reward",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverCompleteQuest,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_quest_giver_complete_quest",
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::QuestGiverChooseReward,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_quest_giver_choose_reward",
    }
}

// ── Handler implementations ──────────────────────────────────────────────────

impl WorldSession {
    /// CMSG_QUEST_GIVER_STATUS_QUERY — returns the quest status icon for an NPC.
    /// C# ref: QuestHandler.HandleQuestGiverStatusQuery
    pub async fn handle_quest_giver_status_query(&mut self, mut pkt: wow_packet::WorldPacket) {
        let guid = match pkt.read_packed_guid() {
            Ok(g) => g,
            Err(_) => { warn!("QuestGiverStatusQuery: failed to read GUID"); return; }
        };

        let npc_entry = guid.entry();
        let status = self.get_quest_giver_status(npc_entry);

        debug!(
            account = self.account_id,
            npc_entry = npc_entry,
            status = status,
            "QuestGiverStatus"
        );

        self.send_packet(&QuestGiverStatus { guid, status });
    }

    /// CMSG_QUEST_GIVER_HELLO — player right-clicks a quest NPC.
    /// Opens the quest list dialog for this NPC.
    /// C# ref: QuestHandler.HandleQuestGiverHello
    pub async fn handle_quest_giver_hello(&mut self, mut pkt: wow_packet::WorldPacket) {
        let guid = match pkt.read_packed_guid() {
            Ok(g) => g,
            Err(_) => { warn!("QuestGiverHello: failed to read GUID"); return; }
        };

        let npc_entry = guid.entry();
        let quest_store = match &self.quest_store {
            Some(s) => Arc::clone(s),
            None => { debug!("No quest store"); return; }
        };

        let available = quest_store.quests_for_starter(npc_entry);
        let quests: Vec<QuestListEntry> = available.iter()
            .filter(|q| self.can_take_quest(q))
            .map(|q| QuestListEntry {
                quest_id: q.id,
                quest_type: q.quest_type,
                quest_level: q.quest_level,
                quest_max_scaling_level: q.quest_max_scaling_level,
                quest_flags: q.flags,
                quest_flags_ex: q.flags_ex,
                repeatable: q.is_repeatable(),
                title: q.log_title.clone(),
            })
            .collect();

        if quests.is_empty() {
            debug!(account = self.account_id, npc_entry, "NPC has no available quests");
            return;
        }

        info!(
            account = self.account_id,
            npc_entry = npc_entry,
            count = quests.len(),
            "Sending quest list"
        );

        self.send_packet(&QuestGiverQuestList {
            guid,
            greeting: String::new(),
            greet_emote_delay: 0,
            greet_emote_type: 0,
            quests,
        });
    }

    /// CMSG_QUEST_GIVER_QUERY_QUEST — player clicks a quest name in the list.
    /// Shows full quest details (objectives, rewards) before accepting.
    /// C# ref: QuestHandler.HandleQuestGiverQueryQuest
    pub async fn handle_quest_giver_query_quest(&mut self, mut pkt: wow_packet::WorldPacket) {
        let guid = match pkt.read_packed_guid() {
            Ok(g) => g,
            Err(_) => { warn!("QuestGiverQueryQuest: failed to read GUID"); return; }
        };
        let quest_id: u32 = pkt.read_uint32().unwrap_or(0);
        let _resend_offer: bool = pkt.read_uint8().unwrap_or(0) != 0;

        let quest_store = match &self.quest_store {
            Some(s) => Arc::clone(s),
            None => return,
        };

        let quest = match quest_store.get(quest_id) {
            Some(q) => q,
            None => {
                warn!(account = self.account_id, quest_id, "QuestGiverQueryQuest: unknown quest");
                return;
            }
        };

        let objectives: Vec<QuestObjectiveSimple> = quest.objectives.iter().map(|obj| {
            QuestObjectiveSimple {
                id: obj.id,
                object_id: obj.object_id,
                amount: obj.amount,
                obj_type: obj.obj_type,
            }
        }).collect();

        let mut rewards = QuestRewardsBlock::default();
        rewards.money = quest.reward_money_difficulty as i32;
        for i in 0..4 {
            rewards.items[i] = (quest.reward_items[i], quest.reward_amounts[i]);
        }
        for i in 0..3 {
            rewards.display_spells[i] = quest.reward_display_spell[i];
        }
        rewards.completion_spell = quest.reward_spell as i32;

        self.send_packet(&QuestGiverQuestDetails {
            giver_guid: guid,
            quest_id,
            quest_flags: [quest.flags, quest.flags_ex, quest.flags_ex2],
            suggested_party_members: quest.suggested_group_num,
            objectives,
            rewards,
            title: quest.log_title.clone(),
            description: quest.quest_description.clone(),
            log_description: quest.log_description.clone(),
            auto_launched: false,
        });
    }

    /// CMSG_QUEST_GIVER_ACCEPT_QUEST — player clicks "Accept" in the quest details dialog.
    /// Saves quest to characters DB and confirms to the client.
    /// C# ref: QuestHandler.HandleQuestGiverAcceptQuest
    pub async fn handle_quest_giver_accept_quest(&mut self, mut pkt: wow_packet::WorldPacket) {
        let _guid = match pkt.read_packed_guid() {
            Ok(g) => g,
            Err(_) => { warn!("QuestGiverAcceptQuest: failed to read GUID"); return; }
        };
        let quest_id: u32 = pkt.read_uint32().unwrap_or(0);
        let _start_cheat: bool = pkt.read_uint8().unwrap_or(0) != 0;

        // Validate quest exists
        let quest_store = match &self.quest_store {
            Some(s) => Arc::clone(s),
            None => return,
        };
        if quest_store.get(quest_id).is_none() {
            warn!(account = self.account_id, quest_id, "AcceptQuest: unknown quest");
            return;
        }

        // Full eligibility check: SatisfyQuestStatus + PrevQuestId + race/class/level
        // C# ref: Player.CanTakeQuest(quest, true)
        if let Some(quest) = quest_store.get(quest_id) {
            if !self.can_take_quest(quest) {
                warn!(
                    account = self.account_id, quest_id,
                    race = self.player_race, class = self.player_class, level = self.player_level,
                    "AcceptQuest: player does not meet requirements (CanTakeQuest failed)"
                );
                return;
            }
        }

        // Check quest limit (max 25 active quests — C# SharedConst.MaxQuestLogSize)
        if self.player_quests.len() >= 25 {
            warn!(account = self.account_id, "Quest log full");
            return;
        }

        // Build objective counts (one slot per objective)
        let obj_count = quest_store.get(quest_id).map_or(0, |q| q.objectives.len());

        // Add to local state
        self.player_quests.insert(quest_id, PlayerQuestStatus {
            quest_id,
            status: 1, // Incomplete
            explored: false,
            objective_counts: vec![0; obj_count],
        });

        // Save to DB
        self.save_quest_to_db(quest_id, 1).await;

        info!(account = self.account_id, quest_id, "Quest accepted");

        // Notify client — quest added popup
        self.send_packet(&QuestGiverQuestComplete {
            quest_id,
            xp: 0,
            money: 0,
            skill_points: 0,
            use_quest_reward_currency: false,
        });
    }

    /// CMSG_QUEST_LOG_REMOVE_QUEST — player abandons a quest from the quest log.
    /// C# ref: QuestHandler.HandleQuestLogRemoveQuest
    pub async fn handle_quest_log_remove_quest(&mut self, mut pkt: wow_packet::WorldPacket) {
        let slot: u8 = pkt.read_uint8().unwrap_or(255);

        // In a full implementation we'd track quest IDs by slot.
        // For now, read quest_id from the packet (TrinityCore sends it as well).
        // The slot-to-quest mapping comes from the quest log order.
        // We iterate to find the nth quest.
        let quest_id = self.player_quests.keys()
            .nth(slot as usize)
            .copied();

        if let Some(qid) = quest_id {
            self.player_quests.remove(&qid);
            self.delete_quest_from_db(qid).await;
            info!(account = self.account_id, quest_id = qid, slot, "Quest abandoned");
        } else {
            warn!(account = self.account_id, slot, "QuestLogRemoveQuest: slot not found");
        }
    }

    /// CMSG_QUERY_QUEST_INFO — client asks for full quest template data by ID.
    /// Used to populate the quest log and tooltip.
    /// C# ref: QuestHandler.HandleQueryQuestInfo
    pub async fn handle_query_quest_info(&mut self, mut pkt: wow_packet::WorldPacket) {
        let quest_id: u32 = pkt.read_uint32().unwrap_or(0);
        let _guid = pkt.read_packed_guid(); // requester GUID (usually player)

        let quest_store = match &self.quest_store {
            Some(s) => Arc::clone(s),
            None => {
                self.send_packet(&QueryQuestInfoResponse {
                    quest_id, allow: false,
                    ..Default::default()
                });
                return;
            }
        };

        match quest_store.get(quest_id) {
            None => {
                self.send_packet(&QueryQuestInfoResponse {
                    quest_id, allow: false,
                    ..Default::default()
                });
            }
            Some(quest) => {
                let objectives: Vec<QuestObjectiveInfo> = quest.objectives.iter().map(|obj| {
                    QuestObjectiveInfo {
                        id: obj.id,
                        obj_type: obj.obj_type,
                        storage_index: obj.storage_index,
                        object_id: obj.object_id,
                        amount: obj.amount,
                        flags: obj.flags,
                        flags2: obj.flags2,
                        progress_bar_weight: obj.progress_bar_weight,
                        description: obj.description.clone(),
                    }
                }).collect();

                self.send_packet(&QueryQuestInfoResponse {
                    quest_id,
                    allow: true,
                    quest_type: quest.quest_type,
                    quest_level: quest.quest_level,
                    quest_max_scaling_level: quest.quest_max_scaling_level,
                    min_level: quest.min_level,
                    quest_sort_id: quest.quest_sort_id,
                    quest_info_id: quest.quest_info_id,
                    suggested_group_num: quest.suggested_group_num,
                    reward_next_quest: quest.reward_next_quest,
                    reward_xp_difficulty: quest.reward_xp_difficulty,
                    reward_money_difficulty: quest.reward_money_difficulty,
                    flags: quest.flags,
                    flags_ex: quest.flags_ex,
                    flags_ex2: quest.flags_ex2,
                    reward_items: quest.reward_items,
                    reward_amounts: quest.reward_amounts,
                    reward_display_spell: quest.reward_display_spell,
                    reward_spell: quest.reward_spell,
                    objectives,
                    log_title: quest.log_title.clone(),
                    log_description: quest.log_description.clone(),
                    quest_description: quest.quest_description.clone(),
                    area_description: quest.area_description.clone(),
                    quest_completion_log: quest.quest_completion_log.clone(),
                });
            }
        }
    }

    /// CMSG_QUEST_GIVER_REQUEST_REWARD — player talks to NPC to turn in a completed quest.
    /// C# ref: QuestHandler.HandleQuestgiverRequestReward
    /// Sent when player right-clicks a quest-ender NPC and has the quest in Complete status.
    /// Server responds with SMSG_QUEST_GIVER_OFFER_REWARD_MESSAGE (reward selection dialog).
    pub async fn handle_quest_giver_request_reward(&mut self, mut pkt: wow_packet::WorldPacket) {
        let guid = match pkt.read_packed_guid() {
            Ok(g) => g,
            Err(_) => { warn!("QuestGiverRequestReward: failed to read GUID"); return; }
        };
        let quest_id: u32 = pkt.read_uint32().unwrap_or(0);

        let quest = {
            let store = match &self.quest_store {
                Some(s) => Arc::clone(s),
                None => return,
            };
            match store.get(quest_id) {
                Some(q) => q.clone(),
                None => {
                    warn!(account = self.account_id, quest_id, "RequestReward: unknown quest");
                    return;
                }
            }
        };

        // C#: if (GetPlayer().CanCompleteQuest(questID)) GetPlayer().CompleteQuest(questID)
        // We check if all objectives are done; if so, upgrade status to Complete (2).
        let is_complete = self.player_quests.get(&quest_id)
            .map_or(false, |qs| qs.status == 2);

        if !is_complete {
            // Objectives not finished — silently ignore
            // (C# would send SMSG_QUEST_GIVER_REQUEST_ITEMS instead)
            debug!(account = self.account_id, quest_id, "RequestReward: quest not complete");
            return;
        }

        // Build rewards block for the offer-reward dialog
        let mut rewards = QuestRewardsBlock::default();
        rewards.money = quest.reward_money_difficulty as i32;
        for i in 0..4 {
            rewards.items[i] = (quest.reward_items[i], quest.reward_amounts[i]);
        }
        for i in 0..3 {
            rewards.display_spells[i] = quest.reward_display_spell[i];
        }
        rewards.completion_spell = quest.reward_spell as i32;
        // Populate choice items for the dialog
        for i in 0..6 {
            rewards.choice_items[i] = (quest.reward_choice_items[i].0, quest.reward_choice_items[i].1);
        }

        // C#: SendQuestGiverOfferReward(quest, questGiverGUID, true)
        self.send_packet(&QuestGiverOfferReward {
            giver_guid: guid,
            quest_id,
            quest_flags: [quest.flags, quest.flags_ex, quest.flags_ex2],
            suggested_party_members: quest.suggested_group_num,
            rewards,
            title: quest.log_title.clone(),
            reward_text: quest.quest_completion_log.clone(),
            auto_launched: false,
        });
    }

    /// CMSG_QUEST_GIVER_COMPLETE_QUEST — player talks to quest-ender NPC.
    /// If objectives are done: show reward dialog. Else: show "still need X" dialog.
    /// C# ref: QuestHandler.HandleQuestGiverCompleteQuest
    pub async fn handle_quest_giver_complete_quest(&mut self, mut pkt: wow_packet::WorldPacket) {
        let guid = match pkt.read_packed_guid() {
            Ok(g) => g,
            Err(_) => { warn!("QuestGiverCompleteQuest: failed to read GUID"); return; }
        };
        let quest_id: u32 = pkt.read_uint32().unwrap_or(0);
        let _from_script: bool = pkt.read_bit().unwrap_or(false);

        let quest_store = match &self.quest_store {
            Some(s) => Arc::clone(s),
            None => return,
        };

        let quest = match quest_store.get(quest_id) {
            Some(q) => q,
            None => {
                warn!(account = self.account_id, quest_id, "QuestGiverCompleteQuest: unknown quest");
                return;
            }
        };

        // Check if player has the quest active
        if !self.has_quest(quest_id) {
            debug!(account = self.account_id, quest_id, "Player doesn't have quest");
            return;
        }

        // Build rewards block
        let mut rewards = QuestRewardsBlock::default();
        rewards.money = quest.reward_money_difficulty as i32;
        for i in 0..4 {
            rewards.items[i] = (quest.reward_items[i], quest.reward_amounts[i]);
        }
        for i in 0..3 {
            rewards.display_spells[i] = quest.reward_display_spell[i];
        }
        rewards.completion_spell = quest.reward_spell as i32;

        // Check if all objectives are done — C# GetQuestStatus == QuestStatus.Complete
        let is_complete = self.player_quests.get(&quest_id)
            .map_or(false, |qs| qs.status == 2);

        if !is_complete {
            // Not all objectives done — send "you still need X" dialog
            // C# ref: SendQuestGiverRequestItems(quest, guid, canComplete=false, false)
            self.send_packet(&QuestGiverRequestItems {
                giver_guid: guid,
                quest_id,
                quest_flags: [quest.flags, quest.flags_ex, quest.flags_ex2],
                suggested_party_members: quest.suggested_group_num,
                status_flags: 0,
                money_cost: 0,
                title: quest.log_title.clone(),
                completion_text: quest.area_description.clone(),
            });
            return;
        }

        // All objectives done — show offer reward dialog
        self.send_packet(&QuestGiverOfferReward {
            giver_guid: guid,
            quest_id,
            quest_flags: [quest.flags, quest.flags_ex, quest.flags_ex2],
            suggested_party_members: quest.suggested_group_num,
            rewards,
            title: quest.log_title.clone(),
            reward_text: quest.quest_completion_log.clone(),
            auto_launched: false,
        });
    }

    /// CMSG_QUEST_GIVER_CHOOSE_REWARD — player clicks "Complete Quest" in reward dialog.
    /// Gives XP, gold, items. Removes quest from active log.
    /// C# ref: QuestHandler.HandleQuestGiverChooseReward
    pub async fn handle_quest_giver_choose_reward(&mut self, mut pkt: wow_packet::WorldPacket) {
        let _guid = pkt.read_packed_guid();
        let quest_id: u32       = pkt.read_uint32().unwrap_or(0);
        let choice_item_id: u32 = pkt.read_uint32().unwrap_or(0);
        let _loot_item_type: u32 = pkt.read_uint32().unwrap_or(0); // 0=Item, 1=Currency

        let quest = {
            let store = match &self.quest_store {
                Some(s) => Arc::clone(s),
                None => return,
            };
            match store.get(quest_id) {
                Some(q) => q.clone(),
                None => {
                    warn!(account = self.account_id, quest_id, "ChooseReward: unknown quest");
                    return;
                }
            }
        };

        // SatisfyQuestStatus — C# HandleQuestgiverChooseReward line ~370
        // Player must have the quest active AND it must be complete (status=2)
        let quest_status = self.player_quests.get(&quest_id).map(|qs| qs.status);
        match quest_status {
            Some(2) => {} // Complete — ok
            Some(1) => {
                warn!(account = self.account_id, quest_id, "ChooseReward: quest not complete yet");
                return;
            }
            _ => {
                warn!(account = self.account_id, quest_id, "ChooseReward: player doesn't have quest");
                return;
            }
        }

        // Validate choice item — C# HandleQuestgiverChooseReward lines 255-310
        // If client sends a non-zero choice item, it must be in reward_choice_items.
        if choice_item_id != 0 {
            let valid = quest.reward_choice_items.iter()
                .any(|(item_id, _qty)| *item_id == choice_item_id);
            if !valid {
                warn!(
                    account = self.account_id, quest_id, choice_item_id,
                    "ChooseReward: choice item not valid for this quest (possible exploit)"
                );
                return;
            }
        }

        // Give gold reward
        let money = quest.reward_money_difficulty;
        if money > 0 {
            self.player_gold = self.player_gold.saturating_add(money as u64);
            self.save_player_gold().await;
        }

        let xp = self.calculate_quest_xp(quest.reward_xp_difficulty, quest.quest_level);

        // RewardQuest — C# Player.RewardQuest:
        // 1. Remove from active quest log
        // 2. Mark as rewarded (status=3) in DB and in memory
        // Non-repeatable quests go into rewarded_quests; repeatable quests stay removed
        self.player_quests.remove(&quest_id);
        if !quest.is_repeatable() {
            self.rewarded_quests.insert(quest_id);
            self.save_quest_to_db(quest_id, 3).await; // status=3 = Rewarded
        } else {
            self.delete_quest_from_db(quest_id).await;
        }

        info!(
            account = self.account_id,
            quest_id, xp, gold = money,
            repeatable = quest.is_repeatable(),
            "Quest rewarded"
        );

        // SMSG_QUEST_GIVER_QUEST_COMPLETE — reward popup with XP/gold
        self.send_packet(&QuestGiverQuestComplete {
            quest_id,
            xp,
            money,
            skill_points: 0,
            use_quest_reward_currency: false,
        });

        // SMSG_QUEST_UPDATE_COMPLETE — removes from quest log UI
        self.send_packet(&QuestUpdateComplete { quest_id });

        // Give XP reward — C# Player.RewardQuest → GiveXP
        if xp > 0 {
            let player_guid = self.player_guid.unwrap_or(wow_core::ObjectGuid::new(0, 0));
            self.give_xp(xp, player_guid, false).await;
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    /// Returns the quest giver status for an NPC (controls the ! ? icon above its head).
    fn get_quest_giver_status(&self, npc_entry: u32) -> u64 {
        let Some(store) = &self.quest_store else { return quest_giver_status::NONE; };

        // Check if NPC ends any quest the player has completed (blue ?)
        // C# ref: GetQuestDialogStatus → QuestGiverStatus.Reward
        let has_turn_in = store.quests_for_ender(npc_entry)
            .iter()
            .any(|q| {
                self.player_quests.get(&q.id)
                    .map_or(false, |qs| qs.status == 2)
            });

        if has_turn_in {
            return quest_giver_status::CAN_REWARD; // blue ?
        }

        // Check if NPC starts any quest the player can take (yellow !)
        let has_available = store.quests_for_starter(npc_entry)
            .iter()
            .any(|q| self.can_take_quest(q));

        if has_available {
            quest_giver_status::AVAILABLE // yellow !
        } else {
            quest_giver_status::NONE
        }
    }

    /// Check if the player currently has an active quest with the given ID.
    pub fn has_quest(&self, quest_id: u32) -> bool {
        self.player_quests.contains_key(&quest_id)
    }

    /// Full eligibility check before accepting a quest.
    /// C# ref: Player.CanTakeQuest (SatisfyQuestStatus + SatisfyQuestRace/Class/Level + PrevQuest)
    pub fn can_take_quest(&self, quest: &wow_data::quest::QuestTemplate) -> bool {
        // SatisfyQuestStatus — C# lines 1624-1654
        // If quest is already rewarded (non-repeatable), cannot take again.
        if self.rewarded_quests.contains(&quest.id) && !quest.is_repeatable() {
            debug!(account = self.account_id, quest_id = quest.id, "CanTakeQuest: already rewarded");
            return false;
        }
        // If quest is already active, cannot accept again.
        if self.player_quests.contains_key(&quest.id) {
            debug!(account = self.account_id, quest_id = quest.id, "CanTakeQuest: already active");
            return false;
        }

        // SatisfyQuestPreviousQuest — C# lines 1415-1440
        // prev_quest_id > 0 → previous quest must have been rewarded
        // prev_quest_id < 0 → previous quest must be currently active (Incomplete)
        if quest.prev_quest_id != 0 {
            let prev_id = quest.prev_quest_id.unsigned_abs();
            if quest.prev_quest_id > 0 {
                if !self.rewarded_quests.contains(&prev_id) {
                    debug!(
                        account = self.account_id,
                        quest_id = quest.id, prev_id,
                        "CanTakeQuest: prev quest not rewarded"
                    );
                    return false;
                }
            } else {
                // negative: prev quest must be active
                let active = self.player_quests.get(&prev_id)
                    .map_or(false, |qs| qs.status == 1);
                if !active {
                    debug!(
                        account = self.account_id,
                        quest_id = quest.id, prev_id,
                        "CanTakeQuest: negative prev quest not active"
                    );
                    return false;
                }
            }
        }

        // SatisfyQuestRace + SatisfyQuestClass + SatisfyQuestLevel
        quest.is_available_for(self.player_race, self.player_class, self.player_level)
    }

    /// Save quest status to the characters database.
    async fn save_quest_to_db(&self, quest_id: u32, status: u8) {
        use wow_database::CharStatements;

        let guid = match self.player_guid {
            Some(g) => g.counter() as u32,
            None => return,
        };
        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };

        let mut stmt = char_db.prepare(CharStatements::INS_CHAR_QUEST_STATUS);
        stmt.set_u32(0, guid);
        stmt.set_u32(1, quest_id);
        stmt.set_u8(2, status);
        stmt.set_u8(3, status);  // ON DUPLICATE KEY UPDATE status
        stmt.set_u8(4, 0);       // explored

        if let Err(e) = char_db.execute(&stmt).await {
            warn!(account = self.account_id, quest_id, "Failed to save quest status: {e}");
        }
    }

    /// Delete a quest from the characters database (abandon).
    async fn delete_quest_from_db(&self, quest_id: u32) {
        use wow_database::CharStatements;

        let guid = match self.player_guid {
            Some(g) => g.counter() as u32,
            None => return,
        };
        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };

        let mut stmt = char_db.prepare(CharStatements::DEL_CHAR_QUEST_STATUS);
        stmt.set_u32(0, guid);
        stmt.set_u32(1, quest_id);

        if let Err(e) = char_db.execute(&stmt).await {
            warn!(account = self.account_id, quest_id, "Failed to delete quest: {e}");
        }
    }

    /// Load all active quests for this player from the characters DB.
    pub(crate) async fn load_player_quests(&mut self) {
        use wow_database::CharStatements;

        let guid = match self.player_guid {
            Some(g) => g.counter() as u32,
            None => return,
        };
        let char_db = match self.char_db() {
            Some(db) => Arc::clone(db),
            None => return,
        };

        let mut stmt = char_db.prepare(CharStatements::SEL_CHAR_QUEST_STATUS);
        stmt.set_u32(0, guid);

        let result = match char_db.query(&stmt).await {
            Ok(r) => r,
            Err(e) => {
                warn!(account = self.account_id, "Failed to load quest status: {e}");
                return;
            }
        };

        self.player_quests.clear();
        self.rewarded_quests.clear();

        if !result.is_empty() {
            let mut result = result;
            loop {
                let quest_id: u32  = result.try_read::<u32>(0).unwrap_or(0);
                let status: u8     = result.try_read::<u8>(1).unwrap_or(0);
                let explored: bool = result.try_read::<u8>(2).unwrap_or(0) != 0;

                if status == 3 {
                    // Rewarded (C# QuestStatus.Rewarded / m_RewardedQuests)
                    // Non-repeatable quests cannot be re-taken once rewarded.
                    self.rewarded_quests.insert(quest_id);
                } else {
                    // Active (1=Incomplete) or complete-but-not-turned-in (2=Complete)
                    let obj_count = self.quest_store.as_ref()
                        .and_then(|s| s.get(quest_id))
                        .map_or(0, |q| q.objectives.len());
                    self.player_quests.insert(quest_id, PlayerQuestStatus {
                        quest_id,
                        status,
                        explored,
                        objective_counts: vec![0; obj_count],
                    });
                }

                if !result.next_row() { break; }
            }
        }

        info!(
            account = self.account_id,
            active = self.player_quests.len(),
            rewarded = self.rewarded_quests.len(),
            "Loaded player quests"
        );
    }
}

// ── PlayerQuestStatus ────────────────────────────────────────────────────────

/// Tracks one active quest for a player.
#[derive(Debug, Clone)]
pub struct PlayerQuestStatus {
    pub quest_id: u32,
    /// 0=None, 1=Incomplete, 2=Complete, 3=Failed
    pub status: u8,
    pub explored: bool,
    /// Progress per objective (indexed by objective.storage_index).
    /// value = current count toward the required amount.
    pub objective_counts: Vec<i32>,
}
