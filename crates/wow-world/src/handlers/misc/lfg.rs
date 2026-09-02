// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Private lfg capability handlers extracted from the legacy misc owner.

use tracing::{info, warn};
use wow_constants::ClientOpcodes;
use wow_constants::unit::Team;
use wow_handler::{PacketProcessing, SessionStatus};

use crate::session::registry::PacketHandlerEntry;
use wow_packet::ClientPacket;
use wow_packet::packets::misc::{
    DfGetJoinStatus, DfGetSystemInfo, LfgBlackList, LfgListBlacklist, LfgListBlacklistEntry,
    LfgPlayerDungeonInfo, LfgPlayerInfo, LfgPlayerQuestRewardCurrency, LfgPlayerQuestRewardItem,
    LfgUpdateStatus,
};

use super::{
    LFG_LOCKSTATUS_INSUFFICIENT_EXPANSION_LIKE_CPP, LFG_LOCKSTATUS_MISSING_ACHIEVEMENT_LIKE_CPP,
    LFG_LOCKSTATUS_MISSING_ITEM_LIKE_CPP, LFG_LOCKSTATUS_NOT_IN_SEASON_LIKE_CPP,
    LFG_LOCKSTATUS_QUEST_NOT_COMPLETED_LIKE_CPP, LFG_LOCKSTATUS_RAID_LOCKED_LIKE_CPP,
    LFG_LOCKSTATUS_TOO_HIGH_LEVEL_LIKE_CPP, LFG_LOCKSTATUS_TOO_LOW_GEAR_SCORE_LIKE_CPP,
    LFG_LOCKSTATUS_TOO_LOW_LEVEL_LIKE_CPP,
};

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::DfGetSystemInfo,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_df_get_system_info",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_df_get_system_info(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::DfGetJoinStatus,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadSafe,
        handler_name: "handle_df_get_join_status",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_df_get_join_status(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RequestConquestFormulaConstants,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::Inplace,
        handler_name: "handle_request_conquest_formula_constants",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_request_conquest_formula_constants(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::RequestLfgListBlacklist,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_request_lfg_list_blacklist",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_request_lfg_list_blacklist(pkt).await })
        },
    }
}

inventory::submit! {
    PacketHandlerEntry {
        opcode: ClientOpcodes::LfgListGetStatus,
        status: SessionStatus::LoggedIn,
        processing: PacketProcessing::ThreadUnsafe,
        handler_name: "handle_lfg_list_get_status",
        handler: |session, pkt| {
            Box::pin(async move { session.handle_lfg_list_get_status(pkt).await })
        },
    }
}

impl crate::session::WorldSession {
    pub async fn handle_df_get_system_info(&mut self, mut pkt: wow_packet::WorldPacket) {
        let request = match DfGetSystemInfo::read(&mut pkt) {
            Ok(request) => request,
            Err(error) => {
                warn!(
                    account = self.account_id,
                    "DFGetSystemInfo parse failed: {error}"
                );
                return;
            }
        };

        if request.player {
            if let Some(info) = self.lfg_player_lock_info_like_cpp() {
                self.send_packet(&info);
            }
        } else {
            // C++ `SendLfgPartyLockInfo` returns before sending when the player
            // is not in a group. Rust does not expose a live LFG group manager
            // here yet, so the no-group branch remains silent.
        }
    }

