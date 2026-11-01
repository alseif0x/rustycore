// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Pure primary-profession capacity and equipment-association planning.
//!
//! The target C++ has two separate concepts:
//!
//! - C++ `World.cpp:1135` reads `CONFIG_MAX_PRIMARY_TRADE_SKILL` with a default
//!   of 2; that path does not impose Rust's existing 0..=11 validation bound.
//! - `ActivePlayerData::ProfessionSkillLine[2]` in
//!   `Entities/Object/Updates/UpdateFields.h:755` associates at most two of
//!   those professions with the profession equipment slots.
//!
//! The legacy fork stores free profession capacity in `CharacterPoints`,
//! which is also the talent-point field (`Player.h:1848-1849`, `Player.cpp:2359`).
//! Rust derives used capacity from active root profession skills and never
//! reads or mutates talent points here.

use std::collections::{BTreeMap, BTreeSet};

use wow_data::SkillLineStore;

pub use wow_config::{
    DEFAULT_MAX_PRIMARY_TRADE_SKILLS_LIKE_CPP, MAX_PRIMARY_TRADE_SKILLS_CONFIG_LIKE_CPP,
};
pub const NO_PRIMARY_PROFESSION_EQUIPMENT_SLOT_LIKE_CPP: i8 = -1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlayerSkillProfessionSnapshotLikeCpp {
    pub skill_id: u32,
    pub value: u16,
    pub profession_slot: i8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PrimaryProfessionEquipmentSlotLikeCpp {
    First,
    Second,
}

impl PrimaryProfessionEquipmentSlotLikeCpp {
    const ALL: [Self; 2] = [Self::First, Self::Second];

    fn from_db_value_like_cpp(value: i8) -> Option<Self> {
        match value {
            0 => Some(Self::First),
            1 => Some(Self::Second),
            _ => None,
        }
    }

    pub fn db_value_like_cpp(self) -> i8 {
        match self {
            Self::First => 0,
            Self::Second => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimaryProfessionSlotNormalizationReasonLikeCpp {
    InactiveSkill,
    NonPrimarySkill,
    OutOfRange,
    Duplicate,
    FillEmptyAssociation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrimaryProfessionSlotNormalizationLikeCpp {
    pub skill_id: u32,
    pub original_slot: i8,
    pub normalized_slot: Option<PrimaryProfessionEquipmentSlotLikeCpp>,
    pub reason: PrimaryProfessionSlotNormalizationReasonLikeCpp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlannedPrimaryProfessionLikeCpp {
    pub skill_id: u32,
    /// `None` persists as C++ `professionSlot = -1`.
    pub equipment_slot: Option<PrimaryProfessionEquipmentSlotLikeCpp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrimaryProfessionCapacityAnalysisLikeCpp {
    configured_max: u8,
    existing_professions: Vec<PlannedPrimaryProfessionLikeCpp>,
    slot_normalizations: Vec<PrimaryProfessionSlotNormalizationLikeCpp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrimaryProfessionCapacityPlanLikeCpp {
    pub configured_max: u8,
    pub used_before: usize,
    pub free_before: usize,
    pub existing_professions: Vec<PlannedPrimaryProfessionLikeCpp>,
    pub new_professions: Vec<PlannedPrimaryProfessionLikeCpp>,
    pub slot_normalizations: Vec<PrimaryProfessionSlotNormalizationLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimaryProfessionCapacityPlanErrorLikeCpp {
    MissingSkillLineStore,
    MissingPlayerSkillSnapshot,
    InvalidConfiguredMaximum {
        configured: u8,
    },
    MissingSkillLinePayload {
        skill_id: u32,
    },
    CapacityExceeded {
        configured_max: u8,
        used: usize,
        requested_new: usize,
    },
}

#[derive(Debug, Clone, Copy)]
struct ExistingPrimaryProfessionLikeCpp {
    skill_id: u32,
    original_slot: i8,
    normalized_slot: Option<PrimaryProfessionEquipmentSlotLikeCpp>,
    normalization_reason: Option<PrimaryProfessionSlotNormalizationReasonLikeCpp>,
}

/// Classifies current skills and produces a deterministic, non-mutating
/// normalization analysis.
pub fn analyze_primary_professions_like_cpp(
    configured_max: u8,
    skill_lines: &SkillLineStore,
    current_skills: impl IntoIterator<Item = PlayerSkillProfessionSnapshotLikeCpp>,
) -> Result<PrimaryProfessionCapacityAnalysisLikeCpp, PrimaryProfessionCapacityPlanErrorLikeCpp> {
    if configured_max > MAX_PRIMARY_TRADE_SKILLS_CONFIG_LIKE_CPP {
        return Err(
            PrimaryProfessionCapacityPlanErrorLikeCpp::InvalidConfiguredMaximum {
                configured: configured_max,
            },
        );
    }

    let mut current_skills: Vec<_> = current_skills.into_iter().collect();
    current_skills.sort_by_key(|skill| skill.skill_id);

    let mut existing_primary = Vec::new();
    let mut occupied_equipment_slots =
        BTreeMap::<PrimaryProfessionEquipmentSlotLikeCpp, u32>::new();
    let mut slot_normalizations = Vec::new();

    for skill in current_skills {
        if skill.value == 0 {
            if skill.profession_slot != NO_PRIMARY_PROFESSION_EQUIPMENT_SLOT_LIKE_CPP {
                slot_normalizations.push(PrimaryProfessionSlotNormalizationLikeCpp {
                    skill_id: skill.skill_id,
                    original_slot: skill.profession_slot,
                    normalized_slot: None,
                    reason: PrimaryProfessionSlotNormalizationReasonLikeCpp::InactiveSkill,
                });
            }
            continue;
        }

        let Some(is_primary) = skill_lines.is_primary_profession_skill_like_cpp(skill.skill_id)
        else {
            return Err(
                PrimaryProfessionCapacityPlanErrorLikeCpp::MissingSkillLinePayload {
                    skill_id: skill.skill_id,
                },
            );
        };

        if !is_primary {
            if skill.profession_slot != NO_PRIMARY_PROFESSION_EQUIPMENT_SLOT_LIKE_CPP {
                slot_normalizations.push(PrimaryProfessionSlotNormalizationLikeCpp {
                    skill_id: skill.skill_id,
                    original_slot: skill.profession_slot,
                    normalized_slot: None,
                    reason: PrimaryProfessionSlotNormalizationReasonLikeCpp::NonPrimarySkill,
                });
            }
            continue;
        }

        let (normalized_slot, normalization_reason) =
            match PrimaryProfessionEquipmentSlotLikeCpp::from_db_value_like_cpp(
                skill.profession_slot,
            ) {
                Some(slot) if !occupied_equipment_slots.contains_key(&slot) => {
                    occupied_equipment_slots.insert(slot, skill.skill_id);
                    (Some(slot), None)
                }
                Some(_) => (
                    None,
                    Some(PrimaryProfessionSlotNormalizationReasonLikeCpp::Duplicate),
                ),
                None if skill.profession_slot == NO_PRIMARY_PROFESSION_EQUIPMENT_SLOT_LIKE_CPP => {
                    (None, None)
                }
                None => (
                    None,
                    Some(PrimaryProfessionSlotNormalizationReasonLikeCpp::OutOfRange),
                ),
            };

        existing_primary.push(ExistingPrimaryProfessionLikeCpp {
            skill_id: skill.skill_id,
            original_slot: skill.profession_slot,
            normalized_slot,
            normalization_reason,
        });
    }

    // Match C++ login fixup intent deterministically: existing active
    // professions without an association get the lowest free physical slot
    // before a newly learned profession can claim one.
    for profession in &mut existing_primary {
        if profession.normalized_slot.is_some() {
            continue;
        }
        let free_slot = PrimaryProfessionEquipmentSlotLikeCpp::ALL
            .into_iter()
            .find(|slot| !occupied_equipment_slots.contains_key(slot));
        if let Some(slot) = free_slot {
            profession.normalized_slot = Some(slot);
            occupied_equipment_slots.insert(slot, profession.skill_id);
            profession.normalization_reason =
                Some(profession.normalization_reason.unwrap_or(
                    PrimaryProfessionSlotNormalizationReasonLikeCpp::FillEmptyAssociation,
                ));
        }

        let normalized_db_value = profession
            .normalized_slot
            .map(PrimaryProfessionEquipmentSlotLikeCpp::db_value_like_cpp)
            .unwrap_or(NO_PRIMARY_PROFESSION_EQUIPMENT_SLOT_LIKE_CPP);
        if normalized_db_value != profession.original_slot {
            slot_normalizations.push(PrimaryProfessionSlotNormalizationLikeCpp {
                skill_id: profession.skill_id,
                original_slot: profession.original_slot,
                normalized_slot: profession.normalized_slot,
                reason: profession.normalization_reason.unwrap_or(
                    PrimaryProfessionSlotNormalizationReasonLikeCpp::FillEmptyAssociation,
                ),
            });
        }
    }
    slot_normalizations.sort_by_key(|normalization| normalization.skill_id);

    Ok(PrimaryProfessionCapacityAnalysisLikeCpp {
        configured_max,
        existing_professions: existing_primary
            .into_iter()
            .map(|profession| PlannedPrimaryProfessionLikeCpp {
                skill_id: profession.skill_id,
                equipment_slot: profession.normalized_slot,
            })
            .collect(),
        slot_normalizations,
    })
}

/// Plans an all-or-none capacity decision for already-resolved skill-line
/// IDs without mutating or reserving shared state.
///
/// Resolving trainer wrappers and known spells belongs to #157; applying and
/// persisting the returned assignments belongs to #158. #159 must recompute
/// this plan under its mutation boundary before committing.
pub fn plan_primary_professions_like_cpp(
    analysis: &PrimaryProfessionCapacityAnalysisLikeCpp,
    skill_lines: &SkillLineStore,
    requested_skill_ids: impl IntoIterator<Item = u32>,
) -> Result<PrimaryProfessionCapacityPlanLikeCpp, PrimaryProfessionCapacityPlanErrorLikeCpp> {
    let existing_ids: BTreeSet<_> = analysis
        .existing_professions
        .iter()
        .map(|profession| profession.skill_id)
        .collect();
    let mut seen_requested = BTreeSet::new();
    let mut requested_primary = Vec::new();
    for skill_id in requested_skill_ids {
        let Some(is_primary) = skill_lines.is_primary_profession_skill_like_cpp(skill_id) else {
            return Err(
                PrimaryProfessionCapacityPlanErrorLikeCpp::MissingSkillLinePayload { skill_id },
            );
        };
        if is_primary && !existing_ids.contains(&skill_id) && seen_requested.insert(skill_id) {
            requested_primary.push(skill_id);
        }
    }

    let used_before = existing_ids.len();
    let requested_new = requested_primary.len();
    let free_before = usize::from(analysis.configured_max).saturating_sub(used_before);
    if requested_new > free_before {
        return Err(
            PrimaryProfessionCapacityPlanErrorLikeCpp::CapacityExceeded {
                configured_max: analysis.configured_max,
                used: used_before,
                requested_new,
            },
        );
    }

    let mut occupied_equipment_slots: BTreeMap<_, _> = analysis
        .existing_professions
        .iter()
        .filter_map(|profession| {
            profession
                .equipment_slot
                .map(|slot| (slot, profession.skill_id))
        })
        .collect();
    let mut new_professions = Vec::with_capacity(requested_new);
    for skill_id in requested_primary {
        let equipment_slot = PrimaryProfessionEquipmentSlotLikeCpp::ALL
            .into_iter()
            .find(|slot| !occupied_equipment_slots.contains_key(slot));
        if let Some(slot) = equipment_slot {
            occupied_equipment_slots.insert(slot, skill_id);
        }
        new_professions.push(PlannedPrimaryProfessionLikeCpp {
            skill_id,
            equipment_slot,
        });
    }

    Ok(PrimaryProfessionCapacityPlanLikeCpp {
        configured_max: analysis.configured_max,
        used_before,
        free_before,
        existing_professions: analysis.existing_professions.clone(),
        new_professions,
        slot_normalizations: analysis.slot_normalizations.clone(),
    })
}

#[cfg(test)]
#[path = "../unit_tests/profession.rs"]
mod tests;
