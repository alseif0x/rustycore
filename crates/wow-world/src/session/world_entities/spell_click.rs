//! Represented spell-click interaction with observed creatures.
//!
//! Moved out of the Session root under #599. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(in crate::session) fn represented_spell_click_creature_snapshot_like_cpp(
        &self,
        guid: ObjectGuid,
    ) -> Option<RepresentedSpellClickCreatureSnapshotLikeCpp> {
        if guid.is_empty() || !guid.is_any_type_creature() {
            return None;
        }
        let manager = self.canonical_map_manager.as_ref()?;
        let Ok(manager) = manager.lock() else {
            return None;
        };
        let map = manager.find_map(u32::from(self.player_map_id_like_cpp()), 0)?;
        map.map()
            .with_creature_or_pet_like_cpp(guid, |creature, pet_owner_guid| {
                RepresentedSpellClickCreatureSnapshotLikeCpp {
                    guid: creature.guid(),
                    entry: creature.entry(),
                    map_id: creature.unit().world().map_id(),
                    instance_id: creature.unit().world().instance_id(),
                    position: creature.position(),
                    phase_shift: creature.unit().world().phase_shift().clone(),
                    npc_flags: creature.ai_ownership().npc_flags,
                    faction_template_id: creature.unit().data().faction_template.max(0) as u32,
                    level: u32::from(creature.level()),
                    health: creature.current_health(),
                    max_health: creature.max_health(),
                    is_alive: creature.is_alive(),
                    is_in_world: creature.unit().world().object().is_in_world(),
                    is_summon: creature.is_summon_like_cpp(),
                    owner_guid: pet_owner_guid.or(creature.unit().subsystems().control.owner_guid),
                }
            })
    }
    pub(crate) fn represented_can_see_spell_click_on_creature_like_cpp(
        &self,
        creature_guid: ObjectGuid,
    ) -> RepresentedCanSeeSpellClickOutcomeLikeCpp {
        let Some(spell_click_store) = self.npc_spell_click_store.as_ref() else {
            return RepresentedCanSeeSpellClickOutcomeLikeCpp::ExactContextUnrepresented;
        };
        let Some(condition_store) = self.condition_store.as_ref() else {
            return RepresentedCanSeeSpellClickOutcomeLikeCpp::ExactContextUnrepresented;
        };
        let Some(creature) = self.represented_spell_click_creature_snapshot_like_cpp(creature_guid)
        else {
            return RepresentedCanSeeSpellClickOutcomeLikeCpp::ExactContextUnrepresented;
        };
        if !creature.is_in_world {
            return RepresentedCanSeeSpellClickOutcomeLikeCpp::Hidden;
        }

        if (u64::from(creature.npc_flags) & UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP) == 0 {
            return RepresentedCanSeeSpellClickOutcomeLikeCpp::Hidden;
        }

        let click_bounds = spell_click_store.spell_click_info_map_bounds_like_cpp(creature.entry);
        if click_bounds.is_empty() {
            return RepresentedCanSeeSpellClickOutcomeLikeCpp::Hidden;
        }

        let Some(clicker_object) = self.build_condition_player_object_like_cpp() else {
            return RepresentedCanSeeSpellClickOutcomeLikeCpp::ExactContextUnrepresented;
        };
        let mut target_object = WorldObject::new(
            false,
            TypeId::Unit,
            wow_constants::TypeMask::OBJECT | wow_constants::TypeMask::UNIT,
        );
        target_object.object_mut().create(creature.guid);
        target_object.object_mut().set_entry(creature.entry);
        let _ = target_object.set_map(creature.map_id, creature.instance_id);
        target_object.relocate(creature.position);
        *target_object.phase_shift_mut() = creature.phase_shift.clone();

        let Some(player_unit_snapshot) = self.condition_player_unit_snapshot_like_cpp() else {
            return RepresentedCanSeeSpellClickOutcomeLikeCpp::ExactContextUnrepresented;
        };
        let player_snapshot = self.condition_player_snapshot_like_cpp();
        let creature_unit_snapshot = crate::conditions::ConditionUnitSnapshot {
            level: creature.level,
            health: creature.health,
            max_health: creature.max_health,
            class_mask: 0,
            race: 0,
            creature_type: None,
            is_alive: creature.is_alive,
            is_charmed: false,
            in_water: false,
            unit_state: 0,
            stand_state: UnitStandStateType::Stand as u32,
        };
        let player_condition_store = self.player_condition_store().cloned();
        let Some(player_condition_context) = self.represented_player_condition_context_like_cpp()
        else {
            return RepresentedCanSeeSpellClickOutcomeLikeCpp::ExactContextUnrepresented;
        };
        let area_table_store = self.area_table_store.as_ref().cloned();

        for click_info in click_bounds {
            match click_info.user_type {
                SPELL_CLICK_USER_FRIEND_LIKE_CPP => {
                    let player_faction_template = self.player_faction_template_id_like_cpp();
                    if creature.is_summon
                        || self.faction_template_store.is_none()
                        || player_faction_template.is_none()
                    {
                        return RepresentedCanSeeSpellClickOutcomeLikeCpp::ExactContextUnrepresented;
                    }
                    let reaction = self.represented_get_reaction_to_like_cpp(
                        RepresentedGetReactionInputLikeCpp {
                            self_faction_template_id: player_faction_template.unwrap_or(0),
                            target_faction_template_id: creature.faction_template_id,
                            same_object: false,
                            attackable_by_summoner: false,
                            same_charmer_or_owner_or_self: false,
                            self_has_player_owner: true,
                            target_has_player_owner: false,
                            target_player_owner_is_current_session: false,
                            target_owner_forced_rank_for_self: None,
                            same_player_owner: false,
                            duel_in_progress: false,
                            same_raid: false,
                            self_unit_player_controlled: true,
                            target_unit_player_controlled: false,
                            self_ffa_pvp: false,
                            target_ffa_pvp: false,
                            self_ignores_reputation: false,
                            target_ignores_reputation: false,
                            target_is_unit: true,
                            target_player_contested_pvp: false,
                        },
                    );
                    if reaction < wow_data::reputation::ReputationRankLikeCpp::Friendly {
                        return RepresentedCanSeeSpellClickOutcomeLikeCpp::Hidden;
                    }
                }
                SPELL_CLICK_USER_PARTY_LIKE_CPP | SPELL_CLICK_USER_RAID_LIKE_CPP => {
                    return RepresentedCanSeeSpellClickOutcomeLikeCpp::ExactContextUnrepresented;
                }
                _ => {}
            }

            if crate::conditions::is_object_meeting_spell_click_conditions_like_cpp(
                condition_store,
                creature.entry,
                click_info.spell_id,
                Some(&clicker_object),
                Some(&target_object),
                |condition, source_info| {
                    source_info.set_unit_target_snapshot(0, player_unit_snapshot);
                    source_info.set_player_target_snapshot(0, player_snapshot);
                    source_info.set_unit_target_snapshot(1, creature_unit_snapshot);
                    if let Some(store) = player_condition_store.as_ref() {
                        source_info.set_player_condition_store(store.as_ref());
                        if let Some(context) = player_condition_context.as_context(self) {
                            source_info.set_player_condition_context(0, context);
                        }
                    }
                    crate::conditions::condition_meets_basic_like_cpp(
                        condition,
                        source_info,
                        |area_id, required_area_id| {
                            area_table_store.as_ref().is_some_and(|store| {
                                store.is_in_area_like_cpp(area_id, required_area_id)
                            })
                        },
                    )
                    .value()
                    .unwrap_or(false)
                },
            ) {
                return RepresentedCanSeeSpellClickOutcomeLikeCpp::Visible;
            }
        }

        RepresentedCanSeeSpellClickOutcomeLikeCpp::Hidden
    }
    pub(in crate::session) async fn apply_represented_spell_click_creature_damage_to_clicker_like_cpp(
        &mut self,
        spell_id: i32,
        player_guid: ObjectGuid,
        damage_amount: u32,
    ) -> Result<(), &'static str> {
        if self.player_guid() != Some(player_guid) {
            return Err("Target player not current session");
        }
        let Some((original_health, health_after, _, applied_damage, _)) = self
            .apply_owned_player_damage_like_cpp(damage_amount, wow_constants::DeathState::Corpse)
        else {
            return Err("Target player owner not available");
        };
        if damage_amount > 0 && applied_damage == 0 {
            debug!(
                account = self.account_id,
                player = ?player_guid,
                spell_id,
                damage = damage_amount,
                "Skipping spellclick creature-caster damage because C++ EffectSchoolDMG requires alive target"
            );
            return Ok(());
        }
        self.sync_player_registry_state_like_cpp();
        if health_after != original_health {
            self.send_player_health_update_like_cpp(player_guid, u64::from(health_after));
        }

        Ok(())
    }
    pub(in crate::session) async fn apply_represented_spell_click_creature_damage_to_clickee_like_cpp(
        &mut self,
        spell_id: i32,
        creature_guid: ObjectGuid,
        damage_amount: u32,
    ) -> Result<(), &'static str> {
        let account_id = self.account_id;
        let values_update = self
            .mutate_world_creature(creature_guid, |creature| {
                if !creature.is_alive() {
                    debug!(
                        account = account_id,
                        creature = ?creature_guid,
                        spell_id,
                        damage = damage_amount,
                        "Skipping spellclick creature-caster damage because C++ EffectSchoolDMG requires alive target"
                    );
                    return None;
                }
                let _died = creature.take_damage_before_death_state_like_cpp(damage_amount);
                Some(creature.creature.unit().values_update())
            })
            .ok_or("Target creature not found")?;
        let Some(values_update) = values_update else {
            return Ok(());
        };

        if self.client_visible_guids_like_cpp.contains(&creature_guid)
            && let Some(update) = self.represented_unit_values_update_to_update_object_like_cpp(
                creature_guid,
                self.player_map_id_like_cpp(),
                &values_update,
            )
        {
            self.send_packet(&update);
        }

        Ok(())
    }
}
