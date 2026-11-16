//! 02245dcd Player.cpp::Player::_LoadSkills row and post-row phases.
use super::super::skill_sources::{
    SkillLoadSource, SkillRaceClassMetadata, SkillSetError, SkillUpdate,
};
use super::set::SkillSetEffects;
use super::{PLAYER_MAX_SKILLS, PlayerSkills, SkillFields, SkillUpdateState, Status};
use std::collections::{HashMap, HashSet};

const SKILL_CATEGORY_ARMOR_LIKE_CPP: i8 = 8;
const SKILL_CATEGORY_SECONDARY_LIKE_CPP: i8 = 9;
const SKILL_CATEGORY_LANGUAGE_LIKE_CPP: i8 = 10;
const SKILL_CATEGORY_PROFESSION_LIKE_CPP: i8 = 11;
const SKILL_RUNEFORGING_LIKE_CPP: u32 = 960;
const SKILL_FIST_WEAPONS_LIKE_CPP: u32 = 473;
const SKILL_UNARMED_LIKE_CPP: u32 = 162;

/// One decoded character_skills row, in original query order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SkillLoadRow {
    pub skill: u16,
    pub value: u16,
    pub maximum: u16,
    pub profession_slot: i8,
}

/// The native row loop stopped at PLAYER_MAX_SKILLS and continued its later phases.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkillLoadOutcome {
    Complete,
    CapacityStopped,
}

/// A construction error retains already-applied field/effect prefixes. The
/// caller must discard this unadmitted skill owner rather than retry or save it.
#[derive(Debug)]
pub enum SkillLoadError<E, S, O> {
    Source(S),
    SkillSet(SkillSetError<E, S>),
    Order(O),
    InvalidOrderKeySet,
    InvalidProfessionSlot(i8),
    MissingLoadedSkillLine(u32),
}

