use wow_constants::PowerType;
use wow_entities::PlayerResurrectionPhase as Phase;

use super::owner::{
    Owner, Progress, advance, clear_complete, complete, owner_is_current, run_bones, save_player,
};
use crate::{CorpseHostLikeCpp, ResurrectionOperationCatalogs};

pub(super) async fn continue_self_resurrection(
    host: &mut impl CorpseHostLikeCpp,
    owner: Owner,
    catalogs: ResurrectionOperationCatalogs<'_>,
    health: u32,
    mana: u32,
    phase: Phase,
) -> Progress {
    match phase {
        Phase::ResurrectPlayerReady => {
            if !super::owner::owner_is_current_in_residence(host, owner)
                || !advance(
                    host,
                    owner,
                    Phase::ResurrectPlayerReady,
                    Phase::ResurrectPlayerInFlight,
                )
            {
                return Progress::Held;
            }
            if crate::player_resurrection::resurrect_player_native(host, 0.0).is_err() {
                return Progress::Held;
            }
            if !owner_is_current(host, owner)
                || !advance(
                    host,
                    owner,
                    Phase::ResurrectPlayerInFlight,
                    Phase::VisibilityInFlight,
                )
            {
                return Progress::Held;
            }
            crate::player_resurrection::finish_resurrection_visibility(host, catalogs).await;
            if !super::owner::owner_is_current_in_residence(host, owner)
                || !advance(
                    host,
                    owner,
                    Phase::VisibilityInFlight,
                    Phase::ObtainSpellsInFlight,
                )
            {
                return Progress::Held;
            }
            if let Err(reason) = host.cast_all_obtain_spells_like_cpp(catalogs).await {
                tracing::warn!(reason, "Self-resurrection obtain-spell phase failed");
                return Progress::Held;
            }
            if !super::owner::owner_is_current_in_residence(host, owner)
                || !apply_self_resurrection_values(host, owner, health, mana)
                || !advance(
                    host,
                    owner,
                    Phase::ObtainSpellsInFlight,
                    Phase::RecoveryBonesReady,
                )
            {
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
        Phase::RecoveryBonesConfirmed | Phase::PlayerSaveApplied => complete(host, owner, phase),
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

fn apply_self_resurrection_values(
    host: &mut impl CorpseHostLikeCpp,
    owner: Owner,
    health: u32,
    mana: u32,
) -> bool {
    if !owner_is_current(host, owner) {
        return false;
    }
    let update = host.core().with_owned_player_mut_like_cpp(|player| {
        if player.guid() != owner.guid {
            return None;
        }
        // The native setters apply the current post-obtain maxima, matching
        // C++ SetHealth/SetPower after the spell's frozen input calculation.
        player.unit_mut().set_health(u64::from(health));
        player.unit_mut().set_power(PowerType::Mana, mana as i32);
        player.unit_mut().set_power(PowerType::Rage, 0);
        let full_energy = player.get_max_power(PowerType::Energy);
        player.unit_mut().set_power(PowerType::Energy, full_energy);
        player.unit_mut().set_power(PowerType::Focus, 0);
        Some(())
    });
    let Some(Some(())) = update else {
        return false;
    };
    host.sync_player_registry_state_like_cpp();
    true
}
