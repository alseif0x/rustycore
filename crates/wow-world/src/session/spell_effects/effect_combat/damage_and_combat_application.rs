use super::*;

impl WorldSession {
    pub(in crate::session) fn apply_effect_environmental_damage_like_cpp(
        &mut self,
        effect: &wow_data::SpellEffectInfo,
        target_guid: ObjectGuid,
    ) {
        if effect.effect != wow_data::spell::spell_effect_types::SPELL_EFFECT_ENVIRONMENTAL_DAMAGE {
            return;
        }

        let Some(player_guid) = self.player_guid() else {
            return;
        };
        if target_guid != player_guid || self.resolved_player_is_alive_like_cpp() != Some(true) {
            return;
        }
        if self
            .resolved_player_damage_control_like_cpp()
            .is_none_or(|state| state.environmental_damage_immune)
        {
            return;
        }

        let damage = u32::try_from(effect.effect_base_points.max(0)).unwrap_or(0);
        let Some((original_health, health_after, _, _, _)) =
            self.apply_owned_player_damage_like_cpp(damage, wow_constants::DeathState::JustDied)
        else {
            return;
        };
        self.sync_player_registry_state_like_cpp();
        if health_after != original_health {
            self.send_player_health_update_like_cpp(player_guid, u64::from(health_after));
        }
        self.send_environmental_damage_log_like_cpp(
            player_guid,
            DAMAGE_FIRE_LIKE_CPP,
            damage,
            0,
            0,
        );
    }

    /// C++ `Spell::EffectTaunt` / `SPELL_EFFECT_ATTACK_ME`.
    pub(in crate::session) fn apply_taunt_effect_like_cpp(
        &mut self,
        spell_id: i32,
        target_guid: ObjectGuid,
        enforce_effect_taunt_current_victim_gate: bool,
        provenance: wow_entities::AuraCastProvenanceLikeCpp,
    ) -> Result<(), &'static str> {
        let player_guid = self.player_guid().ok_or("No player GUID")?;
        let difficulty = self.current_map_difficulty_id_like_cpp();
        let taunt_effect_mask = self.spell_store().and_then(|store| {
            store
                .effects_for_difficulty_like_cpp(
                    spell_id,
                    difficulty,
                    self.difficulty_store().map(AsRef::as_ref),
                )
                .and_then(|effects| {
                    effects.iter().find_map(|effect| {
                        (effect.effect
                            == wow_data::spell::spell_effect_types::SPELL_EFFECT_APPLY_AURA
                            && effect.effect_aura
                                == wow_data::spell::aura_types::SPELL_AURA_MOD_TAUNT)
                            .then(|| 1u32.checked_shl(effect.effect_index))
                            .flatten()
                    })
                })
        });
        let duration_ms = {
            let duration_index = self
                .spell_catalogs
                .spell_misc_store
                .as_deref()
                .and_then(|store| {
                    store.entry_for_spell_difficulty_like_cpp(
                        u32::try_from(spell_id).unwrap_or(0),
                        self.current_map_difficulty_id_like_cpp(),
                    )
                })
                .map(|entry| u32::from(entry.duration_index))
                .unwrap_or(0);
            spell_duration_ms_like_cpp(
                duration_index,
                self.spell_catalogs.spell_duration_store.as_deref(),
            )
        };

        let Some((threat_value, taunt_slot)) = self
            .mutate_world_creature(target_guid, |creature| {
                let combat = &mut creature.creature.unit_mut().subsystems_mut().combat;
                if !combat.owner_can_have_threat_list
                    || (enforce_effect_taunt_current_victim_gate
                        && combat.current_victim_guid == Some(player_guid))
                {
                    return None;
                }
                let threat = (!combat.is_threat_list_empty(false))
                    .then(|| combat.match_unit_threat_to_highest_threat_like_cpp(player_guid))
                    .flatten();
                if taunt_effect_mask.is_none() {
                    combat.set_threat_taunt_state(
                        player_guid,
                        wow_entities::ThreatTauntState::Taunt(1),
                    );
                    combat.current_victim_guid = Some(player_guid);
                }
                let taunt_slot =
                    taunt_effect_mask
                        .filter(|_| duration_ms != 0)
                        .and_then(|effect_mask| {
                            creature.apply_taunt_aura_with_provenance_like_cpp(
                                player_guid,
                                u32::try_from(spell_id).ok()?,
                                effect_mask,
                                duration_ms,
                                provenance,
                            )
                        });
                Some((threat, taunt_slot))
            })
            .flatten()
        else {
            return Ok(());
        };

