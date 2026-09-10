//! The combat and creature ticks that drive represented effect execution.
//!
//! Moved out of the Session root under #621. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;
use crate::session_rules::position_is_in_dist_strict_2d_like_cpp;

impl WorldSession {
    /// Called every ~200ms from the update loop.
    /// Advances creature movement state and sends MonsterMove packets.
    /// Extract of the creatures tick body. Returns all bytes that must be sent
    /// to the session channel. Callers must flush via `flush_runtime_output`.
    ///
    /// No `send_tx.send` or `send_packet` calls may remain here — each is
    /// converted to `output.packets.push` at the same relative position.
    pub(crate) fn run_creatures_tick(&mut self) -> RuntimeOutput {
        let mut output = RuntimeOutput::new();
        let Some(player_phase_shift) = self.represented_player_phase_shift_like_cpp() else {
            return output;
        };

        // Collect movement packets (avoids borrow conflict with send_packet)
        let mut to_send: Vec<Vec<u8>> = Vec::new();

        let guids = self.active_world_creature_guids_for_update_like_cpp();

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
                if !crate::session_rules::creature_message_to_set_target_allows_like_cpp(
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
    /// Wrapper: runs the creatures tick and flushes packets to the session channel.
    pub(crate) fn tick_creatures_sync(&mut self) {
        let out = self.run_creatures_tick();
        self.flush_runtime_output(out);
    }
    /// Called every ~100ms. Handles auto-attack swing timer (player → creature).
    /// Extract of the combat tick body. Returns all bytes that must be sent to
    /// the session channel. Callers must flush via `flush_runtime_output`.
    ///
    /// No `send_tx.send` or `send_packet` calls may remain here — each is
    /// converted to `output.packets.push` at the same relative position.
    pub(crate) fn run_combat_tick(&mut self) -> RuntimeOutput {
        use wow_packet::ServerPacket;
        use wow_packet::packets::combat::{
            AttackerStateUpdate, HIT_INFO_NORMAL_SWING, SAttackStop, VICTIM_STATE_HIT,
        };
        use wow_packet::packets::movement::MonsterMoveStop;

        let mut output = RuntimeOutput::new();

        let Some(player_guid) = self.player_guid() else {
            return output;
        };
        self.revalidate_canonical_player_combat_refs_like_cpp(player_guid);
        let canonical_attack_state = self.canonical_player_attack_state_like_cpp();
        let Some(combat_target) = (match canonical_attack_state {
            Some(Some(target)) => Some(target),
            // C++: Unit::GetVictim() is authoritative. If the canonical Player
            // exists but has no victim, do not resurrect stale session mirrors.
            Some(None) => None,
            None => self.resolved_combat_target_like_cpp().flatten(),
        }) else {
            self.set_combat_target_like_cpp(None);
            self.set_in_combat_like_cpp(false);
            return output;
        };
        self.set_combat_target_like_cpp(Some(combat_target));
        let canonical_threat_before = self
            .canonical_creature_threat_value_like_cpp(combat_target, player_guid)
            .unwrap_or(0.0);
        let now = Instant::now();
        let diff_ms = now
            .saturating_duration_since(self.combat_tick_last_at_like_cpp)
            .as_millis()
            .min(u128::from(u32::MAX)) as u32;
        self.combat_tick_last_at_like_cpp = now;
        // Check if target still exists. C++ `Unit::UpdateMeleeAttackingState`
        // is driven from `GetVictim()`; if the represented creature vanished,
        // clear the canonical player's attack state as well as session mirrors.
        #[derive(Clone, Copy)]
        enum CombatTargetRuntimeLikeCpp {
            WorldCreature {
                position: Position,
                combat_reach: f32,
                bounding_radius: f32,
            },
            CanonicalPlayer {
                position: Position,
                combat_reach: f32,
                bounding_radius: f32,
            },
        }

        let target_runtime = self
            .mutate_world_creature(combat_target, |creature| {
                let unit_data = creature.creature.unit().data();
                CombatTargetRuntimeLikeCpp::WorldCreature {
                    position: creature.position(),
                    combat_reach: unit_data.combat_reach,
                    bounding_radius: unit_data.bounding_radius,
                }
            })
            .or_else(|| {
                self.mutate_canonical_player_by_guid_like_cpp(combat_target, |player| {
                    let unit_data = player.unit().data();
                    CombatTargetRuntimeLikeCpp::CanonicalPlayer {
                        position: player.unit().world().position(),
                        combat_reach: unit_data.combat_reach,
                        bounding_radius: unit_data.bounding_radius,
                    }
                })
            });
        let Some(target_runtime) = target_runtime else {
            let _ = self.mutate_canonical_player_like_cpp(|player| {
                let unit = player.unit_mut();
                unit.attack_stop_like_cpp();
                unit.subsystems_mut()
                    .combat
                    .purge_combat_ref_like_cpp(combat_target);
            });
            self.set_combat_target_like_cpp(None);
            self.set_in_combat_like_cpp(false);
            return output;
        };
        let (target_position, target_combat_reach, target_bounding_radius) = match target_runtime {
            CombatTargetRuntimeLikeCpp::WorldCreature {
                position,
                combat_reach,
                bounding_radius,
            }
            | CombatTargetRuntimeLikeCpp::CanonicalPlayer {
                position,
                combat_reach,
                bounding_radius,
            } => (position, combat_reach, bounding_radius),
        };
        let player_position = self.player_position_like_cpp();
        let player_combat_reach = self
            .mutate_canonical_player_like_cpp(|player| player.unit().data().combat_reach)
            .unwrap_or(0.0);
        let in_melee_range = player_position
            .map(|position| {
                is_within_melee_range_like_cpp(
                    position,
                    player_combat_reach,
                    target_position,
                    target_combat_reach,
                )
            })
            .unwrap_or(true);
        let facing_target = player_position
            .map(|position| {
                is_within_target_boundary_radius_like_cpp(
                    position,
                    player_combat_reach,
                    target_position,
                    target_combat_reach,
                    target_bounding_radius,
                ) || is_unit_facing_target_for_melee_like_cpp(position, target_position)
            })
            .unwrap_or(true);
        // C++ line-of-sight for the player's own swing is not ported yet; the
        // session passes the same value the retired `Option<bool>` field always
        // held in production (#28). The parameter stays so the branch is
        // reachable from a test and from whoever owns the tick.
        let within_los = true;
        let canonical_attack_update = self.take_canonical_player_attack_swings_like_cpp(
            diff_ms,
            in_melee_range,
            facing_target,
            within_los,
        );
        let (canonical_swing_damages, swing_error_update) = match canonical_attack_update {
            Some((damages, swing_error_update)) => (Some(damages), swing_error_update),
            None if self.canonical_map_manager.is_some() => return output,
            None => (None, None),
        };
        if let Some(swing_error) = swing_error_update {
            self.set_player_attack_swing_error_like_cpp(swing_error);
        }

        if let CombatTargetRuntimeLikeCpp::CanonicalPlayer { .. } = target_runtime {
            let Some((swings, target_level)) = self
                .mutate_canonical_player_by_guid_like_cpp(combat_target, |victim| {
                    apply_player_melee_to_canonical_player_like_cpp(
                        victim,
                        canonical_swing_damages.as_deref().unwrap_or(&[]),
                    )
                })
                .flatten()
            else {
                let _ = self.mutate_canonical_player_like_cpp(|player| {
                    player.unit_mut().attack_stop_like_cpp()
                });
                self.set_combat_target_like_cpp(None);
                self.set_in_combat_like_cpp(false);
                return output;
            };

            if !swings.is_empty() {
                let _ = self.mutate_canonical_player_like_cpp(|player| {
                    player
                        .unit_mut()
                        .set_last_damaged_target_like_cpp(Some(combat_target));
                });
            }
            for (dmg, over_damage) in &swings {
                let state_update = AttackerStateUpdate {
                    attacker: player_guid,
                    victim: combat_target,
                    hit_info: HIT_INFO_NORMAL_SWING,
                    damage: *dmg as i32,
                    over_damage: *over_damage,
                    victim_state: VICTIM_STATE_HIT,
                    school_mask: 1,
                    target_level,
                    expansion: 2,
                };
                output.packets.push(state_update.to_bytes());
            }
            return output;
        }

        let tap_group_guids = self.current_group_member_guids_for_tap_like_cpp(player_guid);

        // Gather combat data from the canonical map-owned creature before
        // emitting combat packets.
        let Some(PlayerMeleeCreatureHitLikeCpp {
            swings,
            entry: target_entry,
            level: target_level,
            died: now_dead,
            move_stop,
            values_update,
        }) = self
            .mutate_world_creature(combat_target, |creature| {
                apply_player_melee_to_legacy_creature_like_cpp(
                    creature,
                    player_guid,
                    &tap_group_guids,
                    canonical_swing_damages.as_deref(),
                )
            })
            .flatten()
        else {
            return output;
        };

        if !swings.is_empty() {
            let _ = self.mutate_canonical_player_like_cpp(|player| {
                player
                    .unit_mut()
                    .set_last_damaged_target_like_cpp(Some(combat_target));
            });
            if !now_dead {
                let total_damage: u32 = swings.iter().map(|(damage, _, _)| *damage).sum();
                let _ = self.mirror_canonical_creature_threat_from_attacker_like_cpp(
                    combat_target,
                    player_guid,
                    canonical_threat_before + total_damage as f32,
                );
            }
        }

        for (dmg, _swing_killed, over_damage) in &swings {
            let state_update = AttackerStateUpdate {
                attacker: player_guid,
                victim: combat_target,
                hit_info: HIT_INFO_NORMAL_SWING,
                damage: *dmg as i32,
                over_damage: *over_damage,
                victim_state: VICTIM_STATE_HIT,
                school_mask: 1,
                target_level,
                expansion: 2,
            };
            output.packets.push(state_update.to_bytes());
        }

        if self.client_visible_guids_like_cpp.contains(&combat_target)
            && let Some(update) = self.represented_unit_values_update_to_update_object_like_cpp(
                combat_target,
                self.player_map_id_like_cpp(),
                &values_update,
            )
        {
            output.packets.push(update.to_bytes());
        }

        if now_dead {
            self.queue_pending_creature_kill_like_cpp(
                player_guid,
                combat_target,
                target_entry,
                target_level,
            );
            if let Some((current_pos, spline_id)) = move_stop {
                output.packets.push(
                    MonsterMoveStop {
                        mover_guid: combat_target,
                        current_pos,
                        spline_id,
                    }
                    .to_bytes(),
                );
            }
            let stop = SAttackStop {
                attacker: player_guid,
                victim: combat_target,
                now_dead: true,
            };
            output.packets.push(stop.to_bytes());
            let _ = self.mutate_canonical_player_like_cpp(|player| {
                let unit = player.unit_mut();
                unit.attack_stop_like_cpp();
                unit.subsystems_mut()
                    .combat
                    .purge_combat_ref_like_cpp(combat_target);
            });
            self.revalidate_canonical_player_combat_refs_like_cpp(player_guid);
            self.set_combat_target_like_cpp(None);
            self.set_in_combat_like_cpp(false);
        }
        output
    }
    /// Wrapper: runs the combat tick and flushes packets to the session channel.
    pub(crate) fn tick_combat_sync(&mut self) {
        let out = self.run_combat_tick();
        self.flush_runtime_output(out);
    }
    /// Read the [`RuntimeTickOwner`] from the shared [`MapManager`], copy it
    /// (enum is `Copy`), and release the lock before returning.
    ///
    /// Falls back to `Session` if no map manager is attached.
    pub(crate) fn runtime_tick_owner_like_cpp(&self) -> RuntimeTickOwner {
        self.map_manager
            .as_ref()
            .map(crate::map_manager::shared_runtime_tick_owner_like_cpp)
            .unwrap_or(RuntimeTickOwner::Session)
    }
}
