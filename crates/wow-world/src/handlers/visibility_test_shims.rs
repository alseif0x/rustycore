// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Test-only method entry points for the visibility bodies moved to
//! `wow_world_entities::visibility_session` (#1363 slice 8a). Each forwards to
//! the moved body with this session as its host, so the scenarios keep their
//! method syntax and the character hotspot's test lines do not grow.

#![cfg(test)]

use wow_core::Position;

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
}
