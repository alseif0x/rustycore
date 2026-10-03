//! 02245dcd Player.cpp:5750-5764,6077-6189; Player.h:687-704,2409-2422.
//! One owned field array and ID/slot/status index; no rank-value mirror.
mod bonus;
mod set;
#[cfg(test)]
mod tests;
use crate::forever::creation::InitialSkillFields;
pub use bonus::SkillBonusError;
pub use set::{
    ProfessionItemMove, SkillCriteria, SkillSetEffects, SkillSetError, SkillSetInputError,
    SkillSetOutcome, SkillSetSources,
};
use std::collections::HashMap;
use wow_data::forever_birth::BirthCatalog;

// Source PLAYER_MAX_SKILLS and UF::SkillInfo array width. Not independent
// proof of the native 70170 update-field wire or an instantiated full Player.
pub const PLAYER_MAX_SKILLS: usize = 300;

#[repr(u8)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SkillUpdateState {
    #[default]
    Unchanged = 0,
    Changed = 1,
    New = 2,
    Deleted = 3,
}
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct SkillFields {
    line: u16,
    step: u16,
    rank: u16,
    starting_rank: u16,
    maximum: u16,
    temporary_bonus: i16,
    permanent_bonus: u16,
}
impl SkillFields {
    pub fn line(&self) -> u16 {
        self.line
    }
    pub fn step(&self) -> u16 {
        self.step
    }
    pub fn rank(&self) -> u16 {
        self.rank
    }
    pub fn starting_rank(&self) -> u16 {
        self.starting_rank
    }
    pub fn maximum(&self) -> u16 {
        self.maximum
    }
    pub fn temporary_bonus(&self) -> i16 {
        self.temporary_bonus
    }
    // Source field/get-by-pos is uint16, while GetSkillPermBonusValue narrows
    // to int16. Keep that distinction for source arithmetic and persistence.
    pub fn permanent_bonus(&self) -> u16 {
        self.permanent_bonus
    }
}
struct Status {
    slot: u16,
    state: SkillUpdateState,
}

/// A source SetSkill input, not proof of an applied or learned skill.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkillUpdate {
    pub skill: u32,
    pub step: u16,
    pub value: u16,
    pub maximum: u16,
}
pub struct PlayerSkills {
    fields: Box<[SkillFields; PLAYER_MAX_SKILLS]>,
    status: HashMap<u32, Status>,
    // Source ActivePlayerData::ProfessionSkillLine, not a Session/inventory mirror.
    professions: [i32; 2],
}
impl PlayerSkills {
    /// Player.cpp:5780-5793: Classic child admission uses the parent's pure
    /// fields, never bonuses or the requested child rank. A zero-value
    /// request is not rewritten. None means the source early missing-line return.
    /// This query does not execute SetSkill, rewards, aura/enchantment/criteria
    /// effects, profession inventory handling or RAII final child synchronization.
    pub fn normalize_classic_skill_update(
        &self,
        birth: &BirthCatalog,
        mut update: SkillUpdate,
    ) -> Option<SkillUpdate> {
        let line = birth.skill_line(update.skill)?;
        if update.value != 0 && classic_child(birth, line.parent_skill) {
            if let Some(parent) = self.live_field(line.parent_skill) {
                update.step = parent.step;
                update.value = parent.rank;
                update.maximum = parent.maximum;
            } else {
                update.step = 0;
                update.value = 0;
                update.maximum = 0;
            }
        }
        Some(update)
    }

    /// Player.cpp:6041-6045. Missing line/parent retain the supplied identity.
    pub fn classic_profession_skill(birth: &BirthCatalog, skill: u32) -> u32 {
        let Some(line) = birth.skill_line(skill) else {
            return skill;
        };
        if classic_child(birth, line.parent_skill) {
            line.parent_skill
        } else {
            skill
        }
    }
    /// Execute InitializeSkillFields only. Blueprint default requests are NOT
    /// applied or saved; HasSkill stays false until real SetSkill effects run.
    /// Caller owns this component as part of the in-construction Player.
    pub fn initialize(initial: &InitialSkillFields) -> Self {
        Self::from_ids(initial.fields().iter().map(|seed| seed.skill()))
    }
    fn from_ids(ids: impl IntoIterator<Item = u32>) -> Self {
        let mut result = Self {
            fields: Box::new([SkillFields::default(); PLAYER_MAX_SKILLS]),
            status: HashMap::new(),
            professions: [0; 2],
        };
        for (slot, skill) in ids.into_iter().take(PLAYER_MAX_SKILLS).enumerate() {
            result.fields[slot].line = skill as u16;
            result.fields[slot].starting_rank = 1;
            result.status.entry(skill).or_insert(Status {
                slot: slot as u16,
                state: SkillUpdateState::Unchanged,
            });
        }
        result
    }
    pub fn fields(&self) -> &[SkillFields; PLAYER_MAX_SKILLS] {
        &self.fields
    }
    /// Read-only identity/state projection; no exposed mutable rank index.
    /// There is deliberately no unordered-status iteration/persistence order
    /// promise. Save must establish its separate source-order/transaction fence.
    pub fn status(&self, skill: u32) -> Option<(u16, SkillUpdateState)> {
        self.status
            .get(&skill)
            .map(|status| (status.slot, status.state))
    }
    fn live_field(&self, skill: u32) -> Option<&SkillFields> {
        if skill == 0 {
            return None;
        }
        let status = self.status.get(&skill)?;
        if status.state == SkillUpdateState::Deleted {
            return None;
        }
        let field = &self.fields[usize::from(status.slot)];
        (field.rank != 0).then_some(field)
    }
    pub fn has_skill(&self, skill: u32) -> bool {
        self.live_field(skill).is_some()
    }
    pub fn step(&self, skill: u32) -> u16 {
        self.live_field(skill).map_or(0, |field| field.step)
    }
    pub fn pure_value(&self, skill: u32) -> u16 {
        self.live_field(skill).map_or(0, |field| field.rank)
    }
    pub fn pure_maximum(&self, skill: u32) -> u16 {
        self.live_field(skill).map_or(0, |field| field.maximum)
    }
    pub fn temporary_bonus(&self, skill: u32) -> i16 {
        self.live_field(skill)
            .map_or(0, |field| field.temporary_bonus)
    }
    pub fn permanent_bonus(&self, skill: u32) -> i16 {
        self.live_field(skill)
            .map_or(0, |field| field.permanent_bonus as i16)
    }
    pub fn value(&self, skill: u32) -> u16 {
        self.live_field(skill)
            .map_or(0, |field| value(field, field.rank, true))
    }
    pub fn maximum(&self, skill: u32) -> u16 {
        self.live_field(skill)
            .map_or(0, |field| value(field, field.maximum, true))
    }
    pub fn base_value(&self, skill: u32) -> u16 {
        self.live_field(skill)
            .map_or(0, |field| value(field, field.rank, false))
    }
}

fn classic_child(birth: &BirthCatalog, parent: u32) -> bool {
    parent != 0
        && birth
            .skill_line(parent)
            .is_some_and(|line| matches!(line.category, 9 | 11))
}

fn value(field: &SkillFields, rank: u16, temporary: bool) -> u16 {
    let mut result = i32::from(rank) + i32::from(field.permanent_bonus);
    if temporary {
        result += i32::from(field.temporary_bonus);
    }
    // Source clamps only negative int32 results, then return-type uint16
    // narrowing wraps high bits. It does not saturate at maximum or u16::MAX.
    if result < 0 { 0 } else { result as u16 }
}
