// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RepresentedPlayerSkillStateLikeCpp {
    Unchanged,
    Changed,
    New,
    Deleted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RepresentedPlayerSkillLikeCpp {
    pub skill_id: u16,
    pub step: u16,
    pub value: u16,
    pub max: u16,
    pub profession_slot: i8,
    pub state: RepresentedPlayerSkillStateLikeCpp,
}

#[cfg(any(test, feature = "test-fixtures"))]
pub fn is_non_durable_skill_tombstone_like_cpp(
    skill: &RepresentedPlayerSkillLikeCpp,
) -> bool {
    skill.step == 0
        && skill.value == 0
        && skill.max == 0
        && skill.profession_slot == -1
        && matches!(
            skill.state,
            RepresentedPlayerSkillStateLikeCpp::Unchanged
                | RepresentedPlayerSkillStateLikeCpp::Deleted
        )
}

pub fn canonical_player_skill_record_like_cpp(
    skill: RepresentedPlayerSkillLikeCpp,
) -> wow_entities::PlayerSkillRecord {
    wow_entities::PlayerSkillRecord {
        skill_line_id: u32::from(skill.skill_id),
        current_value: skill.value,
        max_value: skill.max,
        step: skill.step,
        profession_slot: skill.profession_slot,
        state: match skill.state {
            RepresentedPlayerSkillStateLikeCpp::Unchanged => {
                wow_entities::PlayerSkillLoadState::Unchanged
            }
            RepresentedPlayerSkillStateLikeCpp::Changed => {
                wow_entities::PlayerSkillLoadState::Changed
            }
            RepresentedPlayerSkillStateLikeCpp::New => wow_entities::PlayerSkillLoadState::New,
            RepresentedPlayerSkillStateLikeCpp::Deleted => {
                wow_entities::PlayerSkillLoadState::Deleted
            }
        },
    }
}

pub fn represented_player_skill_record_like_cpp(
    skill: &wow_entities::PlayerSkillRecord,
) -> Option<RepresentedPlayerSkillLikeCpp> {
    Some(RepresentedPlayerSkillLikeCpp {
        skill_id: u16::try_from(skill.skill_line_id).ok()?,
        step: skill.step,
        value: skill.current_value,
        max: skill.max_value,
        profession_slot: skill.profession_slot,
        state: match skill.state {
            wow_entities::PlayerSkillLoadState::Unchanged => {
                RepresentedPlayerSkillStateLikeCpp::Unchanged
            }
            wow_entities::PlayerSkillLoadState::Changed => {
                RepresentedPlayerSkillStateLikeCpp::Changed
            }
            wow_entities::PlayerSkillLoadState::New => RepresentedPlayerSkillStateLikeCpp::New,
            wow_entities::PlayerSkillLoadState::Deleted => {
                RepresentedPlayerSkillStateLikeCpp::Deleted
            }
        },
    })
}

#[allow(dead_code)]
pub fn represented_skill_records_from_values_like_cpp(
    skill_values: &HashMap<u16, u16>,
) -> HashMap<u16, RepresentedPlayerSkillLikeCpp> {
    skill_values
        .iter()
        .map(|(&skill_id, &value)| {
            (
                skill_id,
                RepresentedPlayerSkillLikeCpp {
                    skill_id,
                    step: 0,
                    value,
                    max: value,
                    profession_slot: -1,
                    state: RepresentedPlayerSkillStateLikeCpp::Unchanged,
                },
            )
        })
        .collect()
}

pub fn represented_skill_values_from_records_like_cpp(
    skill_records: &HashMap<u16, RepresentedPlayerSkillLikeCpp>,
) -> HashMap<u16, u16> {
    skill_records
        .iter()
        .map(|(&skill_id, record)| (skill_id, record.value))
        .collect()
}
