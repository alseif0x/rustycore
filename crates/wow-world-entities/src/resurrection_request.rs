// Copyright (c) 2026 alseif0x
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Complete C++ `Player::ResurrectUsingRequestData` operation and its
//! Player-owned continuation across near/far teleport and async boundaries.

use wow_constants::PowerType;
use wow_core::ObjectGuid;
use wow_entities::{
    Player, PlayerResurrectionContinuation, PlayerResurrectionOperation, PlayerResurrectionPhase,
    PlayerResurrectionRequestLikeCpp,
};

use crate::{CorpseHostLikeCpp, ResurrectionOperationCatalogs};

pub(crate) fn accept_request(
    host: &mut impl CorpseHostLikeCpp,
    resurrecter: ObjectGuid,
) -> Option<PlayerResurrectionContinuation> {
    let canonical = host
        .core()
        .with_owned_player_mut_like_cpp(|player| player.accept_resurrection_request(resurrecter))
        .flatten();
    if canonical.is_some() {
        return canonical;
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    if host.test_fixtures_ref_like_cpp().is_some()
        && host.core().player_handle_like_cpp.is_none()
        && host.represented_resurrection_requested_by_like_cpp(resurrecter)
    {
        let mut hub = host.hub_mut();
        if hub
            .fixtures
            .combat
            .represented_delayed_resurrection_after_teleport_like_cpp
            .is_some()
        {
            return None;
        }
        let request = hub
            .fixtures
            .combat
            .represented_resurrection_request_like_cpp?;
        if request.resurrecter.is_empty() || request.resurrecter != resurrecter {
            return None;
        }
        let continuation = PlayerResurrectionContinuation {
            operation: PlayerResurrectionOperation::RequestData(request),
            phase: PlayerResurrectionPhase::TeleportInFlight,
        };
        hub.fixtures
            .combat
            .represented_resurrection_request_like_cpp = None;
        hub.fixtures
            .combat
            .represented_delayed_resurrection_after_teleport_like_cpp = Some(continuation);
        return Some(continuation);
    }
    None
}

pub(crate) fn begin_teleport_revive(
    host: &mut impl CorpseHostLikeCpp,
    map_id: u32,
    position: wow_core::Position,
) -> bool {
    let continuation = PlayerResurrectionContinuation {
        operation: PlayerResurrectionOperation::TeleportRevivePercent {
            map_id,
            position,
            restore_percent: 0.5,
        },
        phase: PlayerResurrectionPhase::ResurrectPlayerReady,
    };
    let canonical = host
        .core()
        .with_owned_player_mut_like_cpp(|player| {
            let state = player.resurrection_state_mut_like_cpp();
            if state.delayed_after_teleport.is_some() {
                return false;
            }
            state.delayed_after_teleport = Some(continuation);
            true
        })
        .unwrap_or(false);
    #[cfg(any(test, feature = "test-fixtures"))]
    let canonical = {
        let fixture_available = host.test_fixtures_ref_like_cpp().is_some();
        canonical
            || host.core().player_handle_like_cpp.is_none() && fixture_available && {
                let mut hub = host.hub_mut();
                if hub
                    .fixtures
                    .combat
                    .represented_delayed_resurrection_after_teleport_like_cpp
                    .is_some()
                {
                    return false;
                }
                hub.fixtures
                    .combat
                    .represented_delayed_resurrection_after_teleport_like_cpp = Some(continuation);
                true
            }
    };
    #[cfg(not(any(test, feature = "test-fixtures")))]
    let canonical = canonical;
    canonical
}

fn continuation(host: &impl CorpseHostLikeCpp) -> Option<PlayerResurrectionContinuation> {
    let canonical = host
        .core()
        .with_owned_player_like_cpp(|player| {
            player.resurrection_state_like_cpp().delayed_after_teleport
        })
        .flatten();
    #[cfg(any(test, feature = "test-fixtures"))]
    return canonical.or_else(|| {
        (host.core().player_handle_like_cpp.is_none())
            .then(|| {
                host.test_fixtures_ref_like_cpp().and_then(|fixtures| {
                    fixtures
                        .combat
                        .represented_delayed_resurrection_after_teleport_like_cpp
                })
            })
            .flatten()
    });
    #[cfg(not(any(test, feature = "test-fixtures")))]
    canonical
}

fn owner_residence_current(
    host: &impl CorpseHostLikeCpp,
    owner: Option<crate::recovery_fences::Owner>,
) -> bool {
    match owner {
        Some(owner) => crate::recovery_fences::owner_is_current_in_residence(host, owner),
        None => host.core().player_handle_like_cpp.is_none(),
    }
}

pub fn player_resurrection_continuation_pending(host: &impl CorpseHostLikeCpp) -> bool {
    continuation(host).is_some()
}

/// A full resurrection save returned Deferred specifically because an active
/// transfer still prevents capture. Callers may retain the owner for the
/// transfer completion path; every other incomplete result remains a fence.
pub fn player_resurrection_save_waiting_for_transfer(host: &impl CorpseHostLikeCpp) -> bool {
    continuation(host).is_some_and(|continuation| {
        continuation.phase == PlayerResurrectionPhase::PlayerSaveDeferred
    }) && host
        .hub_ref()
        .player_teleport_state_snapshot_like_cpp()
        .is_some_and(|transfer| {
            transfer.post_add.is_some()
                || transfer.far_pending
                    && transfer.recovery != wow_entities::PlayerTransferRecovery::Terminal
        })
}

pub(crate) fn advance_phase(
    host: &mut impl CorpseHostLikeCpp,
    expected: PlayerResurrectionPhase,
    next: PlayerResurrectionPhase,
) -> bool {
    let canonical = host
        .core()
        .with_owned_player_mut_like_cpp(|player| {
            player.advance_resurrection_continuation(expected, next)
        })
        .unwrap_or(false);
    #[cfg(any(test, feature = "test-fixtures"))]
    let canonical = {
        let fixture_available = host.test_fixtures_ref_like_cpp().is_some();
        canonical
            || host.core().player_handle_like_cpp.is_none() && fixture_available && {
                let mut hub = host.hub_mut();
                let Some(continuation) = hub
                    .fixtures
                    .combat
                    .represented_delayed_resurrection_after_teleport_like_cpp
                    .as_mut()
                else {
                    return false;
                };
                if continuation.phase != expected {
                    return false;
                }
                continuation.phase = next;
                true
            }
    };
    #[cfg(not(any(test, feature = "test-fixtures")))]
    let canonical = canonical;
    canonical
}

fn clear_completed(host: &mut impl CorpseHostLikeCpp) -> bool {
    let canonical = host
        .core()
        .with_owned_player_mut_like_cpp(Player::clear_completed_resurrection_continuation)
        .unwrap_or(false);
    #[cfg(any(test, feature = "test-fixtures"))]
    let canonical = {
        let fixture_available = host.test_fixtures_ref_like_cpp().is_some();
        canonical
            || host.core().player_handle_like_cpp.is_none() && fixture_available && {
                let mut hub = host.hub_mut();
                if !hub
                    .fixtures
                    .combat
                    .represented_delayed_resurrection_after_teleport_like_cpp
                    .is_some_and(|continuation| {
                        continuation.phase == PlayerResurrectionPhase::Complete
                    })
                {
                    return false;
                }
                hub.fixtures
                    .combat
                    .represented_delayed_resurrection_after_teleport_like_cpp = None;
                true
            }
    };
    #[cfg(not(any(test, feature = "test-fixtures")))]
    let canonical = canonical;
    canonical
}

fn no_teleport_pending(host: &impl CorpseHostLikeCpp) -> bool {
    !host.hub_ref().near_teleport_pending_like_cpp()
        && !host.hub_ref().represented_far_teleport_pending_like_cpp()
}

/// A valid near or far ACK resolves only a continuation whose native owner has
/// an ACK-resolvable phase. A cancelled healer TeleportTo future remains held:
/// its ACK cannot prove that the awaited call returned.
pub fn mark_player_resurrection_teleport_completed(host: &mut impl CorpseHostLikeCpp) -> bool {
    let Some(continuation) = continuation(host) else {
        return true;
    };
    if matches!(
        continuation.operation,
        PlayerResurrectionOperation::Repop | PlayerResurrectionOperation::CreateCorpse
    ) {
        return false;
    }
    let recovery_operation = matches!(
        continuation.operation,
        PlayerResurrectionOperation::ReclaimCorpse { .. }
            | PlayerResurrectionOperation::SpiritHealer { .. }
            | PlayerResurrectionOperation::CorpseLogin { .. }
            | PlayerResurrectionOperation::ForcedRepop
            | PlayerResurrectionOperation::SelfRes { .. }
    );
    if recovery_operation {
        return match continuation.phase {
            PlayerResurrectionPhase::ResurrectPlayerReady
            | PlayerResurrectionPhase::RecoveryBonesReady
            | PlayerResurrectionPhase::RecoveryBonesConfirmed
            | PlayerResurrectionPhase::ReadyToSavePlayer
            | PlayerResurrectionPhase::PlayerSaveDeferred
            | PlayerResurrectionPhase::PlayerSaveApplied
            | PlayerResurrectionPhase::TeleportCompleted
            | PlayerResurrectionPhase::CorpseLoginGhostReady
            | PlayerResurrectionPhase::Complete => true,
            _ => false,
        };
    }
    match continuation.phase {
        PlayerResurrectionPhase::TeleportInFlight => advance_phase(
            host,
            PlayerResurrectionPhase::TeleportInFlight,
            PlayerResurrectionPhase::TeleportCompleted,
        ),
        PlayerResurrectionPhase::AwaitingTeleport => advance_phase(
            host,
            PlayerResurrectionPhase::AwaitingTeleport,
            PlayerResurrectionPhase::TeleportCompleted,
        ),
        PlayerResurrectionPhase::TeleportCompleted
        | PlayerResurrectionPhase::ResurrectPlayerReady
        | PlayerResurrectionPhase::ResurrectPlayerInFlight
        | PlayerResurrectionPhase::VisibilityInFlight
        | PlayerResurrectionPhase::ObtainSpellsInFlight
        | PlayerResurrectionPhase::ApplyRequestValues
        | PlayerResurrectionPhase::AuraInFlight
        | PlayerResurrectionPhase::BonesInFlight
        | PlayerResurrectionPhase::ReadyToSavePlayer
        | PlayerResurrectionPhase::SavingPlayer
        | PlayerResurrectionPhase::PlayerSaveDeferred
        | PlayerResurrectionPhase::PlayerSaveApplied
        | PlayerResurrectionPhase::RecoveryBonesReady
        | PlayerResurrectionPhase::RecoveryCorpseBonesInFlight
        | PlayerResurrectionPhase::RecoveryBonesConfirmed
        | PlayerResurrectionPhase::SpiritHealerDurabilityInFlight
        | PlayerResurrectionPhase::CorpseLoginGhostReady
        | PlayerResurrectionPhase::CorpseLoginGhostInFlight
        | PlayerResurrectionPhase::ReleaseWispAuraInFlight
        | PlayerResurrectionPhase::ReleaseGhostAuraInFlight
        | PlayerResurrectionPhase::CorpseBonesInFlight
        | PlayerResurrectionPhase::CorpseSaveInFlight
        | PlayerResurrectionPhase::CorpsePublished
        | PlayerResurrectionPhase::Complete => true,
    }
}

fn set_request_values(
    host: &mut impl CorpseHostLikeCpp,
    request: PlayerResurrectionRequestLikeCpp,
) -> bool {
    let updated = host.core().with_owned_player_mut_like_cpp(|player| {
        player.unit_mut().set_health(u64::from(request.health));
        player
            .unit_mut()
            .set_power(PowerType::Mana, request.mana as i32);
        player.unit_mut().set_power(PowerType::Rage, 0);
        for power in [PowerType::Energy, PowerType::Focus] {
            let full_power = player.get_max_power(power).max(0);
            player.unit_mut().set_power(power, full_power);
        }
    });
    let Some(()) = updated else {
        return false;
    };
    host.sync_player_registry_state_like_cpp();
    true
}

/// Continue only after a valid immediate, near-ACK or far-worldport completion.
/// An in-flight phase is intentionally retained and never replayed.
pub(crate) async fn process_after_teleport(
    host: &mut impl CorpseHostLikeCpp,
    catalogs: ResurrectionOperationCatalogs<'_>,
) -> bool {
    let Some(mut continuation) = continuation(host) else {
        return true;
    };
    if matches!(
        continuation.operation,
        PlayerResurrectionOperation::ReclaimCorpse { .. }
            | PlayerResurrectionOperation::SpiritHealer { .. }
            | PlayerResurrectionOperation::CorpseLogin { .. }
            | PlayerResurrectionOperation::ForcedRepop
            | PlayerResurrectionOperation::SelfRes { .. }
    ) {
        if crate::recovery_fences::resume_state(host)
            != crate::recovery_fences::RecoveryResumeState::Ready
        {
            return false;
        }
        return matches!(
            crate::recovery_fences::continue_operation(host, catalogs,).await,
            crate::recovery_fences::Progress::Complete
        );
    }
    if matches!(
        continuation.operation,
        PlayerResurrectionOperation::TeleportRevivePercent { .. }
    ) {
        return run_teleport_revive(host, catalogs).await;
    }
    let PlayerResurrectionOperation::RequestData(request) = continuation.operation else {
        return false;
    };
    let owner = crate::recovery_fences::owner(host);
    if !owner_residence_current(host, owner) {
        return false;
    }
    if !no_teleport_pending(host) {
        return false;
    }
    if matches!(
        continuation.phase,
        PlayerResurrectionPhase::VisibilityInFlight
            | PlayerResurrectionPhase::ObtainSpellsInFlight
            | PlayerResurrectionPhase::ResurrectPlayerInFlight
            | PlayerResurrectionPhase::AuraInFlight
            | PlayerResurrectionPhase::BonesInFlight
            | PlayerResurrectionPhase::SavingPlayer
            | PlayerResurrectionPhase::ReleaseWispAuraInFlight
            | PlayerResurrectionPhase::ReleaseGhostAuraInFlight
            | PlayerResurrectionPhase::CorpseBonesInFlight
            | PlayerResurrectionPhase::CorpseSaveInFlight
            | PlayerResurrectionPhase::CorpsePublished
    ) {
        // These effects may have reached their external boundary before this
        // future was cancelled. Keep the continuation; never replay them blindly.
        return false;
    }
    if continuation.phase == PlayerResurrectionPhase::Complete {
        return clear_completed(host);
    }
    if continuation.phase == PlayerResurrectionPhase::PlayerSaveApplied {
        return advance_phase(
            host,
            PlayerResurrectionPhase::PlayerSaveApplied,
            PlayerResurrectionPhase::Complete,
        ) && clear_completed(host);
    }
    if continuation.phase == PlayerResurrectionPhase::TeleportInFlight
        || continuation.phase == PlayerResurrectionPhase::AwaitingTeleport
    {
        // Cancellation before TeleportTo returned is not proof that C++ reached
        // its post-call IsBeingTeleported check or a valid ACK. Coordinates
        // cannot release either phase.
        return false;
    } else if continuation.phase == PlayerResurrectionPhase::TeleportCompleted {
        if !advance_phase(
            host,
            PlayerResurrectionPhase::TeleportCompleted,
            PlayerResurrectionPhase::ResurrectPlayerReady,
        ) {
            return false;
        }
        continuation.phase = PlayerResurrectionPhase::ResurrectPlayerReady;
    }

    if continuation.phase == PlayerResurrectionPhase::ResurrectPlayerReady {
        if !advance_phase(
            host,
            PlayerResurrectionPhase::ResurrectPlayerReady,
            PlayerResurrectionPhase::ResurrectPlayerInFlight,
        ) {
            return false;
        }
        continuation.phase = PlayerResurrectionPhase::ResurrectPlayerInFlight;
        if let Err(reason) = super::player_resurrection::resurrect_player_native(host, 0.0) {
            tracing::warn!(
                account = host.core().account_id,
                reason,
                "Resurrection request native phase did not complete"
            );
            return false;
        }
        if !advance_phase(
            host,
            PlayerResurrectionPhase::ResurrectPlayerInFlight,
            PlayerResurrectionPhase::VisibilityInFlight,
        ) {
            return false;
        }
        continuation.phase = PlayerResurrectionPhase::VisibilityInFlight;
    }
    if continuation.phase == PlayerResurrectionPhase::VisibilityInFlight {
        super::player_resurrection::finish_resurrection_visibility(host, catalogs).await;
        if !owner_residence_current(host, owner)
            || !advance_phase(
                host,
                PlayerResurrectionPhase::VisibilityInFlight,
                PlayerResurrectionPhase::ObtainSpellsInFlight,
            )
        {
            return false;
        }
        continuation.phase = PlayerResurrectionPhase::ObtainSpellsInFlight;
    }
    if continuation.phase == PlayerResurrectionPhase::ObtainSpellsInFlight {
        if let Err(reason) = host.cast_all_obtain_spells_like_cpp(catalogs).await {
            tracing::warn!(reason, "Request resurrection obtain-spell phase failed");
            return false;
        }
        if !owner_residence_current(host, owner)
            || !advance_phase(
                host,
                PlayerResurrectionPhase::ObtainSpellsInFlight,
                PlayerResurrectionPhase::ApplyRequestValues,
            )
        {
            return false;
        }
        continuation.phase = PlayerResurrectionPhase::ApplyRequestValues;
    }
    if continuation.phase == PlayerResurrectionPhase::ApplyRequestValues {
        if !set_request_values(host, request) {
            return false;
        }
        let next = if request.aura == 0 {
            PlayerResurrectionPhase::BonesInFlight
        } else {
            PlayerResurrectionPhase::AuraInFlight
        };
        if !advance_phase(host, PlayerResurrectionPhase::ApplyRequestValues, next) {
            return false;
        }
        continuation.phase = next;
    }
    if continuation.phase == PlayerResurrectionPhase::AuraInFlight {
        if let Ok(spell_id) = i32::try_from(request.aura) {
            let _ = host
                .cast_triggered_self_spell_like_cpp(
                    catalogs,
                    spell_id,
                    None,
                    Some(request.resurrecter),
                )
                .await;
        }
        if !advance_phase(
            host,
            PlayerResurrectionPhase::AuraInFlight,
            PlayerResurrectionPhase::BonesInFlight,
        ) {
            return false;
        }
        continuation.phase = PlayerResurrectionPhase::BonesInFlight;
    }
    if continuation.phase == PlayerResurrectionPhase::BonesInFlight {
        match crate::player_death::convert_corpse_to_bones_for_resurrection(host).await {
            crate::player_death::CorpseBonesConversion::Unavailable => return false,
            crate::player_death::CorpseBonesConversion::NoCorpse => {
                if !advance_phase(
                    host,
                    PlayerResurrectionPhase::BonesInFlight,
                    PlayerResurrectionPhase::Complete,
                ) {
                    return false;
                }
                return clear_completed(host);
            }
            crate::player_death::CorpseBonesConversion::Converted => {
                return run_resurrection_save(
                    host,
                    catalogs.item_guid_generator,
                    PlayerResurrectionPhase::BonesInFlight,
                )
                .await;
            }
        }
    }
    if continuation.phase == PlayerResurrectionPhase::ReadyToSavePlayer
        || continuation.phase == PlayerResurrectionPhase::PlayerSaveDeferred
    {
        return run_resurrection_save(host, catalogs.item_guid_generator, continuation.phase).await;
    }
    clear_completed(host)
}

async fn run_resurrection_save(
    host: &mut impl CorpseHostLikeCpp,
    item_guid_generator: &wow_core::ObjectGuidGenerator,
    expected: PlayerResurrectionPhase,
) -> bool {
    if host.player_logout_with_save_like_cpp() {
        if !advance_phase(host, expected, PlayerResurrectionPhase::Complete) {
            return false;
        }
        return clear_completed(host);
    }
    if expected == PlayerResurrectionPhase::BonesInFlight
        && !advance_phase(
            host,
            PlayerResurrectionPhase::BonesInFlight,
            PlayerResurrectionPhase::ReadyToSavePlayer,
        )
    {
        return false;
    } else if expected != PlayerResurrectionPhase::BonesInFlight
        && expected != PlayerResurrectionPhase::ReadyToSavePlayer
        && expected != PlayerResurrectionPhase::PlayerSaveDeferred
    {
        return false;
    }
    match host
        .save_player_after_corpse_bones_like_cpp(item_guid_generator)
        .await
    {
        crate::ResurrectionSaveOutcome::Applied => {
            if !advance_phase(
                host,
                PlayerResurrectionPhase::PlayerSaveApplied,
                PlayerResurrectionPhase::Complete,
            ) {
                return false;
            }
            clear_completed(host)
        }
        crate::ResurrectionSaveOutcome::Deferred => {
            if continuation(host).is_some_and(|continuation| {
                continuation.phase == PlayerResurrectionPhase::SavingPlayer
            }) {
                let _ = advance_phase(
                    host,
                    PlayerResurrectionPhase::SavingPlayer,
                    PlayerResurrectionPhase::PlayerSaveDeferred,
                );
            }
            false
        }
        crate::ResurrectionSaveOutcome::Unresolved => false,
    }
}

pub async fn continue_player_resurrection_after_teleport(
    host: &mut impl CorpseHostLikeCpp,
    catalogs: ResurrectionOperationCatalogs<'_>,
) -> bool {
    process_after_teleport(host, catalogs).await
}

pub(crate) async fn run_teleport_revive(
    host: &mut impl CorpseHostLikeCpp,
    catalogs: ResurrectionOperationCatalogs<'_>,
) -> bool {
    let Some(continuation) = continuation(host) else {
        return false;
    };
    let owner = crate::recovery_fences::owner(host);
    if !owner_residence_current(host, owner) {
        return false;
    }
    let PlayerResurrectionOperation::TeleportRevivePercent {
        map_id: _,
        position: _,
        restore_percent,
    } = continuation.operation
    else {
        return false;
    };
    if continuation.phase != PlayerResurrectionPhase::ResurrectPlayerReady {
        return false;
    }
    if !advance_phase(
        host,
        PlayerResurrectionPhase::ResurrectPlayerReady,
        PlayerResurrectionPhase::ResurrectPlayerInFlight,
    ) {
        return false;
    }
    if let Err(reason) = super::player_resurrection::resurrect_player_native(host, restore_percent)
    {
        tracing::warn!(
            account = host.core().account_id,
            reason,
            "Teleport resurrection native phase did not complete"
        );
        return false;
    }
    if !advance_phase(
        host,
        PlayerResurrectionPhase::ResurrectPlayerInFlight,
        PlayerResurrectionPhase::VisibilityInFlight,
    ) {
        return false;
    }
    super::player_resurrection::finish_resurrection_visibility(host, catalogs).await;
    if !owner_residence_current(host, owner)
        || !advance_phase(
            host,
            PlayerResurrectionPhase::VisibilityInFlight,
            PlayerResurrectionPhase::ObtainSpellsInFlight,
        )
    {
        return false;
    }
    if let Err(reason) = host.cast_all_obtain_spells_like_cpp(catalogs).await {
        tracing::warn!(reason, "Teleport resurrection obtain-spell phase failed");
        return false;
    }
    if !owner_residence_current(host, owner)
        || !advance_phase(
            host,
            PlayerResurrectionPhase::ObtainSpellsInFlight,
            PlayerResurrectionPhase::Complete,
        )
    {
        return false;
    }
    clear_completed(host)
}

pub async fn revive_player_for_teleport<H: CorpseHostLikeCpp>(
    host: &mut H,
    catalogs: ResurrectionOperationCatalogs<'_>,
    map_id: u32,
    destination: wow_core::Position,
) -> bool {
    begin_teleport_revive(host, map_id, destination) && run_teleport_revive(host, catalogs).await
}
