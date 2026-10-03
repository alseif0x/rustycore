//! Represented combat state and PvP flags at the Session boundary.
//!
//! Moved out of the Session root under #617. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn update_pvp_flag_like_cpp(&mut self, curr_time: i64) {
        let Some(guid) = self.player_guid() else {
            return;
        };

        if crate::session::hub_ref(self).player_is_pvp_like_cpp(guid) != Some(true) {
            return;
        }

        let Some(state) = crate::session::hub_ref(self).player_world_local_state_like_cpp() else {
            return;
        };
        let Some(end_timer) = state.pvp_end_timer_like_cpp() else {
            return;
        };

        if curr_time < end_timer.saturating_add(300) || state.is_pvp_hostile_like_cpp() {
            return;
        }

        if end_timer <= curr_time {
            #[cfg_attr(not(test), allow(unused_mut))]
            let mut mutated = self.core.with_owned_player_mut_like_cpp(|player| {
                player.expire_pvp_timer_like_cpp();
            });
            #[cfg(test)]
            if mutated.is_none() && self.core.player_handle_like_cpp.is_none() {
                mutated = self
                    .core
                    .mutate_canonical_player_by_guid_like_cpp(guid, |player| {
                        player.expire_pvp_timer_like_cpp();
                    });
            }
            #[cfg(test)]
            if self.core.player_handle_like_cpp.is_none() {
                self.fixtures.combat.player_pvp_end_timer_like_cpp = None;
            }
            let _ = mutated;
        }

        crate::session::hub_mut(self).update_player_pvp_like_cpp(false, false);
        self.sync_player_registry_state_like_cpp();
    }
    pub(crate) fn apply_toggle_pvp_like_cpp(&mut self) {
        let Some(guid) = self.player_guid() else {
            return;
        };
        let Some(in_pvp) = crate::session::hub_ref(self).player_has_in_pvp_flag_like_cpp(guid)
        else {
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
            let mut mutated = self.core.with_owned_player_mut_like_cpp(|player| {
                player.set_player_flag(PLAYER_FLAGS_IN_PVP_LIKE_CPP);
                player.remove_player_flag(PLAYER_FLAGS_PVP_TIMER_LIKE_CPP);
            });
            #[cfg(test)]
            if mutated.is_none() && self.core.player_handle_like_cpp.is_none() {
                mutated = self
                    .core
                    .mutate_canonical_player_by_guid_like_cpp(guid, |player| {
                        player.set_player_flag(PLAYER_FLAGS_IN_PVP_LIKE_CPP);
                        player.remove_player_flag(PLAYER_FLAGS_PVP_TIMER_LIKE_CPP);
                    });
                self.fixtures.combat.player_in_pvp_flag_like_cpp = true;
            }
            let _ = mutated;

            if crate::session::hub_ref(self).player_is_pvp_like_cpp(guid) == Some(false)
                || crate::session::hub_ref(self)
                    .player_world_local_state_like_cpp()
                    .is_some_and(|state| state.pvp_end_timer_like_cpp().is_some())
            {
                crate::session::hub_mut(self).update_player_pvp_like_cpp(true, true);
            }
        } else if !crate::session::hub_ref(self).player_war_mode_local_active_like_cpp() {
            #[cfg_attr(not(test), allow(unused_mut))]
            let mut mutated = self.core.with_owned_player_mut_like_cpp(|player| {
                player.remove_player_flag(PLAYER_FLAGS_IN_PVP_LIKE_CPP);
                player.set_player_flag(PLAYER_FLAGS_PVP_TIMER_LIKE_CPP);
            });
            #[cfg(test)]
            if mutated.is_none() && self.core.player_handle_like_cpp.is_none() {
                mutated = self
                    .core
                    .mutate_canonical_player_by_guid_like_cpp(guid, |player| {
                        player.remove_player_flag(PLAYER_FLAGS_IN_PVP_LIKE_CPP);
                        player.set_player_flag(PLAYER_FLAGS_PVP_TIMER_LIKE_CPP);
                    });
                self.fixtures.combat.player_in_pvp_flag_like_cpp = false;
            }
            let _ = mutated;

            let Some(state) = crate::session::hub_ref(self).player_world_local_state_like_cpp()
            else {
                return;
            };
            if !state.is_pvp_hostile_like_cpp()
                && crate::session::hub_ref(self).player_is_pvp_like_cpp(guid) == Some(true)
            {
                let now = wow_entities::game_time_secs_like_cpp();
                let _ = crate::session::hub_mut(self).set_player_pvp_end_timer_like_cpp(Some(now));
            }
        }

        self.sync_player_registry_state_like_cpp();
    }
    pub(in crate::session) fn combat_stop_like_cpp(&mut self) {
        let Some(player_guid) = self.player_guid() else {
            crate::session::hub_mut(self).set_combat_target_like_cpp(None);
            crate::session::hub_mut(self).set_in_combat_like_cpp(false);
            return;
        };

        let stopped_target = crate::session::hub_mut(self).stop_player_attack_like_cpp();
        let owner_guids = {
            let Some(manager) = self.core.canonical_map_manager.as_ref().cloned() else {
                return self.finish_combat_stop_like_cpp(player_guid, stopped_target, Vec::new());
            };
            let Ok(mut manager) = manager.lock() else {
                return self.finish_combat_stop_like_cpp(player_guid, stopped_target, Vec::new());
            };
            let Some(managed) =
                manager.find_map_mut(u32::from(self.core.player_map_id_like_cpp()), 0)
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
        crate::session::hub_mut(self).set_combat_target_like_cpp(None);
        crate::session::hub_mut(self).set_in_combat_like_cpp(false);
        for owner_guid in owner_guids {
            let _ = self.core.mutate_world_creature(owner_guid, |owner| {
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
        self.catalogs.combat_ratings_game_table = Some(table);
    }

    pub fn set_regen_game_tables(&mut self, tables: Arc<RegenGameTablesLikeCpp>) {
        self.catalogs.regen_game_tables = Some(tables);
    }

    pub(in crate::session) fn reset_contested_pvp_like_cpp(&mut self) {
        let Some(_guid) = self.player_guid() else {
            return;
        };

        #[cfg_attr(not(test), allow(unused_mut))]
        let mut mutated = self.core.with_owned_player_mut_like_cpp(|player| {
            player.clear_contested_pvp_like_cpp();
        });
        #[cfg(test)]
        if mutated.is_none() && self.core.player_handle_like_cpp.is_none() {
            mutated = self
                .core
                .mutate_canonical_player_by_guid_like_cpp(_guid, |player| {
                    player.clear_contested_pvp_like_cpp();
                });
            self.fixtures.combat.player_contested_pvp_timer_like_cpp = 0;
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
            .core
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

        self.core.send_player_values_update_like_cpp(&update);
        self.sync_player_registry_state_like_cpp();
        true
    }
    /// C++ `HandlePlayerLogin`: realm-wide FFA is enabled after the player is
    /// in the map, except for GMs and players whose loaded/zone-restored flags
    /// already say they are resting.
    pub(in crate::session) fn apply_represented_ffa_pvp_login_state_like_cpp(&mut self) -> bool {
        if !self.view.is_ffa_pvp_realm_like_cpp
            || crate::session::hub_ref(self).player_is_game_master_like_cpp() != Some(false)
            || self.resolved_visible_resting_like_cpp() != Some(false)
        {
            return false;
        }

        self.set_represented_ffa_pvp_flag_like_cpp(true)
    }
    pub fn set_pvp_realm_like_cpp(&mut self, is_pvp_realm: bool) {
        self.view.is_pvp_realm_like_cpp = is_pvp_realm;
    }
    pub fn set_ffa_pvp_realm_like_cpp(&mut self, is_ffa_pvp_realm: bool) {
        self.view.is_ffa_pvp_realm_like_cpp = is_ffa_pvp_realm;
    }
    #[cfg(test)]
    pub(crate) fn represented_combat_stat_recalculations_like_cpp(
        &self,
    ) -> &[RepresentedCombatStatRecalculationLikeCpp] {
        self.inventory
            .represented_combat_stat_recalculations_for_test_like_cpp()
    }
}

#[cfg(test)]
#[path = "../../../unit_tests/session/combat/state/f3_shims.rs"]
mod f3_shims;
