// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Quest system data structures and in-memory store.
//!
//! Loads `quest_template`, `quest_objectives`, creature quest relations,
//! and GameObject quest relations from the world database at startup.

use std::collections::{HashMap, HashSet};
use tracing::{info, warn};

// ── Constants (matching C# SharedConst) ──────────────────────────────────────
pub const QUEST_REWARD_ITEM_COUNT: usize = 4;
pub const QUEST_REWARD_CHOICES_COUNT: usize = 6;
pub const QUEST_REWARD_REPUTATIONS_COUNT: usize = 5;
pub const QUEST_REWARD_CURRENCY_COUNT: usize = 4;
pub const QUEST_REWARD_DISPLAY_SPELL_COUNT: usize = 3;
pub const QUEST_ITEM_DROP_COUNT: usize = 4;

pub const QUEST_FLAGS_DAILY_LIKE_CPP: u32 = 0x0000_1000;
pub const QUEST_FLAGS_COMPLETION_AREA_TRIGGER_LIKE_CPP: u32 = 0x0000_0004;
pub const QUEST_FLAGS_HIDE_REWARD_POI_LIKE_CPP: u32 = 0x0000_0020;
pub const QUEST_OBJECTIVE_AREATRIGGER_LIKE_CPP: u8 = 10;
pub const QUEST_FLAGS_WEEKLY_LIKE_CPP: u32 = 0x0000_8000;
pub const QUEST_FLAGS_EX_LEGENDARY_LIKE_CPP: u32 = 0x0000_0100;
const QUEST_TYPE_TURNIN_LIKE_CPP: u8 = 0;
const QUEST_SPECIAL_FLAGS_REPEATABLE_LIKE_CPP: u32 = 0x0000_0001;
const QUEST_SPECIAL_FLAGS_AUTO_PUSH_TO_PARTY_LIKE_CPP: u32 = 0x0000_0002;
const QUEST_SPECIAL_FLAGS_AUTO_ACCEPT_LIKE_CPP: u32 = 0x0000_0004;
pub const QUEST_SPECIAL_FLAGS_DF_QUEST_LIKE_CPP: u32 = 0x0000_0008;
pub const QUEST_SPECIAL_FLAGS_MONTHLY_LIKE_CPP: u32 = 0x0000_0010;
const QUEST_SPECIAL_FLAGS_DB_ALLOWED_LIKE_CPP: u32 = QUEST_SPECIAL_FLAGS_REPEATABLE_LIKE_CPP
    | QUEST_SPECIAL_FLAGS_AUTO_PUSH_TO_PARTY_LIKE_CPP
    | QUEST_SPECIAL_FLAGS_AUTO_ACCEPT_LIKE_CPP
    | QUEST_SPECIAL_FLAGS_DF_QUEST_LIKE_CPP
    | QUEST_SPECIAL_FLAGS_MONTHLY_LIKE_CPP;

const QUEST_SORT_SEASONAL_LIKE_CPP: i32 = 22;
const QUEST_SORT_SPECIAL_LIKE_CPP: i32 = 284;
const QUEST_SORT_LUNAR_FESTIVAL_LIKE_CPP: i32 = 366;
const QUEST_SORT_MIDSUMMER_LIKE_CPP: i32 = 369;
const QUEST_SORT_BREWFEST_LIKE_CPP: i32 = 370;
const QUEST_SORT_NOBLEGARDEN_LIKE_CPP: i32 = 374;
const QUEST_SORT_LOVE_IS_IN_THE_AIR_LIKE_CPP: i32 = 376;
const QUEST_SORT_SEASONAL_NEGATIVE_LIKE_CPP: i32 = -QUEST_SORT_SEASONAL_LIKE_CPP;
const QUEST_SORT_SPECIAL_NEGATIVE_LIKE_CPP: i32 = -QUEST_SORT_SPECIAL_LIKE_CPP;
const QUEST_SORT_LUNAR_FESTIVAL_NEGATIVE_LIKE_CPP: i32 = -QUEST_SORT_LUNAR_FESTIVAL_LIKE_CPP;
const QUEST_SORT_MIDSUMMER_NEGATIVE_LIKE_CPP: i32 = -QUEST_SORT_MIDSUMMER_LIKE_CPP;
const QUEST_SORT_BREWFEST_NEGATIVE_LIKE_CPP: i32 = -QUEST_SORT_BREWFEST_LIKE_CPP;
const QUEST_SORT_NOBLEGARDEN_NEGATIVE_LIKE_CPP: i32 = -QUEST_SORT_NOBLEGARDEN_LIKE_CPP;
const QUEST_SORT_LOVE_IS_IN_THE_AIR_NEGATIVE_LIKE_CPP: i32 =
    -QUEST_SORT_LOVE_IS_IN_THE_AIR_LIKE_CPP;

// ── QuestObjective ────────────────────────────────────────────────────────────

/// A single objective for a quest (kill X, loot Y, explore Z, etc.)
/// C# ref: QuestObjective struct / quest_objectives table
#[derive(Debug, Clone)]
pub struct QuestObjective {
    pub id: u32,
    pub quest_id: u32,
    /// 0=Monster, 1=Item, 2=GameObject, 3=TalkTo, 4=Currency,
    /// 5=LearnSpell, 6=MinReputation, 7=MaxReputation, 8=Money,
    /// 9=PlayerKills, 10=AreaTrigger, ...
    pub obj_type: u8,
    pub order: u8,
    pub storage_index: i8,
    pub object_id: i32,
    pub amount: i32,
    pub flags: u32,
    pub flags2: u32,
    pub progress_bar_weight: f32,
    pub description: String,
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
        if self.min_level > 0 && (level as i32) < self.min_level {
            return false;
        }

        // Max level check
        if self.max_level > 0 && level > self.max_level {
            return false;
        }

        true
    }
}

/// C++ `ObjectMgr::LoadQuests` post-load normalization before `Quest` helpers are observable.
fn nonzero_abs_i32_to_u32_like_cpp(value: i32) -> Option<u32> {
    let abs = value.unsigned_abs();
    (abs != 0).then_some(abs)
}

fn push_unique_sorted_like_cpp(values: &mut Vec<u32>, value: u32) {
    if !values.contains(&value) {
        values.push(value);
        values.sort_unstable();
    }
}

