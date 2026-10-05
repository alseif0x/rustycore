// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Trainer handlers: CMSG_TRAINER_LIST, CMSG_TRAINER_BUY_SPELL.
//!
//! Flow for CMSG_TRAINER_LIST:
//!   1. Parse trainer GUID from packet.
//!   2. Resolve creature entry (NPC template ID) from the live map.
//!   3. Look up TrainerId in the process-wide C++-shaped trainer store.
//!   4. Read the trainer spells from that same immutable store snapshot.
//!   5. Determine usability per spell (known / available / unavailable).
//!   6. Send SMSG_TRAINER_LIST.
//!
//! Flow for CMSG_TRAINER_BUY_SPELL:
//!   1. Parse trainer GUID, trainer ID, spell ID.
//!   2. Recompute the same immutable offer from the current snapshot.
//!   3. Reject an unavailable offer or insufficient exact discounted money.
//!   4. Under the exclusive money boundary, atomically persist the fee and the
//!      exact prepared spell/skill result.
//!   5. Install committed runtime state, then publish money, visual kits and
//!      acquisition actions in C++ success order.
//!
//! C++ refs: `WorldSession::HandleTrainerListOpcode` / `SendTrainerList`
//! (`Handlers/NPCHandler.cpp:98-132`) and `Trainer::SendSpells` /
//! `Trainer::TeachSpell` (`Entities/Creature/Trainer.cpp:41-231`).

use std::sync::Arc;

use tracing::{debug, info, warn};

use wow_data::{BattlePetClassificationLikeCpp, SkillLineAbilityCoverageLikeCpp};

#[cfg(test)]
use wow_packet::packets::spell::PlaySpellVisualKit;
use wow_packet::packets::trainer::TrainerBuyFailed;

use crate::session::WorldSession;
use wow_conditions as conditions;
use wow_world_application::{
    TRAINER_BUY_NPC_FLAGS_LIKE_CPP, TrainerAdmissionProofLikeCpp, TrainerBattlePetProofLikeCpp,
    TrainerBuyAdmissionLikeCpp, TrainerListOfferResultLikeCpp, TrainerOfferDecisionLikeCpp,
    TrainerOfferInputLikeCpp, TrainerOfferPreflightLikeCpp, TrainerProductLikeCpp,
    TrainerUnavailableReasonLikeCpp, finish_trainer_offer_after_projection_like_cpp,
    prepare_trainer_offer_like_cpp, resolve_creature_trainer_like_cpp,
    trainer_condition_admission_proof_like_cpp, trainer_list_required_npc_flags_like_cpp,
    trainer_price_like_cpp,
    trainer_spell_class_race_fit_like_cpp as fit_trainer_spell_class_race_rows_like_cpp,
    trainer_spell_product_like_cpp as classify_trainer_spell_product_like_cpp,
};

fn trainer_spell_class_race_fit_like_cpp(
    session: &WorldSession,
    spell_id: u32,
) -> TrainerAdmissionProofLikeCpp {
    let Some(skills) = session.skill_store() else {
        return TrainerAdmissionProofLikeCpp::Indeterminate;
    };
    let Ok(spell_id) = i32::try_from(spell_id) else {
        return TrainerAdmissionProofLikeCpp::Indeterminate;
    };
    let rows = match skills.skill_line_ability_coverage_by_spell_like_cpp(spell_id) {
        SkillLineAbilityCoverageLikeCpp::CoveredZero => {
            return TrainerAdmissionProofLikeCpp::Proven(true);
        }
        SkillLineAbilityCoverageLikeCpp::Indeterminate(_) => {
            return TrainerAdmissionProofLikeCpp::Indeterminate;
        }
        SkillLineAbilityCoverageLikeCpp::Rows(rows) => rows,
    };
    let race = crate::session::hub_ref(&session).player_race_like_cpp();
    let class = crate::session::hub_ref(&session).player_class_like_cpp();
    fit_trainer_spell_class_race_rows_like_cpp(skills, rows, race, class)
}

fn trainer_spell_product_like_cpp(session: &WorldSession, spell_id: u32) -> TrainerProductLikeCpp {
    let Some(catalog) = session.catalogs.spell_catalogs.spell_acquisition_catalog() else {
        return TrainerProductLikeCpp::InvalidOrUnsupportedWrapper;
    };
    classify_trainer_spell_product_like_cpp(catalog, spell_id)
}

// ── Handler registrations ─────────────────────────────────────────────────────

mod registrations;

// ── Handler implementations ───────────────────────────────────────────────────

impl WorldSession {
    /// C++ `HandleShowTradeSkillOpcode` currently only logs this request.
    pub async fn handle_show_trade_skill(&mut self) {
        if let Some(player_guid) = self.player_guid() {
            debug!("ShowTradeSkill from {:?}", player_guid);
        } else {
            debug!("ShowTradeSkill from account {}", self.core.account_id);
        }
    }

