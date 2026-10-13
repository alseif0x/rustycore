// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the visibility bodies moved to
//! `wow_world_entities::visibility_session` (#1363 slice 8a). Each forwards to
//! the moved body with this session as its host, so the scenarios keep their
//! method syntax and the character hotspot's test lines do not grow.
//!
//! The four `split_world_entities_*` wrappers below came from
//! `session/world_entities/{aggro,creature_query,gameobject_overrides,gameobject_state}.rs`
//! verbatim: their only production callers were the moved bodies, which now
//! make the same split and state call through the host.

#![cfg(test)]

use wow_core::{ObjectGuid, Position};
use wow_world_entities::{CreatureCreateStatsLikeCpp, CreatureSpawnCatalogsLikeCpp};

use crate::session::WorldSession;

impl WorldSession {
    pub(crate) async fn send_nearby_creatures(
        &mut self,
        map_id: u16,
        position: &Position,
        zone_id: u32,
    ) {
        let catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        wow_world_entities::send_nearby_creatures_with_catalogs_like_cpp(
            self, &catalogs, map_id, position, zone_id,
        )
        .await;
    }

    pub(crate) async fn update_visibility(&mut self) {
        let catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        wow_world_entities::update_visibility_with_catalogs_like_cpp(self, &catalogs).await;
    }

    pub(crate) async fn send_nearby_gameobjects(
        &mut self,
        map_id: u16,
        position: &Position,
        zone_id: u32,
    ) {
        wow_world_entities::send_nearby_gameobjects(self, map_id, position, zone_id).await;
    }

    pub(crate) fn creature_aggro_radius_for_faction_template_like_cpp(
        &self,
        faction_template_id: u32,
        default_radius: f32,
    ) -> f32 {
        let (state, hub) = crate::session::split_world_entities_ref(self);
        state.creature_aggro_radius_for_faction_template_like_cpp(
            hub,
            faction_template_id,
            default_radius,
        )
    }

    pub(crate) fn creature_create_stats_with_catalogs_like_cpp(
        &self,
        catalogs: &CreatureSpawnCatalogsLikeCpp,
        entry: u32,
        level: u8,
        unit_class: u8,
        classification: u32,
        regen_health: bool,
        db_cur_health: u32,
        db_cur_mana: u32,
    ) -> CreatureCreateStatsLikeCpp {
        let (state, hub) = crate::session::split_world_entities_ref(self);
        state.creature_create_stats_with_catalogs_like_cpp(
            hub,
            catalogs,
            entry,
            level,
            unit_class,
            classification,
            regen_health,
            db_cur_health,
            db_cur_mana,
        )
    }

    pub(crate) fn record_represented_gameobject_db_phase_shift_like_cpp(
        &mut self,
        guid: ObjectGuid,
        map_id: u16,
        phase_use_flags: u8,
        phase_id: u16,
        phase_group_id: u32,
        terrain_swap_map: i32,
    ) {
        let (state, mut hub) = crate::session::split_world_entities_mut(self);
        state.record_represented_gameobject_db_phase_shift_like_cpp(
            &mut hub,
            guid,
            map_id,
            phase_use_flags,
            phase_id,
            phase_group_id,
            terrain_swap_map,
        )
    }

    pub(crate) fn record_represented_gameobject_runtime_state_like_cpp(
        &mut self,
        map_id: u16,
        guid: ObjectGuid,
        entry: u32,
        position: wow_core::Position,
        go_type: u8,
    ) {
        let (state, mut hub) = crate::session::split_world_entities_mut(self);
        state.record_represented_gameobject_runtime_state_like_cpp(
            &mut hub, map_id, guid, entry, position, go_type,
        )
    }
}