fn normalize_quest_flags_like_cpp(flags: u32, special_flags: u32) -> (u32, u32) {
    let mut flags = flags;
    let mut special_flags = special_flags & QUEST_SPECIAL_FLAGS_DB_ALLOWED_LIKE_CPP;

    if flags & QUEST_FLAGS_DAILY_LIKE_CPP != 0 && flags & QUEST_FLAGS_WEEKLY_LIKE_CPP != 0 {
        flags &= !QUEST_FLAGS_DAILY_LIKE_CPP;
    }

    if flags & (QUEST_FLAGS_DAILY_LIKE_CPP | QUEST_FLAGS_WEEKLY_LIKE_CPP) != 0
        || special_flags & QUEST_SPECIAL_FLAGS_MONTHLY_LIKE_CPP != 0
    {
        special_flags |= QUEST_SPECIAL_FLAGS_REPEATABLE_LIKE_CPP;
    }

    (flags, special_flags)
}

impl QuestObjective {
    /// C++ `QuestObjective::IsStoringFlag`.
    pub fn is_storing_flag_like_cpp(&self) -> bool {
        matches!(self.obj_type, 10 | 11 | 12 | 14 | 19 | 20)
    }

    /// C++ condition validation limit for `CONDITION_QUEST_OBJECTIVE_PROGRESS`.
    pub fn condition_progress_limit_like_cpp(&self) -> i32 {
        if self.is_storing_flag_like_cpp() {
            1
        } else {
            self.amount
        }
    }
}

// ── QuestStore ────────────────────────────────────────────────────────────────

/// In-memory store of all quest templates and NPC relations.
pub struct QuestStore {
    /// Quest templates by ID.
    pub quests: HashMap<u32, QuestTemplate>,
    /// NPC entry → list of quest IDs this NPC starts.
    pub starter_quests: HashMap<u32, Vec<u32>>,
    /// NPC entry → list of quest IDs this NPC ends.
    pub ender_quests: HashMap<u32, Vec<u32>>,
    /// GameObject template entry → list of quest IDs this GameObject starts.
    pub gameobject_starter_quests: HashMap<u32, Vec<u32>>,
    /// GameObject template entry → list of quest IDs this GameObject ends.
    pub gameobject_ender_quests: HashMap<u32, Vec<u32>>,
}

impl QuestStore {
    pub fn from_template_rows_like_cpp(templates: Vec<QuestTemplate>) -> Self {
        let mut store = Self::new();
        for mut quest in templates {
            (quest.flags, quest.special_flags) =
                normalize_quest_flags_like_cpp(quest.flags, quest.special_flags);
            store.quests.insert(quest.id, quest);
        }
        store.normalize_dependent_quest_metadata_like_cpp();
        info!("Loaded {} quest templates", store.quests.len());
        store
    }

    pub fn apply_special_flag_rows_like_cpp(&mut self, rows: Vec<(u32, u32)>) {
        let mut special_count = 0;
        for (id, special_flags) in rows {
            if let Some(quest) = self.quests.get_mut(&id) {
                (quest.flags, quest.special_flags) =
                    normalize_quest_flags_like_cpp(quest.flags, special_flags);
                special_count += 1;
            }
        }
        info!("Applied {special_count} quest_template_addon SpecialFlags rows like C++");
    }

    pub fn apply_seasonal_relation_rows_like_cpp(&mut self, rows: Vec<(u32, u32)>) {
        let mut seasonal_count = 0;
        for (quest_id, event_entry) in rows {
            if let Some(quest) = self.quests.get_mut(&quest_id) {
                if let Ok(event_id) = u16::try_from(event_entry) {
                    quest.event_id_for_quest = event_id;
                    seasonal_count += 1;
                } else {
                    warn!(
                        quest_id,
                        event_entry,
                        "Skipping seasonal quest relation with event id outside u16 range"
                    );
                }
            } else {
                warn!(
                    quest_id,
                    event_entry, "Skipping seasonal quest relation for missing quest template"
                );
            }
        }
        info!(
            "Loaded {seasonal_count} seasonal quest event relations (GameEvent max range guard remains with GameEvent metadata owner)"
        );
    }

    pub fn apply_objective_rows_like_cpp(&mut self, rows: Vec<QuestObjective>) {
        let mut objective_count = 0;
        for objective in rows {
            if let Some(quest) = self.quests.get_mut(&objective.quest_id) {
                quest.objectives.push(objective);
                objective_count += 1;
            }
        }
        info!("Loaded {objective_count} quest objectives");
    }

    pub fn apply_creature_starter_rows_like_cpp(&mut self, rows: Vec<(u32, u32)>) {
        for (npc, quest) in rows {
            if self.quests.contains_key(&quest) {
                self.starter_quests.entry(npc).or_default().push(quest);
            }
        }
    }

    pub fn apply_creature_ender_rows_like_cpp(&mut self, rows: Vec<(u32, u32)>) {
        for (npc, quest) in rows {
            if self.quests.contains_key(&quest) {
                self.ender_quests.entry(npc).or_default().push(quest);
            }
        }
    }

    pub fn apply_gameobject_starter_rows_like_cpp(&mut self, rows: Vec<(u32, u32)>) {
        for (entry, quest) in rows {
            self.insert_gameobject_starter_relation_like_cpp(entry, quest);
        }
    }

    pub fn apply_gameobject_ender_rows_like_cpp(&mut self, rows: Vec<(u32, u32)>) {
        for (entry, quest) in rows {
            self.insert_gameobject_ender_relation_like_cpp(entry, quest);
        }
    }

    pub fn log_relation_counts_like_cpp(&self) {
        info!(
            "Quest relations: NPC {} starters / {} enders, GameObject {} starters / {} enders",
            self.starter_quests.len(),
            self.ender_quests.len(),
            self.gameobject_starter_quests.len(),
            self.gameobject_ender_quests.len()
        );
    }

    pub fn new() -> Self {
        Self {
            quests: HashMap::new(),
            starter_quests: HashMap::new(),
            ender_quests: HashMap::new(),
            gameobject_starter_quests: HashMap::new(),
            gameobject_ender_quests: HashMap::new(),
        }
    }

    pub fn from_quests_like_cpp(quests: impl IntoIterator<Item = QuestTemplate>) -> Self {
        let mut store = Self {
            quests: quests.into_iter().map(|quest| (quest.id, quest)).collect(),
            starter_quests: HashMap::new(),
            ender_quests: HashMap::new(),
            gameobject_starter_quests: HashMap::new(),
            gameobject_ender_quests: HashMap::new(),
        };
        store.normalize_dependent_quest_metadata_like_cpp();
        store
    }

