//! Sequential Creature and GameObject catalog refresh fallback.

use super::*;
use std::collections::HashSet;

impl WorldSession {
    pub(super) async fn refresh_catalog_visibility(
        &mut self,
        catalogs: &CreatureSpawnCatalogsLikeCpp,
        map_id: u16,
        realm_id: u16,
        pos: Position,
        range: f32,
        bounds: (f32, f32, f32, f32),
    ) {
        let creature_spawn_catalogs = catalogs;
        let (x_min, x_max, y_min, y_max) = bounds;
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
                let Some(spawn) = self.materialize_creature_spawn_row_with_catalogs_like_cpp(
                    creature_spawn_catalogs,
                    map_id,
                    row,
                    &pos,
                    range,
                ) else {
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
}
