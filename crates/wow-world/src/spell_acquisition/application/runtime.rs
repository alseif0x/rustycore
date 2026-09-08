// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Generic runtime application for an already validated spell-acquisition plan.
//!
//! The application owns identity, preflight, ordering and fault boundaries.
//! Concrete Session state, row conversion and packet presentation live in the
//! runtime adapter.

use std::collections::BTreeSet;

use super::{
    PlayerSpellAcquisitionPublicationFaultPointLikeCpp,
    PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp, PlayerSpellAcquisitionSnapshotLikeCpp,
    PreparedPlayerSpellAcquisitionActionsLikeCpp, PreparedPlayerSpellAcquisitionLikeCpp,
    SpellAcquisitionPostCommitActionLikeCpp,
};

/// Runtime capability required by the spell-acquisition application.
///
/// The application deliberately knows only the canonical identity and the
/// ordered mutation/publication seam. Session rows, tombstone retention and
/// packets belong to the concrete adapter.
pub(crate) trait PlayerSpellAcquisitionRuntimeLikeCpp {
    fn character_guid(&self) -> Option<wow_core::ObjectGuid>;
    fn has_canonical_player(&self) -> bool;
    fn install_snapshot(
        &mut self,
        snapshot: &PlayerSpellAcquisitionSnapshotLikeCpp,
        new_non_durable_skill_tombstone_ids: &BTreeSet<u16>,
    ) -> Result<(), PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp>;
    fn begin_action_batch(&mut self);
    fn record_action(&mut self, action: SpellAcquisitionPostCommitActionLikeCpp);
    fn grant_dual_wield(&mut self) -> bool;
    fn publish_action(
        &mut self,
        action: &SpellAcquisitionPostCommitActionLikeCpp,
        trait_definition_id: Option<i32>,
    );
}

/// Applies and publishes an already committed plan. This function contains no
/// await point: the live runtime is installed first and the ordered C++ action
/// stream is observed only afterwards.
pub(crate) fn apply_prepared_player_spell_acquisition_like_cpp<
    R: ?Sized + PlayerSpellAcquisitionRuntimeLikeCpp,
>(
    runtime: &mut R,
    prepared: &PreparedPlayerSpellAcquisitionLikeCpp,
) -> Result<(), PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp> {
    require_prepared_runtime_character_like_cpp(runtime, prepared.character_guid)?;
    apply_prepared_player_spell_acquisition_with_fault_like_cpp(runtime, prepared, |_| Ok(()))
}

/// Applies the committed runtime snapshot, then invokes one infallible
/// publication hook immediately before the ordered learning actions. Trainer
/// purchases use the hook for C++'s money and visual publications, after every
/// runtime owner and replacement has already been proven safe.
pub(crate) fn apply_prepared_player_spell_acquisition_with_before_actions_like_cpp<
    R: ?Sized + PlayerSpellAcquisitionRuntimeLikeCpp,
    Before,
>(
    runtime: &mut R,
    prepared: &PreparedPlayerSpellAcquisitionLikeCpp,
    before_actions: Before,
) -> Result<(), PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp>
where
    Before: FnOnce(&mut R),
{
    require_prepared_runtime_character_like_cpp(runtime, prepared.character_guid)?;
    apply_player_spell_acquisition_runtime_snapshot_with_before_actions_and_fault_like_cpp(
        runtime,
        &prepared.runtime_snapshot,
        &prepared.non_durable_skill_tombstone_ids,
        &prepared.post_commit_actions,
        before_actions,
        |_| Ok(()),
    )
}

/// Installs an already committed runtime snapshot while deferring every
/// observable learning action. Cross-socket workflows use the returned action
/// bundle after their physical writer fences have preserved C++ order.
pub(crate) fn install_prepared_player_spell_acquisition_runtime_like_cpp<
    R: ?Sized + PlayerSpellAcquisitionRuntimeLikeCpp,
