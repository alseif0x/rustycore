//! Source stored/absent branches; source final synchronization is in set.rs.
use super::super::{SkillFields, SkillUpdateState, Status};
use super::{
    PlayerSkills, ProfessionItemMove, SkillCriteria, SkillSetEffects, SkillSetError,
    SkillSetInputError, SkillSetOutcome, SkillSetSources, SkillUpdate,
};
use std::collections::BTreeSet;
use wow_data::forever_birth::SkillLineRecord;

impl PlayerSkills {
    pub(super) fn apply<E: SkillSetEffects>(
        &mut self,
        input: SkillUpdate,
        line: &SkillLineRecord,
        classic_child: bool,
        sources: &SkillSetSources<'_>,
        effects: &mut E,
        active: &mut BTreeSet<u32>,
    ) -> Result<SkillSetOutcome, SkillSetError<E::Error>> {
        if let Some(slot) = self.status.get(&input.skill).map(|status| status.slot) {
            let old = self.fields[usize::from(slot)].rank;
            if input.value != 0 {
                if line.parent_skill != 0
                    && !classic_child
                    && line.parent_tier_index > 0
                    && i32::from(self.step(line.parent_skill)) < line.parent_tier_index
                {
                    self.enable_parent(line, classic_child, sources, effects, active)?;
                }
                if input.value < old {
                    effects
                        .update_enchantments(self, input.skill, old, input.value)
                        .map_err(SkillSetError::Effect)?;
                }
                let field = &mut self.fields[usize::from(slot)];
                field.step = input.step;
                field.rank = input.value;
                field.maximum = input.maximum;
                self.learn_skill_rewards(
                    input.skill,
                    u32::from(input.value),
                    sources.race,
                    sources,
                    effects,
                )?;
                if input.value > old {
                    effects
                        .update_enchantments(self, input.skill, old, input.value)
                        .map_err(SkillSetError::Effect)?;
                    if input.skill == 762 {
                        effects
                            .update_mount_capability(self)
                            .map_err(SkillSetError::Effect)?;
                    }
                }
                self.skill_criteria(input.skill, effects)?;
                let status = self
                    .status
                    .get_mut(&input.skill)
                    .expect("skill effects cannot erase status");
                if matches!(
                    status.state,
                    SkillUpdateState::Unchanged | SkillUpdateState::Deleted
                ) {
                    if old == 0 {
                        status.state = if status.state == SkillUpdateState::Deleted {
                            SkillUpdateState::Changed
                        } else {
                            SkillUpdateState::New
                        };
                        self.assign_profession(sources.birth, input.skill);
                        self.refresh_skill_auras(input.skill, effects)?;
                    } else {
                        status.state = SkillUpdateState::Changed;
                    }
                }
            } else if old != 0 {
                if let Some(profession) = self.profession_slot(input.skill) {
                    for offset in 0..3 {
                        let slot = 19 + (profession as u8) * 3 + offset;
                        if effects
                            .store_profession_item(self, slot)
                            .map_err(SkillSetError::Effect)?
                            == ProfessionItemMove::InventoryFull
                        {
                            effects
                                .display_inventory_full(self)
                                .map_err(SkillSetError::Effect)?;
                            return Ok(SkillSetOutcome::InventoryFull);
                        }
                    }
                    self.professions[profession] = 0;
                }
                effects
                    .update_enchantments(self, input.skill, old, 0)
                    .map_err(SkillSetError::Effect)?;
                let field = &mut self.fields[usize::from(slot)];
                *field = SkillFields {
                    line: field.line,
                    starting_rank: 1,
                    ..Default::default()
                };
                let status = self
                    .status
                    .get_mut(&input.skill)
                    .expect("skill effects cannot erase status");
                status.state = if status.state == SkillUpdateState::New {
                    SkillUpdateState::Unchanged
                } else {
                    SkillUpdateState::Deleted
                };
                for ability in sources.birth.abilities_for_skill(input.skill) {
                    effects
                        .remove_spell(
                            self,
                            sources.spells.first_spell_in_chain(ability.spell as u32),
                        )
                        .map_err(SkillSetError::Effect)?;
                }
                for child in sources.birth.child_lines(input.skill) {
                    self.set_inner(
                        SkillUpdate {
                            skill: child.id,
                            step: 0,
                            value: 0,
                            maximum: 0,
                        },
                        sources,
                        effects,
                        active,
                    )?;
                }
            }
        } else {
            // Source captures the slot BEFORE recursively adding child skills.
            // Slot zero is treated as failure (!skillSlot), even when empty.
            let Some(slot) = self
                .fields
                .iter()
                .position(|field| field.line == 0)
                .filter(|&slot| slot != 0)
            else {
                return Ok(SkillSetOutcome::NoFreeSlot);
            };
            if line.parent_skill != 0 {
                self.enable_parent(line, classic_child, sources, effects, active)?;
            } else {
                for child in sources.birth.child_lines(input.skill) {
                    if !self.has_skill(child.id) {
                        self.set_inner(
                            SkillUpdate {
                                skill: child.id,
                                step: 0,
                                value: 0,
                                maximum: 0,
                            },
                            sources,
                            effects,
                            active,
                        )?;
                    }
                }
                self.assign_profession(sources.birth, input.skill);
            }
            self.fields[slot] = SkillFields {
                line: input.skill as u16,
                step: input.step,
                rank: input.value,
                starting_rank: 1,
                maximum: input.maximum,
                ..Default::default()
            };
            effects
                .update_enchantments(self, input.skill, 0, input.value)
                .map_err(SkillSetError::Effect)?;
            self.status.entry(input.skill).or_insert(Status {
                slot: slot as u16,
                state: SkillUpdateState::New,
            });
            if input.value != 0 {
                self.refresh_skill_auras(input.skill, effects)?;
                self.learn_skill_rewards(
                    input.skill,
                    u32::from(input.value),
                    sources.race,
                    sources,
                    effects,
                )?;
                self.skill_criteria(input.skill, effects)?;
            }
        }
        Ok(SkillSetOutcome::Finished)
    }

