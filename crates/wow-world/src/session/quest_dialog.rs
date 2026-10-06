// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest dialog: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{ActiveState, Arc, InventoryResult, ObjectGuid};
use super::{QUEST_MENU_ICON_AVAILABLE_LIKE_CPP, QUEST_MENU_ICON_COMPLETE_LIKE_CPP};
use super::{QUEST_MENU_ICON_TURN_IN_LIKE_CPP, QuestListEntry};
use super::{QuestRewardsBlock, RepresentedGameObjectUseEffect};
use super::{WorldSession, info, quest};

pub(in crate::session) fn quest_has_represented_item_objective_like_cpp(
    quest: &wow_data::quest::QuestTemplate,
) -> bool {
    wow_world_application::represented_quest_has_item_objective_like_cpp(quest)
}

pub(in crate::session) fn quest_rewards_block_like_cpp(
    quest: &wow_data::quest::QuestTemplate,
) -> QuestRewardsBlock {
    wow_world_application::represented_quest_rewards_block_like_cpp(quest)
}

pub(in crate::session) fn quest_giver_creature_id_from_source_like_cpp(
    source_guid: ObjectGuid,
) -> i32 {
    if source_guid.is_any_type_creature() {
        i32::try_from(source_guid.entry()).unwrap_or(0)
    } else {
        0
    }
}

pub(in crate::session) fn represented_quest_menu_item_log_rows_like_cpp(
    menu_items: &[RepresentedPreparedQuestMenuItemLikeCpp],
) -> Vec<(u32, String, u8, bool, bool, u32, u32)> {
    menu_items
        .iter()
        .map(|item| {
            (
                item.quest.id,
                item.quest.log_title.clone(),
                item.quest_icon,
                item.has_starter_relation,
                item.has_involved_relation,
                item.quest.allowable_classes,
                item.quest.flags,
            )
        })
        .collect()
}

pub(in crate::session) use wow_world_core::session::react_state_from_db_like_cpp;

pub(in crate::session) const fn active_state_from_db_like_cpp(value: u8) -> ActiveState {
    match value {
        0x00 => ActiveState::Decide,
        0x01 => ActiveState::Passive,
        0x81 => ActiveState::Disabled,
        0xC1 => ActiveState::Enabled,
        _ => ActiveState::Disabled,
    }
}

pub(in crate::session) use wow_world_core::session::power_type_from_u8_like_cpp;

pub(in crate::session) use wow_world_entities::{
    sheath_state_from_u8_like_cpp, unit_stand_state_from_u8_like_cpp,
};

