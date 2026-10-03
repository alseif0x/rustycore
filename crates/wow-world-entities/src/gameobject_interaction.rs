use std::sync::Arc;
use std::time::Instant;

use wow_core::ObjectGuid;
use wow_world_core::session::HubMut;

use crate::WorldEntitiesState;

impl WorldEntitiesState {
    /// C++ fishing-hole release performs AddUse, MaxOpens comparison, and
    /// SetLootState on one world thread. Keep all three under one map lock so
    /// two concurrent personal releases cannot finish in `Ready` after max.
    pub fn release_canonical_fishing_hole_like_cpp(
        &mut self,
        hub: &mut HubMut<'_>,
        guid: ObjectGuid,
        max_opens: Option<u32>,
    ) -> Option<(
        u32,
        wow_entities::LootState,
        wow_map::map::GameObjectSetLootStateOutcomeLikeCpp,
    )> {
        let map_key = hub
            .core
            .canonical_object_lookup_map_key_like_cpp(u32::from(
                hub.core.player_map_id_like_cpp(),
            ))?;
        let game_time_secs = i64::try_from(wow_core::GameTime::now().as_secs()).unwrap_or(i64::MAX);
        let manager = Arc::clone(hub.core.canonical_map_manager.as_ref()?);
        let mut manager = manager.lock().ok()?;
        let managed = manager.find_map_mut(map_key.map_id, map_key.instance_id)?;
        let map = managed.map_mut();
        let use_count = {
            let gameobject = map.get_typed_game_object_mut(guid)?;
            gameobject.add_use_like_cpp();
            gameobject.use_times()
        };
        let loot_state = if max_opens.is_some_and(|max_opens| use_count >= max_opens) {
            wow_entities::LootState::JustDeactivated
        } else {
            wow_entities::LootState::Ready
        };
        let outcome = map.set_gameobject_loot_state_like_cpp(
            guid,
            loot_state,
            None,
            game_time_secs,
            0,
            false,
        );
        Some((use_count, loot_state, outcome))
    }

    pub fn record_represented_fishing_hole_max_opens_like_cpp(
        &mut self,
        guid: ObjectGuid,
        max_opens: u32,
    ) {
        self.represented_gameobject_use_states
            .entry(guid)
            .or_default()
            .fishing_hole_max_opens = Some(max_opens);
    }

    pub fn record_represented_fishing_hole_radius_like_cpp(
        &mut self,
        guid: ObjectGuid,
        radius: u32,
    ) {
        self.represented_gameobject_use_states
            .entry(guid)
            .or_default()
            .fishing_hole_radius = Some(radius as f32);
    }

    pub fn lookup_represented_fishing_hole_around_like_cpp(
        &self,
        gameobject_guid: ObjectGuid,
    ) -> Option<ObjectGuid> {
        const FISHING_HOLE_SEARCH_RANGE_LIKE_CPP: f32 =
            20.0 + wow_movement::CONTACT_DISTANCE_LIKE_CPP;

        let source = self
            .represented_gameobject_use_states
            .get(&gameobject_guid)?;
        let source_position = source.position?;
        let source_map_id = source.map_id;
        let now = Instant::now();
        let mut nearest: Option<(ObjectGuid, f32)> = None;

        for (candidate_guid, candidate) in &self.represented_gameobject_use_states {
            if *candidate_guid == gameobject_guid {
                continue;
            }
            if candidate.go_type != Some(wow_entities::GAMEOBJECT_TYPE_FISHING_HOLE as u8) {
                continue;
            }
            if source_map_id.is_some() && candidate.map_id != source_map_id {
                continue;
            }
            if candidate
                .per_player_despawn_until
                .is_some_and(|until| until > now)
            {
                continue;
            }
            let Some(candidate_position) = candidate.position else {
                continue;
            };
            let Some(fishing_hole_radius) = candidate.fishing_hole_radius else {
                continue;
            };
            if !source_position
                .is_within_dist(&candidate_position, FISHING_HOLE_SEARCH_RANGE_LIKE_CPP)
                || !source_position.is_within_dist(&candidate_position, fishing_hole_radius)
            {
                continue;
            }
            let distance = source_position.distance(&candidate_position);
            if nearest.is_none_or(|(_, nearest_distance)| distance < nearest_distance) {
                nearest = Some((*candidate_guid, distance));
            }
        }

        nearest.map(|(guid, _)| guid)
    }
}
