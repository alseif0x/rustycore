// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! The admitted trainer purchase, moved out of the World shell.
//!
//! C++ `WorldSession::HandleTrainerBuySpellOpcode` (`Handlers/NPCHandler.cpp:132`,
//! registered `Opcodes.cpp:978` as `STATUS_LOGGEDIN`/`PROCESS_INPLACE`) and the
//! `Trainer::TeachSpell` order it drives (`Entities/Creature/Trainer.cpp:79-145`).
//! The body keeps its original parse, gates, order, log strings and packets; only
//! the access path changed. The World session lends its hub, so the representative
//! reads, the realm failure publications and the canonical kick use the hub while
//! the shell-only capabilities (the NPCHandler admission context, the acquisition
//! snapshot, the durable save, the exclusive money fence, the trainer contexts and
//! the represented battle-pet journal) stay behind [`TrainerBuySpellHostLikeCpp`].

use std::collections::BTreeMap;
use std::future::Future;

use tracing::info;
use wow_core::{ObjectGuid, ObjectGuidGenerator};
use wow_data::TrainerSpellLikeCpp;
use wow_data::battle_pet_selection::BattlePetSelectionStoreLikeCpp;
use wow_packet::WorldPacket;
use wow_packet::packets::trainer::TrainerBuyFailed;
use wow_spell_acquisition::{
    PlayerAcquisitionLifecycleLikeCpp, PlayerCastAcquisitionResolutionLikeCpp,
    PlayerFuturePlayerConditionResolutionLikeCpp, PlayerSpellAcquisitionSnapshotLikeCpp,
    SpellAcquisitionSnapshotAdapterErrorLikeCpp,
};
use wow_world_core::session::{HubMut, HubRef};
use wow_world_lifecycle::ExclusivePlayerMoneyPersistenceLikeCpp;

use super::{
    AppTrainerBuyAdmissionCxLikeCpp, AppTrainerBuyCx, PreparedBattlePetTrainerOfferLikeCpp,
    TRAINER_BUY_NPC_FLAGS_LIKE_CPP, TrainerAcquisitionPublicationLikeCpp,
    TrainerAcquisitionResultLikeCpp, TrainerBuyAdmissionLikeCpp, TrainerOfferDecisionLikeCpp,
};
use crate::spell_acquisition::snapshot_has_pending_durable_save_like_cpp;

/// Capabilities of the existing admitted trainer purchase that the World shell
/// still owns: the session hub, the admission/projection contexts built from
/// disjoint session owners and catalogs, the money/durable-save fences and the
/// represented battle-pet journal. Each method is invoked at the exact point the
/// World body invoked its counterpart, so no step value or resumption is needed.
pub trait TrainerBuySpellHostLikeCpp {
    /// The session's hub, which the moved body uses for represented reads, realm
    /// failure publication and the canonical kick.
    fn trainer_buy_spell_hub_ref_like_cpp(&self) -> HubRef<'_>;
    fn trainer_buy_spell_hub_mut_like_cpp(&mut self) -> HubMut<'_>;

    /// C++ `HandleTrainerBuySpellOpcode` admission (`NPCHandler.cpp:132-202`):
    /// the trainer-window proof and the packet parse read the player aura shell
    /// and the session catalogs, which the moved body does not reach.
    fn trainer_buy_admission_context_like_cpp(&mut self) -> AppTrainerBuyAdmissionCxLikeCpp<'_>;

    /// The absolute acquisition snapshot of C++ `Player::SaveToDB` reads the
    /// session's represented spell, skill, trait and override authority.
    fn spell_acquisition_snapshot_like_cpp(
        &self,
        lifecycle: PlayerAcquisitionLifecycleLikeCpp,
        future_player_condition_resolutions: Vec<PlayerFuturePlayerConditionResolutionLikeCpp>,
        cast_resolutions: BTreeMap<u32, PlayerCastAcquisitionResolutionLikeCpp>,
    ) -> Result<PlayerSpellAcquisitionSnapshotLikeCpp, SpellAcquisitionSnapshotAdapterErrorLikeCpp>;