#[derive(Debug, Clone)]
pub(in crate::session) struct RepresentedPreparedQuestMenuItemLikeCpp {
    pub(in crate::session) quest: wow_data::quest::QuestTemplate,
    pub(in crate::session) quest_icon: u8,
    pub(in crate::session) has_starter_relation: bool,
    pub(in crate::session) has_involved_relation: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ResetSeasonalQuestStatusReasonLikeCpp {
    MissingEvent,
    EmptyEvent,
    RemovedOlderCompletions,
    NoOlderCompletions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ResetSeasonalQuestStatusOutcomeLikeCpp {
    pub event_id: u16,
    pub event_start_time: u64,
    pub reason: ResetSeasonalQuestStatusReasonLikeCpp,
    pub removed_quest_ids: Vec<u32>,
    pub completed_bit_cleared: usize,
    pub completed_bit_skipped_no_quest_v2_store: usize,
    pub completed_bit_skipped_zero_unique_bit: usize,
    pub completed_bit_no_change_or_noop: usize,
    pub completed_bit_clear_unrepresented: usize,
    pub event_bucket_erased: bool,
    pub seasonal_quest_changed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SeasonalQuestStatusDbRowLikeCpp {
    pub quest_id: u32,
    pub event_id: u32,
    pub completed_time: i64,
}

pub(crate) use wow_world_application::{
    RepresentedPendingQuestSharingLikeCpp, RepresentedPushQuestToPartyOutcomeLikeCpp,
    RepresentedPushQuestToPartyOutcomeReasonLikeCpp, RepresentedQuestCompleteStatusUpdateLikeCpp,
    RepresentedQuestConfirmAcceptLikeCpp, RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp,
    RepresentedQuestPushResultResponseLikeCpp,
};
pub(crate) use wow_world_instances::RepresentedAdventureMapStartQuestLikeCpp;

#[cfg(any(test, feature = "test-fixtures"))]
pub(crate) use wow_world_application::{
    RepresentedQuestRewardMailLikeCpp, RepresentedQuestRewardReputationLikeCpp,
    RepresentedQuestRewardSpellCastLikeCpp, RepresentedQuestRewardSpellKindLikeCpp,
    RepresentedQuestRewardTalentPointsLikeCpp, RepresentedQuestRewardTitleLikeCpp,
};

#[cfg(any(test, feature = "test-fixtures"))]
pub(crate) use wow_world_social::RepresentedForceDeselectLikeCpp;

pub(crate) use wow_world_application::RepresentedQuestRewardReputationSourceLikeCpp;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct LoadSeasonalQuestStatusOutcomeLikeCpp {
    pub rows_seen: usize,
    pub inserted: usize,
    pub replaced: usize,
    pub skipped_no_quest_store: usize,
    pub skipped_missing_quest: usize,
    pub skipped_event_out_of_range: usize,
    pub skipped_negative_completed_time: usize,
    pub completed_bit_set: usize,
    pub completed_bit_skipped_no_quest_v2_store: usize,
    pub completed_bit_skipped_zero_unique_bit: usize,
    pub completed_bit_no_change_or_noop: usize,
    pub seasonal_quest_changed: bool,
}

impl WorldSession {
    pub(crate) fn use_represented_creature_questgiver_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
        creature_entry: u32,
    ) -> bool {
        let Some(quest_store) = self.catalogs.quests.store.as_ref().map(Arc::clone) else {
            return false;
        };

        let menu_items =
            self.represented_creature_quest_menu_items_like_cpp(&quest_store, creature_entry);
        let Some(quests) = self.player_quest_gameplay_snapshot_like_cpp() else {
            return false;
        };
        let ender_candidates = quest_store
            .quests_for_ender(creature_entry)
            .iter()
            .map(|quest| {
                (
                    quest.id,
                    quest.log_title.clone(),
                    quest.allowable_races,
                    quest.allowable_classes,
                    quest.min_level,
                    quest.max_level,
                    quests
                        .statuses_like_cpp()
                        .get(&quest.id)
                        .map(|status| status.status),
                    quests.rewarded_quest_ids_like_cpp().contains(&quest.id),
                )
            })
            .collect::<Vec<_>>();
        let starter_candidates = quest_store
            .quests_for_starter(creature_entry)
            .iter()
            .map(|quest| {
                (
                    quest.id,
                    quest.log_title.clone(),
                    quest.allowable_races,
                    quest.allowable_classes,
                    quest.min_level,
                    quest.max_level,
                    quest.is_available_for(
                        crate::session::hub_ref(self).player_race_like_cpp(),
                        crate::session::hub_ref(self).player_class_like_cpp(),
                        crate::session::hub_ref(self).player_level_like_cpp(),
                    ),
                    self.can_take_quest(quest),
                    quests
                        .statuses_like_cpp()
                        .get(&quest.id)
                        .map(|status| status.status),
                    quests.rewarded_quest_ids_like_cpp().contains(&quest.id),
                )
            })
            .collect::<Vec<_>>();
        info!(
            creature_entry,
            race = crate::session::hub_ref(self).player_race_like_cpp(),
            class = crate::session::hub_ref(self).player_class_like_cpp(),
            level = crate::session::hub_ref(self).player_level_like_cpp(),
            ender_candidates = ?ender_candidates,
            starter_candidates = ?starter_candidates,
            menu_items = ?represented_quest_menu_item_log_rows_like_cpp(&menu_items),
            "Prepared creature questgiver fallback menu like C++"
        );
        if menu_items.is_empty() {
            return false;
        }

        self.send_represented_prepared_quest_like_cpp(creature_guid, menu_items);
        true
    }

    pub(crate) fn use_represented_gameobject_questgiver_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
        gameobject_entry: u32,
        source: wow_entities::QuestgiverUseSource,
    ) -> bool {
        self.world_entities
            .record_represented_gameobject_use_effect_like_cpp(
                RepresentedGameObjectUseEffect::SendGossip {
                    gameobject_guid,
                    player_guid,
                    gossip_id: source.gossip_id,
                },
            );

        let Some(quest_store) = self.catalogs.quests.store.as_ref().map(Arc::clone) else {
            return true;
        };

        let menu_items =
            self.represented_gameobject_quest_menu_items_like_cpp(&quest_store, gameobject_entry);
        if !menu_items.is_empty() {
            self.send_represented_prepared_quest_like_cpp(gameobject_guid, menu_items);
        }

        true
    }

    pub(in crate::session) fn represented_quest_giver_query_creature_menu_item_like_cpp(
        &self,
        quest_store: &wow_data::quest::QuestStore,
        creature_entry: u32,
        quest: &wow_data::quest::QuestTemplate,
    ) -> Option<RepresentedPreparedQuestMenuItemLikeCpp> {
        if quest_store.creature_has_ender_relation_like_cpp(creature_entry, quest.id) {
            if self.represented_player_quest_status_is_complete_or_incomplete_like_cpp(quest.id) {
                return Some(RepresentedPreparedQuestMenuItemLikeCpp {
                    quest: quest.clone(),
                    quest_icon: QUEST_MENU_ICON_COMPLETE_LIKE_CPP,
                    has_starter_relation: false,
                    has_involved_relation: true,
                });
            }
            // C++ `HandleQuestgiverQueryQuestOpcode` accepts hasQuest OR
            // hasInvolvedQuest; `PrepareQuestMenu` still checks starters after
            // skipping inactive involved quests on the same source.
        }

        if quest_store.creature_has_starter_relation_like_cpp(creature_entry, quest.id) {
            return self.represented_starter_quest_menu_item_like_cpp(quest);
        }

        None
    }

    pub(in crate::session) fn represented_quest_giver_query_gameobject_menu_item_like_cpp(
        &self,
        quest_store: &wow_data::quest::QuestStore,
        gameobject_entry: u32,
        quest: &wow_data::quest::QuestTemplate,
    ) -> Option<RepresentedPreparedQuestMenuItemLikeCpp> {
        if quest_store.gameobject_has_ender_relation_like_cpp(gameobject_entry, quest.id) {
            if self.represented_player_quest_status_is_complete_or_incomplete_like_cpp(quest.id) {
                return Some(RepresentedPreparedQuestMenuItemLikeCpp {
                    quest: quest.clone(),
                    quest_icon: QUEST_MENU_ICON_COMPLETE_LIKE_CPP,
                    has_starter_relation: false,
                    has_involved_relation: true,
                });
            }
            // C++ `HandleQuestgiverQueryQuestOpcode` accepts hasQuest OR
            // hasInvolvedQuest; `PrepareQuestMenu` still checks starters after
            // skipping inactive involved quests on the same source.
        }

        if quest_store.gameobject_has_starter_relation_like_cpp(gameobject_entry, quest.id) {
            return self.represented_starter_quest_menu_item_like_cpp(quest);
        }

        None
    }

    pub(in crate::session) fn represented_creature_quest_menu_items_like_cpp(
        &self,
        quest_store: &wow_data::quest::QuestStore,
        creature_entry: u32,
    ) -> Vec<RepresentedPreparedQuestMenuItemLikeCpp> {
        let mut menu_items = Vec::new();

        // C++ `Player::PrepareQuestMenu`: involved/ender relations first, icon 4 for
        // represented COMPLETE/INCOMPLETE local quest status.
        for quest in quest_store.quests_for_ender(creature_entry) {
            if self.represented_player_quest_status_is_complete_or_incomplete_like_cpp(quest.id) {
                menu_items.push(RepresentedPreparedQuestMenuItemLikeCpp {
                    quest: quest.clone(),
                    quest_icon: QUEST_MENU_ICON_COMPLETE_LIKE_CPP,
                    has_starter_relation: false,
                    has_involved_relation: true,
                });
            }
        }

        // Starter relations second, preserving C++ quest-icon selection for later
        // `SendPreparedQuest` single-item auto-open.
        for quest in quest_store.quests_for_starter(creature_entry) {
            if let Some(menu_item) = self.represented_starter_quest_menu_item_like_cpp(quest) {
                menu_items.push(menu_item);
            }
        }

        menu_items
    }

    pub(in crate::session) fn represented_gameobject_quest_menu_items_like_cpp(
        &self,
        quest_store: &wow_data::quest::QuestStore,
        gameobject_entry: u32,
    ) -> Vec<RepresentedPreparedQuestMenuItemLikeCpp> {
        let mut menu_items = Vec::new();

        // C++ `Player::PrepareQuestMenu`: GO involved/ender relations first, then
        // starters; do not contaminate GameObject sources with Creature relations.
        for quest in quest_store.quests_for_gameobject_ender(gameobject_entry) {
            if self.represented_player_quest_status_is_complete_or_incomplete_like_cpp(quest.id) {
                menu_items.push(RepresentedPreparedQuestMenuItemLikeCpp {
                    quest: quest.clone(),
                    quest_icon: QUEST_MENU_ICON_COMPLETE_LIKE_CPP,
                    has_starter_relation: false,
                    has_involved_relation: true,
                });
            }
        }

        for quest in quest_store.quests_for_gameobject_starter(gameobject_entry) {
            if let Some(menu_item) = self.represented_starter_quest_menu_item_like_cpp(quest) {
                menu_items.push(menu_item);
            }
        }

        menu_items
    }

    pub(in crate::session) fn represented_starter_quest_menu_item_like_cpp(
        &self,
        quest: &wow_data::quest::QuestTemplate,
    ) -> Option<RepresentedPreparedQuestMenuItemLikeCpp> {
        if !self.can_take_quest(quest) {
            return None;
        }

        let quest_icon = if quest.is_turn_in_like_cpp()
            && (!quest.is_repeatable()
                || quest.is_daily_like_cpp()
                || quest.is_weekly_like_cpp()
                || quest.is_monthly_like_cpp())
        {
            QUEST_MENU_ICON_TURN_IN_LIKE_CPP
        } else if quest.is_turn_in_like_cpp() {
            QUEST_MENU_ICON_COMPLETE_LIKE_CPP
        } else if self.represented_player_quest_status_is_none_like_cpp(quest.id) {
            QUEST_MENU_ICON_AVAILABLE_LIKE_CPP
        } else {
            return None;
        };

        Some(RepresentedPreparedQuestMenuItemLikeCpp {
            quest: quest.clone(),
            quest_icon,
            has_starter_relation: true,
            has_involved_relation: false,
        })
    }

    pub(in crate::session) fn quest_list_entry_from_menu_item_like_cpp(
        &self,
        menu_item: &RepresentedPreparedQuestMenuItemLikeCpp,
    ) -> QuestListEntry {
        let quest = &menu_item.quest;
        QuestListEntry {
            quest_id: quest.id,
            // C++ `PlayerMenu::SendQuestGiverQuestListMessage` writes
            // `QuestMenuItem::QuestIcon`, not `Quest::GetQuestType`.
            quest_type: menu_item.quest_icon,
            quest_level: 0,
            quest_max_scaling_level: 0,
            quest_flags: quest.flags,
            quest_flags_ex: quest.flags_ex,
            repeatable: quest.is_turn_in_like_cpp()
                && quest.is_repeatable()
                && !quest.is_daily_or_weekly_like_cpp()
                && !quest.is_monthly_like_cpp(),
            important: self.represented_quest_is_important_like_cpp(quest),
            title: quest.log_title.clone(),
        }
    }
}