    fn lfg_player_lock_info_like_cpp(&self) -> Option<LfgPlayerInfo> {
        let Some(store) = self.lfg_dungeon_store_like_cpp() else {
            return Some(LfgPlayerInfo::empty());
        };

        let level = self.player_level_like_cpp();
        let expansion = self.expansion;
        let current_item_level = self.represented_average_item_level_like_cpp()?.max(0.0) as i32;

        let mut info = LfgPlayerInfo {
            blacklist: LfgBlackList::default(),
            dungeons: Vec::new(),
        };

        for dungeon_id in store.locked_dungeon_ids_like_cpp() {
            let Some(dungeon) = store.get(dungeon_id) else {
                continue;
            };
            if self.map_store().is_some_and(|map_store| {
                !wow_data::lfg_dungeon_is_known_map_like_cpp(dungeon, map_store)
            }) {
                continue;
            }
            if dungeon.type_id == wow_data::LFG_TYPE_RANDOM_LIKE_CPP
                && (dungeon.min_level > level || dungeon.max_level < level)
            {
                continue;
            }
            if let Some(reason) = self.lfg_lock_status_like_cpp(dungeon, level, expansion) {
                info.blacklist.slots.push(LfgListBlacklistEntry {
                    slot: dungeon.entry_like_cpp(),
                    reason,
                    sub_reason1: i32::from(dungeon.required_item_level),
                    sub_reason2: current_item_level,
                    soft_lock: 0,
                });
            }
        }
        info.blacklist.slots.sort_unstable_by_key(|lock| lock.slot);

        for slot in store.random_and_active_seasonal_dungeon_entries_like_cpp(
            level,
            expansion,
            |dungeon_id| self.lfg_season_is_active_like_cpp(dungeon_id),
        ) {
            let mut dungeon_info = LfgPlayerDungeonInfo::random_dungeon_like_cpp(slot);
            if let Some(reward) = store.random_dungeon_reward_like_cpp(slot, level) {
                self.populate_lfg_player_dungeon_reward_like_cpp(&mut dungeon_info, reward);
            }
            info.dungeons.push(dungeon_info);
        }

        Some(info)
    }

    fn lfg_season_is_active_like_cpp(&self, _dungeon_id: u32) -> bool {
        // C++ delegates this to `LFGMgr::IsSeasonActive`, backed by holiday
        // state. The current Rust runtime has no live holiday manager wired
        // into LFG yet; inactive is the C++-safe default for seasonal rows.
        false
    }

    pub(super) fn lfg_lock_status_like_cpp(
        &self,
        dungeon: &wow_data::LfgDungeonDataLikeCpp,
        level: u8,
        expansion: u8,
    ) -> Option<u32> {
        if dungeon.expansion > expansion {
            return Some(LFG_LOCKSTATUS_INSUFFICIENT_EXPANSION_LIKE_CPP);
        }
        if self.lfg_is_disabled_map_type_for_player_like_cpp(
            wow_data::DISABLE_TYPE_MAP,
            dungeon.map,
            dungeon.difficulty,
        ) {
            return Some(LFG_LOCKSTATUS_NOT_IN_SEASON_LIKE_CPP);
        }
        if self.lfg_is_disabled_map_type_for_player_like_cpp(
            wow_data::DISABLE_TYPE_LFG_MAP,
            dungeon.map,
            dungeon.difficulty,
        ) {
            return Some(LFG_LOCKSTATUS_RAID_LOCKED_LIKE_CPP);
        }
        if self.lfg_has_active_instance_lock_like_cpp(dungeon.map, dungeon.difficulty) {
            return Some(LFG_LOCKSTATUS_RAID_LOCKED_LIKE_CPP);
        }
        if dungeon.min_level > level {
            return Some(LFG_LOCKSTATUS_TOO_LOW_LEVEL_LIKE_CPP);
        }
        if dungeon.max_level < level {
            return Some(LFG_LOCKSTATUS_TOO_HIGH_LEVEL_LIKE_CPP);
        }
        if dungeon.seasonal && !self.lfg_season_is_active_like_cpp(dungeon.id) {
            return Some(LFG_LOCKSTATUS_NOT_IN_SEASON_LIKE_CPP);
        }
        let Some(current_item_level) = self.represented_average_item_level_like_cpp() else {
            return Some(LFG_LOCKSTATUS_TOO_LOW_GEAR_SCORE_LIKE_CPP);
        };
        if f32::from(dungeon.required_item_level) > current_item_level {
            return Some(LFG_LOCKSTATUS_TOO_LOW_GEAR_SCORE_LIKE_CPP);
        }
        if let Some(requirement) = self
            .access_requirement_store()
            .and_then(|store| store.get(dungeon.map, dungeon.difficulty))
        {
            if requirement.completed_achievement != 0
                && !self.access_requirement_leader_has_achievement_like_cpp(
                    requirement.completed_achievement,
                )
            {
                return Some(LFG_LOCKSTATUS_MISSING_ACHIEVEMENT_LIKE_CPP);
            }

            match crate::session::player_team_for_race_cpp(self.player_race_like_cpp()) {
                Team::Alliance
                    if requirement.quest_done_a != 0
                        && !self.player_quest_gameplay_snapshot_like_cpp().is_some_and(
                            |state| state.rewarded_quest_ids.contains(&requirement.quest_done_a),
                        ) =>
                {
                    return Some(LFG_LOCKSTATUS_QUEST_NOT_COMPLETED_LIKE_CPP);
                }
                Team::Horde
                    if requirement.quest_done_h != 0
                        && !self.player_quest_gameplay_snapshot_like_cpp().is_some_and(
                            |state| state.rewarded_quest_ids.contains(&requirement.quest_done_h),
                        ) =>
                {
                    return Some(LFG_LOCKSTATUS_QUEST_NOT_COMPLETED_LIKE_CPP);
                }
                _ => {}
            }

            if requirement.item != 0 {
                if !self.represented_has_item_count_like_cpp(requirement.item, 1)
                    && (requirement.item2 == 0
                        || !self.represented_has_item_count_like_cpp(requirement.item2, 1))
                {
                    return Some(LFG_LOCKSTATUS_MISSING_ITEM_LIKE_CPP);
                }
            } else if requirement.item2 != 0
                && !self.represented_has_item_count_like_cpp(requirement.item2, 1)
            {
                return Some(LFG_LOCKSTATUS_MISSING_ITEM_LIKE_CPP);
            }
        }
        None
    }

