//! Synchronous Map, Legacy and registry visibility capture and publication.

use super::*;
use std::collections::HashSet;

impl WorldSession {
    pub(super) fn try_refresh_owned_visibility(
        &mut self,
        map_id: u16,
        pos: Position,
        range: f32,
    ) -> bool {
        let map_creatures = self.visible_world_creatures_from_map_like_cpp(map_id, &pos);
        let canonical_gameobjects =
            self.visible_gameobjects_from_canonical_map_like_cpp(map_id, &pos, range);
        let canonical_dynamic_objects =
            self.visible_dynamic_objects_from_canonical_map_like_cpp(map_id, &pos, range);
        let canonical_area_triggers =
            self.visible_area_triggers_from_canonical_map_like_cpp(map_id, &pos, range);
        let canonical_misc_objects =
            self.visible_misc_objects_from_canonical_map_like_cpp(map_id, &pos, range);
        let canonical_transports = self.visible_transports_from_canonical_map_like_cpp(map_id);
        let transports_visibility_available = canonical_transports.is_some();
        let visible_other_players =
            self.visible_other_players_from_registry_like_cpp(map_id, &pos, range);
        if self.has_world_map_manager_like_cpp()
            || canonical_gameobjects.is_some()
            || canonical_dynamic_objects.is_some()
            || canonical_area_triggers.is_some()
            || canonical_misc_objects.is_some()
            || canonical_transports.is_some()
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
                    let facts = creature.create();
                    let creature_pos = facts.position();
                    let guid = creature.guid();
                    info!(
                        account = self.account_id,
                        map_id,
                        idx,
                        ?guid,
                        entry = facts.entry(),
                        level = facts.level(),
                        hp = facts.current_hp(),
                        max_hp = facts.max_hp(),
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
            let mut new_visible_transports: HashSet<ObjectGuid> = HashSet::new();
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
            let mut created_transports = 0usize;
            let mut initial_visible_creatures_like_cpp = Vec::new();
            for (index, creature) in map_creatures.iter().enumerate() {
                let guid = creature.guid();
                new_visible_creatures.insert(guid);
                if !self.client_visible_guids_like_cpp.contains(&guid) {
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
                    if creature_vis_trace {
                        let creature_pos = facts.position();
                        info!(
                            account = self.account_id,
                            map_id,
                            ?guid,
                            entry = facts.entry(),
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
                        &facts.position(),
                        facts
                            .active_move_spline()
                            .and_then(crate::entity_update_bridge::create_object_spline_data_like_cpp),
                    ));
                    initial_visible_creatures_like_cpp.push(index);
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

            if let Some(transports) = canonical_transports {
                new_visible_transports =
                    transports.iter().map(|transport| transport.guid).collect();
                let server_time_ms = crate::session::game_time_ms_like_cpp();
                for transport in transports {
                    if !self
                        .client_visible_transports_like_cpp
                        .contains(&transport.guid)
                    {
                        update_blocks.push(UpdateObject::create_transport_block(
                            transport,
                            server_time_ms,
                        ));
                        created_transports += 1;
                    }
                }
                let removed_transports: Vec<ObjectGuid> = self
                    .client_visible_transports_like_cpp
                    .snapshot_like_cpp()
                    .into_iter()
                    .filter(|guid| !new_visible_transports.contains(guid))
                    .collect();
                out_of_range_guids.extend(removed_transports);
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
            let publish_visibility = || {
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
                                created_transports,
                                "RUST_UPDATEOBJECT visibility_update plan"
                            );
                            for line in update.debug_create_summary_like_cpp() {
                                info!("RUST_UPDATEOBJECT visibility_update {line}");
                            }
                        }
                        self.send_packet(&update);
                        for index in &initial_visible_creatures_like_cpp {
                            self.send_initial_visible_packets_for_creature_like_cpp(
                                map_creatures[*index].initial_auras(),
                            );
                        }
                    },
                );
            };
            if transports_visibility_available {
                let transports_like_cpp = self.client_visible_transports_like_cpp.clone();
                transports_like_cpp.publish_transition_like_cpp(
                    |guid| new_visible_transports.contains(guid),
                    new_visible_transports.iter().copied(),
                    publish_visibility,
                );
            } else {
                publish_visibility();
            }
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
            return true;
        }

        false
    }
}
