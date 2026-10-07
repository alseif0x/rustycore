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

use wow_data::{BattlePetClassificationLikeCpp, SkillLineAbilityCoverageLikeCpp};

#[cfg(test)]
use wow_packet::packets::spell::PlaySpellVisualKit;

use crate::session::WorldSession;
use wow_conditions as conditions;
use wow_world_application::{
    TrainerAdmissionProofLikeCpp, TrainerBattlePetProofLikeCpp, TrainerListOfferResultLikeCpp,
    TrainerOfferDecisionLikeCpp, TrainerOfferInputLikeCpp, TrainerOfferPreflightLikeCpp,
    TrainerProductLikeCpp, TrainerUnavailableReasonLikeCpp,
    finish_trainer_offer_after_projection_like_cpp, prepare_trainer_offer_like_cpp,
    resolve_creature_trainer_like_cpp, trainer_condition_admission_proof_like_cpp,
    trainer_list_required_npc_flags_like_cpp, trainer_price_like_cpp,
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

mod host;
mod registrations;

// ── Handler implementations ───────────────────────────────────────────────────

impl WorldSession {
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

    #[cfg(test)]
    pub async fn handle_trainer_buy_spell(&mut self, pkt: wow_packet::WorldPacket) {
        let generators = self.id_generators_for_test_like_cpp();
        let selection = self
            .battle_pet_selection_store_like_cpp()
            .cloned()
            .unwrap_or_else(|| {
                Arc::new(wow_data::battle_pet_selection::BattlePetSelectionStoreLikeCpp::default())
            });
        wow_world_application::handle_trainer_buy_spell_with_generator_like_cpp(
            self,
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
