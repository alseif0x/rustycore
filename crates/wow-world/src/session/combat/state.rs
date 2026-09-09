//! Represented combat state and PvP flags at the Session boundary.
//!
//! Moved out of the Session root under #617. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(in crate::session) fn player_is_pvp_like_cpp(&self, guid: ObjectGuid) -> Option<bool> {
        if self.player_guid() != Some(guid) {
            return None;
        }
        let canonical = self.with_owned_player_like_cpp(|player| {
            player
                .unit()
                .pvp_flags_like_cpp()
                .contains(UnitPvpFlags::PVP)
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            if let Some(flags) = self.canonical_player_pvp_flags_like_cpp(guid) {
                return Some(flags.contains(UnitPvpFlags::PVP));
            }
            return Some(self.player_pvp_enabled_like_cpp);
        }
        canonical
    }
    #[cfg(test)]
    pub(in crate::session) fn canonical_player_pvp_flags_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<UnitPvpFlags> {
        if self.player_guid() == Some(guid)
            && let Some(flags) =
                self.with_owned_player_like_cpp(|player| player.unit().pvp_flags_like_cpp())
        {
            return Some(flags);
        }
        let map_id = u32::from(self.player_map_id_like_cpp());
        let manager = Arc::clone(self.canonical_map_manager.as_ref()?);
        let manager = manager.lock().ok()?;
        let mut result = None;
        manager.do_for_all_maps_with_map_id(map_id, |managed| {
            if result.is_none() {
                result = managed
                    .map()
                    .get_typed_player(guid)
                    .map(|player| player.unit().pvp_flags_like_cpp());
            }
        });
        result
    }
    pub(in crate::session) fn player_has_in_pvp_flag_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<bool> {
        if self.player_guid() != Some(guid) {
            return None;
        }
        let canonical = self.with_owned_player_like_cpp(|player| {
            player.has_player_flag(PLAYER_FLAGS_IN_PVP_LIKE_CPP)
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            if let Some(value) =
                self.canonical_player_has_player_flag_like_cpp(guid, PLAYER_FLAGS_IN_PVP_LIKE_CPP)
            {
                return Some(value);
            }
            return Some(self.player_in_pvp_flag_like_cpp);
        }
        canonical
    }
    pub(in crate::session) fn update_player_pvp_like_cpp(
        &mut self,
        state: bool,
        override_state: bool,
    ) {
        let end_timer = if !state || override_state {
            None
        } else {
            Some(wow_entities::game_time_secs_like_cpp())
        };
        #[cfg_attr(not(test), allow(unused_mut))]
        let mut mutated = self.with_owned_player_mut_like_cpp(|player| {
            player.gameplay_state_mut().world_local.pvp_end_timer = end_timer;
            if state {
                player.unit_mut().set_pvp_flag_like_cpp(UnitPvpFlags::PVP);
            } else {
                player
                    .unit_mut()
                    .remove_pvp_flag_like_cpp(UnitPvpFlags::PVP);
            }
        });
        #[cfg(test)]
        if mutated.is_none()
            && self.player_handle_like_cpp.is_none()
            && let Some(guid) = self.player_guid()
        {
            mutated = self.mutate_canonical_player_by_guid_like_cpp(guid, |player| {
                player.gameplay_state_mut().world_local.pvp_end_timer = end_timer;
                if state {
                    player.unit_mut().set_pvp_flag_like_cpp(UnitPvpFlags::PVP);
                } else {
                    player
                        .unit_mut()
                        .remove_pvp_flag_like_cpp(UnitPvpFlags::PVP);
                }
            });
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.player_pvp_end_timer_like_cpp = end_timer;
            self.player_pvp_enabled_like_cpp = state;
        }
        let _ = mutated;
    }
    pub(crate) fn update_pvp_flag_like_cpp(&mut self, curr_time: i64) {
        let Some(guid) = self.player_guid() else {
            return;
        };

        if self.player_is_pvp_like_cpp(guid) != Some(true) {
            return;
        }

        let Some(state) = self.player_world_local_state_like_cpp() else {
            return;
        };
        let Some(end_timer) = state.pvp_end_timer else {
            return;
        };

        if curr_time < end_timer.saturating_add(300) || state.pvp_hostile {
            return;
        }

        if end_timer <= curr_time {
            #[cfg_attr(not(test), allow(unused_mut))]
            let mut mutated = self.with_owned_player_mut_like_cpp(|player| {
                player.gameplay_state_mut().world_local.pvp_end_timer = None;
                player.remove_player_flag(PLAYER_FLAGS_PVP_TIMER_LIKE_CPP);
            });
            #[cfg(test)]
            if mutated.is_none() && self.player_handle_like_cpp.is_none() {
                mutated = self.mutate_canonical_player_by_guid_like_cpp(guid, |player| {
                    player.gameplay_state_mut().world_local.pvp_end_timer = None;
                    player.remove_player_flag(PLAYER_FLAGS_PVP_TIMER_LIKE_CPP);
                });
            }
            #[cfg(test)]
            if self.player_handle_like_cpp.is_none() {
                self.player_pvp_end_timer_like_cpp = None;
            }
            let _ = mutated;
        }

        self.update_player_pvp_like_cpp(false, false);
        self.sync_player_registry_state_like_cpp();
    }
    pub(crate) fn apply_toggle_pvp_like_cpp(&mut self) {
        let Some(guid) = self.player_guid() else {
            return;
        };
        let Some(in_pvp) = self.player_has_in_pvp_flag_like_cpp(guid) else {
            return;
        };
        self.apply_set_pvp_like_cpp(!in_pvp);
    }
    pub(crate) fn apply_set_pvp_like_cpp(&mut self, enable_pvp: bool) {
        let Some(guid) = self.player_guid() else {
            return;
        };

        if enable_pvp {
            #[cfg_attr(not(test), allow(unused_mut))]
            let mut mutated = self.with_owned_player_mut_like_cpp(|player| {
                player.set_player_flag(PLAYER_FLAGS_IN_PVP_LIKE_CPP);
                player.remove_player_flag(PLAYER_FLAGS_PVP_TIMER_LIKE_CPP);
            });
            #[cfg(test)]
            if mutated.is_none() && self.player_handle_like_cpp.is_none() {
                mutated = self.mutate_canonical_player_by_guid_like_cpp(guid, |player| {
                    player.set_player_flag(PLAYER_FLAGS_IN_PVP_LIKE_CPP);
                    player.remove_player_flag(PLAYER_FLAGS_PVP_TIMER_LIKE_CPP);
                });
                self.player_in_pvp_flag_like_cpp = true;
            }
            let _ = mutated;

            if self.player_is_pvp_like_cpp(guid) == Some(false)
                || self
                    .player_world_local_state_like_cpp()
                    .is_some_and(|state| state.pvp_end_timer.is_some())
            {
                self.update_player_pvp_like_cpp(true, true);
            }
        } else if !self.player_war_mode_local_active_like_cpp() {
            #[cfg_attr(not(test), allow(unused_mut))]
            let mut mutated = self.with_owned_player_mut_like_cpp(|player| {
                player.remove_player_flag(PLAYER_FLAGS_IN_PVP_LIKE_CPP);
                player.set_player_flag(PLAYER_FLAGS_PVP_TIMER_LIKE_CPP);
            });
            #[cfg(test)]
            if mutated.is_none() && self.player_handle_like_cpp.is_none() {
                mutated = self.mutate_canonical_player_by_guid_like_cpp(guid, |player| {
                    player.remove_player_flag(PLAYER_FLAGS_IN_PVP_LIKE_CPP);
                    player.set_player_flag(PLAYER_FLAGS_PVP_TIMER_LIKE_CPP);
                });
                self.player_in_pvp_flag_like_cpp = false;
            }
            let _ = mutated;

            let Some(state) = self.player_world_local_state_like_cpp() else {
                return;
            };
            if !state.pvp_hostile && self.player_is_pvp_like_cpp(guid) == Some(true) {
                let now = wow_entities::game_time_secs_like_cpp();
                let _ = self.mutate_player_world_local_state_like_cpp(|state| {
                    state.pvp_end_timer = Some(now);
                });
            }
        }

        self.sync_player_registry_state_like_cpp();
    }
    pub(in crate::session) fn begin_canonical_player_combat_ref_like_cpp(
        &mut self,
        attacker_guid: ObjectGuid,
        victim_guid: ObjectGuid,
        relation_represented: bool,
        attacker_is_friendly_to_victim: bool,
        victim_is_friendly_to_attacker: bool,
    ) -> bool {
        let Some(map_key) = self.current_canonical_player_map_key_like_cpp() else {
            return false;
        };
        let Some(manager) = self.canonical_map_manager.as_ref().cloned() else {
            return false;
        };
        let Ok(mut manager) = manager.lock() else {
            return false;
        };
        let Some(managed) = manager.find_map_mut(map_key.map_id, map_key.instance_id) else {
            return false;
        };
        begin_combat_ref_on_map_like_cpp(
            managed.map_mut(),
            attacker_guid,
            victim_guid,
            relation_represented,
            attacker_is_friendly_to_victim,
            victim_is_friendly_to_attacker,
        )
    }
    pub(in crate::session) fn revalidate_canonical_player_combat_refs_like_cpp(
        &mut self,
        player_guid: ObjectGuid,
    ) {
        let Some(map_key) = self.current_canonical_player_map_key_like_cpp() else {
            return;
        };
        let Some(manager) = self.canonical_map_manager.as_ref().cloned() else {
            return;
        };
        let Ok(mut manager) = manager.lock() else {
            return;
        };
        let Some(managed) = manager.find_map_mut(map_key.map_id, map_key.instance_id) else {
            return;
        };
        if managed.map().get_typed_player(player_guid).is_some() {
            managed.map_mut().revalidate_all_combat_refs_like_cpp();
        }
    }
    pub(in crate::session) fn combat_stop_like_cpp(&mut self) {
        let Some(player_guid) = self.player_guid() else {
            self.set_combat_target_like_cpp(None);
            self.set_in_combat_like_cpp(false);
            return;
        };

        let stopped_target = self.stop_player_attack_like_cpp();
        let owner_guids = {
            let Some(manager) = self.canonical_map_manager.as_ref().cloned() else {
                return self.finish_combat_stop_like_cpp(player_guid, stopped_target, Vec::new());
            };
            let Ok(mut manager) = manager.lock() else {
                return self.finish_combat_stop_like_cpp(player_guid, stopped_target, Vec::new());
            };
            let Some(managed) = manager.find_map_mut(u32::from(self.player_map_id_like_cpp()), 0)
            else {
                drop(manager);
                return self.finish_combat_stop_like_cpp(player_guid, stopped_target, Vec::new());
            };
            let map = managed.map_mut();
            let Some(player) = map.get_typed_player_mut(player_guid) else {
                drop(manager);
                return self.finish_combat_stop_like_cpp(player_guid, stopped_target, Vec::new());
            };

            let mut owner_guids: Vec<ObjectGuid> = player
                .unit()
                .subsystems()
                .combat
                .pve_refs
                .keys()
                .chain(player.unit().subsystems().combat.pvp_refs.keys())
                .chain(player.unit().subsystems().combat.attackers.iter())
                .copied()
                .collect();
            owner_guids.sort_unstable();
            owner_guids.dedup();

            player.unit_mut().subsystems_mut().combat.end_all_combat();
            player.unit_mut().subsystems_mut().combat.clear_attackers();

            for owner_guid in &owner_guids {
                if let Some(owner) = map.get_typed_player_mut(*owner_guid) {
                    if owner.unit().attacking() == Some(player_guid) {
                        let _ = owner.unit_mut().attack_stop_like_cpp();
                    }
                    owner
                        .unit_mut()
                        .subsystems_mut()
                        .combat
                        .purge_combat_ref_like_cpp(player_guid);
                    owner.unit_mut().remove_attacker_like_cpp(player_guid);
                } else if let Some(owner) = map.get_typed_creature_mut(*owner_guid) {
                    if owner.unit().attacking() == Some(player_guid) {
                        let _ = owner.unit_mut().attack_stop_like_cpp();
                    }
                    owner
                        .unit_mut()
                        .subsystems_mut()
                        .combat
                        .purge_combat_ref_like_cpp(player_guid);
                    owner.unit_mut().remove_attacker_like_cpp(player_guid);
                }
            }

            owner_guids
        };

        self.finish_combat_stop_like_cpp(player_guid, stopped_target, owner_guids);
    }
    fn finish_combat_stop_like_cpp(
        &mut self,
        player_guid: ObjectGuid,
        stopped_target: Option<ObjectGuid>,
        owner_guids: Vec<ObjectGuid>,
    ) {
        self.set_combat_target_like_cpp(None);
        self.set_in_combat_like_cpp(false);
        for owner_guid in owner_guids {
            let _ = self.mutate_world_creature(owner_guid, |owner| {
                if owner.creature.unit().attacking() == Some(player_guid) {
                    let _ = owner.creature.unit_mut().attack_stop_like_cpp();
                }
                owner
                    .creature
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .purge_combat_ref_like_cpp(player_guid);
                owner
                    .creature
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .scale_threat(player_guid, 0.0);
                owner
                    .creature
                    .unit_mut()
                    .remove_attacker_like_cpp(player_guid);
                if owner.creature.ai_ownership().combat_target == Some(player_guid) {
                    owner.creature.ai_ownership_mut().combat_target = None;
                }
            });
        }

        if let Some(target) = stopped_target {
            self.send_packet(&wow_packet::packets::combat::SAttackStop {
                attacker: player_guid,
                victim: target,
                now_dead: false,
            });
        }
        self.send_packet(&wow_packet::packets::combat::CancelCombat);
        self.sync_player_registry_state_like_cpp();
    }
    pub fn set_combat_ratings_game_table(&mut self, table: Arc<CombatRatingsGameTableLikeCpp>) {
        self.combat_ratings_game_table = Some(table);
    }
    pub(crate) fn combat_rating_multiplier_like_cpp(&self, level: u8, rating: u32) -> f32 {
        self.combat_ratings_game_table
            .as_ref()
            .map(|table| table.rating_multiplier_like_cpp(u16::from(level), rating))
            .unwrap_or(1.0)
    }
    pub(in crate::session) fn represented_has_pvp_rules_enabled_like_cpp(&self) -> bool {
        self.player_has_visible_aura_spell_like_cpp(SPELL_PVP_RULES_ENABLED_LIKE_CPP)
            .unwrap_or(false)
    }
    pub(in crate::session) fn reset_contested_pvp_like_cpp(&mut self) {
        let Some(_guid) = self.player_guid() else {
            return;
        };

        #[cfg_attr(not(test), allow(unused_mut))]
        let mut mutated = self.with_owned_player_mut_like_cpp(|player| {
            player
                .unit_mut()
                .clear_unit_state(UnitState::ATTACK_PLAYER.bits());
            player.remove_player_flag(PLAYER_FLAGS_CONTESTED_PVP_LIKE_CPP);
            player.gameplay_state_mut().world_local.contested_pvp_timer = 0;
        });
        #[cfg(test)]
        if mutated.is_none() && self.player_handle_like_cpp.is_none() {
            mutated = self.mutate_canonical_player_by_guid_like_cpp(_guid, |player| {
                player
                    .unit_mut()
                    .clear_unit_state(UnitState::ATTACK_PLAYER.bits());
                player.remove_player_flag(PLAYER_FLAGS_CONTESTED_PVP_LIKE_CPP);
                player.gameplay_state_mut().world_local.contested_pvp_timer = 0;
            });
            self.player_contested_pvp_timer_like_cpp = 0;
        }
        let _ = mutated;
        self.sync_player_registry_state_like_cpp();
    }
    /// Mirror `Unit::SetPvpFlag` / `RemovePvpFlag` for the realm-wide FFA bit.
    ///
    /// The canonical player remains the runtime authority. The isolated delta
    /// keeps this direct owner update limited to `UnitData::PvpFlags` without
    /// clearing or leaking any other dirty canonical fields that the map tick
    /// still owns.
    pub(in crate::session) fn set_represented_ffa_pvp_flag_like_cpp(
        &mut self,
        enabled: bool,
    ) -> bool {
        let update = self
            .mutate_canonical_player_like_cpp(|player| {
                let before = player.unit().pvp_flags_like_cpp();
                if before.contains(UnitPvpFlags::FFA_PVP) == enabled {
                    return None;
                }

                if enabled {
                    player
                        .unit_mut()
                        .set_pvp_flag_like_cpp(UnitPvpFlags::FFA_PVP);
                } else {
                    player
                        .unit_mut()
                        .remove_pvp_flag_like_cpp(UnitPvpFlags::FFA_PVP);
                }
                let after = player.unit().pvp_flags_like_cpp();

                let mut delta = Player::new(None, false);
                delta.unit_mut().replace_all_pvp_flags_like_cpp(before);
                delta.clear_data_changes();
                delta.unit_mut().replace_all_pvp_flags_like_cpp(after);
                Some(delta.values_update(true))
            })
            .flatten();
        let Some(update) = update else {
            return false;
        };

        self.send_player_values_update_like_cpp(&update);
        self.sync_player_registry_state_like_cpp();
        true
    }
    /// C++ `HandlePlayerLogin`: realm-wide FFA is enabled after the player is
    /// in the map, except for GMs and players whose loaded/zone-restored flags
    /// already say they are resting.
    pub(in crate::session) fn apply_represented_ffa_pvp_login_state_like_cpp(&mut self) -> bool {
        if !self.is_ffa_pvp_realm_like_cpp
            || self.player_is_game_master_like_cpp() != Some(false)
            || self.resolved_visible_resting_like_cpp() != Some(false)
        {
            return false;
        }

        self.set_represented_ffa_pvp_flag_like_cpp(true)
    }
    pub fn set_pvp_realm_like_cpp(&mut self, is_pvp_realm: bool) {
        self.is_pvp_realm_like_cpp = is_pvp_realm;
    }
    pub fn set_ffa_pvp_realm_like_cpp(&mut self, is_ffa_pvp_realm: bool) {
        self.is_ffa_pvp_realm_like_cpp = is_ffa_pvp_realm;
    }
    pub(crate) fn resolved_combat_target_like_cpp(&self) -> Option<Option<ObjectGuid>> {
        let canonical = self.with_owned_player_like_cpp(|player| player.unit().attacking());
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.combat_target);
        }
        canonical
    }
    pub(crate) fn set_combat_target_like_cpp(&mut self, target: Option<ObjectGuid>) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| player.unit_mut().set_attacking(target))
            .is_some();
        #[cfg(test)]
        if canonical || self.player_handle_like_cpp.is_none() {
            self.combat_target = target;
        }
        canonical || cfg!(test) && self.player_handle_like_cpp.is_none()
    }
    pub(crate) fn resolved_in_combat_like_cpp(&self) -> Option<bool> {
        let canonical = self
            .with_owned_player_like_cpp(|player| player.unit().subsystems().combat.has_combat());
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.in_combat);
        }
        canonical
    }
    /// Publish C++ `CombatManager::HasCombat` from the canonical Player to the
    /// bounded directory view. The argument remains only for pre-owner tests;
    /// production never manufactures combat state outside `CombatSubsystem`.
    pub(crate) fn set_in_combat_like_cpp(&mut self, in_combat: bool) {
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.in_combat = in_combat;
            if let (Some(guid), Some(registry)) = (self.player_guid(), &self.player_registry) {
                registry.publish_in_combat_for_control_channel(
                    guid,
                    &self.session_command_tx,
                    in_combat,
                );
            }
            return;
        }
        let canonical = self.resolved_in_combat_like_cpp();
        #[cfg(not(test))]
        let _ = in_combat;
        let Some(in_combat) = canonical else {
            return;
        };
        if let (Some(guid), Some(registry)) = (self.player_guid(), &self.player_registry) {
            registry.publish_in_combat_for_control_channel(
                guid,
                &self.session_command_tx,
                in_combat,
            );
        }
    }
    #[cfg(test)]
    pub(crate) fn represented_combat_stat_recalculations_like_cpp(
        &self,
    ) -> &[RepresentedCombatStatRecalculationLikeCpp] {
        &self.represented_combat_stat_recalculations_like_cpp
    }
    pub(crate) fn represented_set_advanced_combat_logging_like_cpp(&mut self, enable: bool) {
        self.advanced_combat_logging_enabled_like_cpp
            .store(enable, Ordering::Relaxed);
    }
    pub(crate) fn represented_advanced_combat_logging_enabled_like_cpp(&self) -> bool {
        self.advanced_combat_logging_enabled_like_cpp
            .load(Ordering::Relaxed)
    }
    #[cfg(test)]
    pub(crate) fn set_player_pvp_hostile_like_cpp(&mut self, hostile: bool) {
        let _ = self.mutate_player_world_local_state_like_cpp(|state| {
            state.pvp_hostile = hostile;
        });
    }
    #[cfg(test)]
    pub(crate) fn set_player_pvp_state_like_cpp(
        &mut self,
        hostile: bool,
        pvp_enabled: bool,
        in_pvp_flag: bool,
    ) {
        self.set_player_pvp_hostile_like_cpp(hostile);
        self.update_player_pvp_like_cpp(pvp_enabled, true);
        if let Some(guid) = self.player_guid() {
            let _ = self.with_owned_player_mut_like_cpp(|player| {
                if in_pvp_flag {
                    player.set_player_flag(PLAYER_FLAGS_IN_PVP_LIKE_CPP);
                } else {
                    player.remove_player_flag(PLAYER_FLAGS_IN_PVP_LIKE_CPP);
                }
            });
            let _ = guid;
        }
        if self.player_handle_like_cpp.is_none() {
            self.player_pvp_enabled_like_cpp = pvp_enabled;
            self.player_in_pvp_flag_like_cpp = in_pvp_flag;
        }
    }
}
