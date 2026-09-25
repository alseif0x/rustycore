// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest dialog: private Session responsibility.
//! Relocated under #1233; canonical state, phase order and public paths are unchanged.

use super::{ActiveState, Arc, InventoryResult, ObjectGuid, PetType, PowerType};
use super::{QUEST_MENU_ICON_AVAILABLE_LIKE_CPP, QUEST_MENU_ICON_COMPLETE_LIKE_CPP};
use super::{QUEST_MENU_ICON_TURN_IN_LIKE_CPP, QUEST_OBJECTIVE_ITEM_LIKE_CPP, QuestListEntry};
use super::{QuestRewardsBlock, ReactState, RepresentedGameObjectUseEffect, SheathState};
use super::{UnitStandStateType, WorldSession, info, quest};

pub(in crate::session) fn quest_has_represented_item_objective_like_cpp(
    quest: &wow_data::quest::QuestTemplate,
) -> bool {
    quest
        .objectives
        .iter()
        .any(|objective| objective.obj_type == QUEST_OBJECTIVE_ITEM_LIKE_CPP)
}

pub(in crate::session) fn quest_rewards_block_like_cpp(
    quest: &wow_data::quest::QuestTemplate,
) -> QuestRewardsBlock {
    let mut rewards = QuestRewardsBlock {
        money: quest.reward_money_difficulty as i32,
        completion_spell: quest.reward_spell as i32,
        ..QuestRewardsBlock::default()
    };
    for (idx, reward_item) in quest.reward_items.iter().enumerate() {
        if let Some(reward_slot) = rewards.items.get_mut(idx) {
            let amount = quest.reward_amounts.get(idx).copied().unwrap_or(0);
            *reward_slot = (*reward_item, amount);
        }
    }
    for (idx, display_spell) in quest.reward_display_spell.iter().enumerate() {
        if let Some(slot) = rewards.display_spells.get_mut(idx) {
            *slot = *display_spell;
        }
    }
    for (idx, choice_item) in quest.reward_choice_items.iter().enumerate() {
        if let Some(slot) = rewards.choice_items.get_mut(idx) {
            *slot = *choice_item;
        }
    }
    rewards.choice_item_types = quest.reward_choice_item_types;
    rewards
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

pub(in crate::session) const fn react_state_from_db_like_cpp(value: u8) -> ReactState {
    match value {
        0 => ReactState::Passive,
        1 => ReactState::Defensive,
        2 => ReactState::Aggressive,
        _ => ReactState::Passive,
    }
}

pub(in crate::session) const fn pet_type_from_db_like_cpp(value: u8) -> PetType {
    match value {
        0 => PetType::Summon,
        1 => PetType::Hunter,
        _ => PetType::Max,
    }
}

pub(in crate::session) const fn active_state_from_db_like_cpp(value: u8) -> ActiveState {
    match value {
        0x00 => ActiveState::Decide,
        0x01 => ActiveState::Passive,
        0x81 => ActiveState::Disabled,
        0xC1 => ActiveState::Enabled,
        _ => ActiveState::Disabled,
    }
}

pub(in crate::session) const fn power_type_from_u8_like_cpp(power: u8) -> PowerType {
    match power {
        1 => PowerType::Rage,
        2 => PowerType::Focus,
        3 => PowerType::Energy,
        4 => PowerType::Happiness,
        5 => PowerType::Runes,
        6 => PowerType::RunicPower,
        7 => PowerType::SoulShards,
        8 => PowerType::LunarPower,
        9 => PowerType::HolyPower,
        10 => PowerType::AlternatePower,
        11 => PowerType::Maelstrom,
        12 => PowerType::Chi,
        13 => PowerType::Insanity,
        14 => PowerType::ComboPoints,
        15 => PowerType::DemonicFury,
        16 => PowerType::ArcaneCharges,
        17 => PowerType::Fury,
        18 => PowerType::Pain,
        19 => PowerType::Essence,
        20 => PowerType::RuneBlood,
        21 => PowerType::RuneFrost,
        22 => PowerType::RuneUnholy,
        23 => PowerType::AlternateQuest,
        24 => PowerType::AlternateEncounter,
        25 => PowerType::AlternateMount,
        _ => PowerType::Mana,
    }
}

#[cfg(test)]
pub(in crate::session) const fn primary_power_type_for_player_class_like_cpp(
    class_id: u8,
) -> PowerType {
    match class_id {
        1 => PowerType::Rage,
        4 => PowerType::Energy,
        6 => PowerType::RunicPower,
        _ => PowerType::Mana,
    }
}

pub(in crate::session) const fn unit_stand_state_from_u8_like_cpp(value: u8) -> UnitStandStateType {
    match value {
        1 => UnitStandStateType::Sit,
        2 => UnitStandStateType::SitChair,
        3 => UnitStandStateType::Sleep,
        4 => UnitStandStateType::SitLowChair,
        5 => UnitStandStateType::SitMediumChair,
        6 => UnitStandStateType::SitHighChair,
        7 => UnitStandStateType::Dead,
        8 => UnitStandStateType::Kneel,
        9 => UnitStandStateType::Submerged,
        10 => UnitStandStateType::Max,
        _ => UnitStandStateType::Stand,
    }
}

pub(in crate::session) const fn sheath_state_from_u8_like_cpp(value: u8) -> SheathState {
    match value {
        1 => SheathState::Melee,
        2 => SheathState::Ranged,
        _ => SheathState::Unarmed,
    }
}

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

/// Session-local representation of C++ `Player::GetPlayerSharingQuest()` state
/// until full party/ObjectAccessor quest sharing runtime owns it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedPendingQuestSharingLikeCpp {
    pub sender_guid: ObjectGuid,
    pub quest_id: u32,
}