    fn lfg_is_disabled_map_type_for_player_like_cpp(
        &self,
        disable_type: u32,
        map_id: u32,
        dungeon_difficulty: u8,
    ) -> bool {
        let Some(disable_mgr) = self.disable_mgr() else {
            return false;
        };
        let Some(map_store) = self.map_store() else {
            return false;
        };

        let current_map_id = u32::from(self.player_map_id_like_cpp());
        let (_, area_id) = self.player_zone_area_like_cpp();
        let current_map_instance_type = map_store
            .get(current_map_id)
            .map(|entry| entry.instance_type);

        disable_mgr.is_disabled_for_like_cpp(
            disable_type,
            map_id,
            Some(wow_data::DisableWorldObjectRefLikeCpp {
                type_id: wow_constants::TypeId::Player,
                map_id: current_map_id,
                area_id,
                is_pet: false,
                is_battle_arena: current_map_instance_type == Some(wow_data::MAP_ARENA_LIKE_CPP),
                is_battleground: current_map_instance_type
                    == Some(wow_data::MAP_BATTLEGROUND_LIKE_CPP),
                player_map_difficulty: Some(dungeon_difficulty),
            }),
            0,
            Some(map_store.as_ref()),
        )
    }

    pub(super) fn populate_lfg_player_dungeon_reward_like_cpp(
        &self,
        dungeon_info: &mut LfgPlayerDungeonInfo,
        reward: &wow_data::LfgDungeonRewardLikeCpp,
    ) {
        let Some(quest_store) = self.quest_store.as_ref() else {
            return;
        };
        let Some(mut quest) = quest_store.get(reward.first_quest_id) else {
            return;
        };

        dungeon_info.first_reward = self.can_reward_lfg_quest_like_cpp(quest, false);
        if std::env::var_os("RUSTYCORE_LFG_TRACE").is_some() {
            info!(
                slot = dungeon_info.slot,
                first_quest_id = reward.first_quest_id,
                other_quest_id = reward.other_quest_id,
                special_flags = quest.special_flags,
                is_df = quest.is_df_quest_like_cpp(),
                df_done = self
                    .player_quest_gameplay_snapshot_like_cpp()
                    .is_some_and(|state| state.df_quest_ids.contains(&quest.id)),
                first_reward = dungeon_info.first_reward,
                "RUST_LFG_TRACE reward decision"
            );
        }
        if !dungeon_info.first_reward {
            if reward.other_quest_id == 0 {
                return;
            }
            let Some(other_quest) = quest_store.get(reward.other_quest_id) else {
                return;
            };
            quest = other_quest;
        }

        dungeon_info.rewards.reward_money = self.quest_money_reward_like_cpp(quest) as i32;
        dungeon_info.rewards.reward_xp = self.quest_xp_reward_like_cpp(quest) as i32;

        for (idx, &item_id) in quest.reward_items.iter().enumerate() {
            if item_id == 0 {
                continue;
            }
            dungeon_info.rewards.items.push(LfgPlayerQuestRewardItem {
                item_id: item_id as i32,
                quantity: quest.reward_amounts.get(idx).copied().unwrap_or(0) as i32,
            });
        }

        for (idx, &currency_id) in quest.reward_currencies.iter().enumerate() {
            if currency_id == 0 {
                continue;
            }
            dungeon_info
                .rewards
                .currency
                .push(LfgPlayerQuestRewardCurrency {
                    currency_id: currency_id as i32,
                    quantity: quest.reward_currency_amounts.get(idx).copied().unwrap_or(0) as i32,
                });
        }
    }

