//! Nearby Creature delivery through map ownership or the catalog fallback.

use super::*;

impl WorldSession {
    /// Send nearby creatures to the client as UpdateObject packets.
    ///
    /// Queries the world database for creatures within visibility range
    /// on the player's map, builds CreatureCreateData for each, and sends
    /// a batched UpdateObject.
    pub async fn send_nearby_creatures_with_catalogs_like_cpp(
        &mut self,
        catalogs: &CreatureSpawnCatalogsLikeCpp,
        map_id: u16,
        position: &Position,
        _zone_id: u32,
    ) {
        let map_creatures = self.visible_world_creatures_from_map_like_cpp(map_id, position);
        if self.has_world_map_manager_like_cpp() {
            let mut blocks = Vec::with_capacity(map_creatures.len());
            let mut visible_guids = Vec::with_capacity(map_creatures.len());
            for creature in &map_creatures {
                let guid = creature.guid();
                // C++ `Player::UpdateVisibilityOf` (Player.cpp) only builds a CREATE block
                // when the object is NOT already in `m_clientGUIDs` (`!HaveAtClient`).
                // Re-creating an object the client already knows sends a duplicate CREATE,
                // which the Wrath client rejects by resetting the connection. This function
                // runs on world-port/spawn and must not re-create already-known creatures.
                visible_guids.push(guid);
                if self.client_visible_guids_like_cpp.contains(&guid) {
                    continue;
                }
                let facts = creature.create();
                let mut create_data = facts.create_data().clone();
                create_data.health = i64::from(facts.current_hp());
                create_data.max_health = i64::from(facts.max_hp());
                create_data.level = facts.level();
                create_data.npc_flags = facts.npc_flags_mask();
                create_data.npc_flags = self
                    .represented_viewer_dependent_creature_npc_flags_like_cpp(
                        guid,
                        create_data.npc_flags,
                    );
                create_data.current_area_id = 0;
                blocks.push(UpdateObject::create_creature_block_with_spline(
                    create_data,
                    &facts.position(),
                    facts
                        .active_move_spline()
                        .and_then(crate::entity_update_bridge::create_object_spline_data_like_cpp),
                ));
            }

            // Publish the creature membership and its create blocks as one step;
            // see `publish_transition_like_cpp`.
            let visibility_like_cpp = self.client_visible_guids_like_cpp.clone();
            visibility_like_cpp.publish_transition_like_cpp(
                |guid| !guid.is_any_type_creature(),
                visible_guids.iter().copied(),
                || {
                    if blocks.is_empty() {
                        return;
                    }
                    let update = UpdateObject::create_creatures(blocks, map_id);
                    if std::env::var_os("RUSTYCORE_UPDATEOBJECT_TRACE").is_some() {
                        for line in update.debug_create_summary_like_cpp() {
                            info!("RUST_UPDATEOBJECT map_owned_creatures {line}");
                        }
                    }
                    self.send_packet(&update);
                },
            );
            self.last_visibility_pos = Some(*position);
            debug!(
                "Sent {} map-owned creatures to account {} on map {}",
                visible_guids.len(),
                self.account_id,
                map_id
            );
            return;
        }

        let port = match self.visibility_spawn_catalog_persistence_port_like_cpp() {
            Some(port) => port,
            None => {
                self.client_visible_guids_like_cpp
                    .retain(|guid| !guid.is_any_type_creature());
                self.last_visibility_pos = Some(*position);
                warn!("No world database — skipping creature spawn");
                return;
            }
        };

        let x_min = position.x - DEFAULT_VISIBILITY_DISTANCE_LIKE_CPP;
        let x_max = position.x + DEFAULT_VISIBILITY_DISTANCE_LIKE_CPP;
        let y_min = position.y - DEFAULT_VISIBILITY_DISTANCE_LIKE_CPP;
        let y_max = position.y + DEFAULT_VISIBILITY_DISTANCE_LIKE_CPP;

        let rows = match tokio::time::timeout(
            std::time::Duration::from_secs(5),
            port.load_creatures_in_bounds_like_cpp(
                wow_persistence::VisibilitySpawnCatalogRequestLikeCpp {
                    map_id,
                    x_min,
                    x_max,
                    y_min,
                    y_max,
                },
            ),
        )
        .await
        {
            Ok(wow_persistence::VisibilitySpawnCatalogOutcomeLikeCpp::Loaded(rows)) => rows,
            Ok(wow_persistence::VisibilitySpawnCatalogOutcomeLikeCpp::Failed { reason }) => {
                warn!("Failed to query creatures for map {map_id}: {reason}");
                return;
            }
            Err(_) => {
                warn!("Creature query timed out for map {map_id}");
                return;
            }
        };

        if rows.is_empty() {
            self.client_visible_guids_like_cpp
                .retain(|guid| !guid.is_any_type_creature());
            self.last_visibility_pos = Some(*position);
            return;
        }

        let mut blocks = Vec::new();
        let mut visible_guids = Vec::new();
        for row in &rows {
            let Some(spawn) = self.materialize_creature_spawn_row_with_catalogs_like_cpp(
                catalogs,
                map_id,
                row,
                position,
                DEFAULT_VISIBILITY_DISTANCE_LIKE_CPP,
            ) else {
                continue;
            };

            self.register_materialized_creature_spawn_like_cpp(map_id, &spawn);
            blocks.push(self.viewer_creature_create_block_like_cpp(&spawn));
            visible_guids.push(spawn.guid);
        }

        if blocks.is_empty() {
            return;
        }

        let count = blocks.len();
        let update = UpdateObject::create_creatures(blocks, map_id);
        if std::env::var_os("RUSTYCORE_UPDATEOBJECT_TRACE").is_some() {
            for line in update.debug_create_summary_like_cpp() {
                info!("RUST_UPDATEOBJECT nearby_creatures {line}");
            }
        }
        // Mirror C++ Player::m_clientGUIDs semantics: this is the exact set
        // of creatures sent to this client, not every creature loaded on map.
        // The membership and its packet are published as one step; see
        // `publish_transition_like_cpp`.
        let visibility_like_cpp = self.client_visible_guids_like_cpp.clone();
        visibility_like_cpp.publish_transition_like_cpp(
            |guid| !guid.is_any_type_creature(),
            visible_guids.iter().copied(),
            || self.send_packet(&update),
        );
        self.last_visibility_pos = Some(*position);
        let mob_count = visible_guids
            .iter()
            .filter(|g| {
                self.mutate_world_creature(**g, |creature| creature.npc_flags() == 0)
                    .unwrap_or(false)
            })
            .count();
        let npc_count = visible_guids.len().saturating_sub(mob_count);
        info!(
            "Sent {} creatures ({} mobs / {} npcs) to account {} on map {}",
            count, mob_count, npc_count, self.account_id, map_id
        );
    }

}
