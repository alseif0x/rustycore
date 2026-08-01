// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Pure trainer-offer admission shared by list and purchase adapters.
//!
//! C++ anchors: `Trainer::SendSpells`, `Trainer::CanTeachSpell`,
//! `Trainer::GetSpellState`, `Player::IsSpellFitByClassAndRace`, and
//! `Player::GetReputationPriceDiscount`.  This boundary deliberately repairs
//! the legacy condition-revalidation, transitive-profession, and `float`
//! pricing defects documented by issue #157.

use wow_data::reputation::ReputationRankLikeCpp;

use crate::profession::{
    PrimaryProfessionCapacityPlanErrorLikeCpp, PrimaryProfessionCapacityPlanLikeCpp,
};
use crate::spell_acquisition::{
    SpellAcquisitionIndeterminateLikeCpp, SpellAcquisitionOutcomeLikeCpp,
    SpellAcquisitionPlanLikeCpp, SpellAcquisitionRootLikeCpp,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TrainerHiddenReasonLikeCpp {
    MissingTrainerMembership,
    ClassOrRaceMismatch,
    ClassOrRaceIndeterminate,
    ConditionRejected,
    ConditionIndeterminate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TrainerKnownReasonLikeCpp {
    DirectSourceSpell,
    AllValidWrapperTargets,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TrainerUnavailableReasonLikeCpp {
    InvalidEffectiveMetadata,
    RequiredSkill {
        skill_id: u32,
        required: u16,
        actual: u16,
    },
    RequiredAbility {
        spell_id: u32,
        index: u8,
    },
    RequiredLevel {
        required: u8,
        actual: u8,
    },
    InvalidOrUnsupportedWrapper,
    ConfirmedBattlePetSpecies {
        species_id: u32,
    },
    BattlePetMetadataIndeterminate,
    AcquisitionIndeterminate(SpellAcquisitionIndeterminateLikeCpp),
    ProfessionCapacity(PrimaryProfessionCapacityPlanErrorLikeCpp),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PreparedTrainerOfferLikeCpp {
    pub source_spell_id: u32,
    pub effective_price: u32,
    pub acquisition_plan: SpellAcquisitionPlanLikeCpp,
    pub profession_plan: PrimaryProfessionCapacityPlanLikeCpp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TrainerOfferDecisionLikeCpp {
    Hidden(TrainerHiddenReasonLikeCpp),
    Known(TrainerKnownReasonLikeCpp),
    Unavailable(TrainerUnavailableReasonLikeCpp),
    Available(PreparedTrainerOfferLikeCpp),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrainerAdmissionProofLikeCpp {
    Proven(bool),
    Indeterminate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TrainerProductLikeCpp {
    Direct,
    Wrapper { valid_learn_targets: Vec<u32> },
    InvalidOrUnsupportedWrapper,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrainerBattlePetProofLikeCpp {
    NotBattlePet,
    Species(u32),
    Indeterminate,
}

pub(crate) struct TrainerOfferInputLikeCpp<'a> {
    pub source_spell_id: u32,
    pub is_exact_member: bool,
    pub class_race: TrainerAdmissionProofLikeCpp,
    pub condition: TrainerAdmissionProofLikeCpp,
    pub directly_known: bool,
    pub required_skill: Option<(u32, u16)>,
    pub skill_value: &'a dyn Fn(u32) -> Option<u16>,
    pub required_abilities: [u32; 3],
    pub knows_spell: &'a dyn Fn(u32) -> bool,
    pub required_level: u8,
    pub player_level: u8,
    pub product: TrainerProductLikeCpp,
    pub battle_pet: TrainerBattlePetProofLikeCpp,
    pub effective_price: u32,
}

/// Builds immutable evidence for one current snapshot.  Projection and
/// capacity are closures so the expensive authorities are invoked only after
/// every earlier C++-ordered gate has passed.
pub(crate) fn decide_trainer_offer_like_cpp<Project, Capacity>(
    input: TrainerOfferInputLikeCpp<'_>,
    project: Project,
    capacity: Capacity,
) -> TrainerOfferDecisionLikeCpp
where
    Project: FnOnce(SpellAcquisitionRootLikeCpp) -> SpellAcquisitionOutcomeLikeCpp,
    Capacity: FnOnce(
        &[u32],
    ) -> Result<
        PrimaryProfessionCapacityPlanLikeCpp,
        PrimaryProfessionCapacityPlanErrorLikeCpp,
    >,
{
    if !input.is_exact_member {
        return TrainerOfferDecisionLikeCpp::Hidden(
            TrainerHiddenReasonLikeCpp::MissingTrainerMembership,
        );
    }
    match input.class_race {
        TrainerAdmissionProofLikeCpp::Proven(false) => {
            return TrainerOfferDecisionLikeCpp::Hidden(
                TrainerHiddenReasonLikeCpp::ClassOrRaceMismatch,
            );
        }
        TrainerAdmissionProofLikeCpp::Indeterminate => {
            return TrainerOfferDecisionLikeCpp::Hidden(
                TrainerHiddenReasonLikeCpp::ClassOrRaceIndeterminate,
            );
        }
        TrainerAdmissionProofLikeCpp::Proven(true) => {}
    }
    match input.condition {
        TrainerAdmissionProofLikeCpp::Proven(false) => {
            return TrainerOfferDecisionLikeCpp::Hidden(
                TrainerHiddenReasonLikeCpp::ConditionRejected,
            );
        }
        TrainerAdmissionProofLikeCpp::Indeterminate => {
            return TrainerOfferDecisionLikeCpp::Hidden(
                TrainerHiddenReasonLikeCpp::ConditionIndeterminate,
            );
        }
        TrainerAdmissionProofLikeCpp::Proven(true) => {}
    }
    if input.directly_known {
        return TrainerOfferDecisionLikeCpp::Known(TrainerKnownReasonLikeCpp::DirectSourceSpell);
    }
    if let Some((skill_id, required)) = input.required_skill {
        let actual = (input.skill_value)(skill_id).unwrap_or(0);
        if actual < required {
            return TrainerOfferDecisionLikeCpp::Unavailable(
                TrainerUnavailableReasonLikeCpp::RequiredSkill {
                    skill_id,
                    required,
                    actual,
                },
            );
        }
    }
    for (index, spell_id) in input.required_abilities.into_iter().enumerate() {
        if spell_id != 0 && !(input.knows_spell)(spell_id) {
            return TrainerOfferDecisionLikeCpp::Unavailable(
                TrainerUnavailableReasonLikeCpp::RequiredAbility {
                    spell_id,
                    index: index as u8,
                },
            );
        }
    }
    if input.player_level < input.required_level {
        return TrainerOfferDecisionLikeCpp::Unavailable(
            TrainerUnavailableReasonLikeCpp::RequiredLevel {
                required: input.required_level,
                actual: input.player_level,
            },
        );
    }

    let root = match input.product {
        TrainerProductLikeCpp::Direct => {
            SpellAcquisitionRootLikeCpp::DirectLearn(input.source_spell_id)
        }
        TrainerProductLikeCpp::Wrapper {
            valid_learn_targets,
        } => {
            if valid_learn_targets.is_empty() {
                return TrainerOfferDecisionLikeCpp::Unavailable(
                    TrainerUnavailableReasonLikeCpp::InvalidOrUnsupportedWrapper,
                );
            }
            if valid_learn_targets
                .iter()
                .all(|spell_id| (input.knows_spell)(*spell_id))
            {
                return TrainerOfferDecisionLikeCpp::Known(
                    TrainerKnownReasonLikeCpp::AllValidWrapperTargets,
                );
            }
            SpellAcquisitionRootLikeCpp::TrainerWrapperCast(input.source_spell_id)
        }
        TrainerProductLikeCpp::InvalidOrUnsupportedWrapper => {
            return TrainerOfferDecisionLikeCpp::Unavailable(
                TrainerUnavailableReasonLikeCpp::InvalidOrUnsupportedWrapper,
            );
        }
    };

    match input.battle_pet {
        TrainerBattlePetProofLikeCpp::NotBattlePet => {}
        TrainerBattlePetProofLikeCpp::Species(species_id) => {
            return TrainerOfferDecisionLikeCpp::Unavailable(
                TrainerUnavailableReasonLikeCpp::ConfirmedBattlePetSpecies { species_id },
            );
        }
        TrainerBattlePetProofLikeCpp::Indeterminate => {
            return TrainerOfferDecisionLikeCpp::Unavailable(
                TrainerUnavailableReasonLikeCpp::BattlePetMetadataIndeterminate,
            );
        }
    }

    let acquisition_plan = match project(root) {
        SpellAcquisitionOutcomeLikeCpp::Deterministic(plan) => plan,
        SpellAcquisitionOutcomeLikeCpp::Indeterminate(reason) => {
            return TrainerOfferDecisionLikeCpp::Unavailable(
                TrainerUnavailableReasonLikeCpp::AcquisitionIndeterminate(reason),
            );
        }
    };
    let profession_plan = match capacity(&acquisition_plan.root_primary_profession_skill_ids) {
        Ok(plan) => plan,
        Err(reason) => {
            return TrainerOfferDecisionLikeCpp::Unavailable(
                TrainerUnavailableReasonLikeCpp::ProfessionCapacity(reason),
            );
        }
    };
    TrainerOfferDecisionLikeCpp::Available(PreparedTrainerOfferLikeCpp {
        source_spell_id: input.source_spell_id,
        effective_price: input.effective_price,
        acquisition_plan,
        profession_plan,
    })
}

/// C++ `MoneyCost * float reputationDiscount`, including its observable f32 rounding.
pub(crate) fn trainer_price_like_cpp(base_cost: u32, rank: ReputationRankLikeCpp) -> u32 {
    let discount = if rank <= ReputationRankLikeCpp::Neutral {
        1.0_f32
    } else {
        1.0_f32
            - 0.05_f32
                * f32::from(
                    rank.as_u8()
                        .saturating_sub(ReputationRankLikeCpp::Neutral.as_u8()),
                )
    };
    (base_cost as f32 * discount) as u32
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::spell_acquisition::{
        PlayerAcquisitionLifecycleLikeCpp, PlayerSpellAcquisitionSnapshotLikeCpp,
    };

    fn acquisition_plan(
        root: SpellAcquisitionRootLikeCpp,
        professions: Vec<u32>,
    ) -> SpellAcquisitionPlanLikeCpp {
        let source_snapshot = PlayerSpellAcquisitionSnapshotLikeCpp {
            spells: Vec::new(),
            skills: Vec::new(),
            occupied_skill_slots: 0,
            overrides: Vec::new(),
            race: 1,
            class: 1,
            level: 80,
            lifecycle: PlayerAcquisitionLifecycleLikeCpp::InWorld,
            future_player_condition_resolutions: Vec::new(),
            cast_resolutions: BTreeMap::new(),
        };
        SpellAcquisitionPlanLikeCpp {
            root,
            source_snapshot: source_snapshot.clone(),
            mutations: Vec::new(),
            spell_transitions: Vec::new(),
            skill_transitions: Vec::new(),
            override_transitions: Vec::new(),
            root_primary_profession_skill_ids: professions,
            profession_association_inputs: Vec::new(),
            post_commit_actions: Vec::new(),
            diagnostics: Vec::new(),
            resulting_snapshot: source_snapshot,
        }
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

    fn base_input<'a>(
        skill_value: &'a dyn Fn(u32) -> Option<u16>,
        knows_spell: &'a dyn Fn(u32) -> bool,
    ) -> TrainerOfferInputLikeCpp<'a> {
        TrainerOfferInputLikeCpp {
            source_spell_id: 100,
            is_exact_member: true,
            class_race: TrainerAdmissionProofLikeCpp::Proven(true),
            condition: TrainerAdmissionProofLikeCpp::Proven(true),
            directly_known: false,
            required_skill: None,
            skill_value,
            required_abilities: [0; 3],
            knows_spell,
            required_level: 1,
            player_level: 80,
            product: TrainerProductLikeCpp::Direct,
            battle_pet: TrainerBattlePetProofLikeCpp::NotBattlePet,
            effective_price: 95,
        }
    }

    fn decide_without_late_work(
        input: TrainerOfferInputLikeCpp<'_>,
    ) -> TrainerOfferDecisionLikeCpp {
        decide_trainer_offer_like_cpp(
            input,
            |_| panic!("an earlier admission gate must short-circuit projection"),
            |_| panic!("an earlier admission gate must short-circuit capacity"),
        )
    }

    #[test]
    fn price_preserves_every_cpp_rank_and_float_rounding_edges() {
        use ReputationRankLikeCpp::*;
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
    fn battle_pet_and_indeterminate_acquisition_fail_closed() {
        let skill = |_| None;
        let known = |_| false;
        let mut input = base_input(&skill, &known);
        input.battle_pet = TrainerBattlePetProofLikeCpp::Species(77);
        assert_eq!(
            decide_without_late_work(input),
            TrainerOfferDecisionLikeCpp::Unavailable(
                TrainerUnavailableReasonLikeCpp::ConfirmedBattlePetSpecies { species_id: 77 }
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
