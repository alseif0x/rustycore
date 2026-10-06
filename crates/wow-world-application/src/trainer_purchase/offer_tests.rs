#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, HashMap};

    use super::super::*;
    use wow_spell_acquisition::{
        PlayerAcquisitionLifecycleLikeCpp, PlayerSpellAcquisitionSnapshotLikeCpp,
    };

    fn acquisition_plan(
        root: SpellAcquisitionRootLikeCpp,
        professions: Vec<u32>,
    ) -> SpellAcquisitionPlanLikeCpp {
        let source_snapshot = PlayerSpellAcquisitionSnapshotLikeCpp {
            character_guid: None,
            spells: Vec::new(),
            skills: Vec::new(),
            occupied_skill_slots: 0,
            overrides: Vec::new(),
            primary_profession_skill_ids: Vec::new(),
            non_durable_skill_tombstone_ids: Vec::new(),
            race: 1,
            class: 1,
            level: 80,
            lifecycle: PlayerAcquisitionLifecycleLikeCpp::InWorld,
            future_player_condition_resolutions: Vec::new(),
            cast_resolutions: BTreeMap::new(),
        };
        SpellAcquisitionPlanLikeCpp::no_publications_for_test_like_cpp(
            root,
            source_snapshot,
            professions,
        )
    }

    fn capacity_plan(new_professions: Vec<u32>) -> PrimaryProfessionCapacityPlanLikeCpp {
        PrimaryProfessionCapacityPlanLikeCpp {
            configured_max: 11,
            used_before: 0,
            free_before: 11,
            existing_professions: Vec::new(),
            new_professions: new_professions
                .into_iter()
                .map(
                    |skill_id| crate::profession::PlannedPrimaryProfessionLikeCpp {
                        skill_id,
                        equipment_slot: None,
                    },
                )
                .collect(),
            slot_normalizations: Vec::new(),
        }
    }

    fn base_input(
        skill_value: &dyn Fn(u32) -> Option<u16>,
        knows_spell: &dyn Fn(u32) -> bool,
    ) -> TrainerOfferInputLikeCpp {
        let mut skill_rows = HashMap::new();
        if let Some(value) = skill_value(164) {
            skill_rows.insert(
                164,
                wow_world_core::session::RepresentedPlayerSkillLikeCpp {
                    skill_id: 164,
                    step: 0,
                    value,
                    max: value,
                    profession_slot: -1,
                    state: wow_world_core::session::RepresentedPlayerSkillStateLikeCpp::Unchanged,
                },
            );
        }
        let spell_rows = [100_u32, 200, 201, 202, 203]
            .into_iter()
            .filter(|spell_id| knows_spell(*spell_id))
            .map(|spell_id| {
                let spell_id = spell_id as i32;
                (
                    spell_id,
                    wow_world_spell::RepresentedPlayerSpellLikeCpp {
                        spell_id,
                        active: true,
                        disabled: false,
                        dependent: false,
                        favorite: false,
                        state: wow_world_spell::RepresentedPlayerSpellStateLikeCpp::Unchanged,
                    },
                )
            })
            .collect();
        TrainerOfferInputLikeCpp {
            source_spell_id: 100,
            is_exact_member: true,
            class_race: TrainerAdmissionProofLikeCpp::Proven(true),
            condition: TrainerAdmissionProofLikeCpp::Proven(true),
            directly_known: false,
            required_skill: None,
            skill_rows,
            required_abilities: [0; 3],
            spell_rows,
            required_level: 1,
            player_level: 80,
            product: TrainerProductLikeCpp::Direct,
            battle_pet: TrainerBattlePetProofLikeCpp::NotBattlePet,
            effective_price: 95,
        }
    }

    fn decide_without_late_work(input: TrainerOfferInputLikeCpp) -> TrainerOfferDecisionLikeCpp {
        decide_trainer_offer_like_cpp(
            input,
            |_| panic!("an earlier admission gate must short-circuit projection"),
            |_| panic!("an earlier admission gate must short-circuit capacity"),
        )
    }

    #[test]
    fn price_preserves_every_cpp_rank_and_float_rounding_edges() {
        use wow_constants::reputation::ReputationRankLikeCpp::*;
        assert_eq!(
            [
                Hated, Hostile, Unfriendly, Neutral, Friendly, Honored, Revered, Exalted
            ]
            .map(|rank| trainer_price_like_cpp(100, rank)),
            [100, 100, 100, 100, 95, 90, 85, 80]
        );
        assert_eq!(trainer_price_like_cpp(0, Friendly), 0);
        assert_eq!(trainer_price_like_cpp(1, Friendly), 0);
        assert_eq!(trainer_price_like_cpp(2_207_541, Friendly), 2_097_164);
        assert_eq!(trainer_price_like_cpp(16_777_217, Neutral), 16_777_216);
        assert_eq!(trainer_price_like_cpp(u32::MAX, Exalted), 3_435_973_888);
    }

    #[test]
    fn hidden_and_known_gates_short_circuit_in_contract_order() {
        let skill = |_| Some(450);
        let known = |_| false;
        let mut input = base_input(&skill, &known);
        input.is_exact_member = false;
        assert_eq!(
            decide_without_late_work(input),
            TrainerOfferDecisionLikeCpp::Hidden(
                TrainerHiddenReasonLikeCpp::MissingTrainerMembership
            )
        );

        let mut input = base_input(&skill, &known);
        input.class_race = TrainerAdmissionProofLikeCpp::Proven(false);
        assert!(matches!(
            decide_without_late_work(input),
            TrainerOfferDecisionLikeCpp::Hidden(TrainerHiddenReasonLikeCpp::ClassOrRaceMismatch)
        ));

        let mut input = base_input(&skill, &known);
        input.condition = TrainerAdmissionProofLikeCpp::Indeterminate;
        assert!(matches!(
            decide_without_late_work(input),
            TrainerOfferDecisionLikeCpp::Hidden(TrainerHiddenReasonLikeCpp::ConditionIndeterminate)
        ));

        let mut input = base_input(&skill, &known);
        input.directly_known = true;
        input.required_skill = Some((164, 500));
        assert_eq!(
            decide_without_late_work(input),
            TrainerOfferDecisionLikeCpp::Known(TrainerKnownReasonLikeCpp::DirectSourceSpell)
        );
    }

    #[test]
    fn skill_each_ability_and_level_are_unavailable_in_cpp_order() {
        let skill = |_| Some(74);
        let known = |_| true;
        let mut input = base_input(&skill, &known);
        input.required_skill = Some((164, 75));
        assert!(matches!(
            decide_without_late_work(input),
            TrainerOfferDecisionLikeCpp::Unavailable(
                TrainerUnavailableReasonLikeCpp::RequiredSkill { .. }
            )
        ));

        for (missing_index, missing_spell) in [200_u32, 202, 203].into_iter().enumerate() {
            let known = move |spell_id| spell_id != missing_spell;
            let mut input = base_input(&skill, &known);
            input.required_abilities = [200, 202, 203];
            assert_eq!(
                decide_without_late_work(input),
                TrainerOfferDecisionLikeCpp::Unavailable(
                    TrainerUnavailableReasonLikeCpp::RequiredAbility {
                        spell_id: missing_spell,
                        index: missing_index as u8,
                    }
                )
            );
        }

        let mut input = base_input(&skill, &known);
        input.required_level = 81;
        assert!(matches!(
            decide_without_late_work(input),
            TrainerOfferDecisionLikeCpp::Unavailable(
                TrainerUnavailableReasonLikeCpp::RequiredLevel {
                    required: 81,
                    actual: 80
                }
            )
        ));
    }

    #[test]
    fn wrapper_known_requires_all_valid_player_learn_targets() {
        let skill = |_| None;
        let all_known = |spell_id| matches!(spell_id, 200 | 201);
        let mut input = base_input(&skill, &all_known);
        input.product = TrainerProductLikeCpp::Wrapper {
            valid_learn_targets: vec![200, 201],
        };
        assert_eq!(
            decide_without_late_work(input),
            TrainerOfferDecisionLikeCpp::Known(TrainerKnownReasonLikeCpp::AllValidWrapperTargets)
        );

        let none_known = |_| false;
        let mut input = base_input(&skill, &none_known);
        input.product = TrainerProductLikeCpp::Wrapper {
            valid_learn_targets: Vec::new(),
        };
        assert_eq!(
            decide_without_late_work(input),
            TrainerOfferDecisionLikeCpp::Unavailable(
                TrainerUnavailableReasonLikeCpp::InvalidOrUnsupportedWrapper
            )
        );
    }

    #[test]
    fn direct_battle_pet_species_is_a_purchasable_offer_and_wrapper_keeps_acquisition() {
        let skill = |_| None;
        let known = |_| false;
        let mut input = base_input(&skill, &known);
        input.battle_pet = TrainerBattlePetProofLikeCpp::Species(77);
        assert_eq!(
            decide_without_late_work(input),
            TrainerOfferDecisionLikeCpp::AvailableBattlePet(PreparedBattlePetTrainerOfferLikeCpp {
                source_spell_id: 100,
                effective_price: 95,
                species_id: 77,
            })
        );

        // C++ `Trainer::TeachSpell` resolves `IsCastable()` first: a
        // wrapper-castable spell with a battle-pet classification never
        // reaches `AddPet` and keeps the normal acquisition path, but
        // retains the species for the shared silent cap and visual
        // suppression (`Trainer.cpp:99-109,121-125`).
        let mut input = base_input(&skill, &known);
        input.battle_pet = TrainerBattlePetProofLikeCpp::Species(77);
        input.product = TrainerProductLikeCpp::Wrapper {
            valid_learn_targets: vec![200],
        };
        let decision = decide_trainer_offer_like_cpp(
            input,
            |root| {
                assert_eq!(root, SpellAcquisitionRootLikeCpp::TrainerWrapperCast(100));
                SpellAcquisitionOutcomeLikeCpp::Deterministic(acquisition_plan(root, vec![]))
            },
            |roots| Ok(capacity_plan(roots.to_vec())),
        );
        let TrainerOfferDecisionLikeCpp::Available(offer) = decision else {
            panic!("wrapper-castable battle-pet spell keeps the acquisition path");
        };
        assert_eq!(offer.battle_pet_species_id, Some(77));
    }

    #[test]
    fn indeterminate_battle_pet_metadata_and_acquisition_fail_closed() {
        let skill = |_| None;
        let known = |_| false;
        let mut input = base_input(&skill, &known);
        input.battle_pet = TrainerBattlePetProofLikeCpp::Indeterminate;
        assert_eq!(
            decide_without_late_work(input),
            TrainerOfferDecisionLikeCpp::Unavailable(
                TrainerUnavailableReasonLikeCpp::BattlePetMetadataIndeterminate
            )
        );

        let input = base_input(&skill, &known);
        assert!(matches!(
            decide_trainer_offer_like_cpp(
                input,
                |_| SpellAcquisitionOutcomeLikeCpp::Indeterminate(
                    SpellAcquisitionIndeterminateLikeCpp::MissingTrainerProjectionMetadata
                ),
                |_| unreachable!(),
            ),
            TrainerOfferDecisionLikeCpp::Unavailable(
                TrainerUnavailableReasonLikeCpp::AcquisitionIndeterminate(_)
            )
        ));
    }

    #[test]
    fn available_preserves_projection_price_and_all_profession_roots() {
        let skill = |_| None;
        let known = |_| false;
        let decision = decide_trainer_offer_like_cpp(
            base_input(&skill, &known),
            |root| {
                assert_eq!(root, SpellAcquisitionRootLikeCpp::DirectLearn(100));
                SpellAcquisitionOutcomeLikeCpp::Deterministic(acquisition_plan(
                    root,
                    vec![164, 165],
                ))
            },
            |roots| {
                assert_eq!(roots, [164, 165]);
                Ok(capacity_plan(roots.to_vec()))
            },
        );
        let TrainerOfferDecisionLikeCpp::Available(offer) = decision else {
            panic!("complete evidence must prepare an offer");
        };
        assert_eq!(offer.source_spell_id, 100);
        assert_eq!(offer.effective_price, 95);
        assert_eq!(
            offer.acquisition_plan.root_primary_profession_skill_ids,
            vec![164, 165]
        );
        assert_eq!(
            offer
                .profession_plan
                .new_professions
                .iter()
                .map(|profession| profession.skill_id)
                .collect::<Vec<_>>(),
            vec![164, 165]
        );
    }

    #[test]
    fn partially_known_wrapper_uses_wrapper_root_and_capacity_failure_stays_unavailable() {
        let skill = |_| None;
        let knows_first = |spell_id| spell_id == 200;
        let mut input = base_input(&skill, &knows_first);
        input.product = TrainerProductLikeCpp::Wrapper {
            valid_learn_targets: vec![200, 201],
        };
        let decision = decide_trainer_offer_like_cpp(
            input,
            |root| {
                assert_eq!(root, SpellAcquisitionRootLikeCpp::TrainerWrapperCast(100));
                SpellAcquisitionOutcomeLikeCpp::Deterministic(acquisition_plan(root, vec![164]))
            },
            |_| {
                Err(
                    PrimaryProfessionCapacityPlanErrorLikeCpp::CapacityExceeded {
                        configured_max: 2,
                        used: 2,
                        requested_new: 1,
                    },
                )
            },
        );
        assert_eq!(
            decision,
            TrainerOfferDecisionLikeCpp::Unavailable(
                TrainerUnavailableReasonLikeCpp::ProfessionCapacity(
                    PrimaryProfessionCapacityPlanErrorLikeCpp::CapacityExceeded {
                        configured_max: 2,
                        used: 2,
                        requested_new: 1,
                    }
                )
            )
        );
    }
}
