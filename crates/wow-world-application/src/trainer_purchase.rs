// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The admitted normal-trainer acquisition, from preparation through publication.
//! NPC admission and battle-pet saga routing precede this operation. The adapter
//! supplies the existing money exclusion; this operation retains it through both
//! physical writer fences. C++: Trainer::TeachSpell, Trainer.cpp:79-145.

use std::future::Future;

use crate::spell_acquisition::{
    PlayerSpellAcquisitionRuntimeLikeCpp, PreparedPlayerSpellAcquisitionActionsLikeCpp,
    PreparedPlayerSpellAcquisitionLikeCpp,
    PreparedPlayerSpellAcquisitionOutcomeLikeCpp,
    apply_prepared_player_spell_acquisition_actions_like_cpp,
    install_prepared_player_spell_acquisition_actions_runtime_like_cpp,
    install_prepared_player_spell_acquisition_runtime_like_cpp,
    prepare_player_spell_acquisition_like_cpp,
    validate_prepared_player_spell_acquisition_actions_runtime_like_cpp,
    validate_prepared_player_spell_acquisition_runtime_like_cpp,
};
use crate::PrimaryProfessionCapacityPlanLikeCpp;
use wow_spell_acquisition::{SpellAcquisitionPlanLikeCpp, PlayerSpellAcquisitionSnapshotLikeCpp};

mod context;
mod controller;
mod buy_admission;
pub use buy_admission::AppTrainerBuyAdmissionCxLikeCpp;
mod offer;
mod projection;
pub use projection::TrainerProjectionCatalogsLikeCpp;
mod publication;
mod runtime_install;