    /// The pre-purchase save owns the duplicate-login claim and the durable
    /// Player save fence; its outcome is not read, exactly as before.
    fn save_current_player_to_db_with_generator_like_cpp<'a>(
        &'a mut self,
        item_guid_generator: &'a ObjectGuidGenerator,
    ) -> impl Future<Output = ()> + Send;

    /// Closes detached payout admission and acquires the exclusive character
    /// money fence the purchase retains through its COMMIT.
    fn begin_exclusive_player_money_persistence_like_cpp(
        &mut self,
    ) -> impl Future<Output = Option<ExclusivePlayerMoneyPersistenceLikeCpp>> + Send;

    /// The admitted acquisition runtime is built from the session's disjoint
    /// owners, selected catalogs and cfg(test) fixtures.
    fn trainer_buy_context_like_cpp<'a>(
        &'a mut self,
        item_guid_generator: &'a ObjectGuidGenerator,
        battle_pet_selection: &'a BattlePetSelectionStoreLikeCpp,
    ) -> AppTrainerBuyCx<'a>;

    /// The trainer offer decision is projected from the complete spell, skill,
    /// condition and represented player authority of the session.
    fn trainer_offer_decision_like_cpp(
        &mut self,
        trainer_id: u32,
        trainer_spell: &TrainerSpellLikeCpp,
        faction_template_id: u32,
    ) -> TrainerOfferDecisionLikeCpp;

    /// Issue #161's recoverable saga owns the battle-pet branch end to end; the
    /// moved body discards its execution report exactly as this shell did.
    fn execute_battle_pet_trainer_purchase_with_generator_like_cpp<'a>(
        &'a mut self,
        item_guid_generator: &'a ObjectGuidGenerator,
        battle_pet_selection_store: &'a BattlePetSelectionStoreLikeCpp,
        money_persistence: ExclusivePlayerMoneyPersistenceLikeCpp,
        trainer_guid: ObjectGuid,
        trainer_id: u32,
        offer: PreparedBattlePetTrainerOfferLikeCpp,
    ) -> impl Future<Output = ()> + Send;

    /// C++ `Trainer.cpp:99-109`: the silent per-species capacity gate reads the
    /// represented battle-pet account journal, which lives on the session shell.
    fn trainer_buy_battle_pet_capacity_capped_like_cpp(&self, species_id: u32) -> bool;

    /// Canonical represented money, read under the session's inventory owner.
    fn resolved_player_money_like_cpp(&self) -> Option<u64>;

    /// C++ `Player::SaveToDB` drains the represented quest objectives the
    /// committed purchase changed, through the session's quest state.
    fn drain_represented_quest_objective_progress_with_generator_like_cpp<'a>(
        &'a mut self,
        item_guid_generator: &'a ObjectGuidGenerator,
    ) -> impl Future<Output = ()> + Send;
}

