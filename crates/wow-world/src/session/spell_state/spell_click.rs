//! Represented spell-click plans and their execution at the Session boundary.
//!
//! Moved out of the Session root under #601. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub(crate) fn represented_handle_spell_click_plan_like_cpp(
        &self,
        creature_guid: ObjectGuid,
    ) -> RepresentedSpellClickPlanLikeCpp {
        self.represented_handle_spell_click_plan_with_seat_like_cpp(creature_guid, None)
    }
    pub(crate) fn represented_handle_spell_click_plan_with_seat_like_cpp(
        &self,
        creature_guid: ObjectGuid,
        seat_id: Option<i8>,
    ) -> RepresentedSpellClickPlanLikeCpp {
        let mut plan = RepresentedSpellClickPlanLikeCpp::default();

        let Some(spell_click_store) = self.npc_spell_click_store.as_ref() else {
            plan.exact_context_unrepresented = true;
            return plan;
        };
        let Some(condition_store) = self.condition_store.as_ref() else {
            plan.exact_context_unrepresented = true;
            return plan;
        };
        let Some(creature) = self.represented_spell_click_creature_snapshot_like_cpp(creature_guid)
        else {
            return plan;
        };
        if !creature.is_in_world {
            return plan;
        }

        let click_bounds = spell_click_store.spell_click_info_map_bounds_like_cpp(creature.entry);
        if click_bounds.is_empty() {
            return plan;
        }

        let Some(clicker_object) = self.build_condition_player_object_like_cpp() else {
            plan.exact_context_unrepresented = true;
            return plan;
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
            plan.exact_context_unrepresented = true;
            return plan;
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
            plan.exact_context_unrepresented = true;
            return plan;
        };
        let area_table_store = self.area_table_store.as_ref().cloned();

        for click_info in click_bounds {
            let requirements_fit = match click_info.user_type {
                SPELL_CLICK_USER_FRIEND_LIKE_CPP => {
                    let player_faction_template = self.player_faction_template_id_like_cpp();
                    if creature.is_summon
                        || self.faction_template_store.is_none()
                        || player_faction_template.is_none()
                    {
                        plan.exact_context_unrepresented = true;
                        continue;
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
                    reaction >= wow_data::reputation::ReputationRankLikeCpp::Friendly
                }
                SPELL_CLICK_USER_PARTY_LIKE_CPP | SPELL_CLICK_USER_RAID_LIKE_CPP => {
                    plan.exact_context_unrepresented = true;
                    continue;
                }
                _ => true,
            };
            if !requirements_fit {
                continue;
            }

            let conditions_match =
                crate::conditions::is_object_meeting_spell_click_conditions_like_cpp(
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
                );
            if !conditions_match {
                continue;
            }

            let vehicle_seat_data = if let Some(seat_id) = seat_id {
                let Ok(spell_id) = i32::try_from(click_info.spell_id) else {
                    plan.exact_context_unrepresented = true;
                    continue;
                };
                let Some(spell_info) = self.spell_store().and_then(|store| store.get(spell_id))
                else {
                    plan.exact_context_unrepresented = true;
                    continue;
                };
                let Some(control_effect) = spell_info.effects().iter().find(|effect| {
                    effect.effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA
                        && effect.effect_aura
                            == wow_data::spell::aura_types::SPELL_AURA_CONTROL_VEHICLE
                }) else {
                    continue;
                };
                Some((
                    control_effect.effect_index,
                    i32::from(seat_id) + 1,
                    i32::from(seat_id),
                ))
            } else {
                None
            };

            plan.casts.push(RepresentedSpellClickCastLikeCpp {
                spell_id: click_info.spell_id,
                caster: if (click_info.cast_flags & NPC_CLICK_CAST_CASTER_CLICKER_LIKE_CPP) != 0 {
                    RepresentedSpellClickUnitRefLikeCpp::Clicker
                } else {
                    RepresentedSpellClickUnitRefLikeCpp::Clickee
                },
                target: if (click_info.cast_flags & NPC_CLICK_CAST_TARGET_CLICKER_LIKE_CPP) != 0 {
                    RepresentedSpellClickUnitRefLikeCpp::Clicker
                } else {
                    RepresentedSpellClickUnitRefLikeCpp::Clickee
                },
                original_caster: if (click_info.cast_flags
                    & NPC_CLICK_CAST_ORIG_CASTER_OWNER_LIKE_CPP)
                    != 0
                {
                    RepresentedSpellClickUnitRefLikeCpp::Owner
                } else {
                    RepresentedSpellClickUnitRefLikeCpp::Clicker
                },
                cast_flags: click_info.cast_flags,
                vehicle_seat_id: seat_id,
                vehicle_control_effect_index: vehicle_seat_data
                    .map(|(effect_index, _, _)| effect_index),
                vehicle_spellmod_basepoint_value: vehicle_seat_data
                    .map(|(_, spellmod_value, _)| spellmod_value),
                vehicle_aura_fallback_basepoint_value: vehicle_seat_data
                    .map(|(_, _, fallback_value)| fallback_value),
            });
        }

        if creature.guid.is_any_type_creature() {
            plan.ai_on_spell_click_unrepresented = true;
        }
        plan
    }
    pub(crate) async fn execute_represented_spell_click_plan_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
        creature_guid: ObjectGuid,
        plan: &RepresentedSpellClickPlanLikeCpp,
    ) -> RepresentedSpellClickExecutionOutcomeLikeCpp {
        let Some(player_guid) = self.player_guid() else {
            return RepresentedSpellClickExecutionOutcomeLikeCpp {
                planned_casts: plan.casts.len(),
                failed_casts: plan.casts.len(),
                ..Default::default()
            };
        };
        let clickee_owner_guid = self
            .represented_spell_click_creature_snapshot_like_cpp(creature_guid)
            .and_then(|creature| creature.owner_guid);

        let mut outcome = RepresentedSpellClickExecutionOutcomeLikeCpp {
            planned_casts: plan.casts.len(),
            ..Default::default()
        };

        for cast in &plan.casts {
            if cast.vehicle_seat_id.is_some() {
                outcome.skipped_unrepresented_caster += 1;
                continue;
            }
            if cast.caster == RepresentedSpellClickUnitRefLikeCpp::Clickee {
                match self
                    .execute_represented_spell_click_clickee_caster_like_cpp(
                        creature_guid,
                        cast,
                        player_guid,
                        clickee_owner_guid,
                    )
                    .await
                {
                    RepresentedSpellClickClickeeCasterOutcomeLikeCpp::Executed => {
                        outcome.executed_casts += 1;
                    }
                    RepresentedSpellClickClickeeCasterOutcomeLikeCpp::UnsupportedCaster => {
                        outcome.skipped_unrepresented_caster += 1;
                    }
                    RepresentedSpellClickClickeeCasterOutcomeLikeCpp::UnsupportedTarget => {
                        outcome.skipped_unrepresented_target += 1;
                    }
                    RepresentedSpellClickClickeeCasterOutcomeLikeCpp::UnsupportedOriginalCaster => {
                        outcome.skipped_unrepresented_original_caster += 1;
                    }
                    RepresentedSpellClickClickeeCasterOutcomeLikeCpp::Failed => {
                        outcome.failed_casts += 1;
                    }
                }
                continue;
            }
            if cast.caster != RepresentedSpellClickUnitRefLikeCpp::Clicker {
                outcome.skipped_unrepresented_caster += 1;
                continue;
            }
            match cast.original_caster {
                RepresentedSpellClickUnitRefLikeCpp::Clicker => {}
                RepresentedSpellClickUnitRefLikeCpp::Owner
                    if clickee_owner_guid == Some(player_guid) => {}
                _ => {
                    outcome.skipped_unrepresented_original_caster += 1;
                    continue;
                }
            }

            let target_guid = match cast.target {
                RepresentedSpellClickUnitRefLikeCpp::Clicker => player_guid,
                RepresentedSpellClickUnitRefLikeCpp::Clickee => creature_guid,
                RepresentedSpellClickUnitRefLikeCpp::Owner => {
                    outcome.skipped_unrepresented_target += 1;
                    continue;
                }
            };
            let Ok(spell_id) = i32::try_from(cast.spell_id) else {
                outcome.failed_casts += 1;
                continue;
            };
            let Some(cast_id) = self.next_represented_spell_cast_guid_like_cpp(spell_id) else {
                outcome.failed_casts += 1;
                continue;
            };

            let result = self
                .execute_spell_with_visual_and_target_data_and_generator_like_cpp(
                    item_guid_generator,
                    creature_spawn_catalogs,
                    spell_id,
                    target_guid,
                    cast_id,
                    wow_packet::packets::spell::SpellCastVisual::default(),
                    SpellTargetData {
                        flags: 0x2,
                        unit: target_guid,
                        item: ObjectGuid::EMPTY,
                        ..SpellTargetData::default()
                    },
                )
                .await;
            if result.is_ok() {
                outcome.executed_casts += 1;
            } else {
                outcome.failed_casts += 1;
            }
        }

        if plan.ai_on_spell_click_unrepresented {
            let has_unrepresented_or_failed_casts = outcome.skipped_unrepresented_caster > 0
                || outcome.skipped_unrepresented_target > 0
                || outcome.skipped_unrepresented_original_caster > 0
                || outcome.failed_casts > 0;
            if outcome.executed_casts > 0 {
                let represented = if self
                    .mutate_world_creature(creature_guid, |creature| {
                        creature
                            .creature
                            .record_ai_spell_click_inform(player_guid, true);
                    })
                    .is_some()
                {
                    true
                } else {
                    self.mutate_canonical_creature_by_guid_like_cpp(creature_guid, |creature| {
                        creature.record_ai_spell_click_inform(player_guid, true);
                    })
                    .is_some()
                };
                outcome.ai_on_spell_click_represented = represented;
                outcome.ai_on_spell_click_unrepresented =
                    !represented || has_unrepresented_or_failed_casts;
            } else {
                outcome.ai_on_spell_click_unrepresented = true;
            }
        }

        outcome
    }
    #[cfg(test)]
    pub(crate) async fn execute_represented_spell_click_plan_like_cpp(
        &mut self,
        creature_guid: ObjectGuid,
        plan: &RepresentedSpellClickPlanLikeCpp,
    ) -> RepresentedSpellClickExecutionOutcomeLikeCpp {
        let generators = self.id_generators_for_test_like_cpp();
        let creature_spawn_catalogs = self.creature_spawn_catalogs_for_test_like_cpp();
        self.execute_represented_spell_click_plan_with_generator_like_cpp(
            generators.item.as_ref(),
            &creature_spawn_catalogs,
            creature_guid,
            plan,
        )
        .await
    }
    pub(crate) fn update_visible_spell_clicks_like_cpp(&mut self) -> usize {
        let Some(spell_click_store) = self.npc_spell_click_store.as_ref() else {
            return 0;
        };
        let Some(condition_store) = self.condition_store.as_ref() else {
            return 0;
        };

        let mut sent = 0;
        let visible_guids = self
            .client_visible_guids_like_cpp
            .snapshot_like_cpp()
            .into_iter()
            .collect::<Vec<_>>();
        for guid in visible_guids {
            if !guid.is_creature_or_vehicle() {
                continue;
            }
            let Some(creature) = self.represented_spell_click_creature_snapshot_like_cpp(guid)
            else {
                continue;
            };
            if (u64::from(creature.npc_flags) & UNIT_NPC_FLAG_SPELLCLICK_LIKE_CPP) == 0 {
                continue;
            }
            let click_bounds =
                spell_click_store.spell_click_info_map_bounds_like_cpp(creature.entry);
            if !click_bounds.iter().any(|click| {
                crate::conditions::has_conditions_for_spell_click_event_like_cpp(
                    condition_store,
                    creature.entry,
                    click.spell_id,
                )
            }) {
                continue;
            }

            let mut packet_update =
                wow_packet::packets::update::UnitDataValuesDeltaUpdate::default();
            packet_update.changed_object_type_mask = 1 << wow_entities::TYPEID_UNIT;
            packet_update.unit_data_mask[113 / 32] |= 1 << (113 % 32);
            packet_update.unit_data_mask[114 / 32] |= 1 << (114 % 32);
            packet_update.npc_flags = [creature.npc_flags, 0];
            let update = self.represented_unit_packet_update_to_update_object_like_cpp(
                guid,
                self.player_map_id_like_cpp(),
                packet_update,
            );
            self.send_packet(&update);
            sent += 1;
        }

        sent
    }
    pub(in crate::session) fn represented_vehicle_seat_spell_click_plan_available_like_cpp(
        &self,
        vehicle_guid: ObjectGuid,
        seat_id: i8,
    ) -> bool {
        !self
            .represented_handle_spell_click_plan_with_seat_like_cpp(vehicle_guid, Some(seat_id))
            .casts
            .is_empty()
    }
}