pub use context::{AppTrainerCx, TrainerAcquisitionCatalogsLikeCpp};
pub use controller::{
    AppTrainerBuyCx, AppTrainerListCx, TrainerBuyAdmissionLikeCpp,
    TrainerListCatalogsLikeCpp, TrainerListOfferResultLikeCpp,
    resolve_creature_trainer_like_cpp,
    trainer_list_required_npc_flags_like_cpp,
    trainer_spell_class_race_fit_like_cpp, trainer_spell_product_like_cpp,
    TRAINER_BUY_NPC_FLAGS_LIKE_CPP,
};
pub use offer::{
    PreparedBattlePetTrainerOfferLikeCpp, TrainerAdmissionProofLikeCpp,
    TrainerBattlePetProofLikeCpp, TrainerHiddenReasonLikeCpp, TrainerKnownReasonLikeCpp,
    TrainerOfferDecisionLikeCpp, TrainerOfferInputLikeCpp, TrainerOfferPreflightLikeCpp,
    TrainerOfferProjectionLikeCpp, TrainerProductLikeCpp,
    TrainerUnavailableReasonLikeCpp, decide_trainer_offer_like_cpp,
    finish_trainer_offer_after_projection_like_cpp, prepare_trainer_offer_like_cpp,
    trainer_condition_admission_proof_like_cpp, trainer_price_like_cpp,
};
#[cfg(any(test, feature = "test-fixtures"))]
pub use context::TrainerAcquisitionFixturesLikeCpp;
pub use runtime_install::{
    install_player_spell_acquisition_runtime_snapshot_like_cpp,
    install_represented_spell_acquisition_runtime_like_cpp,
    publish_spell_acquisition_action_like_cpp,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedTrainerOfferLikeCpp {
    pub source_spell_id: u32,
    pub effective_price: u32,
    pub acquisition_plan: SpellAcquisitionPlanLikeCpp,
    pub profession_plan: PrimaryProfessionCapacityPlanLikeCpp,
    /// C++ resolves the battle-pet species before `IsCastable()`
    /// (`Trainer.cpp:99-128`): a castable spell with a confirmed species
    /// keeps the normal wrapper acquisition but retains the silent
    /// per-species capacity gate and suppresses the trainer visual kits.
    /// `None` for spells without a battle-pet classification.
    pub battle_pet_species_id: Option<u32>,
}

pub struct TrainerAcquisitionPublicationLikeCpp {
    pub trainer_guid: wow_core::ObjectGuid,
    pub player_guid: wow_core::ObjectGuid,
    pub trainer_position: wow_core::Position,
    pub suppress_visuals: bool,
}

/// Capabilities of the existing admitted trainer operation, not a new owner.
/// No synchronous Player/Map guard is returned or retained across these awaits.
pub trait TrainerAcquisitionRuntimeLikeCpp:
    PlayerSpellAcquisitionRuntimeLikeCpp
{
    type MoneyExclusion: Send;
    fn commit_acquisition(
        &mut self,
        exclusion: Self::MoneyExclusion,
        prepared: Option<&PreparedPlayerSpellAcquisitionLikeCpp>,
        before: u64,
        after: u64,
    ) -> impl Future<Output = Option<Self::MoneyExclusion>> + Send;
    fn stage_money(&mut self, before: u64, after: u64) -> bool;
    fn publish_money(&mut self, after: u64);
    fn fence_instance_before_realm(&self) -> impl Future<Output = bool> + Send;
    fn publish_visuals(&self, publication: &TrainerAcquisitionPublicationLikeCpp);
    fn fence_realm_before_instance(&self) -> impl Future<Output = bool> + Send;
    fn publish_skills(&mut self);
}

#[derive(Clone, Copy)]
pub enum TrainerAcquisitionResultLikeCpp {
    Applied,
    InvalidPreparation,
    PersistenceUnavailable,
    RuntimeInstallationFailed,
    MoneyOwnerUnavailable,
    WriterFenceFailed,
    PublicationFailed,
}

/// Keep admission closed while the caller publishes a failure or quarantines
/// the session. Returning only an enum would release the fence too early.
pub struct TrainerAcquisitionCompletionLikeCpp<Exclusion> {
    pub result: TrainerAcquisitionResultLikeCpp,
    _exclusion: Option<Exclusion>,
}

enum PreparedTrainerAcquisitionLikeCpp {
    Durable(PreparedPlayerSpellAcquisitionLikeCpp),
    ActionsOnly(PreparedPlayerSpellAcquisitionActionsLikeCpp),
    NoChange,
}

pub async fn execute_trainer_acquisition_like_cpp<R: TrainerAcquisitionRuntimeLikeCpp>(
    runtime: &mut R,
    exclusion: R::MoneyExclusion,
    offer: &PreparedTrainerOfferLikeCpp,
    snapshot: &PlayerSpellAcquisitionSnapshotLikeCpp,
    money_before: u64,
    money_after: u64,
    publication: &TrainerAcquisitionPublicationLikeCpp,
) -> TrainerAcquisitionCompletionLikeCpp<R::MoneyExclusion> {
    use TrainerAcquisitionResultLikeCpp as Result;
    let mut exclusion = Some(exclusion);
    macro_rules! finish {
        ($result:expr) => {
            return TrainerAcquisitionCompletionLikeCpp {
                result: $result,
                _exclusion: exclusion,
            }
        };
    }
    let prepared = match prepare_player_spell_acquisition_like_cpp(
        &offer.acquisition_plan,
        &offer.profession_plan,
        snapshot,
    ) {
        Ok(PreparedPlayerSpellAcquisitionOutcomeLikeCpp::Ready(prepared)) => {
            PreparedTrainerAcquisitionLikeCpp::Durable(prepared)
        }
        Ok(PreparedPlayerSpellAcquisitionOutcomeLikeCpp::ActionsOnly(prepared)) => {
            PreparedTrainerAcquisitionLikeCpp::ActionsOnly(prepared)
        }
        Ok(PreparedPlayerSpellAcquisitionOutcomeLikeCpp::NoChange) => {
            PreparedTrainerAcquisitionLikeCpp::NoChange
        }
        Ok(PreparedPlayerSpellAcquisitionOutcomeLikeCpp::AlreadyApplied) | Err(_) => {
            finish!(Result::InvalidPreparation);
        }
    };
    let valid = match &prepared {
        PreparedTrainerAcquisitionLikeCpp::Durable(prepared) => {
            validate_prepared_player_spell_acquisition_runtime_like_cpp(runtime, prepared)
        }
        PreparedTrainerAcquisitionLikeCpp::ActionsOnly(prepared) => {
            validate_prepared_player_spell_acquisition_actions_runtime_like_cpp(runtime, prepared)
        }
        PreparedTrainerAcquisitionLikeCpp::NoChange => Ok(()),
    };
    if valid.is_err() {
        finish!(Result::InvalidPreparation);
    }
    let durable = match &prepared {
        PreparedTrainerAcquisitionLikeCpp::Durable(prepared) => Some(prepared),
        _ => None,
    };
    exclusion = runtime
        .commit_acquisition(
            exclusion.take().expect("admitted money exclusion"),
            durable,
            money_before,
            money_after,
        )
        .await;
    if exclusion.is_none() {
        finish!(Result::PersistenceUnavailable);
    }
    let installed = match prepared {
        PreparedTrainerAcquisitionLikeCpp::Durable(prepared) => {
            let skills_changed =
                prepared.source_snapshot.skills != prepared.runtime_snapshot.skills;
            install_prepared_player_spell_acquisition_runtime_like_cpp(runtime, &prepared)
                .map(|actions| (skills_changed, Some(actions)))
        }
        PreparedTrainerAcquisitionLikeCpp::ActionsOnly(prepared) => {
            install_prepared_player_spell_acquisition_actions_runtime_like_cpp(runtime, prepared)
                .map(|actions| (false, Some(actions)))
        }
        PreparedTrainerAcquisitionLikeCpp::NoChange => Ok((false, None)),
    };
    let Ok((skills_changed, actions)) = installed else {
        finish!(Result::RuntimeInstallationFailed);
    };
    if !runtime.stage_money(money_before, money_after) {
        finish!(Result::MoneyOwnerUnavailable);
    }
    if money_before != money_after {
        runtime.publish_money(money_after);
    }
    if !runtime.fence_instance_before_realm().await {
        finish!(Result::WriterFenceFailed);
    }
    runtime.publish_visuals(publication);
    if !runtime.fence_realm_before_instance().await {
        finish!(Result::WriterFenceFailed);
    }
    if skills_changed {
        runtime.publish_skills();
    }
    if let Some(actions) = actions
        && apply_prepared_player_spell_acquisition_actions_like_cpp(runtime, &actions).is_err()
    {
        finish!(Result::PublicationFailed);
    }
    finish!(Result::Applied);
}

#[cfg(test)]
#[path = "../unit_tests/trainer_purchase.rs"]
mod tests;
