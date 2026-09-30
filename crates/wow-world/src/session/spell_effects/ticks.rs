//! The combat and creature ticks that drive represented effect execution.
//!
//! Moved out of the Session root under #621. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;
mod creature_tick;

impl WorldSession {
    /// Called every ~200ms from the update loop.
    /// Advances creature movement state and sends MonsterMove packets.
    /// Extract of the creatures tick body. Returns all bytes that must be sent
    /// to the session channel. Callers must flush via `flush_runtime_output`.
    ///
    /// No `send_tx.send` or `send_packet` calls may remain here — each is
    /// converted to `output.packets.push` at the same relative position.
    pub(crate) fn run_creatures_tick(&mut self) -> RuntimeOutput {
        self.run_creatures_tick_with_fixture_phase(false)
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
            AttackerStateUpdate, HIT_INFO_AFFECTS_VICTIM, SAttackStop, VICTIM_STATE_HIT,
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
            .saturating_duration_since(self.view.combat_tick_last_at_like_cpp)
            .as_millis()
            .min(u128::from(u32::MAX)) as u32;
        self.view.combat_tick_last_at_like_cpp = now;
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
            for (index, (dmg, over_damage)) in swings.iter().enumerate() {
                let (hit_info, victim_state, original_damage) = canonical_swing_damages
                    .as_deref()
                    .and_then(|swings| swings.get(index))
                    .map_or(
                        (HIT_INFO_AFFECTS_VICTIM, VICTIM_STATE_HIT, *dmg as i32),
                        |swing| {
                            (
                                swing.hit_info,
                                swing.victim_state,
                                swing.original_damage as i32,
                            )
                        },
                    );
                let state_update = AttackerStateUpdate {
                    attacker: player_guid,
                    victim: combat_target,
                    hit_info,
                    damage: *dmg as i32,
                    original_damage,
                    over_damage: *over_damage,
                    blocked: canonical_swing_damages
                        .as_deref()
                        .and_then(|swings| swings.get(index))
                        .map_or(0, |swing| swing.blocked as i32),
                    absorbed: 0,
                    victim_state,
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
            swing_presentations,
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

        for (index, (dmg, _swing_killed, over_damage)) in swings.iter().enumerate() {
            let (hit_info, victim_state, blocked, original_damage) = swing_presentations
                .get(index)
                .copied()
                .unwrap_or((HIT_INFO_AFFECTS_VICTIM, VICTIM_STATE_HIT, 0, *dmg));
            let state_update = AttackerStateUpdate {
                attacker: player_guid,
                victim: combat_target,
                hit_info,
                damage: *dmg as i32,
                original_damage: original_damage as i32,
                over_damage: *over_damage,
                blocked: blocked as i32,
                absorbed: 0,
                victim_state,
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