    fn trainer_spell_condition_proof_like_cpp(
        &self,
        trainer_id: u32,
        spell_id: u32,
    ) -> TrainerAdmissionProofLikeCpp {
        let Some(store) = self.condition_store() else {
            return TrainerAdmissionProofLikeCpp::Indeterminate;
        };
        let Some(player_object) =
            crate::session::hub_ref(self).build_condition_player_object_like_cpp()
        else {
            return TrainerAdmissionProofLikeCpp::Indeterminate;
        };
        let player_condition_store = self.player_condition_store();
        let Some(player_condition_context) = self.represented_player_condition_context_like_cpp()
        else {
            return TrainerAdmissionProofLikeCpp::Indeterminate;
        };
        let Some(player_unit_snapshot) =
            crate::session::hub_ref(self).condition_player_unit_snapshot_like_cpp()
        else {
            return TrainerAdmissionProofLikeCpp::Indeterminate;
        };
        let player_snapshot = crate::session::hub_ref(self).condition_player_snapshot_like_cpp();
        let mut unsupported = false;
        let meets = conditions::is_object_meeting_trainer_spell_conditions_like_cpp(
            store.as_ref(),
            trainer_id,
            spell_id,
            Some(&player_object),
            |condition, source_info| {
                source_info.set_unit_target_snapshot(0, player_unit_snapshot);
                source_info.set_player_target_snapshot(0, player_snapshot);
                if let Some(store) = player_condition_store {
                    source_info.set_player_condition_store(store.as_ref());
                    if let Some(context) = player_condition_context.as_context(self) {
                        source_info.set_player_condition_context(0, context);
                    }
                }
                match conditions::condition_meets_basic_like_cpp(
                    condition,
                    source_info,
                    |current_area, required_area| current_area == required_area,
                ) {
                    conditions::ConditionMeetResult::Evaluated(value) => value,
                    conditions::ConditionMeetResult::Unsupported => {
                        unsupported = true;
                        false
                    }
                }
            },
        );
        trainer_condition_admission_proof_like_cpp(meets, unsupported)
    }

    fn trainer_offer_decision_like_cpp(
        &mut self,
        trainer_id: u32,
        trainer_spell: &wow_data::TrainerSpellLikeCpp,
        faction_template_id: u32,
    ) -> TrainerOfferDecisionLikeCpp {
        self.trainer_list_publication_context_like_cpp()
            .trainer_offer_decision_like_cpp(trainer_id, trainer_spell, faction_template_id)
    }

    /// Handle `CMSG_TRAINER_LIST` (0x34ad).
    ///
    /// Opens the trainer window: resolves the NPC, reads the loaded trainer store,
    /// and sends SMSG_TRAINER_LIST back to the client.
    pub async fn handle_trainer_list(&mut self, hello: wow_packet::packets::gossip::Hello) {
        self.send_trainer_list_like_cpp(hello, None);
    }

    pub(crate) async fn handle_trainer_list_for_gossip_option_like_cpp(
        &mut self,
        hello: wow_packet::packets::gossip::Hello,
        gossip_menu_id: u32,
        gossip_option_id: u32,
    ) {
        self.send_trainer_list_like_cpp(hello, Some((gossip_menu_id, gossip_option_id)));
    }

    fn send_trainer_list_like_cpp(
        &mut self,
        hello: wow_packet::packets::gossip::Hello,
        gossip_option: Option<(u32, u32)>,
    ) {
        self.trainer_list_publication_context_like_cpp()
            .send_trainer_list_like_cpp(hello, gossip_option);
    }