>(
    runtime: &mut R,
    prepared: &PreparedPlayerSpellAcquisitionLikeCpp,
) -> Result<
    PreparedPlayerSpellAcquisitionActionsLikeCpp,
    PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp,
> {
    require_prepared_runtime_character_like_cpp(runtime, prepared.character_guid)?;
    preflight_player_spell_acquisition_runtime_owners_like_cpp(
        runtime,
        &prepared.post_commit_actions,
    )?;
    apply_player_spell_acquisition_runtime_snapshot_with_before_actions_and_fault_like_cpp(
        runtime,
        &prepared.runtime_snapshot,
        &prepared.non_durable_skill_tombstone_ids,
        &[],
        |_| {},
        |_| Ok(()),
    )?;
    apply_player_spell_acquisition_non_packet_runtime_actions_like_cpp(
        runtime,
        &prepared.post_commit_actions,
    )?;
    Ok(PreparedPlayerSpellAcquisitionActionsLikeCpp {
        character_guid: prepared.character_guid,
        runtime_snapshot: prepared.runtime_snapshot.clone(),
        post_commit_actions: prepared.post_commit_actions.clone(),
        runtime_actions_already_applied: true,
    })
}

/// Proves every runtime owner needed after COMMIT is available before a
/// trainer fee or acquisition row becomes durable.
pub(crate) fn validate_prepared_player_spell_acquisition_runtime_like_cpp<
    R: ?Sized + PlayerSpellAcquisitionRuntimeLikeCpp,
>(
    runtime: &R,
    prepared: &PreparedPlayerSpellAcquisitionLikeCpp,
) -> Result<(), PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp> {
    require_prepared_runtime_character_like_cpp(runtime, prepared.character_guid)?;
    preflight_player_spell_acquisition_runtime_owners_like_cpp(
        runtime,
        &prepared.post_commit_actions,
    )
}

pub(crate) fn validate_prepared_player_spell_acquisition_actions_runtime_like_cpp<
    R: ?Sized + PlayerSpellAcquisitionRuntimeLikeCpp,
>(
    runtime: &R,
    prepared: &PreparedPlayerSpellAcquisitionActionsLikeCpp,
) -> Result<(), PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp> {
    require_prepared_runtime_character_like_cpp(runtime, prepared.character_guid)?;
    preflight_player_spell_acquisition_runtime_owners_like_cpp(
        runtime,
        &prepared.post_commit_actions,
    )
}

/// Apply non-packet effects from an actions-only cast immediately after its
/// trainer fee commits. Later publication still records the complete causal
/// action stream, but will not apply these runtime effects a second time.
pub(crate) fn install_prepared_player_spell_acquisition_actions_runtime_like_cpp<
    R: ?Sized + PlayerSpellAcquisitionRuntimeLikeCpp,
>(
    runtime: &mut R,
    mut prepared: PreparedPlayerSpellAcquisitionActionsLikeCpp,
) -> Result<
    PreparedPlayerSpellAcquisitionActionsLikeCpp,
    PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp,
> {
    validate_prepared_player_spell_acquisition_actions_runtime_like_cpp(runtime, &prepared)?;
    apply_player_spell_acquisition_non_packet_runtime_actions_like_cpp(
        runtime,
        &prepared.post_commit_actions,
    )?;
    prepared.runtime_actions_already_applied = true;
    Ok(prepared)
}

/// Applies the exact prepared plan with C++ `Player::LearnSpell` timing. The
/// runtime and packets change synchronously while the plan's persistence states
/// remain dirty for the ordinary `Player::SaveToDB` lifecycle.
pub(crate) fn apply_prepared_player_spell_acquisition_before_save_like_cpp<
    R: ?Sized + PlayerSpellAcquisitionRuntimeLikeCpp,
