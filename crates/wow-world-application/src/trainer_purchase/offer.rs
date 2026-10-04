// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Pure trainer-offer admission shared by trainer-list and purchase adapters.
//!
//! The caller supplies proofs from current canonical owners. This module
//! decides from those inputs and invokes expensive projections only after the
//! earlier offer gates pass.

use std::collections::{BTreeMap, HashMap};

use wow_data::reputation::ReputationRankLikeCpp;

use crate::profession::{
    PrimaryProfessionCapacityPlanErrorLikeCpp, PrimaryProfessionCapacityPlanLikeCpp,
};
use wow_spell_acquisition::{
    SpellAcquisitionIndeterminateLikeCpp, SpellAcquisitionOutcomeLikeCpp,
    SpellAcquisitionPlanLikeCpp, SpellAcquisitionRootLikeCpp,
};
use wow_world_core::session::RepresentedPlayerSkillLikeCpp;
use wow_world_spell::{RepresentedPlayerSpellLikeCpp, RepresentedPlayerSpellStateLikeCpp};

use super::PreparedTrainerOfferLikeCpp;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrainerHiddenReasonLikeCpp {
    MissingTrainerMembership,
    ClassOrRaceMismatch,
    ClassOrRaceIndeterminate,
    ConditionRejected,
    ConditionIndeterminate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrainerKnownReasonLikeCpp {
    DirectSourceSpell,
    AllValidWrapperTargets,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrainerUnavailableReasonLikeCpp {
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
    BattlePetMetadataIndeterminate,
    AcquisitionIndeterminate(SpellAcquisitionIndeterminateLikeCpp),
    ProfessionCapacity(PrimaryProfessionCapacityPlanErrorLikeCpp),
}

/// C++ `Trainer::TeachSpell` reaches `BattlePetMgr::AddPet` for a confirmed
/// species only when the trainer spell is not castable. The live purchase
/// saga remains a separate operation after this pure offer decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedBattlePetTrainerOfferLikeCpp {
    pub source_spell_id: u32,
    pub effective_price: u32,
    pub species_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrainerOfferDecisionLikeCpp {
    Hidden(TrainerHiddenReasonLikeCpp),
    Known(TrainerKnownReasonLikeCpp),
    Unavailable(TrainerUnavailableReasonLikeCpp),
    Available(PreparedTrainerOfferLikeCpp),
    AvailableBattlePet(PreparedBattlePetTrainerOfferLikeCpp),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrainerAdmissionProofLikeCpp {
    Proven(bool),
    Indeterminate,
}

/// Preserve the target evaluator's fail-closed result when a supported
/// condition passed alongside an unsupported one.
pub fn trainer_condition_admission_proof_like_cpp(
    meets: bool,
    saw_unsupported: bool,
) -> TrainerAdmissionProofLikeCpp {
    if meets {
        // C++ ElseGroups are ORed. A supported passing group proves the
        // result even when another, irrelevant group is not representable.
        TrainerAdmissionProofLikeCpp::Proven(true)
    } else if saw_unsupported {
        TrainerAdmissionProofLikeCpp::Indeterminate
    } else {
        TrainerAdmissionProofLikeCpp::Proven(false)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrainerProductLikeCpp {
    Direct,
    Wrapper { valid_learn_targets: Vec<u32> },
    InvalidOrUnsupportedWrapper,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrainerBattlePetProofLikeCpp {
    NotBattlePet,
    Species(u32),
    Indeterminate,
}

pub struct TrainerOfferInputLikeCpp {
    pub source_spell_id: u32,
    pub is_exact_member: bool,
    pub class_race: TrainerAdmissionProofLikeCpp,
    pub condition: TrainerAdmissionProofLikeCpp,
    pub directly_known: bool,
    pub required_skill: Option<(u32, u16)>,
    pub skill_rows: HashMap<u16, RepresentedPlayerSkillLikeCpp>,
    pub required_abilities: [u32; 3],
    pub spell_rows: BTreeMap<i32, RepresentedPlayerSpellLikeCpp>,
    pub required_level: u8,
    pub player_level: u8,
    pub product: TrainerProductLikeCpp,
    pub battle_pet: TrainerBattlePetProofLikeCpp,
    pub effective_price: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TrainerOfferProjectionLikeCpp {
    pub root: SpellAcquisitionRootLikeCpp,
    pub source_spell_id: u32,
    pub effective_price: u32,
    pub battle_pet_species_id: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrainerOfferPreflightLikeCpp {
    Decision(TrainerOfferDecisionLikeCpp),
    NeedsProjection(TrainerOfferProjectionLikeCpp),
}

fn represented_trainer_spell_known_like_cpp(
    spell_rows: &BTreeMap<i32, RepresentedPlayerSpellLikeCpp>,
    spell_id: u32,
) -> bool {
    i32::try_from(spell_id).ok().is_some_and(|spell_id| {
        spell_rows.get(&spell_id).is_some_and(|row| {
            row.state != RepresentedPlayerSpellStateLikeCpp::Removed && !row.disabled
        })
    })
}

/// Resolve every pre-projection Trainer gate using selected owned snapshots.
/// This lets a World adapter run canonical projection only at its original
/// point, then return the result without handing callbacks into the App.
pub fn prepare_trainer_offer_like_cpp(
    input: TrainerOfferInputLikeCpp,
) -> TrainerOfferPreflightLikeCpp {
    use TrainerOfferPreflightLikeCpp as Preflight;
    if !input.is_exact_member {
        return Preflight::Decision(TrainerOfferDecisionLikeCpp::Hidden(
            TrainerHiddenReasonLikeCpp::MissingTrainerMembership,
        ));
    }
    match input.class_race {
        TrainerAdmissionProofLikeCpp::Proven(false) => {
            return Preflight::Decision(TrainerOfferDecisionLikeCpp::Hidden(
                TrainerHiddenReasonLikeCpp::ClassOrRaceMismatch,
            ));
        }
        TrainerAdmissionProofLikeCpp::Indeterminate => {
            return Preflight::Decision(TrainerOfferDecisionLikeCpp::Hidden(
                TrainerHiddenReasonLikeCpp::ClassOrRaceIndeterminate,
            ));
        }
        TrainerAdmissionProofLikeCpp::Proven(true) => {}
    }
    match input.condition {
        TrainerAdmissionProofLikeCpp::Proven(false) => {
            return Preflight::Decision(TrainerOfferDecisionLikeCpp::Hidden(
                TrainerHiddenReasonLikeCpp::ConditionRejected,
            ));
        }
        TrainerAdmissionProofLikeCpp::Indeterminate => {
            return Preflight::Decision(TrainerOfferDecisionLikeCpp::Hidden(
                TrainerHiddenReasonLikeCpp::ConditionIndeterminate,
            ));
        }
        TrainerAdmissionProofLikeCpp::Proven(true) => {}
    }
    if input.directly_known {
        return Preflight::Decision(TrainerOfferDecisionLikeCpp::Known(
            TrainerKnownReasonLikeCpp::DirectSourceSpell,
        ));
    }
    if let Some((skill_id, required)) = input.required_skill {
        let actual = u16::try_from(skill_id)
            .ok()
            .and_then(|skill_id| input.skill_rows.get(&skill_id).map(|row| row.value))
            .unwrap_or(0);
        if actual < required {
            return Preflight::Decision(TrainerOfferDecisionLikeCpp::Unavailable(
                TrainerUnavailableReasonLikeCpp::RequiredSkill {
                    skill_id,
                    required,
                    actual,
                },
            ));
        }
    }
    for (index, spell_id) in input.required_abilities.into_iter().enumerate() {
        if spell_id != 0 && !represented_trainer_spell_known_like_cpp(&input.spell_rows, spell_id) {
            return Preflight::Decision(TrainerOfferDecisionLikeCpp::Unavailable(
                TrainerUnavailableReasonLikeCpp::RequiredAbility {
                    spell_id,
                    index: index as u8,
                },
            ));
        }
    }
    if input.player_level < input.required_level {
        return Preflight::Decision(TrainerOfferDecisionLikeCpp::Unavailable(
            TrainerUnavailableReasonLikeCpp::RequiredLevel {
                required: input.required_level,
                actual: input.player_level,
            },
        ));
    }

    let root = match input.product {
        TrainerProductLikeCpp::Direct => {
            SpellAcquisitionRootLikeCpp::DirectLearn(input.source_spell_id)
        }
        TrainerProductLikeCpp::Wrapper { valid_learn_targets } => {
            if valid_learn_targets.is_empty() {
                return Preflight::Decision(TrainerOfferDecisionLikeCpp::Unavailable(
                    TrainerUnavailableReasonLikeCpp::InvalidOrUnsupportedWrapper,
                ));
            }
            if valid_learn_targets
                .iter()
                .all(|spell_id| represented_trainer_spell_known_like_cpp(&input.spell_rows, *spell_id))
            {
                return Preflight::Decision(TrainerOfferDecisionLikeCpp::Known(
                    TrainerKnownReasonLikeCpp::AllValidWrapperTargets,
                ));
            }
            SpellAcquisitionRootLikeCpp::TrainerWrapperCast(input.source_spell_id)
        }
        TrainerProductLikeCpp::InvalidOrUnsupportedWrapper => {
            return Preflight::Decision(TrainerOfferDecisionLikeCpp::Unavailable(
                TrainerUnavailableReasonLikeCpp::InvalidOrUnsupportedWrapper,
            ));
        }
    };

    let battle_pet_species_id = match input.battle_pet {
        TrainerBattlePetProofLikeCpp::NotBattlePet => None,
        TrainerBattlePetProofLikeCpp::Species(species_id) => {
            if matches!(root, SpellAcquisitionRootLikeCpp::DirectLearn(_)) {
                return Preflight::Decision(
                    TrainerOfferDecisionLikeCpp::AvailableBattlePet(
                        PreparedBattlePetTrainerOfferLikeCpp {
                            source_spell_id: input.source_spell_id,
                            effective_price: input.effective_price,
                            species_id,
                        },
                    ),
                );
            }
            Some(species_id)
        }
        TrainerBattlePetProofLikeCpp::Indeterminate => {
            return Preflight::Decision(TrainerOfferDecisionLikeCpp::Unavailable(
                TrainerUnavailableReasonLikeCpp::BattlePetMetadataIndeterminate,
            ));
        }
    };

    Preflight::NeedsProjection(TrainerOfferProjectionLikeCpp {
        root,
        source_spell_id: input.source_spell_id,
        effective_price: input.effective_price,
        battle_pet_species_id,
    })
}

/// Complete a preflight only after the World owner has performed its
/// canonical projection and, for a deterministic plan, capacity read.
pub fn finish_trainer_offer_after_projection_like_cpp(
    source_spell_id: u32,
    effective_price: u32,
    battle_pet_species_id: Option<u32>,
    outcome: SpellAcquisitionOutcomeLikeCpp,
    capacity: Option<
        Result<
            PrimaryProfessionCapacityPlanLikeCpp,
            PrimaryProfessionCapacityPlanErrorLikeCpp,
        >,
    >,
) -> TrainerOfferDecisionLikeCpp {
    let plan = match outcome {
        SpellAcquisitionOutcomeLikeCpp::Deterministic(plan) => plan,
        SpellAcquisitionOutcomeLikeCpp::Indeterminate(reason) => {
            return TrainerOfferDecisionLikeCpp::Unavailable(
                TrainerUnavailableReasonLikeCpp::AcquisitionIndeterminate(reason),
            );
        }
    };
    let profession_plan = match capacity.expect("capacity follows deterministic acquisition") {
        Ok(plan) => plan,
        Err(reason) => {
            return TrainerOfferDecisionLikeCpp::Unavailable(
                TrainerUnavailableReasonLikeCpp::ProfessionCapacity(reason),
            );
        }
    };
    TrainerOfferDecisionLikeCpp::Available(PreparedTrainerOfferLikeCpp {
        source_spell_id,
        effective_price,
        acquisition_plan: plan,
        profession_plan,
        battle_pet_species_id,
    })
}

pub fn decide_trainer_offer_like_cpp<Project, Capacity>(
    input: TrainerOfferInputLikeCpp,
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
    match prepare_trainer_offer_like_cpp(input) {
        TrainerOfferPreflightLikeCpp::Decision(decision) => decision,
        TrainerOfferPreflightLikeCpp::NeedsProjection(projection) => {
            let TrainerOfferProjectionLikeCpp {
                root,
                source_spell_id,
                effective_price,
                battle_pet_species_id,
            } = projection;
            let outcome = project(root);
            let capacity = match &outcome {
                SpellAcquisitionOutcomeLikeCpp::Deterministic(plan) => {
                    Some(capacity(&plan.root_primary_profession_skill_ids))
                }
                SpellAcquisitionOutcomeLikeCpp::Indeterminate(_) => None,
            };
            finish_trainer_offer_after_projection_like_cpp(
                source_spell_id,
                effective_price,
                battle_pet_species_id,
                outcome,
                capacity,
            )
        }
    }
}

pub fn trainer_price_like_cpp(base_cost: u32, rank: ReputationRankLikeCpp) -> u32 {
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
#[path = "offer_tests.rs"]
mod tests;
