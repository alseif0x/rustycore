// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Validated application boundary for a deterministic acquisition plan.
//!
//! The planner is the sole semantic owner.  This module only proves that its
//! causal stream is internally coherent, pins it to the exact current player
//! authority, and translates the final snapshot into durable/runtime rows.

use super::*;
#[cfg(test)]
use crate::profession::{
    PrimaryProfessionCapacityPlanLikeCpp, PrimaryProfessionEquipmentSlotLikeCpp,
};
#[cfg(test)]
use wow_persistence::{
    PlayerSpellAcquisitionDurableOperationLikeCpp,
    PlayerSpellAcquisitionSkillRowLikeCpp as DurablePlayerSkillRowLikeCpp,
    PlayerSpellAcquisitionSpellRowLikeCpp as DurablePlayerSpellRowLikeCpp,
};
#[cfg(test)]
use wow_persistence::{
    PlayerSpellAcquisitionMoneyReconciliationLikeCpp,
    classify_player_spell_acquisition_money_reconciliation_like_cpp,
};

#[cfg(test)]
#[path = "../../unit_tests/spell_acquisition/application/tests/mod.rs"]
mod tests;

pub(crate) use wow_world_application::{
    PlayerSpellAcquisitionPersistenceOutcomeLikeCpp,
    PlayerSpellAcquisitionPublicationFaultPointLikeCpp,
    PlayerSpellAcquisitionPrepareErrorLikeCpp, PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp,
    PlayerSpellAcquisitionRuntimeLikeCpp, PreparedPlayerSpellAcquisitionActionsLikeCpp,
    PreparedPlayerSpellAcquisitionLikeCpp, PreparedPlayerSpellAcquisitionOutcomeLikeCpp,
    apply_prepared_player_spell_acquisition_actions_like_cpp,
    apply_prepared_player_spell_acquisition_before_save_like_cpp,
    apply_prepared_player_spell_acquisition_like_cpp,
    apply_prepared_player_spell_acquisition_with_before_actions_like_cpp,
    apply_prepared_player_spell_acquisition_with_fault_like_cpp,
    install_prepared_player_spell_acquisition_actions_runtime_like_cpp,
    install_prepared_player_spell_acquisition_runtime_like_cpp,
    persist_player_spell_acquisition_through_port_like_cpp,
    player_spell_acquisition_persistence_request_like_cpp,
    prepare_player_spell_acquisition_like_cpp,
    snapshot_has_pending_durable_save_like_cpp,
    validate_prepared_player_spell_acquisition_actions_runtime_like_cpp,
    validate_prepared_player_spell_acquisition_runtime_like_cpp,
};
