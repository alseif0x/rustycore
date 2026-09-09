//! Represented area, zone, trigger and weather state.
//!
//! Moved out of the Session root under #632. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    /// Set the DB2-backed area trigger store for this session.
    #[cfg(test)]
    pub fn set_area_trigger_db2_store(&mut self, store: Arc<AreaTriggerDb2Store>) {
        self.area_trigger_db2_store = Some(store);
    }
    /// Set the area trigger teleport store for this session.
    #[cfg(test)]
    pub fn set_area_trigger_store(&mut self, store: Arc<AreaTriggerStore>) {
        self.area_trigger_store = Some(store);
    }
    #[cfg(test)]
    pub fn set_area_trigger_script_store(&mut self, store: Arc<AreaTriggerScriptStoreLikeCpp>) {
        self.area_trigger_script_store = Some(store);
    }
    #[cfg(test)]
    pub fn set_area_trigger_script_dispatcher_like_cpp(
        &mut self,
        dispatcher: AreaTriggerScriptDispatcherLikeCpp,
    ) {
        self.area_trigger_script_dispatcher_like_cpp = Some(dispatcher);
    }
    pub(crate) fn dispatch_area_trigger_script_like_cpp(
        &mut self,
        dispatcher: Option<&AreaTriggerScriptDispatcherLikeCpp>,
        script_id: ScriptIdLikeCpp,
        trigger_id: u32,
        entered: bool,
    ) -> Option<bool> {
        let dispatcher = Arc::clone(dispatcher?);
        Some(dispatcher(self, script_id, trigger_id, entered))
    }
    #[cfg(test)]
    pub fn set_tavern_area_trigger_store(&mut self, store: Arc<TavernAreaTriggerStoreLikeCpp>) {
        self.tavern_area_trigger_store = Some(store);
    }
    pub(crate) fn represented_is_tavern_area_trigger_like_cpp(
        &self,
        taverns: &TavernAreaTriggerStoreLikeCpp,
        trigger_id: u32,
    ) -> bool {
        taverns.is_tavern_area_trigger_like_cpp(trigger_id)
    }
    /// C++ `Player::IsInAreaTriggerRadius`.
    pub(crate) fn player_is_in_area_trigger_radius_like_cpp(
        &self,
        trigger: &wow_data::AreaTriggerDb2Entry,
    ) -> bool {
        let Some(pos) = self.player_position_like_cpp() else {
            return false;
        };

        let Some(trigger_map_id) = u16::try_from(trigger.continent_id).ok() else {
            return false;
        };
        let Some(player_phase_shift) = self.represented_player_phase_shift_like_cpp() else {
            return false;
        };
        if self.player_map_id_like_cpp() != trigger_map_id
            && !player_phase_shift.has_visible_map_id_like_cpp(u32::from(trigger_map_id))
        {
            return false;
        }

        if trigger.phase_id != 0 || trigger.phase_group_id != 0 || trigger.phase_use_flags != 0 {
            let (trigger_phase_shift, _) = self.db_spawn_phase_shift_like_cpp(
                trigger_map_id,
                trigger.phase_use_flags as u8,
                u16::try_from(trigger.phase_id).unwrap_or_default(),
                u32::try_from(trigger.phase_group_id).unwrap_or_default(),
                -1,
            );
            if !self.can_see_phase_shift_like_cpp(&trigger_phase_shift) {
                return false;
            }
        }

        let center = Position::new(trigger.pos.x, trigger.pos.y, trigger.pos.z, trigger.box_yaw);
        if trigger.radius > 0.0 {
            pos.is_within_dist(&center, trigger.radius)
        } else {
            Self::position_is_within_area_trigger_box_like_cpp(
                &pos,
                &center,
                trigger.box_length / 2.0,
                trigger.box_width / 2.0,
                trigger.box_height / 2.0,
            )
        }
    }
    pub fn set_area_table_store(&mut self, store: Arc<AreaTableStore>) {
        self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        self.area_table_store = Some(store);
    }
    pub(crate) fn area_table_store(&self) -> Option<&Arc<AreaTableStore>> {
        self.area_table_store.as_ref()
    }
    fn update_represented_hostile_area_state_like_cpp(&mut self, zone: &wow_data::AreaTableEntry) {
        let war_mode_active = self.player_war_mode_local_active_like_cpp();
        let zone_hostile = if zone.is_sanctuary_like_cpp() {
            false
        } else if (zone.flags
            & (AREA_FLAG_FREE_FOR_ALL_PVP_LIKE_CPP | AREA_FLAG_COMBAT_ZONE_LIKE_CPP))
            != 0
        {
            true
        } else if (zone.flags & AREA_FLAG_ENEMIES_PVP_FLAGGED_LIKE_CPP) != 0 {
            if (zone.flags & AREA_FLAG_CONTESTED_LIKE_CPP) != 0 {
                war_mode_active
            } else {
                let faction_group_mask = self
                    .area_table_store
                    .as_ref()
                    .map(|store| store.faction_group_mask_like_cpp(zone.id))
                    .unwrap_or(0);
                self.player_faction_template_id_like_cpp()
                    .and_then(|id| {
                        self.faction_template_store
                            .as_ref()
                            .and_then(|store| store.get(id))
                    })
                    .is_some_and(|faction_template| {
                        if (faction_template.friend_group & faction_group_mask) != 0 {
                            false
                        } else if (faction_template.enemy_group & faction_group_mask) != 0 {
                            true
                        } else {
                            self.is_pvp_realm_like_cpp
                        }
                    })
            }
        } else {
            false
        };
        let _ = self.mutate_player_world_local_state_like_cpp(|state| {
            state.pvp_hostile = zone_hostile || war_mode_active;
        });
    }
    pub(crate) fn handle_represented_tavern_area_trigger_with_catalog_like_cpp(
        &mut self,
        taverns: &TavernAreaTriggerStoreLikeCpp,
        trigger_id: u32,
        entered: bool,
    ) -> bool {
        if !self.represented_is_tavern_area_trigger_like_cpp(taverns, trigger_id) {
            return false;
        }

        if self.set_represented_tavern_resting_like_cpp(trigger_id, entered) {
            self.send_represented_resting_player_flag_update_like_cpp();
        }
        if self.is_ffa_pvp_realm_like_cpp {
            // C++ `MiscHandler.cpp::HandleAreaTriggerOpcode` toggles FFA
            // directly from `packet.Entered`, independently of RestMgr's
            // aggregate mask. Leaving an inn can therefore restore FFA while
            // a city/faction rest flag still keeps PLAYER_FLAGS_RESTING set.
            self.set_represented_ffa_pvp_flag_like_cpp(!entered);
        }
        true
    }
    #[cfg(test)]
    pub(crate) fn handle_represented_tavern_area_trigger_like_cpp(
        &mut self,
        trigger_id: u32,
        entered: bool,
    ) -> bool {
        let taverns = self
            .tavern_area_trigger_store
            .clone()
            .unwrap_or_else(|| Arc::new(TavernAreaTriggerStoreLikeCpp::default()));
        self.handle_represented_tavern_area_trigger_with_catalog_like_cpp(
            taverns.as_ref(),
            trigger_id,
            entered,
        )
    }
    pub(crate) fn player_explored_zones_snapshot_like_cpp(
        &self,
    ) -> Option<[u64; PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP]> {
        let canonical =
            self.with_owned_player_like_cpp(|player| *player.explored_zones_blocks_like_cpp());
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.represented_explored_zones_like_cpp);
        }
        canonical
    }
    pub(crate) fn represented_explored_zones_db_string_like_cpp(&self) -> Option<String> {
        Some(explored_zones_db_string_from_blocks_like_cpp(
            &self.player_explored_zones_snapshot_like_cpp()?,
        ))
    }
    /// Represented C++ `Player::UpdateArea` criteria branch.
    ///
    /// C++ records `EnterArea`/`LeaveArea` after updating area-dependent state when
    /// `oldArea != newArea`; this represented slice records those criteria and
    /// the C++ area rest flag side effects. PvP flags, phasing, aura checks, quest push, mount
    /// capability refresh, and chat-channel updates remain runtime gaps.
    pub(crate) fn update_area_represented_like_cpp(&mut self, new_area: u32) -> bool {
        self.update_area_represented_with_rest_update_like_cpp(new_area, true)
    }
    fn update_area_represented_with_rest_update_like_cpp(
        &mut self,
        new_area: u32,
        send_rest_update: bool,
    ) -> bool {
        let Some(world_local) = self.player_world_local_state_like_cpp() else {
            return false;
        };
        let old_area = world_local.area_id;
        if old_area != new_area {
            self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        }
        if self
            .mutate_player_world_local_state_like_cpp(|state| {
                if old_area != new_area {
                    state.zone_area_authority_complete = false;
                }
                state.area_id = new_area;
            })
            .is_none()
        {
            return false;
        }
        let zone_id = world_local.zone_id;
        let _ = self.with_owned_player_mut_like_cpp(|player| {
            player
                .unit_mut()
                .world_mut()
                .set_zone_and_area(zone_id, new_area);
        });

        let mut rest_changed = false;
        let area_resting = self.area_table_store.as_ref().and_then(|store| {
            store.get(new_area).map(|area| {
                let team = player_team_for_race_cpp(self.player_race_like_cpp());
                match team {
                    Team::Alliance => area.alliance_resting_like_cpp(),
                    Team::Horde | Team::Other => area.horde_resting_like_cpp(),
                }
            })
        });
        rest_changed |= self.update_represented_rest_flag_like_cpp(
            REST_FLAG_IN_FACTION_AREA_LIKE_CPP,
            area_resting.unwrap_or(false),
        );
        if send_rest_update && rest_changed {
            self.send_represented_resting_player_flag_update_like_cpp();
        }

        if old_area == new_area {
            return false;
        }

        #[cfg(test)]
        {
            self.represented_area_zone_criteria_like_cpp
                .push(RepresentedAreaZoneCriteriaLikeCpp::EnterArea(new_area));
            self.represented_area_zone_criteria_like_cpp
                .push(RepresentedAreaZoneCriteriaLikeCpp::LeaveArea(old_area));
        }
        true
    }
    /// Represented C++ `Player::UpdateZone` criteria branch.
    ///
    /// C++ first updates `m_zoneUpdateId`, then calls `UpdateArea(newArea)`, then
    /// returns early if the new zone has no `AreaTableEntry`. Therefore top-level
    /// area criteria are recorded only when the zone changes and the new zone row
    /// exists, while area criteria may already have been recorded by `UpdateArea`.
    pub(crate) fn update_zone_represented_like_cpp(
        &mut self,
        new_zone: u32,
        new_area: u32,
    ) -> bool {
        self.update_zone_represented_with_rest_update_like_cpp(new_zone, new_area, true)
    }
    pub(crate) fn update_zone_represented_without_rest_update_packet_like_cpp(
        &mut self,
        new_zone: u32,
        new_area: u32,
    ) -> bool {
        self.update_zone_represented_with_rest_update_like_cpp(new_zone, new_area, false)
    }
    fn update_zone_represented_with_rest_update_like_cpp(
        &mut self,
        new_zone: u32,
        new_area: u32,
        send_rest_update: bool,
    ) -> bool {
        if self.player_guid().is_none() {
            return false;
        }

        let Some(world_local) = self.player_world_local_state_like_cpp() else {
            return false;
        };
        let old_zone = world_local.zone_id;
        if old_zone != new_zone {
            self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        }
        if self
            .mutate_player_world_local_state_like_cpp(|state| {
                if old_zone != new_zone {
                    state.zone_area_authority_complete = false;
                }
                state.zone_id = new_zone;
            })
            .is_none()
        {
            return false;
        }
        let area_id = world_local.area_id;
        let _ = self.with_owned_player_mut_like_cpp(|player| {
            player
                .unit_mut()
                .world_mut()
                .set_zone_and_area(new_zone, area_id);
        });
        if self
            .mutate_player_rest_state_like_cpp(|state| {
                // Pending publication survives same-zone reentry after a cancelled post-add.
                state.defer_flag_sync = true;
            })
            .is_none()
        {
            return false;
        }
        self.update_area_represented_with_rest_update_like_cpp(new_area, false);

        let zone_entry = self
            .area_table_store
            .as_ref()
            .and_then(|store| store.get(new_zone).copied());
        let Some(zone) = zone_entry else {
            let rest_flag_update_dirty = self
                .mutate_player_rest_state_like_cpp(|state| {
                    state.defer_flag_sync = false;
                    state.deferred_flag_update_dirty
                })
                .unwrap_or(false);
            if send_rest_update
                && rest_flag_update_dirty
                && self.send_represented_resting_player_flag_update_like_cpp()
            {
                let _ = self.mutate_player_rest_state_like_cpp(|state| {
                    state.deferred_flag_update_dirty = false;
                });
            }
            return true;
        };

        self.update_represented_hostile_area_state_like_cpp(&zone);
        let Some(world_local) = self.player_world_local_state_like_cpp() else {
            return false;
        };
        // C++ keeps an existing city-rest flag in a hostile, non-sanctuary
        // LinkedChat zone. It removes the flag only in the outer non-LinkedChat
        // branch; the hostile inner branch performs no RestMgr mutation.
        if zone.linked_chat_like_cpp() {
            if !world_local.pvp_hostile || zone.is_sanctuary_like_cpp() {
                self.set_represented_rest_flag_like_cpp(REST_FLAG_IN_CITY_LIKE_CPP, 0);
            }
        } else {
            self.remove_represented_rest_flag_like_cpp(REST_FLAG_IN_CITY_LIKE_CPP);
        }
        let rest_flag_update_dirty = self
            .mutate_player_rest_state_like_cpp(|state| {
                state.defer_flag_sync = false;
                state.deferred_flag_update_dirty
            })
            .unwrap_or(false);
        if send_rest_update
            && rest_flag_update_dirty
            && self.send_represented_resting_player_flag_update_like_cpp()
        {
            let _ = self.mutate_player_rest_state_like_cpp(|state| {
                state.deferred_flag_update_dirty = false;
            });
        }

        if old_zone == new_zone {
            return false;
        }

        #[cfg(test)]
        {
            self.represented_area_zone_criteria_like_cpp.push(
                RepresentedAreaZoneCriteriaLikeCpp::EnterTopLevelArea(new_zone),
            );
            self.represented_area_zone_criteria_like_cpp.push(
                RepresentedAreaZoneCriteriaLikeCpp::LeaveTopLevelArea(old_zone),
            );
        }
        true
    }
    /// Represented C++ `Player::CheckAreaExploreAndOutdoor` discovery branch.
    ///
    /// This slice covers `AreaTableEntry::AreaBit`, `AddExploredZones`, the player-values update,
    /// `CriteriaType::RevealWorldMapOverlay`, the exploration XP branch, and the
    /// `CONFIG_VMAP_INDOOR_CHECK` aura-removal branch when a represented
    /// `WorldObject::IsOutdoors()` value is available. Terrain/VMAP ownership of
    /// the outdoors state remains a map-runtime gap.
    pub(crate) async fn check_area_explore_and_outdoor_represented_with_catalogs_like_cpp(
        &mut self,
        progression: &ProgressionCatalogsLikeCpp,
        area_id: u32,
    ) -> bool {
        if self.resolved_player_is_alive_like_cpp() != Some(true) {
            return false;
        }

        if self.resolved_is_in_taxi_flight_like_cpp() != Some(false) {
            return false;
        }

        self.remove_indoor_outdoor_auras_for_current_position_represented_like_cpp();

        if area_id == 0 {
            return false;
        }

        let Some(area_entry) = self
            .area_table_store
            .as_ref()
            .and_then(|store| store.get(area_id))
            .copied()
        else {
            return false;
        };

        let Some((offset, mask)) =
            area_entry.explored_zone_bit_like_cpp(PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP)
        else {
            return false;
        };

        let Some(explored_zones) = self.player_explored_zones_snapshot_like_cpp() else {
            return false;
        };
        if explored_zones[offset] & mask != 0 {
            return false;
        }

        #[cfg(test)]
        self.represented_reveal_world_map_overlay_criteria_like_cpp
            .push(area_id);

        if let Some(update) = self.mutate_canonical_player_like_cpp(|player| {
            player.add_explored_zones_like_cpp(offset, mask);
            player.values_update(true)
        }) {
            self.send_player_values_update_like_cpp(&update);
        }

        if area_entry.exploration_level > 0 {
            use wow_packet::packets::misc::ExplorationExperience;

            let max_level = max_level_for_expansion_like_cpp(self.server_expansion_like_cpp);
            let xp = if self.player_level_like_cpp() >= max_level {
                0
            } else {
                progression
                    .exploration_base_xp
                    .exploration_xp_reward_like_cpp(
                        self.player_level_like_cpp(),
                        area_entry.exploration_level,
                        progression.exploration_xp_rate,
                        progression.min_discovered_scaled_xp_ratio,
                    )
            };

            if xp != 0 {
                self.give_xp(xp, ObjectGuid::EMPTY, 1.0).await;
            }
            self.send_packet(&ExplorationExperience {
                area_id: area_id as i32,
                experience: xp as i32,
            });
        }

        true
    }
    #[cfg(test)]
    pub(crate) async fn check_area_explore_and_outdoor_represented_like_cpp(
        &mut self,
        area_id: u32,
    ) -> bool {
        let progression = self.progression_catalogs_for_test_like_cpp();
        self.check_area_explore_and_outdoor_represented_with_catalogs_like_cpp(
            &progression,
            area_id,
        )
        .await
    }
    #[cfg(test)]
    pub(crate) fn represented_area_zone_criteria_like_cpp(
        &self,
    ) -> &[RepresentedAreaZoneCriteriaLikeCpp] {
        &self.represented_area_zone_criteria_like_cpp
    }
    /// C++ `Player::AddExploredZones` loop for `CONFIG_START_ALL_EXPLORED` on first login.
    pub(crate) fn apply_represented_first_login_explored_zones_with_catalogs_like_cpp(
        &mut self,
        player_bootstrap: &PlayerBootstrapCatalogsLikeCpp,
    ) -> usize {
        if !player_bootstrap.start_all_explored {
            return 0;
        }

        let Some((applied, update)) = self
            .mutate_canonical_player_like_cpp(|player| {
                let mut applied = 0usize;
                for index in 0..wow_entities::PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP {
                    if player.add_explored_zones_like_cpp(index, u64::MAX) {
                        applied += 1;
                    }
                }

                (applied > 0).then(|| (applied, player.values_update(true)))
            })
            .flatten()
        else {
            return 0;
        };

        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.represented_explored_zones_like_cpp =
                [u64::MAX; PLAYER_EXPLORED_ZONES_SIZE_LIKE_CPP];
        }
        self.send_player_values_update_like_cpp(&update);
        applied
    }
    #[cfg(test)]
    pub(crate) fn apply_represented_first_login_explored_zones_like_cpp(&mut self) -> usize {
        let player_bootstrap = self.player_bootstrap_catalogs_for_test_like_cpp();
        self.apply_represented_first_login_explored_zones_with_catalogs_like_cpp(&player_bootstrap)
    }
    /// Check for area triggers at the player's current position.
    ///
    /// This is called after movement updates to handle:
    /// - Teleportation triggers (e.g., dungeon exits)
    /// - Spell effects (e.g., silencing fields)
    /// - Custom trigger actions
    ///
    /// Manages trigger state to prevent retriggering:
    /// - Entry: when player enters a trigger (was not in one)
    /// - Exit: when player leaves a trigger (was in one, no longer is)
    pub async fn check_area_triggers_with_catalogs_like_cpp(
        &mut self,
        catalogs: &AreaTriggerCatalogsLikeCpp,
    ) {
        let Some(pos) = self.player_position_like_cpp() else {
            return;
        };
        let store = catalogs.destinations.as_ref();

        let (exited_trigger_id, entered_trigger) = {
            // Get all triggers at the current position on the player's current map.
            let triggers = store.get_triggers_at_position(self.player_map_id_like_cpp(), &pos);
            let exited_trigger_id = self.active_area_trigger.filter(|prev_trigger_id| {
                !triggers
                    .iter()
                    .any(|trigger| trigger.trigger_id == *prev_trigger_id)
            });
            let entered_trigger = triggers
                .first()
                .map(|trigger| (trigger.trigger_id, trigger.teleport.clone()));
            (exited_trigger_id, entered_trigger)
        };

        // Check if we've exited the previous trigger
        if let Some(prev_trigger_id) = exited_trigger_id {
            info!(
                account = self.account_id,
                trigger_id = prev_trigger_id,
                "Exited area trigger"
            );
            self.handle_represented_tavern_area_trigger_with_catalog_like_cpp(
                catalogs.taverns.as_ref(),
                prev_trigger_id,
                false,
            );
            self.active_area_trigger = None;
        }

        // Check if we've entered a new trigger
        if let Some((trigger_id, teleport)) = entered_trigger {
            // Only trigger if this is a NEW trigger (wasn't active before)
            if self.active_area_trigger != Some(trigger_id) {
                info!(
                    account = self.account_id,
                    trigger_id, "Entered area trigger"
                );
                self.active_area_trigger = Some(trigger_id);

                if self.handle_represented_tavern_area_trigger_with_catalog_like_cpp(
                    catalogs.taverns.as_ref(),
                    trigger_id,
                    true,
                ) {
                    return;
                }

                // Handle teleportation if present
                if let Some(ref teleport) = teleport {
                    info!(
                        account = self.account_id,
                        trigger_id,
                        target_map = teleport.target_map,
                        target_x = teleport.target_position.x,
                        target_y = teleport.target_position.y,
                        target_z = teleport.target_position.z,
                        "Teleporting player via area trigger"
                    );
                    self.teleport_to(teleport.target_map, teleport.target_position)
                        .await;
                }
            }
        }
    }
    #[cfg(test)]
    pub async fn check_area_triggers(&mut self) {
        let catalogs = self.area_trigger_catalogs_for_test_like_cpp();
        self.check_area_triggers_with_catalogs_like_cpp(&catalogs)
            .await;
    }
    pub(crate) fn set_area_spirit_healer_guid_like_cpp(&mut self, healer_guid: ObjectGuid) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player
                    .resurrection_state_mut_like_cpp()
                    .area_spirit_healer_guid = healer_guid;
            })
            .is_some();
        #[cfg(test)]
        if canonical || self.player_handle_like_cpp.is_none() {
            self.area_spirit_healer_guid_like_cpp = healer_guid;
        }
        canonical || cfg!(test) && self.player_handle_like_cpp.is_none()
    }
    pub(crate) fn area_spirit_healer_guid_like_cpp(&self) -> Option<ObjectGuid> {
        let canonical = self.with_owned_player_like_cpp(|player| {
            player.resurrection_state_like_cpp().area_spirit_healer_guid
        });
        #[cfg(test)]
        if canonical.is_none() && self.player_handle_like_cpp.is_none() {
            return Some(self.area_spirit_healer_guid_like_cpp);
        }
        canonical
    }
    pub(crate) fn set_player_zone_area_like_cpp(&mut self, zone_id: u32, area_id: u32) {
        let changed = self
            .player_zone_area_like_cpp()
            .is_some_and(|current| current != (zone_id, area_id));
        if changed {
            self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        }
        let _ = self.mutate_player_world_local_state_like_cpp(|state| {
            if changed {
                state.zone_area_authority_complete = false;
            }
            state.zone_id = zone_id;
            state.area_id = area_id;
        });
        let _ = self.with_owned_player_mut_like_cpp(|player| {
            player
                .unit_mut()
                .world_mut()
                .set_zone_and_area(zone_id, area_id);
        });
    }
    pub(crate) fn set_player_zone_area_authority_complete_like_cpp(&mut self, complete: bool) {
        let _ = self.mutate_player_world_local_state_like_cpp(|state| {
            state.zone_area_authority_complete = complete;
        });
        if !complete {
            self.invalidate_canonical_player_spell_hit_aura_authority_like_cpp();
        }
    }
    pub(crate) fn player_zone_area_like_cpp(&self) -> Option<(u32, u32)> {
        self.player_world_local_state_like_cpp()
            .map(|state| (state.zone_id, state.area_id))
    }
}
