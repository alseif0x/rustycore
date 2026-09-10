//! Represented creature death, kill hooks and the reputation they award.
//!
//! Moved out of the Session root under #599. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    /// Queue one creature kill for the loot and reward phases, deduplicated.
    ///
    /// Both writers of this queue — `run_combat_tick` and the delivery handler
    /// for a map-owned resolution (#28) — must enqueue identically, so the
    /// dedup lives here instead of being written twice. The queues stay private
    /// to the session; a handler asks for the transition, not for the fields.
    pub(crate) fn queue_pending_creature_kill_like_cpp(
        &mut self,
        killer_guid: ObjectGuid,
        creature_guid: ObjectGuid,
        creature_entry: u32,
        creature_level: u8,
    ) {
        if !self
            .pending_creature_kill_loot_like_cpp
            .contains(&creature_guid)
        {
            self.pending_creature_kill_loot_like_cpp.push(creature_guid);
        }
        if !self
            .pending_creature_kill_rewards_like_cpp
            .iter()
            .any(|reward| reward.creature_guid == creature_guid)
        {
            self.pending_creature_kill_rewards_like_cpp
                .push(PendingCreatureKillRewardLikeCpp {
                    killer_guid,
                    creature_guid,
                    creature_entry,
                    creature_level,
                });
        }
    }
    /// XP reward for killing a creature.
    /// C++ `Trinity::XP::Gain` / `Trinity::XP::BaseGain`.
    pub(crate) fn creature_kill_xp(&self, mob_level: u8) -> u32 {
        let pl = self.player_level_like_cpp() as i32;
        let ml = mob_level as i32;

        // nBaseExp by content level (WotLK = 71-80 content)
        let n_base_exp: i32 = if pl >= 71 {
            580
        } else if pl >= 61 {
            235
        } else {
            45
        };

        // Gray level check
        let gray = self.gray_level(pl as u8) as i32;
        if ml <= gray {
            return 0;
        }

        let base_gain = if ml >= pl {
            let diff = (ml - pl).min(4);
            ((pl * 5 + n_base_exp) * (20 + diff) / 10 + 1) / 2
        } else {
            let zd = self.zero_difference(pl as u8) as i32;
            (pl * 5 + n_base_exp) * (zd + ml - pl) / zd
        };

        base_gain.max(0) as u32
    }
    pub(in crate::session) fn represented_creature_kill_reputation_rate_like_cpp(
        &mut self,
        player_guid: ObjectGuid,
        creature_guid: ObjectGuid,
    ) -> f32 {
        let (Some(group_guid), Some(group_registry)) = (
            self.resolved_group_guid_like_cpp(),
            self.group_registry.as_ref(),
        ) else {
            return 1.0;
        };
        let Some(group) = group_registry.get(&group_guid) else {
            return 1.0;
        };
        if !group.members.contains(&player_guid) {
            return 1.0;
        }
        let group_members = group.members.clone();
        drop(group);

        let Some((player_level, player_map_id, player_position, _)) =
            self.represented_player_group_reward_state_like_cpp(player_guid)
        else {
            return 1.0;
        };

        if self
            .map_store()
            .and_then(|store| store.get(u32::from(player_map_id)))
            .is_some_and(|entry| entry.is_dungeon())
        {
            return 1.0;
        }

        let (reward_map_id, reward_position) = self
            .mutate_world_creature(creature_guid, |creature| {
                (creature.map_id() as u16, creature.position())
            })
            .unwrap_or((player_map_id, player_position));

        let mut count = 0_u32;
        let mut sum_level = 0_u32;
        for member_guid in group_members {
            if member_guid == player_guid {
                count = count.saturating_add(1);
                sum_level = sum_level.saturating_add(u32::from(player_level));
                continue;
            }
            let Some((member_level, _member_map_id, _member_position, is_alive)) =
                self.represented_player_group_reward_state_like_cpp(member_guid)
            else {
                continue;
            };
            if is_alive
                && self.represented_player_at_group_reward_distance_like_cpp(
                    member_guid,
                    reward_map_id,
                    reward_position,
                )
            {
                count = count.saturating_add(1);
                sum_level = sum_level.saturating_add(u32::from(member_level));
            }
        }

        if count == 0 || sum_level == 0 {
            return 1.0;
        }

        crate::session_rules::xp_in_group_rate_like_cpp(count, false) * f32::from(player_level)
            / sum_level as f32
    }
    fn reward_creature_kill_reputation_branch_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
        faction_id: u32,
        reputation_value: i32,
        max_cap: u8,
        creature_level: u8,
        kill_reward_rate: f32,
    ) {
        #[cfg(not(test))]
        let _ = creature_guid;
        if faction_id == 0 {
            return;
        }

        let effective_faction_id = self
            .represented_championing_faction_for_kill_like_cpp()
            .unwrap_or(faction_id);

        let faction_store = match self.faction_store().map(Arc::clone) {
            Some(store) => store,
            None => return,
        };
        let Some(faction_entry) = faction_store.get(effective_faction_id).cloned() else {
            return;
        };

        let mut reputation = self.calculate_kill_reputation_gain_like_cpp(
            creature_level,
            reputation_value,
            effective_faction_id,
        );
        reputation = (reputation as f32 * kill_reward_rate) as i32;
        if reputation == 0 {
            return;
        }

        let Some(current_rank) = self.with_reputation_mgr_like_cpp(|mgr| {
            mgr.rank_for_faction_entry_like_cpp(
                &faction_entry,
                self.friendship_rep_reaction_store.as_deref(),
                self.player_race_like_cpp(),
                self.player_class_like_cpp(),
            )
        }) else {
            return;
        };
        let spillover_only = current_rank.as_u8() > max_cap;

        let reputation_spillover_template_store =
            self.reputation_spillover_template_store().map(Arc::clone);
        let friendship_rep_reaction_store = self.friendship_rep_reaction_store().map(Arc::clone);
        let paragon_reputation_store = self.paragon_reputation_store().map(Arc::clone);
        let currency_types_store = self.currency_types_store().map(Arc::clone);
        let db_spillover_template = reputation_spillover_template_store
            .as_deref()
            .and_then(|store| store.get(effective_faction_id));
        let options = crate::reputation::mgr::SetReputationOptionsLikeCpp {
            incremental: true,
            spillover_only,
            no_spillover: false,
            reputation_gain_rate: self.reputation_rates_like_cpp().gain,
            paragon_reward_quest_status_none_like_cpp: true,
            renown_current_level_like_cpp: 0,
            renown_currency_increased_cap_quantity_like_cpp: 0,
            player_race: self.player_race_like_cpp(),
            player_class: self.player_class_like_cpp(),
        };
        let Some((outcome, packet)) = self.mutate_reputation_mgr_like_cpp(|mgr| {
            let outcome = mgr.set_reputation_like_cpp(
                &faction_entry,
                reputation,
                options,
                &faction_store,
                db_spillover_template,
                friendship_rep_reaction_store.as_deref(),
                paragon_reputation_store.as_deref(),
                currency_types_store.as_deref(),
            );
            let packet = outcome
                .send_state_rep_list_id
                .map(|rep_list_id| mgr.set_faction_standing_packet_like_cpp(Some(rep_list_id)));
            (outcome, packet)
        }) else {
            return;
        };
        if let Some(packet) = packet {
            self.send_packet(&packet);
        }
        #[cfg(not(test))]
        let _ = outcome;
        #[cfg(test)]
        if outcome.applied {
            self.represented_creature_kill_events_like_cpp.push(
                RepresentedCreatureKillEventLikeCpp::CreatureKillReputationAwarded {
                    creature_guid,
                    faction_id: effective_faction_id,
                    reputation,
                    spillover_only,
                },
            );
        }
    }
    pub(in crate::session) fn reward_reputation_from_creature_kill_like_cpp(
        &mut self,
        creature_entry: u32,
        creature_guid: ObjectGuid,
        creature_level: u8,
        kill_reward_rate: f32,
    ) {
        if self
            .mutate_world_creature(creature_guid, |creature| {
                creature.creature.is_reputation_gain_disabled()
            })
            .unwrap_or(false)
        {
            return;
        }

        let Some(rep) = self
            .creature_onkill_reputation_store()
            .and_then(|store| store.get(creature_entry))
            .copied()
        else {
            return;
        };
        let team = player_team_for_race_cpp(self.player_race_like_cpp());

        if rep.rep_faction_1 != 0 && (!rep.team_dependent || team == Team::Alliance) {
            self.reward_creature_kill_reputation_branch_like_cpp(
                creature_guid,
                rep.rep_faction_1,
                rep.rep_value_1,
                rep.reputation_max_cap_1,
                creature_level,
                kill_reward_rate,
            );
        }
        if rep.rep_faction_2 != 0 && (!rep.team_dependent || team == Team::Horde) {
            self.reward_creature_kill_reputation_branch_like_cpp(
                creature_guid,
                rep.rep_faction_2,
                rep.rep_value_2,
                rep.reputation_max_cap_2,
                creature_level,
                kill_reward_rate,
            );
        }
    }
    /// Called when the player kills a creature. Checks all active kill-objective quests
    /// and updates progress. Sends SMSG_QUEST_UPDATE_ADD_CREDIT if progress was made.
    pub(crate) async fn on_creature_killed_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_entry: u32,
        creature_guid: wow_core::ObjectGuid,
    ) {
        // C++ QUEST_OBJECTIVE_MONSTER.
        self.update_represented_storing_value_quest_objective_progress_like_cpp(
            item_guid_generator,
            0,
            creature_entry as i32,
            1,
            creature_guid,
        )
        .await;
    }
    #[cfg(test)]
    pub(crate) async fn on_creature_killed(
        &mut self,
        creature_entry: u32,
        creature_guid: wow_core::ObjectGuid,
    ) {
        let Some(generator) = self.item_guid_generator_like_cpp_for_bridge() else {
            return;
        };
        self.on_creature_killed_with_generator_like_cpp(
            generator.as_ref(),
            creature_entry,
            creature_guid,
        )
        .await;
    }
    pub(in crate::session) async fn process_pending_creature_kills_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
    ) {
        let mut pending = std::mem::take(&mut self.pending_creature_kill_loot_like_cpp);
        pending.sort_by_key(|guid| (guid.high_value(), guid.low_value()));
        pending.dedup();
        for creature_guid in pending {
            self.ensure_represented_creature_kill_loot_like_cpp(creature_guid)
                .await;
        }

        let mut rewards = std::mem::take(&mut self.pending_creature_kill_rewards_like_cpp);
        rewards.sort_by_key(|reward| {
            (
                reward.creature_guid.high_value(),
                reward.creature_guid.low_value(),
            )
        });
        rewards.dedup_by_key(|reward| reward.creature_guid);
        for reward in rewards {
            let can_give_experience = self
                .mutate_world_creature(reward.creature_guid, |creature| {
                    creature.creature.can_give_experience_like_cpp()
                })
                .unwrap_or(false);
            let xp = can_give_experience
                .then(|| self.creature_kill_xp(reward.creature_level))
                .unwrap_or(0);
            if xp > 0 {
                // The represented reward queue is still solo-session only.
                // Full C++ KillRewarder group fanout must pre-scale XP per
                // member before passing its separate `_groupRate` here.
                self.give_xp(xp, reward.creature_guid, 1.0).await;
            }
            let reputation_rate = self.represented_creature_kill_reputation_rate_like_cpp(
                reward.killer_guid,
                reward.creature_guid,
            );
            self.reward_reputation_from_creature_kill_like_cpp(
                reward.creature_entry,
                reward.creature_guid,
                reward.creature_level,
                reputation_rate,
            );
            self.on_creature_killed_with_generator_like_cpp(
                item_guid_generator,
                reward.creature_entry,
                reward.creature_guid,
            )
            .await;
            #[cfg(test)]
            self.record_represented_creature_kill_hooks_like_cpp(
                reward.killer_guid,
                reward.creature_guid,
            );
            if let Some(values_update) = self
                .complete_represented_creature_death_state_after_kill_hooks_like_cpp(
                    reward.killer_guid,
                    reward.creature_guid,
                )
                && self
                    .client_visible_guids_like_cpp
                    .contains(&reward.creature_guid)
                && let Some(update) = self.represented_unit_values_update_to_update_object_like_cpp(
                    reward.creature_guid,
                    self.player_map_id_like_cpp(),
                    &values_update,
                )
            {
                self.send_packet(&update);
            }
        }
    }
    #[cfg(test)]
    pub(in crate::session) fn record_represented_creature_kill_hooks_like_cpp(
        &mut self,
        attacker_guid: ObjectGuid,
        creature_guid: ObjectGuid,
    ) {
        let reward_source = self
            .mutate_world_creature(creature_guid, |creature| {
                (creature.map_id() as u16, creature.position())
            })
            .or_else(|| {
                self.player_position_like_cpp()
                    .map(|position| (self.player_map_id_like_cpp(), position))
            });
        let Some(reward_source) = reward_source else {
            return;
        };
        let mut tappers = self
            .mutate_world_creature(creature_guid, |creature| {
                creature.creature.tap_list().to_vec()
            })
            .unwrap_or_default();
        if tappers.is_empty() {
            tappers.push(attacker_guid);
        }
        let mut unique_tappers = Vec::with_capacity(tappers.len());
        for tapper in tappers {
            if !unique_tappers.contains(&tapper) {
                unique_tappers.push(tapper);
            }
        }

        self.represented_creature_kill_events_like_cpp.push(
            RepresentedCreatureKillEventLikeCpp::KillerProc {
                attacker_guid,
                victim_guid: creature_guid,
            },
        );

        for tapper_guid in unique_tappers {
            if !self.represented_player_at_group_reward_distance_like_cpp(
                tapper_guid,
                reward_source.0,
                reward_source.1,
            ) {
                continue;
            }
            self.represented_creature_kill_events_like_cpp.push(
                RepresentedCreatureKillEventLikeCpp::TapperTargetDiesProc {
                    tapper_guid,
                    victim_guid: creature_guid,
                },
            );
        }

        self.represented_creature_kill_events_like_cpp.push(
            RepresentedCreatureKillEventLikeCpp::VictimDeathProc {
                victim_guid: creature_guid,
            },
        );
        self.represented_creature_kill_events_like_cpp.push(
            RepresentedCreatureKillEventLikeCpp::DeliveredKillingBlowCriteria {
                player_guid: attacker_guid,
                victim_guid: creature_guid,
                quantity: 1,
            },
        );
    }
    fn represented_creature_can_skin_after_death_state_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
    ) -> bool {
        let skin_loot_id = self
            .mutate_world_creature(creature_guid, |creature| {
                creature.creature.ai_ownership().skin_loot_id
            })
            .unwrap_or(0);
        if skin_loot_id == 0 {
            return false;
        }
        self.loot_stores
            .as_ref()
            .and_then(|stores| stores.get(&LootStoreKind::Skinning))
            .is_some_and(|store| store.collect_loot_ids_like_cpp().contains(&skin_loot_id))
    }
    #[cfg(test)]
    pub(crate) fn represented_creature_kill_events_like_cpp(
        &self,
    ) -> &[RepresentedCreatureKillEventLikeCpp] {
        &self.represented_creature_kill_events_like_cpp
    }
    pub(in crate::session) fn complete_represented_creature_death_state_after_kill_hooks_like_cpp(
        &mut self,
        attacker_guid: ObjectGuid,
        creature_guid: ObjectGuid,
    ) -> Option<wow_entities::UnitValuesUpdate> {
        #[cfg(not(test))]
        let _ = attacker_guid;
        let lootable = self
            .loot_table
            .get(&creature_guid)
            .is_some_and(|loot| loot.coins != 0 || loot.unlooted_count != 0);
        let can_skin = self.represented_creature_can_skin_after_death_state_like_cpp(creature_guid);
        let values_update = self.mutate_world_creature(creature_guid, |creature| {
            creature.complete_death_state_after_kill_hooks_like_cpp();
            creature.apply_corpse_loot_flags_after_death_state_like_cpp(lootable, can_skin);
            creature.creature.unit().values_update()
        })?;
        #[cfg(test)]
        self.represented_creature_kill_events_like_cpp.push(
            RepresentedCreatureKillEventLikeCpp::DeathStateJustDied {
                victim_guid: creature_guid,
            },
        );
        #[cfg(test)]
        self.represented_creature_kill_events_like_cpp.push(
            RepresentedCreatureKillEventLikeCpp::ZoneScriptUnitDeath {
                unit_guid: creature_guid,
            },
        );
        #[cfg(test)]
        self.record_represented_tapper_pet_killed_unit_hooks_like_cpp(creature_guid);
        #[cfg(test)]
        self.represented_creature_kill_events_like_cpp.push(
            RepresentedCreatureKillEventLikeCpp::LootFlagsApplied {
                creature_guid,
                lootable,
                can_skin,
                skinnable: can_skin,
            },
        );
        #[cfg(test)]
        self.represented_creature_kill_events_like_cpp.push(
            RepresentedCreatureKillEventLikeCpp::CreatureOnHealthDepletedAi {
                creature_guid,
                attacker_guid,
                is_kill: true,
            },
        );
        #[cfg(test)]
        self.represented_creature_kill_events_like_cpp.push(
            RepresentedCreatureKillEventLikeCpp::CreatureJustDiedAi {
                creature_guid,
                killer_guid: attacker_guid,
            },
        );
        #[cfg(test)]
        if attacker_guid.is_player() {
            self.represented_creature_kill_events_like_cpp.push(
                RepresentedCreatureKillEventLikeCpp::ScriptMgrOnCreatureKill {
                    killer_guid: attacker_guid,
                    creature_guid,
                },
            );
        }
        Some(values_update)
    }
}