/// Evidence for the bounded `HandleQuestPushResult` sender-match seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedQuestPushResultResponseLikeCpp {
    pub receiver_guid: ObjectGuid,
    pub sender_guid: ObjectGuid,
    pub parsed_quest_id: u32,
    pub pending_quest_id: u32,
    pub result: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedPushQuestToPartyOutcomeReasonLikeCpp {
    NotAllowed,
    NotDaily,
    QuestPoolActiveCheckUnrepresented,
    NotInParty,
    GroupRuntimeUnrepresented,
    ReceiverBusy,
    ReceiverDead,
    ReceiverAlreadyDone,
    ReceiverOnQuest,
    ReceiverLogFull,
    ReceiverSatisfyQuestDayAlreadyDone,
    ReceiverSatisfyQuestMinLevelLowLevel,
    ReceiverSatisfyQuestMaxLevelHighLevel,
    ReceiverSatisfyQuestClassWrongClass,
    ReceiverSatisfyQuestRaceWrongRace,
    ReceiverSatisfyQuestReputationLowFaction,
    ReceiverSatisfyQuestReputationHighFaction,
    ReceiverSatisfyQuestPreviousQuestPrerequisite,
    ReceiverSatisfyQuestDependentPreviousQuestsPrerequisite,
    ReceiverSatisfyQuestDependentBreadcrumbQuestsPrerequisite,
    ReceiverSatisfyQuestExpansionRequiredExpansion,
    ReceiverCanTakeQuestInvalid,
    #[allow(dead_code)]
    ReceiverRepeatableTurnInRequestItemsUnrepresented,
    ReceiverRepeatableTurnInRequestItemsPrompted,
    ReceiverRepeatableTurnInRequestItemsPromptCommandFailed,
    ReceiverSuccessQuestDetailsPrompted,
    ReceiverQuestDetailsPromptCommandFailed,
    ReceiverEligibilityUnrepresented,
}

/// Session-local evidence for the bounded sender-side `HandlePushQuestToParty` preflight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedPushQuestToPartyOutcomeLikeCpp {
    pub sender_guid: Option<ObjectGuid>,
    pub quest_id: u32,
    pub target_guid: Option<ObjectGuid>,
    pub reason: RepresentedPushQuestToPartyOutcomeReasonLikeCpp,
    pub quest_pool_active_check_unrepresented: bool,
    pub group_runtime_unrepresented: bool,
    pub receiver_fanout_unrepresented: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedAdventureMapStartQuestLikeCpp {
    pub quest_id: u32,
    pub adventure_map_poi_id: u32,
    pub player_condition_id: u32,
}

/// Represented outcome for the bounded post-template `HandleQuestConfirmAccept` gates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp {
    OriginalPlayerMissing,
    NotInSameRaid,
    OriginalPlayerNotActiveQuest,
    ReceiverCanTakeQuestFailed,
    ReceiverCanAddQuestLogFull,
    ReceiverCanAddQuestSourceItemFailed,
    ReceiverGiveQuestSourceItemStartQuestNoGrant,
    ReceiverGiveQuestSourceItemMaxCountNoGrant,
    ReceiverGiveQuestSourceItemStoredNewItem,
    ReceiverGiveQuestSourceItemBoundObjectiveNoGrant,
    GiveQuestSourceItemStoreNewItemUnrepresented,
    ReceiverAddQuestLocalStateRepresented,
    #[allow(dead_code)]
    AddQuestRuntimeUnrepresented,
}

/// Evidence that `HandleQuestConfirmAccept` reached the post-clear/template-present seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedQuestConfirmAcceptLikeCpp {
    pub receiver_guid: Option<ObjectGuid>,
    pub sender_guid_before_clear: ObjectGuid,
    pub quest_id: u32,
    pub raw_quest_id: i32,
    pub reason: RepresentedQuestConfirmAcceptOutcomeReasonLikeCpp,
    pub object_accessor_unrepresented: bool,
    pub party_runtime_unrepresented: bool,
    pub can_add_source_item_unrepresented: bool,
    pub can_add_source_item_result: Option<InventoryResult>,
    pub add_quest_runtime_unrepresented: bool,
    pub source_spell_unrepresented: bool,
    /// Source spell id whose C++ triggered self-casts are represented as evidence only.
    pub represented_source_spell_id: Option<u32>,
    /// Count of represented triggered self-casts C++ would perform in this shared-confirm path.
    pub represented_source_spell_self_casts: u8,
}

