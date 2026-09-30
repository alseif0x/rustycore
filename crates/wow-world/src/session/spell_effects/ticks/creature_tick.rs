//! Existing session-owned corpse/respawn/movement tick, with opt-in fixture phase.
use super::*;
use wow_core::position_is_in_dist_strict_2d_like_cpp;

impl WorldSession {
    pub(crate) fn run_creatures_tick_with_fixture_phase(&mut self, fixture_phase: bool) -> RuntimeOutput {
        let mut output = RuntimeOutput::new();
        let Some(player_phase_shift) = self.represented_player_phase_shift_like_cpp().or_else(|| {
            (fixture_phase && self.player_handle_like_cpp.is_none())
                .then(PhaseShift::default)
        }) else {
            return output;
        };

        // Collect movement packets (avoids borrow conflict with send_packet)
        let mut to_send: Vec<Vec<u8>> = Vec::new();

        let guids = self.active_world_creature_guids_with_fixture_phase(fixture_phase);

        // ── Corpse despawn ─────────────────────────────────────────────────
        // C++ refs: `Creature::RemoveCorpse` / `AllLootRemovedFromCorpse`.
        // After `corpse_despawn_at` passes, remove the dead creature from the
        // world and notify the client (destroy block in SMSG_UPDATE_OBJECT).
        let now = std::time::Instant::now();
        let despawn_guids: Vec<wow_core::ObjectGuid> = guids
            .iter()
            .filter(|g| {
                self.mutate_world_creature(**g, |c| {
                    !c.is_alive() && c.corpse_despawn_due_like_cpp()
                })
                .unwrap_or(false)
            })
            .copied()
            .collect();

        if !despawn_guids.is_empty() {
            use wow_packet::ServerPacket;
            use wow_packet::packets::update::UpdateObject;

            let map_id = self.player_map_id_like_cpp();
            for g in &despawn_guids {
                // Before removing, save data needed for respawn.
                if let Some(c) = self.remove_world_creature(*g) {
                    // C++ `AllLootRemovedFromCorpse` sets
                    // `m_respawnTime = max(m_corpseRemoveTime + m_respawnDelay, m_respawnTime)`.
                    let respawn_at = now
                        + std::time::Duration::from_secs(
                            c.creature.ai_ownership().respawn_time_secs,
                        );
                    // instance_id=0: legacy path — consistent with register/remove/mutate_world_creature.
                    self.push_map_respawn_like_cpp(
                        map_id,
                        0,
                        crate::map_manager::pending_respawn_from_world_creature_like_cpp(
                            &c, respawn_at, map_id,
                        ),
                    );
                    tracing::info!(
                        "Corpse despawned: {:?} (entry {}) — respawn in {}s",
                        g,
                        c.entry(),
                        c.creature.ai_ownership().respawn_time_secs
                    );
                }
                self.client_visible_guids_like_cpp.remove(g);
            }
            let pkt = UpdateObject::destroy_objects(despawn_guids, map_id);
            output.packets.push(pkt.to_bytes());
        }
        // ── Respawn queue ──────────────────────────────────────────────────
        // C++ refs: `Map::ProcessRespawns` drains `Map::_respawnTimes`, then
        // `Map::DoRespawn(SPAWN_TYPE_CREATURE, spawnId, gridId)` recreates via
        // `Creature::LoadFromDB`.
        // Lock is acquired inside drain_ready_map_respawns_like_cpp and released
        // before returning; register_world_creature and packet building happen below,
        // outside the lock.
        // instance_id=0: legacy path — consistent with register/remove/mutate_world_creature.
        let current_map_id = self.player_map_id_like_cpp();
        let ready: Vec<PendingRespawn> =
            self.drain_ready_map_respawns_like_cpp(current_map_id, 0, now);

        for r in ready {
            use wow_packet::ServerPacket;
            use wow_packet::packets::update::UpdateObject;

            let guid = r.create_data.guid;
            let entry = r.create_data.entry;
            tracing::info!(
                "Creature respawned: {:?} (entry {}) at {:?}",
                guid,
                entry,
                r.home_pos
            );
            let respawn_position = crate::map_manager::pending_respawn_create_position_like_cpp(&r);

            // Recreate canonical map state.
            self.register_world_creature_with_flags_extra_movement_and_default_motion_like_cpp(
                r.map_id,
                respawn_position,
                r.create_data.clone(),
                r.min_dmg,
                r.max_dmg,
                r.aggro_radius,
                r.loot_id,
                r.skin_loot_id,
                r.gold_min,
                r.gold_max,
                r.respawn_delay_secs,
                r.selected_equipment_id,
                r.original_equipment_id,
                r.script_name.clone(),
                r.string_id.clone(),
                r.addon.clone(),
                r.boss_id,
                r.dungeon_encounter_id,
                r.phase_use_flags,
                r.phase_id,
                r.phase_group_id,
                r.terrain_swap_map,
                r.flags_extra,
                r.ground_movement_type,
                r.swim_allowed,
                r.flight_movement_type,
                r.rooted,
                r.chase_movement_type,
                r.random_movement_type,
                r.interaction_pause_timer_ms,
                r.wander_distance,
                r.default_movement_type,
                r.waypoint_path_id,
            );

            // Send CREATE block to client with C++ viewer-dependent
            // `UnitData::NpcFlags[0]` filtering; runtime keeps full flags.
            let mut viewer_create_data = r.create_data.clone();
            viewer_create_data.npc_flags = self
                .represented_viewer_dependent_creature_npc_flags_like_cpp(
                    guid,
                    viewer_create_data.npc_flags,
                );
            let block = UpdateObject::create_creature_block(viewer_create_data, &respawn_position);
            let pkt = UpdateObject::create_creatures(vec![block], r.map_id);
            output.packets.push(pkt.to_bytes());
            self.client_visible_guids_like_cpp.insert(guid);
        }
        // ──────────────────────────────────────────────────────────────────

        let mmap_runtime_config = self.mmap_runtime_config_like_cpp.clone();
        let mmap_pathfinder = self.mmap_pathfinder_like_cpp.clone();
        let live_terrain = self.map_manager.as_ref().and_then(|manager| {
            manager
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .terrain()
        });
        let visible_guids = self.client_visible_guids_like_cpp.snapshot_like_cpp();
        let player_position = self.player_position_like_cpp();
        let player_map_id = u32::from(self.player_map_id_like_cpp());
        let player_instance_id = self
            .current_canonical_player_map_key_like_cpp()
            .map(|key| key.instance_id)
            .unwrap_or(0);
        let monster_move_trace = std::env::var_os("RUSTYCORE_MONSTER_MOVE_TRACE").is_some();
        for guid in guids {
            let _ = self.mutate_world_creature(guid, |creature| {
                let Some(player_position) = player_position else {
                    if monster_move_trace {
                        tracing::info!(
                            ?guid,
                            visible_count = visible_guids.len(),
                            "RUST_MONSTER_MOVE_DELIVERY session rejected before movement: missing player position"
                        );
                    }
                    return;
                };

                let is_visible = visible_guids.contains(&guid);
                let same_map = creature.map_id() == player_map_id;
                let same_instance = creature.instance_id() == player_instance_id;
                let same_phase = player_phase_shift.can_see(creature.phase_shift());
                let range = creature.visibility_range_like_cpp();
                let in_range = position_is_in_dist_strict_2d_like_cpp(
                    &creature.position(),
                    &player_position,
                    range,
                );
                if !crate::session::creature_message_to_set_target_allows_like_cpp(
                    creature,
                    visible_guids.contains(&guid),
                    player_map_id,
                    player_instance_id,
                    &player_position,
                    &player_phase_shift,
                    false,
                ) {
                    if monster_move_trace {
                        tracing::info!(
                            ?guid,
                            is_visible,
                            same_map,
                            same_instance,
                            same_phase,
                            in_range,
                            range,
                            player_map_id,
                            player_instance_id,
                            creature_map = creature.map_id(),
                            creature_instance = creature.instance_id(),
                            visible_count = visible_guids.len(),
                            "RUST_MONSTER_MOVE_DELIVERY session rejected before movement"
                        );
                    }
                    return;
                }

                // The per-session tick is not the default runtime owner, and it
                // has no cross-object accessor here, so chase target snapshots
                // are only supplied by the global owner.
                if let Some(pkt) = step_creature_movement_like_cpp(
                    creature,
                    guid,
                    &mmap_runtime_config,
                    mmap_pathfinder.as_deref(),
                    live_terrain.as_deref(),
                    None,
                    200,
                ) {
                    if monster_move_trace {
                        tracing::info!(
                            ?guid,
                            player_map_id,
                            player_instance_id,
                            creature_map = creature.map_id(),
                            creature_instance = creature.instance_id(),
                            visible_count = visible_guids.len(),
                            packet_len = pkt.len(),
                            "RUST_MONSTER_MOVE_DELIVERY session sent"
                        );
                    }
                    to_send.push(pkt);
                }
            });
        }

        // Collect movement packets into output in the same order they were built.
        if monster_move_trace && !to_send.is_empty() {
            tracing::info!(
                packet_count = to_send.len(),
                "RUST_MONSTER_MOVE_DELIVERY session output"
            );
        }
        output.packets.extend(to_send);
        output
    }
}