    /// C++ `ObjectMgr::LoadQuests` represented metadata normalization for quest dependencies.
    ///
    /// Ownership: `QuestStore` owns static DB quest metadata and these post-load derived vectors.
    /// Runtime handlers/sessions may read the normalized vectors in later slices, but must not
    /// write back into this store.
    pub fn normalize_dependent_quest_metadata_like_cpp(&mut self) {
        for quest in self.quests.values_mut() {
            quest.dependent_previous_quests.clear();
            quest.dependent_breadcrumb_quests.clear();
        }

        let mut quest_ids: Vec<u32> = self.quests.keys().copied().collect();
        quest_ids.sort_unstable();

        for quest_id in &quest_ids {
            let Some(quest) = self.quests.get(quest_id) else {
                continue;
            };

            let prev_quest_id = quest.prev_quest_id;
            let next_quest_id = quest.next_quest_id;
            let breadcrumb_for_quest_id = quest.breadcrumb_for_quest_id;

            if let Some(prev_id) = nonzero_abs_i32_to_u32_like_cpp(prev_quest_id) {
                if self
                    .quests
                    .get(&prev_id)
                    .is_some_and(|previous| previous.breadcrumb_for_quest_id == 0)
                    && prev_quest_id > 0
                {
                    if let Some(quest) = self.quests.get_mut(quest_id) {
                        push_unique_sorted_like_cpp(&mut quest.dependent_previous_quests, prev_id);
                    }
                }
            }

            if next_quest_id != 0 && self.quests.contains_key(&next_quest_id) {
                if let Some(next_quest) = self.quests.get_mut(&next_quest_id) {
                    push_unique_sorted_like_cpp(
                        &mut next_quest.dependent_previous_quests,
                        *quest_id,
                    );
                }
            }

            if let Some(breadcrumb_target_id) =
                nonzero_abs_i32_to_u32_like_cpp(breadcrumb_for_quest_id)
            {
                if !self.quests.contains_key(&breadcrumb_target_id) {
                    if let Some(quest) = self.quests.get_mut(quest_id) {
                        quest.breadcrumb_for_quest_id = 0;
                    }
                }
            }
        }

        for source_quest_id in quest_ids {
            let mut current_quest_id = source_quest_id;
            let mut breadcrumb_for_quest_id = self
                .quests
                .get(&current_quest_id)
                .and_then(|quest| nonzero_abs_i32_to_u32_like_cpp(quest.breadcrumb_for_quest_id));
            let mut seen = HashSet::new();

            while let Some(target_quest_id) = breadcrumb_for_quest_id {
                if !seen.insert(current_quest_id) {
                    if let Some(quest) = self.quests.get_mut(&current_quest_id) {
                        quest.breadcrumb_for_quest_id = 0;
                    }
                    break;
                }

                if !self.quests.contains_key(&target_quest_id) {
                    break;
                }

                if let Some(target_quest) = self.quests.get_mut(&target_quest_id) {
                    push_unique_sorted_like_cpp(
                        &mut target_quest.dependent_breadcrumb_quests,
                        source_quest_id,
                    );
                }

                current_quest_id = target_quest_id;
                breadcrumb_for_quest_id = self.quests.get(&current_quest_id).and_then(|quest| {
                    nonzero_abs_i32_to_u32_like_cpp(quest.breadcrumb_for_quest_id)
                });
            }
        }
    }

    /// Applies C++ source-item/source-spell metadata normalization to all loaded quest templates.
    ///
    /// Ownership: callers provide item/spell validity predicates from the future composition
    /// layer. This store owns only static quest metadata and never infers item/spell existence.
    pub fn normalize_source_item_spell_metadata_like_cpp(
        &mut self,
        item_exists: impl Fn(u32) -> bool,
        spell_valid: impl Fn(u32) -> bool,
    ) {
        for quest in self.quests.values_mut() {
            quest.normalize_source_item_spell_like_cpp(&item_exists, &spell_valid);
        }
    }

    pub fn get(&self, id: u32) -> Option<&QuestTemplate> {
        self.quests.get(&id)
    }

    /// Complete C++ `sObjectMgr->GetQuestTemplates()` projection. Callers
    /// that audit global login/update producers must inspect every template,
    /// not only quests related to the current Player.
    pub fn quests_like_cpp(&self) -> impl Iterator<Item = &QuestTemplate> {
        self.quests.values()
    }

    pub fn objective_like_cpp(&self, objective_id: u32) -> Option<&QuestObjective> {
        self.quests
            .values()
            .flat_map(|quest| quest.objectives.iter())
            .find(|objective| objective.id == objective_id)
    }

    pub fn objectives_like_cpp(&self) -> impl Iterator<Item = &QuestObjective> {
        self.quests
            .values()
            .flat_map(|quest| quest.objectives.iter())
    }