        if let Some(threat_value) = threat_value {
            self.sync_represented_creature_threat_to_canonical_like_cpp(
                target_guid,
                player_guid,
                threat_value,
            );
        }
        if let (Some(slot), Some(effect_mask)) = (taunt_slot, taunt_effect_mask) {
            use wow_packet::ServerPacket;

            let visible_duration_ms = u32::try_from(duration_ms).unwrap_or(u32::MAX);
            let aura = AuraApplication {
                spell_id,
                difficulty_id: self.current_map_difficulty_id_like_cpp(),
                caster_guid: player_guid,
                slot,
                duration_total: visible_duration_ms,
                duration_remaining: visible_duration_ms,
                stack_count: 1,
                aura_flags: 0,
                effect_mask,
                aura_interrupt_flags: 0,
                aura_interrupt_flags2: 0,
                represented_effect: None,
                represented_amount: 0,
                represented_effect_amounts: Vec::new(),
                represented_misc_value: None,
                represented_multiplier: 1.0,
                applied_at: Instant::now(),
            };
            let packet = wow_packet::packets::misc::AuraUpdate {
                unit_guid: target_guid,
                update_all: false,
                auras: vec![crate::session::player_aura_info_like_cpp(
                    &aura,
                    self.player_level_like_cpp(),
                    self.player_map_id_like_cpp(),
                )],
            };
            self.send_packet(&packet);
            self.broadcast_creature_packet_to_visible_set_like_cpp(target_guid, packet.to_bytes());
        }

        // C++ special-cases Hand of Reckoning (62124) by casting 67485 on
        // non-player targets outside the threat-list path. That triggered spell
        // requires broader spell/aura runtime and is intentionally outside this
        // represented threat-list slice.
        if spell_id == 62124 {
            debug!(
                account = self.account_id,
                target = ?target_guid,
                "represented EffectTaunt does not yet cast Hand of Reckoning damage spell 67485"
            );
        }

