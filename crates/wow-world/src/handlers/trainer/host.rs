// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! World-side adapter for the application trainer purchase (#1263 F4 remate).
//!
//! The application crate owns the C++ `WorldSession::HandleTrainerBuySpellOpcode`
//! body (`Handlers/NPCHandler.cpp:132`); this session only lends the shell-only
//! capabilities that body cannot reach: the session hub, the NPCHandler
//! admission context, the acquisition snapshot, the durable save, the exclusive
//! money fence, the trainer offer contexts and the represented battle-pet
//! journal. Every method delegates to the existing World operation at the exact
//! point the World body invoked it, so no World body is duplicated and no step
//! value is needed.

use std::collections::BTreeMap;
use std::future::Future;

use wow_core::{ObjectGuid, ObjectGuidGenerator};
use wow_data::TrainerSpellLikeCpp;
use wow_data::battle_pet_selection::BattlePetSelectionStoreLikeCpp;
use wow_spell_acquisition::{
    PlayerAcquisitionLifecycleLikeCpp, PlayerCastAcquisitionResolutionLikeCpp,
    PlayerFuturePlayerConditionResolutionLikeCpp, PlayerSpellAcquisitionSnapshotLikeCpp,
    SpellAcquisitionSnapshotAdapterErrorLikeCpp,
};
use wow_world_application::{
    AppTrainerBuyAdmissionCxLikeCpp, AppTrainerBuyCx, PreparedBattlePetTrainerOfferLikeCpp,
    TrainerBuySpellHostLikeCpp, TrainerOfferDecisionLikeCpp,
};
use wow_world_core::session::{HubMut, HubRef};
use wow_world_lifecycle::ExclusivePlayerMoneyPersistenceLikeCpp;

use crate::session::WorldSession;

impl TrainerBuySpellHostLikeCpp for WorldSession {
    fn trainer_buy_spell_hub_ref_like_cpp(&self) -> HubRef<'_> {
        crate::session::hub_ref(self)
    }

    fn trainer_buy_spell_hub_mut_like_cpp(&mut self) -> HubMut<'_> {
        crate::session::hub_mut(self)
    }

    fn trainer_buy_admission_context_like_cpp(&mut self) -> AppTrainerBuyAdmissionCxLikeCpp<'_> {
        WorldSession::trainer_buy_admission_context_like_cpp(self)
    }

    fn spell_acquisition_snapshot_like_cpp(
        &self,
        lifecycle: PlayerAcquisitionLifecycleLikeCpp,
        future_player_condition_resolutions: Vec<PlayerFuturePlayerConditionResolutionLikeCpp>,
        cast_resolutions: BTreeMap<u32, PlayerCastAcquisitionResolutionLikeCpp>,
    ) -> Result<PlayerSpellAcquisitionSnapshotLikeCpp, SpellAcquisitionSnapshotAdapterErrorLikeCpp>
    {
        WorldSession::spell_acquisition_snapshot_like_cpp(
            self,
            lifecycle,
            future_player_condition_resolutions,
            cast_resolutions,
        )
    }

    fn save_current_player_to_db_with_generator_like_cpp<'a>(
        &'a mut self,
        item_guid_generator: &'a ObjectGuidGenerator,
    ) -> impl Future<Output = ()> + Send {
        async move {
            WorldSession::save_current_player_to_db_with_generator_like_cpp(
                self,
                item_guid_generator,
            )
            .await;
        }
    }

    fn begin_exclusive_player_money_persistence_like_cpp(
        &mut self,
    ) -> impl Future<Output = Option<ExclusivePlayerMoneyPersistenceLikeCpp>> + Send {
        WorldSession::begin_exclusive_player_money_persistence_like_cpp(self)
    }

    fn trainer_buy_context_like_cpp<'a>(
        &'a mut self,
        item_guid_generator: &'a ObjectGuidGenerator,
        battle_pet_selection: &'a BattlePetSelectionStoreLikeCpp,
    ) -> AppTrainerBuyCx<'a> {
        WorldSession::trainer_buy_context_like_cpp(self, item_guid_generator, battle_pet_selection)
    }

    fn trainer_offer_decision_like_cpp(
        &mut self,
        trainer_id: u32,
        trainer_spell: &TrainerSpellLikeCpp,
        faction_template_id: u32,
    ) -> TrainerOfferDecisionLikeCpp {
        WorldSession::trainer_offer_decision_like_cpp(
            self,
            trainer_id,
            trainer_spell,
            faction_template_id,
        )
    }

    fn execute_battle_pet_trainer_purchase_with_generator_like_cpp<'a>(
        &'a mut self,
        item_guid_generator: &'a ObjectGuidGenerator,
        battle_pet_selection_store: &'a BattlePetSelectionStoreLikeCpp,
        money_persistence: ExclusivePlayerMoneyPersistenceLikeCpp,
        trainer_guid: ObjectGuid,
        trainer_id: u32,
        offer: PreparedBattlePetTrainerOfferLikeCpp,
    ) -> impl Future<Output = ()> + Send {
        async move {
            WorldSession::execute_battle_pet_trainer_purchase_with_generator_like_cpp(
                self,
                item_guid_generator,
                battle_pet_selection_store,
                money_persistence,
                trainer_guid,
                trainer_id,
                offer,
            )
            .await;
        }
    }

    fn trainer_buy_battle_pet_capacity_capped_like_cpp(&self, species_id: u32) -> bool {
        crate::session::cx_pets_ref(self)
            .battle_pet_account_owner_lease_like_cpp()
            .map(|(owner, _)| owner.has_max_pet_count_like_cpp(species_id, self.player_guid()))
            .unwrap_or(true)
    }

    fn resolved_player_money_like_cpp(&self) -> Option<u64> {
        WorldSession::resolved_player_money_like_cpp(self)
    }

    fn drain_represented_quest_objective_progress_with_generator_like_cpp<'a>(
        &'a mut self,
        item_guid_generator: &'a ObjectGuidGenerator,
    ) -> impl Future<Output = ()> + Send {
        async move {
            WorldSession::drain_represented_quest_objective_progress_with_generator_like_cpp(
                self,
                item_guid_generator,
            )
            .await;
        }
    }
}
