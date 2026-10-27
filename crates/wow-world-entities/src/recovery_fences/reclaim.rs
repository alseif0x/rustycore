use wow_entities::{PlayerResurrectionPhase as Phase};

use super::owner::{Owner, Progress, advance, clear_complete, complete, owner_is_current, run_bones, save_player};
use crate::{CorpseHostLikeCpp, ResurrectionOperationCatalogs};

pub(super) async fn continue_reclaim(
    host: &mut impl CorpseHostLikeCpp,
    owner: Owner,
    catalogs: ResurrectionOperationCatalogs<'_>,
    restore_percent: f32,
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
            if crate::player_resurrection::resurrect_player_native(host, restore_percent).is_err() {
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
                tracing::warn!(reason, "Reclaim resurrection obtain-spell phase failed");
                return Progress::Held;
            }
            if !owner_is_current(host, owner)
                || !super::owner::owner_is_current_in_residence(host, owner)
                || !advance(host, owner, Phase::ObtainSpellsInFlight, Phase::RecoveryBonesReady)
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
