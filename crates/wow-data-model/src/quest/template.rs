//! Full quest metadata and its existing pure model operations.

use super::{QuestEligibilityRules, QuestObjective, QuestObjectiveRulesLikeCpp, QuestRewardRules};
use wow_constants::quest::{
    QUEST_REWARD_ITEM_COUNT,
    QUEST_REWARD_CHOICES_COUNT,
    QUEST_REWARD_REPUTATIONS_COUNT,
    QUEST_REWARD_CURRENCY_COUNT,
    QUEST_REWARD_DISPLAY_SPELL_COUNT,
    QUEST_ITEM_DROP_COUNT,
    QUEST_FLAGS_DAILY as QUEST_FLAGS_DAILY_LIKE_CPP,
    QUEST_FLAGS_WEEKLY as QUEST_FLAGS_WEEKLY_LIKE_CPP,
    QUEST_TYPE_TURNIN as QUEST_TYPE_TURNIN_LIKE_CPP,
    QUEST_SPECIAL_FLAGS_REPEATABLE as QUEST_SPECIAL_FLAGS_REPEATABLE_LIKE_CPP,
    QUEST_SPECIAL_FLAGS_DF_QUEST as QUEST_SPECIAL_FLAGS_DF_QUEST_LIKE_CPP,
    QUEST_SPECIAL_FLAGS_MONTHLY as QUEST_SPECIAL_FLAGS_MONTHLY_LIKE_CPP,
    QUEST_SORT_SEASONAL_NEGATIVE as QUEST_SORT_SEASONAL_NEGATIVE_LIKE_CPP,
    QUEST_SORT_SPECIAL_NEGATIVE as QUEST_SORT_SPECIAL_NEGATIVE_LIKE_CPP,
    QUEST_SORT_LUNAR_FESTIVAL_NEGATIVE as QUEST_SORT_LUNAR_FESTIVAL_NEGATIVE_LIKE_CPP,
    QUEST_SORT_MIDSUMMER_NEGATIVE as QUEST_SORT_MIDSUMMER_NEGATIVE_LIKE_CPP,
    QUEST_SORT_BREWFEST_NEGATIVE as QUEST_SORT_BREWFEST_NEGATIVE_LIKE_CPP,
    QUEST_SORT_NOBLEGARDEN_NEGATIVE as QUEST_SORT_NOBLEGARDEN_NEGATIVE_LIKE_CPP,
    QUEST_SORT_LOVE_IS_IN_THE_AIR_NEGATIVE as QUEST_SORT_LOVE_IS_IN_THE_AIR_NEGATIVE_LIKE_CPP,
    QUEST_FLAGS_EX_IS_WORLD_QUEST_LIKE_CPP, QUEST_FLAGS_EX_REWARDS_IGNORE_CAPS_LIKE_CPP,
};

mod level_requirements;
mod rewards;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestInfoEntry {
    pub id: u32,
    pub info_name: String,
    pub quest_type: i8,
    pub modifiers: i32,
    pub profession: u16,
}

// ── QuestTemplate ─────────────────────────────────────────────────────────────

