use wow_entities::{PlayerResurrectionOperation, PlayerResurrectionPhase as Phase};

use super::owner::{
    Owner, Progress, advance, clear_complete, complete, owner_is_current, run_bones, save_player,
    update,
};
use crate::{
    CorpseHostLikeCpp, ResurrectionOperationCatalogs, closest_graveyard_like_cpp,
    player_corpse_location_like_cpp,
};

pub(super) async fn continue_spirit_healer(
    host: &mut impl CorpseHostLikeCpp,
    owner: Owner,
    catalogs: ResurrectionOperationCatalogs<'_>,
    graveyards: Option<&wow_data::GraveyardStore>,
    corpse_graveyard: Option<u32>,
    phase: Phase,
) -> Progress {
    match phase {
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
            if !owner_is_current(host, owner)
                || !super::owner::owner_is_current_in_residence(host, owner)
                || !advance(host, owner, Phase::VisibilityInFlight, Phase::ObtainSpellsInFlight)
            {
                return Progress::Held;
            }
            if let Err(reason) = host.cast_all_obtain_spells_like_cpp(catalogs).await {
                tracing::warn!(reason, "Spirit-healer resurrection obtain-spell phase failed");
                return Progress::Held;
            }
            if !owner_is_current(host, owner)
                || !super::owner::owner_is_current_in_residence(host, owner)
                || !advance(host, owner, Phase::ObtainSpellsInFlight, Phase::AuraInFlight)
            {
                return Progress::Held;
            }
            crate::player_resurrection::apply_resurrection_sickness(host, catalogs).await;
            if !advance(
                host,
                owner,
                Phase::AuraInFlight,
                Phase::SpiritHealerDurabilityInFlight,
            ) {
                return Progress::Held;
            }
            host.apply_spirit_healer_durability_loss();
            let Some(graveyards) = graveyards else {
                return Progress::Held;
            };
            let corpse_graveyard = player_corpse_location_like_cpp(host)
                .and_then(|(map_id, position)| closest_graveyard_like_cpp(host, graveyards, map_id, position));
            if !update(
                host,
                owner,
                Phase::SpiritHealerDurabilityInFlight,
                PlayerResurrectionOperation::SpiritHealer { corpse_graveyard },
                Phase::RecoveryBonesReady,
            ) {
                return Progress::Held;
            }
            Progress::Continue
        }
        Phase::RecoveryBonesReady => {
            run_bones(host, owner, catalogs.item_guid_generator, true).await
        }
        Phase::ReadyToSavePlayer | Phase::PlayerSaveDeferred => {
            save_player(host, owner, catalogs.item_guid_generator).await
        }
        Phase::RecoveryBonesConfirmed | Phase::PlayerSaveApplied => {
            finish_spirit_healer(host, owner, graveyards, corpse_graveyard, phase).await
        }
        Phase::TeleportCompleted => complete(host, owner, Phase::TeleportCompleted),
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

async fn finish_spirit_healer(
    host: &mut impl CorpseHostLikeCpp,
    owner: Owner,
    graveyards: Option<&wow_data::GraveyardStore>,
    corpse_graveyard: Option<u32>,
    phase: Phase,
) -> Progress {
    let Some(corpse_graveyard) = corpse_graveyard else {
        return complete(host, owner, phase);
    };
    let Some(graveyards) = graveyards else {
        return Progress::Held;
    };
    let ghost_graveyard = host.hub_ref().player_position_like_cpp().and_then(|position| {
        let map_id = u32::from(host.core().player_map_id_like_cpp());
        closest_graveyard_like_cpp(host, graveyards, map_id, position)
    });
    if ghost_graveyard == Some(corpse_graveyard) {
        return complete(host, owner, phase);
    }
    let Some((map_id, position)) = host
        .hub_ref()
        .catalogs
        .world_safe_loc_store_like_cpp
        .as_ref()
        .and_then(|store| store.get(corpse_graveyard))
        .map(|entry| (entry.map_id, entry.position))
    else {
        return complete(host, owner, phase);
    };
    if !advance(host, owner, phase, Phase::TeleportInFlight) {
        return Progress::Held;
    }
    host.teleport_to(map_id, position).await;
    if !owner_is_current(host, owner) {
        return Progress::Held;
    }
    if advance(host, owner, Phase::TeleportInFlight, Phase::Complete)
        && clear_complete(host, owner)
    {
        Progress::Complete
    } else {
        Progress::Held
    }
}
