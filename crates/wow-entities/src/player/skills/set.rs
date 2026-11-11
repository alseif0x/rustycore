//! 02245dcd Player.cpp:5768-6045. One synchronous SetSkill coordinator.
//! Mandatory effect capability: no default/no-op production executor exists.
mod apply;
mod defaults;
mod professions;
mod rewards;
#[cfg(test)]
#[path = "../../../unit_tests/player/skills/set/mod.rs"]
mod tests;
use super::{PlayerSkills, SkillSetSource, SkillUpdate, classic_child};
use std::collections::BTreeSet;

pub use crate::SkillSetError;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkillSetOutcome {
    Finished,
    MissingLine,
    NoFreeSlot,
    InventoryFull,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkillCriteria {
    Raised,
    AchieveStep,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProfessionItemMove {
    StoredOrAbsent,
    InventoryFull,
}

/// Required Player effects for the complete SetSkill use case. Implementations
/// execute against the caller's same canonical Player, including reentrant
/// learning. They must not fake completion, retain skill-state copies, defer
/// effects into a success queue or hold map/entity guards during delivery/I/O.
/// There is no production implementation yet: creation stays disabled.
pub trait SkillSetEffects {
    type Error;
    /// Unit.h:769 returns uint8, not the wider stored Level field.
    fn player_level(&self) -> u8;
    fn is_in_world(&self) -> bool;
    fn update_enchantments(
        &mut self,
        skills: &mut PlayerSkills,
        skill: u32,
        old: u16,
        new: u16,
    ) -> Result<(), Self::Error>;
    /// Source ConditionMgr::IsPlayerMeetingCondition on the same Player.
    /// Unavailable evaluation is an error, not true/false fabricated admission.
    fn meets_player_condition(
        &mut self,
        skills: &mut PlayerSkills,
        condition: u32,
    ) -> Result<bool, Self::Error>;
    /// IsObjectMeetingNotGroupedConditions source type 35, entry=ability ID.
    /// The concrete ConditionMgr owns missing-list/ElseGroup/negative rules.
    fn meets_skill_ability_conditions(
        &mut self,
        skills: &mut PlayerSkills,
        ability: u32,
    ) -> Result<bool, Self::Error>;
    /// Source AddSpell(spell,true,true,true,false,false,fromSkill). Its bool
    /// means visible active admission, not durable success, and is ignored here.
    fn add_reward_spell(
        &mut self,
        skills: &mut PlayerSkills,
        spell: u32,
        from_skill: u32,
    ) -> Result<bool, Self::Error>;
    /// Source LearnSpell(spell,true,fromSkill), including live notifications.
    fn learn_reward_spell(
        &mut self,
        skills: &mut PlayerSkills,
        spell: u32,
        from_skill: u32,
    ) -> Result<(), Self::Error>;
    fn update_mount_capability(&mut self, skills: &mut PlayerSkills) -> Result<(), Self::Error>;
    fn update_criteria(
        &mut self,
        skills: &mut PlayerSkills,
        skill: u32,
        criterion: SkillCriteria,
    ) -> Result<(), Self::Error>;
    /// For this aura type, iterate source GetAuraEffectsByType order, filter
    /// MiscValue == int32(skill), then HandleEffect(SKILL,true). Called for
    /// types 30, 400, 98 in that order; mutations use these same skill fields.
    fn refresh_bonus_auras(
        &mut self,
        skills: &mut PlayerSkills,
        skill: u32,
        aura_type: u32,
    ) -> Result<(), Self::Error>;
    /// Source bag 0/slot: absent => StoredOrAbsent. Otherwise CanStoreItem
    /// (NULL_BAG,NULL_SLOT,item,false), then RemoveItem and StoreItem, both
    /// update=true. On full bags, preserve earlier moves and do not move this
    /// item. No all-or-nothing inventory plan or invented spare destination.
    fn store_profession_item(
        &mut self,
        skills: &mut PlayerSkills,
        slot: u8,
    ) -> Result<ProfessionItemMove, Self::Error>;
    fn display_inventory_full(&mut self, skills: &mut PlayerSkills) -> Result<(), Self::Error>;
    /// Source RemoveSpell(first-in-chain), with its ordinary default flags.
    fn remove_spell(&mut self, skills: &mut PlayerSkills, spell: u32) -> Result<(), Self::Error>;
}

impl PlayerSkills {
    /// Execute source SetSkill fields/state and mandatory ordered effects.
    /// Not a save or full Player constructor. Effects/cycle errors retain the
    /// completed prefix; discard an unadmitted construction instead of saving,
    /// retrying, publishing success or pretending atomic rollback. Ordinary
    /// inventory/no-slot returns still execute source final child sync.
    pub fn set_skill<E: SkillSetEffects, S: SkillSetSource>(
        &mut self,
        input: SkillUpdate,
        sources: &S,
        effects: &mut E,
    ) -> Result<SkillSetOutcome, SkillSetError<E::Error, S::Error>> {
        self.set_inner(input, sources, effects, &mut BTreeSet::new())
    }

    fn set_inner<E: SkillSetEffects, S: SkillSetSource>(
        &mut self,
        input: SkillUpdate,
        sources: &S,
        effects: &mut E,
        active: &mut BTreeSet<u32>,
    ) -> Result<SkillSetOutcome, SkillSetError<E::Error, S::Error>> {
        let Some(line) = sources.skill_line(input.skill) else {
            return Ok(SkillSetOutcome::MissingLine); // Before source RAII guard.
        };
        let child = classic_child(sources, line.parent_skill);
        let input = self
            .normalize_classic_skill_update(sources, input)
            .expect("same immutable line");
        if !active.insert(input.skill) {
            return Err(SkillSetError::RecursiveSkillCycle);
        }
        let result = self.apply(input, &line, child, sources, effects, active);
        let result = match result {
            Ok(outcome) if !child => self
                .sync_children(input.skill, sources, effects, active)
                .map(|()| outcome),
            other => other,
        };
        active.remove(&input.skill);
        result
    }

    fn sync_children<E: SkillSetEffects, S: SkillSetSource>(
        &mut self,
        skill: u32,
        sources: &S,
        effects: &mut E,
        active: &mut BTreeSet<u32>,
    ) -> Result<(), SkillSetError<E::Error, S::Error>> {
        let Some(line) = sources.skill_line(skill) else {
            return Ok(());
        };
        if line.parent_skill != 0 || !matches!(line.category, 9 | 11) {
            return Ok(());
        }
        // Source captures parent pure values once, then queries each child live.
        let step = self.step(skill);
        let value = self.pure_value(skill);
        let maximum = self.pure_maximum(skill);
        for child in sources.child_lines(skill) {
            if self.step(child.id) != step
                || self.pure_value(child.id) != value
                || self.pure_maximum(child.id) != maximum
            {
                self.set_inner(
                    SkillUpdate {
                        skill: child.id,
                        step,
                        value,
                        maximum,
                    },
                    sources,
                    effects,
                    active,
                )?;
            }
        }
        Ok(())
    }
}