        Ok(())
    }

    /// Helper: apply damage to target creature.
    pub(in crate::session) async fn apply_damage_with_generator_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        spell_id: Option<i32>,
        target_guid: ObjectGuid,
        damage_amount: u32,
    ) -> Result<(), &'static str> {
        let player_guid = self.player_guid().ok_or("No player GUID")?;
        self.apply_damage_from_caster_like_cpp(
            item_guid_generator,
            spell_id,
            player_guid,
            target_guid,
            damage_amount,
            ObjectGuid::create_null(),
            0,
        )
        .await
    }

    #[cfg(test)]
    pub(in crate::session) async fn apply_damage(
        &mut self,
        spell_id: Option<i32>,
        target_guid: ObjectGuid,
        damage_amount: u32,
    ) -> Result<(), &'static str> {
        let generators = self.id_generators_for_test_like_cpp();
        self.apply_damage_with_generator_like_cpp(
            generators.item.as_ref(),
            spell_id,
            target_guid,
            damage_amount,
        )
        .await
    }

    pub(in crate::session) async fn apply_damage_from_caster_like_cpp(
        &mut self,
        item_guid_generator: &wow_core::ObjectGuidGenerator,
        spell_id: Option<i32>,
        caster_guid: ObjectGuid,
        target_guid: ObjectGuid,
        damage_amount: u32,
        // C++ `SpellNonMeleeDamage::castId`/`SpellVisual`: the combat log needs
        // the cast identity the client correlates with `SMSG_SPELL_GO`. Callers
        // without a represented cast context pass an empty GUID and no visual.
        cast_id: ObjectGuid,
        spell_visual_id: u32,
    ) -> Result<(), &'static str> {
        use wow_packet::ServerPacket;
        use wow_packet::packets::movement::MonsterMoveStop;

        let player_guid = self.player_guid().ok_or("No player GUID")?;
        let account_id = self.account_id;
        let caster_is_session_player = caster_guid == player_guid;
        let controlling_player_guid = if caster_guid.is_player() {
            Some(caster_guid)
        } else {
            self.mutate_world_creature(caster_guid, |creature| {
                creature
                    .creature
                    .unit()
                    .subsystems()
                    .control
                    .charmer_or_owner_guid()
                    .filter(|guid| guid.is_player())
            })
            .flatten()
        };
        let caster_rewards_session_player = controlling_player_guid == Some(player_guid);
        let tap_group_guids = caster_rewards_session_player
            .then(|| self.current_group_member_guids_for_tap_like_cpp(player_guid))
            .unwrap_or_default();
        let difficulty = self.current_map_difficulty_id_like_cpp();
        let difficulty_store = self.difficulty_store().cloned();
        let suppress_harmful_threat = spell_id.is_some_and(|spell_id| {
            self.spell_store().is_some_and(|store| {
                store.has_attribute_for_difficulty_like_cpp(
                    spell_id,
                    difficulty,
                    difficulty_store.as_deref(),
                    1,
                    wow_data::spell::attributes::SPELL_ATTR1_NO_THREAT,
                ) || store.has_attribute_for_difficulty_like_cpp(
                    spell_id,
                    difficulty,
                    difficulty_store.as_deref(),
                    4,
                    wow_data::spell::attributes::SPELL_ATTR4_NO_HARMFUL_THREAT,
                )
            })
        });
        let no_initial_threat = spell_id.is_some_and(|spell_id| {
            self.spell_store().is_some_and(|store| {
                store.has_attribute_for_difficulty_like_cpp(
                    spell_id,
                    difficulty,
                    difficulty_store.as_deref(),
                    2,
                    wow_data::spell::attributes::SPELL_ATTR2_NO_INITIAL_THREAT,
                )
            })
        });
        let spell_threat_entry = spell_id
            .and_then(|spell_id| u32::try_from(spell_id).ok())
            .and_then(|spell_id| self.spell_threat_entry_like_cpp(spell_id))
            .copied();
        let spell_threat_pct_mod = spell_threat_entry.map_or(1.0, |entry| entry.pct_mod);
        let spell_school_mask = spell_id
            .and_then(|spell_id| u32::try_from(spell_id).ok())
            .map_or(1, |spell_id| {
                self.spell_school_mask_for_difficulty_like_cpp(
                    spell_id,
                    self.current_map_difficulty_id_like_cpp(),
                )
            });
        let caster_school_threat_mod = if caster_is_session_player {
            self.hydrate_canonical_threat_relevant_auras_like_cpp();
            self.mutate_canonical_player_like_cpp(|player| {
                player
                    .unit()
                    .subsystems()
                    .auras
                    .total_aura_multiplier_by_misc_mask_like_cpp(
                        wow_data::spell::aura_types::SPELL_AURA_MOD_THREAT,
                        spell_school_mask,
                    )
            })
            .unwrap_or(1.0)
        } else {
            1.0
        };

        // Si target es otra criatura — mutate canonical shared map state.
        let damage_outcome = self
            .mutate_world_creature(target_guid, |creature| {
                if !creature.is_alive() {
                    debug!(
                        account = account_id,
                        creature = ?target_guid,
                        damage = damage_amount,
                        "Skipping spell damage because C++ EffectSchoolDMG requires alive target"
                    );
                    return None;
                }
                info!(
                    account = account_id,
                    creature = ?target_guid,
                    damage = damage_amount,
                    "Dealt damage to creature"
                );

                if caster_rewards_session_player {
                    creature
                        .creature
                        .set_tapped_by_player(player_guid, &tap_group_guids);
                }
                // C++ `SpellNonMeleeDamage::preHitHealth`, read before
                // `DealDamage` so the log can report the overkill.
                let pre_hit_health = creature.current_hp();
                let died = creature.take_damage_before_death_state_like_cpp(damage_amount);
                let newly_engaged = !died
                    && damage_amount > 0
                    && !suppress_harmful_threat
                    && !(no_initial_threat && !creature.creature.is_in_combat())
                    && creature.creature.ai_ownership().combat_target.is_none();
                let threat_value = if !died
                    && damage_amount > 0
                    && !suppress_harmful_threat
                    && !(no_initial_threat && !creature.creature.is_in_combat())
                {
                    // C++ `Spell::DoAllEffectOnTarget` calls `Unit::AtTargetAttacked`, then
                    // `Unit::DealDamage` adds threat for non-player hostile victims.
                    if creature.creature.ai_ownership().combat_target.is_none() {
                        creature.enter_combat(caster_guid);
                    }
                    creature
                        .creature
                        .unit_mut()
                        .subsystems_mut()
                        .combat
                        .add_threat(
                            caster_guid,
                            damage_amount as f32 * spell_threat_pct_mod * caster_school_threat_mod,
                        );
                    creature
                        .creature
                        .unit()
                        .subsystems()
                        .combat
                        .threat_value(caster_guid)
                } else {
                    None
                };
                let kill_info = if died {
                    info!(
                        "Creature {} (entry={}) killed",
                        target_guid,
                        creature.entry()
                    );
                    let move_stop = creature.stop_move_spline_like_cpp().map(|stop| {
                        MonsterMoveStop {
                            mover_guid: target_guid,
                            current_pos: stop.position,
                            spline_id: stop.spline_id,
                        }
                        .to_bytes()
                    });
                    Some((creature.entry(), target_guid, move_stop))
                } else {
                    None
                };
                Some((
                    kill_info,
                    creature.creature.unit().values_update(),
                    threat_value,
                    newly_engaged,
                    pre_hit_health,
                ))
            })
            .ok_or("Target creature not found")?;
        let Some((kill_info, mut values_update, threat_value, newly_engaged, pre_hit_health)) =
            damage_outcome
        else {
            return Ok(());
        };

        // C++ `Unit::DealSpellDamage` sends the combat log for the hit before
        // the kill cascade (`Unit.cpp:1250-1260`, `Unit::SendSpellNonMeleeDamageLog`
        // `Unit.cpp:5353-5380`). The represented hit has no spell absorb, resist
        // or block stage for a creature target yet, and no spell critical
        // representation, so those fields and `HitInfo` stay zero.
        if let Some(spell_id) = spell_id {
            let damage = damage_amount.min(i32::MAX as u32) as i32;
            self.send_packet(&wow_packet::packets::combat::SpellNonMeleeDamageLog {
                target: target_guid,
                caster: caster_guid,
                cast_id,
                spell_id,
                visual_id: spell_visual_id.min(i32::MAX as u32) as i32,
                damage,
                original_damage: damage,
                overkill: if damage_amount > pre_hit_health {
                    i32::try_from(damage_amount - pre_hit_health).unwrap_or(i32::MAX)
                } else {
                    -1
                },
                school_mask: spell_school_mask.min(u32::from(u8::MAX)) as u8,
                absorbed: 0,
                resisted: 0,
                shield_block: 0,
                periodic: false,
                flags: 0,
            });
        }
        if let Some(threat_value) = threat_value {
            self.sync_represented_creature_threat_to_canonical_like_cpp(
                target_guid,
                caster_guid,
                threat_value,
            );
        }
        if newly_engaged {
            self.publish_spell_pull_attack_start_like_cpp(target_guid, caster_guid);
        }

        // Process creature death outside the mutable borrow
        if let Some((entry, guid, move_stop)) = kill_info {
            if let Some(bytes) = move_stop {
                let _ = self.send_tx().send(bytes);
            }
            self.ensure_represented_creature_kill_loot_like_cpp(guid)
                .await;
            // Give XP for the kill
            let (mob_level, can_give_experience) = self
                .mutate_world_creature(guid, |creature| {
                    (
                        creature.level(),
                        creature.creature.can_give_experience_like_cpp(),
                    )
                })
                .unwrap_or((1, false));
            let xp = can_give_experience
                .then(|| self.creature_kill_xp(mob_level))
                .unwrap_or(0);
            if caster_rewards_session_player && xp > 0 {
                // This direct spell-damage kill path has no represented
                // KillRewarder group fanout yet; do not advertise a group
                // rate until the XP amount is scaled per member like C++.
                self.give_xp(xp, guid, 1.0).await;
            }
            if caster_rewards_session_player {
                let reputation_rate =
                    self.represented_creature_kill_reputation_rate_like_cpp(player_guid, guid);
                self.reward_reputation_from_creature_kill_like_cpp(
                    entry,
                    guid,
                    mob_level,
                    reputation_rate,
                );
                self.on_creature_killed_with_generator_like_cpp(item_guid_generator, entry, guid)
                    .await;
                #[cfg(test)]
                self.record_represented_creature_kill_hooks_like_cpp(player_guid, guid);
            }
            if let Some(death_values_update) = self
                .complete_represented_creature_death_state_after_kill_hooks_like_cpp(
                    caster_guid,
                    guid,
                )
            {
                values_update = death_values_update;
            }
        }

        if self.client_visible_guids_like_cpp.contains(&target_guid)
            && let Some(update) = self.represented_unit_values_update_to_update_object_like_cpp(
                target_guid,
                self.player_map_id_like_cpp(),
                &values_update,
            )
        {
            self.send_packet(&update);
        }

        Ok(())
    }
}
