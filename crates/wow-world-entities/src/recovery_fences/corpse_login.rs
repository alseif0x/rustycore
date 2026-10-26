use wow_entities::PlayerResurrectionPhase as Phase;

use super::owner::{
    Owner, Progress, advance, clear_complete, owner_is_current, run_bones,
};
use crate::{CorpseHostLikeCpp, ResurrectionOperationCatalogs};

const AT_LOGIN_RESURRECT: u16 = 0x0100;

pub(super) async fn continue_login(
    host: &mut impl CorpseHostLikeCpp,
    owner: Owner,
    catalogs: ResurrectionOperationCatalogs<'_>,
    validated_corpse_location: Option<(u32, wow_core::Position, bool)>,
    phase: Phase,
) -> Progress {
    match phase {
        Phase::RecoveryBonesReady => {
            run_bones(host, owner, catalogs.item_guid_generator, false).await
        }
        Phase::RecoveryBonesConfirmed => {
            continue_after_bones(host, owner, validated_corpse_location)
        }
        Phase::ResurrectPlayerReady => {
            if !advance(
                host,
                owner,
                Phase::ResurrectPlayerReady,
                Phase::ResurrectPlayerInFlight,
            ) {
                return Progress::Held;
            }
            if crate::player_resurrection::resurrect_player_native(host, 0.5).is_err() {
                return Progress::Held;
            }
            if !advance(
                host,
                owner,
                Phase::ResurrectPlayerInFlight,
                Phase::VisibilityInFlight,
            ) {
                return Progress::Held;
            }
            crate::player_resurrection::finish_resurrection_visibility(host, catalogs).await;
            if !owner_is_current(host, owner) {
                return Progress::Held;
            }
            clear_at_login_and_advance(host, owner, Phase::VisibilityInFlight)
        }
        Phase::CorpseLoginGhostReady => {
            if !advance(
                host,
                owner,
                Phase::CorpseLoginGhostReady,
                Phase::CorpseLoginGhostInFlight,
            ) {
                return Progress::Held;
            }
            crate::corpse_login::restore_ghost_auras(host, catalogs).await;
            if !owner_is_current(host, owner)
                || !advance(
                    host,
                    owner,
                    Phase::CorpseLoginGhostInFlight,
                    Phase::Complete,
                )
            {
                return Progress::Held;
            }
            if clear_complete(host, owner) {
                Progress::Complete
            } else {
                Progress::Held
            }
        }
        Phase::Complete => {
            if clear_complete(host, owner) {
                Progress::Complete
            } else {
                Progress::Held
            }
        }
        _ => Progress::Held,
    }
}

fn continue_after_bones(
    host: &mut impl CorpseHostLikeCpp,
    owner: Owner,
    validated_corpse_location: Option<(u32, wow_core::Position, bool)>,
) -> Progress {
    let Some((alive, at_login_resurrect)) = host.core().with_owned_player_like_cpp(|player| {
        (
            player.unit().is_alive(),
            player.persistent_capability_state_like_cpp().at_login_flags & AT_LOGIN_RESURRECT != 0,
        )
    }) else {
        return Progress::Held;
    };
    if !alive && at_login_resurrect {
        if advance(
            host,
            owner,
            Phase::RecoveryBonesConfirmed,
            Phase::ResurrectPlayerReady,
        ) {
            return Progress::Continue;
        }
        return Progress::Held;
    }

    if !alive && !at_login_resurrect {
        if let Some((map_id, position, instanceable)) = validated_corpse_location {
            let installed = host.core().with_owned_player_mut_like_cpp(|player| {
                if player.guid() != owner.guid {
                    return false;
                }
                player
                    .resurrection_state_mut_like_cpp()
                    .corpse_location = Some((map_id, position));
                true
            });
            if installed != Some(true) {
                return Progress::Held;
            }
            if host
                .hub_mut()
                .mutate_active_player_update_state_like_cpp(|state| {
                    if instanceable {
                        state.active_local_flags &= !crate::PLAYER_LOCAL_FLAG_RELEASE_TIMER;
                    } else {
                        state.active_local_flags |= crate::PLAYER_LOCAL_FLAG_RELEASE_TIMER;
                    }
                })
                .is_none()
            {
                return Progress::Held;
            }
        }
    }
    clear_at_login_and_advance(host, owner, Phase::RecoveryBonesConfirmed)
}

fn clear_at_login_and_advance(
    host: &mut impl CorpseHostLikeCpp,
    owner: Owner,
    phase: Phase,
) -> Progress {
    if !owner_is_current(host, owner) {
        return Progress::Held;
    }
    let changed = host.core().with_owned_player_mut_like_cpp(|player| {
        if player.guid() != owner.guid {
            return false;
        }
        player.remove_at_login_flags_like_cpp(AT_LOGIN_RESURRECT);
        player.advance_resurrection_continuation(phase, Phase::CorpseLoginGhostReady)
    });
    if changed == Some(true) {
        Progress::Continue
    } else {
        Progress::Held
    }
}

