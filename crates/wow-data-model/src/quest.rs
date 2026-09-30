/// A single static quest objective definition loaded from `quest_objectives`.
///
/// Target reference: `Quests/QuestDef.h`, `QuestObjective` (442). This retains
/// the existing Rust representation, including loader ordering metadata.
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

impl QuestObjective {
    /// C++ `QuestObjective::IsStoringFlag` (`QuestDef.h:477`).
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

/// Borrowed immutable quest metadata consumed by entity-owned progress rules.
#[derive(Debug, Clone, Copy)]
pub struct QuestObjectiveRulesLikeCpp<'a> {
    id: u32,
    flags: u32,
    limit_time_secs: i64,
    repeatable: bool,
    objectives: &'a [QuestObjective],
}

impl<'a> QuestObjectiveRulesLikeCpp<'a> {
    pub fn new(
        id: u32,
        flags: u32,
        limit_time_secs: i64,
        repeatable: bool,
        objectives: &'a [QuestObjective],
    ) -> Self {
        Self {
            id,
            flags,
            limit_time_secs,
            repeatable,
            objectives,
        }
    }

    pub const fn id(&self) -> u32 {
        self.id
    }

    pub const fn flags(&self) -> u32 {
        self.flags
    }

    pub const fn limit_time_secs(&self) -> i64 {
        self.limit_time_secs
    }

    pub const fn is_repeatable_like_cpp(&self) -> bool {
        self.repeatable
    }

    pub const fn objectives(&self) -> &'a [QuestObjective] {
        self.objectives
    }
}

/// The represented status gate's reason for rejecting quest acceptance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestStatusBlock {
    AlreadyRewarded,
    AlreadyActive,
}

/// The represented day gate's reason for rejecting quest acceptance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuestDayCooldownBlock {
    DungeonFinder,
    Daily,
}

/// Borrowed quest metadata used by Player's ordered acceptance predicates.
///
/// The quest catalog owns the full row and the normalized relationship lists;
/// this view lets entity rules read only the fields they need without cloning
/// or depending on the catalog crate.
#[derive(Debug, Clone, Copy)]
pub struct QuestEligibilityRules<'a> {
    id: u32,
    repeatable: bool,
    exclusive_group: i32,
    previous_quest_id: i32,
    dependent_previous_quest_ids: &'a [u32],
    dependent_breadcrumb_quest_ids: &'a [u32],
    daily: bool,
    dungeon_finder: bool,
    weekly: bool,
    monthly: bool,
    seasonal: bool,
    event_id: u16,
}

impl<'a> QuestEligibilityRules<'a> {
    #[allow(clippy::too_many_arguments)]
    pub const fn new(
        id: u32,
        repeatable: bool,
        exclusive_group: i32,
        previous_quest_id: i32,
        dependent_previous_quest_ids: &'a [u32],
        dependent_breadcrumb_quest_ids: &'a [u32],
        daily: bool,
        dungeon_finder: bool,
        weekly: bool,
        monthly: bool,
        seasonal: bool,
        event_id: u16,
    ) -> Self {
        Self {
            id,
            repeatable,
            exclusive_group,
            previous_quest_id,
            dependent_previous_quest_ids,
            dependent_breadcrumb_quest_ids,
            daily,
            dungeon_finder,
            weekly,
            monthly,
            seasonal,
            event_id,
        }
    }

    pub const fn id(&self) -> u32 {
        self.id
    }

    pub const fn is_repeatable(&self) -> bool {
        self.repeatable
    }

    pub const fn exclusive_group(&self) -> i32 {
        self.exclusive_group
    }

    pub const fn previous_quest_id(&self) -> i32 {
        self.previous_quest_id
    }

    pub const fn dependent_previous_quest_ids(&self) -> &'a [u32] {
        self.dependent_previous_quest_ids
    }

    pub const fn dependent_breadcrumb_quest_ids(&self) -> &'a [u32] {
        self.dependent_breadcrumb_quest_ids
    }

    pub const fn is_daily(&self) -> bool {
        self.daily
    }

    pub const fn is_dungeon_finder(&self) -> bool {
        self.dungeon_finder
    }

    pub const fn is_weekly(&self) -> bool {
        self.weekly
    }

    pub const fn is_monthly(&self) -> bool {
        self.monthly
    }

    pub const fn is_seasonal(&self) -> bool {
        self.seasonal
    }

    pub const fn event_id(&self) -> u16 {
        self.event_id
    }
}

/// Only scalar quest metadata used by XP/money valuation; no catalog or state.
#[derive(Debug, Clone, Copy)]
pub struct QuestRewardRules {
    quest_level: i32,
    max_scaling_level: i32,
    dungeon_finder: bool,
    xp_difficulty: u32,
    xp_multiplier: f32,
    money_difficulty: u32,
    money_multiplier: f32,
}

impl QuestRewardRules {
    pub const fn new(
        quest_level: i32,
        max_scaling_level: i32,
        dungeon_finder: bool,
        xp_difficulty: u32,
        xp_multiplier: f32,
        money_difficulty: u32,
        money_multiplier: f32,
    ) -> Self {
        Self {
            quest_level,
            max_scaling_level,
            dungeon_finder,
            xp_difficulty,
            xp_multiplier,
            money_difficulty,
            money_multiplier,
        }
    }

    pub const fn quest_level(&self) -> i32 {
        self.quest_level
    }

    pub const fn max_scaling_level(&self) -> i32 {
        self.max_scaling_level
    }

    pub const fn is_dungeon_finder(&self) -> bool {
        self.dungeon_finder
    }

    pub const fn xp_difficulty(&self) -> u32 {
        self.xp_difficulty
    }

    pub const fn xp_multiplier(&self) -> f32 {
        self.xp_multiplier
    }

    pub const fn money_difficulty(&self) -> u32 {
        self.money_difficulty
    }

    pub const fn money_multiplier(&self) -> f32 {
        self.money_multiplier
    }
}

mod template;
pub use template::{QuestInfoEntry, QuestTemplate};

#[cfg(test)]
#[path = "quest/tests.rs"]
mod tests;
