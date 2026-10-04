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
        hub.core.loot_release_access_like_cpp()
            .release_canonical_fishing_hole_like_cpp(guid, max_opens)
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
