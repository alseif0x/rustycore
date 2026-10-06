use wow_core::ObjectGuid;
use wow_world_core::session::{HubMut, HubRef, RepresentedGameObjectAccessLikeCpp};

use crate::{RepresentedGameObjectUseEffect, WorldEntitiesState};

impl WorldEntitiesState {
    pub fn record_represented_gameobject_interact_radius_override_like_cpp(
        &mut self,
        guid: ObjectGuid,
        interact_radius_override: u32,
    ) {
        self.represented_gameobject_use_states
            .entry(guid)
            .or_default()
            .interact_radius_override =
            (interact_radius_override != 0).then_some(interact_radius_override);
    }

    pub fn record_represented_gameobject_icon_interaction_like_cpp(
        &mut self,
        guid: ObjectGuid,
        allows_interaction: bool,
    ) {
        self.represented_gameobject_use_states
            .entry(guid)
            .or_default()
            .icon_name_allows_interaction_like_cpp = Some(allows_interaction);
    }

    pub fn represented_gameobject_can_interact_with_like_cpp(
        &self,
        hub: HubRef<'_>,
        guid: ObjectGuid,
        interaction_distance: f32,
    ) -> Option<RepresentedGameObjectAccessLikeCpp> {
        let access = hub.core.canonical_gameobject_access_like_cpp(guid)?;
        let player_position = hub.player_position_like_cpp()?;
        if access
            .position
            .is_within_dist(&player_position, interaction_distance)
        {
            Some(access)
        } else {
            None
        }
    }

    pub fn represented_gameobject_use_allowed_by_mover_like_cpp(
        &self,
        hub: HubRef<'_>,
        gameobject_usable_mounted: bool,
    ) -> bool {
        let Some(player_guid) = hub.core.player_guid() else {
            return false;
        };
        if hub.player_moved_unit_guid_like_cpp() == Some(player_guid) {
            return true;
        }

        hub.player_vehicle_seat_state_like_cpp()
            .and_then(|(flags, _)| flags)
            .is_some()
            || hub.resolved_player_mounted_like_cpp() == Some(true)
            || gameobject_usable_mounted
    }

    pub fn record_represented_gameobject_report_use_ai_like_cpp(
        &mut self,
        gameobject_guid: ObjectGuid,
        player_guid: ObjectGuid,
    ) -> bool {
        let handled = self
            .represented_gameobject_use_states
            .get(&gameobject_guid)
            .map(|state| state.report_use_ai_returns_true)
            .unwrap_or(false);
        self.represented_gameobject_use_effects
            .push(RepresentedGameObjectUseEffect::ReportUseAi {
                gameobject_guid,
                player_guid,
                handled,
            });
        handled
    }
}