    /// Handle `CMSG_TRAINER_BUY_SPELL` (0x34ae).
    ///
    /// Revalidates the immutable offer under the exclusive character-money
    /// boundary, commits its fee and prepared acquisition once, then publishes
    /// the C++ money/visual/learning order from the committed result.
    pub async fn handle_trainer_buy_spell_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        battle_pet_selection_store: &wow_data::battle_pet_selection::BattlePetSelectionStoreLikeCpp,
        pkt: wow_packet::WorldPacket,
    ) {
        let Some(req) = self
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
        if let Ok(snapshot) = self.spell_acquisition_snapshot_like_cpp(
            crate::spell_acquisition::PlayerAcquisitionLifecycleLikeCpp::InWorld,
            Vec::new(),
            std::collections::BTreeMap::new(),
        ) && crate::spell_acquisition::snapshot_has_pending_durable_save_like_cpp(&snapshot)
        {
            self.save_current_player_to_db_with_generator_like_cpp(item_guid_generator)
                .await;
        }
        // Close detached money admission and reconcile every previously
        // admitted payout before deriving the price, balance or acquisition
        // snapshot that will be persisted.
        let Some(money_persistence) = self
            .begin_exclusive_player_money_persistence_like_cpp()
            .await
        else {
            return;
        };

        // The await above is an intentional race boundary. Re-resolve every
        // mutable/current authority rather than trusting the preliminary
        // membership proof retained only to match the early C++ failure path.
        let Some(fresh_access) = crate::session::hub_ref(self)
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
                self.trainer_buy_context_like_cpp(item_guid_generator, battle_pet_selection_store);
            context.resolve_buy_spell_after_boundary_like_cpp(
                trainer_guid,
                trainer_id,
                spell_id as u32,
            )
        };
        let fresh_trainer_spell = match fresh_resolution {
            Err(TrainerBuyAdmissionLikeCpp::InteractionMismatch) => return,
            Err(
                TrainerBuyAdmissionLikeCpp::TrainerStoreUnavailable
                | TrainerBuyAdmissionLikeCpp::TrainerUnavailable
                | TrainerBuyAdmissionLikeCpp::SpellUnavailable,
            ) => {
                self.send_packet_realm(&TrainerBuyFailed {
                    trainer_guid,
                    spell_id,
                    reason: 0,
                });
                return;
            }
            Ok(trainer_spell) => trainer_spell,
        };
        let decision = self.trainer_offer_decision_like_cpp(
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
                self.execute_battle_pet_trainer_purchase_with_generator_like_cpp(
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
                self.send_packet_realm(&TrainerBuyFailed {
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
            let capped = crate::session::cx_pets_ref(self)
                .battle_pet_account_owner_lease_like_cpp()
                .map(|(owner, _)| owner.has_max_pet_count_like_cpp(species_id, self.player_guid()))
                .unwrap_or(true);
            if capped {
                return;
            }
        }
        let Some(old_money) = self.resolved_player_money_like_cpp() else {
            return;
        };
        let price = u64::from(offer.effective_price);
        if old_money < price {
            self.send_packet_realm(&TrainerBuyFailed {
                trainer_guid,
                spell_id,
                reason: 1,
            });
            return;
        }
        let new_money = old_money - price;

        let Ok(current_snapshot) = self.spell_acquisition_snapshot_like_cpp(
            crate::spell_acquisition::PlayerAcquisitionLifecycleLikeCpp::InWorld,
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
            self.send_packet_realm(&TrainerBuyFailed {
                trainer_guid,
                spell_id,
                reason: 0,
            });
            return;
        };
        let Some(player_guid) = current_snapshot.character_guid else {
            self.send_packet_realm(&TrainerBuyFailed {
                trainer_guid,
                spell_id,
                reason: 0,
            });
            return;
        };
        let completion = {
            let mut runtime =
                self.trainer_buy_context_like_cpp(item_guid_generator, battle_pet_selection_store);
            runtime
                .execute_admitted_acquisition_like_cpp(
                    money_persistence,
                    &offer,
                    &current_snapshot,
                    old_money,
                    new_money,
                    &crate::spell_acquisition::TrainerAcquisitionPublicationLikeCpp {
                        trainer_guid,
                        player_guid,
                        trainer_position: fresh_access.position,
                        suppress_visuals: offer.battle_pet_species_id.is_some(),
                    },
                )
                .await
        };
        use crate::spell_acquisition::TrainerAcquisitionResultLikeCpp as Result;
        match completion.result {
            Result::Applied => {}
            Result::InvalidPreparation => {
                self.send_packet_realm(&TrainerBuyFailed {
                    trainer_guid,
                    spell_id,
                    reason: 0,
                });
                return;
            }
            Result::PersistenceUnavailable => return,
            Result::RuntimeInstallationFailed => {
                self.kick(
                    "committed trainer acquisition could not install runtime state; relog required",
                );
                return;
            }
            Result::MoneyOwnerUnavailable => {
                self.kick("canonical Player money owner became unavailable after trainer COMMIT");
                return;
            }
            Result::WriterFenceFailed => {
                self.kick("trainer socket ordering fence failed after durable acquisition");
                return;
            }
            Result::PublicationFailed => {
                self.kick(
                    "committed trainer acquisition could not publish runtime state; relog required",
                );
                return;
            }
        }
        // Preserve the exclusion until failure handling above has completed.
        drop(completion);
        self.drain_represented_quest_objective_progress_with_generator_like_cpp(
            item_guid_generator,
        )
        .await;

        info!(
            account = self.core.account_id,
            trainer_id,
            spell_id,
            effective_price = offer.effective_price,
            remaining_money = new_money,
            "Trainer purchase committed and published"
        );
    }

    #[cfg(test)]
    pub async fn handle_trainer_buy_spell(&mut self, pkt: wow_packet::WorldPacket) {
        let generators = self.id_generators_for_test_like_cpp();
        let selection = self
            .battle_pet_selection_store_like_cpp()
            .cloned()
            .unwrap_or_else(|| {
                Arc::new(wow_data::battle_pet_selection::BattlePetSelectionStoreLikeCpp::default())
            });
        self.handle_trainer_buy_spell_with_generator_like_cpp(
            generators.item.as_ref(),
            selection.as_ref(),
            pkt,
        )
        .await;
    }
}

#[cfg(test)]
#[path = "../../unit_tests/handlers/trainer/tests/mod.rs"]
mod tests;