/// Full quest data loaded from the world database.
/// C# ref: Quest class / quest_template table
#[derive(Debug, Clone)]
pub struct QuestTemplate {
    pub id: u32,
    pub quest_type: u8,
    pub quest_level: i32,
    pub quest_max_scaling_level: i32,
    /// C++ `Quest::GetQuestPackageID()` / `_packageID` from `quest_template.QuestPackageID`.
    pub quest_package_id: u32,
    pub min_level: i32,
    pub quest_sort_id: i32,
    pub quest_info_id: u16,
    pub suggested_group_num: u8,
    pub reward_next_quest: u32,
    pub reward_xp_difficulty: u32,
    pub reward_xp_multiplier: f32,
    pub reward_money_difficulty: u32,
    pub reward_money_multiplier: f32,
    pub reward_bonus_money: u32,
    pub reward_display_spell: [u32; QUEST_REWARD_DISPLAY_SPELL_COUNT],
    pub reward_spell: u32,
    pub reward_honor: u32,
    pub reward_title_id: u32,
    pub reward_skill_line_id: u32,
    pub reward_skill_points: u32,
    pub reward_mail_template_id: u32,
    pub reward_mail_delay_secs: u32,
    pub reward_mail_sender_entry: u32,
    /// C++ `Quest::RewardFactionId[0..5]` from `quest_template.RewardFactionID1..5`.
    pub reward_faction_ids: [u32; QUEST_REWARD_REPUTATIONS_COUNT],
    /// C++ `Quest::RewardFactionValue[0..5]` from `quest_template.RewardFactionValue1..5`.
    pub reward_faction_values: [i32; QUEST_REWARD_REPUTATIONS_COUNT],
    /// C++ `Quest::RewardFactionOverride[0..5]` from `quest_template.RewardFactionOverride1..5`.
    pub reward_faction_overrides: [i32; QUEST_REWARD_REPUTATIONS_COUNT],
    /// C++ `Quest::RewardFactionCapIn[0..5]` from `quest_template.RewardFactionCapIn1..5`.
    pub reward_faction_cap_in: [i32; QUEST_REWARD_REPUTATIONS_COUNT],
    /// C++ `Quest::GetRewardReputationMask()` / `_rewardReputationMask`.
    pub reward_faction_flags: u32,
    /// C++ `Quest::GetSrcItemId()` / `_sourceItemId` from `quest_template.StartItem`.
    pub source_item_id: u32,
    /// C++ `Quest::GetSrcItemCount()` / `_sourceItemIdCount` from `quest_template_addon.ProvidedItemCount`.
    pub source_item_count: u32,
    /// C++ `Quest::GetSrcSpell()` / `_sourceSpellID` from `quest_template_addon.SourceSpellID`.
    pub source_spell_id: u32,
    /// C++ `Quest::GetLimitTime()` / `_limitTime` from `quest_template.TimeAllowed`.
    pub limit_time_secs: i64,
    /// C++ `Quest::GetExpansion()` / `quest_template.Expansion`.
    pub expansion: i32,
    pub flags: u32,
    pub flags_ex: u32,
    pub flags_ex2: u32,
    pub special_flags: u32,
    pub event_id_for_quest: u16,
    pub reward_items: [u32; QUEST_REWARD_ITEM_COUNT],
    pub reward_amounts: [u32; QUEST_REWARD_ITEM_COUNT],
    pub reward_currencies: [u32; QUEST_REWARD_CURRENCY_COUNT],
    pub reward_currency_amounts: [u32; QUEST_REWARD_CURRENCY_COUNT],
    pub item_drop: [u32; QUEST_ITEM_DROP_COUNT],
    pub item_drop_quantity: [u32; QUEST_ITEM_DROP_COUNT],
    // Strings
    pub log_title: String,
    pub log_description: String,
    pub quest_description: String,
    pub area_description: String,
    pub quest_completion_log: String,
    // Objectives
    pub objectives: Vec<QuestObjective>,

    // ── Eligibility filters ──────────────────────────────────────────────────
    /// Bitmask of allowed races: bit (race-1) set = allowed.
    /// 0 = all races allowed (RaceMask::Playable default).
    pub allowable_races: u64,
    /// Bitmask of allowed classes: bit (class-1) set = allowed.
    /// 0 = all classes allowed.
    pub allowable_classes: u32,
    /// Maximum player level to take this quest. 0 = no limit.
    pub max_level: u8,
    /// Previous quest that must be completed first. 0 = none.
    /// Positive = must be rewarded. Negative = must be active (Incomplete).
    pub prev_quest_id: i32,
    /// C++ `_nextQuestID`/`Quest::GetNextQuestId()` from `quest_template_addon`.
    pub next_quest_id: u32,
    /// C++ `_exclusiveGroup`/`Quest::GetExclusiveGroup()` from `quest_template_addon`.
    pub exclusive_group: i32,
    /// C++ `_breadcrumbForQuestId`/`Quest::GetBreadcrumbForQuestId()` from `quest_template_addon`.
    pub breadcrumb_for_quest_id: i32,
    /// C++ `Quest::DependentPreviousQuests`, rebuilt post-load by ObjectMgr-style normalization.
    ///
    /// This is derived metadata, not a raw DB column.
    pub dependent_previous_quests: Vec<u32>,
    /// C++ `Quest::DependentBreadcrumbQuests`, rebuilt post-load by ObjectMgr-style normalization.
    ///
    /// This is derived metadata, not a raw DB column.
    pub dependent_breadcrumb_quests: Vec<u32>,
    /// C++ `Quest::GetRequiredMinRepFaction()` from `quest_template_addon`.
    pub required_min_rep_faction: u32,
    /// C++ `Quest::GetRequiredMinRepValue()` from `quest_template_addon`.
    pub required_min_rep_value: i32,
    /// C++ `Quest::GetRequiredMaxRepFaction()` from `quest_template_addon`.
    pub required_max_rep_faction: u32,
    /// C++ `Quest::GetRequiredMaxRepValue()` from `quest_template_addon`.
    pub required_max_rep_value: i32,
    /// C++ `Quest::GetRequiredSkill()` / `_requiredSkillId` — QuestDef.h:585, quest_template_addon.
    pub required_skill_id: u32,
    /// C++ `Quest::GetRequiredSkillValue()` / `_requiredSkillPoints` — QuestDef.h:586, ObjectMgr.cpp:4623.
    pub required_skill_points: u32,
    /// Optional reward choices player can choose (up to 6). (item_id, quantity).
    /// item_id == 0 means that slot is empty.
    pub reward_choice_items: [(u32, u32); QUEST_REWARD_CHOICES_COUNT],
    /// C++ `Quest::RewardChoiceItemType`, loaded from `quest_reward_choice_items.Type1..Type6`.
    ///
    /// `0 = LootItemType::Item`, `1 = LootItemType::Currency`.
    pub reward_choice_item_types: [u8; QUEST_REWARD_CHOICES_COUNT],
}

