use super::*;

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