/// Evidence for represented `Player::CompleteQuest` status-update side effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedQuestCompleteStatusUpdateLikeCpp {
    pub quest_id: u32,
    pub old_status: u8,
    pub new_status: u8,
    pub send_quest_update_called: bool,
    pub quest_slot_state_complete_represented: bool,
    pub quest_slot_state_live_update_unrepresented: bool,
    pub visible_gameobjects_or_spellclicks_refresh_unrepresented: bool,
    pub spell_area_runtime_unrepresented: bool,
    pub tracking_event_auto_reward_unrepresented: bool,
    pub quest_tracker_complete_time_unrepresented: bool,
    pub script_status_change_unrepresented: bool,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedQuestRewardSpellKindLikeCpp {
    RewardSpell,
    RewardDisplaySpell { index: u8 },
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedQuestRewardSpellCastLikeCpp {
    pub quest_id: u32,
    pub spell_id: u32,
    pub kind: RepresentedQuestRewardSpellKindLikeCpp,
    pub can_delay_teleport_like_cpp: bool,
    pub spell_info_lookup_unrepresented: bool,
    pub caster_selection_unrepresented: bool,
    pub cast_spell_runtime_unrepresented: bool,
}

#[cfg(test)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RepresentedForceDeselectLikeCpp {
    pub caster_guid: ObjectGuid,
    pub visibility_range_yards: u32,
    pub break_target_packet_bytes: Vec<u8>,
    pub clear_target_packet_bytes: Vec<u8>,
    pub hostile_visible_fanout_unrepresented: bool,
    pub attacker_pet_attack_stop_unrepresented: bool,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedQuestRewardTitleLikeCpp {
    pub quest_id: u32,
    pub title_id: u32,
    pub char_title_lookup_unrepresented: bool,
    pub set_title_runtime_unrepresented: bool,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedQuestRewardTalentPointsLikeCpp {
    pub quest_id: u32,
    pub points: u32,
    pub init_talent_for_level_unrepresented: bool,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedQuestRewardMailLikeCpp {
    pub quest_id: u32,
    pub mail_template_id: u32,
    pub delay_secs: u32,
    pub sender_entry: Option<u32>,
    pub quest_giver_guid: Option<ObjectGuid>,
    pub mail_template_lookup_unrepresented: bool,
    pub mail_draft_runtime_unrepresented: bool,
    pub character_db_transaction_unrepresented: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RepresentedQuestRewardReputationSourceLikeCpp {
    Quest,
    DailyQuest,
    WeeklyQuest,
    MonthlyQuest,
    RepeatableQuest,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RepresentedQuestRewardReputationLikeCpp {
    pub quest_id: u32,
    pub slot: u8,
    pub faction_id: u32,
    pub reward_faction_value: i32,
    pub reward_faction_override: i32,
    pub reward_faction_cap_in: i32,
    pub base_reputation_before_gain: i32,
    pub reputation_after_low_level_rate_like_cpp: i32,
    pub reputation_after_reward_rate_like_cpp: i32,
    pub no_quest_bonus: bool,
    pub no_spillover: bool,
    pub source: RepresentedQuestRewardReputationSourceLikeCpp,
    pub faction_store_lookup_unrepresented: bool,
    pub quest_faction_reward_store_lookup_unrepresented: bool,
    pub reputation_reward_rate_lookup_unrepresented: bool,
    pub gray_level_script_hook_unrepresented: bool,
    pub reputation_rank_cap_check_unrepresented: bool,
    pub calculate_reputation_gain_unrepresented: bool,
    pub modify_reputation_runtime_unrepresented: bool,
}

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
        let Some(quest_store) = self.quests.store.as_ref().map(Arc::clone) else {
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
                        self.player_race_like_cpp(),
                        self.player_class_like_cpp(),
                        self.player_level_like_cpp(),
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
            race = self.player_race_like_cpp(),
            class = self.player_class_like_cpp(),
            level = self.player_level_like_cpp(),
            ender_candidates = ?ender_candidates,
            starter_candidates = ?starter_candidates,
            menu_items = ?self.represented_quest_menu_item_log_rows_like_cpp(&menu_items),
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
        self.represented_gameobject_use_effects
            .push(RepresentedGameObjectUseEffect::SendGossip {
                gameobject_guid,
                player_guid,
                gossip_id: source.gossip_id,
            });

        let Some(quest_store) = self.quests.store.as_ref().map(Arc::clone) else {
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

    pub(in crate::session) fn represented_quest_menu_item_log_rows_like_cpp(
        &self,
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
