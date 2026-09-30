//! Nearby-creature delivery and visibility refresh operations.

use super::*;

#[path = "creatures/nearby.rs"]
mod nearby;
#[path = "creatures/owned_refresh.rs"]
mod owned_refresh;
#[path = "creatures/catalog_refresh.rs"]
mod catalog_refresh;

impl WorldSession {
    fn viewer_creature_create_block_like_cpp(
        &mut self,
        spawn: &MaterializedCreatureSpawnLikeCpp,
    ) -> UpdateBlock {
        let mut viewer_create_data = spawn.create_data.clone();
        viewer_create_data.npc_flags = self
            .represented_viewer_dependent_creature_npc_flags_like_cpp(
                spawn.guid,
                viewer_create_data.npc_flags,
            );
        UpdateObject::create_creature_block(viewer_create_data, &spawn.position)
    }

    /// Dynamic visibility update — called when the player moves significantly.
    ///
    /// Queries the DB for all creatures/GOs in the new range, diffs against
    /// the current visible set, and sends:
    ///  - SMSG_UPDATE_OBJECT (CreateObject2) for newly visible objects
    ///  - SMSG_UPDATE_OBJECT (OutOfRange) for objects that left the range
    ///
    /// Threshold: only triggers if the player moved more than 50 yards from
    /// the last visibility update position.
    pub async fn update_visibility_with_catalogs_like_cpp(
        &mut self,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
    ) {
        // ── Position & threshold check ──────────────────────────────────
        self.sync_represented_farsight_clear_from_canonical_like_cpp();
        let pos = match self.represented_visibility_source_position_like_cpp() {
            Some(p) => p,
            None => return,
        };
        let forced_refresh = self.consume_movement_visibility_refresh_request_like_cpp();

        if !forced_refresh && let Some(last) = self.last_visibility_pos {
            let dx = pos.x - last.x;
            let dy = pos.y - last.y;
            if dx * dx + dy * dy < 50.0 * 50.0 {
                return; // haven't moved enough yet
            }
        }

        let map_id = self.player_map_id_like_cpp();
        let realm_id = self.realm_id();

        let range = self.player_map_visibility_range_like_cpp(map_id);
        let x_min = pos.x - range;
        let x_max = pos.x + range;
        let y_min = pos.y - range;
        let y_max = pos.y + range;

        if self.try_refresh_owned_visibility(map_id, pos, range) {
            return;
        }
        self.refresh_catalog_visibility(
            creature_spawn_catalogs,
            map_id,
            realm_id,
            pos,
            range,
            (x_min, x_max, y_min, y_max),
        )
        .await;
    }
}

#[cfg(test)]
impl WorldSession {
    pub async fn send_nearby_creatures(&mut self, map_id: u16, position: &Position, zone_id: u32) {
        let catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        self.send_nearby_creatures_with_catalogs_like_cpp(&catalogs, map_id, position, zone_id)
            .await;
    }

    pub async fn update_visibility(&mut self) {
        let catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        self.update_visibility_with_catalogs_like_cpp(&catalogs)
            .await;
    }
}
