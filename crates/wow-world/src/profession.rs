// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Pure primary-profession capacity and equipment-association planning.
//!
//! The target C++ has two separate concepts:
//!
//! - `CONFIG_MAX_PRIMARY_TRADE_SKILL` permits 0..=11 learned primary
//!   professions.
//! - `ActivePlayerData::ProfessionSkillLine[2]` associates at most two of
//!   those professions with the profession equipment slots.
//!
//! The legacy fork incorrectly stores free profession capacity in
//! `CharacterPoints`, which is also the talent-point field. Rust derives used
//! capacity from active root profession skills instead and never reads or
//! mutates talent points here.

use std::collections::{BTreeMap, BTreeSet};

use wow_data::SkillLineStore;

use crate::session::WorldSession;

pub(crate) use wow_config::{
    DEFAULT_MAX_PRIMARY_TRADE_SKILLS_LIKE_CPP, MAX_PRIMARY_TRADE_SKILLS_CONFIG_LIKE_CPP,
};
pub(crate) const NO_PRIMARY_PROFESSION_EQUIPMENT_SLOT_LIKE_CPP: i8 = -1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlayerSkillProfessionSnapshotLikeCpp {
    pub skill_id: u32,
    pub value: u16,
    pub profession_slot: i8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum PrimaryProfessionEquipmentSlotLikeCpp {
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

    pub(crate) fn db_value_like_cpp(self) -> i8 {
        match self {
            Self::First => 0,
            Self::Second => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PrimaryProfessionSlotNormalizationReasonLikeCpp {
    InactiveSkill,
    NonPrimarySkill,
    OutOfRange,
    Duplicate,
    FillEmptyAssociation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PrimaryProfessionSlotNormalizationLikeCpp {
    pub skill_id: u32,
    pub original_slot: i8,
    pub normalized_slot: Option<PrimaryProfessionEquipmentSlotLikeCpp>,
    pub reason: PrimaryProfessionSlotNormalizationReasonLikeCpp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlannedPrimaryProfessionLikeCpp {
    pub skill_id: u32,
    /// `None` persists as C++ `professionSlot = -1`.
    pub equipment_slot: Option<PrimaryProfessionEquipmentSlotLikeCpp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PrimaryProfessionCapacityAnalysisLikeCpp {
    configured_max: u8,
    existing_professions: Vec<PlannedPrimaryProfessionLikeCpp>,
    slot_normalizations: Vec<PrimaryProfessionSlotNormalizationLikeCpp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PrimaryProfessionCapacityPlanLikeCpp {
    pub configured_max: u8,
    pub used_before: usize,
    pub free_before: usize,
    pub existing_professions: Vec<PlannedPrimaryProfessionLikeCpp>,
    pub new_professions: Vec<PlannedPrimaryProfessionLikeCpp>,
    pub slot_normalizations: Vec<PrimaryProfessionSlotNormalizationLikeCpp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PrimaryProfessionCapacityPlanErrorLikeCpp {
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
pub(crate) fn analyze_primary_professions_like_cpp(
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
pub(crate) fn plan_primary_professions_like_cpp(
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

impl WorldSession {
    pub(crate) fn plan_primary_profession_capacity_like_cpp(
        &self,
        requested_skill_ids: impl IntoIterator<Item = u32>,
    ) -> Result<PrimaryProfessionCapacityPlanLikeCpp, PrimaryProfessionCapacityPlanErrorLikeCpp>
    {
        let Some(skill_lines) = self.skill_line_store() else {
            return Err(PrimaryProfessionCapacityPlanErrorLikeCpp::MissingSkillLineStore);
        };
        if !self.player_skill_records_loaded_like_cpp() {
            return Err(PrimaryProfessionCapacityPlanErrorLikeCpp::MissingPlayerSkillSnapshot);
        }
        let current_skills = self.player_skill_records_like_cpp().values().map(|skill| {
            PlayerSkillProfessionSnapshotLikeCpp {
                skill_id: u32::from(skill.skill_id),
                value: skill.value,
                profession_slot: skill.profession_slot,
            }
        });

        let analysis = analyze_primary_professions_like_cpp(
            self.max_primary_trade_skills_like_cpp(),
            skill_lines,
            current_skills,
        )?;
        plan_primary_professions_like_cpp(&analysis, skill_lines, requested_skill_ids)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{collections::HashMap, sync::Arc};

    use crate::session::RepresentedPlayerSkillLikeCpp;
    use wow_data::SkillLineEntry;

    fn skill_line(id: u32, category_id: i8, parent_skill_line_id: u32) -> SkillLineEntry {
        SkillLineEntry {
            id,
            display_name: String::new(),
            alternate_verb: String::new(),
            description: String::new(),
            horde_display_name: String::new(),
            override_source_info_display_name: String::new(),
            category_id,
            spell_icon_file_id: 0,
            can_link: 0,
            parent_skill_line_id,
            parent_tier_index: 0,
            flags: 0,
            spell_book_spell_id: 0,
        }
    }

    fn profession(skill_id: u32, profession_slot: i8) -> PlayerSkillProfessionSnapshotLikeCpp {
        PlayerSkillProfessionSnapshotLikeCpp {
            skill_id,
            value: 1,
            profession_slot,
        }
    }

    fn skill_lines() -> SkillLineStore {
        SkillLineStore::from_entries([
            skill_line(100, 11, 0),
            skill_line(200, 11, 0),
            skill_line(300, 11, 0),
            skill_line(400, 11, 0),
            skill_line(101, 11, 100),
            skill_line(500, 9, 0),
        ])
    }

    fn plan(
        configured_max: u8,
        current_skills: impl IntoIterator<Item = PlayerSkillProfessionSnapshotLikeCpp>,
        requested_skill_ids: impl IntoIterator<Item = u32>,
    ) -> Result<PrimaryProfessionCapacityPlanLikeCpp, PrimaryProfessionCapacityPlanErrorLikeCpp>
    {
        let skill_lines = skill_lines();
        let analysis =
            analyze_primary_professions_like_cpp(configured_max, &skill_lines, current_skills)?;
        plan_primary_professions_like_cpp(&analysis, &skill_lines, requested_skill_ids)
    }

    #[test]
    fn capacity_counts_active_primary_skills_even_without_equipment_association() {
        let error = plan(2, [profession(100, -1), profession(200, -1)], [300]).unwrap_err();

        assert_eq!(
            error,
            PrimaryProfessionCapacityPlanErrorLikeCpp::CapacityExceeded {
                configured_max: 2,
                used: 2,
                requested_new: 1,
            }
        );
    }

    #[test]
    fn configured_third_profession_is_valid_without_a_third_equipment_slot() {
        let plan = plan(3, [profession(100, 0), profession(200, 1)], [300]).unwrap();

        assert_eq!(
            plan.new_professions,
            vec![PlannedPrimaryProfessionLikeCpp {
                skill_id: 300,
                equipment_slot: None,
            }]
        );
        assert_eq!(plan.used_before, 2);
        assert_eq!(plan.free_before, 1);
    }

    #[test]
    fn configured_eleven_professions_remain_independent_from_two_equipment_slots() {
        let plan = plan(11, [], [100, 200, 300, 400]).unwrap();

        assert_eq!(plan.configured_max, 11);
        assert_eq!(
            plan.new_professions
                .iter()
                .map(|profession| profession.equipment_slot)
                .collect::<Vec<_>>(),
            vec![
                Some(PrimaryProfessionEquipmentSlotLikeCpp::First),
                Some(PrimaryProfessionEquipmentSlotLikeCpp::Second),
                None,
                None,
            ]
        );
    }

    #[test]
    fn multi_skill_capacity_plan_is_all_or_none() {
        assert_eq!(
            plan(2, [profession(100, 0)], [200, 300]),
            Err(
                PrimaryProfessionCapacityPlanErrorLikeCpp::CapacityExceeded {
                    configured_max: 2,
                    used: 1,
                    requested_new: 2,
                }
            )
        );
    }

    #[test]
    fn plan_preserves_requested_order_while_deduplicating_active_and_nonprimary_skills() {
        let plan = plan(3, [profession(100, 0)], [100, 300, 200, 300, 101, 500, 200]).unwrap();

        assert_eq!(
            plan.new_professions,
            vec![
                PlannedPrimaryProfessionLikeCpp {
                    skill_id: 300,
                    equipment_slot: Some(PrimaryProfessionEquipmentSlotLikeCpp::Second),
                },
                PlannedPrimaryProfessionLikeCpp {
                    skill_id: 200,
                    equipment_slot: None,
                },
            ],
            "already-resolved IDs retain their first-occurrence C++ order"
        );
    }

    #[test]
    fn existing_professions_fill_holes_before_new_professions() {
        let plan = plan(3, [profession(100, 1), profession(200, -1)], [300]).unwrap();

        assert_eq!(
            plan.existing_professions,
            vec![
                PlannedPrimaryProfessionLikeCpp {
                    skill_id: 100,
                    equipment_slot: Some(PrimaryProfessionEquipmentSlotLikeCpp::Second),
                },
                PlannedPrimaryProfessionLikeCpp {
                    skill_id: 200,
                    equipment_slot: Some(PrimaryProfessionEquipmentSlotLikeCpp::First),
                },
            ]
        );
        assert_eq!(plan.new_professions[0].equipment_slot, None);
    }

    #[test]
    fn duplicate_and_out_of_range_slots_are_normalized_deterministically() {
        let plan = plan(
            3,
            [profession(200, 0), profession(100, 0), profession(300, 9)],
            [],
        )
        .unwrap();

        assert_eq!(
            plan.existing_professions,
            vec![
                PlannedPrimaryProfessionLikeCpp {
                    skill_id: 100,
                    equipment_slot: Some(PrimaryProfessionEquipmentSlotLikeCpp::First),
                },
                PlannedPrimaryProfessionLikeCpp {
                    skill_id: 200,
                    equipment_slot: Some(PrimaryProfessionEquipmentSlotLikeCpp::Second),
                },
                PlannedPrimaryProfessionLikeCpp {
                    skill_id: 300,
                    equipment_slot: None,
                },
            ]
        );
        assert!(plan.slot_normalizations.iter().any(|change| {
            change.skill_id == 200
                && change.original_slot == 0
                && change.normalized_slot == Some(PrimaryProfessionEquipmentSlotLikeCpp::Second)
                && change.reason == PrimaryProfessionSlotNormalizationReasonLikeCpp::Duplicate
        }));
        assert!(plan.slot_normalizations.iter().any(|change| {
            change.skill_id == 300
                && change.original_slot == 9
                && change.normalized_slot.is_none()
                && change.reason == PrimaryProfessionSlotNormalizationReasonLikeCpp::OutOfRange
        }));
    }

    #[test]
    fn reduced_or_zero_configuration_preserves_existing_but_blocks_new() {
        let existing = [profession(100, 0), profession(200, 1)];
        let zero_plan = plan(0, existing, []).unwrap();
        assert_eq!(zero_plan.used_before, 2);
        assert_eq!(zero_plan.free_before, 0);

        assert!(matches!(
            plan(1, existing, [300]),
            Err(
                PrimaryProfessionCapacityPlanErrorLikeCpp::CapacityExceeded {
                    configured_max: 1,
                    used: 2,
                    requested_new: 1,
                }
            )
        ));
    }

    #[test]
    fn active_unhydrated_effective_skill_fails_closed_but_inactive_row_does_not() {
        let skill_lines = SkillLineStore::from_hydrated_entries_and_effective_ids_like_cpp(
            [skill_line(100, 11, 0)],
            [100, 999],
        );

        assert_eq!(
            analyze_primary_professions_like_cpp(2, &skill_lines, [profession(999, -1)],),
            Err(
                PrimaryProfessionCapacityPlanErrorLikeCpp::MissingSkillLinePayload {
                    skill_id: 999,
                }
            )
        );

        let mut inactive = profession(999, 0);
        inactive.value = 0;
        let analysis = analyze_primary_professions_like_cpp(2, &skill_lines, [inactive]).unwrap();
        let plan = plan_primary_professions_like_cpp(&analysis, &skill_lines, []).unwrap();
        assert_eq!(plan.used_before, 0);
        assert_eq!(
            plan.slot_normalizations,
            vec![PrimaryProfessionSlotNormalizationLikeCpp {
                skill_id: 999,
                original_slot: 0,
                normalized_slot: None,
                reason: PrimaryProfessionSlotNormalizationReasonLikeCpp::InactiveSkill,
            }]
        );
    }

    #[test]
    fn inactive_and_nonprimary_slots_are_cleared_without_consuming_capacity() {
        let mut inactive = profession(100, 0);
        inactive.value = 0;
        let plan = plan(
            1,
            [inactive, profession(500, 1), profession(999, -1)],
            [200],
        )
        .unwrap();

        assert_eq!(plan.used_before, 0);
        assert_eq!(
            plan.new_professions,
            vec![PlannedPrimaryProfessionLikeCpp {
                skill_id: 200,
                equipment_slot: Some(PrimaryProfessionEquipmentSlotLikeCpp::First),
            }]
        );
        assert!(plan.slot_normalizations.iter().any(|change| {
            change.skill_id == 100
                && change.reason == PrimaryProfessionSlotNormalizationReasonLikeCpp::InactiveSkill
                && change.normalized_slot.is_none()
        }));
        assert!(plan.slot_normalizations.iter().any(|change| {
            change.skill_id == 500
                && change.reason == PrimaryProfessionSlotNormalizationReasonLikeCpp::NonPrimarySkill
                && change.normalized_slot.is_none()
        }));
    }

    #[test]
    fn invalid_configuration_fails_closed_before_planning() {
        assert_eq!(
            plan(12, [], []),
            Err(
                PrimaryProfessionCapacityPlanErrorLikeCpp::InvalidConfiguredMaximum {
                    configured: 12,
                }
            )
        );
    }

    #[test]
    fn world_session_capacity_is_loaded_fail_closed_and_independent_from_talent_points() {
        let (_packet_tx, packet_rx) = flume::bounded(1);
        let (send_tx, _send_rx) = flume::bounded(1);
        let mut session = WorldSession::new(
            1,
            "profession-test".to_string(),
            0,
            2,
            2,
            54_261,
            vec![0; 40],
            "enUS".to_string(),
            packet_rx,
            send_tx,
        );

        assert_eq!(
            session.max_primary_trade_skills_like_cpp(),
            DEFAULT_MAX_PRIMARY_TRADE_SKILLS_LIKE_CPP
        );
        for (configured, expected) in [(0, 0), (1, 1), (2, 2), (11, 11), (12, 2)] {
            session.set_max_primary_trade_skills_like_cpp(configured);
            assert_eq!(
                session.max_primary_trade_skills_like_cpp(),
                expected,
                "runtime value {configured}"
            );
        }

        assert_eq!(
            session.plan_primary_profession_capacity_like_cpp([300]),
            Err(PrimaryProfessionCapacityPlanErrorLikeCpp::MissingSkillLineStore),
            "missing immutable metadata must fail closed"
        );

        session.set_skill_line_store(Arc::new(skill_lines()));
        assert_eq!(
            session.plan_primary_profession_capacity_like_cpp([300]),
            Err(PrimaryProfessionCapacityPlanErrorLikeCpp::MissingPlayerSkillSnapshot),
            "an empty pre-login mirror must not be treated as free capacity"
        );

        session.set_player_skill_records_like_cpp(HashMap::from([
            (
                100,
                RepresentedPlayerSkillLikeCpp {
                    skill_id: 100,
                    value: 1,
                    max: 75,
                    profession_slot: 0,
                },
            ),
            (
                200,
                RepresentedPlayerSkillLikeCpp {
                    skill_id: 200,
                    value: 1,
                    max: 75,
                    profession_slot: 1,
                },
            ),
        ]));
        session.set_max_primary_trade_skills_like_cpp(3);
        session.set_player_character_points_like_cpp(99);
        let with_talent_points = session
            .plan_primary_profession_capacity_like_cpp([300])
            .unwrap();
        assert_eq!(session.player_character_points_like_cpp(), 99);

        session.set_player_character_points_like_cpp(0);
        let without_talent_points = session
            .plan_primary_profession_capacity_like_cpp([300])
            .unwrap();
        assert_eq!(with_talent_points, without_talent_points);
        assert_eq!(session.player_character_points_like_cpp(), 0);

        session.set_max_primary_trade_skills_like_cpp(12);
        assert_eq!(session.max_primary_trade_skills_like_cpp(), 2);
    }
}
