use wow_entities::PlayerResurrectionPhase as Phase;
use wow_packet::packets::misc::DeathReleaseLoc;

use super::owner::{Owner, Progress, clear_complete, complete, run_bones, save_player};
use crate::{CorpseHostLikeCpp, ResurrectionOperationCatalogs};

pub(super) async fn continue_forced_repop(
    host: &mut impl CorpseHostLikeCpp,
    owner: Owner,
    catalogs: ResurrectionOperationCatalogs<'_>,
    graveyards: Option<&wow_data::GraveyardStore>,
    phase: Phase,
) -> Progress {
    match phase {
        Phase::RecoveryBonesReady => {
            run_bones(host, owner, catalogs.item_guid_generator, true).await
        }
        Phase::ReadyToSavePlayer | Phase::PlayerSaveDeferred => {
            save_player(host, owner, catalogs.item_guid_generator).await
        }
        Phase::RecoveryBonesConfirmed | Phase::PlayerSaveApplied => {
            finish_forced_repop(host, owner, catalogs, graveyards, phase).await
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

async fn finish_forced_repop(
    host: &mut impl CorpseHostLikeCpp,
    owner: Owner,
    catalogs: ResurrectionOperationCatalogs<'_>,
    graveyards: Option<&wow_data::GraveyardStore>,
    phase: Phase,
) -> Progress {
    let Some(graveyards) = graveyards else {
        return Progress::Held;
    };
    let Some((map_id, position)) = host
        .core()
        .with_owned_player_like_cpp(|player| {
            (player.guid() == owner.guid).then(|| {
                (player.unit().world().map_id(), player.unit().world().position())
            })
        })
        .flatten()
    else {
        return Progress::Held;
    };
    let below_min_height = position.z < host.hub_ref().player_min_height_like_cpp(position);
    let player_is_dead = host.hub_ref().resolved_player_is_alive_like_cpp() == Some(false);
    let closest = crate::closest_graveyard_like_cpp(host, graveyards, map_id, position);
    crate::player_death::stop_repop_countdown_like_cpp(host);
    let grave = closest.and_then(|safe_loc_id| {
        host.hub_ref()
            .catalogs
            .world_safe_loc_store_like_cpp
            .as_ref()
            .and_then(|store| store.get(safe_loc_id))
            .map(|entry| (entry.map_id, entry.position))
    });
    let homebind = if grave.is_none() && below_min_height {
        host.core()
            .with_owned_player_like_cpp(|player| {
                (player.guid() == owner.guid)
                    .then_some(player.gameplay_state().homebind)
                    .flatten()
            })
            .flatten()
    } else {
        None
    };

    // Deletion and the required save are confirmed before this handoff. The
    // existing TeleportTo owner retains any near/far revive continuation.
    if complete(host, owner, phase) != Progress::Complete {
        return Progress::Held;
    }

    if let Some((grave_map_id, grave_position)) = grave {
        let far_revive = player_is_dead && grave_map_id != map_id;
        if far_revive
            && !crate::resurrection_request::begin_teleport_revive(
                host,
                grave_map_id,
                grave_position,
            )
        {
            return Progress::Held;
        }
        host.teleport_to_with_options_like_cpp(
            catalogs,
            grave_map_id,
            grave_position,
            crate::TELE_REVIVE_AT_TELEPORT_LIKE_CPP,
        )
        .await;
        if far_revive {
            let transfer_pending = host
                .hub_ref()
                .player_teleport_state_snapshot_like_cpp()
                .is_some_and(|state| state.far_pending || state.post_add.is_some())
                || host.pending_teleport_like_cpp().is_some();
            if !transfer_pending {
                // TeleportTo returned without admitting a far transfer. Its
                // unstarted revive phase has no native effect to preserve.
                if complete(host, owner, Phase::ResurrectPlayerReady) != Progress::Complete {
                    return Progress::Held;
                }
            }
        }
        if host.hub_ref().resolved_player_is_alive_like_cpp() == Some(false) {
            host.core().send_packet_realm(&DeathReleaseLoc {
                map_id: grave_map_id as i32,
                x: grave_position.x,
                y: grave_position.y,
                z: grave_position.z,
            });
        }
    } else if let Some(homebind) = homebind {
        host.teleport_to_with_options_like_cpp(
            catalogs,
            homebind.map_id,
            homebind.position,
            crate::TELE_TO_NONE_LIKE_CPP,
        )
        .await;
    }
    let _ = host.core().with_owned_player_mut_like_cpp(|player| {
        if player.guid() == owner.guid {
            player.remove_player_flag(crate::player_resurrection::PLAYER_FLAGS_IS_OUT_OF_BOUNDS);
        }
    });
    Progress::Complete
}