    fn enable_parent<E: SkillSetEffects>(
        &mut self,
        line: &SkillLineRecord,
        classic_child: bool,
        sources: &SkillSetSources<'_>,
        effects: &mut E,
        active: &mut BTreeSet<u32>,
    ) -> Result<(), SkillSetError<E::Error>> {
        if line.parent_skill == 0 || classic_child || line.parent_tier_index <= 0 {
            return Ok(());
        }
        let rc = sources
            .world
            .skill_race_class_info(line.parent_skill, sources.race, sources.class)
            .map_err(|error| SkillSetError::Source(SkillSetInputError::World(error)))?;
        if let Some(rc) = rc
            && let Some(maximum) = sources
                .world
                .skill_tier_value(rc.tier as i32 as u32, line.parent_tier_index as u32 - 1)
        {
            self.set_inner(
                SkillUpdate {
                    skill: line.parent_skill,
                    step: line.parent_tier_index as u16,
                    value: self.pure_value(line.parent_skill).max(1),
                    maximum: maximum as u16,
                },
                sources,
                effects,
                active,
            )?;
        }
        Ok(())
    }
    fn refresh_skill_auras<E: SkillSetEffects>(
        &mut self,
        skill: u32,
        effects: &mut E,
    ) -> Result<(), SkillSetError<E::Error>> {
        for aura in [30, 400, 98] {
            effects
                .refresh_bonus_auras(self, skill, aura)
                .map_err(SkillSetError::Effect)?;
        }
        Ok(())
    }
    fn skill_criteria<E: SkillSetEffects>(
        &mut self,
        skill: u32,
        effects: &mut E,
    ) -> Result<(), SkillSetError<E::Error>> {
        for criterion in [SkillCriteria::Raised, SkillCriteria::AchieveStep] {
            effects
                .update_criteria(self, skill, criterion)
                .map_err(SkillSetError::Effect)?;
        }
        Ok(())
    }
}
