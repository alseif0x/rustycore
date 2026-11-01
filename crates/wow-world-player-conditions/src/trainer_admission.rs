// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Trainer admission proof shared by the player-condition projection and the
//! trainer-offer admission in `wow-world-application` (#1363 A6: moved down
//! from `trainer_purchase/offer.rs` with the quest cluster).

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
