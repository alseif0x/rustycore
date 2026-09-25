//! The represented spell execution entry point that drives effect application.
//!
//! Moved out of the Session root under #621. Behaviour is preserved; the
//! canonical owner of this state is unchanged.

use super::*;

mod completion;
mod post_spell_go;
mod preparation;
mod primary_effect_fallback;
mod target_resolution;

use target_resolution::ResolvedSpellEffectTargetsLikeCpp;

/// Represented aura duration for a creature target: the same bounded 30-second
/// convention the player self-cast chain uses until `SpellDuration.db2` is
/// consumed for every represented aura.
const REPRESENTED_CREATURE_AURA_DURATION_MS_LIKE_CPP: u32 = 30_000;

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

        // Load spell metadata.
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

        let Some(represented_focus_object) = self.prepare_represented_spell_execution_like_cpp(
            &spell_info,
            &target_data,
            spell_id,
            cast_id,
            &spell_visual,
            metadata,
        )?
        else {
            return Ok(());
        };

        // C++ `Spell::SelectSpellTargets` resolves implicit destinations in
        // effect/target order before `Spell::SendSpellGo`; later selections
        // replace earlier ones through `SpellCastTargets::ModDst`.
        let ResolvedSpellEffectTargetsLikeCpp {
            target_data,
            spell_go_target_data,
            effect_target_data_like_cpp,
        } = self.resolve_spell_effect_target_data_like_cpp(
            spell_id,
            caster_guid,
            &spell_info,
            target_data,
            represented_focus_object,
        );

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
            cast_time_ms: crate::session::game_time_ms_like_cpp(),
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
        self.apply_post_spell_go_target_effects_like_cpp(
            creature_spawn_catalogs,
            &spell_info,
            &target_data,
            &effect_target_data_like_cpp,
            spell_id,
            caster_guid,
            target_guid,
            spell_visual_id,
            effect_type,
            effect_base_points,
        )
        .await;

        let force_visibility_after_gameobject_summon = self
            .apply_spell_gameobject_summon_effects_like_cpp(
                spell_id,
                &spell_info,
                &target_data,
                &effect_target_data_like_cpp,
                represented_focus_object,
            );
        if force_visibility_after_gameobject_summon {
            self.force_update_visibility_with_catalogs_like_cpp(creature_spawn_catalogs)
                .await;
        }

        let direct_spell_effects_like_cpp: Vec<(u32, i32, u32, i32, i32, f32, f32)> =
            if spell_info.effects().is_empty() {
                vec![(effect_type, effect_base_points, 0, 0, 0, 0.0, 0.0)]
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
                            effect.effect_bonus_coefficient_from_ap,
                            effect.effect_amplitude,
                        )
                    })
                    .collect()
            };
        // C++ owns one `Spell` object per cast, so `_executeLogEffects` starts
        // empty for it; the represented session reuses one accumulator.
        self.represented_spell_execute_log_effects_like_cpp.clear();
        for (
            direct_effect_type,
            direct_effect_base_points,
            direct_effect_index,
            direct_effect_misc_value_1,
            direct_effect_trigger_spell,
            direct_effect_bonus_coefficient_from_ap,
            direct_effect_amplitude,
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
                        let heal_amount = self.represented_spell_healing_bonus_done_like_cpp(
                            spell_id,
                            caster_guid,
                            target_guid,
                            spell_info.effect_bonus_coefficient,
                            direct_effect_bonus_coefficient_from_ap,
                            heal_amount,
                        );
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
                        x,
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
                    let drain_damage = self.power_drain_pre_scaled_damage_like_cpp(
                        spell_id,
                        direct_effect_index,
                        caster_guid,
                        target_guid,
                        spell_info.effect_bonus_coefficient,
                        direct_effect_bonus_coefficient_from_ap,
                        direct_effect_base_points,
                    );
                    self.apply_power_drain_effect_like_cpp(
                        item_guid_generator,
                        spell_id,
                        x,
                        drain_damage,
                        direct_effect_misc_value_1,
                        target_guid,
                        false,
                        direct_effect_amplitude,
                        cast_id,
                        spell_visual_id,
                    )
                    .await;
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_ENERGIZE => {
                    self.apply_energize_effect_like_cpp(
                        spell_id,
                        caster_guid,
                        direct_effect_base_points,
                        direct_effect_misc_value_1,
                        target_guid,
                        false,
                    );
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_ENERGIZE_PCT => {
                    self.apply_energize_effect_like_cpp(
                        spell_id,
                        caster_guid,
                        direct_effect_base_points,
                        direct_effect_misc_value_1,
                        target_guid,
                        true,
                    );
                }
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_POWER_BURN => {
                    let burn_damage = self.power_drain_pre_scaled_damage_like_cpp(
                        spell_id,
                        direct_effect_index,
                        caster_guid,
                        target_guid,
                        spell_info.effect_bonus_coefficient,
                        direct_effect_bonus_coefficient_from_ap,
                        direct_effect_base_points,
                    );
                    self.apply_power_drain_effect_like_cpp(
                        item_guid_generator,
                        spell_id,
                        x,
                        burn_damage,
                        direct_effect_misc_value_1,
                        target_guid,
                        true,
                        direct_effect_amplitude,
                        cast_id,
                        spell_visual_id,
                    )
                    .await;
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
                x if x == wow_data::spell::spell_effect_types::SPELL_EFFECT_DURABILITY_DAMAGE => {
                    self.apply_durability_damage_effect_like_cpp(
                        x,
                        direct_effect_base_points,
                        direct_effect_misc_value_1,
                        target_guid,
                    );
                }
                x if x
                    == wow_data::spell::spell_effect_types::SPELL_EFFECT_DURABILITY_DAMAGE_PCT =>
                {
                    self.apply_durability_damage_pct_effect_like_cpp(
                        direct_effect_base_points,
                        direct_effect_misc_value_1,
                        target_guid,
                    );
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
                    self.apply_taunt_effect_like_cpp(
                        spell_id,
                        target_guid,
                        true,
                        wow_entities::AuraCastProvenanceLikeCpp {
                            cast_id,
                            spell_visual_id: spell_visual_id.min(i32::MAX as u32) as i32,
                        },
                    )?;
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
                        let damage_amount = self.represented_spell_damage_bonus_done_like_cpp(
                            spell_id,
                            direct_effect_index,
                            caster_guid,
                            target_guid,
                            spell_info.effect_bonus_coefficient,
                            direct_effect_bonus_coefficient_from_ap,
                            damage_amount,
                        );
                        self.apply_damage_from_caster_like_cpp(
                            item_guid_generator,
                            Some(spell_id),
                            caster_guid,
                            target_guid,
                            damage_amount,
                            cast_id,
                            spell_visual_id,
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
            // C++ `Spell::EffectApplyAura` applies to `unitTarget`, not to the
            // caster. The represented player chain below is the self-cast path;
            // a creature target receives one canonical `AuraApplication`
            // carrying every applied effect slot.
            let mut creature_aura_applied = false;
            for effect in spell_info.effects().iter().filter(|effect| {
                effect.effect == wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA
            }) {
                // The represented taunt path owns `SPELL_AURA_MOD_TAUNT` and
                // its `EffectTaunt` current-victim gate, so that aura keeps
                // flowing through the chain below instead of the generic
                // creature application.
                if target_guid != player_guid
                    && target_guid.is_creature()
                    && effect.effect_aura != wow_data::spell::aura_types::SPELL_AURA_MOD_TAUNT
                {
                    if creature_aura_applied {
                        continue;
                    }
                    creature_aura_applied = true;
                    let effect_mask = spell_info
                        .effects()
                        .iter()
                        .filter(|candidate| {
                            candidate.effect
                                == wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA
                        })
                        .fold(0u32, |mask, candidate| {
                            mask | 1u32.checked_shl(candidate.effect_index).unwrap_or(0)
                        });
                    self.apply_creature_aura_with_provenance_like_cpp(
                        spell_id,
                        caster_guid,
                        target_guid,
                        effect_mask,
                        REPRESENTED_CREATURE_AURA_DURATION_MS_LIKE_CPP,
                        wow_entities::AuraCastProvenanceLikeCpp {
                            cast_id,
                            spell_visual_id: spell_visual_id.min(i32::MAX as u32) as i32,
                        },
                    )?;
                    continue;
                }
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
                        self.apply_taunt_effect_like_cpp(
                            spell_id,
                            target_guid,
                            false,
                            wow_entities::AuraCastProvenanceLikeCpp {
                                cast_id,
                                spell_visual_id: spell_visual_id.min(i32::MAX as u32) as i32,
                            },
                        )?;
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
                    self.apply_aura_with_effect_mask_and_provenance_like_cpp(
                        spell_id,
                        player_guid,
                        30000,
                        0x00000001,
                        1u32 << effect.effect_index,
                        wow_entities::AuraCastProvenanceLikeCpp {
                            cast_id,
                            spell_visual_id: spell_visual_id.min(i32::MAX as u32) as i32,
                        },
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

        self.apply_primary_spell_effect_fallback_like_cpp(
            effect_type,
            &spell_info,
            spell_id,
            player_guid,
            cast_id,
            spell_visual_id,
        )?;

        self.complete_spell_execution_like_cpp(
            spell_id,
            caster_guid,
            target_guid,
            player_guid,
            spell_info,
            metadata,
        )?;

        Ok(())
    }
}
