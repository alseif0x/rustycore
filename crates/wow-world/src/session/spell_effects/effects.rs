//! Remaining represented effect operations owned by this responsibility.
//!
//! Moved out of the Session root under #621. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    /// Flush a [`RuntimeOutput`] to the session send channel.
    ///
    /// Must be called outside any map lock. `flume` bounded `send` may block if
    /// the channel is full — behaviour is identical to the previous direct sends.
    pub(crate) fn flush_runtime_output(&self, out: RuntimeOutput) {
        for pkt in out.packets {
            let _ = self.send_tx().send(pkt);
        }
    }
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
    pub(in crate::session) fn apply_play_movie_effect_like_cpp(
        &mut self,
        target_guid: ObjectGuid,
        movie_id: i32,
    ) {
        let Some(player_guid) = self.player_guid() else {
            return;
        };
        if target_guid != player_guid {
            return;
        }

        let Ok(movie_id) = u32::try_from(movie_id) else {
            return;
        };
        let Some(movie_store) = self.movie_store.as_ref() else {
            return;
        };
        if movie_store.get(movie_id).is_none() {
            return;
        }

        if self
            .mutate_player_cinematic_state_like_cpp(|state| state.movie_id = Some(movie_id))
            .is_none()
        {
            return;
        }
        self.send_packet(&wow_packet::packets::misc::TriggerMovie { movie_id });
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
            self.player_map_id_like_cpp(),
            &update,
        ) {
            self.send_packet(&packet);
        }
    }
    /// C++ `Spell::EffectUpgradeHeirloom`.
    pub(in crate::session) fn apply_upgrade_heirloom_effect_like_cpp(
        &mut self,
        metadata: SpellCastMetadata,
    ) {
        let Ok(item_id) = u32::try_from(metadata.misc[0]) else {
            return;
        };
        let Some(cast_item_entry) = metadata.cast_item_entry else {
            return;
        };
        let Some(update) = self.upgrade_account_heirloom_like_cpp(item_id, cast_item_entry as i32)
        else {
            return;
        };
        let Some(player_guid) = self.player_guid() else {
            return;
        };
        if let Some(packet) = player_values_update_to_update_object(
            player_guid,
            self.player_map_id_like_cpp(),
            &update,
        ) {
            self.send_packet(&packet);
        }
    }
    /// C++ `Spell::EffectGrantBattlePetExperience`.
    pub(in crate::session) async fn apply_grant_battle_pet_experience_effect_like_cpp(
        &mut self,
        metadata: SpellCastMetadata,
        xp: u16,
    ) {
        let Some(pet_guid) = metadata.unit_target_battle_pet_companion_guid else {
            return;
        };

        let _ = self
            .battle_pet_grant_experience_durable_like_cpp(
                pet_guid,
                xp,
                RepresentedBattlePetXpSourceLikeCpp::SpellEffect,
                1.0,
            )
            .await;
    }
    /// C++ `Spell::EffectGrantBattlePetLevel`.
    pub(in crate::session) async fn apply_grant_battle_pet_level_effect_like_cpp(
        &mut self,
        metadata: SpellCastMetadata,
        granted_levels: u16,
    ) {
        let Some(pet_guid) = metadata.unit_target_battle_pet_companion_guid else {
            return;
        };

        let _ = self
            .battle_pet_grant_level_durable_like_cpp(pet_guid, granted_levels)
            .await;
    }
    pub(in crate::session) fn represented_disallowed_mount_form_like_cpp(
        &self,
    ) -> Option<Option<u32>> {
        if self.represented_transform_spell_allows_mount_like_cpp()? {
            return Some(None);
        }
        let visible_auras = self.resolved_player_visible_auras_like_cpp()?;

        if let (Some(spell_store), Some(shapeshift_form_store)) = (
            self.spell_store(),
            self.spell_catalogs.spell_shapeshift_form_store.as_ref(),
        ) {
            for aura in visible_auras.values() {
                let Some(spell_info) = spell_store.get(aura.spell_id) else {
                    continue;
                };

                for effect in spell_info
                    .effects()
                    .iter()
                    .filter(|effect| effect.is_mod_shapeshift_aura_like_cpp())
                {
                    let Ok(form_id) = u32::try_from(effect.effect_misc_value_1) else {
                        continue;
                    };
                    let Some(form) = shapeshift_form_store.get(form_id) else {
                        return Some(Some(form_id));
                    };
                    if form.flags & SPELL_SHAPESHIFT_FORM_FLAG_STANCE_LIKE_CPP == 0 {
                        return Some(Some(form_id));
                    }
                }
            }
        }

        let (display_id, native_display_id) = self.canonical_player_display_ids_like_cpp()?;
        if display_id == 0 || display_id == native_display_id {
            return Some(None);
        }

        let Some(display_store) = self.creatures.display_info_store.as_ref() else {
            return Some(None);
        };
        let Some(display_extra_store) = self.creatures.display_info_extra_store.as_ref() else {
            return Some(None);
        };
        let Some(display) = display_store.get(display_id) else {
            return Some(Some(display_id));
        };
        let Ok(display_extra_id) = u32::try_from(display.extended_display_info_id) else {
            return Some(Some(display_id));
        };
        let Some(display_extra) = display_extra_store.get(display_extra_id) else {
            return Some(Some(display_id));
        };

        if let (Some(model_store), Some(chr_races_store)) = (
            self.creatures.model_data_store.as_ref(),
            self.chr.races_store.as_ref(),
        ) {
            let model_cannot_mount =
                model_store
                    .get(u32::from(display.model_id))
                    .is_some_and(|model| {
                        model.flags
                            & CREATURE_MODEL_DATA_FLAG_CAN_MOUNT_WHILE_TRANSFORMED_AS_THIS_LIKE_CPP
                            == 0
                    });
            let race_cannot_mount = u32::try_from(display_extra.display_race_id)
                .ok()
                .and_then(|race_id| chr_races_store.get(race_id))
                .is_some_and(|race| race.flags & CHR_RACES_FLAG_CAN_MOUNT_LIKE_CPP == 0);
            if model_cannot_mount && race_cannot_mount {
                return Some(Some(display_id));
            }
        }

        Some(None)
    }
    fn represented_transform_spell_allows_mount_like_cpp(&self) -> Option<bool> {
        let spell_store = self.spell_store()?;
        let visible_auras = self.resolved_player_visible_auras_like_cpp()?;

        Some(visible_auras.values().any(|aura| {
            let Some(spell_info) = spell_store.get(aura.spell_id) else {
                return false;
            };
            let is_transform_spell = spell_info.effects().iter().any(|effect| {
                effect.effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA
                    && effect.effect_aura == wow_data::spell::aura_types::SPELL_AURA_TRANSFORM
            });
            is_transform_spell
                && spell_store.has_attribute0_like_cpp(
                    aura.spell_id,
                    wow_data::spell::attributes::SPELL_ATTR0_ALLOW_WHILE_MOUNTED,
                )
        }))
    }
    pub(in crate::session) fn set_homebind_like_cpp(
        &mut self,
        binder_id: ObjectGuid,
        homebind: RepresentedHomebindLikeCpp,
    ) {
        if !self.set_represented_homebind_like_cpp(homebind) {
            return;
        }
        self.persist_player_homebind_like_cpp(homebind);

        self.send_packet(&wow_packet::packets::misc::BindPointUpdate {
            x: homebind.position.x,
            y: homebind.position.y,
            z: homebind.position.z,
            map_id: homebind.map_id,
            area_id: homebind.area_id,
        });
        self.send_packet_realm(&wow_packet::packets::misc::PlayerBound {
            binder_id,
            area_id: homebind.area_id,
        });
    }
    pub(crate) fn represented_homebind_like_cpp(&self) -> Option<RepresentedHomebindLikeCpp> {
        let canonical = self
            .with_owned_player_like_cpp(|player| player.gameplay_state().homebind)
            .flatten();
        if canonical.is_some() {
            return canonical;
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            return self.represented_homebind_like_cpp;
        }
        None
    }
    pub(crate) fn set_represented_homebind_like_cpp(
        &mut self,
        homebind: RepresentedHomebindLikeCpp,
    ) -> bool {
        let canonical = self
            .with_owned_player_mut_like_cpp(|player| {
                player.gameplay_state_mut().homebind = Some(homebind)
            })
            .is_some();
        if canonical {
            return true;
        }
        #[cfg(test)]
        if self.player_handle_like_cpp.is_none() {
            self.represented_homebind_like_cpp = Some(homebind);
            return true;
        }
        false
    }
    pub(in crate::session) fn represented_object_target_position_like_cpp(
        &self,
        target_data: &SpellTargetData,
    ) -> Option<Position> {
        let target_guid = target_data.unit;
        if target_guid.is_empty() {
            return None;
        }
        let player_map_key = self.current_canonical_player_map_key_like_cpp()?;
        let manager = self.canonical_map_manager.as_ref()?;
        let manager = manager.lock().ok()?;
        let map = manager.find_map(player_map_key.map_id, player_map_key.instance_id)?;
        if let Some(player) = map.map().get_typed_player(target_guid) {
            return Some(player.unit().world().position());
        }
        if let Some(creature) = map
            .map()
            .creature_transform_vitals_snapshot_like_cpp(target_guid)
        {
            return Some(creature.position);
        }
        if let Some(gameobject) = map.map().get_typed_game_object(target_guid) {
            return Some(gameobject.world().position());
        }
        map.map()
            .get_typed_dynamic_object(target_guid)
            .map(|dynamic_object| dynamic_object.world().position())
    }
    pub(in crate::session) fn represented_nearby_candidate_meets_implicit_conditions_like_cpp(
        &self,
        candidate: &WorldObject,
        candidate_unit_snapshot: Option<crate::conditions::ConditionUnitSnapshot>,
        implicit_conditions: Option<&[wow_data::Condition]>,
        caster_object: Option<&WorldObject>,
        player_unit_snapshot: crate::conditions::ConditionUnitSnapshot,
        player_snapshot: crate::conditions::ConditionPlayerSnapshot,
        player_condition_store: Option<&wow_data::PlayerConditionStore>,
        player_condition_context: Option<&RepresentedPlayerConditionContextLikeCpp>,
        area_table_store: Option<&wow_data::AreaTableStore>,
    ) -> bool {
        let Some(conditions) = implicit_conditions.filter(|conditions| !conditions.is_empty())
        else {
            return true;
        };
        let Some(condition_store) = self.condition_store.as_ref() else {
            return false;
        };

        // C++ `WorldObjectSpellTargetCheck` builds `ConditionSourceInfo(nullptr, caster)`
        // and then assigns the tested target to condition slot 0.
        let mut source_info = crate::conditions::ConditionSourceInfo::from_targets(
            Some(candidate),
            caster_object,
            None,
        );
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

        crate::conditions::is_object_meet_to_conditions_like_cpp(
            &mut source_info,
            conditions,
            condition_store.as_ref(),
            |condition, source_info| {
                crate::conditions::condition_meets_basic_like_cpp(
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
    pub(in crate::session) fn implicit_target_conditions_like_cpp(
        &self,
        spell_id: i32,
        effect_index: u32,
    ) -> Option<Arc<wow_data::ConditionContainer>> {
        self.spell_catalogs
            .spell_store
            .as_deref()
            .and_then(|store| store.implicit_target_conditions_like_cpp(spell_id, effect_index))
            .and_then(|conditions| conditions.upgrade())
    }
    pub(in crate::session) fn has_implicit_target_conditions_like_cpp(
        &self,
        spell_id: i32,
        effect_index: u32,
    ) -> bool {
        self.implicit_target_conditions_like_cpp(spell_id, effect_index)
            .is_some_and(|conditions| !conditions.is_empty())
    }
    /// Consume represented C++ `DynamicObject::SetCasterViewpoint()` ->
    /// `Player::SetViewpoint(..., true)` evidence into this live session's
    /// represented `Player::m_seer` seam after the canonical map mutation has
    /// returned and no map lock is held by this caller.
    ///
    /// C++ anchors: `SpellEffects.cpp:2237-2261`, `DynamicObject.cpp:209-225`,
    /// `Player.cpp:25344-25387`, `Player.h:2432,2438`, and
    /// `Player.cpp:23343-23349`. Ownership remains canonical `Map::map_objects`;
    /// sync direction is map outcome -> session represented `m_seer` only.
    pub(in crate::session) fn send_set_viewpoint_target_visibility_like_cpp(
        &mut self,
        dynamic_object_guid: ObjectGuid,
    ) -> bool {
        if self
            .client_visible_guids_like_cpp
            .contains(&dynamic_object_guid)
        {
            return false;
        }

        let Some(player_map_key) = self.current_canonical_player_map_key_like_cpp() else {
            return false;
        };
        let Ok(packet_map_id) = u16::try_from(player_map_key.map_id) else {
            return false;
        };
        let Some(manager) = self.canonical_map_manager.as_ref() else {
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

        let create_data = crate::session_rules::dynamic_object_create_data_from_canonical_like_cpp(
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
        self.client_visible_guids_like_cpp
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

        self.send_active_player_farsight_object_values_update_like_cpp(
            player_guid,
            dynamic_object_guid,
        );
        // C++ `Player::SetViewpoint(target, true)` orders the direct
        // `UpdateVisibilityOf(target)` after writing `FarsightObject` and before
        // `SetSeer(target)`, so this represented target-only create is consumed
        // before updating the session-local represented `m_seer`.
        self.send_set_viewpoint_target_visibility_like_cpp(dynamic_object_guid);
        self.represented_seer_guid_like_cpp = Some(dynamic_object_guid);
        player_set_viewpoint.update_visibility_requested
    }
    /// C++ `Spell::EffectAddExtraAttacks`.
    ///
    /// Represented boundary: current canonical player target only. This stores
    /// C++ `Unit::AddExtraAttacks` state on the target unit; consuming the
    /// queued extra swings during melee update and combat-log emission remains
    /// outside this bounded slice.
    pub(in crate::session) fn apply_add_extra_attacks_effect_like_cpp(
        &mut self,
        damage: i32,
        target_guid: ObjectGuid,
    ) -> bool {
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        if target_guid != player_guid || self.resolved_player_is_alive_like_cpp() != Some(true) {
            return false;
        }
        let count = damage as u32;
        self.mutate_canonical_player_like_cpp(|player| {
            player
                .unit_mut()
                .add_extra_attacks_like_cpp(count)
                .is_some()
        })
        .unwrap_or(false)
    }
    /// C++ `Spell::EffectInebriate`.
    ///
    /// Represented boundary: current canonical player target/caster only. This
    /// updates `PlayerData::Inebriation` with C++ clamp semantics. Drunken
    /// Vomit (`67468`), fake-inebriate aura/invisibility side effects,
    /// sobering timer reset and `SMSG_CROSSED_INEBRIATION_THRESHOLD` fanout
    /// remain outside this bounded slice.
    pub(in crate::session) fn apply_inebriate_effect_like_cpp(
        &mut self,
        damage: i32,
        target_guid: ObjectGuid,
    ) -> bool {
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        if target_guid != player_guid {
            return false;
        }

        self.mutate_canonical_player_like_cpp(|player| {
            let current = i32::from(player.inebriation_like_cpp());
            let next = (current + damage).clamp(0, 100) as u8;
            if next == player.inebriation_like_cpp() {
                return false;
            }
            player.set_inebriation_like_cpp(next);
            true
        })
        .unwrap_or(false)
    }
    /// C++ `Spell::EffectDismissPet`.
    ///
    /// Represented boundary: current represented pet target only. Full C++
    /// `Unit::IsPet` / `Pet::Remove(PET_SAVE_NOT_IN_SLOT)` requires the live
    /// pet runtime; this slice preserves the hit-target no-op semantics and
    /// clears the represented active pet state when the target is that pet.
    pub(in crate::session) fn apply_dismiss_pet_effect_like_cpp(
        &mut self,
        target_guid: ObjectGuid,
    ) -> bool {
        if self.player_pet_guid_state_like_cpp().flatten() != Some(target_guid) {
            return false;
        }

        self.invalidate_represented_character_pet_empty_authority_like_cpp();
        let _ = self.set_player_pet_guid_like_cpp(None);
        #[cfg(test)]
        {
            self.represented_pet_react_state_like_cpp =
                wow_packet::packets::pet::REACT_DEFENSIVE_LIKE_CPP;
            self.represented_pet_command_state_like_cpp =
                wow_packet::packets::pet::COMMAND_FOLLOW_LIKE_CPP;
        }
        true
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

            self.represented_force_deselects_like_cpp
                .push(RepresentedForceDeselectLikeCpp {
                    caster_guid,
                    visibility_range_yards: DEFAULT_VISIBILITY_DISTANCE_YARDS_LIKE_CPP,
                    break_target_packet_bytes,
                    clear_target_packet_bytes,
                    hostile_visible_fanout_unrepresented: true,
                    attacker_pet_attack_stop_unrepresented: true,
                });
        }
        #[cfg(not(test))]
        let _ = caster_guid;
        true
    }
    pub(in crate::session) fn apply_dual_wield_effect_like_cpp(
        &mut self,
        target_guid: ObjectGuid,
    ) -> Result<(), &'static str> {
        let player_guid = self.player_guid().ok_or("No player GUID")?;
        if target_guid != player_guid {
            return Ok(());
        }

        let _ = self.mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_can_dual_wield_like_cpp(true);
        });

        Ok(())
    }
    pub(in crate::session) fn apply_titan_grip_effect_like_cpp(
        &mut self,
        penalty_spell_id: i32,
    ) -> Result<(), &'static str> {
        let _ = self.player_guid().ok_or("No player GUID")?;

        let penalty_spell_id = u32::try_from(penalty_spell_id).unwrap_or(0);
        let _ = self.mutate_canonical_player_like_cpp(|player| {
            player.set_can_titan_grip(true, penalty_spell_id);
        });

        Ok(())
    }
    pub(in crate::session) fn apply_parry_effect_like_cpp(&mut self) -> Result<(), &'static str> {
        let _ = self.player_guid().ok_or("No player GUID")?;

        let _ = self.mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_can_parry_like_cpp(true);
        });

        Ok(())
    }
    pub(in crate::session) fn apply_block_effect_like_cpp(&mut self) -> Result<(), &'static str> {
        let _ = self.player_guid().ok_or("No player GUID")?;

        let _ = self.mutate_canonical_player_like_cpp(|player| {
            player.unit_mut().set_can_block_like_cpp(true);
        });

        Ok(())
    }
    pub(in crate::session) fn apply_give_honor_effect_like_cpp(
        &mut self,
        damage: i32,
        target_guid: ObjectGuid,
    ) -> Result<(), &'static str> {
        let player_guid = self.player_guid().ok_or("No player GUID")?;
        if target_guid != player_guid {
            return Ok(());
        }

        let honor_update = self.add_honor_xp_to_current_player_like_cpp(damage);

        // C++ `Spell::EffectGiveHonor` sets only `Honor` and `OriginalHonor`
        // before `SendDirectMessage(packet.Write())`; `Target` and `Rank`
        // remain default-initialized.
        self.send_packet(&wow_packet::packets::combat::PvpCredit {
            original_honor: damage,
            honor: damage,
            target: ObjectGuid::EMPTY,
            rank: 0,
        });
        if let Some(update) = honor_update
            && let Some(packet) = player_values_update_to_update_object(
                player_guid,
                self.player_map_id_like_cpp(),
                &update,
            )
        {
            self.send_packet(&packet);
        }
        Ok(())
    }
    /// C++ `Spell::EffectModifyCooldown`.
    ///
    /// Represented boundary: the current Rust runtime exposes `SpellHistory`
    /// through the canonical player seam only. Other unit targets remain a
    /// no-op until generic unit spell histories are live.
    pub(in crate::session) fn apply_modify_cooldown_effect_like_cpp(
        &mut self,
        damage: i32,
        effect_trigger_spell: i32,
        target_guid: ObjectGuid,
    ) -> bool {
        let Some(player_guid) = self.player_guid() else {
            return false;
        };
        if target_guid != player_guid {
            return false;
        }
        let Ok(trigger_spell_id) = u32::try_from(effect_trigger_spell) else {
            return false;
        };
        if trigger_spell_id == 0 {
            return false;
        }

        let now_ms = u64::from(crate::session_rules::game_time_ms_like_cpp());
        self.mutate_canonical_player_like_cpp(|player| {
            player
                .unit_mut()
                .subsystems_mut()
                .spells
                .history
                .modify_cooldown(trigger_spell_id, i64::from(damage), false, now_ms)
        })
        .unwrap_or(false)
    }
    pub(crate) fn current_map_is_dungeon_like_cpp(&self) -> bool {
        self.current_map_dungeon_state_like_cpp().unwrap_or(false)
    }
    /// Resolve the current map's C++ `MapEntry::IsDungeon` classification
    /// without conflating missing Map.db2 metadata with an overworld map.
    pub(crate) fn current_map_dungeon_state_like_cpp(&self) -> Option<bool> {
        self.maps
            .store
            .as_ref()
            .and_then(|store| store.get(u32::from(self.player_map_id_like_cpp())))
            .map(|entry| entry.is_dungeon())
    }
    /// C++ `Spell::EffectDistract` / `SPELL_EFFECT_DISTRACT`.
    pub(in crate::session) fn apply_distract_effect_like_cpp(
        &mut self,
        damage: i32,
        target_guid: ObjectGuid,
        target_data: &SpellTargetData,
    ) -> Result<(), &'static str> {
        use wow_packet::ServerPacket;
        use wow_packet::packets::movement::{MonsterMove, MovementMonsterSpline};

        let Some(destination) = target_data.dst_location.map(|location| location.position) else {
            return Ok(());
        };
        let duration_ms = u32::try_from(damage.max(0))
            .unwrap_or(0)
            .saturating_mul(1_000);

        let packet = self
            .mutate_world_creature(target_guid, |creature| {
                if creature.creature.ai_ownership().combat_target.is_some() {
                    return None;
                }
                if creature.creature.unit().has_unit_state(
                    (UnitState::CONFUSED | UnitState::STUNNED | UnitState::FLEEING).bits(),
                ) {
                    return None;
                }

                let orientation = creature.position().angle_to(&destination);
                let (_, from, spline) =
                    creature.begin_distract_movement_like_cpp(duration_ms, orientation)?;
                Some(
                    MonsterMove {
                        mover_guid: target_guid,
                        current_pos: from,
                        spline: MovementMonsterSpline::from_move_spline(&spline),
                    }
                    .to_bytes(),
                )
            })
            .flatten();

        if let Some(packet) = packet {
            let _ = self.send_tx().send(packet);
        }

        Ok(())
    }
    /// C++ `Spell::EffectSanctuary`.
    pub(in crate::session) fn apply_sanctuary_effect_like_cpp(
        &mut self,
        target_guid: ObjectGuid,
    ) -> Result<(), &'static str> {
        if target_guid.is_empty() {
            return Ok(());
        }

        if target_guid.is_player() && !self.current_map_is_dungeon_like_cpp() {
            self.stop_represented_player_pve_combat_like_cpp(target_guid);
            return Ok(());
        }

        let owner_guids = self.canonical_threatened_by_me_owner_guids_like_cpp(target_guid);
        for owner_guid in owner_guids {
            self.scale_canonical_owner_threat_to_target_zero_like_cpp(owner_guid, target_guid);
            let _ = self.mutate_world_creature(owner_guid, |owner| {
                owner
                    .creature
                    .unit_mut()
                    .subsystems_mut()
                    .combat
                    .scale_threat(target_guid, 0.0);
            });
        }
        Ok(())
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
                account = self.account_id,
                "Skipping represented kill-credit spell effect with negative MiscValue"
            );
            return Ok(());
        };
        if creature_entry == 0 {
            return Ok(());
        }

        if group_reward_like_cpp {
            debug!(
                account = self.account_id,
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
    pub(in crate::session) async fn apply_instakill_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        spell_id: i32,
        target_guid: ObjectGuid,
    ) -> Result<(), &'static str> {
        let caster = self.player_guid().ok_or("No player GUID")?;
        let Some(current_hp) = self
            .mutate_world_creature(target_guid, |creature| {
                creature.is_alive().then_some(creature.current_hp())
            })
            .flatten()
        else {
            return Ok(());
        };

        self.send_packet(&wow_packet::packets::combat::SpellInstakillLog {
            target: target_guid,
            caster,
            spell_id,
        });
        self.apply_damage_with_generator_like_cpp(
            item_guid_generator,
            Some(spell_id),
            target_guid,
            current_hp,
        )
        .await
    }
}
