// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The admitted normal-trainer acquisition, from preparation through publication.
//! NPC admission and battle-pet saga routing precede this operation. The adapter
//! supplies the existing money exclusion; this operation retains it through both
//! physical writer fences. C++: Trainer::TeachSpell, Trainer.cpp:79-145.

use super::*;
use crate::trainer_offer::PreparedTrainerOfferLikeCpp;
use std::future::Future;

pub(crate) struct TrainerAcquisitionPublicationLikeCpp {
    pub trainer_guid: wow_core::ObjectGuid,
    pub player_guid: wow_core::ObjectGuid,
    pub trainer_position: wow_core::Position,
    pub suppress_visuals: bool,
}

/// Capabilities of the existing admitted trainer operation, not a new owner.
/// No synchronous Player/Map guard is returned or retained across these awaits.
pub(crate) trait TrainerAcquisitionRuntimeLikeCpp:
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
pub(crate) enum TrainerAcquisitionResultLikeCpp {
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
pub(crate) struct TrainerAcquisitionCompletionLikeCpp<Exclusion> {
    pub result: TrainerAcquisitionResultLikeCpp,
    _exclusion: Option<Exclusion>,
}

enum PreparedTrainerAcquisitionLikeCpp {
    Durable(PreparedPlayerSpellAcquisitionLikeCpp),
    ActionsOnly(PreparedPlayerSpellAcquisitionActionsLikeCpp),
    NoChange,
}

pub(crate) async fn execute_trainer_acquisition_like_cpp<R: TrainerAcquisitionRuntimeLikeCpp>(
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
