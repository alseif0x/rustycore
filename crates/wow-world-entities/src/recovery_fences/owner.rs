use wow_core::ObjectGuid;
use wow_entities::{
    PlayerResurrectionContinuation, PlayerResurrectionOperation, PlayerResurrectionPhase,
};

use crate::{CorpseHostLikeCpp, ResurrectionOperationCatalogs};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecoveryResumeState {
    None,
    Ready,
    Waiting,
    Held,
}

#[derive(Clone, Copy)]
pub(crate) struct Owner {
    pub(crate) handle: wow_map::PlayerHandle,
    pub(crate) guid: ObjectGuid,
    pub(crate) residence: Option<(wow_map::MapKey, u64)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Progress {
    Continue,
    Complete,
    Waiting,
    Held,
}

fn active_residence(
    host: &impl CorpseHostLikeCpp,
    handle: wow_map::PlayerHandle,
) -> Option<(wow_map::MapKey, u64)> {
    let manager = host.core().canonical_map_manager.as_ref()?.lock().ok()?;
    manager.player_active_residence_revision_like_cpp(handle)
}

pub(crate) fn owner(host: &impl CorpseHostLikeCpp) -> Option<Owner> {
    let handle = host.core().player_handle_like_cpp?;
    let guid = host.player_guid()?;
    if guid != handle.guid() {
        return None;
    }
    let residence = active_residence(host, handle);
    Some(Owner {
        handle,
        guid,
        residence,
    })
}

pub(crate) fn owner_is_current(host: &impl CorpseHostLikeCpp, owner: Owner) -> bool {
    host.core().player_handle_like_cpp == Some(owner.handle)
        && host.player_guid() == Some(owner.guid)
        && owner.guid == owner.handle.guid()
        && host
            .core()
            .with_owned_player_like_cpp(|player| player.guid() == owner.guid)
            .unwrap_or(false)
}

pub(crate) fn owner_is_current_in_residence(host: &impl CorpseHostLikeCpp, owner: Owner) -> bool {
    if !owner_is_current(host, owner) {
        return false;
    }
    let Some(expected_residence) = owner.residence else {
        return false;
    };
    active_residence(host, owner.handle) == Some(expected_residence)
}

fn current_for(
    host: &impl CorpseHostLikeCpp,
    owner: Owner,
) -> Option<PlayerResurrectionContinuation> {
    if !owner_is_current(host, owner) {
        return None;
    }
    host.core()
        .with_owned_player_like_cpp(|player| {
            (player.guid() == owner.guid)
                .then(|| player.resurrection_state_like_cpp().delayed_after_teleport)
                .flatten()
        })
        .flatten()
}

pub(crate) fn begin(
    host: &mut impl CorpseHostLikeCpp,
    operation: PlayerResurrectionOperation,
    phase: PlayerResurrectionPhase,
) -> bool {
    let Some(owner) = owner(host) else {
        return false;
    };
    if host.core().player_handle_like_cpp != Some(owner.handle)
        || host.player_guid() != Some(owner.guid)
    {
        return false;
    }
    host.core()
        .with_owned_player_mut_like_cpp(|player| {
            player.guid() == owner.guid && player.begin_resurrection_operation(operation, phase)
        })
        .unwrap_or(false)
}

pub(super) fn advance(
    host: &mut impl CorpseHostLikeCpp,
    owner: Owner,
    expected: PlayerResurrectionPhase,
    next: PlayerResurrectionPhase,
) -> bool {
    if !owner_is_current(host, owner) {
        return false;
    }
    host.core()
        .with_owned_player_mut_like_cpp(|player| {
            player.guid() == owner.guid && player.advance_resurrection_continuation(expected, next)
        })
        .unwrap_or(false)
}

pub(super) fn update(
    host: &mut impl CorpseHostLikeCpp,
    owner: Owner,
    expected: PlayerResurrectionPhase,
    operation: PlayerResurrectionOperation,
    next: PlayerResurrectionPhase,
) -> bool {
    if !owner_is_current(host, owner) {
        return false;
    }
    host.core()
        .with_owned_player_mut_like_cpp(|player| {
            player.guid() == owner.guid
                && player.update_resurrection_operation(expected, operation, next)
        })
        .unwrap_or(false)
}

pub(super) fn clear_complete(host: &mut impl CorpseHostLikeCpp, owner: Owner) -> bool {
    if !owner_is_current(host, owner) {
        return false;
    }
    host.core()
        .with_owned_player_mut_like_cpp(|player| {
            player.guid() == owner.guid && player.clear_completed_resurrection_continuation()
        })
        .unwrap_or(false)
}

pub(super) fn complete(
    host: &mut impl CorpseHostLikeCpp,
    owner: Owner,
    expected: PlayerResurrectionPhase,
) -> Progress {
    if !advance(host, owner, expected, PlayerResurrectionPhase::Complete) {
        return Progress::Held;
    }
    if clear_complete(host, owner) {
        Progress::Complete
    } else {
        Progress::Held
    }
}

fn transfer_pending(host: &impl CorpseHostLikeCpp) -> bool {
    host.hub_ref()
        .player_teleport_state_snapshot_like_cpp()
        .is_some_and(|state| {
            state.post_add.is_some()
                || state.far_pending
                    && state.recovery != wow_entities::PlayerTransferRecovery::Terminal
        })
}

pub fn resume_state(host: &impl CorpseHostLikeCpp) -> RecoveryResumeState {
    let Some(owner) = owner(host) else {
        return if host.core().player_handle_like_cpp.is_some() || host.player_guid().is_some() {
            RecoveryResumeState::Held
        } else {
            RecoveryResumeState::None
        };
    };
    if !owner_is_current(host, owner) {
        return RecoveryResumeState::Held;
    }
    let continuation = host.core().with_owned_player_like_cpp(|player| {
        (player.guid() == owner.guid)
            .then(|| player.resurrection_state_like_cpp().delayed_after_teleport)
            .flatten()
    });
    let Some(continuation) = continuation else {
        return RecoveryResumeState::Held;
    };
    let Some(continuation) = continuation else {
        return RecoveryResumeState::None;
    };
    let recovery_operation = matches!(
        continuation.operation,
        PlayerResurrectionOperation::ReclaimCorpse { .. }
            | PlayerResurrectionOperation::SpiritHealer { .. }
            | PlayerResurrectionOperation::CorpseLogin { .. }
            | PlayerResurrectionOperation::ForcedRepop
            | PlayerResurrectionOperation::SelfRes { .. }
    );
    let request_save_recovery = matches!(
        continuation.operation,
        PlayerResurrectionOperation::RequestData(_)
    ) && matches!(
        continuation.phase,
        PlayerResurrectionPhase::ReadyToSavePlayer
            | PlayerResurrectionPhase::PlayerSaveDeferred
            | PlayerResurrectionPhase::PlayerSaveApplied
            | PlayerResurrectionPhase::Complete
    );
    if !recovery_operation && !request_save_recovery {
        return RecoveryResumeState::None;
    }
    if matches!(
        continuation.operation,
        PlayerResurrectionOperation::SpiritHealer { .. }
    ) && continuation.phase == PlayerResurrectionPhase::TeleportInFlight
    {
        return if transfer_pending(host) {
            RecoveryResumeState::Waiting
        } else {
            RecoveryResumeState::Held
        };
    }
    if matches!(
        continuation.operation,
        PlayerResurrectionOperation::SpiritHealer { .. }
    ) && continuation.phase == PlayerResurrectionPhase::TeleportCompleted
    {
        return RecoveryResumeState::Ready;
    }
    match continuation.phase {
        PlayerResurrectionPhase::ResurrectPlayerReady
        | PlayerResurrectionPhase::RecoveryBonesReady
        | PlayerResurrectionPhase::RecoveryBonesConfirmed
        | PlayerResurrectionPhase::PlayerSaveApplied
        | PlayerResurrectionPhase::CorpseLoginGhostReady
        | PlayerResurrectionPhase::Complete => RecoveryResumeState::Ready,
        PlayerResurrectionPhase::ReadyToSavePlayer
        | PlayerResurrectionPhase::PlayerSaveDeferred
            if transfer_pending(host) =>
        {
            RecoveryResumeState::Waiting
        }
        PlayerResurrectionPhase::ReadyToSavePlayer
        | PlayerResurrectionPhase::PlayerSaveDeferred => RecoveryResumeState::Ready,
        PlayerResurrectionPhase::AwaitingTeleport => RecoveryResumeState::Waiting,
        _ => RecoveryResumeState::Held,
    }
}

pub(crate) async fn continue_operation(
    host: &mut impl CorpseHostLikeCpp,
    catalogs: ResurrectionOperationCatalogs<'_>,
) -> Progress {
    let Some(owner) = owner(host) else {
        return Progress::Held;
    };
    if !owner_is_current(host, owner) {
        return Progress::Held;
    }
    let mut visited = Vec::<PlayerResurrectionContinuation>::new();
    loop {
        let Some(continuation) = current_for(host, owner) else {
            return Progress::Held;
        };
        if visited.contains(&continuation) {
            return Progress::Held;
        }
        visited.push(continuation);
        let result = match continuation.operation {
            PlayerResurrectionOperation::ReclaimCorpse { restore_percent } => {
                super::reclaim::continue_reclaim(
                    host,
                    owner,
                    catalogs,
                    restore_percent,
                    continuation.phase,
                )
                .await
            }
            PlayerResurrectionOperation::SpiritHealer { corpse_graveyard } => {
                super::spirit_healer::continue_spirit_healer(
                    host,
                    owner,
                    catalogs,
                    catalogs.graveyards,
                    corpse_graveyard,
                    continuation.phase,
                )
                .await
            }
            PlayerResurrectionOperation::CorpseLogin {
                validated_corpse_location,
            } => {
                super::corpse_login::continue_login(
                    host,
                    owner,
                    catalogs,
                    validated_corpse_location,
                    continuation.phase,
                )
                .await
            }
            PlayerResurrectionOperation::ForcedRepop => {
                super::forced_repop::continue_forced_repop(
                    host,
                    owner,
                    catalogs,
                    catalogs.graveyards,
                    continuation.phase,
                )
                .await
            }
            PlayerResurrectionOperation::SelfRes { health, mana } => {
                super::self_resurrection::continue_self_resurrection(
                    host,
                    owner,
                    catalogs,
                    health,
                    mana,
                    continuation.phase,
                )
                .await
            }
            _ => return Progress::Held,
        };
        if result != Progress::Continue {
            return result;
        }
    }
}

pub(super) async fn run_bones(
    host: &mut impl CorpseHostLikeCpp,
    owner: Owner,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    trigger_save: bool,
) -> Progress {
    if !advance(
        host,
        owner,
        PlayerResurrectionPhase::RecoveryBonesReady,
        PlayerResurrectionPhase::RecoveryCorpseBonesInFlight,
    ) {
        return Progress::Held;
    }
    match crate::player_death::convert_corpse_to_bones_for_resurrection(host).await {
        crate::player_death::CorpseBonesConversion::Unavailable => Progress::Held,
        crate::player_death::CorpseBonesConversion::NoCorpse => {
            if owner_is_current(host, owner)
                && advance(
                    host,
                    owner,
                    PlayerResurrectionPhase::RecoveryCorpseBonesInFlight,
                    PlayerResurrectionPhase::RecoveryBonesConfirmed,
                )
            {
                Progress::Continue
            } else {
                Progress::Held
            }
        }
        crate::player_death::CorpseBonesConversion::Converted => {
            if !owner_is_current(host, owner) {
                return Progress::Held;
            }
            if trigger_save && !host.player_logout_with_save_like_cpp() {
                if !advance(
                    host,
                    owner,
                    PlayerResurrectionPhase::RecoveryCorpseBonesInFlight,
                    PlayerResurrectionPhase::ReadyToSavePlayer,
                ) {
                    return Progress::Held;
                }
                save_player(host, owner, item_guid_generator).await
            } else if advance(
                host,
                owner,
                PlayerResurrectionPhase::RecoveryCorpseBonesInFlight,
                PlayerResurrectionPhase::RecoveryBonesConfirmed,
            ) {
                Progress::Continue
            } else {
                Progress::Held
            }
        }
    }
}

pub(super) async fn save_player(
    host: &mut impl CorpseHostLikeCpp,
    owner: Owner,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
) -> Progress {
    let Some(current) = current_for(host, owner) else {
        return Progress::Held;
    };
    if !matches!(
        current.phase,
        PlayerResurrectionPhase::ReadyToSavePlayer | PlayerResurrectionPhase::PlayerSaveDeferred
    ) {
        return if current.phase == PlayerResurrectionPhase::PlayerSaveApplied {
            Progress::Continue
        } else {
            Progress::Held
        };
    }
    let outcome = host
        .save_player_after_corpse_bones_like_cpp(item_guid_generator)
        .await;
    if !owner_is_current(host, owner) {
        return Progress::Held;
    }
    match outcome {
        crate::ResurrectionSaveOutcome::Applied => {
            if current_for(host, owner)
                .is_some_and(|current| current.phase == PlayerResurrectionPhase::PlayerSaveApplied)
            {
                Progress::Continue
            } else {
                Progress::Held
            }
        }
        crate::ResurrectionSaveOutcome::Deferred => {
            if current_for(host, owner).is_some_and(|current| {
                current.phase == PlayerResurrectionPhase::PlayerSaveDeferred
                    && transfer_pending(host)
            }) {
                Progress::Waiting
            } else {
                Progress::Held
            }
        }
        crate::ResurrectionSaveOutcome::Unresolved => Progress::Held,
    }
}