>(
    runtime: &mut R,
    prepared: &PreparedPlayerSpellAcquisitionLikeCpp,
) -> Result<(), PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp> {
    require_prepared_runtime_character_like_cpp(runtime, prepared.character_guid)?;
    apply_player_spell_acquisition_runtime_snapshot_with_fault_like_cpp(
        runtime,
        &prepared.pending_save_runtime_snapshot,
        &prepared.non_durable_skill_tombstone_ids,
        &prepared.post_commit_actions,
        |_| Ok(()),
    )
}

pub(crate) fn apply_prepared_player_spell_acquisition_with_fault_like_cpp<
    R: ?Sized + PlayerSpellAcquisitionRuntimeLikeCpp,
    F,
>(
    runtime: &mut R,
    prepared: &PreparedPlayerSpellAcquisitionLikeCpp,
    fault: F,
) -> Result<(), PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp>
where
    F: FnMut(PlayerSpellAcquisitionPublicationFaultPointLikeCpp) -> Result<(), ()>,
{
    require_prepared_runtime_character_like_cpp(runtime, prepared.character_guid)?;
    apply_player_spell_acquisition_runtime_snapshot_with_fault_like_cpp(
        runtime,
        &prepared.runtime_snapshot,
        &prepared.non_durable_skill_tombstone_ids,
        &prepared.post_commit_actions,
        fault,
    )
}

fn apply_player_spell_acquisition_runtime_snapshot_with_fault_like_cpp<
    R: ?Sized + PlayerSpellAcquisitionRuntimeLikeCpp,
    F,
>(
    runtime: &mut R,
    runtime_snapshot: &PlayerSpellAcquisitionSnapshotLikeCpp,
    new_non_durable_skill_tombstone_ids: &BTreeSet<u16>,
    post_commit_actions: &[SpellAcquisitionPostCommitActionLikeCpp],
    fault: F,
) -> Result<(), PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp>
where
    F: FnMut(PlayerSpellAcquisitionPublicationFaultPointLikeCpp) -> Result<(), ()>,
{
    apply_player_spell_acquisition_runtime_snapshot_with_before_actions_and_fault_like_cpp(
        runtime,
        runtime_snapshot,
        new_non_durable_skill_tombstone_ids,
        post_commit_actions,
        |_| {},
        fault,
    )
}

fn apply_player_spell_acquisition_runtime_snapshot_with_before_actions_and_fault_like_cpp<
    R: ?Sized + PlayerSpellAcquisitionRuntimeLikeCpp,
    Before,
    F,
>(
    runtime: &mut R,
    runtime_snapshot: &PlayerSpellAcquisitionSnapshotLikeCpp,
    new_non_durable_skill_tombstone_ids: &BTreeSet<u16>,
    post_commit_actions: &[SpellAcquisitionPostCommitActionLikeCpp],
    before_actions: Before,
    fault: F,
) -> Result<(), PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp>
where
    Before: FnOnce(&mut R),
    F: FnMut(PlayerSpellAcquisitionPublicationFaultPointLikeCpp) -> Result<(), ()>,
{
    preflight_player_spell_acquisition_runtime_owners_like_cpp(runtime, post_commit_actions)?;
    runtime.install_snapshot(runtime_snapshot, new_non_durable_skill_tombstone_ids)?;
    before_actions(runtime);
    publish_player_spell_acquisition_actions_with_fault_like_cpp(
        runtime,
        runtime_snapshot,
        post_commit_actions,
        false,
        fault,
    )
}

pub(crate) fn apply_prepared_player_spell_acquisition_actions_like_cpp<
    R: ?Sized + PlayerSpellAcquisitionRuntimeLikeCpp,
>(
    runtime: &mut R,
    prepared: &PreparedPlayerSpellAcquisitionActionsLikeCpp,
) -> Result<(), PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp> {
    require_prepared_runtime_character_like_cpp(runtime, prepared.character_guid)?;
    preflight_player_spell_acquisition_runtime_owners_like_cpp(
        runtime,
        &prepared.post_commit_actions,
    )?;
    publish_player_spell_acquisition_actions_with_fault_like_cpp(
        runtime,
        &prepared.runtime_snapshot,
        &prepared.post_commit_actions,
        prepared.runtime_actions_already_applied,
        |_| Ok(()),
    )
}