/// Handle `CMSG_TRAINER_BUY_SPELL` (0x34ae).
///
/// Revalidates the immutable offer under the exclusive character-money
/// boundary, commits its fee and prepared acquisition once, then publishes
/// the C++ money/visual/learning order from the committed result.
pub async fn handle_trainer_buy_spell_with_generator_like_cpp<H>(
    host: &mut H,
    item_guid_generator: &ObjectGuidGenerator,
    battle_pet_selection_store: &BattlePetSelectionStoreLikeCpp,
    pkt: WorldPacket,
) where
    H: TrainerBuySpellHostLikeCpp + Send,
{
    let Some(req) = host
        .trainer_buy_admission_context_like_cpp()
        .admit_packet_like_cpp(pkt)
    else {
        return;
    };

    let trainer_guid = req.trainer_guid;
    let trainer_id = req.trainer_id;
    let spell_id = req.spell_id;

    // Ordinary in-world LearnSpell/skill mutations remain dirty until
    // Player::SaveToDB. Persist that current authority before preparing
    // the trainer's absolute replacement; rejecting normal dirty state
    // would make trainers unusable between autosaves. The duplicate-login
    // claim keeps this save and the following purchase under the same sole
    // live Player authority, while the purchase revalidates everything
    // after both awaits.
    if let Ok(snapshot) = host.spell_acquisition_snapshot_like_cpp(
        PlayerAcquisitionLifecycleLikeCpp::InWorld,
        Vec::new(),
        BTreeMap::new(),
    ) && snapshot_has_pending_durable_save_like_cpp(&snapshot)
    {
        host.save_current_player_to_db_with_generator_like_cpp(item_guid_generator)
            .await;
    }
    // Close detached money admission and reconcile every previously
    // admitted payout before deriving the price, balance or acquisition
    // snapshot that will be persisted.
    let Some(money_persistence) = host
        .begin_exclusive_player_money_persistence_like_cpp()
        .await
    else {
        return;
    };

    // The await above is an intentional race boundary. Re-resolve every
    // mutable/current authority rather than trusting the preliminary
    // membership proof retained only to match the early C++ failure path.
    let Some(fresh_access) = host
        .trainer_buy_spell_hub_ref_like_cpp()
        .represented_npc_can_interact_with_like_cpp(
            trainer_guid,
            TRAINER_BUY_NPC_FLAGS_LIKE_CPP,
            0,
        )
    else {
        return;
    };
    let fresh_resolution = {
        let context =
            host.trainer_buy_context_like_cpp(item_guid_generator, battle_pet_selection_store);
        context.resolve_buy_spell_after_boundary_like_cpp(trainer_guid, trainer_id, spell_id as u32)
    };
    let fresh_trainer_spell = match fresh_resolution {
        Err(TrainerBuyAdmissionLikeCpp::InteractionMismatch) => return,
        Err(
            TrainerBuyAdmissionLikeCpp::TrainerStoreUnavailable
            | TrainerBuyAdmissionLikeCpp::TrainerUnavailable
            | TrainerBuyAdmissionLikeCpp::SpellUnavailable,
        ) => {
            host.trainer_buy_spell_hub_mut_like_cpp()
                .core
                .send_packet_realm(&TrainerBuyFailed {
                    trainer_guid,
                    spell_id,
                    reason: 0,
                });
            return;
        }
        Ok(trainer_spell) => trainer_spell,
    };
    let decision = host.trainer_offer_decision_like_cpp(
        trainer_id as u32,
        &fresh_trainer_spell,
        fresh_access.faction_template_id,
    );
    let offer = match decision {
        TrainerOfferDecisionLikeCpp::Available(offer) => offer,
        TrainerOfferDecisionLikeCpp::AvailableBattlePet(offer) => {
            // Issue #161: the recoverable saga owns the battle-pet
            // branch end to end (admission, charge, durable command,
            // one pet, completion, compensation and publication).
            host.execute_battle_pet_trainer_purchase_with_generator_like_cpp(
                item_guid_generator,
                battle_pet_selection_store,
                money_persistence,
                trainer_guid,
                trainer_id as u32,
                offer,
            )
            .await;
            return;
        }
        _ => {
            host.trainer_buy_spell_hub_mut_like_cpp()
                .core
                .send_packet_realm(&TrainerBuyFailed {
                    trainer_guid,
                    spell_id,
                    reason: 0,
                });
            return;
        }
    };
    // C++ `Trainer.cpp:99-109`: every spell with a confirmed battle-pet
    // species — castable or not — applies the silent per-species
    // capacity gate (no packet, no charge) before the money check.
    if let Some(species_id) = offer.battle_pet_species_id {
        let capped = host.trainer_buy_battle_pet_capacity_capped_like_cpp(species_id);
        if capped {
            return;
        }
    }
    let Some(old_money) = host.resolved_player_money_like_cpp() else {
        return;
    };
    let price = u64::from(offer.effective_price);
    if old_money < price {
        host.trainer_buy_spell_hub_mut_like_cpp()
            .core
            .send_packet_realm(&TrainerBuyFailed {
                trainer_guid,
                spell_id,
                reason: 1,
            });
        return;
    }
    let new_money = old_money - price;

    let Ok(current_snapshot) = host.spell_acquisition_snapshot_like_cpp(
        PlayerAcquisitionLifecycleLikeCpp::InWorld,
        offer
            .acquisition_plan
            .source_snapshot
            .future_player_condition_resolutions
            .clone(),
        offer
            .acquisition_plan
            .source_snapshot
            .cast_resolutions
            .clone(),
    ) else {
        host.trainer_buy_spell_hub_mut_like_cpp()
            .core
            .send_packet_realm(&TrainerBuyFailed {
                trainer_guid,
                spell_id,
                reason: 0,
            });
        return;
    };
    let Some(player_guid) = current_snapshot.character_guid else {
        host.trainer_buy_spell_hub_mut_like_cpp()
            .core
            .send_packet_realm(&TrainerBuyFailed {
                trainer_guid,
                spell_id,
                reason: 0,
            });
        return;
    };
    let completion = {
        let mut runtime =
            host.trainer_buy_context_like_cpp(item_guid_generator, battle_pet_selection_store);
        runtime
            .execute_admitted_acquisition_like_cpp(
                money_persistence,
                &offer,
                &current_snapshot,
                old_money,
                new_money,
                &TrainerAcquisitionPublicationLikeCpp {
                    trainer_guid,
                    player_guid,
                    trainer_position: fresh_access.position,
                    suppress_visuals: offer.battle_pet_species_id.is_some(),
                },
            )
            .await
    };
    use TrainerAcquisitionResultLikeCpp as Result;
    match completion.result {
        Result::Applied => {}
        Result::InvalidPreparation => {
            host.trainer_buy_spell_hub_mut_like_cpp()
                .core
                .send_packet_realm(&TrainerBuyFailed {
                    trainer_guid,
                    spell_id,
                    reason: 0,
                });
            return;
        }
        Result::PersistenceUnavailable => return,
        Result::RuntimeInstallationFailed => {
            host.trainer_buy_spell_hub_mut_like_cpp().core.kick(
                "committed trainer acquisition could not install runtime state; relog required",
            );
            return;
        }
        Result::MoneyOwnerUnavailable => {
            host.trainer_buy_spell_hub_mut_like_cpp()
                .core
                .kick("canonical Player money owner became unavailable after trainer COMMIT");
            return;
        }
        Result::WriterFenceFailed => {
            host.trainer_buy_spell_hub_mut_like_cpp()
                .core
                .kick("trainer socket ordering fence failed after durable acquisition");
            return;
        }
        Result::PublicationFailed => {
            host.trainer_buy_spell_hub_mut_like_cpp().core.kick(
                "committed trainer acquisition could not publish runtime state; relog required",
            );
            return;
        }
    }
    // Preserve the exclusion until failure handling above has completed.
    drop(completion);
    host.drain_represented_quest_objective_progress_with_generator_like_cpp(item_guid_generator)
        .await;

    info!(
        account = host.trainer_buy_spell_hub_ref_like_cpp().core.account_id,
        trainer_id,
        spell_id,
        effective_price = offer.effective_price,
        remaining_money = new_money,
        "Trainer purchase committed and published"
    );
}
