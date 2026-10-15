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

#[cfg(test)]
use wow_data::SkillLineAbilityCoverageLikeCpp;

#[cfg(test)]
use wow_packet::packets::spell::PlaySpellVisualKit;

use crate::session::WorldSession;
use wow_world_application::{
    TrainerBattlePetProofLikeCpp, TrainerListOfferResultLikeCpp, TrainerOfferDecisionLikeCpp,
    TrainerOfferInputLikeCpp, TrainerOfferPreflightLikeCpp, TrainerUnavailableReasonLikeCpp,
    finish_trainer_offer_after_projection_like_cpp, prepare_trainer_offer_like_cpp,
    resolve_creature_trainer_like_cpp, trainer_list_required_npc_flags_like_cpp,
    trainer_price_like_cpp,
};
#[cfg(test)]
use wow_world_player_conditions::TrainerAdmissionProofLikeCpp;
#[cfg(test)]
use wow_world_player_conditions::trainer_condition_admission_proof_like_cpp;

// ── Handler registrations ─────────────────────────────────────────────────────

mod host;

// ── Handler implementations ───────────────────────────────────────────────────

impl WorldSession {
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
    pub(crate) async fn handle_trainer_list(&mut self, hello: wow_packet::packets::gossip::Hello) {
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
    pub(crate) async fn handle_trainer_buy_spell(&mut self, pkt: wow_packet::WorldPacket) {
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