fn apply_player_spell_acquisition_non_packet_runtime_actions_like_cpp<
    R: ?Sized + PlayerSpellAcquisitionRuntimeLikeCpp,
>(
    runtime: &mut R,
    post_commit_actions: &[SpellAcquisitionPostCommitActionLikeCpp],
) -> Result<(), PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp> {
    // Apply canonical owner mutations before replacing the retained causal
    // batch. Preflight guarantees that a successful durable commit cannot be
    // followed by an unrepresented dual-wield effect.
    for action in post_commit_actions {
        if matches!(
            action,
            SpellAcquisitionPostCommitActionLikeCpp::GrantDualWield { .. }
        ) && !runtime.grant_dual_wield()
        {
            return Err(PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp::InvalidPreparedRuntime);
        }
    }
    runtime.begin_action_batch();
    for action in post_commit_actions.iter().cloned() {
        runtime.record_action(action);
    }
    Ok(())
}

fn require_prepared_runtime_character_like_cpp<R: ?Sized + PlayerSpellAcquisitionRuntimeLikeCpp>(
    runtime: &R,
    character_guid: Option<wow_core::ObjectGuid>,
) -> Result<(), PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp> {
    if character_guid.is_none() || runtime.character_guid() != character_guid {
        return Err(PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp::InvalidPreparedRuntime);
    }
    Ok(())
}

fn preflight_player_spell_acquisition_runtime_owners_like_cpp<
    R: ?Sized + PlayerSpellAcquisitionRuntimeLikeCpp,
>(
    runtime: &R,
    post_commit_actions: &[SpellAcquisitionPostCommitActionLikeCpp],
) -> Result<(), PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp> {
    if post_commit_actions.iter().any(|action| {
        matches!(
            action,
            SpellAcquisitionPostCommitActionLikeCpp::GrantDualWield { .. }
        )
    }) && !runtime.has_canonical_player()
    {
        return Err(PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp::InvalidPreparedRuntime);
    }
    Ok(())
}

fn publish_player_spell_acquisition_actions_with_fault_like_cpp<
    R: ?Sized + PlayerSpellAcquisitionRuntimeLikeCpp,
    F,
>(
    runtime: &mut R,
    runtime_snapshot: &PlayerSpellAcquisitionSnapshotLikeCpp,
    post_commit_actions: &[SpellAcquisitionPostCommitActionLikeCpp],
    runtime_actions_already_applied: bool,
    mut fault: F,
) -> Result<(), PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp>
where
    F: FnMut(PlayerSpellAcquisitionPublicationFaultPointLikeCpp) -> Result<(), ()>,
{
    if !runtime_actions_already_applied {
        runtime.begin_action_batch();
    }
    for (index, action) in post_commit_actions.iter().cloned().enumerate() {
        fault(PlayerSpellAcquisitionPublicationFaultPointLikeCpp::BeforeAction(index))
            .map_err(|()| PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp::PublicationInterrupted)?;
        if !runtime_actions_already_applied {
            runtime.record_action(action.clone());
        }

        if matches!(
            action,
            SpellAcquisitionPostCommitActionLikeCpp::GrantDualWield { .. }
        ) {
            if !runtime_actions_already_applied && !runtime.grant_dual_wield() {
                return Err(PlayerSpellAcquisitionRuntimeApplyErrorLikeCpp::InvalidPreparedRuntime);
            }
            continue;
        }

        let trait_definition_id = match &action {
            SpellAcquisitionPostCommitActionLikeCpp::LearnedSpell { spell_id, .. } => {
                runtime_snapshot
                    .spells
                    .iter()
                    .find(|spell| spell.spell_id == *spell_id)
                    .and_then(|spell| spell.trait_definition_id)
            }
            _ => None,
        };
        runtime.publish_action(&action, trait_definition_id);
    }
    Ok(())
}
