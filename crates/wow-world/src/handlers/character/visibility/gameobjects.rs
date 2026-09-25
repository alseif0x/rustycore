//! Nearby GameObject delivery operation.

use super::*;

impl WorldSession {
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