    /// Get all quests a given NPC can offer.
    pub fn quests_for_starter(&self, npc_entry: u32) -> Vec<&QuestTemplate> {
        self.starter_quests
            .get(&npc_entry)
            .map(|ids| ids.iter().filter_map(|id| self.quests.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get all quests a given NPC can complete/turn-in.
    pub fn quests_for_ender(&self, npc_entry: u32) -> Vec<&QuestTemplate> {
        self.ender_quests
            .get(&npc_entry)
            .map(|ids| ids.iter().filter_map(|id| self.quests.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get all quests a given GameObject can offer.
    pub fn quests_for_gameobject_starter(&self, go_entry: u32) -> Vec<&QuestTemplate> {
        self.gameobject_starter_quests
            .get(&go_entry)
            .map(|ids| ids.iter().filter_map(|id| self.quests.get(id)).collect())
            .unwrap_or_default()
    }

    /// Get all quests a given GameObject can complete/turn-in.
    pub fn quests_for_gameobject_ender(&self, go_entry: u32) -> Vec<&QuestTemplate> {
        self.gameobject_ender_quests
            .get(&go_entry)
            .map(|ids| ids.iter().filter_map(|id| self.quests.get(id)).collect())
            .unwrap_or_default()
    }

    /// Creature entries that have `creature_questender`/involved relation for `quest_id`.
    ///
    /// C++ uses reverse bounds over ObjectMgr relation multimaps. Rust's current store is
    /// entry → quest IDs, so this read-only reverse lookup sorts entries for deterministic
    /// represented output rather than depending on `HashMap` iteration order.
    pub fn creature_ender_entries_for_quest_like_cpp(&self, quest_id: u32) -> Vec<u32> {
        let mut entries: Vec<u32> = self
            .ender_quests
            .iter()
            .filter_map(|(&entry, quest_ids)| quest_ids.contains(&quest_id).then_some(entry))
            .collect();
        entries.sort_unstable();
        entries
    }

    /// GameObject entries that have `gameobject_questender`/involved relation for `quest_id`.
    ///
    /// C++ response callers must apply the `0x80000000` GameObject mask themselves.
    pub fn gameobject_ender_entries_for_quest_like_cpp(&self, quest_id: u32) -> Vec<u32> {
        let mut entries: Vec<u32> = self
            .gameobject_ender_quests
            .iter()
            .filter_map(|(&entry, quest_ids)| quest_ids.contains(&quest_id).then_some(entry))
            .collect();
        entries.sort_unstable();
        entries
    }

    /// C++ `ObjectMgr::LoadQuestRelationsHelper` insert guard for `gameobject_queststarter`.
    pub fn insert_gameobject_starter_relation_like_cpp(
        &mut self,
        go_entry: u32,
        quest_id: u32,
    ) -> bool {
        if !self.quests.contains_key(&quest_id) {
            return false;
        }

        self.gameobject_starter_quests
            .entry(go_entry)
            .or_default()
            .push(quest_id);
        true
    }

    /// C++ `ObjectMgr::LoadQuestRelationsHelper` insert guard for `gameobject_questender`.
    pub fn insert_gameobject_ender_relation_like_cpp(
        &mut self,
        go_entry: u32,
        quest_id: u32,
    ) -> bool {
        if !self.quests.contains_key(&quest_id) {
            return false;
        }

        self.gameobject_ender_quests
            .entry(go_entry)
            .or_default()
            .push(quest_id);
        true
    }

    /// Whether a given NPC starts a specific quest.
    pub fn creature_has_starter_relation_like_cpp(&self, npc_entry: u32, quest_id: u32) -> bool {
        self.starter_quests
            .get(&npc_entry)
            .is_some_and(|ids| ids.contains(&quest_id))
    }

    /// Whether a given NPC ends a specific quest.
    pub fn creature_has_ender_relation_like_cpp(&self, npc_entry: u32, quest_id: u32) -> bool {
        self.ender_quests
            .get(&npc_entry)
            .is_some_and(|ids| ids.contains(&quest_id))
    }

    /// Whether a given GameObject starts a specific quest.
    pub fn gameobject_has_starter_relation_like_cpp(&self, go_entry: u32, quest_id: u32) -> bool {
        self.gameobject_starter_quests
            .get(&go_entry)
            .is_some_and(|ids| ids.contains(&quest_id))
    }

    /// Whether a given GameObject ends a specific quest.
    pub fn gameobject_has_ender_relation_like_cpp(&self, go_entry: u32, quest_id: u32) -> bool {
        self.gameobject_ender_quests
            .get(&go_entry)
            .is_some_and(|ids| ids.contains(&quest_id))
    }

    /// Whether a given NPC starts any quest.
    pub fn npc_has_start_quests(&self, npc_entry: u32) -> bool {
        self.starter_quests
            .get(&npc_entry)
            .map_or(false, |v| !v.is_empty())
    }

    /// Whether a given NPC ends any quest.
    pub fn npc_has_end_quests(&self, npc_entry: u32) -> bool {
        self.ender_quests
            .get(&npc_entry)
            .map_or(false, |v| !v.is_empty())
    }

    /// Whether a given GameObject starts any quest.
    pub fn gameobject_has_start_quests(&self, go_entry: u32) -> bool {
        self.gameobject_starter_quests
            .get(&go_entry)
            .map_or(false, |v| !v.is_empty())
    }

    /// Whether a given GameObject ends any quest.
    pub fn gameobject_has_end_quests(&self, go_entry: u32) -> bool {
        self.gameobject_ender_quests
            .get(&go_entry)
            .map_or(false, |v| !v.is_empty())
    }
}

impl Default for QuestStore {
    fn default() -> Self {
        Self::new()
    }
}

// ── QuestPoolMgr represented metadata seam ───────────────────────────────────

/// Row from C++ `quest_pool_members` joined to `quest_pool_template`.
///
/// C++ anchor: `QuestPoolMgr::LoadFromDB`, `QuestPools.cpp:75-125`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestPoolMemberRowLikeCpp {
    pub quest_id: u32,
    pub pool_id: u32,
    pub pool_index: u32,
    pub num_active: Option<u32>,
}

/// Row from C++ `pool_quest_save`.
///
/// C++ anchor: `QuestPoolMgr::LoadFromDB`, `QuestPools.cpp:128-160`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QuestPoolSavedActiveRowLikeCpp {
    pub pool_id: u32,
    pub quest_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuestPoolLikeCpp {
    pub pool_id: u32,
    pub num_active: u32,
    pub members: Vec<Vec<u32>>,
    pub active_quests: HashSet<u32>,
}

/// Read-only C++-shaped subset of `QuestPoolMgr` sufficient for `IsQuestActive`.
///
/// This deliberately does not implement C++ regeneration/RNG or DB persistence from
/// `QuestPools.cpp:163-250`; when saved rows are absent/incomplete the store remains a
/// represented snapshot of the metadata and saved active rows supplied by the caller.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct QuestPoolStoreLikeCpp {
    pools: HashMap<u32, QuestPoolLikeCpp>,
    pool_lookup: HashMap<u32, u32>,
}

impl QuestPoolStoreLikeCpp {
    pub fn from_rows_like_cpp(
        quest_store: &QuestStore,
        member_rows: impl IntoIterator<Item = QuestPoolMemberRowLikeCpp>,
        saved_active_rows: impl IntoIterator<Item = QuestPoolSavedActiveRowLikeCpp>,
    ) -> Self {
        let mut pools: HashMap<u32, QuestPoolLikeCpp> = HashMap::new();
        let mut first_valid_pool_kind: HashMap<u32, QuestPoolKindLikeCpp> = HashMap::new();

        for row in member_rows {
            let Some(num_active) = row.num_active else {
                continue;
            };
            let Some(quest) = quest_store.get(row.quest_id) else {
                continue;
            };
            let Some(kind) = QuestPoolKindLikeCpp::from_quest_like_cpp(quest) else {
                continue;
            };

            first_valid_pool_kind.entry(row.pool_id).or_insert(kind);
            let pool = pools
                .entry(row.pool_id)
                .or_insert_with(|| QuestPoolLikeCpp {
                    pool_id: row.pool_id,
                    num_active,
                    members: Vec::new(),
                    active_quests: HashSet::new(),
                });

            let pool_index = row.pool_index as usize;
            if pool_index >= pool.members.len() {
                pool.members.resize_with(pool_index + 1, Vec::new);
            }
            pool.members[pool_index].push(row.quest_id);
        }

        let mut saved_active_by_pool: HashMap<u32, HashSet<u32>> = HashMap::new();
        for row in saved_active_rows {
            if pools.contains_key(&row.pool_id) {
                saved_active_by_pool
                    .entry(row.pool_id)
                    .or_default()
                    .insert(row.quest_id);
            }
        }

        for pool in pools.values_mut() {
            let Some(saved_active) = saved_active_by_pool.get(&pool.pool_id) else {
                continue;
            };

            for member in &pool.members {
                let Some(first_quest_id) = member.first() else {
                    continue;
                };

                if saved_active.contains(first_quest_id) {
                    pool.active_quests.extend(member.iter().copied());
                }
            }
        }

        let mut pool_lookup = HashMap::new();
        for (pool_id, pool) in &pools {
            if first_valid_pool_kind.contains_key(pool_id) {
                for quest_id in pool.members.iter().flatten().copied() {
                    pool_lookup.entry(quest_id).or_insert(*pool_id);
                }
            }
        }

        Self { pools, pool_lookup }
    }

    /// C++ `QuestPoolMgr::IsQuestActive`: non-pooled quests are active; pooled quests are
    /// active iff present in their pool's `activeQuests` set.
    ///
    /// C++ anchor: `QuestPools.cpp:286-292`.
    pub fn is_quest_active_like_cpp(&self, quest_id: u32) -> bool {
        let Some(pool_id) = self.pool_lookup.get(&quest_id) else {
            return true;
        };

        self.pools
            .get(pool_id)
            .is_none_or(|pool| pool.active_quests.contains(&quest_id))
    }

    pub fn is_quest_pooled_like_cpp(&self, quest_id: u32) -> bool {
        self.pool_lookup.contains_key(&quest_id)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QuestPoolKindLikeCpp {
    Daily,
    Weekly,
    Monthly,
}

impl QuestPoolKindLikeCpp {
    fn from_quest_like_cpp(quest: &QuestTemplate) -> Option<Self> {
        if quest.is_daily_like_cpp() {
            Some(Self::Daily)
        } else if quest.is_weekly_like_cpp() {
            Some(Self::Weekly)
        } else if quest.is_monthly_like_cpp() {
            Some(Self::Monthly)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quest_with_sort_and_flags(
        quest_sort_id: i32,
        flags: u32,
        special_flags: u32,
    ) -> QuestTemplate {
        let (flags, special_flags) = normalize_quest_flags_like_cpp(flags, special_flags);
        QuestTemplate {
            id: 1,
            quest_type: 2,
            quest_level: 1,
            quest_max_scaling_level: 0,
            quest_package_id: 0,
            min_level: 1,
            quest_sort_id,
            quest_info_id: 0,
            suggested_group_num: 0,
            reward_next_quest: 0,
            reward_xp_difficulty: 0,
            reward_xp_multiplier: 1.0,
            reward_money_difficulty: 0,
            reward_money_multiplier: 1.0,
            reward_bonus_money: 0,
            reward_display_spell: [0; QUEST_REWARD_DISPLAY_SPELL_COUNT],
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
            flags,
            flags_ex: 0,
            flags_ex2: 0,
            special_flags,
            event_id_for_quest: 0,
            reward_items: [0; QUEST_REWARD_ITEM_COUNT],
            reward_amounts: [0; QUEST_REWARD_ITEM_COUNT],
            reward_currencies: [0; QUEST_REWARD_CURRENCY_COUNT],
            reward_currency_amounts: [0; QUEST_REWARD_CURRENCY_COUNT],
            item_drop: [0; QUEST_ITEM_DROP_COUNT],
            item_drop_quantity: [0; QUEST_ITEM_DROP_COUNT],
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
            reward_choice_items: [(0, 0); QUEST_REWARD_CHOICES_COUNT],
            reward_choice_item_types: [0; QUEST_REWARD_CHOICES_COUNT],
        }
    }

    fn quest_with_id(id: u32) -> QuestTemplate {
        let mut quest = quest_with_sort_and_flags(0, 0, 0);
        quest.id = id;
        quest
    }

    #[test]
    fn decoded_catalog_rows_apply_flags_objectives_and_relations_like_cpp() {
        let quest = quest_with_id(42);
        let objective = QuestObjective {
            id: 7,
            quest_id: 42,
            obj_type: QUEST_OBJECTIVE_AREATRIGGER_LIKE_CPP,
            order: 0,
            storage_index: 0,
            object_id: 99,
            amount: 1,
            flags: 0,
            flags2: 0,
            progress_bar_weight: 0.0,
            description: String::new(),
        };
        let mut store = QuestStore::from_template_rows_like_cpp(vec![quest]);
        store.apply_special_flag_rows_like_cpp(vec![(42, QUEST_SPECIAL_FLAGS_MONTHLY_LIKE_CPP)]);
        store.apply_seasonal_relation_rows_like_cpp(vec![(42, 9)]);
        store.apply_objective_rows_like_cpp(vec![objective]);
        store.apply_creature_starter_rows_like_cpp(vec![(100, 42), (100, 404)]);
        store.apply_creature_ender_rows_like_cpp(vec![(101, 42)]);
        store.apply_gameobject_starter_rows_like_cpp(vec![(200, 42)]);
        store.apply_gameobject_ender_rows_like_cpp(vec![(201, 42)]);

        let quest = store.get(42).unwrap();
        assert!(quest.is_monthly_like_cpp());
        assert!(quest.is_repeatable());
        assert_eq!(quest.event_id_for_quest_like_cpp(), 9);
        assert_eq!(quest.objectives.len(), 1);
        assert_eq!(store.starter_quests.get(&100).unwrap(), &[42]);
        assert_eq!(store.ender_quests.get(&101).unwrap(), &[42]);
        assert_eq!(store.gameobject_starter_quests.get(&200).unwrap(), &[42]);
        assert_eq!(store.gameobject_ender_quests.get(&201).unwrap(), &[42]);
    }

    #[test]
    fn quest_dependent_previous_positive_prev_pushes_current_like_cpp() {
        let previous = quest_with_id(100);
        let mut current = quest_with_id(200);
        current.prev_quest_id = 100;

        let store = QuestStore::from_quests_like_cpp([previous, current]);

        assert_eq!(store.get(200).unwrap().dependent_previous_quests, vec![100]);
    }

    #[test]
    fn quest_dependent_previous_negative_prev_does_not_push_like_cpp() {
        let previous = quest_with_id(100);
        let mut current = quest_with_id(200);
        current.prev_quest_id = -100;

        let store = QuestStore::from_quests_like_cpp([previous, current]);

        assert!(store.get(200).unwrap().dependent_previous_quests.is_empty());
    }

    #[test]
    fn quest_next_quest_id_pushes_source_into_target_dependent_previous_like_cpp() {
        let mut source = quest_with_id(100);
        source.next_quest_id = 200;
        let target = quest_with_id(200);

        let store = QuestStore::from_quests_like_cpp([source, target]);

        assert_eq!(store.get(200).unwrap().dependent_previous_quests, vec![100]);
    }

    #[test]
    fn quest_missing_breadcrumb_target_zeroes_breadcrumb_for_quest_id_like_cpp() {
        let mut breadcrumb = quest_with_id(100);
        breadcrumb.breadcrumb_for_quest_id = 999;

        let store = QuestStore::from_quests_like_cpp([breadcrumb]);

        assert_eq!(store.get(100).unwrap().breadcrumb_for_quest_id, 0);
    }

    #[test]
    fn quest_breadcrumb_chain_informs_each_target_of_source_breadcrumb_like_cpp() {
        let mut source = quest_with_id(100);
        source.breadcrumb_for_quest_id = 200;
        let mut middle = quest_with_id(200);
        middle.breadcrumb_for_quest_id = 300;
        let target = quest_with_id(300);

        let store = QuestStore::from_quests_like_cpp([source, middle, target]);

        assert!(
            store
                .get(200)
                .unwrap()
                .dependent_breadcrumb_quests
                .contains(&100)
        );
        assert!(
            store
                .get(300)
                .unwrap()
                .dependent_breadcrumb_quests
                .contains(&100)
        );
    }

    #[test]
    fn quest_breadcrumb_loop_clears_lowest_source_current_link_without_panic_like_cpp() {
        let mut first = quest_with_id(100);
        first.breadcrumb_for_quest_id = 200;
        let mut second = quest_with_id(200);
        second.breadcrumb_for_quest_id = 100;

        let store = QuestStore::from_quests_like_cpp([first, second]);

        assert_eq!(store.get(100).unwrap().breadcrumb_for_quest_id, 0);
    }

    #[test]
    fn quest_missing_prev_and_next_do_not_fabricate_dependent_previous_like_cpp() {
        let mut quest = quest_with_id(100);
        quest.prev_quest_id = 777;
        quest.next_quest_id = 888;

        let store = QuestStore::from_quests_like_cpp([quest]);

        assert!(store.get(100).unwrap().dependent_previous_quests.is_empty());
    }

    #[test]
    fn seasonal_sort_negative_non_repeatable_is_seasonal_like_cpp() {
        assert!(
            quest_with_sort_and_flags(-QUEST_SORT_SEASONAL_LIKE_CPP, 0, 0).is_seasonal_like_cpp()
        );
    }

    #[test]
    fn seasonal_sort_repeatable_after_object_mgr_normalization_is_not_seasonal_like_cpp() {
        assert!(
            quest_with_sort_and_flags(
                -QUEST_SORT_SEASONAL_LIKE_CPP,
                0,
                QUEST_SPECIAL_FLAGS_REPEATABLE_LIKE_CPP
            )
            .is_repeatable()
        );
        assert!(
            !quest_with_sort_and_flags(
                -QUEST_SORT_SEASONAL_LIKE_CPP,
                0,
                QUEST_SPECIAL_FLAGS_REPEATABLE_LIKE_CPP
            )
            .is_seasonal_like_cpp()
        );
        assert!(
            !quest_with_sort_and_flags(
                -QUEST_SORT_SEASONAL_LIKE_CPP,
                QUEST_FLAGS_DAILY_LIKE_CPP,
                0
            )
            .is_seasonal_like_cpp()
        );
        assert!(
            !quest_with_sort_and_flags(
                -QUEST_SORT_SEASONAL_LIKE_CPP,
                QUEST_FLAGS_WEEKLY_LIKE_CPP,
                0
            )
            .is_seasonal_like_cpp()
        );
        assert!(
            !quest_with_sort_and_flags(
                -QUEST_SORT_SEASONAL_LIKE_CPP,
                0,
                QUEST_SPECIAL_FLAGS_MONTHLY_LIKE_CPP
            )
            .is_seasonal_like_cpp()
        );
    }

    #[test]
    fn seasonal_object_mgr_normalization_masks_special_flags_and_daily_weekly_prefers_weekly_like_cpp()
     {
        let disallowed_special_bit = 0x0000_0020;
        let quest = quest_with_sort_and_flags(
            -QUEST_SORT_SEASONAL_LIKE_CPP,
            QUEST_FLAGS_DAILY_LIKE_CPP | QUEST_FLAGS_WEEKLY_LIKE_CPP,
            disallowed_special_bit,
        );

        assert_eq!(quest.flags & QUEST_FLAGS_DAILY_LIKE_CPP, 0);
        assert_ne!(quest.flags & QUEST_FLAGS_WEEKLY_LIKE_CPP, 0);
        assert_eq!(quest.special_flags & disallowed_special_bit, 0);
        assert!(quest.is_repeatable());
        assert!(!quest.is_seasonal_like_cpp());
    }

    #[test]
    fn seasonal_sort_i32_min_is_not_seasonal_and_does_not_panic_like_cpp() {
        assert!(!quest_with_sort_and_flags(i32::MIN, 0, 0).is_seasonal_like_cpp());
    }

    #[test]
    fn non_seasonal_sort_is_not_seasonal_like_cpp() {
        assert!(!quest_with_sort_and_flags(-101, 0, 0).is_seasonal_like_cpp());
        assert!(
            !quest_with_sort_and_flags(QUEST_SORT_SEASONAL_LIKE_CPP, 0, 0).is_seasonal_like_cpp()
        );
    }

    #[test]
    fn love_is_in_the_air_sort_is_seasonal_like_cpp() {
        assert!(
            quest_with_sort_and_flags(-QUEST_SORT_LOVE_IS_IN_THE_AIR_LIKE_CPP, 0, 0)
                .is_seasonal_like_cpp()
        );
    }

    #[test]
    fn event_id_for_quest_defaults_zero_like_cpp() {
        assert_eq!(
            quest_with_sort_and_flags(-QUEST_SORT_SEASONAL_LIKE_CPP, 0, 0)
                .event_id_for_quest_like_cpp(),
            0
        );
    }

    fn daily_quest(id: u32) -> QuestTemplate {
        let mut quest = quest_with_sort_and_flags(0, QUEST_FLAGS_DAILY_LIKE_CPP, 0);
        quest.id = id;
        quest
    }

    fn weekly_quest(id: u32) -> QuestTemplate {
        let mut quest = quest_with_sort_and_flags(0, QUEST_FLAGS_WEEKLY_LIKE_CPP, 0);
        quest.id = id;
        quest
    }

    #[test]
    fn quest_source_item_missing_zeroes_item_id_but_keeps_count_like_cpp() {
        let mut quest = quest_with_id(300);
        quest.source_item_id = 700;
        quest.source_item_count = 4;

        quest.normalize_source_item_spell_like_cpp(|_| false, |_| true);

        assert_eq!(quest.source_item_id, 0);
        // C++ uses an inner if/else-if; after clearing the id, it does not run the outer
        // `sourceItemId == 0 && count > 0` branch in the same iteration.
        assert_eq!(quest.source_item_count, 4);
    }

    #[test]
    fn quest_source_item_existing_with_zero_count_sets_one_like_cpp() {
        let mut quest = quest_with_id(301);
        quest.source_item_id = 701;
        quest.source_item_count = 0;

        quest.normalize_source_item_spell_like_cpp(|item_id| item_id == 701, |_| true);

        assert_eq!(quest.source_item_id, 701);
        assert_eq!(quest.source_item_count, 1);
    }

    #[test]
    fn quest_source_item_zero_with_positive_count_clears_count_like_cpp() {
        let mut quest = quest_with_id(302);
        quest.source_item_id = 0;
        quest.source_item_count = 3;

        quest.normalize_source_item_spell_like_cpp(|_| true, |_| true);

        assert_eq!(quest.source_item_id, 0);
        assert_eq!(quest.source_item_count, 0);
    }

    #[test]
    fn quest_source_spell_invalid_zeroes_spell_like_cpp() {
        let mut quest = quest_with_id(303);
        quest.source_spell_id = 900;

        quest.normalize_source_item_spell_like_cpp(|_| true, |_| false);

        assert_eq!(quest.source_spell_id, 0);
    }

    #[test]
    fn quest_source_spell_valid_is_preserved_like_cpp() {
        let mut quest = quest_with_id(304);
        quest.source_spell_id = 901;

        quest.normalize_source_item_spell_like_cpp(|_| true, |spell_id| spell_id == 901);

        assert_eq!(quest.source_spell_id, 901);
    }

    #[test]
    fn quest_source_fields_can_be_constructed_raw_before_normalization() {
        let mut quest = quest_with_id(305);
        quest.source_item_id = 702;
        quest.source_item_count = 0;
        quest.source_spell_id = 902;

        assert_eq!(quest.source_item_id, 702);
        assert_eq!(quest.source_item_count, 0);
        assert_eq!(quest.source_spell_id, 902);
    }

    #[test]
    fn quest_store_source_item_spell_normalization_uses_caller_predicates_like_cpp() {
        let mut invalid_item = quest_with_id(306);
        invalid_item.source_item_id = 703;
        invalid_item.source_item_count = 5;
        let mut valid_item_zero_count = quest_with_id(307);
        valid_item_zero_count.source_item_id = 704;
        valid_item_zero_count.source_item_count = 0;
        let mut invalid_spell = quest_with_id(308);
        invalid_spell.source_spell_id = 903;

        let mut store =
            QuestStore::from_quests_like_cpp([invalid_item, valid_item_zero_count, invalid_spell]);
        store.normalize_source_item_spell_metadata_like_cpp(|item_id| item_id == 704, |_| false);

        assert_eq!(store.get(306).unwrap().source_item_id, 0);
        assert_eq!(store.get(306).unwrap().source_item_count, 5);
        assert_eq!(store.get(307).unwrap().source_item_id, 704);
        assert_eq!(store.get(307).unwrap().source_item_count, 1);
        assert_eq!(store.get(308).unwrap().source_spell_id, 0);
    }

    #[test]
    fn quest_pool_non_pooled_quest_is_active_like_cpp() {
        let store = QuestStore::from_quests_like_cpp([daily_quest(100)]);
        let pools = QuestPoolStoreLikeCpp::from_rows_like_cpp(&store, [], []);

        assert!(!pools.is_quest_pooled_like_cpp(100));
        assert!(pools.is_quest_active_like_cpp(100));
    }

    #[test]
    fn quest_pool_pooled_saved_active_quest_is_active_like_cpp() {
        let store = QuestStore::from_quests_like_cpp([daily_quest(101), daily_quest(102)]);
        let pools = QuestPoolStoreLikeCpp::from_rows_like_cpp(
            &store,
            [
                QuestPoolMemberRowLikeCpp {
                    quest_id: 101,
                    pool_id: 7,
                    pool_index: 0,
                    num_active: Some(1),
                },
                QuestPoolMemberRowLikeCpp {
                    quest_id: 102,
                    pool_id: 7,
                    pool_index: 1,
                    num_active: Some(1),
                },
            ],
            [QuestPoolSavedActiveRowLikeCpp {
                pool_id: 7,
                quest_id: 101,
            }],
        );

        assert!(pools.is_quest_pooled_like_cpp(101));
        assert!(pools.is_quest_active_like_cpp(101));
    }

    #[test]
    fn quest_pool_pooled_not_saved_active_quest_is_inactive_like_cpp() {
        let store = QuestStore::from_quests_like_cpp([daily_quest(103), daily_quest(104)]);
        let pools = QuestPoolStoreLikeCpp::from_rows_like_cpp(
            &store,
            [
                QuestPoolMemberRowLikeCpp {
                    quest_id: 103,
                    pool_id: 8,
                    pool_index: 0,
                    num_active: Some(1),
                },
                QuestPoolMemberRowLikeCpp {
                    quest_id: 104,
                    pool_id: 8,
                    pool_index: 1,
                    num_active: Some(1),
                },
            ],
            [QuestPoolSavedActiveRowLikeCpp {
                pool_id: 8,
                quest_id: 103,
            }],
        );

        assert!(pools.is_quest_pooled_like_cpp(104));
        assert!(!pools.is_quest_active_like_cpp(104));
    }

    #[test]
    fn quest_pool_saved_first_quest_activates_entire_member_index_like_cpp() {
        let store = QuestStore::from_quests_like_cpp([daily_quest(201), daily_quest(202)]);
        let pools = QuestPoolStoreLikeCpp::from_rows_like_cpp(
            &store,
            [
                QuestPoolMemberRowLikeCpp {
                    quest_id: 201,
                    pool_id: 12,
                    pool_index: 0,
                    num_active: Some(1),
                },
                QuestPoolMemberRowLikeCpp {
                    quest_id: 202,
                    pool_id: 12,
                    pool_index: 0,
                    num_active: Some(1),
                },
            ],
            [QuestPoolSavedActiveRowLikeCpp {
                pool_id: 12,
                quest_id: 201,
            }],
        );

        assert!(pools.is_quest_pooled_like_cpp(201));
        assert!(pools.is_quest_pooled_like_cpp(202));
        assert!(pools.is_quest_active_like_cpp(201));
        assert!(pools.is_quest_active_like_cpp(202));
    }

    #[test]
    fn quest_pool_saved_non_first_quest_does_not_activate_member_index_like_cpp() {
        let store = QuestStore::from_quests_like_cpp([daily_quest(203), daily_quest(204)]);
        let pools = QuestPoolStoreLikeCpp::from_rows_like_cpp(
            &store,
            [
                QuestPoolMemberRowLikeCpp {
                    quest_id: 203,
                    pool_id: 13,
                    pool_index: 0,
                    num_active: Some(1),
                },
                QuestPoolMemberRowLikeCpp {
                    quest_id: 204,
                    pool_id: 13,
                    pool_index: 0,
                    num_active: Some(1),
                },
            ],
            [QuestPoolSavedActiveRowLikeCpp {
                pool_id: 13,
                quest_id: 204,
            }],
        );

        assert!(pools.is_quest_pooled_like_cpp(203));
        assert!(pools.is_quest_pooled_like_cpp(204));
        assert!(!pools.is_quest_active_like_cpp(203));
        assert!(!pools.is_quest_active_like_cpp(204));
    }

    #[test]
    fn quest_pool_missing_quest_and_pool_rows_skip_without_panic_like_cpp() {
        let store = QuestStore::from_quests_like_cpp([weekly_quest(105)]);
        let pools = QuestPoolStoreLikeCpp::from_rows_like_cpp(
            &store,
            [
                QuestPoolMemberRowLikeCpp {
                    quest_id: 999,
                    pool_id: 9,
                    pool_index: 0,
                    num_active: Some(1),
                },
                QuestPoolMemberRowLikeCpp {
                    quest_id: 105,
                    pool_id: 10,
                    pool_index: 0,
                    num_active: None,
                },
            ],
            [QuestPoolSavedActiveRowLikeCpp {
                pool_id: 404,
                quest_id: 105,
            }],
        );

        assert!(!pools.is_quest_pooled_like_cpp(105));
        assert!(pools.is_quest_active_like_cpp(105));
        assert!(pools.is_quest_active_like_cpp(999));
    }

    #[test]
    fn quest_pool_non_daily_weekly_monthly_member_is_skipped_and_active_like_cpp() {
        let mut normal = quest_with_sort_and_flags(0, 0, 0);
        normal.id = 106;
        let store = QuestStore::from_quests_like_cpp([normal]);
        let pools = QuestPoolStoreLikeCpp::from_rows_like_cpp(
            &store,
            [QuestPoolMemberRowLikeCpp {
                quest_id: 106,
                pool_id: 11,
                pool_index: 0,
                num_active: Some(1),
            }],
            [QuestPoolSavedActiveRowLikeCpp {
                pool_id: 11,
                quest_id: 106,
            }],
        );

        assert!(!pools.is_quest_pooled_like_cpp(106));
        assert!(pools.is_quest_active_like_cpp(106));
    }

    #[test]
    fn gameobject_quest_relations_skip_missing_quests_and_stay_separate_from_creatures_like_cpp() {
        let mut quest_two = quest_with_sort_and_flags(0, 0, 0);
        quest_two.id = 2;
        let mut store =
            QuestStore::from_quests_like_cpp([quest_with_sort_and_flags(0, 0, 0), quest_two]);

        store.starter_quests.entry(10).or_default().push(1);
        store.ender_quests.entry(10).or_default().push(2);

        assert!(store.insert_gameobject_starter_relation_like_cpp(1000, 1));
        assert!(store.insert_gameobject_ender_relation_like_cpp(1000, 2));
        assert!(!store.insert_gameobject_starter_relation_like_cpp(1000, 999));
        assert!(!store.insert_gameobject_ender_relation_like_cpp(1000, 999));

        assert_eq!(
            store
                .quests_for_gameobject_starter(1000)
                .into_iter()
                .map(|quest| quest.id)
                .collect::<Vec<_>>(),
            vec![1]
        );
        assert_eq!(
            store
                .quests_for_gameobject_ender(1000)
                .into_iter()
                .map(|quest| quest.id)
                .collect::<Vec<_>>(),
            vec![2]
        );
        assert_eq!(
            store
                .quests_for_starter(10)
                .into_iter()
                .map(|quest| quest.id)
                .collect::<Vec<_>>(),
            vec![1]
        );
        assert_eq!(
            store
                .quests_for_ender(10)
                .into_iter()
                .map(|quest| quest.id)
                .collect::<Vec<_>>(),
            vec![2]
        );
        assert!(store.gameobject_has_start_quests(1000));
        assert!(store.gameobject_has_end_quests(1000));
        assert!(!store.gameobject_has_start_quests(2000));
        assert!(!store.gameobject_has_end_quests(2000));
        assert!(!store.npc_has_start_quests(1000));
        assert!(!store.npc_has_end_quests(1000));
    }
}
