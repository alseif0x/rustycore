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

#[cfg(test)]
use wow_data::SkillLineAbilityCoverageLikeCpp;

#[cfg(test)]
use wow_packet::packets::spell::PlaySpellVisualKit;

#[cfg(test)]
use crate::session::WorldSession;
#[cfg(test)]
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

// The CMSG_TRAINER_LIST entry points, the offer decision and the trainer
// contexts moved to `wow_world_application` (#1363 slice 12a); `host.rs` lends
// them this session's fields.

#[cfg(test)]
#[path = "../../unit_tests/handlers/trainer/tests/mod.rs"]
mod tests;
