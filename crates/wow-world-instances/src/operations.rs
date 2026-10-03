use crate::InstanceState;
#[cfg(any(test, feature = "test-fixtures"))]
use crate::RepresentedAreaZoneCriteriaLikeCpp;
#[cfg(any(test, feature = "test-fixtures"))]
use wow_entities::PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP;
use wow_core::ObjectGuid;
use wow_data::TavernAreaTriggerStoreLikeCpp;
use wow_world_core::session::{HubMut, PlayerBootstrapCatalogsLikeCpp};

impl InstanceState {
    pub fn represented_is_tavern_area_trigger_like_cpp(
        &self,
        taverns: &TavernAreaTriggerStoreLikeCpp,
        trigger_id: u32,
    ) -> bool {
        taverns.is_tavern_area_trigger_like_cpp(trigger_id)
    }

    #[cfg(any(test, feature = "test-fixtures"))]
    pub fn represented_area_zone_criteria_like_cpp(
        &self,
    ) -> &[RepresentedAreaZoneCriteriaLikeCpp] {
        &self.represented_area_zone_criteria_like_cpp
    }

    /// C++ `Player::AddExploredZones` loop for `CONFIG_START_ALL_EXPLORED` on first login.
    pub fn apply_represented_first_login_explored_zones_with_catalogs_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        player_bootstrap: &PlayerBootstrapCatalogsLikeCpp,
    ) -> usize {
        if !player_bootstrap.start_all_explored {
            return 0;
        }

        let Some((applied, update)) = hub
            .core
            .mutate_canonical_player_like_cpp(|player| {
                let mut applied = 0usize;
                for index in 0..wow_entities::PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP {
                    if player.add_explored_zones_like_cpp(index, u64::MAX) {
                        applied += 1;
                    }
                }

                (applied > 0).then(|| (applied, player.values_update(true)))
            })
            .flatten()
        else {
            return 0;
        };

        #[cfg(any(test, feature = "test-fixtures"))]
        if hub.core.player_handle_like_cpp.is_none() {
            self.represented_explored_zones_like_cpp =
                [u64::MAX; PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP];
        }
        hub.core.send_player_values_update_like_cpp(&update);
        applied
    }

    pub fn set_area_spirit_healer_guid_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        healer_guid: ObjectGuid,
    ) -> bool {
        let canonical = hub
            .core
            .with_owned_player_mut_like_cpp(|player| {
                player
                    .resurrection_state_mut_like_cpp()
                    .area_spirit_healer_guid = healer_guid;
            })
            .is_some();
        #[cfg(any(test, feature = "test-fixtures"))]
        if canonical || hub.core.player_handle_like_cpp.is_none() {
            hub.fixtures.combat.area_spirit_healer_guid_like_cpp = healer_guid;
        }
        canonical || cfg!(any(test, feature = "test-fixtures")) && hub.core.player_handle_like_cpp.is_none()
    }

    pub fn set_player_zone_area_authority_complete_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        complete: bool,
    ) {
        let _ = hub.set_player_zone_area_authority_like_cpp(complete);
        if !complete {
            hub.core
                .invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        }
    }
}