impl QuestTemplate {
    /// Borrow only the metadata consumed by Player quest-eligibility rules.
    /// The full quest row and its normalized lists remain owned by this catalog.
    pub fn eligibility_rules(&self) -> QuestEligibilityRules<'_> {
        QuestEligibilityRules::new(
            self.id,
            self.is_repeatable(),
            self.exclusive_group,
            self.prev_quest_id,
            &self.dependent_previous_quests,
            &self.dependent_breadcrumb_quests,
            self.is_daily_like_cpp(),
            self.is_df_quest_like_cpp(),
            self.is_weekly_like_cpp(),
            self.is_monthly_like_cpp(),
            self.is_seasonal_like_cpp(),
            self.event_id_for_quest_like_cpp(),
        )
    }

    /// Borrow the metadata used by Player objective rules without cloning the
    /// catalog or exposing a data-store dependency to the entity layer.
    pub fn objective_rules_like_cpp(&self) -> QuestObjectiveRulesLikeCpp<'_> {
        QuestObjectiveRulesLikeCpp::new(
            self.id,
            self.flags,
            self.limit_time_secs,
            self.is_repeatable(),
            &self.objectives,
        )
    }

    /// Calculate the accepted quest window from the caller's authoritative
    /// clock sample. Session owns the sample; the quest model owns `TimeAllowed`.
    pub fn accepted_and_end_time_like_cpp(&self, accept_time: i64) -> (i64, i64) {
        let end_time = if self.limit_time_secs > 0 {
            accept_time.saturating_add(self.limit_time_secs)
        } else {
            0
        };
        (accept_time, end_time)
    }

    /// Classify the C++ currency source emitted for this quest reward.
    pub fn currency_gain_source_like_cpp(
        &self,
    ) -> wow_constants::currency::CurrencyGainSourceLikeCpp {
        use wow_constants::currency::CurrencyGainSourceLikeCpp;

        if (self.flags_ex & QUEST_FLAGS_EX_REWARDS_IGNORE_CAPS_LIKE_CPP) != 0 {
            if (self.flags_ex & QUEST_FLAGS_EX_IS_WORLD_QUEST_LIKE_CPP) != 0 {
                return CurrencyGainSourceLikeCpp::WorldQuestRewardIgnoreCaps;
            }
            return CurrencyGainSourceLikeCpp::QuestRewardIgnoreCaps;
        }

        if self.is_daily_like_cpp() {
            CurrencyGainSourceLikeCpp::DailyQuestReward
        } else if self.is_weekly_like_cpp() {
            CurrencyGainSourceLikeCpp::WeeklyQuestReward
        } else if (self.flags_ex & QUEST_FLAGS_EX_IS_WORLD_QUEST_LIKE_CPP) != 0 {
            CurrencyGainSourceLikeCpp::WorldQuestReward
        } else {
            CurrencyGainSourceLikeCpp::QuestReward
        }
    }

    /// C++ `Quest::IsRepeatable()` exact helper: only `QUEST_SPECIAL_FLAGS_REPEATABLE`.
    pub fn is_repeatable(&self) -> bool {
        self.special_flags & QUEST_SPECIAL_FLAGS_REPEATABLE_LIKE_CPP != 0
    }

    /// C++ `Quest::IsDaily()`: `QUEST_FLAGS_DAILY`.
    pub fn is_daily_like_cpp(&self) -> bool {
        self.flags & QUEST_FLAGS_DAILY_LIKE_CPP != 0
    }

    /// C++ `Quest::IsWeekly()`: `QUEST_FLAGS_WEEKLY`.
    pub fn is_weekly_like_cpp(&self) -> bool {
        self.flags & QUEST_FLAGS_WEEKLY_LIKE_CPP != 0
    }

    /// C++ `Quest::IsDFQuest()`: `QUEST_SPECIAL_FLAGS_DF_QUEST`.
    pub fn is_df_quest_like_cpp(&self) -> bool {
        self.special_flags & QUEST_SPECIAL_FLAGS_DF_QUEST_LIKE_CPP != 0
    }

    /// C++ `Quest::IsDailyOrWeekly()`.
    pub fn is_daily_or_weekly_like_cpp(&self) -> bool {
        self.is_daily_like_cpp() || self.is_weekly_like_cpp()
    }

    /// C++ `Quest::IsMonthly()`: `QUEST_SPECIAL_FLAGS_MONTHLY`.
    pub fn is_monthly_like_cpp(&self) -> bool {
        self.special_flags & QUEST_SPECIAL_FLAGS_MONTHLY_LIKE_CPP != 0
    }

    /// C++ `Quest::IsTurnIn()` with config gate represented by stored quest type only.
    pub fn is_turn_in_like_cpp(&self) -> bool {
        self.quest_type == QUEST_TYPE_TURNIN_LIKE_CPP
    }

    /// C++ `Quest::IsSeasonal()` exact quest sort set plus non-repeatable guard.
    pub fn is_seasonal_like_cpp(&self) -> bool {
        matches!(
            self.quest_sort_id,
            QUEST_SORT_SEASONAL_NEGATIVE_LIKE_CPP
                | QUEST_SORT_SPECIAL_NEGATIVE_LIKE_CPP
                | QUEST_SORT_LUNAR_FESTIVAL_NEGATIVE_LIKE_CPP
                | QUEST_SORT_MIDSUMMER_NEGATIVE_LIKE_CPP
                | QUEST_SORT_BREWFEST_NEGATIVE_LIKE_CPP
                | QUEST_SORT_LOVE_IS_IN_THE_AIR_NEGATIVE_LIKE_CPP
                | QUEST_SORT_NOBLEGARDEN_NEGATIVE_LIKE_CPP
        ) && !self.is_repeatable()
    }

    /// C++ `Quest::GetEventIdForQuest()`. Defaults to 0 until seasonal relation load sets it.
    pub fn event_id_for_quest_like_cpp(&self) -> u16 {
        self.event_id_for_quest
    }

    /// C++ `ObjectMgr::LoadQuests` source-item/source-spell metadata normalization.
    ///
    /// The caller owns item/spell stores and passes real predicates. `load_quests` keeps raw DB
    /// values until composition can provide those predicates without making
    /// `wow-data` depend on runtime stores.
    pub fn normalize_source_item_spell_like_cpp(
        &mut self,
        item_exists: impl Fn(u32) -> bool,
        spell_valid: impl Fn(u32) -> bool,
    ) {
        if self.source_item_id != 0 {
            if !item_exists(self.source_item_id) {
                self.source_item_id = 0;
            } else if self.source_item_count == 0 {
                self.source_item_count = 1;
            }
        } else if self.source_item_count > 0 {
            self.source_item_count = 0;
        }

        if self.source_spell_id != 0 && !spell_valid(self.source_spell_id) {
            self.source_spell_id = 0;
        }
    }

    /// Returns true if the given player (race, class, level) can take this quest.
    /// C# ref: SatisfyQuestRace + SatisfyQuestClass + SatisfyQuestLevel
    pub fn is_available_for(&self, race: u8, class: u8, level: u8) -> bool {
        // Race check: 0 means all races allowed
        if self.allowable_races != 0 {
            let race_bit = 1u64 << (race.saturating_sub(1) as u64);
            if self.allowable_races & race_bit == 0 {
                return false;
            }
        }

        // Class check: 0 means all classes allowed
        if self.allowable_classes != 0 {
            let class_bit = 1u32 << (class.saturating_sub(1) as u32);
            if self.allowable_classes & class_bit == 0 {
                return false;
            }
        }

        // Min level check
        if !self.meets_min_level(level) {
            return false;
        }

        // Max level check
        if !self.meets_max_level(level) {
            return false;
        }

        true
    }
}

impl QuestTemplate {
    /// Match a nonempty loaded reward slot by ID and type; quantity is not a discriminator.
    pub fn reward_choice_matches_loaded_type(&self, loot_item_type: u8, choice_item_id: u32) -> bool {
        self
            .reward_choice_items
            .iter()
            .zip(self.reward_choice_item_types.iter())
            .any(|((item_id, _quantity), item_type)| {
                *item_id != 0 && *item_id == choice_item_id && *item_type == loot_item_type
            })
    }
}