impl PlayerSkills {
    /// Apply one complete isolated _LoadSkills operation to this skill owner.
    /// The provider admits only a traversal order; Rust never iterates its local
    /// hash map. Effects run synchronously against this same mutable owner.
    pub fn load_skills<S, E, O>(
        &mut self,
        rows: &[SkillLoadRow],
        sources: &S,
        effects: &mut E,
        order: impl FnOnce(&[u32], usize) -> Result<Vec<u32>, O>,
    ) -> Result<SkillLoadOutcome, SkillLoadError<E::Error, S::Error, O>>
    where
        S: SkillLoadSource,
        E: SkillSetEffects,
    {
        let race = sources.race();
        let mut loaded_skill_values = HashMap::<u32, u32>::new();
        let mut first_assignments = Vec::<u32>::new();
        let mut pending_professions = Vec::<u32>::new();
        let mut outcome = SkillLoadOutcome::Complete;

        for row in rows {
            if self.status.len() >= PLAYER_MAX_SKILLS {
                outcome = SkillLoadOutcome::CapacityStopped;
                break;
            }

            let skill = u32::from(row.skill);
            let Some(rc) = sources
                .skill_race_class_info(skill)
                .map_err(SkillLoadError::Source)?
            else {
                self.try_insert_status(
                    skill,
                    Status {
                        slot: self.status.len() as u16,
                        state: SkillUpdateState::Deleted,
                    },
                );
                continue;
            };

            let line = sources.skill_line(u32::from(rc.skill));
            let mut step = 0u16;
            let mut value = row.value;
            let mut maximum = row.maximum;
            if let Some(line) = line {
                if let Some(tier) = sources.skill_tier_values(signed_tier_id(rc)) {
                    if let Some(index) = tier
                        .iter()
                        .position(|&tier_value| tier_value == u32::from(maximum))
                    {
                        step = (index + 1) as u16;
                    }
                } else if u32::from(rc.skill) == SKILL_RUNEFORGING_LIKE_CPP
                    || line.category == SKILL_CATEGORY_ARMOR_LIKE_CPP
                {
                    value = 1;
                    maximum = 1;
                } else if line.category == SKILL_CATEGORY_LANGUAGE_LIKE_CPP {
                    value = 300;
                    maximum = 300;
                } else {
                    maximum = u16::from(effects.player_level()) * 5;
                }
            }

            let slot = match self.status.get(&skill) {
                Some(status) => usize::from(status.slot),
                None => {
                    let slot = self.status.len();
                    self.try_insert_status(
                        skill,
                        Status {
                            slot: slot as u16,
                            state: SkillUpdateState::Unchanged,
                        },
                    );
                    slot
                }
            };

            if let Some(line) = line {
                if step == 0 && line.category == SKILL_CATEGORY_SECONDARY_LIKE_CPP {
                    step = maximum / 75;
                }
                if line.category == SKILL_CATEGORY_PROFESSION_LIKE_CPP {
                    if step == 0 {
                        step = maximum / 75;
                    }
                    if line.parent_skill == 0 {
                        if row.profession_slot != -1 {
                            let profession_slot = usize::try_from(row.profession_slot)
                                .ok()
                                .filter(|&index| index < self.professions.len())
                                .ok_or(SkillLoadError::InvalidProfessionSlot(
                                    row.profession_slot,
                                ))?;
                            self.professions[profession_slot] = i32::from(row.skill);
                        } else {
                            // Repeated rows intentionally remain repeated fixups.
                            // Accumulate them after status creation below.
                            pending_professions.push(skill);
                        }
                    }
                }
            }

            self.fields[slot].line = row.skill;
            if value == 0 {
                self.status
                    .get_mut(&skill)
                    .expect("skill row retains its status node")
                    .state = SkillUpdateState::Deleted;
                continue;
            }

            self.fields[slot] = SkillFields {
                line: row.skill,
                step,
                rank: value,
                starting_rank: 1,
                maximum,
                temporary_bonus: 0,
                permanent_bonus: 0,
            };
            match loaded_skill_values.entry(skill) {
                std::collections::hash_map::Entry::Occupied(mut entry) => {
                    *entry.get_mut() = u32::from(value);
                }
                std::collections::hash_map::Entry::Vacant(entry) => {
                    first_assignments.push(skill);
                    entry.insert(u32::from(value));
                }
            }
        }

        let ordered_skills =
            order(&first_assignments, loaded_skill_values.len()).map_err(SkillLoadError::Order)?;
        validate_order(&loaded_skill_values, &ordered_skills)
            .map_err(|()| SkillLoadError::InvalidOrderKeySet)?;

        for skill in ordered_skills {
            let value = loaded_skill_values[&skill];
            self.learn_skill_rewards(skill, value, race, sources, effects)
                .map_err(SkillLoadError::SkillSet)?;

            let line = sources
                .skill_line(skill)
                .ok_or(SkillLoadError::MissingLoadedSkillLine(skill))?;
            if line.parent_skill != 0
                && line.parent_tier_index > 0
                && i32::from(self.step(line.parent_skill)) < line.parent_tier_index
            {
                if let Some(rc) = sources
                    .skill_race_class_info(line.parent_skill)
                    .map_err(SkillLoadError::Source)?
                    && let Some(maximum) = sources
                        .skill_tier_value(signed_tier_id(rc), (line.parent_tier_index - 1) as u32)
                {
                    let _ = self
                        .set_skill(
                            SkillUpdate {
                                skill: line.parent_skill,
                                step: line.parent_tier_index as u16,
                                value: self.pure_value(line.parent_skill).max(1),
                                maximum: maximum as u16,
                            },
                            sources,
                            effects,
                        )
                        .map_err(SkillLoadError::SkillSet)?;
                }
            }

            for child in sources.child_lines(skill) {
                if self.status.len() >= PLAYER_MAX_SKILLS {
                    break;
                }
                if self.status.contains_key(&child.id) {
                    continue;
                }
                let slot = self.status.len();
                self.fields[slot].line = child.id as u16;
                self.fields[slot].starting_rank = 1;
                self.try_insert_status(
                    child.id,
                    Status {
                        slot: slot as u16,
                        state: SkillUpdateState::Unchanged,
                    },
                );
            }
        }

        for skill in pending_professions {
            self.assign_loaded_profession(sources, skill);
        }

        if self.has_skill(SKILL_FIST_WEAPONS_LIKE_CPP) {
            let _ = self
                .set_skill(
                    SkillUpdate {
                        skill: SKILL_FIST_WEAPONS_LIKE_CPP,
                        step: 0,
                        value: self.value(SKILL_UNARMED_LIKE_CPP),
                        maximum: u16::from(effects.player_level()) * 5,
                    },
                    sources,
                    effects,
                )
                .map_err(SkillLoadError::SkillSet)?;
        }

        Ok(outcome)
    }
}

fn signed_tier_id(rc: SkillRaceClassMetadata) -> u32 {
    i32::from(rc.tier) as u32
}

fn validate_order(
    loaded_skill_values: &HashMap<u32, u32>,
    ordered_skills: &[u32],
) -> Result<(), ()> {
    if ordered_skills.len() != loaded_skill_values.len() {
        return Err(());
    }
    let mut seen = HashSet::with_capacity(ordered_skills.len());
    for &skill in ordered_skills {
        if !loaded_skill_values.contains_key(&skill) || !seen.insert(skill) {
            return Err(());
        }
    }
    Ok(())
}
