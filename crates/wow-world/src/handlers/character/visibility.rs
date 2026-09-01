// Copyright (c) 2026 alseif0x
// RustyCore — WoW WotLK 3.4.3 server in Rust
// Based on TrinityCore protocol research (https://github.com/TrinityCore/TrinityCore)
// Licensed under GPL v3 — https://www.gnu.org/licenses/gpl-3.0.html

//! Character visibility, nearby object creation and phase updates.

use super::*;

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

    /// Send nearby creatures to the client as UpdateObject packets.
    ///
    /// Queries the world database for creatures within visibility range
    /// on the player's map, builds CreatureCreateData for each, and sends
    /// a batched UpdateObject.
    pub async fn send_nearby_creatures(&mut self, map_id: u16, position: &Position, _zone_id: u32) {
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
                let mut create_data = creature.create_data.clone();
                create_data.health = i64::from(creature.current_hp());
                create_data.max_health = i64::from(creature.max_hp());
                create_data.level = creature.level();
                create_data.npc_flags = creature.npc_flags_mask_like_cpp();
                create_data.npc_flags = self
                    .represented_viewer_dependent_creature_npc_flags_like_cpp(
                        guid,
                        create_data.npc_flags,
                    );
                create_data.current_area_id = 0;
                blocks.push(UpdateObject::create_creature_block_with_spline(
                    create_data,
                    &creature.position(),
                    creature.active_move_spline_like_cpp().cloned(),
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
            let Some(spawn) = self.materialize_creature_spawn_row_like_cpp(
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

    /// Dynamic visibility update — called when the player moves significantly.
    ///
    /// Queries the DB for all creatures/GOs in the new range, diffs against
    /// the current visible set, and sends:
    ///  - SMSG_UPDATE_OBJECT (CreateObject2) for newly visible objects
    ///  - SMSG_UPDATE_OBJECT (OutOfRange) for objects that left the range
    ///
    /// Threshold: only triggers if the player moved more than 50 yards from
    /// the last visibility update position.
    pub async fn update_visibility(&mut self) {
        use std::collections::HashSet;

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

        let map_creatures = self.visible_world_creatures_from_map_like_cpp(map_id, &pos);
        let canonical_gameobjects =
            self.visible_gameobjects_from_canonical_map_like_cpp(map_id, &pos, range);
        let canonical_dynamic_objects =
            self.visible_dynamic_objects_from_canonical_map_like_cpp(map_id, &pos, range);
        let canonical_area_triggers =
            self.visible_area_triggers_from_canonical_map_like_cpp(map_id, &pos, range);
        let canonical_misc_objects =
            self.visible_misc_objects_from_canonical_map_like_cpp(map_id, &pos, range);
        let visible_other_players =
            self.visible_other_players_from_registry_like_cpp(map_id, &pos, range);
        if self.has_world_map_manager_like_cpp()
            || canonical_gameobjects.is_some()
            || canonical_dynamic_objects.is_some()
            || canonical_area_triggers.is_some()
            || canonical_misc_objects.is_some()
            || self.player_registry().is_some()
        {
            let creature_vis_trace = std::env::var_os("RUSTYCORE_CREATURE_VIS_TRACE").is_some();
            if creature_vis_trace {
                info!(
                    account = self.account_id,
                    map_id,
                    x = pos.x,
                    y = pos.y,
                    z = pos.z,
                    visibility_range = range,
                    candidate_creatures = map_creatures.len(),
                    already_visible_guids = self.client_visible_guids_like_cpp.len(),
                    "RUST_CREATURE_VIS visibility_candidates"
                );
                for (idx, creature) in map_creatures.iter().take(80).enumerate() {
                    let creature_pos = creature.position();
                    let guid = creature.guid();
                    info!(
                        account = self.account_id,
                        map_id,
                        idx,
                        ?guid,
                        entry = creature.entry(),
                        level = creature.level(),
                        hp = creature.current_hp(),
                        max_hp = creature.max_hp(),
                        x = creature_pos.x,
                        y = creature_pos.y,
                        z = creature_pos.z,
                        distance_2d = creature_pos.distance_2d(&pos),
                        already_client_visible = self.client_visible_guids_like_cpp.contains(&guid),
                        "RUST_CREATURE_VIS candidate"
                    );
                }
                if map_creatures.len() > 80 {
                    info!(
                        account = self.account_id,
                        map_id,
                        omitted = map_creatures.len() - 80,
                        "RUST_CREATURE_VIS candidates_omitted"
                    );
                }
            }
            let mut new_visible_creatures: HashSet<ObjectGuid> = HashSet::new();
            let mut new_visible_gos: HashSet<ObjectGuid> = HashSet::new();
            let mut new_visible_dynamic_objects: HashSet<ObjectGuid> = HashSet::new();
            let mut new_visible_area_triggers: HashSet<ObjectGuid> = HashSet::new();
            let mut new_visible_corpses: HashSet<ObjectGuid> = HashSet::new();
            let mut new_visible_scene_objects: HashSet<ObjectGuid> = HashSet::new();
            let mut new_visible_conversations: HashSet<ObjectGuid> = HashSet::new();
            let mut new_visible_players: HashSet<ObjectGuid> = HashSet::new();
            let mut update_blocks: Vec<UpdateBlock> = Vec::new();
            let mut out_of_range_guids: Vec<ObjectGuid> = Vec::new();
            let mut created_creatures = 0usize;
            let mut created_gameobjects = 0usize;
            let mut created_dynamic_objects = 0usize;
            let mut created_area_triggers = 0usize;
            let mut created_corpses = 0usize;
            let mut created_scene_objects = 0usize;
            let mut created_conversations = 0usize;
            let mut created_players = 0usize;
            let mut initial_visible_creatures_like_cpp = Vec::new();
            for creature in &map_creatures {
                let guid = creature.guid();
                new_visible_creatures.insert(guid);
                if !self.client_visible_guids_like_cpp.contains(&guid) {
                    let mut create_data = creature.create_data.clone();
                    create_data.health = i64::from(creature.current_hp());
                    create_data.max_health = i64::from(creature.max_hp());
                    create_data.level = creature.level();
                    create_data.npc_flags = creature.npc_flags_mask_like_cpp();
                    create_data.npc_flags = self
                        .represented_viewer_dependent_creature_npc_flags_like_cpp(
                            guid,
                            create_data.npc_flags,
                        );
                    if creature_vis_trace {
                        let creature_pos = creature.position();
                        info!(
                            account = self.account_id,
                            map_id,
                            ?guid,
                            entry = creature.entry(),
                            level = create_data.level,
                            hp = create_data.health,
                            max_hp = create_data.max_health,
                            npc_flags = create_data.npc_flags,
                            unit_flags = create_data.unit_flags,
                            unit_flags2 = create_data.unit_flags2,
                            unit_flags3 = create_data.unit_flags3,
                            x = creature_pos.x,
                            y = creature_pos.y,
                            z = creature_pos.z,
                            "RUST_CREATURE_VIS create_creature"
                        );
                    }
                    update_blocks.push(UpdateObject::create_creature_block_with_spline(
                        create_data,
                        &creature.position(),
                        creature.active_move_spline_like_cpp().cloned(),
                    ));
                    initial_visible_creatures_like_cpp.push(creature.clone());
                    created_creatures += 1;
                }
            }

            let removed_creatures: Vec<ObjectGuid> = self
                .client_visible_guids_like_cpp
                .snapshot_like_cpp()
                .into_iter()
                .filter(|g| g.is_any_type_creature() && !new_visible_creatures.contains(g))
                .collect();
            if !removed_creatures.is_empty() {
                debug!(
                    "Visibility update: {} map-owned creatures out of range",
                    removed_creatures.len()
                );
                out_of_range_guids.extend(removed_creatures);
            }

            if let Some(gameobjects) = canonical_gameobjects {
                new_visible_gos = gameobjects.iter().map(|go| go.guid).collect();
                for gameobject in gameobjects {
                    if !self
                        .client_visible_guids_like_cpp
                        .contains(&gameobject.guid)
                    {
                        update_blocks.push(UpdateObject::create_gameobject_block(gameobject));
                        created_gameobjects += 1;
                    }
                }
                let removed_gos: Vec<ObjectGuid> = self
                    .client_visible_guids_like_cpp
                    .snapshot_like_cpp()
                    .into_iter()
                    .filter(|g| g.is_game_object() && !new_visible_gos.contains(g))
                    .collect();
                for guid in &removed_gos {
                    self.represented_gameobject_phase_shifts.remove(guid);
                }

                if !removed_gos.is_empty() {
                    debug!(
                        "Visibility update: {} canonical game objects out of range",
                        removed_gos.len()
                    );
                    out_of_range_guids.extend(removed_gos);
                }
            }

            if let Some(dynamic_objects) = canonical_dynamic_objects {
                new_visible_dynamic_objects = dynamic_objects
                    .iter()
                    .map(|dynamic_object| dynamic_object.guid)
                    .collect();
                for dynamic_object in dynamic_objects {
                    if !self
                        .client_visible_guids_like_cpp
                        .contains(&dynamic_object.guid)
                    {
                        update_blocks
                            .push(UpdateObject::create_dynamic_object_block(dynamic_object));
                        created_dynamic_objects += 1;
                    }
                }
                let removed_dynamic_objects: Vec<ObjectGuid> = self
                    .client_visible_guids_like_cpp
                    .snapshot_like_cpp()
                    .into_iter()
                    .filter(|g| g.is_dynamic_object() && !new_visible_dynamic_objects.contains(g))
                    .collect();

                if !removed_dynamic_objects.is_empty() {
                    debug!(
                        "Visibility update: {} canonical dynamic objects out of range",
                        removed_dynamic_objects.len()
                    );
                    out_of_range_guids.extend(removed_dynamic_objects);
                }
            }

            if let Some(area_triggers) = canonical_area_triggers {
                new_visible_area_triggers = area_triggers
                    .iter()
                    .map(|area_trigger| area_trigger.guid)
                    .collect();
                for area_trigger in area_triggers {
                    if !self
                        .client_visible_guids_like_cpp
                        .contains(&area_trigger.guid)
                    {
                        update_blocks.push(UpdateObject::create_area_trigger_block(area_trigger));
                        created_area_triggers += 1;
                    }
                }
                let removed_area_triggers: Vec<ObjectGuid> = self
                    .client_visible_guids_like_cpp
                    .snapshot_like_cpp()
                    .into_iter()
                    .filter(|g| g.is_area_trigger() && !new_visible_area_triggers.contains(g))
                    .collect();

                if !removed_area_triggers.is_empty() {
                    debug!(
                        "Visibility update: {} canonical area triggers out of range",
                        removed_area_triggers.len()
                    );
                    out_of_range_guids.extend(removed_area_triggers);
                }
            }

            if let Some((corpses, scene_objects, conversations)) = canonical_misc_objects {
                new_visible_corpses = corpses.iter().map(|corpse| corpse.guid).collect();
                for corpse in corpses {
                    if !self.client_visible_guids_like_cpp.contains(&corpse.guid) {
                        update_blocks.push(UpdateObject::create_corpse_block(corpse));
                        created_corpses += 1;
                    }
                }

                new_visible_scene_objects = scene_objects.iter().map(|scene| scene.guid).collect();
                for scene_object in scene_objects {
                    if !self
                        .client_visible_guids_like_cpp
                        .contains(&scene_object.guid)
                    {
                        update_blocks.push(UpdateObject::create_scene_object_block(scene_object));
                        created_scene_objects += 1;
                    }
                }

                new_visible_conversations = conversations
                    .iter()
                    .map(|conversation| conversation.guid)
                    .collect();
                for conversation in conversations {
                    if !self
                        .client_visible_guids_like_cpp
                        .contains(&conversation.guid)
                    {
                        update_blocks.push(UpdateObject::create_conversation_block(conversation));
                        created_conversations += 1;
                    }
                }

                let removed_misc_objects: Vec<ObjectGuid> = self
                    .client_visible_guids_like_cpp
                    .snapshot_like_cpp()
                    .into_iter()
                    .filter(|guid| {
                        (guid.is_corpse() && !new_visible_corpses.contains(guid))
                            || (guid.is_scene_object() && !new_visible_scene_objects.contains(guid))
                            || (guid.is_conversation() && !new_visible_conversations.contains(guid))
                    })
                    .collect();
                out_of_range_guids.extend(removed_misc_objects);
            }

            for (guid, player) in visible_other_players {
                new_visible_players.insert(guid);
                if self.client_visible_guids_like_cpp.contains(&guid) {
                    continue;
                }

                let mut update =
                    player_visibility_create_update_from_snapshot_like_cpp(&player, map_id);
                if let Some(block) = update.blocks.pop() {
                    update_blocks.push(block);
                    created_players += 1;
                }
            }
            let removed_players: Vec<ObjectGuid> = self
                .client_visible_guids_like_cpp
                .snapshot_like_cpp()
                .into_iter()
                .filter(|guid| guid.is_player() && !new_visible_players.contains(guid))
                .collect();
            out_of_range_guids.extend(removed_players);

            // The membership replacement and the UpdateObject that carries it are
            // one client-visible step. A cast resolving between them would either
            // skip a viewer whose client already received the create block, or
            // address a caster whose out-of-range block is already queued, so
            // publish both under the same write.
            let visibility_like_cpp = self.client_visible_guids_like_cpp.clone();
            visibility_like_cpp.publish_transition_like_cpp(
                |guid| {
                    !guid.is_any_type_creature()
                        && !guid.is_game_object()
                        && !guid.is_dynamic_object()
                        && !guid.is_area_trigger()
                        && !guid.is_corpse()
                        && !guid.is_scene_object()
                        && !guid.is_conversation()
                        && !guid.is_player()
                },
                new_visible_creatures
                    .iter()
                    .chain(new_visible_gos.iter())
                    .chain(new_visible_dynamic_objects.iter())
                    .chain(new_visible_area_triggers.iter())
                    .chain(new_visible_corpses.iter())
                    .chain(new_visible_scene_objects.iter())
                    .chain(new_visible_conversations.iter())
                    .chain(new_visible_players.iter())
                    .copied(),
                || {
                    if update_blocks.is_empty() && out_of_range_guids.is_empty() {
                        return;
                    }
                    let update = UpdateObject {
                        map_id,
                        num_updates: update_blocks.len() as u32,
                        destroy_guids: Vec::new(),
                        out_of_range_guids,
                        blocks: update_blocks,
                    };
                    if std::env::var_os("RUSTYCORE_UPDATEOBJECT_TRACE").is_some() {
                        info!(
                            map_id,
                            created_creatures,
                            created_gameobjects,
                            created_dynamic_objects,
                            created_area_triggers,
                            created_corpses,
                            created_scene_objects,
                            created_conversations,
                            created_players,
                            "RUST_UPDATEOBJECT visibility_update plan"
                        );
                        for line in update.debug_create_summary_like_cpp() {
                            info!("RUST_UPDATEOBJECT visibility_update {line}");
                        }
                    }
                    self.send_packet(&update);
                    for creature in &initial_visible_creatures_like_cpp {
                        self.send_initial_visible_packets_for_creature_like_cpp(creature);
                    }
                },
            );
            self.last_visibility_pos = Some(pos);
            debug!(
                "Visibility updated at ({:.1}, {:.1}): {} creatures / {} GOs in range",
                pos.x,
                pos.y,
                self.client_visible_guids_like_cpp
                    .snapshot_like_cpp()
                    .into_iter()
                    .filter(|guid| guid.is_any_type_creature())
                    .count(),
                self.client_visible_guids_like_cpp
                    .snapshot_like_cpp()
                    .into_iter()
                    .filter(|guid| guid.is_game_object())
                    .count()
            );
            return;
        }

        // ── CREATURES ───────────────────────────────────────────────────
        let port = match self.visibility_spawn_catalog_persistence_port_like_cpp() {
            Some(port) => port,
            None => return,
        };
        let creatures = match tokio::time::timeout(
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
            _ => return,
        };

        let mut new_visible_creatures: HashSet<ObjectGuid> = HashSet::new();
        let mut update_blocks: Vec<UpdateBlock> = Vec::new();
        let mut out_of_range_guids: Vec<ObjectGuid> = Vec::new();
        let mut created_creatures = 0usize;
        let mut created_gameobjects = 0usize;

        if !creatures.is_empty() {
            for row in &creatures {
                let Some(spawn) =
                    self.materialize_creature_spawn_row_like_cpp(map_id, row, &pos, range)
                else {
                    continue;
                };

                new_visible_creatures.insert(spawn.guid);

                if !self.client_visible_guids_like_cpp.contains(&spawn.guid) {
                    self.register_materialized_creature_spawn_like_cpp(map_id, &spawn);
                    update_blocks.push(self.viewer_creature_create_block_like_cpp(&spawn));
                    created_creatures += 1;
                }
            }
        }

        // Creatures that left range → out-of-range
        let removed_creatures: Vec<ObjectGuid> = self
            .client_visible_guids_like_cpp
            .snapshot_like_cpp()
            .into_iter()
            .filter(|g| g.is_any_type_creature() && !new_visible_creatures.contains(g))
            .collect();

        if !removed_creatures.is_empty() {
            debug!(
                "Visibility update: {} creatures out of range",
                removed_creatures.len()
            );
            out_of_range_guids.extend(removed_creatures);
        }

        // ── GAME OBJECTS ────────────────────────────────────────────────
        let gameobjects = match tokio::time::timeout(
            std::time::Duration::from_secs(5),
            port.load_gameobjects_in_bounds_like_cpp(
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
            _ => {
                self.last_visibility_pos = Some(pos);
                return;
            }
        };

        let mut new_visible_gos: HashSet<ObjectGuid> = HashSet::new();

        if !gameobjects.is_empty() {
            for row in &gameobjects {
                let spawn_guid = row.spawn_guid;
                let entry = row.entry;
                let [pos_x, pos_y, pos_z, orientation] = row.position;
                if !is_within_2d_visibility_range_like_cpp(&pos, pos_x, pos_y, range) {
                    continue;
                }
                let [rot0, rot1, rot2, rot3] = row.rotation;
                // #NEXT.R8.ENTITIES.1216: GameObjectData.ParentRotation from per-spawn
                // gameobject_addon (cols 58-61); NULL (no addon) -> identity (0,0,0,1).
                let [parent_rot0, parent_rot1, parent_rot2, parent_rot3] = row.parent_rotation;
                let anim_progress = row.anim_progress;
                let state = row.state;
                let go_type = row.go_type;
                let display_id = row.display_id;
                let scale = row.scale;
                let template_data = row.template_data.map(|raw| u32::try_from(raw).unwrap_or(0));
                let data2 = template_data[2];
                let data3 = template_data[3];
                let template = GameObjectTemplateData::new(u32::from(go_type), template_data);
                let phase_use_flags = row.phase_use_flags;
                let phase_id = row.phase_id;
                let phase_group_id = row.phase_group_id;
                let terrain_swap_map = row.terrain_swap_map;
                let effective_flags = row.effective_flags;
                let effective_faction = row.effective_faction;
                let override_source_known = row.override_source_known;

                if display_id == 0 {
                    continue;
                }

                let (target_phase_shift, _) = self.db_spawn_phase_shift_like_cpp(
                    map_id,
                    phase_use_flags,
                    phase_id,
                    phase_group_id,
                    terrain_swap_map,
                );
                if !self.can_see_phase_shift_like_cpp(&target_phase_shift) {
                    continue;
                }

                let guid = ObjectGuid::create_world_object(
                    HighGuid::GameObject,
                    0,
                    realm_id,
                    map_id,
                    1,
                    entry,
                    spawn_guid as i64,
                );
                if self.represented_gameobject_is_per_player_despawned_like_cpp(guid) {
                    continue;
                }
                new_visible_gos.insert(guid);
                self.record_represented_gameobject_db_phase_shift_like_cpp(
                    guid,
                    map_id,
                    phase_use_flags,
                    phase_id,
                    phase_group_id,
                    terrain_swap_map,
                );

                if !self.client_visible_guids_like_cpp.contains(&guid) {
                    let go_pos = Position::new(pos_x, pos_y, pos_z, orientation);
                    let dynamic_flags = self
                        .represented_gameobject_dynamic_flags_for_player_like_cpp(
                            entry,
                            &RepresentedGameObjectUseState {
                                go_type: Some(go_type),
                                go_state: represented_go_state_from_i8_like_cpp(state),
                                ..Default::default()
                            },
                        );
                    let create_data = GameObjectCreateData {
                        guid,
                        entry,
                        dynamic_flags,
                        display_id,
                        go_type,
                        position: go_pos,
                        rotation: [rot0, rot1, rot2, rot3],
                        anim_progress,
                        state,
                        // C++ ObjectMgr initializes SQL `GameObjectData::artKit` to zero.
                        art_kit: 0,
                        created_by: ObjectGuid::EMPTY,
                        faction_template: effective_faction as i32,
                        gameobject_flags: effective_flags,
                        world_effect_id: 0,
                        scale,
                        level: 0, // non-transport GameObject: Level unused (period via AnimationData)
                        parent_rotation: [parent_rot0, parent_rot1, parent_rot2, parent_rot3],
                    };
                    update_blocks.push(UpdateObject::create_gameobject_block(create_data));
                    created_gameobjects += 1;
                    self.record_represented_gameobject_runtime_state_like_cpp(
                        map_id, guid, entry, go_pos, go_type,
                    );
                } else {
                    self.record_represented_gameobject_runtime_state_like_cpp(
                        map_id,
                        guid,
                        entry,
                        Position::new(pos_x, pos_y, pos_z, orientation),
                        go_type,
                    );
                }
                self.record_represented_gameobject_override_like_cpp(
                    guid,
                    effective_flags,
                    effective_faction,
                    override_source_known,
                );
                if u32::from(go_type) == GAMEOBJECT_TYPE_FISHING_HOLE {
                    let max_opens = if data2 <= data3 {
                        self.represented_urand_u32_like_cpp(data2, data3)
                    } else {
                        data2
                    };
                    self.record_represented_fishing_hole_max_opens_like_cpp(guid, max_opens);
                    self.record_represented_fishing_hole_radius_like_cpp(guid, template_data[0]);
                }
                self.record_represented_gameobject_interact_radius_override_like_cpp(
                    guid,
                    template.get_interact_radius_override_like_cpp(),
                );
                self.record_represented_gameobject_lock_id_like_cpp(
                    guid,
                    template.get_lock_id_like_cpp(),
                );
                self.record_represented_gameobject_display_model_like_cpp(
                    guid,
                    display_id,
                    scale,
                    [rot0, rot1, rot2, rot3],
                );
                self.record_represented_gameobject_anim_progress_like_cpp(guid, anim_progress);
            }
        }

        let removed_gos: Vec<ObjectGuid> = self
            .client_visible_guids_like_cpp
            .snapshot_like_cpp()
            .into_iter()
            .filter(|g| g.is_game_object() && !new_visible_gos.contains(g))
            .collect();
        for guid in &removed_gos {
            self.represented_gameobject_phase_shifts.remove(guid);
        }

        if !removed_gos.is_empty() {
            debug!(
                "Visibility update: {} game objects out of range",
                removed_gos.len()
            );
            out_of_range_guids.extend(removed_gos);
        }

        if !update_blocks.is_empty() || !out_of_range_guids.is_empty() {
            let update = UpdateObject {
                map_id,
                num_updates: update_blocks.len() as u32,
                destroy_guids: Vec::new(),
                out_of_range_guids,
                blocks: update_blocks,
            };
            if std::env::var_os("RUSTYCORE_UPDATEOBJECT_TRACE").is_some() {
                info!(
                    map_id,
                    created_creatures,
                    created_gameobjects,
                    "RUST_UPDATEOBJECT visibility_update_db plan"
                );
                for line in update.debug_create_summary_like_cpp() {
                    info!("RUST_UPDATEOBJECT visibility_update_db {line}");
                }
            }
            self.send_packet(&update);
        }

        self.client_visible_guids_like_cpp
            .retain(|guid| !guid.is_any_type_creature() && !guid.is_game_object());
        self.client_visible_guids_like_cpp
            .extend(new_visible_creatures.iter().copied());
        self.client_visible_guids_like_cpp
            .extend(new_visible_gos.iter().copied());

        // ── Update position marker ──────────────────────────────────────
        self.last_visibility_pos = Some(pos);
        debug!(
            "Visibility updated at ({:.1}, {:.1}): {} creatures / {} GOs in range",
            pos.x,
            pos.y,
            self.client_visible_guids_like_cpp
                .snapshot_like_cpp()
                .into_iter()
                .filter(|guid| guid.is_any_type_creature())
                .count(),
            self.client_visible_guids_like_cpp
                .snapshot_like_cpp()
                .into_iter()
                .filter(|guid| guid.is_game_object())
                .count()
        );
    }

    /// Send nearby gameobjects to the client as UpdateObject packets.
    pub async fn send_nearby_gameobjects(
        &mut self,
        map_id: u16,
        position: &Position,
        _zone_id: u32,
    ) {
        if let Some(gameobjects) = self.visible_gameobjects_from_canonical_map_like_cpp(
            map_id,
            position,
            DEFAULT_VISIBILITY_DISTANCE_LIKE_CPP,
        ) {
            if gameobjects.is_empty() {
                self.client_visible_guids_like_cpp
                    .retain(|guid| !guid.is_game_object());
                return;
            }

            let go_guids: HashSet<_> = gameobjects.iter().map(|go| go.guid).collect();
            // C++ `Player::UpdateVisibilityOf` (Player.cpp) only CREATES gameobjects NOT
            // already in `m_clientGUIDs` (`!HaveAtClient`). Re-creating a known gameobject
            // sends a duplicate CREATE, which the Wrath client rejects by resetting the
            // connection. This function runs on world-port/spawn and must skip known GOs.
            let known_guids = &self.client_visible_guids_like_cpp;
            let blocks = gameobjects
                .into_iter()
                .filter(|go| !known_guids.contains(&go.guid))
                .map(UpdateObject::create_gameobject_block)
                .collect::<Vec<_>>();
            let count = blocks.len();
            self.client_visible_guids_like_cpp
                .retain(|guid| !guid.is_game_object());
            self.client_visible_guids_like_cpp
                .extend(go_guids.iter().copied());
            if !blocks.is_empty() {
                self.send_packet(&UpdateObject::create_world_objects(blocks, map_id));
            }
            debug!(
                "Sent {} new canonical gameobjects to account {} on map {}",
                count, self.account_id, map_id
            );
            return;
        }

        let port = match self.visibility_spawn_catalog_persistence_port_like_cpp() {
            Some(port) => port,
            None => return,
        };

        let x_min = position.x - DEFAULT_VISIBILITY_DISTANCE_LIKE_CPP;
        let x_max = position.x + DEFAULT_VISIBILITY_DISTANCE_LIKE_CPP;
        let y_min = position.y - DEFAULT_VISIBILITY_DISTANCE_LIKE_CPP;
        let y_max = position.y + DEFAULT_VISIBILITY_DISTANCE_LIKE_CPP;

        let gameobjects = match tokio::time::timeout(
            std::time::Duration::from_secs(5),
            port.load_gameobjects_in_bounds_like_cpp(
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
                warn!("Failed to query gameobjects for map {map_id}: {reason}");
                return;
            }
            Err(_) => {
                warn!("Gameobject query timed out for map {map_id}");
                return;
            }
        };

        if gameobjects.is_empty() {
            return;
        }

        let realm_id = self.realm_id();
        let mut blocks = Vec::new();
        let mut go_guids: Vec<wow_core::ObjectGuid> = Vec::new();
        for row in &gameobjects {
            let spawn_guid = row.spawn_guid;
            let entry = row.entry;
            let [pos_x, pos_y, pos_z, orientation] = row.position;
            if !is_within_2d_visibility_range_like_cpp(
                position,
                pos_x,
                pos_y,
                DEFAULT_VISIBILITY_DISTANCE_LIKE_CPP,
            ) {
                continue;
            }
            let [rot0, rot1, rot2, rot3] = row.rotation;
            // #NEXT.R8.ENTITIES.1216: GameObjectData.ParentRotation from per-spawn
            // gameobject_addon (cols 58-61); NULL (no addon) -> identity (0,0,0,1).
            let [parent_rot0, parent_rot1, parent_rot2, parent_rot3] = row.parent_rotation;
            // C++ GameObject::Create defaults animProgress to 255 (GameObject.cpp:1068,1089);
            // match the other GO paths (canonical ~6920) instead of 0. #NEXT.R8.ENTITIES.1218.
            let anim_progress = row.anim_progress;
            let state = row.state;
            let go_type = row.go_type;
            let display_id = row.display_id;
            let scale = row.scale;
            let template_data = row.template_data.map(|raw| u32::try_from(raw).unwrap_or(0));
            let data2 = template_data[2];
            let data3 = template_data[3];
            let template = GameObjectTemplateData::new(u32::from(go_type), template_data);
            let phase_use_flags = row.phase_use_flags;
            let phase_id = row.phase_id;
            let phase_group_id = row.phase_group_id;
            let terrain_swap_map = row.terrain_swap_map;
            let effective_flags = row.effective_flags;
            let effective_faction = row.effective_faction;
            let override_source_known = row.override_source_known;

            // Skip gameobjects with no display
            if display_id == 0 {
                continue;
            }

            let (target_phase_shift, _) = self.db_spawn_phase_shift_like_cpp(
                map_id,
                phase_use_flags,
                phase_id,
                phase_group_id,
                terrain_swap_map,
            );
            if !self.can_see_phase_shift_like_cpp(&target_phase_shift) {
                continue;
            }

            let guid = ObjectGuid::create_world_object(
                HighGuid::GameObject,
                0,
                realm_id,
                map_id,
                1,
                entry,
                spawn_guid as i64,
            );

            let go_pos = Position::new(pos_x, pos_y, pos_z, orientation);
            let dynamic_flags = self.represented_gameobject_dynamic_flags_for_player_like_cpp(
                entry,
                &RepresentedGameObjectUseState {
                    go_type: Some(go_type),
                    go_state: represented_go_state_from_i8_like_cpp(state),
                    ..Default::default()
                },
            );
            let create_data = GameObjectCreateData {
                guid,
                entry,
                dynamic_flags,
                display_id,
                go_type,
                position: go_pos,
                rotation: [rot0, rot1, rot2, rot3],
                anim_progress,
                state,
                // C++ ObjectMgr initializes SQL `GameObjectData::artKit` to zero.
                art_kit: 0,
                created_by: ObjectGuid::EMPTY,
                faction_template: effective_faction as i32,
                gameobject_flags: effective_flags,
                world_effect_id: 0,
                scale,
                level: 0, // non-transport GameObject: Level unused (period via AnimationData)
                parent_rotation: [parent_rot0, parent_rot1, parent_rot2, parent_rot3],
            };

            blocks.push(UpdateObject::create_gameobject_block(create_data));
            go_guids.push(guid);
            self.record_represented_gameobject_db_phase_shift_like_cpp(
                guid,
                map_id,
                phase_use_flags,
                phase_id,
                phase_group_id,
                terrain_swap_map,
            );
            self.record_represented_gameobject_runtime_state_like_cpp(
                map_id, guid, entry, go_pos, go_type,
            );
            self.record_represented_gameobject_override_like_cpp(
                guid,
                effective_flags,
                effective_faction,
                override_source_known,
            );
            if u32::from(go_type) == GAMEOBJECT_TYPE_FISHING_HOLE {
                let max_opens = if data2 <= data3 {
                    self.represented_urand_u32_like_cpp(data2, data3)
                } else {
                    data2
                };
                self.record_represented_fishing_hole_max_opens_like_cpp(guid, max_opens);
                self.record_represented_fishing_hole_radius_like_cpp(guid, template_data[0]);
            }
            self.record_represented_gameobject_interact_radius_override_like_cpp(
                guid,
                template.get_interact_radius_override_like_cpp(),
            );
            self.record_represented_gameobject_lock_id_like_cpp(
                guid,
                template.get_lock_id_like_cpp(),
            );
            self.record_represented_gameobject_display_model_like_cpp(
                guid,
                display_id,
                scale,
                [rot0, rot1, rot2, rot3],
            );
            self.record_represented_gameobject_anim_progress_like_cpp(guid, anim_progress);
        }

        if blocks.is_empty() {
            return;
        }

        self.client_visible_guids_like_cpp
            .retain(|guid| !guid.is_game_object());
        self.client_visible_guids_like_cpp
            .extend(go_guids.iter().copied());
        let count = blocks.len();
        let update = UpdateObject::create_world_objects(blocks, map_id);
        self.send_packet(&update);
        debug!(
            "Sent {} gameobjects to account {} on map {}",
            count, self.account_id, map_id
        );
    }
}