    fn can_reward_lfg_quest_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
        _msg: bool,
    ) -> bool {
        let Some(recurrence) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return false;
        };
        if !quest.is_df_quest_like_cpp()
            && !quest.is_turn_in_like_cpp()
            && recurrence.statuses.get(&quest.id).is_none_or(|status| {
                status.status != crate::conditions::QUEST_STATUS_COMPLETE_LIKE_CPP
            })
        {
            return false;
        }
        if quest.is_df_quest_like_cpp() {
            return !recurrence.df_quest_ids.contains(&quest.id);
        }
        if quest.is_daily_like_cpp() && recurrence.daily_quest_ids.contains(&quest.id) {
            return false;
        }
        if quest.is_weekly_like_cpp() && recurrence.weekly_quest_ids.contains(&quest.id) {
            return false;
        }
        if quest.is_monthly_like_cpp() && recurrence.monthly_quest_ids.contains(&quest.id) {
            return false;
        }
        if quest.is_seasonal_like_cpp()
            && recurrence
                .seasonal_quests
                .get(&quest.event_id_for_quest_like_cpp())
                .is_some_and(|quests| quests.contains_key(&quest.id))
        {
            return false;
        }

        !recurrence.rewarded_quest_ids.contains(&quest.id)
    }

    pub async fn handle_df_get_join_status(&mut self, mut pkt: wow_packet::WorldPacket) {
        if let Err(error) = DfGetJoinStatus::read(&mut pkt) {
            warn!(
                account = self.account_id,
                "DFGetJoinStatus parse failed: {error}"
            );
            return;
        }

        // C++ `HandleDFGetJoinStatus` returns before sending anything when
        // `Player::isUsingLfg()` is false. Rust has no represented active LFG
        // join state in this handler yet, so preserve that observable branch.
    }

    pub async fn handle_request_conquest_formula_constants(
        &mut self,
        _pkt: wow_packet::WorldPacket,
    ) {
        // C++ registers CMSG_REQUEST_CONQUEST_FORMULA_CONSTANTS as
        // STATUS_UNHANDLED/Handle_NULL.
    }

    pub async fn handle_request_lfg_list_blacklist(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ builds this from `sLFGMgr->GetLockedDungeons(playerGuid)`.
        // Rust does not have that manager state yet, so represent the
        // well-defined no-locks response instead of leaving the client waiting.
        self.send_packet_realm(&LfgListBlacklist::empty());
    }

    pub async fn handle_lfg_list_get_status(&mut self, _pkt: wow_packet::WorldPacket) {
        // C++ `HandleLfgListGetStatus` always sends LFGUpdateStatus for a live
        // player. Until `sLFGMgr` state is ported, Rust represents the
        // well-defined no-ticket/no-queue branch.
        self.send_packet_realm(&LfgUpdateStatus::removed_from_queue());
    }
}
