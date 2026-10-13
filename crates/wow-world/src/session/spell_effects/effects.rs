//! Remaining represented effect operations owned by this responsibility.
//!
//! Moved out of the Session root under #621. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    /// Server-triggered cast with an explicit C++ trigger contract.
    ///
    /// C++ `Unit::CastSpell` always constructs a `Spell`, so `m_castId` is a
    /// real `Map::GenerateLowGuid<HighGuid::Cast>` value even when no client
    /// requested the cast. Publishing an empty CastID leaves the client unable
    /// to correlate the resulting `SMSG_SPELL_GO`, so this entry point
    /// allocates from the admitted Map's shared Cast sequence just like the
    /// normal request path and the represented creature consumer.
    ///
    /// `metadata` carries the caller's own contract; normal client defaults are
    /// never imposed on these consumers.
    pub(crate) async fn execute_server_triggered_spell_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
        spell_id: i32,
        target_guid: ObjectGuid,
        metadata: SpellCastMetadata,
    ) -> Result<(), &'static str> {
        use wow_packet::packets::spell::SpellCastVisual;

        let Some(cast_id) = self.next_represented_spell_cast_guid_like_cpp(spell_id) else {
            return Err("canonical Map cast identity unavailable for a triggered cast");
        };
        self.execute_spell_with_visual_and_target_data_with_metadata_and_generator_like_cpp(
            item_guid_generator,
            creature_spawn_catalogs,
            spell_id,
            target_guid,
            cast_id,
            SpellCastVisual {
                spell_visual_id: 0,
                script_visual_id: 0,
            },
            Default::default(),
            metadata,
        )
        .await
    }
    pub(in crate::session) fn apply_learn_transmog_set_effect_like_cpp(
        &mut self,
        target_guid: ObjectGuid,
        transmog_set_id: i32,
    ) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };
        if target_guid != player_guid {
            return;
        }

        let Ok(transmog_set_id) = u32::try_from(transmog_set_id) else {
            return;
        };
        let Some(update) = self.add_transmog_set_like_cpp(transmog_set_id) else {
            return;
        };
        if let Some(packet) = player_values_update_to_update_object(
            player_guid,
            self.core.player_map_id_like_cpp(),
            &update,
        ) {
            self.send_packet(&packet);
        }
    }
    pub(in crate::session) fn represented_nearby_candidate_meets_implicit_conditions_like_cpp(
        &self,
        candidate: &WorldObject,
        candidate_unit_snapshot: Option<wow_conditions::ConditionUnitSnapshot>,
        implicit_conditions: Option<&[wow_data::Condition]>,
        caster_object: Option<&WorldObject>,
        player_unit_snapshot: wow_conditions::ConditionUnitSnapshot,
        player_snapshot: wow_conditions::ConditionPlayerSnapshot,
        player_condition_store: Option<&wow_data::PlayerConditionStore>,
        player_condition_context: Option<&RepresentedPlayerConditionContextLikeCpp>,
        area_table_store: Option<&wow_data::AreaTableStore>,
    ) -> bool {
        let Some(conditions) = implicit_conditions.filter(|conditions| !conditions.is_empty())
        else {
            return true;
        };
        let Some(condition_store) = self.catalogs.condition_store.as_ref() else {
            return false;
        };

        // C++ `WorldObjectSpellTargetCheck` builds `ConditionSourceInfo(nullptr, caster)`
        // and then assigns the tested target to condition slot 0.
        let mut source_info =
            wow_conditions::ConditionSourceInfo::from_targets(Some(candidate), caster_object, None);
        if let Some(snapshot) = candidate_unit_snapshot {
            source_info.set_unit_target_snapshot(0, snapshot);
        }
        source_info.set_unit_target_snapshot(1, player_unit_snapshot);
        source_info.set_player_target_snapshot(1, player_snapshot);
        if let (Some(store), Some(context)) = (player_condition_store, player_condition_context) {
            source_info.set_player_condition_store(store);
            if let Some(context) = context.as_context(self) {
                source_info.set_player_condition_context(1, context);
            }
        }

        wow_conditions::is_object_meet_to_conditions_like_cpp(
            &mut source_info,
            conditions,
            condition_store.as_ref(),
            |condition, source_info| {
                wow_conditions::condition_meets_basic_like_cpp(
                    condition,
                    source_info,
                    |area_id, required_area_id| {
                        area_table_store.is_some_and(|store| {
                            store.is_in_area_like_cpp(area_id, required_area_id)
                        })
                    },
                )
                .value()
                .unwrap_or(false)
            },
        )
    }
    /// Consume represented C++ `DynamicObject::SetCasterViewpoint()` ->
    /// `Player::SetViewpoint(..., true)` evidence into this live session's
    /// represented `Player::m_seer` seam after the canonical map mutation has
    /// returned and no map lock is held by this caller.
    ///
    /// C++ anchors: `SpellEffects.cpp:2237-2261`, `DynamicObject.cpp:209-225`,
    /// `Player.cpp:25344-25387`, `Player.h:2432,2438`, and
    /// `Player.cpp:23343-23349`. Ownership remains canonical `Map::map_objects`;
    /// the session consumes the outcome for packet publication and test
    /// fixtures; production visibility derives the seer from Player state.
    pub(in crate::session) fn send_set_viewpoint_target_visibility_like_cpp(
        &mut self,
        dynamic_object_guid: ObjectGuid,
    ) -> bool {
        if self
            .core
            .client_visible_guids_like_cpp
            .contains(&dynamic_object_guid)
        {
            return false;
        }

        let Some(player_map_key) = self.core.current_canonical_player_map_key_like_cpp() else {
            return false;
        };
        let Ok(packet_map_id) = u16::try_from(player_map_key.map_id) else {
            return false;
        };
        let Some(manager) = self.core.canonical_map_manager.as_ref() else {
            return false;
        };
        let Ok(manager) = manager.lock() else {
            return false;
        };
        let Some(map) = manager.find_map(player_map_key.map_id, player_map_key.instance_id) else {
            return false;
        };
        let Some(dynamic_object) = map.map().get_typed_dynamic_object(dynamic_object_guid) else {
            return false;
        };
        if dynamic_object.world().map_id() != player_map_key.map_id {
            return false;
        }

        let create_data = crate::session::dynamic_object_create_data_from_canonical_like_cpp(
            dynamic_object_guid,
            dynamic_object,
        );
        drop(manager);

        let block =
            wow_packet::packets::update::UpdateObject::create_dynamic_object_block(create_data);
        self.send_packet(
            &wow_packet::packets::update::UpdateObject::create_world_objects(
                vec![block],
                packet_map_id,
            ),
        );
        self.core
            .client_visible_guids_like_cpp
            .insert(dynamic_object_guid);
        true
    }
    pub(in crate::session) fn consume_add_farsight_set_seer_outcome_like_cpp(
        &mut self,
        outcome: &wow_map::map::FarsightDynamicObjectCreateOutcomeLikeCpp,
    ) -> bool {
        if outcome.status != wow_map::map::FarsightDynamicObjectCreateStatusLikeCpp::Created {
            return false;
        }
        let Some(dynamic_object_guid) = outcome.dynamic_object_guid else {
            return false;
        };
        let Some(caster_viewpoint) = outcome.caster_viewpoint.as_ref() else {
            return false;
        };
        if !caster_viewpoint.apply || caster_viewpoint.dynamic_object_guid != dynamic_object_guid {
            return false;
        }
        let player_set_viewpoint = &caster_viewpoint.player_set_viewpoint;
        if !player_set_viewpoint.apply
            || player_set_viewpoint.target_guid != dynamic_object_guid
            || !player_set_viewpoint.set_seer_requested
            || player_set_viewpoint.status != wow_map::map::PlayerSetViewpointStatusLikeCpp::Applied
        {
            return false;
        }

        let Some(player_guid) = self.player_guid() else {
            return false;
        };

        {
            let (s, h) = crate::session::split_visibility_ref(self);
            s.send_active_player_farsight_object_values_update_like_cpp(
                h,
                player_guid,
                dynamic_object_guid,
            )
        };
        // C++ `Player::SetViewpoint(target, true)` orders the direct
        // `UpdateVisibilityOf(target)` after writing `FarsightObject` and before
        // `SetSeer(target)`, so this represented target-only create is consumed
        // before the next visibility read derives the seer from canonical
        // Player state.
        self.send_set_viewpoint_target_visibility_like_cpp(dynamic_object_guid);
        self.visibility
            .observe_farsight_object_like_cpp(dynamic_object_guid);
        #[cfg(test)]
        {
            self.visibility
                .set_represented_seer_guid_fixture_like_cpp(Some(dynamic_object_guid));
        }
        player_set_viewpoint.update_visibility_requested
    }
    /// C++ `Spell::EffectForceDeselect`.
    ///
    /// Represented boundary: packet construction and effect evidence only.
    /// C++ delivers `SMSG_BREAK_TARGET` and `SMSG_CLEAR_TARGET` to hostile
    /// visible clients around the caster, then stops attacker pets without
    /// threat lists. The hostile visible-set fanout and attacker traversal are
    /// deferred to the live visibility/combat runtime.
    pub(in crate::session) fn apply_force_deselect_effect_like_cpp(&mut self) -> bool {
        let Some(caster_guid) = self.player_guid() else {
            return false;
        };

        #[cfg(test)]
        {
            use wow_packet::ServerPacket;
            let break_target_packet_bytes = wow_packet::packets::combat::BreakTarget {
                unit_guid: caster_guid,
            }
            .to_bytes();
            let clear_target_packet_bytes =
                wow_packet::packets::spell::ClearTarget { guid: caster_guid }.to_bytes();

            self.social
                .record_represented_force_deselect_for_test_like_cpp(
                    RepresentedForceDeselectLikeCpp {
                        caster_guid,
                        visibility_range_yards: DEFAULT_VISIBILITY_DISTANCE_YARDS_LIKE_CPP,
                        break_target_packet_bytes,
                        clear_target_packet_bytes,
                        hostile_visible_fanout_unrepresented: true,
                        attacker_pet_attack_stop_unrepresented: true,
                    },
                );
        }
        #[cfg(not(test))]
        let _ = caster_guid;
        true
    }
    pub(in crate::session) async fn apply_kill_credit_effect_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        target_guid: ObjectGuid,
        creature_entry: i32,
        group_reward_like_cpp: bool,
    ) -> Result<(), &'static str> {
        let player_guid = self.player_guid().ok_or("No player GUID")?;
        if target_guid != player_guid {
            return Ok(());
        }

        let Ok(creature_entry) = u32::try_from(creature_entry) else {
            debug!(
                account = self.core.account_id,
                "Skipping represented kill-credit spell effect with negative MiscValue"
            );
            return Ok(());
        };
        if creature_entry == 0 {
            return Ok(());
        }

        if group_reward_like_cpp {
            debug!(
                account = self.core.account_id,
                creature_entry,
                "Represented SPELL_EFFECT_KILL_CREDIT2 applies current-session credit only; C++ group fanout remains unrepresented"
            );
        }
        self.on_creature_killed_with_generator_like_cpp(
            item_guid_generator,
            creature_entry,
            ObjectGuid::EMPTY,
        )
        .await;
        Ok(())
    }
}
