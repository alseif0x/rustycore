// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

/// A single objective definition. Canonical value type, reexported by wow-data.
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

/// Borrowed objective metadata needed by the represented progress rules.
///
/// This is an immutable call input, not another quest catalog or Player state.
/// Objectives keep their original order and allocation; no list is cloned.
#[derive(Debug, Clone, Copy)]
pub struct QuestObjectiveRulesLikeCpp<'a> {
    pub(super) id: u32,
    pub(super) flags: u32,
    pub(super) limit_time_secs: i64,
    repeatable: bool,
    pub(super) objectives: &'a [QuestObjective],
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

    pub(super) fn is_repeatable(&self) -> bool {
        self.repeatable
    }

    pub(crate) fn objectives_like_cpp(&self) -> &'a [QuestObjective] {
        self.objectives
    }
}
