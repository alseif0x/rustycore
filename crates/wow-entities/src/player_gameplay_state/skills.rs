//! Row lifecycle and slot consumption for the represented skill writer.

use super::{PlayerGameplayState, PlayerSkillRecord};
use crate::PlayerSkillLoadState;

pub struct SkillWritePlan {
    pub record: PlayerSkillRecord,
    slot_delta: u16,
}

impl SkillWritePlan {
    pub fn occupied_slots_after(&self, occupied_slots: u16) -> u16 {
        occupied_slots.saturating_add(self.slot_delta)
    }
}

impl PlayerGameplayState {
    pub fn prepare_skill_write(
        skill_id: u16,
        step: u16,
        value: u16,
        max: u16,
        previous: Option<&PlayerSkillRecord>,
    ) -> SkillWritePlan {
        let step = if value == 0 { 0 } else { step };
        let profession_slot = previous.map(|skill| skill.profession_slot).unwrap_or(-1);
        let state = match previous {
            None => PlayerSkillLoadState::New,
            Some(previous) if value == 0 && previous.current_value != 0 => {
                if previous.state == PlayerSkillLoadState::New {
                    PlayerSkillLoadState::Unchanged
                } else {
                    PlayerSkillLoadState::Deleted
                }
            }
            Some(previous) if value == 0 => previous.state,
            Some(previous)
                if matches!(
                    previous.state,
                    PlayerSkillLoadState::Unchanged | PlayerSkillLoadState::Deleted
                ) =>
            {
                if previous.current_value == 0 {
                    if previous.state == PlayerSkillLoadState::Deleted {
                        PlayerSkillLoadState::Changed
                    } else {
                        PlayerSkillLoadState::New
                    }
                } else {
                    PlayerSkillLoadState::Changed
                }
            }
            Some(previous) => previous.state,
        };
        SkillWritePlan {
            record: PlayerSkillRecord {
                skill_line_id: u32::from(skill_id),
                step,
                current_value: value,
                max_value: max,
                profession_slot,
                state,
            },
            slot_delta: u16::from(previous.is_none()),
        }
    }

    pub fn skill_maximum_for_level(level: u8) -> u16 {
        u16::from(level).saturating_mul(5)
    }
}

#[cfg(test)]
mod tests;
