//! The represented spell execution entry point that drives effect application.
//!
//! Moved out of the Session root under #621. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

impl WorldSession {
    pub async fn execute_spell_with_visual_and_target_data_with_metadata_and_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        creature_spawn_catalogs: &CreatureSpawnCatalogsLikeCpp,
        spell_id: i32,
        target_guid: ObjectGuid,
        cast_id: ObjectGuid,
        spell_visual: wow_packet::packets::spell::SpellCastVisual,
        target_data: SpellTargetData,
        metadata: SpellCastMetadata,
    ) -> Result<(), &'static str> {
        let player_guid = self.player_guid().ok_or("No player GUID")?;
        let caster_guid = metadata.caster_guid_override.unwrap_or(player_guid);

        // Obtener SpellInfo
        let spell_info = self
            .spell_store()
            .and_then(|store| store.get(spell_id))
            .cloned()
            .ok_or("Spell not found")?;
        let effect_type = spell_info.effect_type;
        let effect_base_points = spell_info.effect_base_points;

        info!(
            account = self.account_id,
            spell_id = spell_id,
            target = ?target_guid,
            effect_type = effect_type,
            "Executing spell effect"
        );

        let Some(represented_focus_object) = self.check_represented_cast_preparation_like_cpp(
            &spell_info,
            cast_id,
            &spell_visual,
            metadata,
        )?
        else {
            self.publish_player_cast_interrupted_frames_like_cpp(
                metadata,
                cast_id,
                spell_id,
                &spell_visual,
            );
            return Ok(());
        };

        if self.represented_gameobject_summon_missing_nearby_entry_destination_like_cpp(
            spell_id,
            &spell_info,
            &target_data,
            represented_focus_object,
        ) {
            // C++ `Spell::SelectImplicitNearbyTargets` sends
            // `SPELL_FAILED_BAD_IMPLICIT_TARGETS` and finishes before
            // `SMSG_SPELL_GO` when non-OR_DB nearby-entry destination targets
            // cannot resolve a nearby object.
            self.send_packet(&wow_packet::packets::spell::CastFailed {
                cast_id,
                spell_id,
                visual: spell_visual.clone(),
                reason: SpellCastResult::BadImplicitTargets as i32,
                fail_arg1: 0,
                fail_arg2: 0,
            });
            debug!(
                account = self.account_id,
                spell_id = spell_id,
                "Failing represented GameObject summon because C++ nearby-entry destination search found no target"
            );
            self.publish_player_cast_interrupted_frames_like_cpp(
                metadata,
                cast_id,
                spell_id,
                &spell_visual,
            );
            return Ok(());
        }

        if metadata.from_client
            && !self.take_spell_power_like_cpp(&spell_info, cast_id, spell_id, &spell_visual)
        {
            if metadata.restore_last_spell_cast_time_on_power_failure {
                let _ = self.mutate_cast_execution_like_cpp(|state| {
                    state.last_cast_time = metadata.previous_last_spell_cast_time_on_power_failure;
                });
            }
            self.publish_player_cast_interrupted_frames_like_cpp(
                metadata,
                cast_id,
                spell_id,
                &spell_visual,
            );
            return Ok(());
        }

        // C++ `Spell::SelectSpellTargets` resolves implicit destinations in
        // effect/target order before `Spell::SendSpellGo`; later selections
        // replace earlier ones through `SpellCastTargets::ModDst`.
        let mut target_data = target_data;
        let mut represented_implicit_destination = false;
        let mut effect_target_data_like_cpp = Vec::new();
        for effect in spell_info.effects() {
            if effect.effect == 0 {
                continue;
            }
            for implicit_target in [effect.implicit_target_1, effect.implicit_target_2] {
                let mut isolated_effect = effect.clone();
                isolated_effect.implicit_target_1 = implicit_target;
                isolated_effect.implicit_target_2 = 0;
                let mut selection_input = target_data.clone();
                selection_input.flags &= !0x0000_0040;
                selection_input.dst_location = None;
                selection_input.map_id = None;
                let selected = match implicit_target {
                    wow_data::spell::implicit_targets::TARGET_DEST_HOME => self
                        .represented_home_destination_target_data_like_cpp(
                            caster_guid,
                            &isolated_effect,
                            &selection_input,
                        ),
                    wow_data::spell::implicit_targets::TARGET_DEST_DB => self
                        .represented_db_caster_destination_target_data_like_cpp(
                            caster_guid,
                            &spell_info,
                            &isolated_effect,
                            &selection_input,
                        ),
                    wow_data::spell::implicit_targets::TARGET_DEST_NEARBY_ENTRY
                    | wow_data::spell::implicit_targets::TARGET_DEST_NEARBY_ENTRY_2
                    | wow_data::spell::implicit_targets::TARGET_DEST_NEARBY_ENTRY_OR_DB => self
                        .represented_focus_destination_target_data_like_cpp(
                            spell_id,
                            &isolated_effect,
                            &selection_input,
                            represented_focus_object,
                        )
                        .or_else(|| {
                            self.represented_nearby_entry_destination_target_data_like_cpp(
                                spell_id,
                                &isolated_effect,
                                &selection_input,
                            )
                        }),
                    _ => None,
                };
                if let Some(selected) = selected {
                    target_data = selected;
                    represented_implicit_destination = true;
                }
            }
            // C++ snapshots `m_targets.GetDst()` into
            // `m_destTargets[EffectIndex]` after selecting each effect.
            let mut effect_target_data = target_data.clone();
            if let Some(destination) = effect_target_data.dst_location.as_mut()
                && !destination.transport.is_empty()
                && let Some(world_position) = self
                    .represented_transport_destination_world_position_like_cpp(
                        destination.transport,
                        destination.position,
                    )
            {
                // C++ serializes `_transportOffset` but keeps `_position` as
                // world coordinates for effect execution.
                destination.position = world_position;
            }
            effect_target_data_like_cpp.push((effect.effect_index, effect_target_data));
        }
        let mut spell_go_target_data = target_data.clone();
        if represented_implicit_destination {
            // C++ retains map identity on SpellDestination for effect
            // execution, while SpellCastTargets::Write emits DstLocation XYZ.
            spell_go_target_data.map_id = None;
        }

        // Send SMSG_SPELL_GO
        use wow_packet::packets::spell::SpellGoPkt;

        let spell_visual_id = spell_visual.spell_visual_id;
        // C++ `SendSpellGo` samples the power that remains after the debit
        // above. Triggered consumers keep their own explicit metadata flags;
        // normal client defaults are never imposed on them.
        let go_phase = player_cast::wire::PlayerCastPublicationPhaseLikeCpp::Go;
        let (cast_data, cast_flags) = if metadata.client_cast_id.is_some() {
            self.player_cast_publication_like_cpp(&spell_info, &metadata, go_phase)
        } else {
            (Default::default(), metadata.cast_flags)
        };
        let go_pkt = SpellGoPkt {
            cast_data,
            caster: caster_guid,
            cast_id,
            original_cast_id: metadata.original_cast_id_or(cast_id),
            spell_id,
            visual: spell_visual,
            cast_flags,
            cast_flags_ex: metadata.cast_flags_ex,
            cast_time_ms: crate::session_rules::game_time_ms_like_cpp(),
            target: spell_go_target_data,
            hit_targets: vec![target_guid],
            miss_targets: Vec::new(),
        };
        if metadata.client_cast_id.is_some() {
            self.publish_player_cast_frame_like_cpp(
                metadata,
                wow_packet::ServerPacket::to_bytes(&go_pkt),
            );
        } else {
            self.send_packet(&go_pkt);
        }
        if caster_guid != player_guid && caster_guid.is_any_type_creature() {
            self.broadcast_creature_packet_to_visible_set_like_cpp(
                caster_guid,
                wow_packet::ServerPacket::to_bytes(&go_pkt),
            );
        }
        let mut force_visibility_after_add_farsight = false;
        for effect in spell_info.effects() {
            if effect.effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_ADD_FARSIGHT {
                let effect_target_data = effect_target_data_like_cpp
                    .iter()
                    .find(|(effect_index, _)| *effect_index == effect.effect_index)
                    .map(|(_, target_data)| target_data)
                    .unwrap_or(&target_data);
                if let Some(outcome) = self.apply_effect_add_farsight_like_cpp(
                    spell_id,
                    effect,
                    effect_target_data,
                    spell_visual_id,
                    spell_info.cast_time_ms,
                ) {
                    force_visibility_after_add_farsight |=
                        self.consume_add_farsight_set_seer_outcome_like_cpp(&outcome);
                }
            }
        }
        if force_visibility_after_add_farsight {
            self.force_update_visibility_with_catalogs_like_cpp(creature_spawn_catalogs)
                .await;
        }

        for effect in spell_info.effects() {
            let effect_target_data = effect_target_data_like_cpp
                .iter()
                .find(|(effect_index, _)| *effect_index == effect.effect_index)
                .map(|(_, target_data)| target_data)
                .unwrap_or(&target_data);
            self.apply_effect_teleport_units_like_cpp(effect, target_guid, effect_target_data)
                .await;
            self.apply_effect_bind_like_cpp(effect, caster_guid, target_guid, effect_target_data)
                .await;
        }

        if spell_info.effects().is_empty() {
            let primary_effect_like_cpp = wow_data::SpellEffectInfo {
                effect_index: 0,
                effect: effect_type,
                effect_base_points,
                ..Default::default()
            };
            self.apply_effect_teleport_units_like_cpp(
                &primary_effect_like_cpp,
                target_guid,
                &target_data,
            )
            .await;
            self.apply_effect_bind_like_cpp(
                &primary_effect_like_cpp,
                caster_guid,
                target_guid,
                &target_data,
            )
            .await;
        }

        let mut force_visibility_after_gameobject_summon = false;
        for effect in spell_info.effects() {
            let effect_target_data = effect_target_data_like_cpp
                .iter()
                .find(|(effect_index, _)| *effect_index == effect.effect_index)
                .map(|(_, target_data)| target_data)
                .unwrap_or(&target_data);
            if let Some(outcome) = self.apply_effect_summon_object_wild_with_focus_like_cpp(
                spell_id,
                effect,
                effect_target_data,
                represented_focus_object,
            ) {
                if outcome.map_outcome.as_ref().is_some_and(|map_outcome| {
                    map_outcome.status
                        == wow_map::map::SpellEffectSummonObjectWildStatusLikeCpp::CreatedAddedToMap
                }) {
                    force_visibility_after_gameobject_summon = true;
                }
                continue;
            }

            if spell_effect_is_represented_summon_object_slot_like_cpp(effect.effect) {
                if let Some(outcome) = self.apply_effect_summon_object_slot_like_cpp(
                    spell_id,
                    effect,
                    effect_target_data,
                ) {
                    if outcome.map_outcome.as_ref().is_some_and(|map_outcome| {
                        map_outcome.status
                            == wow_map::map::GameObjectSummonObjectForOwnerSlotStatusLikeCpp::CreatedAddedAndSlotted
                    }) {
                        force_visibility_after_gameobject_summon = true;
                    }
                }
            }
        }
        if force_visibility_after_gameobject_summon {
            self.force_update_visibility_with_catalogs_like_cpp(creature_spawn_catalogs)
                .await;
        }

        let direct_spell_effects_like_cpp: Vec<(u32, i32, u32, i32, i32)> =
            if spell_info.effects().is_empty() {
                vec![(effect_type, effect_base_points, 0, 0, 0)]
            } else {
                spell_info
                    .effects()
                    .iter()
                    .filter(|effect| effect.effect != 0)
                    .map(|effect| {
                        (
                            effect.effect,
                            effect.effect_base_points,
                            effect.effect_index,
                            effect.effect_misc_value_1,
                            effect.effect_trigger_spell,
                        )
                    })
                    .collect()
            };
        for (
            direct_effect_type,
            direct_effect_base_points,
            direct_effect_index,
            direct_effect_misc_value_1,
            direct_effect_trigger_spell,
        ) in direct_spell_effects_like_cpp
        {
            let direct_effect_target_data = effect_target_data_like_cpp
                .iter()
                .find(|(effect_index, _)| *effect_index == direct_effect_index)
                .map(|(_, target_data)| target_data)
                .unwrap_or(&target_data);
            match direct_effect_type {
                x if wow_data::spell::spell_effect_types::is_cpp_null_or_unused_noop(x) => {}
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_INSTAKILL => {
                    self.apply_instakill_like_cpp(item_guid_generator, spell_id, target_guid)
                        .await?;
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL => {
                    if let Ok(heal_amount) = u32::try_from(direct_effect_base_points) {
                        self.apply_heal_from_caster_like_cpp(
                            Some(spell_id),
                            caster_guid,
                            target_guid,
                            heal_amount,
                        )
                        .await?;
                    } else {
                        debug!(
                            account = self.account_id,
                            spell_id,
                            effect_index = direct_effect_index,
                            effect_base_points = direct_effect_base_points,
                            "Skipping SPELL_EFFECT_HEAL because C++ EffectHeal returns when damage < 0"
                        );
                    }
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL_MECHANICAL => {
                    if let Ok(heal_amount) = u32::try_from(direct_effect_base_points) {
                        self.apply_heal_from_caster_like_cpp(
                            Some(spell_id),
                            caster_guid,
                            target_guid,
                            heal_amount,
                        )
                        .await?;
                    } else {
                        debug!(
                            account = self.account_id,
                            spell_id,
                            effect_index = direct_effect_index,
                            effect_base_points = direct_effect_base_points,
                            "Skipping SPELL_EFFECT_HEAL_MECHANICAL because C++ EffectHealMechanical returns when damage < 0"
                        );
                    }
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL_MAX_HEALTH => {
                    self.apply_heal_max_health_like_cpp(
                        spell_id,
                        direct_effect_base_points,
                        caster_guid,
                        target_guid,
                    )
                    .await?;
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL_PCT => {
                    self.apply_heal_pct_like_cpp(
                        spell_id,
                        direct_effect_base_points,
                        caster_guid,
                        target_guid,
                    )
                    .await?;
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_ADD_EXTRA_ATTACKS => {
                    self.apply_add_extra_attacks_effect_like_cpp(
                        direct_effect_base_points,
                        target_guid,
                    );
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_INEBRIATE => {
                    self.apply_inebriate_effect_like_cpp(direct_effect_base_points, target_guid);
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_REPUTATION => {
                    self.apply_reputation_effect_like_cpp(
                        direct_effect_base_points,
                        direct_effect_misc_value_1,
                        target_guid,
                    );
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_POWER_DRAIN => {
                    self.apply_power_drain_effect_like_cpp(
                        direct_effect_base_points,
                        direct_effect_misc_value_1,
                        target_guid,
                        false,
                    );
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_ENERGIZE => {
                    self.apply_energize_effect_like_cpp(
                        direct_effect_base_points,
                        direct_effect_misc_value_1,
                        target_guid,
                        false,
                    );
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_ENERGIZE_PCT => {
                    self.apply_energize_effect_like_cpp(
                        direct_effect_base_points,
                        direct_effect_misc_value_1,
                        target_guid,
                        true,
                    );
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_POWER_BURN => {
                    self.apply_power_drain_effect_like_cpp(
                        direct_effect_base_points,
                        direct_effect_misc_value_1,
                        target_guid,
                        true,
                    );
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_HEALTH_LEECH => {
                    self.apply_health_leech_like_cpp(
                        item_guid_generator,
                        spell_id,
                        direct_effect_base_points,
                        target_guid,
                    )
                    .await?;
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_DUAL_WIELD => {
                    self.apply_dual_wield_effect_like_cpp(target_guid)?;
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_TITAN_GRIP => {
                    self.apply_titan_grip_effect_like_cpp(direct_effect_misc_value_1)?;
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_PARRY => {
                    self.apply_parry_effect_like_cpp()?;
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_BLOCK => {
                    self.apply_block_effect_like_cpp()?;
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_PROFICIENCY => {
                    self.apply_proficiency_effect_like_cpp(spell_id)?;
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_GIVE_HONOR => {
                    self.apply_give_honor_effect_like_cpp(direct_effect_base_points, target_guid)?;
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_THREAT => {
                    self.apply_threat_effect_like_cpp(direct_effect_base_points, target_guid)?;
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_DISTRACT => {
                    self.apply_distract_effect_like_cpp(
                        direct_effect_base_points,
                        target_guid,
                        direct_effect_target_data,
                    )?;
                }
                x if x
                    == wow_data::spell::spell_effect_types::SPELL_EFFECT_MODIFY_THREAT_PERCENT =>
                {
                    self.apply_modify_threat_percent_effect_like_cpp(
                        direct_effect_base_points,
                        target_guid,
                    )?;
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_ATTACK_ME => {
                    self.apply_taunt_effect_like_cpp(spell_id, target_guid, true)?;
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_MODIFY_COOLDOWN => {
                    self.apply_modify_cooldown_effect_like_cpp(
                        direct_effect_base_points,
                        direct_effect_trigger_spell,
                        target_guid,
                    );
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_MODIFY_CHARGES => {
                    self.apply_modify_spell_charges_effect_like_cpp(
                        direct_effect_base_points,
                        direct_effect_misc_value_1,
                        target_guid,
                    );
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_SANCTUARY => {
                    self.apply_sanctuary_effect_like_cpp(target_guid)?;
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_SELF_RESURRECT => {
                    self.apply_self_resurrect_effect_like_cpp(
                        direct_effect_base_points,
                        direct_effect_misc_value_1,
                    );
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_STUCK => {
                    self.apply_stuck_effect_like_cpp().await;
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_PLAY_MOVIE => {
                    self.apply_play_movie_effect_like_cpp(target_guid, direct_effect_misc_value_1);
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_DUEL => {
                    self.apply_duel_effect_like_cpp(
                        spell_id,
                        direct_effect_misc_value_1,
                        target_guid,
                    );
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_DISMISS_PET => {
                    self.apply_dismiss_pet_effect_like_cpp(target_guid);
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_FORCE_DESELECT => {
                    self.apply_force_deselect_effect_like_cpp();
                }
                x if x
                    == wow_data::spell::spell_effect_types::SPELL_EFFECT_CHANGE_RAID_MARKER =>
                {
                    self.apply_change_raid_marker_effect_like_cpp(
                        direct_effect_base_points,
                        direct_effect_target_data,
                    );
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_LEARN_SPELL => {
                    self.apply_learn_spell_effect_like_cpp(
                        direct_effect_trigger_spell,
                        target_guid,
                    )
                    .await;
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_LEARN_TRANSMOG_SET => {
                    self.apply_learn_transmog_set_effect_like_cpp(
                        target_guid,
                        direct_effect_misc_value_1,
                    );
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_UPGRADE_HEIRLOOM => {
                    self.apply_upgrade_heirloom_effect_like_cpp(metadata);
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_UNCAGE_BATTLEPET => {
                    self.apply_uncage_battle_pet_effect_like_cpp(spell_id, cast_id, metadata)
                        .await;
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_GRANT_BATTLEPET_LEVEL =>
                {
                    self.apply_grant_battle_pet_level_effect_like_cpp(
                        metadata,
                        direct_effect_base_points as u16,
                    )
                    .await;
                }
                x if x
                    == wow_data::spell::spell_effect_types::SPELL_EFFECT_GRANT_BATTLEPET_EXPERIENCE =>
                {
                    self.apply_grant_battle_pet_experience_effect_like_cpp(
                        metadata,
                        direct_effect_base_points as u16,
                    )
                    .await;
                }
                x if x
                    == wow_data::spell::spell_effect_types::SPELL_EFFECT_CHANGE_BATTLEPET_QUALITY
                    || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_PULL
                    || x
                        == wow_data::spell::spell_effect_types::SPELL_EFFECT_LEARN_TRANSMOG_ILLUSION
                    || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_TRADE_SKILL => {}
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_QUEST_COMPLETE => {
                    self.apply_quest_complete_effect_like_cpp(
                        item_guid_generator,
                        target_guid,
                        direct_effect_misc_value_1,
                    )
                    .await?;
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_KILL_CREDIT => {
                    self.apply_kill_credit_effect_like_cpp(
                        item_guid_generator,
                        target_guid,
                        direct_effect_misc_value_1,
                        false,
                    )
                    .await?;
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_KILL_CREDIT2 => {
                    self.apply_kill_credit_effect_like_cpp(
                        item_guid_generator,
                        target_guid,
                        direct_effect_misc_value_1,
                        true,
                    )
                    .await?;
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE => {
                    if let Ok(damage_amount) = u32::try_from(direct_effect_base_points) {
                        self.apply_damage_from_caster_like_cpp(
                            item_guid_generator,
                            Some(spell_id),
                            caster_guid,
                            target_guid,
                            damage_amount,
                        )
                        .await?;
                    } else {
                        debug!(
                            account = self.account_id,
                            spell_id,
                            effect_index = direct_effect_index,
                            effect_base_points = direct_effect_base_points,
                            "Skipping SPELL_EFFECT_SCHOOL_DAMAGE because C++ only applies positive m_damage"
                        );
                    }
                }
                x if x
                    == wow_data::spell::spell_effect_types::SPELL_EFFECT_ENVIRONMENTAL_DAMAGE =>
                {
                    self.apply_effect_environmental_damage_like_cpp(
                        &wow_data::SpellEffectInfo {
                            effect_index: direct_effect_index,
                            effect: direct_effect_type,
                            effect_base_points: direct_effect_base_points,
                            ..Default::default()
                        },
                        target_guid,
                    );
                }
                _ => {}
            }
        }

        if !spell_info.effects().is_empty() {
            let apply_aura_rows_like_cpp = spell_info
                .effects()
                .iter()
                .filter(|effect| {
                    effect.effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA
                })
                .count();
            let generic_apply_aura_rows_like_cpp = spell_info
                .effects()
                .iter()
                .filter(|effect| {
                    effect.effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA
                        && !effect.is_mounted_aura_like_cpp()
                        && !effect.is_provide_spell_focus_aura_like_cpp()
                        && !effect.is_battle_pet_xp_pct_aura_like_cpp()
                        && effect.effect_aura
                            != wow_data::spell::aura_types::SPELL_AURA_MOD_DETECT_RANGE
                        && effect.effect_aura
                            != wow_data::spell::aura_types::SPELL_AURA_MOD_DETECTED_RANGE
                        && effect.effect_aura
                            != wow_data::spell::aura_types::SPELL_AURA_MOD_RESTED_XP_CONSUMPTION
                        && effect.effect_aura != wow_data::spell::aura_types::SPELL_AURA_MOD_TAUNT
                })
                .count();
            for effect in spell_info.effects().iter().filter(|effect| {
                effect.effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA
            }) {
                if effect.is_mounted_aura_like_cpp() {
                    self.apply_represented_mounted_aura_like_cpp(spell_id, player_guid, effect)?;
                } else if effect.is_provide_spell_focus_aura_like_cpp() {
                    self.apply_represented_provide_spell_focus_aura_like_cpp(
                        spell_id,
                        player_guid,
                        effect,
                    )?;
                } else if effect.is_battle_pet_xp_pct_aura_like_cpp() {
                    self.apply_represented_battle_pet_xp_pct_aura_like_cpp(
                        spell_id,
                        player_guid,
                        effect,
                    )?;
                } else if effect.effect_aura
                    == wow_data::spell::aura_types::SPELL_AURA_MOD_DETECT_RANGE
                {
                    self.apply_represented_aura_modifier_like_cpp(
                        spell_id,
                        player_guid,
                        effect,
                        RepresentedAuraEffectLikeCpp::ModDetectRange,
                        30_000,
                    )?;
                } else if effect.effect_aura
                    == wow_data::spell::aura_types::SPELL_AURA_MOD_DETECTED_RANGE
                {
                    self.apply_represented_aura_modifier_like_cpp(
                        spell_id,
                        player_guid,
                        effect,
                        RepresentedAuraEffectLikeCpp::ModDetectedRange,
                        30_000,
                    )?;
                } else if effect.effect_aura
                    == wow_data::spell::aura_types::SPELL_AURA_MOD_RESTED_XP_CONSUMPTION
                {
                    self.apply_represented_aura_modifier_like_cpp(
                        spell_id,
                        player_guid,
                        effect,
                        RepresentedAuraEffectLikeCpp::ModRestedXpConsumption,
                        30_000,
                    )?;
                } else if effect.effect_aura == wow_data::spell::aura_types::SPELL_AURA_MOD_TAUNT {
                    if !spell_info.effects().iter().any(|effect| {
                        effect.effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_ATTACK_ME
                    }) {
                        if !target_guid.is_any_type_creature() {
                            return Err("Taunt aura requires a creature target");
                        }
                        self.apply_taunt_effect_like_cpp(spell_id, target_guid, false)?;
                    }
                } else if effect.effect_aura == wow_data::spell::aura_types::SPELL_AURA_MOD_SCALE {
                    self.apply_represented_aura_modifier_like_cpp(
                        spell_id,
                        player_guid,
                        effect,
                        RepresentedAuraEffectLikeCpp::ModScale,
                        30_000,
                    )?;
                } else if effect.effect_aura
                    == wow_data::spell::aura_types::SPELL_AURA_MOD_SPEED_NO_CONTROL
                {
                    self.apply_represented_aura_modifier_like_cpp(
                        spell_id,
                        player_guid,
                        effect,
                        RepresentedAuraEffectLikeCpp::ModSpeedNoControl,
                        30_000,
                    )?;
                } else if effect.effect_aura
                    == wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SPEED
                {
                    self.apply_represented_aura_modifier_like_cpp(
                        spell_id,
                        player_guid,
                        effect,
                        RepresentedAuraEffectLikeCpp::Speed,
                        30_000,
                    )?;
                    self.recompute_represented_run_speed_rate_like_cpp();
                } else if effect.effect_aura
                    == wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_SWIM_SPEED
                {
                    self.apply_represented_aura_modifier_like_cpp(
                        spell_id,
                        player_guid,
                        effect,
                        RepresentedAuraEffectLikeCpp::SwimSpeed,
                        30_000,
                    )?;
                    self.recompute_represented_swim_speed_rate_like_cpp();
                } else if effect.effect_aura
                    == wow_data::spell::aura_types::SPELL_AURA_MOD_DECREASE_SPEED
                {
                    self.apply_represented_aura_modifier_like_cpp(
                        spell_id,
                        player_guid,
                        effect,
                        RepresentedAuraEffectLikeCpp::DecreaseSpeed,
                        30_000,
                    )?;
                    self.recompute_represented_forward_speed_rates_like_cpp();
                    self.recompute_represented_backward_speed_rates_like_cpp();
                } else if effect.effect_aura
                    == wow_data::spell::aura_types::SPELL_AURA_USE_NORMAL_MOVEMENT_SPEED
                {
                    self.apply_represented_aura_modifier_like_cpp(
                        spell_id,
                        player_guid,
                        effect,
                        RepresentedAuraEffectLikeCpp::UseNormalMovementSpeed,
                        30_000,
                    )?;
                    self.recompute_represented_forward_speed_rates_like_cpp();
                } else if effect.effect_aura
                    == wow_data::spell::aura_types::SPELL_AURA_MOD_MINIMUM_SPEED
                {
                    self.apply_represented_aura_modifier_like_cpp(
                        spell_id,
                        player_guid,
                        effect,
                        RepresentedAuraEffectLikeCpp::MinimumSpeed,
                        30_000,
                    )?;
                    self.recompute_represented_run_speed_rate_like_cpp();
                } else if effect.effect_aura
                    == wow_data::spell::aura_types::SPELL_AURA_MOD_MINIMUM_SPEED_RATE
                {
                    self.apply_represented_aura_modifier_like_cpp(
                        spell_id,
                        player_guid,
                        effect,
                        RepresentedAuraEffectLikeCpp::MinimumSpeedRate,
                        30_000,
                    )?;
                    self.recompute_represented_run_speed_rate_like_cpp();
                } else if effect.effect_aura
                    == wow_data::spell::aura_types::SPELL_AURA_MOD_SPEED_ALWAYS
                {
                    self.apply_represented_aura_modifier_like_cpp(
                        spell_id,
                        player_guid,
                        effect,
                        RepresentedAuraEffectLikeCpp::SpeedAlways,
                        30_000,
                    )?;
                    self.recompute_represented_run_speed_rate_like_cpp();
                } else if effect.effect_aura
                    == wow_data::spell::aura_types::SPELL_AURA_MOD_SPEED_NOT_STACK
                {
                    self.apply_represented_aura_modifier_like_cpp(
                        spell_id,
                        player_guid,
                        effect,
                        RepresentedAuraEffectLikeCpp::SpeedNotStack,
                        30_000,
                    )?;
                    self.recompute_represented_run_speed_rate_like_cpp();
                } else if effect.effect_aura
                    == wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_MOUNTED_SPEED
                {
                    self.apply_represented_aura_modifier_like_cpp(
                        spell_id,
                        player_guid,
                        effect,
                        RepresentedAuraEffectLikeCpp::MountedSpeed,
                        30_000,
                    )?;
                    self.recompute_represented_mounted_speed_rates_like_cpp();
                } else if effect.effect_aura
                    == wow_data::spell::aura_types::SPELL_AURA_MOD_MOUNTED_SPEED_ALWAYS
                {
                    self.apply_represented_aura_modifier_like_cpp(
                        spell_id,
                        player_guid,
                        effect,
                        RepresentedAuraEffectLikeCpp::MountedSpeedAlways,
                        30_000,
                    )?;
                    self.recompute_represented_mounted_speed_rates_like_cpp();
                } else if effect.effect_aura
                    == wow_data::spell::aura_types::SPELL_AURA_MOD_MOUNTED_SPEED_NOT_STACK
                {
                    self.apply_represented_aura_modifier_like_cpp(
                        spell_id,
                        player_guid,
                        effect,
                        RepresentedAuraEffectLikeCpp::MountedSpeedNotStack,
                        30_000,
                    )?;
                    self.recompute_represented_mounted_speed_rates_like_cpp();
                } else if effect.effect_aura
                    == wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_MOUNTED_FLIGHT_SPEED
                {
                    self.apply_represented_aura_modifier_like_cpp(
                        spell_id,
                        player_guid,
                        effect,
                        RepresentedAuraEffectLikeCpp::MountedFlightSpeed,
                        30_000,
                    )?;
                    self.update_represented_flight_flags_for_flight_aura_like_cpp(true);
                    self.recompute_represented_mounted_speed_rates_like_cpp();
                } else if effect.effect_aura == wow_data::spell::aura_types::SPELL_AURA_FLY {
                    self.apply_represented_aura_modifier_like_cpp(
                        spell_id,
                        player_guid,
                        effect,
                        RepresentedAuraEffectLikeCpp::Fly,
                        30_000,
                    )?;
                    self.update_represented_flight_flags_for_flight_aura_like_cpp(true);
                } else if effect.effect_aura
                    == wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_FLIGHT_SPEED
                {
                    self.apply_represented_aura_modifier_like_cpp(
                        spell_id,
                        player_guid,
                        effect,
                        RepresentedAuraEffectLikeCpp::FlightSpeed,
                        30_000,
                    )?;
                    self.recompute_represented_flight_speed_rate_like_cpp();
                } else if effect.effect_aura
                    == wow_data::spell::aura_types::SPELL_AURA_MOD_INCREASE_VEHICLE_FLIGHT_SPEED
                {
                    self.apply_represented_aura_modifier_like_cpp(
                        spell_id,
                        player_guid,
                        effect,
                        RepresentedAuraEffectLikeCpp::VehicleFlightSpeed,
                        30_000,
                    )?;
                    self.recompute_represented_flight_speed_rate_like_cpp();
                } else if effect.effect_aura
                    == wow_data::spell::aura_types::SPELL_AURA_MOD_MOUNTED_FLIGHT_SPEED_ALWAYS
                {
                    self.apply_represented_aura_modifier_like_cpp(
                        spell_id,
                        player_guid,
                        effect,
                        RepresentedAuraEffectLikeCpp::MountedFlightSpeedAlways,
                        30_000,
                    )?;
                    self.recompute_represented_mounted_speed_rates_like_cpp();
                } else if effect.effect_aura
                    == wow_data::spell::aura_types::SPELL_AURA_MOD_FLIGHT_SPEED_NOT_STACK
                {
                    self.apply_represented_aura_modifier_like_cpp(
                        spell_id,
                        player_guid,
                        effect,
                        RepresentedAuraEffectLikeCpp::FlightSpeedNotStack,
                        30_000,
                    )?;
                    self.recompute_represented_mounted_speed_rates_like_cpp();
                } else if generic_apply_aura_rows_like_cpp == 1 && apply_aura_rows_like_cpp == 1 {
                    self.apply_aura_with_effect_mask_like_cpp(
                        spell_id,
                        player_guid,
                        30000,
                        0x00000001,
                        1u32 << effect.effect_index,
                    )?;
                } else {
                    debug!(
                        account = self.account_id,
                        spell_id,
                        effect_index = effect.effect_index,
                        "Skipping represented generic multi-effect aura grouping until C++ aura effect-mask application is ported"
                    );
                }
            }
        }

        // Aplicar efecto primario histórico para ramas representadas que aún no
        // tienen un consumer per-effect completo.
        match effect_type {
            x if wow_data::spell::spell_effect_types::is_cpp_null_or_unused_noop(x) => {}
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA => {
                if spell_info.effects().is_empty() {
                    self.apply_aura(spell_id, player_guid, 30000, 0x00000001)?;
                }
            }
            x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_INSTAKILL
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_SCHOOL_DAMAGE
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_ENVIRONMENTAL_DAMAGE
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL_MECHANICAL
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL_MAX_HEALTH
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_HEAL_PCT
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_ADD_EXTRA_ATTACKS
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_INEBRIATE
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_REPUTATION
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_POWER_DRAIN
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_ENERGIZE
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_ENERGIZE_PCT
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_POWER_BURN
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_HEALTH_LEECH
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_DUAL_WIELD
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_PARRY
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_BLOCK
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_GIVE_HONOR
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_THREAT
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_DISTRACT
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_MODIFY_THREAT_PERCENT
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_ATTACK_ME
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_MODIFY_COOLDOWN
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_MODIFY_CHARGES
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_SANCTUARY
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_SELF_RESURRECT
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_STUCK
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_PLAY_MOVIE
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_DUEL
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_DISMISS_PET
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_FORCE_DESELECT
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_CHANGE_RAID_MARKER
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_LEARN_SPELL
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_LEARN_TRANSMOG_SET
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_UPGRADE_HEIRLOOM
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_UNCAGE_BATTLEPET
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_CHANGE_BATTLEPET_QUALITY
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_GRANT_BATTLEPET_LEVEL
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_GRANT_BATTLEPET_EXPERIENCE
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_PULL
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_LEARN_TRANSMOG_ILLUSION
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_TRADE_SKILL
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_QUEST_COMPLETE
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_KILL_CREDIT
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_KILL_CREDIT2
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_BIND
                || x == wow_data::spell::spell_effect_types::SPELL_EFFECT_TELEPORT_UNITS => {}
            _ => {
                debug!("Spell effect type {} not yet implemented", effect_type);
            }
        }

        let mut threat_spell_info = spell_info.clone();
        let difficulty = self.current_map_difficulty_id_like_cpp();
        if let Some(effects) = self.spell_store().and_then(|store| {
            store.effects_for_difficulty_like_cpp(
                spell_id,
                difficulty,
                self.difficulty_store().map(AsRef::as_ref),
            )
        }) {
            threat_spell_info.effects = effects.to_vec();
        }
        self.apply_spell_initial_threat_like_cpp(
            spell_id,
            caster_guid,
            target_guid,
            crate::session_rules::represented_spell_is_positive_like_cpp(&threat_spell_info),
        );

        if caster_guid == player_guid {
            // This represented cooldown state belongs to the session player.
            // A triggered creature cast (for example innkeeper bind spell
            // 3286) must not start or advertise a player cooldown.
            if self
                .mutate_cast_execution_like_cpp(|state| {
                    // A prepared client cast already started its global
                    // cooldown in `Spell::prepare`; a `TRIGGERED_FULL_MASK`
                    // server cast carries `TRIGGERED_IGNORE_GCD` and never
                    // starts one.
                    if metadata.client_cast_id.is_none()
                        && !metadata.triggered_ignores_global_cooldown_like_cpp
                    {
                        state.last_cast_time = Some(Instant::now());
                    }
                    state
                        .last_cast_time_per_spell
                        .insert(spell_id, Instant::now());
                })
                .is_none()
            {
                return Ok(());
            }
            self.record_cast_character_spell_cooldown_like_cpp(
                spell_id,
                spell_info.recovery_time_ms.max(spell_info.cooldown_ms),
            );

            // Notify the owning player so the action bar shows the cooldown.
            use wow_packet::packets::spell::CooldownEvent;
            self.send_packet(&CooldownEvent {
                spell_id,
                is_pet: false,
            });
        }

        Ok(())
    }
}
